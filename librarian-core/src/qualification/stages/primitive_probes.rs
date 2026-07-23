//! Stage 2 — Primitive Probe Executor.
//!
//! Stage 2 verifies primitive model abilities through targeted probes.
//! Each probe sends a specific prompt and validates the output against
//! versioned validator rules. The probes are narrow and deterministic:
//! they verify basic model capabilities without qualitative scoring.
//!
//! Probe types:
//! - PromptResponse: Does the model respond to a simple prompt?
//! - StructuredOutput: Does the model produce valid JSON when asked?
//! - InstructionFollowing: Does the model follow a simple instruction?
//!
//! Stage 2 success does NOT imply:
//! - Work-role qualification
//! - Capability eligibility
//! - Router readiness
//! - Task-specific proficiency
//!
//! Stage 2 success DOES prove:
//! - Model can respond to prompts (not silent)
//! - Model can produce structured output (if tested)
//! - Model can follow basic instructions (if tested)
//! - Validator rules are deterministic and reproducible

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::qualification::run_result::QualificationRunResult;
use crate::qualification::run_state::RunState;
use crate::qualification::runner::{QualificationRunner, RuntimeExecutor};
use crate::qualification::validator_engine::{
    ValidationResult, ValidatorEngine,
};

/// A single primitive probe definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrimitiveProbe {
    /// Unique probe identifier (e.g., "PP-RESPONSE-001").
    pub probe_id: String,

    /// Probe type determines evaluation approach.
    pub probe_type: ProbeType,

    /// The prompt to send to the model.
    pub prompt: String,

    /// Max tokens for this probe's generation.
    pub max_tokens: Option<u32>,

    /// Temperature for this probe's generation.
    pub temperature: Option<f64>,

    /// Validator rules to apply to the output.
    /// Rules are evaluated against the raw model output.
    pub rules: Vec<crate::qualification::validator_engine::ValidationRule>,

    /// Optional description for human review.
    pub description: Option<String>,
}

/// Probe type determines how the output is evaluated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProbeType {
    /// Basic prompt response — does the model respond at all?
    PromptResponse,

    /// Structured output — does the model produce valid JSON?
    StructuredOutput,

    /// Instruction following — does the model follow a simple instruction?
    InstructionFollowing,
}

/// A set of primitive probes to execute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProbeSet {
    /// Unique probe set identifier.
    pub set_id: String,

    /// Probes to execute in order.
    pub probes: Vec<PrimitiveProbe>,

    /// Optional description.
    pub description: Option<String>,
}

/// Individual probe result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProbeResult {
    /// The probe that was executed.
    pub probe_id: String,

    /// The underlying run result for this probe.
    pub run_result: QualificationRunResult,

    /// Validation result from applying rules to the output.
    pub validation: ValidationResult,

    /// Whether this specific probe passed.
    pub passed: bool,

    /// Human-readable evaluation detail.
    pub detail: String,
}

/// Stage 2 probe verdict.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProbeVerdict {
    /// All probes passed.
    #[serde(rename = "pass")]
    Pass,
    /// At least one probe failed.
    #[serde(rename = "fail")]
    Fail,
}

/// Complete Stage 2 Primitive Probe result.
///
/// Aggregates results from multiple probes into a single verdict.
/// Each probe is evaluated independently; the overall verdict requires
/// all probes to pass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stage2PrimitiveProbeResult {
    /// The probe set that was executed.
    pub probe_set_id: String,

    /// The overall verdict.
    pub verdict: ProbeVerdict,

    /// Individual probe results (one per probe).
    pub probe_results: Vec<ProbeResult>,

    /// Number of probes that passed.
    pub passed_count: usize,

    /// Number of probes that failed.
    pub failed_count: usize,

    /// Total number of probes.
    pub total_count: usize,

    /// When the stage 2 evaluation completed (RFC 3339).
    pub evaluated_at: String,
}

impl Stage2PrimitiveProbeResult {
    /// Validate the stage 2 result structure.
    pub fn validate(&self) -> Result<()> {
        if self.probe_set_id.is_empty() {
            anyhow::bail!("probe_set_id is empty");
        }
        if self.evaluated_at.is_empty() {
            anyhow::bail!("evaluated_at is empty");
        }
        if self.probe_results.is_empty() {
            anyhow::bail!("probe_results is empty");
        }
        if self.total_count != self.probe_results.len() {
            anyhow::bail!(
                "total_count ({}) does not match probe_results length ({})",
                self.total_count,
                self.probe_results.len()
            );
        }
        if self.passed_count + self.failed_count != self.total_count {
            anyhow::bail!(
                "passed_count ({}) + failed_count ({}) != total_count ({})",
                self.passed_count,
                self.failed_count,
                self.total_count
            );
        }
        Ok(())
    }

    /// Compute SHA-256 hash of the serialized result.
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
        for probe_result in &self.probe_results {
            probe_result.run_result.assert_no_capability_data()?;
        }
        // Stage2PrimitiveProbeResult adds:
        // - verdict (pass/fail — NOT capability status)
        // - probe_results (evaluation — NOT role assignment)
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

/// Stage 2 Primitive Probe executor.
///
/// Executes a set of primitive probes against a model and evaluates
/// each probe's output against versioned validator rules.
pub struct Stage2PrimitiveProbe {
    /// The underlying qualification runner.
    runner: QualificationRunner,

    /// The validator engine for evaluating outputs.
    validator: ValidatorEngine,
}

impl Stage2PrimitiveProbe {
    /// Create a new stage 2 executor.
    pub fn new(
        fixtures_dir: impl Into<std::path::PathBuf>,
        rules_dir: impl Into<std::path::PathBuf>,
    ) -> Self {
        Self {
            runner: QualificationRunner::new(fixtures_dir),
            validator: ValidatorEngine::new(rules_dir),
        }
    }

    /// Execute a single probe and return the result.
    pub fn execute_probe<E: RuntimeExecutor>(
        &self,
        probe: &PrimitiveProbe,
        request: &librarian_contracts::qualification_request::QualificationRequest,
        executor: &E,
        port: u16,
        process_id: Option<i32>,
        validator_pack_id: &str,
    ) -> ProbeResult {
        // Override the request's task_description with the probe's prompt
        let mut probe_request = request.clone();
        probe_request.execution.task_description = probe.prompt.clone();
        if let Some(max_tokens) = probe.max_tokens {
            probe_request.execution.max_tokens = Some(max_tokens);
        }
        if let Some(temperature) = probe.temperature {
            probe_request.execution.temperature = Some(temperature);
        }

        // Execute via the runner
        let run_result = self.runner.run(&probe_request, executor, port, process_id);

        // Apply validator rules
        let validation = if run_result.state == RunState::Completed {
            if let Some(ref output) = run_result.raw_output {
                self.validator.evaluate(
                    validator_pack_id,
                    &request.execution.runtime_profile_id,
                    &probe.rules,
                    output,
                    run_result.telemetry.output_tokens.map(|t| t as usize),
                )
            } else {
                // Completed but no output — create a failing validation
                ValidationResult {
                    validator_pack_id: validator_pack_id.to_string(),
                    task_pack_id: request.execution.runtime_profile_id.clone(),
                    rule_results: vec![],
                    overall_pass: false,
                    critical_failures: 1,
                    warnings: 0,
                    infos: 0,
                }
            }
        } else {
            // Run failed — create a failing validation
            ValidationResult {
                validator_pack_id: validator_pack_id.to_string(),
                task_pack_id: request.execution.runtime_profile_id.clone(),
                rule_results: vec![],
                overall_pass: false,
                critical_failures: 1,
                warnings: 0,
                infos: 0,
            }
        };

        // Determine probe pass/fail
        let passed = run_result.state == RunState::Completed && validation.overall_pass;

        let detail = if !run_result.state.is_success() {
            format!(
                "Run failed: {} — {}",
                run_result.state.as_str(),
                run_result.error_message.as_deref().unwrap_or("unknown error")
            )
        } else if validation.critical_failures > 0 {
            format!(
                "Validation failed: {} critical failures, {} warnings",
                validation.critical_failures, validation.warnings
            )
        } else {
            format!(
                "Passed: {} rules evaluated, {} passed",
                validation.rule_results.len(),
                validation.rule_results.iter().filter(|r| r.passed).count()
            )
        };

        ProbeResult {
            probe_id: probe.probe_id.clone(),
            run_result,
            validation,
            passed,
            detail,
        }
    }

    /// Execute a complete probe set and return the aggregated result.
    pub fn run<E: RuntimeExecutor>(
        &self,
        probe_set: &ProbeSet,
        request: &librarian_contracts::qualification_request::QualificationRequest,
        executor: &E,
        port: u16,
        process_id: Option<i32>,
        validator_pack_id: &str,
    ) -> Stage2PrimitiveProbeResult {
        let mut probe_results = Vec::new();
        let mut passed_count = 0;
        let mut failed_count = 0;

        for probe in &probe_set.probes {
            let result = self.execute_probe(
                probe,
                request,
                executor,
                port,
                process_id,
                validator_pack_id,
            );
            if result.passed {
                passed_count += 1;
            } else {
                failed_count += 1;
            }
            probe_results.push(result);
        }

        let verdict = if failed_count == 0 {
            ProbeVerdict::Pass
        } else {
            ProbeVerdict::Fail
        };

        Stage2PrimitiveProbeResult {
            probe_set_id: probe_set.set_id.clone(),
            verdict,
            passed_count,
            failed_count,
            total_count: probe_results.len(),
            evaluated_at: chrono::Utc::now().to_rfc3339(),
            probe_results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::common::{
        PacketConstraints, PacketExecutionConfig, PacketModelIdentity,
    };
    use crate::qualification::validator_engine::{RuleSeverity, RuleType};
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

    fn test_request() -> librarian_contracts::qualification_request::QualificationRequest {
        librarian_contracts::qualification_request::QualificationRequest::new(
            "qr-probe-001".to_string(),
            PacketModelIdentity {
                model_id: "minicpm5-1b-q4km".to_string(),
                sha256: "81B64D05A23B".to_string(),
                filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
                quantization: Some("Q4_K_M".to_string()),
            },
            PacketExecutionConfig {
                runtime_profile_id: "prof-q4km".to_string(),
                task_description: "Test prompt".to_string(),
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

    fn response_probe() -> PrimitiveProbe {
        PrimitiveProbe {
            probe_id: "PP-RESPONSE-001".to_string(),
            probe_type: ProbeType::PromptResponse,
            prompt: "Say hello.".to_string(),
            max_tokens: Some(64),
            temperature: Some(0.0),
            rules: vec![crate::qualification::validator_engine::ValidationRule {
                rule_id: "VR-001".to_string(),
                rule_type: RuleType::MinTokens { min: 1 },
                severity: RuleSeverity::Critical,
                description: Some("Must produce at least 1 token".to_string()),
                params: None,
            }],
            description: Some("Basic response test".to_string()),
        }
    }

    fn json_probe() -> PrimitiveProbe {
        PrimitiveProbe {
            probe_id: "PP-JSON-001".to_string(),
            probe_type: ProbeType::StructuredOutput,
            prompt: r#"Return valid JSON: {"key": "value"}"#.to_string(),
            max_tokens: Some(64),
            temperature: Some(0.0),
            rules: vec![crate::qualification::validator_engine::ValidationRule {
                rule_id: "VR-JSON-001".to_string(),
                rule_type: RuleType::ValidJson,
                severity: RuleSeverity::Critical,
                description: Some("Output must be valid JSON".to_string()),
                params: None,
            }],
            description: Some("JSON output test".to_string()),
        }
    }

    fn instruction_probe() -> PrimitiveProbe {
        PrimitiveProbe {
            probe_id: "PP-INSTR-001".to_string(),
            probe_type: ProbeType::InstructionFollowing,
            prompt: "Say exactly the word 'hello' and nothing else.".to_string(),
            max_tokens: Some(32),
            temperature: Some(0.0),
            rules: vec![crate::qualification::validator_engine::ValidationRule {
                rule_id: "VR-INSTR-001".to_string(),
                rule_type: RuleType::ContainsSubstring,
                severity: RuleSeverity::Critical,
                description: Some("Output must contain 'hello'".to_string()),
                params: Some(r#""hello""#.to_string()),
            }],
            description: Some("Instruction following test".to_string()),
        }
    }

    fn failing_probe() -> PrimitiveProbe {
        PrimitiveProbe {
            probe_id: "PP-FAIL-001".to_string(),
            probe_type: ProbeType::PromptResponse,
            prompt: "Say hello.".to_string(),
            max_tokens: Some(64),
            temperature: Some(0.0),
            rules: vec![crate::qualification::validator_engine::ValidationRule {
                rule_id: "VR-FAIL-001".to_string(),
                rule_type: RuleType::ContainsSubstring,
                severity: RuleSeverity::Critical,
                description: Some("Output must contain 'xyznonexistent'".to_string()),
                params: Some(r#""xyznonexistent""#.to_string()),
            }],
            description: Some("Expected to fail".to_string()),
        }
    }

    fn test_probe_set() -> ProbeSet {
        ProbeSet {
            set_id: "PS-001".to_string(),
            probes: vec![response_probe(), json_probe(), instruction_probe()],
            description: Some("Basic probe set".to_string()),
        }
    }

    // MQR-Q3-1: Single probe executes correctly
    #[test]
    fn test_single_probe_execution() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello! How can I help?");
        let result = stage2.execute_probe(
            &response_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert!(result.passed);
        assert_eq!(result.probe_id, "PP-RESPONSE-001");
        assert!(result.run_result.state == RunState::Completed);
    }

    // MQR-Q3-2: Failing run produces failed probe
    #[test]
    fn test_probe_failing_run() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::error("Internal error");
        let result = stage2.execute_probe(
            &response_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert!(!result.passed);
        assert!(result.run_result.state.is_failure());
    }

    // MQR-Q3-3: Validation failure produces failed probe
    #[test]
    fn test_probe_validation_failure() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.execute_probe(
            &failing_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert!(!result.passed);
        assert!(result.validation.critical_failures > 0);
    }

    // MQR-Q3-4: Complete probe set executes all probes
    #[test]
    fn test_probe_set_execution() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.run(
            &test_probe_set(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert_eq!(result.total_count, 3);
        assert_eq!(result.probe_results.len(), 3);
    }

    // MQR-Q3-5: Probe set verdict is Pass when all pass
    #[test]
    fn test_probe_set_all_pass() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        // Return valid JSON containing lowercase "hello" so all probes pass
        let executor = MockExecutor::success(r#"{"hello":"world"}"#);
        let result = stage2.run(
            &test_probe_set(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        // Response probe: passes (MinTokens 1 → output is non-empty)
        // JSON probe: passes (output is valid JSON)
        // Instruction probe: passes (contains "hello")
        assert_eq!(result.verdict, ProbeVerdict::Pass);
        assert_eq!(result.passed_count, 3);
        assert_eq!(result.failed_count, 0);
    }

    // MQR-Q3-6: Probe set verdict is Fail when any fail
    #[test]
    fn test_probe_set_any_fail() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let fail_set = ProbeSet {
            set_id: "PS-FAIL".to_string(),
            probes: vec![response_probe(), failing_probe()],
            description: None,
        };
        let result = stage2.run(
            &fail_set,
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert_eq!(result.verdict, ProbeVerdict::Fail);
        assert!(result.failed_count > 0);
    }

    // MQR-Q3-7: Model identity bound from request
    #[test]
    fn test_model_identity_bound() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("output");
        let result = stage2.execute_probe(
            &response_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert_eq!(result.run_result.model_id, "minicpm5-1b-q4km");
        assert_eq!(result.run_result.model_sha256, "81B64D05A23B");
    }

    // MQR-Q3-8: Probe prompt overrides request task description
    #[test]
    fn test_probe_prompt_overrides() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("output");
        let result = stage2.execute_probe(
            &response_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        // The runner should have used the probe's prompt, not the request's
        assert_eq!(
            result.run_result.settings.task_description,
            "Say hello."
        );
    }

    // MQR-Q3-9: Stage 2 result validates
    #[test]
    fn test_result_validates() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.run(
            &test_probe_set(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert!(result.validate().is_ok());
    }

    // MQR-Q3-10: No capability data in stage 2 result
    #[test]
    fn test_no_capability_data() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.run(
            &test_probe_set(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert!(result.assert_no_capability_data().is_ok());
    }

    // MQR-Q3-11: Serialization round-trip
    #[test]
    fn test_serialization_round_trip() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.run(
            &test_probe_set(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        let json = result.to_json().unwrap();
        let parsed = Stage2PrimitiveProbeResult::from_json(&json).unwrap();
        assert_eq!(result, parsed);
    }

    // MQR-Q3-12: Hash is deterministic
    #[test]
    fn test_hash_deterministic() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.run(
            &test_probe_set(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        let h1 = result.compute_hash().unwrap();
        let h2 = result.compute_hash().unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    // MQR-Q3-13: Counts are correct
    #[test]
    fn test_counts_correct() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.run(
            &test_probe_set(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert_eq!(result.total_count, result.probe_results.len());
        assert_eq!(
            result.passed_count + result.failed_count,
            result.total_count
        );
    }

    // MQR-Q3-14: Evaluated timestamp is set
    #[test]
    fn test_evaluated_at_set() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.run(
            &test_probe_set(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert!(!result.evaluated_at.is_empty());
        assert!(result.evaluated_at.contains("T"));
    }

    // MQR-Q3-15: Single probe set with one probe
    #[test]
    fn test_single_probe_set() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello");
        let single_set = ProbeSet {
            set_id: "PS-SINGLE".to_string(),
            probes: vec![response_probe()],
            description: None,
        };
        let result = stage2.run(
            &single_set,
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert_eq!(result.total_count, 1);
        assert_eq!(result.passed_count, 1);
        assert_eq!(result.failed_count, 0);
        assert_eq!(result.verdict, ProbeVerdict::Pass);
    }

    // MQR-Q3-16: Empty probe set produces zero counts
    #[test]
    fn test_empty_probe_set() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello");
        let empty_set = ProbeSet {
            set_id: "PS-EMPTY".to_string(),
            probes: vec![],
            description: None,
        };
        let result = stage2.run(
            &empty_set,
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert_eq!(result.total_count, 0);
        assert_eq!(result.verdict, ProbeVerdict::Pass);
    }

    // MQR-Q3-17: Probe detail is human-readable
    #[test]
    fn test_probe_detail_readable() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.execute_probe(
            &response_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert!(!result.detail.is_empty());
        assert!(result.detail.contains("Passed") || result.detail.contains("Failed"));
    }

    // MQR-Q3-18: Probe type variants work
    #[test]
    fn test_probe_type_variants() {
        let response = ProbeType::PromptResponse;
        let json = ProbeType::StructuredOutput;
        let instruction = ProbeType::InstructionFollowing;

        assert_eq!(response, ProbeType::PromptResponse);
        assert_eq!(json, ProbeType::StructuredOutput);
        assert_eq!(instruction, ProbeType::InstructionFollowing);
    }

    // MQR-Q3-19: Custom probe with multiple rules
    #[test]
    fn test_multiple_rules() {
        let multi_rule_probe = PrimitiveProbe {
            probe_id: "PP-MULTI-001".to_string(),
            probe_type: ProbeType::PromptResponse,
            prompt: "Say hello.".to_string(),
            max_tokens: Some(64),
            temperature: Some(0.0),
            rules: vec![
                crate::qualification::validator_engine::ValidationRule {
                    rule_id: "VR-MULTI-1".to_string(),
                    rule_type: RuleType::MinTokens { min: 1 },
                    severity: RuleSeverity::Critical,
                    description: None,
                    params: None,
                },
                crate::qualification::validator_engine::ValidationRule {
                    rule_id: "VR-MULTI-2".to_string(),
                    rule_type: RuleType::ContainsSubstring,
                    severity: RuleSeverity::Critical,
                    description: None,
                    params: Some(r#""Hello""#.to_string()),
                },
            ],
            description: None,
        };

        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.execute_probe(
            &multi_rule_probe,
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert!(result.passed);
        assert_eq!(result.validation.rule_results.len(), 2);
    }

    // MQR-Q3-20: Warning rules don't block probe pass
    #[test]
    fn test_warning_rules_dont_block() {
        let warning_probe = PrimitiveProbe {
            probe_id: "PP-WARN-001".to_string(),
            probe_type: ProbeType::PromptResponse,
            prompt: "Say hello.".to_string(),
            max_tokens: Some(64),
            temperature: Some(0.0),
            rules: vec![crate::qualification::validator_engine::ValidationRule {
                rule_id: "VR-WARN-001".to_string(),
                rule_type: RuleType::ContainsSubstring,
                severity: RuleSeverity::Warning,
                description: None,
                params: Some(r#""xyznonexistent""#.to_string()),
            }],
            description: None,
        };

        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let result = stage2.execute_probe(
            &warning_probe,
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        // Warning doesn't block — only critical failures block
        assert!(result.passed);
        assert_eq!(result.validation.critical_failures, 0);
        assert!(result.validation.warnings > 0);
    }

    // MQR-Q3-21: Probe set with mixed pass/fail counts correctly
    #[test]
    fn test_mixed_counts() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello world");
        let mixed_set = ProbeSet {
            set_id: "PS-MIXED".to_string(),
            probes: vec![
                response_probe(),    // passes (MinTokens 1)
                failing_probe(),     // fails (ContainsSubstring "xyznonexistent")
                instruction_probe(), // fails (ContainsSubstring "hello" is case-sensitive)
                failing_probe(),     // fails (ContainsSubstring "xyznonexistent")
            ],
            description: None,
        };
        let result = stage2.run(
            &mixed_set,
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );

        assert_eq!(result.total_count, 4);
        assert_eq!(result.passed_count, 1);
        assert_eq!(result.failed_count, 3);
        assert_eq!(result.verdict, ProbeVerdict::Fail);
    }

    // MQR-Q3-22: All probe types execute through runner
    #[test]
    fn test_all_probe_types_execute() {
        let stage2 = Stage2PrimitiveProbe::new(
            std::path::PathBuf::from("/tmp/fixtures"),
            std::path::PathBuf::from("/tmp/rules"),
        );
        let executor = MockExecutor::success("Hello");

        // Response probe
        let r1 = stage2.execute_probe(
            &response_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );
        assert!(r1.run_result.state == RunState::Completed);

        // JSON probe
        let r2 = stage2.execute_probe(
            &json_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );
        assert!(r2.run_result.state == RunState::Completed);

        // Instruction probe
        let r3 = stage2.execute_probe(
            &instruction_probe(),
            &test_request(),
            &executor,
            9120,
            None,
            "vp-test",
        );
        assert!(r3.run_result.state == RunState::Completed);
    }
}
