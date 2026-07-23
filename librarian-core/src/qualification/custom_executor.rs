//! Custom rule executor — bounded execution layer for custom validator rules.
//!
//! Custom rules are evaluated through a bounded executor that enforces:
//! - Explicit validator identity and versioning
//! - Deterministic inputs
//! - Timeout handling
//! - Failure isolation (panic containment)
//! - Structured validator evidence
//!
//! Critical invariants:
//!   Custom rule PASS ≠ capability approval
//!   Custom rule FAIL ≠ automatic rejection
//!   Custom rule result ≠ Owner decision
//!   Custom rule execution ≠ router mutation

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::panic;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::validator_engine::{RuleSeverity, ValidationRule};

/// Default timeout for custom rule execution (milliseconds).
pub const DEFAULT_CUSTOM_RULE_TIMEOUT_MS: u64 = 5000;

/// Custom rule definition with identity and versioning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomRuleDefinition {
    /// Rule identity (unique within validator pack).
    pub rule_id: String,

    /// Rule version (semantic version).
    pub version: String,

    /// Human-readable description.
    pub description: String,

    /// Rule severity if this custom rule fails.
    pub severity: RuleSeverity,

    /// Rule-specific parameters (arbitrary JSON).
    pub params: serde_json::Value,
}

impl CustomRuleDefinition {
    /// Compute a deterministic content hash for this definition.
    pub fn compute_content_hash(&self) -> Result<String> {
        let content = serde_json::json!({
            "rule_id": self.rule_id,
            "version": self.version,
            "description": self.description,
            "severity": self.severity.as_str(),
            "params": self.params,
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Execution context for a custom rule evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRuleContext {
    /// The model output to validate.
    pub output: String,

    /// Token count if available.
    pub token_count: Option<usize>,

    /// Task pack ID for provenance.
    pub task_pack_id: String,

    /// Additional execution context (JSON).
    pub context: serde_json::Value,
}

/// Outcome of a single custom rule execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomRuleOutcome {
    /// Whether the rule passed.
    pub passed: bool,

    /// Optional message explaining the outcome.
    pub message: Option<String>,

    /// Execution duration in milliseconds (None if timed out).
    pub execution_duration_ms: Option<u64>,

    /// Whether the execution timed out.
    pub timed_out: bool,

    /// Whether the execution panicked (failure isolation).
    pub panicked: bool,
}

/// Structured evidence from custom rule execution.
///
/// This is evidence — it is NOT an authority decision.
/// Persistence of this evidence does not create routing eligibility,
/// capability approval, or roster mutation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomRuleEvidence {
    /// Rule identity.
    pub rule_id: String,

    /// Rule version at time of execution.
    pub version: String,

    /// The outcome of execution.
    pub outcome: CustomRuleOutcome,

    /// Task pack ID.
    pub task_pack_id: String,

    /// Content hash for tamper detection.
    pub content_hash: String,

    /// When the execution occurred.
    pub executed_at: String,
}

impl CustomRuleEvidence {
    /// Compute content hash for tamper detection.
    pub fn compute_content_hash(&self) -> Result<String> {
        let outcome = &self.outcome;
        let content = serde_json::json!({
            "rule_id": self.rule_id,
            "version": self.version,
            "passed": outcome.passed,
            "timed_out": outcome.timed_out,
            "panicked": outcome.panicked,
            "task_pack_id": self.task_pack_id,
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Bounded custom rule executor.
///
/// Executes custom rules with timeout, panic isolation, and structured output.
/// The executor itself has no authority — it produces evidence only.
#[derive(Debug, Clone)]
pub struct CustomRuleExecutor {
    /// Timeout duration for each rule execution.
    timeout: Duration,
}

impl Default for CustomRuleExecutor {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(DEFAULT_CUSTOM_RULE_TIMEOUT_MS),
        }
    }
}

impl CustomRuleExecutor {
    /// Create a new executor with a custom timeout.
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// Get the current timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Execute a custom rule definition with the given context.
    ///
    /// This is the core bounded execution contract:
    /// - Inputs are deterministic (definition + context)
    /// - Execution is time-bounded
    /// - Panics are caught and isolated
    /// - Output is structured evidence (no authority)
    pub fn execute(
        &self,
        definition: &CustomRuleDefinition,
        context: &CustomRuleContext,
    ) -> CustomRuleEvidence {
        let _start = std::time::Instant::now();
        let outcome = self.execute_bounded(definition, context);
        let now = chrono::Utc::now().to_rfc3339();

        let mut evidence = CustomRuleEvidence {
            rule_id: definition.rule_id.clone(),
            version: definition.version.clone(),
            outcome,
            task_pack_id: context.task_pack_id.clone(),
            content_hash: String::new(),
            executed_at: now,
        };

        evidence.content_hash = evidence.compute_content_hash().unwrap_or_default();
        evidence
    }

    /// Core bounded execution with timeout and panic isolation.
    fn execute_bounded(
        &self,
        definition: &CustomRuleDefinition,
        context: &CustomRuleContext,
    ) -> CustomRuleOutcome {
        // Panic-isolated evaluation
        let panic_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.evaluate_custom_rule(definition, context)
        }));

        match panic_result {
            Ok(outcome) => outcome,
            Err(_) => {
                // Panic caught at the outermost level
                CustomRuleOutcome {
                    passed: false,
                    message: Some(format!(
                        "Custom rule '{}' panicked during execution",
                        definition.rule_id
                    )),
                    execution_duration_ms: None,
                    timed_out: false,
                    panicked: true,
                }
            }
        }
    }

    /// Evaluate a custom rule with timeout, running in a separate thread.
    ///
    /// Returns Result<CustomRuleOutcome, String> where:
    /// - Ok(outcome): normal completion or inner panic (caught by catch_unwind inside the thread)
    /// - Err(reason): timeout
    fn evaluate_custom_rule(
        &self,
        definition: &CustomRuleDefinition,
        context: &CustomRuleContext,
    ) -> CustomRuleOutcome {
        // Use channel-based timeout with inner panic isolation
        let def_clone = definition.clone();
        let ctx_clone = context.clone();
        let timeout = self.timeout;

        let (tx, rx) = mpsc::channel::<std::thread::Result<CustomRuleOutcome>>();

        thread::spawn(move || {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                Self::evaluate_inner(&def_clone, &ctx_clone)
            }));
            let _ = tx.send(result);
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(outcome)) => {
                // Normal completion — measure elapsed time
                // (We can't measure precisely here since the thread already finished)
                outcome
            }
            Ok(Err(_panic_info)) => {
                // Inner panic was caught inside the spawned thread
                CustomRuleOutcome {
                    passed: false,
                    message: Some(format!(
                        "Custom rule '{}' panicked during execution",
                        definition.rule_id
                    )),
                    execution_duration_ms: None,
                    timed_out: false,
                    panicked: true,
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timed out
                CustomRuleOutcome {
                    passed: false,
                    message: Some(format!(
                        "Custom rule '{}' timed out after {}ms",
                        definition.rule_id,
                        timeout.as_millis()
                    )),
                    execution_duration_ms: None,
                    timed_out: true,
                    panicked: false,
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Channel disconnected (thread terminated abnormally without sending)
                CustomRuleOutcome {
                    passed: false,
                    message: Some(format!(
                        "Custom rule '{}' terminated abnormally",
                        definition.rule_id
                    )),
                    execution_duration_ms: None,
                    timed_out: false,
                    panicked: true,
                }
            }
        }
    }

    /// Pure evaluation of a custom rule definition against context.
    ///
    /// Built-in evaluation strategies for common patterns.
    /// This is the deterministic core — same inputs always produce same output.
    fn evaluate_inner(
        definition: &CustomRuleDefinition,
        context: &CustomRuleContext,
    ) -> CustomRuleOutcome {
        // Check params for evaluation strategy hints
        let strategy = definition.params.get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("pass");

        match strategy {
            // Contains substring check
            "contains" => {
                let target = definition.params.get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let passed = context.output.contains(target);
                CustomRuleOutcome {
                    passed,
                    message: if passed {
                        None
                    } else {
                        Some(format!("Output does not contain '{}'", target))
                    },
                    execution_duration_ms: None,
                    timed_out: false,
                    panicked: false,
                }
            }

            // Minimum token count
            "min_tokens" => {
                let min = definition.params.get("min")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let passed = context.token_count.map_or(false, |c| c >= min);
                CustomRuleOutcome {
                    passed,
                    message: if passed {
                        None
                    } else {
                        let actual = context.token_count.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                        Some(format!("Token count {} < minimum {}", actual, min))
                    },
                    execution_duration_ms: None,
                    timed_out: false,
                    panicked: false,
                }
            }

            // Always pass
            "pass" => {
                CustomRuleOutcome {
                    passed: true,
                    message: None,
                    execution_duration_ms: None,
                    timed_out: false,
                    panicked: false,
                }
            }

            // Always fail (for testing)
            "fail" => {
                CustomRuleOutcome {
                    passed: false,
                    message: Some(format!(
                        "Custom rule '{}' always fails (strategy: fail)",
                        definition.rule_id
                    )),
                    execution_duration_ms: None,
                    timed_out: false,
                    panicked: false,
                }
            }

            // Panic on purpose (for testing isolation)
            "panic" => {
                panic!("Custom rule '{}' intentionally panicked", definition.rule_id);
            }

            // Infinite loop (for testing timeout)
            "hang" => {
                loop {
                    // Busy-wait until timeout
                    std::thread::sleep(Duration::from_millis(100));
                }
            }

            // Unknown strategy → pass with warning
            _ => {
                CustomRuleOutcome {
                    passed: true,
                    message: Some(format!(
                        "Unknown strategy '{}' for custom rule '{}' — defaulting to pass",
                        strategy, definition.rule_id
                    )),
                    execution_duration_ms: None,
                    timed_out: false,
                    panicked: false,
                }
            }
        }
    }
}

/// Convert a ValidationRule (with Custom type) to a CustomRuleDefinition.
pub fn validation_rule_to_definition(rule: &ValidationRule) -> Option<CustomRuleDefinition> {
    match &rule.rule_type {
        super::validator_engine::RuleType::Custom(_) => {
            let params: serde_json::Value = rule
                .params
                .as_deref()
                .and_then(|p| serde_json::from_str(p).ok())
                .unwrap_or(serde_json::json!({}));

            Some(CustomRuleDefinition {
                rule_id: rule.rule_id.clone(),
                version: params.get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.1.0")
                    .to_string(),
                description: rule.description.clone().unwrap_or_default(),
                severity: rule.severity.clone(),
                params,
            })
        }
        _ => None,
    }
}

/// Apply custom rules to a qualification run result.
///
/// Each rule is executed against the run's raw output and captured
/// token count. The resulting evidence is appended to the run result's
/// `custom_evidence` list, sorted by rule_id for deterministic ordering.
///
/// This is a pure evidence-collection operation:
/// - It does NOT create capability manifests
/// - It does NOT approve or reject capabilities
/// - It does NOT mutate router state
/// - It does NOT alter Owner decisions
/// - It does NOT bypass qualification gates
pub fn apply_custom_rules(
    run_result: &mut crate::qualification::run_result::QualificationRunResult,
    executor: &CustomRuleExecutor,
    definitions: &[CustomRuleDefinition],
) {
    let context = CustomRuleContext {
        output: run_result
            .raw_output
            .as_deref()
            .unwrap_or("")
            .to_string(),
        token_count: run_result.telemetry.output_tokens.map(|t| t as usize),
        task_pack_id: run_result.task_pack_id.clone(),
        context: serde_json::json!({}),
    };

    for def in definitions {
        let evidence = executor.execute(def, &context);
        run_result.custom_evidence.push(evidence);
    }

    // Sort by rule_id for deterministic ordering
    run_result
        .custom_evidence
        .sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn test_executor() -> CustomRuleExecutor {
        CustomRuleExecutor::new(5000)
    }

    fn test_context(output: &str, token_count: Option<usize>) -> CustomRuleContext {
        CustomRuleContext {
            output: output.to_string(),
            token_count,
            task_pack_id: "tp-h4-test".to_string(),
            context: serde_json::json!({}),
        }
    }

    // H4-U1: Content hash is deterministic
    #[test]
    fn test_definition_content_hash_deterministic() {
        let def = CustomRuleDefinition {
            rule_id: "CR-001".to_string(),
            version: "1.0.0".to_string(),
            description: "Check output contains expected keyword".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "contains", "target": "expected"}),
        };
        let h1 = def.compute_content_hash().unwrap();
        let h2 = def.compute_content_hash().unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    // H4-U2: Content hash changes with different definition
    #[test]
    fn test_definition_content_hash_not_trivial() {
        let def1 = CustomRuleDefinition {
            rule_id: "CR-001".to_string(),
            version: "1.0.0".to_string(),
            description: "desc".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "pass"}),
        };
        let def2 = CustomRuleDefinition {
            rule_id: "CR-002".to_string(),
            version: "1.0.0".to_string(),
            description: "desc".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "pass"}),
        };
        assert_ne!(def1.compute_content_hash().unwrap(), def2.compute_content_hash().unwrap());
    }

    // H4-U3: Default executor timeout is reasonable
    #[test]
    fn test_default_timeout() {
        let executor = CustomRuleExecutor::default();
        assert_eq!(executor.timeout(), Duration::from_millis(DEFAULT_CUSTOM_RULE_TIMEOUT_MS));
    }

    // H4-U4: Custom executor with custom timeout
    #[test]
    fn test_custom_timeout() {
        let executor = CustomRuleExecutor::new(1000);
        assert_eq!(executor.timeout(), Duration::from_millis(1000));
    }

    // H4-U5: "pass" strategy produces pass
    #[test]
    fn test_pass_strategy() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-PASS".to_string(),
            version: "1.0.0".to_string(),
            description: "Always pass".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "pass"}),
        };
        let evidence = executor.execute(&def, &test_context("any output", None));
        assert!(evidence.outcome.passed);
        assert!(!evidence.outcome.timed_out);
        assert!(!evidence.outcome.panicked);
    }

    // H4-U6: "fail" strategy produces fail
    #[test]
    fn test_fail_strategy() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-FAIL".to_string(),
            version: "1.0.0".to_string(),
            description: "Always fail".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "fail"}),
        };
        let evidence = executor.execute(&def, &test_context("any output", None));
        assert!(!evidence.outcome.passed);
        assert!(!evidence.outcome.timed_out);
        assert!(!evidence.outcome.panicked);
    }

    // H4-U7: "contains" strategy passes when target present
    #[test]
    fn test_contains_strategy_pass() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-CONTAINS".to_string(),
            version: "1.0.0".to_string(),
            description: "Check contains".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "contains", "target": "expected_keyword"}),
        };
        let evidence = executor.execute(&def, &test_context("output with expected_keyword inside", None));
        assert!(evidence.outcome.passed);
    }

    // H4-U8: "contains" strategy fails when target absent
    #[test]
    fn test_contains_strategy_fail() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-CONTAINS".to_string(),
            version: "1.0.0".to_string(),
            description: "Check contains".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "contains", "target": "missing_keyword"}),
        };
        let evidence = executor.execute(&def, &test_context("output without the keyword", None));
        assert!(!evidence.outcome.passed);
    }

    // H4-U9: "min_tokens" passes when sufficient
    #[test]
    fn test_min_tokens_strategy_pass() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-MINTOKENS".to_string(),
            version: "1.0.0".to_string(),
            description: "Min tokens".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "min_tokens", "min": 10}),
        };
        let evidence = executor.execute(&def, &test_context("output", Some(15)));
        assert!(evidence.outcome.passed);
    }

    // H4-U10: "min_tokens" fails when insufficient
    #[test]
    fn test_min_tokens_strategy_fail() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-MINTOKENS".to_string(),
            version: "1.0.0".to_string(),
            description: "Min tokens".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "min_tokens", "min": 10}),
        };
        let evidence = executor.execute(&def, &test_context("output", Some(5)));
        assert!(!evidence.outcome.passed);
    }

    // H4-U11: Panic isolation — panic caught without crashing test
    #[test]
    fn test_panic_isolation() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-PANIC".to_string(),
            version: "1.0.0".to_string(),
            description: "Intentionally panics".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "panic"}),
        };
        let evidence = executor.execute(&def, &test_context("output", None));
        assert!(!evidence.outcome.passed);
        assert!(evidence.outcome.panicked);
    }

    // H4-U12: Timeout detection
    #[test]
    fn test_timeout_detection() {
        // Very short timeout (100ms) for hang strategy
        let short_executor = CustomRuleExecutor::new(100);
        let def = CustomRuleDefinition {
            rule_id: "CR-HANG".to_string(),
            version: "1.0.0".to_string(),
            description: "Intentionally hangs".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "hang"}),
        };
        let start = Instant::now();
        let evidence = short_executor.execute(&def, &test_context("output", None));
        let elapsed = start.elapsed().as_millis();

        // Should not have waited more than ~2x timeout
        assert!(elapsed < 2000, "Timeout took too long: {}ms", elapsed);

        assert!(!evidence.outcome.passed);
        assert!(evidence.outcome.timed_out);
        assert!(!evidence.outcome.panicked);
    }

    // H4-U13: Evidence content hash is deterministic
    #[test]
    fn test_evidence_hash_deterministic() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-HASH".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "pass"}),
        };
        let e1 = executor.execute(&def, &test_context("output", None));
        let e2 = executor.execute(&def, &test_context("output", None));
        // Content hash is based on outcome + identity
        // Same rule + same context → same outcome → same hash
        // But the executed_at timestamp differs, so content_hash is computed
        // from outcome-only fields (not timestamp). Let's verify.
        let h1 = e1.compute_content_hash().unwrap();
        let h2 = e2.compute_content_hash().unwrap();
        assert_eq!(h1, h2);
    }

    // H4-U14: Evidence content hash changes with different outcome
    #[test]
    fn test_evidence_hash_tamper_detection() {
        let mut evidence = CustomRuleEvidence {
            rule_id: "CR-TEST".to_string(),
            version: "1.0.0".to_string(),
            outcome: CustomRuleOutcome {
                passed: true,
                message: None,
                execution_duration_ms: Some(10),
                timed_out: false,
                panicked: false,
            },
            task_pack_id: "tp-001".to_string(),
            content_hash: String::new(),
            executed_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let original_hash = evidence.compute_content_hash().unwrap();

        // Tamper with outcome
        evidence.outcome.passed = false;
        let tampered_hash = evidence.compute_content_hash().unwrap();
        assert_ne!(original_hash, tampered_hash);
    }

    // H4-U15: Serialization round-trip for evidence
    #[test]
    fn test_evidence_serialization_roundtrip() {
        let evidence = CustomRuleEvidence {
            rule_id: "CR-SER".to_string(),
            version: "1.0.0".to_string(),
            outcome: CustomRuleOutcome {
                passed: true,
                message: Some("All good".to_string()),
                execution_duration_ms: Some(42),
                timed_out: false,
                panicked: false,
            },
            task_pack_id: "tp-001".to_string(),
            content_hash: "abc123".to_string(),
            executed_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&evidence).unwrap();
        let parsed: CustomRuleEvidence = serde_json::from_str(&json).unwrap();

        assert_eq!(evidence.rule_id, parsed.rule_id);
        assert_eq!(evidence.version, parsed.version);
        assert_eq!(evidence.outcome, parsed.outcome);
        assert_eq!(evidence.task_pack_id, parsed.task_pack_id);
    }

    // H4-U16: validation_rule_to_definition extracts version from params
    #[test]
    fn test_validation_rule_to_definition_with_version() {
        let rule = ValidationRule {
            rule_id: "CR-001".to_string(),
            rule_type: super::super::validator_engine::RuleType::Custom("checker".to_string()),
            severity: RuleSeverity::Critical,
            description: Some("My custom check".to_string()),
            params: Some(r#"{"strategy":"pass","version":"2.0.0"}"#.to_string()),
        };
        let def = validation_rule_to_definition(&rule).unwrap();
        assert_eq!(def.rule_id, "CR-001");
        assert_eq!(def.version, "2.0.0");
        assert_eq!(def.severity, RuleSeverity::Critical);
    }

    // H4-U17: validation_rule_to_definition defaults version
    #[test]
    fn test_validation_rule_to_definition_default_version() {
        let rule = ValidationRule {
            rule_id: "CR-002".to_string(),
            rule_type: super::super::validator_engine::RuleType::Custom("checker".to_string()),
            severity: RuleSeverity::Warning,
            description: None,
            params: None,
        };
        let def = validation_rule_to_definition(&rule).unwrap();
        assert_eq!(def.version, "0.1.0");
        assert_eq!(def.severity, RuleSeverity::Warning);
    }

    // H4-U18: validate_rule_to_definition returns None for non-custom rules
    #[test]
    fn test_validation_rule_to_definition_non_custom() {
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: super::super::validator_engine::RuleType::ContainsSubstring,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""test""#.to_string()),
        };
        assert!(validation_rule_to_definition(&rule).is_none());
    }

    // H4-U19: Unknown strategy defaults to pass
    #[test]
    fn test_unknown_strategy_defaults_to_pass() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-UNKNOWN".to_string(),
            version: "1.0.0".to_string(),
            description: "Unknown strategy".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "nonexistent"}),
        };
        let evidence = executor.execute(&def, &test_context("output", None));
        assert!(evidence.outcome.passed);
        assert!(evidence.outcome.message.is_some());
    }

    // H4-U20: Deterministic — same definition + same context = same outcome
    #[test]
    fn test_deterministic_execution() {
        let executor = test_executor();
        let def = CustomRuleDefinition {
            rule_id: "CR-DET".to_string(),
            version: "1.0.0".to_string(),
            description: "Deterministic".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "contains", "target": "hello"}),
        };
        let ctx = test_context("hello world", Some(100));
        let e1 = executor.execute(&def, &ctx);
        let e2 = executor.execute(&def, &ctx);
        assert_eq!(e1.outcome.passed, e2.outcome.passed);
    }
}
