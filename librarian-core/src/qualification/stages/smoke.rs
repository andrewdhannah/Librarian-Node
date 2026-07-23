//! Stage 1 — Smoke Test Executor.
//!
//! The smoke test verifies the most basic property: does the model
//! load, receive a prompt, and produce a non-empty response?
//!
//! Pass criteria (all must hold):
//! - Runner state is Completed (not RunnerFailed/ModelFailed/RuntimeFailed/Timeout)
//! - Raw output is non-empty
//! - HTTP status is 200 (if telemetry available)
//! - Output tokens > 0 (if telemetry available)
//! - Generation duration > 0 ms (if telemetry available)
//!
//! Fail criteria (any triggers failure):
//! - Runner state is a failure state
//! - Raw output is empty
//! - HTTP status is non-200 (if telemetry available)
//! - Generation duration is 0 ms (if telemetry available)
//!
//! GPU release verification:
//! - After the run completes, verify that baseline VRAM is restored
//! - Baseline: 3433 MiB, tolerance: 100 MiB
//!
//! Stage 1 success does NOT imply:
//! - Work-role qualification
//! - Capability eligibility
//! - Router readiness
//!
//! Stage 1 success DOES prove:
//! - Runtime pipeline is functional
//! - Model artifact is loadable
//! - Generation hardware is available
//! - GPU resources were released

use anyhow::Result;
use serde::{Deserialize, Serialize};

use librarian_contracts::qualification_request::QualificationRequest;
use crate::qualification::run_result::QualificationRunResult;
use crate::qualification::run_state::RunState;
use crate::qualification::runner::{QualificationRunner, RuntimeExecutor};

/// Smoke test pass criteria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmokeTestCriteria {
    /// Minimum output tokens required (0 = any output is fine).
    pub min_output_tokens: u32,

    /// Whether HTTP 200 is required.
    pub require_http_200: bool,

    /// Whether generation duration must be > 0.
    pub require_positive_duration: bool,
}

impl Default for SmokeTestCriteria {
    fn default() -> Self {
        Self {
            min_output_tokens: 1,
            require_http_200: true,
            require_positive_duration: true,
        }
    }
}

/// Individual criterion evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CriterionResult {
    /// Criterion name.
    pub name: String,

    /// Whether this criterion passed.
    pub passed: bool,

    /// Human-readable evaluation detail.
    pub detail: String,
}

/// GPU release verification status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuReleaseVerification {
    /// Baseline VRAM in MiB.
    pub baseline_vram_mb: u64,

    /// Available VRAM after run in MiB.
    pub available_vram_mb: Option<u64>,

    /// Tolerance in MiB.
    pub tolerance_mb: u64,

    /// Whether VRAM is within tolerance of baseline.
    pub released: bool,

    /// Human-readable verification detail.
    pub detail: String,
}

/// Stage 1 smoke test verdict.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SmokeTestVerdict {
    /// All criteria passed.
    #[serde(rename = "pass")]
    Pass,
    /// At least one criterion failed.
    #[serde(rename = "fail")]
    Fail,
}

/// Complete Stage 1 smoke test result.
///
/// Extends QualificationRunResult with smoke-specific pass/fail evaluation.
/// The runner produces raw evidence; the smoke test evaluates whether
/// that evidence proves the most basic runtime property.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stage1SmokeTestResult {
    /// The underlying qualification run result.
    pub run_result: QualificationRunResult,

    /// The overall verdict.
    pub verdict: SmokeTestVerdict,

    /// Individual criterion evaluations.
    pub criteria_results: Vec<CriterionResult>,

    /// GPU release verification.
    pub gpu_release: GpuReleaseVerification,

    /// When the smoke test completed (RFC 3339).
    pub evaluated_at: String,
}

impl Stage1SmokeTestResult {
    /// Validate the smoke test result structure.
    pub fn validate(&self) -> Result<()> {
        self.run_result.validate()?;
        if self.evaluated_at.is_empty() {
            anyhow::bail!("evaluated_at is empty");
        }
        if self.criteria_results.is_empty() {
            anyhow::bail!("criteria_results is empty");
        }
        Ok(())
    }

    /// Compute SHA-256 hash of the serialized smoke test result.
    pub fn compute_hash(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        let json = serde_json::to_string(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize for hashing: {}", e))?;
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize to JSON: {}", e))
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Failed to parse from JSON: {}", e))
    }

    /// Assert this result contains no capability authority data.
    pub fn assert_no_capability_data(&self) -> Result<()> {
        self.run_result.assert_no_capability_data()?;
        // Stage1SmokeTestResult adds:
        // - verdict (pass/fail — NOT capability status)
        // - criteria_results (evaluation — NOT role assignment)
        // - gpu_release (resource verification — NOT qualification)
        //
        // There are no fields for:
        // - role
        // - capability_status
        // - qualification_status
        // - approved_roles
        // - router_eligible
        Ok(())
    }
}

/// Stage 1 Smoke Test executor.
///
/// Wraps the QualificationRunner with smoke-specific pass/fail criteria.
/// The smoke test is the simplest possible validation: can the model
/// generate a non-empty response?
pub struct Stage1SmokeTest {
    /// The underlying qualification runner.
    runner: QualificationRunner,

    /// Pass/fail criteria.
    criteria: SmokeTestCriteria,

    /// GPU release baseline in MiB.
    baseline_vram_mb: u64,

    /// GPU release tolerance in MiB.
    gpu_tolerance_mb: u64,
}

impl Stage1SmokeTest {
    /// Create a new smoke test with default settings.
    pub fn new(fixtures_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            runner: QualificationRunner::new(fixtures_dir),
            criteria: SmokeTestCriteria::default(),
            baseline_vram_mb: 3433,
            gpu_tolerance_mb: 100,
        }
    }

    /// Create with custom criteria.
    pub fn with_criteria(
        fixtures_dir: impl Into<std::path::PathBuf>,
        criteria: SmokeTestCriteria,
    ) -> Self {
        Self {
            runner: QualificationRunner::new(fixtures_dir),
            criteria,
            baseline_vram_mb: 3433,
            gpu_tolerance_mb: 100,
        }
    }

    /// Create with custom GPU settings.
    pub fn with_gpu_settings(
        fixtures_dir: impl Into<std::path::PathBuf>,
        baseline_vram_mb: u64,
        gpu_tolerance_mb: u64,
    ) -> Self {
        Self {
            runner: QualificationRunner::new(fixtures_dir),
            criteria: SmokeTestCriteria::default(),
            baseline_vram_mb,
            gpu_tolerance_mb,
        }
    }

    /// Execute the smoke test.
    ///
    /// Runs the qualification request through the runner, then evaluates
    /// the result against smoke-specific pass/fail criteria.
    pub fn run<E: RuntimeExecutor>(
        &self,
        request: &QualificationRequest,
        executor: &E,
        port: u16,
        process_id: Option<i32>,
        available_vram_mb: Option<u64>,
    ) -> Stage1SmokeTestResult {
        // Execute the qualification run
        let run_result = self.runner.run(request, executor, port, process_id);

        // Evaluate criteria
        let criteria_results = self.evaluate_criteria(&run_result);

        // Determine verdict
        let verdict = if criteria_results.iter().all(|c| c.passed) {
            SmokeTestVerdict::Pass
        } else {
            SmokeTestVerdict::Fail
        };

        // GPU release verification
        let gpu_release = self.verify_gpu_release(available_vram_mb);

        Stage1SmokeTestResult {
            run_result,
            verdict,
            criteria_results,
            gpu_release,
            evaluated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Evaluate pass/fail criteria against the run result.
    fn evaluate_criteria(&self, result: &QualificationRunResult) -> Vec<CriterionResult> {
        let mut results = Vec::new();

        // Criterion 1: Runner state is Completed
        results.push(CriterionResult {
            name: "runner_state_completed".to_string(),
            passed: result.state == RunState::Completed,
            detail: format!(
                "Runner state: {} (expected: completed)",
                result.state.as_str()
            ),
        });

        // Criterion 2: Raw output is non-empty
        let output_non_empty = result.raw_output.as_ref().map_or(false, |o| !o.is_empty());
        results.push(CriterionResult {
            name: "output_non_empty".to_string(),
            passed: output_non_empty,
            detail: format!(
                "Raw output: {}",
                if output_non_empty {
                    format!("{} chars", result.raw_output.as_ref().unwrap().len())
                } else {
                    "empty or missing".to_string()
                }
            ),
        });

        // Criterion 3: HTTP 200 (if telemetry available)
        if self.criteria.require_http_200 {
            if let Some(status) = result.telemetry.http_status {
                results.push(CriterionResult {
                    name: "http_status_200".to_string(),
                    passed: status == 200,
                    detail: format!("HTTP status: {} (expected: 200)", status),
                });
            } else {
                results.push(CriterionResult {
                    name: "http_status_200".to_string(),
                    passed: false,
                    detail: "HTTP status: not available".to_string(),
                });
            }
        }

        // Criterion 4: Output tokens > min_output_tokens (if telemetry available)
        if let Some(tokens) = result.telemetry.output_tokens {
            results.push(CriterionResult {
                name: "output_tokens_sufficient".to_string(),
                passed: tokens >= self.criteria.min_output_tokens,
                detail: format!(
                    "Output tokens: {} (minimum: {})",
                    tokens, self.criteria.min_output_tokens
                ),
            });
        } else {
            results.push(CriterionResult {
                name: "output_tokens_sufficient".to_string(),
                passed: false,
                detail: "Output tokens: not available".to_string(),
            });
        }

        // Criterion 5: Generation duration > 0 (if required)
        if self.criteria.require_positive_duration {
            if let Some(duration) = result.telemetry.generation_duration_ms {
                results.push(CriterionResult {
                    name: "positive_generation_duration".to_string(),
                    passed: duration > 0,
                    detail: format!("Generation duration: {} ms", duration),
                });
            } else {
                results.push(CriterionResult {
                    name: "positive_generation_duration".to_string(),
                    passed: false,
                    detail: "Generation duration: not available".to_string(),
                });
            }
        }

        results
    }

    /// Verify GPU release after the run.
    fn verify_gpu_release(&self, available_vram_mb: Option<u64>) -> GpuReleaseVerification {
        match available_vram_mb {
            Some(available) => {
                let diff = if available >= self.baseline_vram_mb {
                    available - self.baseline_vram_mb
                } else {
                    self.baseline_vram_mb - available
                };
                let released = diff <= self.gpu_tolerance_mb;

                GpuReleaseVerification {
                    baseline_vram_mb: self.baseline_vram_mb,
                    available_vram_mb: Some(available),
                    tolerance_mb: self.gpu_tolerance_mb,
                    released,
                    detail: format!(
                        "Available: {} MiB, baseline: {} MiB, diff: {} MiB, tolerance: {} MiB — {}",
                        available,
                        self.baseline_vram_mb,
                        diff,
                        self.gpu_tolerance_mb,
                        if released { "within tolerance" } else { "exceeds tolerance" }
                    ),
                }
            }
            None => GpuReleaseVerification {
                baseline_vram_mb: self.baseline_vram_mb,
                available_vram_mb: None,
                tolerance_mb: self.gpu_tolerance_mb,
                released: false,
                detail: "VRAM not available — cannot verify release".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::common::{
        PacketConstraints, PacketExecutionConfig, PacketModelIdentity,
    };
    use crate::qualification::runner::ExecutionResponse;

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

        fn no_http_status() -> Self {
            Self {
                response: ExecutionResponse {
                    output: "some output".to_string(),
                    input_tokens: Some(10),
                    output_tokens: Some(16),
                    generation_duration_ms: 200,
                    http_status: None,
                    error: None,
                },
            }
        }

        fn no_tokens() -> Self {
            Self {
                response: ExecutionResponse {
                    output: "some output".to_string(),
                    input_tokens: None,
                    output_tokens: None,
                    generation_duration_ms: 200,
                    http_status: Some(200),
                    error: None,
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
        ) -> anyhow::Result<ExecutionResponse> {
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
            "qr-smoke-001".to_string(),
            PacketModelIdentity {
                model_id: "minicpm5-1b-q4km".to_string(),
                sha256: "81B64D05A23B".to_string(),
                filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
                quantization: Some("Q4_K_M".to_string()),
            },
            PacketExecutionConfig {
                runtime_profile_id: "prof-q4km".to_string(),
                task_description: "Say hello.".to_string(),
                max_tokens: Some(64),
                temperature: Some(0.0),
                timeout_seconds: Some(30),
            },
            PacketConstraints {
                require_release_proof: true,
                max_vram_mb: Some(4096),
            },
        )
    }

    // MQR-Q2-1: Successful smoke test passes all criteria
    #[test]
    fn test_smoke_pass() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello! How can I help?");
        let result = smoke.run(&test_request(), &executor, 9120, Some(10804), Some(3433));

        assert_eq!(result.verdict, SmokeTestVerdict::Pass);
        assert!(result.criteria_results.iter().all(|c| c.passed));
        assert!(result.run_result.state == RunState::Completed);
    }

    // MQR-Q2-2: Empty output fails the smoke test
    #[test]
    fn test_smoke_fail_empty_output() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::empty_output();
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert_eq!(result.verdict, SmokeTestVerdict::Fail);
        let output_criterion = result.criteria_results.iter().find(|c| c.name == "output_non_empty").unwrap();
        assert!(!output_criterion.passed);
    }

    // MQR-Q2-3: Runtime error fails the smoke test
    #[test]
    fn test_smoke_fail_runtime_error() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::error("Internal server error");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert_eq!(result.verdict, SmokeTestVerdict::Fail);
        let state_criterion = result.criteria_results.iter().find(|c| c.name == "runner_state_completed").unwrap();
        assert!(!state_criterion.passed);
    }

    // MQR-Q2-4: Network error fails the smoke test
    #[test]
    fn test_smoke_fail_network_error() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::network_error();
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert_eq!(result.verdict, SmokeTestVerdict::Fail);
    }

    // MQR-Q2-5: No HTTP status fails when require_http_200 is true
    #[test]
    fn test_smoke_fail_no_http_status() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::no_http_status();
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert_eq!(result.verdict, SmokeTestVerdict::Fail);
        let http_criterion = result.criteria_results.iter().find(|c| c.name == "http_status_200").unwrap();
        assert!(!http_criterion.passed);
    }

    // MQR-Q2-6: No tokens fails output_tokens criterion
    #[test]
    fn test_smoke_fail_no_tokens() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::no_tokens();
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert_eq!(result.verdict, SmokeTestVerdict::Fail);
        let tokens_criterion = result.criteria_results.iter().find(|c| c.name == "output_tokens_sufficient").unwrap();
        assert!(!tokens_criterion.passed);
    }

    // MQR-Q2-7: GPU release verified when VRAM within tolerance
    #[test]
    fn test_gpu_release_within_tolerance() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, Some(3433));

        assert!(result.gpu_release.released);
        assert_eq!(result.gpu_release.baseline_vram_mb, 3433);
        assert_eq!(result.gpu_release.available_vram_mb, Some(3433));
    }

    // MQR-Q2-8: GPU release fails when VRAM exceeds tolerance
    #[test]
    fn test_gpu_release_exceeds_tolerance() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, Some(3200));

        assert!(!result.gpu_release.released);
        assert!(result.gpu_release.available_vram_mb.unwrap() < 3433 - 100);
    }

    // MQR-Q2-9: GPU release not verified when VRAM unavailable
    #[test]
    fn test_gpu_release_unavailable() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert!(!result.gpu_release.released);
        assert!(result.gpu_release.available_vram_mb.is_none());
    }

    // MQR-Q2-10: All criteria evaluated
    #[test]
    fn test_all_criteria_evaluated() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        // Default criteria: runner_state_completed, output_non_empty, http_status_200,
        // output_tokens_sufficient, positive_generation_duration
        assert!(result.criteria_results.len() >= 4);
        let names: Vec<&str> = result.criteria_results.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"runner_state_completed"));
        assert!(names.contains(&"output_non_empty"));
        assert!(names.contains(&"http_status_200"));
        assert!(names.contains(&"output_tokens_sufficient"));
    }

    // MQR-Q2-11: Custom criteria are respected
    #[test]
    fn test_custom_criteria() {
        let criteria = SmokeTestCriteria {
            min_output_tokens: 100,
            require_http_200: false,
            require_positive_duration: false,
        };
        let smoke = Stage1SmokeTest::with_criteria(
            std::path::PathBuf::from("/tmp/fixtures"),
            criteria,
        );
        let executor = MockExecutor::success("Hi");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        // Should fail because output_tokens (32) < min_output_tokens (100)
        assert_eq!(result.verdict, SmokeTestVerdict::Fail);
        let tokens_criterion = result.criteria_results.iter().find(|c| c.name == "output_tokens_sufficient").unwrap();
        assert!(!tokens_criterion.passed);

        // Should NOT have http_status or duration criteria
        assert!(!result.criteria_results.iter().any(|c| c.name == "http_status_200"));
        assert!(!result.criteria_results.iter().any(|c| c.name == "positive_generation_duration"));
    }

    // MQR-Q2-12: Smoke test result validates
    #[test]
    fn test_result_validates() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert!(result.validate().is_ok());
    }

    // MQR-Q2-13: No capability data in smoke test result
    #[test]
    fn test_no_capability_data() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert!(result.assert_no_capability_data().is_ok());
    }

    // MQR-Q2-14: Serialization round-trip
    #[test]
    fn test_serialization_round_trip() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        let json = result.to_json().unwrap();
        let parsed = Stage1SmokeTestResult::from_json(&json).unwrap();
        assert_eq!(result, parsed);
    }

    // MQR-Q2-15: Hash is deterministic
    #[test]
    fn test_hash_deterministic() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        let h1 = result.compute_hash().unwrap();
        let h2 = result.compute_hash().unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    // MQR-Q2-16: Model identity bound from request
    #[test]
    fn test_model_identity_bound() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert_eq!(result.run_result.model_id, "minicpm5-1b-q4km");
        assert_eq!(result.run_result.model_sha256, "81B64D05A23B");
    }

    // MQR-Q2-17: Lifecycle events include smoke evaluation
    #[test]
    fn test_lifecycle_events_preserved() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        // Should have at least Received and Completed events
        assert!(!result.run_result.lifecycle_events.is_empty());
        let first = &result.run_result.lifecycle_events[0];
        assert_eq!(first.state, RunState::Received);
    }

    // MQR-Q2-18: Default criteria produce reasonable defaults
    #[test]
    fn test_default_criteria() {
        let criteria = SmokeTestCriteria::default();
        assert_eq!(criteria.min_output_tokens, 1);
        assert!(criteria.require_http_200);
        assert!(criteria.require_positive_duration);
    }

    // MQR-Q2-19: GPU tolerance is configurable
    #[test]
    fn test_custom_gpu_settings() {
        let smoke = Stage1SmokeTest::with_gpu_settings(
            std::path::PathBuf::from("/tmp/fixtures"),
            4000,
            50,
        );
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, Some(3980));

        assert!(result.gpu_release.released); // diff=20, tolerance=50
        assert_eq!(result.gpu_release.baseline_vram_mb, 4000);
        assert_eq!(result.gpu_release.tolerance_mb, 50);
    }

    // MQR-Q2-20: Evaluated timestamp is set
    #[test]
    fn test_evaluated_at_set() {
        let smoke = Stage1SmokeTest::new(std::path::PathBuf::from("/tmp/fixtures"));
        let executor = MockExecutor::success("Hello!");
        let result = smoke.run(&test_request(), &executor, 9120, None, None);

        assert!(!result.evaluated_at.is_empty());
        assert!(result.evaluated_at.contains("T"));
    }
}
