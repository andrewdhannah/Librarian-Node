//! Operational runner — executes MQR operational fixtures and produces
//! canonical capability evidence with classified failure observations.

use super::models::{
    CapabilityEvidence, CapabilityResult, FailureObservation, RuntimeConfig,
};
use super::operational_fixtures::{OperationalDomain, OperationalFixtures};
use super::runner::CapabilityRunner;

/// Runner for MQR operational fixtures.
pub struct OperationalRunner;

impl OperationalRunner {
    /// Evaluate an operational fixture and produce classified evidence.
    ///
    /// Combines fixture execution (via CapabilityRunner) with domain-specific
    /// failure classification (via OperationalFixtures::classify_failure).
    pub fn evaluate(
        model_id: &str,
        model_output: &str,
        runtime: &RuntimeConfig,
        domain: &OperationalDomain,
    ) -> CapabilityEvidence {
        let fixtures = OperationalFixtures::all()
            .into_iter()
            .filter(|f| f.category == domain.to_string())
            .collect::<Vec<_>>();

        // Find the first fixture matching the domain and matching expected output
        // This is a simplified single-fixture eval; real impl would iterate all
        for fixture in &fixtures {
            // Use the canonical runner to get the base result
            let mut evidence = CapabilityRunner::evaluate(
                fixture, model_output, model_id, runtime,
            );

            // Override the result with domain-specific classification
            let additional_failures = OperationalFixtures::classify_failure(model_output, domain);
            if !additional_failures.is_empty() {
                // If the runner gave Pass but we found domain-specific issues, fail
                if evidence.result == CapabilityResult::Pass {
                    evidence.result = CapabilityResult::Fail;
                }
                evidence.failures.extend(additional_failures);
            }

            evidence.evaluator_identity.evaluator_id = "mqr-operational".to_string();
            evidence.evaluator_identity.evaluator_version = "1.0.0".to_string();
            evidence.evaluator_identity.upstream_project = "MQR-operational".to_string();
            evidence.fixture_identity.fixture_id = fixture.fixture_id.clone();
            evidence.fixture_identity.fixture_version = fixture.version.clone();
            evidence.evidence_hash = evidence.compute_content_hash();
            return evidence;
        }

        // Fallback: no fixture found
        let now = chrono::Utc::now().to_rfc3339();
        let evidence_id = CapabilityEvidence::compute_evidence_id(
            model_id, "operational-no-fixture", &now,
        );
        let mut evidence = CapabilityEvidence {
            evidence_id,
            model_identity: super::models::ModelIdentity {
                model_id: model_id.to_string(),
                model_sha256: runtime.model_sha256.clone(),
                quantization: runtime.quantization.clone(),
                model_version: "1.0.0".to_string(),
            },
            runtime_configuration: runtime.clone(),
            evaluator_identity: super::models::EvaluatorIdentity {
                evaluator_id: "mqr-operational".to_string(),
                evaluator_version: "1.0.0".to_string(),
                upstream_project: "MQR-operational".to_string(),
            },
            fixture_identity: super::models::FixtureIdentity {
                fixture_id: "operational-no-fixture".to_string(),
                fixture_version: "1.0.0".to_string(),
            },
            execution_context: super::models::ExecutionContext {
                timestamp: now.clone(),
                hardware_lane: runtime.hardware_lane.clone(),
                runtime_build: runtime.runtime_build.clone(),
            },
            result: CapabilityResult::NotTested,
            failures: vec![FailureObservation {
                classification: super::models::FailureClassification::UnsupportedClaim,
                description: "No operational fixture found for domain".to_string(),
                evidence: String::new(),
            }],
            provenance_reference: super::models::ProvenanceReference {
                lineage_hash: None,
                lifecycle_event_id: None,
                model_identity_hash: runtime.model_sha256.clone(),
            },
            evidence_hash: String::new(),
        };
        evidence.evidence_hash = evidence.compute_content_hash();
        evidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_runtime() -> RuntimeConfig {
        RuntimeConfig {
            model_sha256: "sha-256".to_string(),
            quantization: "Q4_K_M".to_string(),
            runtime_build: "build-1".to_string(),
            hardware_lane: "RX 570".to_string(),
            fixture_version: "1.0.0".to_string(),
        }
    }

    #[test]
    fn test_operational_runner_structured_output_pass() {
        let evidence = OperationalRunner::evaluate(
            "m1",
            r#"{"id": 1, "name": "Alice", "status": "ok"}"#,
            &make_runtime(),
            &OperationalDomain::StructuredOutput,
        );
        assert_eq!(evidence.result, CapabilityResult::Pass);
        assert!(evidence.failures.is_empty());
    }

    #[test]
    fn test_operational_runner_structured_output_fail() {
        let evidence = OperationalRunner::evaluate(
            "m1",
            "this is not JSON",
            &make_runtime(),
            &OperationalDomain::StructuredOutput,
        );
        assert_eq!(evidence.result, CapabilityResult::Fail);
        assert!(!evidence.failures.is_empty());
    }

    #[test]
    fn test_operational_runner_factual_pass() {
        let evidence = OperationalRunner::evaluate(
            "m1", "Neil Armstrong", &make_runtime(),
            &OperationalDomain::FactualIntegrity,
        );
        assert_eq!(evidence.result, CapabilityResult::Pass);
    }

    #[test]
    fn test_operational_runner_factual_fail() {
        let evidence = OperationalRunner::evaluate(
            "m1", "I think maybe someone", &make_runtime(),
            &OperationalDomain::FactualIntegrity,
        );
        assert_eq!(evidence.result, CapabilityResult::Fail);
    }

    #[test]
    fn test_operational_runner_authority_boundary() {
        let evidence = OperationalRunner::evaluate(
            "m1", "ok", &make_runtime(),
            &OperationalDomain::StructuredOutput,
        );
        let json = serde_json::to_value(&evidence).unwrap();
        assert!(json.get("manifest_id").is_none());
        assert!(json.get("decision_id").is_none());
        assert!(json.get("approved").is_none());
        assert!(json.get("router_eligible").is_none());
        assert!(json.get("score").is_none());
        assert!(json.get("ranking").is_none());
    }
}
