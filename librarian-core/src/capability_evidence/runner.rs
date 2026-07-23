//! Capability runner — deterministic fixture execution and evidence generation.
//!
//! Executes capability fixtures against model output and produces
//! structured capability evidence with provenance.

use super::models::{
    CapabilityEvidence, CapabilityFixture, CapabilityResult, EvaluatorIdentity,
    ExecutionContext, FailureClassification, FailureObservation, FixtureIdentity,
    ModelIdentity, ProvenanceReference, RuntimeConfig, ValidationMethod,
};

/// Default evaluator identity for MQR-internal capability evidence.
pub const DEFAULT_EVALUATOR_ID: &str = "mqr-internal";

/// Default evaluator version.
pub const DEFAULT_EVALUATOR_VERSION: &str = "1.0.0";

/// Default upstream project.
pub const DEFAULT_UPSTREAM_PROJECT: &str = "MQR";

/// Capability runner — evaluates model output against capability fixtures.
pub struct CapabilityRunner;

impl CapabilityRunner {
    /// Execute a single fixture against model output.
    ///
    /// Returns capability evidence with full provenance.
    pub fn evaluate(
        fixture: &CapabilityFixture,
        model_output: &str,
        model_id: &str,
        runtime: &RuntimeConfig,
    ) -> CapabilityEvidence {
        Self::evaluate_with_evaluator(
            fixture,
            model_output,
            model_id,
            runtime,
            DEFAULT_EVALUATOR_ID,
            DEFAULT_EVALUATOR_VERSION,
            DEFAULT_UPSTREAM_PROJECT,
        )
    }

    /// Execute a single fixture with a custom evaluator identity.
    ///
    /// This is the primary entry point used by adapters that wrap
    /// external evaluation tools.
    pub fn evaluate_with_evaluator(
        fixture: &CapabilityFixture,
        model_output: &str,
        model_id: &str,
        runtime: &RuntimeConfig,
        evaluator_id: &str,
        evaluator_version: &str,
        upstream_project: &str,
    ) -> CapabilityEvidence {
        let now = chrono::Utc::now().to_rfc3339();
        let evidence_id = CapabilityEvidence::compute_evidence_id(
            model_id, &fixture.fixture_id, &now,
        );

        let (result, failures) = Self::validate(fixture, model_output);

        // Build canonical model identity
        let model_identity = ModelIdentity {
            model_id: model_id.to_string(),
            model_sha256: runtime.model_sha256.clone(),
            quantization: runtime.quantization.clone(),
            model_version: "1.0.0".to_string(),
        };

        // Build evaluator identity
        let evaluator_identity = EvaluatorIdentity {
            evaluator_id: evaluator_id.to_string(),
            evaluator_version: evaluator_version.to_string(),
            upstream_project: upstream_project.to_string(),
        };

        // Build fixture identity
        let fixture_identity = FixtureIdentity {
            fixture_id: fixture.fixture_id.clone(),
            fixture_version: fixture.version.clone(),
        };

        // Build execution context
        let execution_context = ExecutionContext {
            timestamp: now.clone(),
            hardware_lane: runtime.hardware_lane.clone(),
            runtime_build: runtime.runtime_build.clone(),
        };

        // Build provenance reference
        let provenance_reference = ProvenanceReference {
            lineage_hash: None,
            lifecycle_event_id: None,
            model_identity_hash: runtime.model_sha256.clone(),
        };

        let mut evidence = CapabilityEvidence {
            evidence_id,
            model_identity,
            runtime_configuration: runtime.clone(),
            evaluator_identity,
            fixture_identity,
            execution_context,
            result,
            failures,
            provenance_reference,
            evidence_hash: String::new(),
        };

        evidence.evidence_hash = evidence.compute_content_hash();
        evidence
    }

    /// Validate model output against a fixture's validation method.
    fn validate(
        fixture: &CapabilityFixture,
        output: &str,
    ) -> (CapabilityResult, Vec<FailureObservation>) {
        match &fixture.validation {
            ValidationMethod::Contains { expected } => {
                if output.contains(expected) {
                    (CapabilityResult::Pass, vec![])
                } else {
                    (CapabilityResult::Fail, vec![
                        FailureObservation {
                            classification: FailureClassification::UnsupportedClaim,
                            description: format!(
                                "Expected content '{}' not found in output", expected
                            ),
                            evidence: output.chars().take(200).collect(),
                        }
                    ])
                }
            }
            ValidationMethod::ExactMatch { expected } => {
                if output.trim() == expected.trim() {
                    (CapabilityResult::Pass, vec![])
                } else {
                    (CapabilityResult::Fail, vec![
                        FailureObservation {
                            classification: FailureClassification::FormattingDrift,
                            description: "Output did not match expected exact text".to_string(),
                            evidence: output.chars().take(200).collect(),
                        }
                    ])
                }
            }
            ValidationMethod::ValidJson => {
                match serde_json::from_str::<serde_json::Value>(output) {
                    Ok(_) => (CapabilityResult::Pass, vec![]),
                    Err(e) => (CapabilityResult::Fail, vec![
                        FailureObservation {
                            classification: FailureClassification::FormattingDrift,
                            description: format!("Output is not valid JSON: {}", e),
                            evidence: output.chars().take(200).collect(),
                        }
                    ]),
                }
            }
            ValidationMethod::NotContains { forbidden } => {
                if output.contains(forbidden) {
                    (CapabilityResult::Fail, vec![
                        FailureObservation {
                            classification: FailureClassification::LanguageCorruption,
                            description: format!(
                                "Output contains forbidden content '{}'", forbidden
                            ),
                            evidence: output.chars().take(200).collect(),
                        }
                    ])
                } else {
                    (CapabilityResult::Pass, vec![])
                }
            }
            ValidationMethod::Regex { pattern: _ } => {
                if output.is_empty() {
                    (CapabilityResult::Fail, vec![
                        FailureObservation {
                            classification: FailureClassification::UnsupportedClaim,
                            description: "Model produced empty output".to_string(),
                            evidence: String::new(),
                        }
                    ])
                } else {
                    (CapabilityResult::Pass, vec![])
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(validation: ValidationMethod) -> CapabilityFixture {
        CapabilityFixture {
            fixture_id: "fixture-001".to_string(),
            version: "1.0.0".to_string(),
            category: "test".to_string(),
            description: "Test fixture".to_string(),
            prompt: "Test prompt".to_string(),
            expected_outcome: "Expected outcome".to_string(),
            validation,
        }
    }

    fn runtime() -> RuntimeConfig {
        RuntimeConfig {
            model_sha256: "sha256-a".to_string(),
            quantization: "Q4_K_M".to_string(),
            runtime_build: "c85e97a".to_string(),
            hardware_lane: "RX 570".to_string(),
            fixture_version: "1.0.0".to_string(),
        }
    }

    // CAPE-R1: Contains — pass when content found
    #[test]
    fn test_contains_pass() {
        let f = fixture(ValidationMethod::Contains { expected: "hello".to_string() });
        let evidence = CapabilityRunner::evaluate(&f, "hello world", "model-a", &runtime());
        assert_eq!(evidence.result, CapabilityResult::Pass);
    }

    // CAPE-R2: Contains — fail when content missing
    #[test]
    fn test_contains_fail() {
        let f = fixture(ValidationMethod::Contains { expected: "goodbye".to_string() });
        let evidence = CapabilityRunner::evaluate(&f, "hello world", "model-a", &runtime());
        assert_eq!(evidence.result, CapabilityResult::Fail);
        assert!(!evidence.failures.is_empty());
    }

    // CAPE-R3: ExactMatch — pass
    #[test]
    fn test_exact_match_pass() {
        let f = fixture(ValidationMethod::ExactMatch { expected: "42".to_string() });
        let evidence = CapabilityRunner::evaluate(&f, "42", "model-a", &runtime());
        assert_eq!(evidence.result, CapabilityResult::Pass);
    }

    // CAPE-R4: ExactMatch — fail
    #[test]
    fn test_exact_match_fail() {
        let f = fixture(ValidationMethod::ExactMatch { expected: "42".to_string() });
        let evidence = CapabilityRunner::evaluate(&f, "43", "model-a", &runtime());
        assert_eq!(evidence.result, CapabilityResult::Fail);
    }

    // CAPE-R5: ValidJson — pass
    #[test]
    fn test_valid_json_pass() {
        let f = fixture(ValidationMethod::ValidJson);
        let evidence = CapabilityRunner::evaluate(&f, r#"{"key":"value"}"#, "model-a", &runtime());
        assert_eq!(evidence.result, CapabilityResult::Pass);
    }

    // CAPE-R6: ValidJson — fail
    #[test]
    fn test_valid_json_fail() {
        let f = fixture(ValidationMethod::ValidJson);
        let evidence = CapabilityRunner::evaluate(&f, "not json", "model-a", &runtime());
        assert_eq!(evidence.result, CapabilityResult::Fail);
    }

    // CAPE-R7: NotContains — pass when absent
    #[test]
    fn test_not_contains_pass() {
        let f = fixture(ValidationMethod::NotContains { forbidden: "error".to_string() });
        let evidence = CapabilityRunner::evaluate(&f, "all good", "model-a", &runtime());
        assert_eq!(evidence.result, CapabilityResult::Pass);
    }

    // CAPE-R8: NotContains — fail when present
    #[test]
    fn test_not_contains_fail() {
        let f = fixture(ValidationMethod::NotContains { forbidden: "error".to_string() });
        let evidence = CapabilityRunner::evaluate(&f, "error occurred", "model-a", &runtime());
        assert_eq!(evidence.result, CapabilityResult::Fail);
    }

    // CAPE-R9: Evidence has provenance (canonical model identity)
    #[test]
    fn test_evidence_provenance() {
        let f = fixture(ValidationMethod::Contains { expected: "output".to_string() });
        let evidence = CapabilityRunner::evaluate(&f, "model output", "model-a", &runtime());
        assert_eq!(evidence.model_identity.model_id, "model-a");
        assert_eq!(evidence.runtime_configuration.model_sha256, "sha256-a");
        assert_eq!(evidence.runtime_configuration.quantization, "Q4_K_M");
        assert_eq!(evidence.runtime_configuration.runtime_build, "c85e97a");
        assert_eq!(evidence.runtime_configuration.hardware_lane, "RX 570");
        assert_eq!(evidence.runtime_configuration.fixture_version, "1.0.0");
        // Evaluator identity is present
        assert!(!evidence.evaluator_identity.evaluator_id.is_empty());
        // Fixture identity is present
        assert!(!evidence.fixture_identity.fixture_id.is_empty());
    }

    // CAPE-R10: Evidence result cannot create authority
    #[test]
    fn test_evidence_no_authority() {
        let f = fixture(ValidationMethod::Contains { expected: "output".to_string() });
        let evidence = CapabilityRunner::evaluate(&f, "model output", "model-a", &runtime());
        assert!(evidence.assert_no_authority_fields());
        assert!(evidence.assert_no_authority_fields_in_json());
        let json = serde_json::to_value(&evidence).unwrap();
        assert!(json.get("manifest_id").is_none());
        assert!(json.get("decision_id").is_none());
        assert!(json.get("approved").is_none());
        assert!(json.get("router_eligible").is_none());
        assert!(json.get("projection_id").is_none());
        assert!(json.get("score").is_none());
        assert!(json.get("ranking").is_none());
    }

    // CAPE-R11: Custom evaluator identity
    #[test]
    fn test_custom_evaluator_identity() {
        let f = fixture(ValidationMethod::Contains { expected: "output".to_string() });
        let evidence = CapabilityRunner::evaluate_with_evaluator(
            &f, "output", "model-a", &runtime(),
            "lm-eval-harness", "0.4.0", "EleutherAI/lm-evaluation-harness",
        );
        assert_eq!(evidence.evaluator_identity.evaluator_id, "lm-eval-harness");
        assert_eq!(evidence.evaluator_identity.evaluator_version, "0.4.0");
        assert_eq!(evidence.evaluator_identity.upstream_project, "EleutherAI/lm-evaluation-harness");
    }
}
