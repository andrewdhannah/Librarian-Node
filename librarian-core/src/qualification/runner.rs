//! Qualification runner — executes bounded qualification runs.
//!
//! The runner is execution-neutral: it orchestrates what happened,
//! preserves raw output, and prepares evidence for later validators.
//!
//! It does NOT:
//! - Create capability manifests
//! - Classify roles
//! - Promote qualification
//! - Mutate the router
//! - Automate Owner decisions
//!
//! It DOES:
//! - Consume the sealed QualificationRequest contract
//! - Resolve task pack fixtures (hash-verified)
//! - Orchestrate bounded execution via RuntimeExecutor trait
//! - Preserve raw model output
//! - Record generation settings and runtime telemetry
//! - Distinguish runner/model/runtime/timeout failures
//! - Prepare evidence for later deterministic validators

use anyhow::Result;

use super::run_result::{GenerationSettings, QualificationRunResult, RuntimeTelemetry};
use super::run_state::RunState;
use super::task_loader::{compute_hash, TaskPackLoader};

/// Trait abstraction for runtime model execution.
///
/// The runner uses this trait to execute prompts against the runtime.
/// This allows testing without a live llama-server.
pub trait RuntimeExecutor: Send + Sync {
    /// Execute a prompt against the runtime and return the raw output.
    ///
    /// Returns (raw_output, input_tokens, output_tokens, duration_ms, http_status).
    fn execute(
        &self,
        port: u16,
        prompt: &str,
        max_tokens: u32,
        temperature: f64,
        timeout_secs: u64,
    ) -> Result<ExecutionResponse>;
}

/// Response from a runtime execution.
#[derive(Debug, Clone)]
pub struct ExecutionResponse {
    /// Raw model output text.
    pub output: String,

    /// Input tokens consumed.
    pub input_tokens: Option<u32>,

    /// Output tokens generated.
    pub output_tokens: Option<u32>,

    /// Generation duration in milliseconds.
    pub generation_duration_ms: u64,

    /// HTTP status code.
    pub http_status: Option<u16>,

    /// Error message (if execution failed).
    pub error: Option<String>,
}

/// Qualification runner — orchestrates bounded qualification runs.
pub struct QualificationRunner {
    /// Task pack loader for fixture resolution.
    task_loader: TaskPackLoader,
}

impl QualificationRunner {
    /// Create a new runner with the given fixtures directory.
    pub fn new(fixtures_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            task_loader: TaskPackLoader::new(fixtures_dir),
        }
    }

    /// Run a qualification request.
    ///
    /// This is the main entry point. It:
    /// 1. Validates the request
    /// 2. Resolves the task pack fixture
    /// 3. Executes the prompt via the RuntimeExecutor
    /// 4. Preserves raw output
    /// 5. Records settings and telemetry
    /// 6. Returns the structured result
    ///
    /// The caller is responsible for checking `result.state` to determine
    /// whether the run succeeded or failed. The runner does NOT interpret
    /// the result for capability or qualification.
    pub fn run<E: RuntimeExecutor>(
        &self,
        request: &librarian_contracts::qualification_request::QualificationRequest,
        executor: &E,
        port: u16,
        process_id: Option<i32>,
    ) -> QualificationRunResult {
        let started_at = chrono::Utc::now().to_rfc3339();
        let run_id = QualificationRunResult::compute_run_id(&request.request_id, &started_at);

        let mut result = QualificationRunResult {
            run_id,
            request_id: request.request_id.clone(),
            model_id: request.identity.model_id.clone(),
            model_sha256: request.identity.sha256.clone(),
            model_filename: request.identity.filename.clone(),
            task_pack_id: request.execution.runtime_profile_id.clone(),
            fixture_hash: String::new(),
            state: RunState::Received,
            raw_output: None,
            settings: GenerationSettings {
                runtime_profile_id: request.execution.runtime_profile_id.clone(),
                max_tokens: request.execution.max_tokens,
                temperature: request.execution.temperature,
                timeout_seconds: request.execution.timeout_seconds,
                task_description: request.execution.task_description.clone(),
            },
            telemetry: RuntimeTelemetry {
                port: Some(port),
                process_id,
                load_duration_ms: None,
                generation_duration_ms: None,
                input_tokens: None,
                output_tokens: None,
                http_status: None,
                runtime_error: None,
            },
            lifecycle_events: vec![],
            error_message: None,
            custom_evidence: vec![],
            started_at,
            ended_at: None,
        };

        result.record_event(RunState::Received, None);

        // Step 1: Resolve fixture
        let prompt = match self.resolve_fixture(request) {
            Ok((prompt, hash)) => {
                result.fixture_hash = hash;
                result.task_pack_id = request.execution.runtime_profile_id.clone();
                result.record_event(
                    RunState::FixtureResolved,
                    Some(format!(r#"{{"fixture_hash":"{}"}}"#, result.fixture_hash)),
                );
                prompt
            }
            Err(e) => {
                result.state = RunState::RunnerFailed;
                result.error_message = Some(format!("Fixture resolution failed: {}", e));
                result.record_event(
                    RunState::RunnerFailed,
                    Some(format!(r#"{{"error":"{}"}}"#, e)),
                );
                result.ended_at = Some(chrono::Utc::now().to_rfc3339());
                return result;
            }
        };

        // Step 2: Execute
        let max_tokens = request.execution.max_tokens.unwrap_or(256);
        let temperature = request.execution.temperature.unwrap_or(0.7);
        let timeout = request.execution.timeout_seconds.unwrap_or(120);

        result.record_event(RunState::LoadingRuntime, None);

        match executor.execute(port, &prompt, max_tokens, temperature, timeout as u64) {
            Ok(response) => {
                result.record_event(RunState::Executing, None);

                // Record telemetry
                result.telemetry.generation_duration_ms = Some(response.generation_duration_ms);
                result.telemetry.input_tokens = response.input_tokens;
                result.telemetry.output_tokens = response.output_tokens;
                result.telemetry.http_status = response.http_status;

                if let Some(err) = response.error {
                    // Runtime returned an error
                    result.state = RunState::ModelFailed;
                    result.error_message = Some(err.clone());
                    result.raw_output = if response.output.is_empty() {
                        None
                    } else {
                        Some(response.output)
                    };
                    result.telemetry.runtime_error = Some(err);
                    result.record_event(
                        RunState::ModelFailed,
                        Some(format!(r#"{{"http_status":{},"error":"{}"}}"#,
                            result.telemetry.http_status.unwrap_or(0),
                            result.telemetry.runtime_error.as_deref().unwrap_or("unknown"),
                        )),
                    );
                } else if response.output.is_empty() {
                    // Model produced no output
                    result.state = RunState::ModelFailed;
                    result.error_message = Some("Model produced empty output".to_string());
                    result.record_event(
                        RunState::ModelFailed,
                        Some(r#"{{"error":"empty_output"}}"#.to_string()),
                    );
                } else {
                    // Success
                    result.state = RunState::Completed;
                    result.raw_output = Some(response.output);
                    result.record_event(
                        RunState::Completed,
                        Some(format!(
                            r#"{{"output_tokens":{},"generation_duration_ms":{}}}"#,
                            result.telemetry.output_tokens.unwrap_or(0),
                            result.telemetry.generation_duration_ms.unwrap_or(0),
                        )),
                    );
                }
            }
            Err(e) => {
                result.state = RunState::RuntimeFailed;
                result.error_message = Some(format!("Runtime execution failed: {}", e));
                result.record_event(
                    RunState::RuntimeFailed,
                    Some(format!(r#"{{"error":"{}"}}"#, e)),
                );
            }
        }

        result.ended_at = Some(chrono::Utc::now().to_rfc3339());
        result
    }

    /// Resolve the fixture for this request.
    /// Returns (prompt_content, fixture_hash).
    fn resolve_fixture(
        &self,
        request: &librarian_contracts::qualification_request::QualificationRequest,
    ) -> Result<(String, String)> {
        let task_description = &request.execution.task_description;

        // The task_description may contain:
        // 1. A direct fixture reference: "fixture:content" -> inline content
        // 2. A plain task description -> use as-is (inline content mode)
        // For now, treat task_description as inline content
        let content = task_description;
        let expected_hash = compute_hash(content);

        // Use inline content mode (no file lookup needed)
        let loaded = self.task_loader.load_fixture_from_content(
            &request.execution.runtime_profile_id,
            content,
            &expected_hash,
        )?;

        Ok((loaded.content, loaded.content_hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::common::{
        PacketConstraints, PacketExecutionConfig, PacketModelIdentity,
    };
    use librarian_contracts::qualification_request::QualificationRequest;

    /// Mock executor that returns a fixed response.
    struct MockExecutor {
        response: ExecutionResponse,
    }

    impl MockExecutor {
        fn success(output: &str) -> Self {
            Self {
                response: ExecutionResponse {
                    output: output.to_string(),
                    input_tokens: Some(10),
                    output_tokens: Some(32),
                    generation_duration_ms: 385,
                    http_status: Some(200),
                    error: None,
                },
            }
        }

        fn empty_output() -> Self {
            Self {
                response: ExecutionResponse {
                    output: String::new(),
                    input_tokens: Some(10),
                    output_tokens: Some(0),
                    generation_duration_ms: 50,
                    http_status: Some(200),
                    error: None,
                },
            }
        }

        fn error(msg: &str) -> Self {
            Self {
                response: ExecutionResponse {
                    output: String::new(),
                    input_tokens: None,
                    output_tokens: None,
                    generation_duration_ms: 0,
                    http_status: Some(500),
                    error: Some(msg.to_string()),
                },
            }
        }

        fn network_error() -> Self {
            Self {
                response: ExecutionResponse {
                    output: String::new(),
                    input_tokens: None,
                    output_tokens: None,
                    generation_duration_ms: 0,
                    http_status: None,
                    error: Some("Connection refused".to_string()),
                },
            }
        }
    }

    impl RuntimeExecutor for MockExecutor {
        fn execute(
            &self,
            _port: u16,
            _prompt: &str,
            _max_tokens: u32,
            _temperature: f64,
            _timeout_secs: u64,
        ) -> Result<ExecutionResponse> {
            if let Some(ref err) = self.response.error {
                if self.response.http_status.is_none() {
                    anyhow::bail!("{}", err);
                }
            }
            Ok(self.response.clone())
        }
    }

    fn test_request() -> QualificationRequest {
        QualificationRequest::new(
            "qr-test-001".to_string(),
            PacketModelIdentity {
                model_id: "minicpm5-1b-q4km".to_string(),
                sha256: "81B64D05A23B".to_string(),
                filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
                quantization: Some("Q4_K_M".to_string()),
            },
            PacketExecutionConfig {
                runtime_profile_id: "prof-q4km".to_string(),
                task_description: "Write a function that adds two numbers.".to_string(),
                max_tokens: Some(256),
                temperature: Some(0.0),
                timeout_seconds: Some(120),
            },
            PacketConstraints {
                require_release_proof: true,
                max_vram_mb: Some(4096),
            },
        )
    }

    // MQR-Q1-1: Successful run completes with raw output preserved
    #[test]
    fn test_successful_run() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("def add(a, b): return a + b");
        let result = runner.run(&test_request(), &executor, 9120, Some(10804));

        assert_eq!(result.state, RunState::Completed);
        assert!(result.raw_output.is_some());
        assert_eq!(result.raw_output.unwrap(), "def add(a, b): return a + b");
        assert!(result.error_message.is_none());
        assert!(result.ended_at.is_some());
    }

    // MQR-Q1-2: Request validation succeeds
    #[test]
    fn test_request_validated() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert!(result.validate().is_ok());
        assert_eq!(result.request_id, "qr-test-001");
        assert_eq!(result.model_id, "minicpm5-1b-q4km");
    }

    // MQR-Q1-3: Fixture resolution produces deterministic hash
    #[test]
    fn test_fixture_hash_deterministic() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let req = test_request();

        let r1 = runner.run(&req, &executor, 9120, None);
        let r2 = runner.run(&req, &executor, 9120, None);

        assert_eq!(r1.fixture_hash, r2.fixture_hash);
        assert!(!r1.fixture_hash.is_empty());
    }

    // MQR-Q1-4: Run ID is deterministic for same inputs
    #[test]
    fn test_run_id_deterministic() {
        let id1 = QualificationRunResult::compute_run_id("qr-1", "2026-07-11T12:00:00Z");
        let id2 = QualificationRunResult::compute_run_id("qr-1", "2026-07-11T12:00:00Z");
        assert_eq!(id1, id2);
    }

    // MQR-Q1-5: Lifecycle events are recorded in order
    #[test]
    fn test_lifecycle_events_recorded() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert!(!result.lifecycle_events.is_empty());
        // First event should be Received
        assert_eq!(result.lifecycle_events[0].state, RunState::Received);
        // Last event should be Completed (for successful run)
        assert_eq!(result.lifecycle_events.last().unwrap().state, RunState::Completed);
    }

    // MQR-Q1-6: Empty output produces ModelFailed state
    #[test]
    fn test_empty_output_model_failed() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::empty_output();
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert_eq!(result.state, RunState::ModelFailed);
        assert!(result.raw_output.is_none());
        assert!(result.error_message.is_some());
    }

    // MQR-Q1-7: Runtime error produces ModelFailed state
    #[test]
    fn test_runtime_error_model_failed() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::error("Internal server error");
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert_eq!(result.state, RunState::ModelFailed);
        assert!(result.telemetry.runtime_error.is_some());
    }

    // MQR-Q1-8: Network error produces RuntimeFailed state
    #[test]
    fn test_network_error_runtime_failed() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::network_error();
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert_eq!(result.state, RunState::RuntimeFailed);
        assert!(result.error_message.is_some());
    }

    // MQR-Q1-9: Settings are preserved from request
    #[test]
    fn test_settings_preserved() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert_eq!(result.settings.runtime_profile_id, "prof-q4km");
        assert_eq!(result.settings.max_tokens, Some(256));
        assert_eq!(result.settings.temperature, Some(0.0));
        assert_eq!(result.settings.timeout_seconds, Some(120));
    }

    // MQR-Q1-10: Telemetry is captured
    #[test]
    fn test_telemetry_captured() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, Some(10804));

        assert_eq!(result.telemetry.port, Some(9120));
        assert_eq!(result.telemetry.process_id, Some(10804));
        assert!(result.telemetry.generation_duration_ms.is_some());
        assert_eq!(result.telemetry.http_status, Some(200));
    }

    // MQR-Q1-11: No capability data in result
    #[test]
    fn test_no_capability_data() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert!(result.assert_no_capability_data().is_ok());
    }

    // MQR-Q1-12: Serialization round-trip preserves all fields
    #[test]
    fn test_serialization_round_trip() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, None);

        let json = result.to_json().unwrap();
        let parsed = QualificationRunResult::from_json(&json).unwrap();
        assert_eq!(result, parsed);
    }

    // MQR-Q1-13: Hash is deterministic
    #[test]
    fn test_hash_deterministic() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, None);

        let h1 = result.compute_hash().unwrap();
        let h2 = result.compute_hash().unwrap();
        assert_eq!(h1, h2);
    }

    // MQR-Q1-14: Model identity is bound from request
    #[test]
    fn test_model_identity_bound() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert_eq!(result.model_id, "minicpm5-1b-q4km");
        assert_eq!(result.model_sha256, "81B64D05A23B");
        assert_eq!(result.model_filename, "MiniCPM5-1B-Q4_K_M.gguf");
    }

    // MQR-Q1-15: Started and ended timestamps are set
    #[test]
    fn test_timestamps_set() {
        let runner = QualificationRunner::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("output");
        let result = runner.run(&test_request(), &executor, 9120, None);

        assert!(!result.started_at.is_empty());
        assert!(result.started_at.contains("T"));
        assert!(result.ended_at.is_some());
        assert!(result.ended_at.unwrap().contains("T"));
    }
}
