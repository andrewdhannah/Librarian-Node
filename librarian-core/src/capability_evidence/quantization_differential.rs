//! Quantization differential — controlled comparison of model variants.
//!
//! Compares the same model under different quantization configurations
//! (e.g., Q8_K_M vs Q4_K_M) using identical capability fixtures.
//!
//! The output is EVIDENCE about what changed between configurations:
//! - "Q4 demonstrated degraded factual integrity on FACT-002 under runtime X"
//! - "Q4 maintained structured output compliance on STRUCT-001"
//!
//! NOT a score or ranking.
//!
//! The differential is a descriptive artifact that exposes where
//! quantization affects capability demonstration, preserving Owner
//! authority over capability decisions.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::models::{
    CapabilityEvidence, CapabilityResult, EvaluatorIdentity, ExecutionContext, FixtureIdentity,
    ModelIdentity, ProvenanceReference, RuntimeConfig,
};

/// Configuration for a single model run in the differential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RunConfig {
    /// Model identifier (must be identical across differential).
    pub model_id: String,
    /// Model SHA-256 (must be identical across differential).
    pub model_sha256: String,
    /// Quantization variant (e.g., "Q8_K_M", "Q4_K_M").
    pub quantization: String,
    /// Runtime build.
    pub runtime_build: String,
    /// Hardware lane.
    pub hardware_lane: String,
    /// Optional label (e.g., "base" or "comparison").
    pub label: String,
}

/// Difference observation between two runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EvidenceDifference {
    /// Both runs produced the same result.
    NoChange,
    /// Base passed but comparison failed.
    BasePassComparisonFail,
    /// Base failed but comparison passed.
    BaseFailComparisonPass,
    /// Both failed but with different failure types.
    DifferentFailures,
    /// Both passed but comparison is degraded.
    BothPassComparisonDegraded,
    /// Comparison was not tested.
    ComparisonNotTested,
}

impl EvidenceDifference {
    /// Human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::NoChange => "Both runs produced the same result",
            Self::BasePassComparisonFail => "Base passed; comparison failed",
            Self::BaseFailComparisonPass => "Base failed; comparison passed",
            Self::DifferentFailures => "Both failed with different failure types",
            Self::BothPassComparisonDegraded => "Both passed but comparison is degraded",
            Self::ComparisonNotTested => "Comparison was not tested",
        }
    }

    /// Serialized string form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoChange => "no_change",
            Self::BasePassComparisonFail => "base_pass_comparison_fail",
            Self::BaseFailComparisonPass => "base_fail_comparison_pass",
            Self::DifferentFailures => "different_failures",
            Self::BothPassComparisonDegraded => "both_pass_comparison_degraded",
            Self::ComparisonNotTested => "comparison_not_tested",
        }
    }
}

/// Differential observation for a single fixture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureDifferential {
    /// Fixture ID being compared.
    pub fixture_id: String,
    /// Base evidence result.
    pub base_result: String,
    /// Comparison evidence result.
    pub comparison_result: String,
    /// Difference classification.
    pub difference: EvidenceDifference,
    /// Human-readable observation (no scores).
    pub observation: String,
    /// Whether base passed.
    pub base_passed: bool,
    /// Whether comparison passed.
    pub comparison_passed: bool,
}

/// Complete differential report for a quantization comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuantizationDifferential {
    /// Base configuration (e.g., Q8_K_M).
    pub base: RunConfig,
    /// Comparison configuration (e.g., Q4_K_M).
    pub comparison: RunConfig,
    /// Per-fixture differential observations.
    pub differentials: Vec<FixtureDifferential>,
    /// Deterministic content hash.
    pub content_hash: String,
    /// When the differential was generated.
    pub generated_at: String,
}

impl QuantizationDifferential {
    /// Compute a deterministic content hash.
    pub fn compute_content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        for d in &self.differentials {
            hasher.update(d.fixture_id.as_bytes());
            hasher.update(d.base_result.as_bytes());
            hasher.update(d.comparison_result.as_bytes());
            hasher.update(difference_to_bytes(&d.difference));
        }
        hasher.update(self.base.quantization.as_bytes());
        hasher.update(self.comparison.quantization.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Count of fixtures where the comparison degraded.
    pub fn degradation_count(&self) -> usize {
        self.differentials.iter().filter(|d| {
            matches!(d.difference,
                EvidenceDifference::BasePassComparisonFail
                | EvidenceDifference::BothPassComparisonDegraded
                | EvidenceDifference::ComparisonNotTested
            )
        }).count()
    }

    /// Count of fixtures where the comparison improved.
    pub fn improvement_count(&self) -> usize {
        self.differentials.iter().filter(|d| {
            matches!(d.difference, EvidenceDifference::BaseFailComparisonPass)
        }).count()
    }

    /// Count of fixtures with no change.
    pub fn no_change_count(&self) -> usize {
        self.differentials.iter().filter(|d| {
            matches!(d.difference, EvidenceDifference::NoChange)
        }).count()
    }

    /// Count of fixtures with different failure types.
    pub fn different_failure_count(&self) -> usize {
        self.differentials.iter().filter(|d| {
            matches!(d.difference, EvidenceDifference::DifferentFailures)
        }).count()
    }
}

fn difference_to_bytes(d: &EvidenceDifference) -> Vec<u8> {
    d.as_str().as_bytes().to_vec()
}

/// Tool for computing quantization differentials.
///
/// NOT a scoring system. Produces descriptive evidence about
/// capability demonstration differences.
pub struct QuantizationDifferentialTool;

impl QuantizationDifferentialTool {
    /// Compute a differential between two evidence sets.
    pub fn compute(
        base: RunConfig,
        comparison: RunConfig,
        base_evidence: &[CapabilityEvidence],
        comparison_evidence: &[CapabilityEvidence],
    ) -> QuantizationDifferential {
        assert_eq!(base.model_id, comparison.model_id,
                  "Differential requires identical model_id");
        assert_eq!(base.model_sha256, comparison.model_sha256,
                   "Differential requires identical model_sha256");

        let now = chrono::Utc::now().to_rfc3339();

        let mut differentials = Vec::new();
        let base_by_fixture = index_by_fixture(base_evidence);
        let comp_by_fixture = index_by_fixture(comparison_evidence);

        for (fixture_id, base_ev) in &base_by_fixture {
            let comp_ev = comp_by_fixture.get(fixture_id);
            let base_result = base_ev.result.as_str().to_string();
            let comparison_result = comp_ev
                .map(|e| e.result.as_str().to_string())
                .unwrap_or_else(|| "not_tested".to_string());
            let base_passed = base_ev.result.is_success();
            let comparison_passed = comp_ev.map(|e| e.result.is_success()).unwrap_or(false);

            let (difference, observation) = classify_difference(
                base_result.clone(),
                comparison_result.clone(),
            );

            differentials.push(FixtureDifferential {
                fixture_id: fixture_id.clone(),
                base_result,
                comparison_result,
                difference,
                observation,
                base_passed,
                comparison_passed,
            });
        }

        for (fixture_id, comp_ev) in &comp_by_fixture {
            if !base_by_fixture.contains_key(fixture_id) {
                let comparison_result = comp_ev.result.as_str().to_string();
                let observation = format!(
                    "Fixture {} was not in base run (model_id={}, quantization={})",
                    fixture_id, comp_ev.model_identity.model_id,
                    comp_ev.runtime_configuration.quantization
                );
                differentials.push(FixtureDifferential {
                    fixture_id: fixture_id.clone(),
                    base_result: "not_tested".to_string(),
                    comparison_result,
                    difference: EvidenceDifference::ComparisonNotTested,
                    observation,
                    base_passed: false,
                    comparison_passed: comp_ev.result.is_success(),
                });
            }
        }

        let mut differential = QuantizationDifferential {
            base,
            comparison,
            differentials,
            content_hash: String::new(),
            generated_at: now,
        };
        differential.content_hash = differential.compute_content_hash();
        differential
    }
}

fn index_by_fixture(evidence: &[CapabilityEvidence]) -> std::collections::HashMap<String, &CapabilityEvidence> {
    let mut map = std::collections::HashMap::new();
    for e in evidence {
        map.insert(e.fixture_identity.fixture_id.clone(), e);
    }
    map
}

fn classify_difference(base: String, comparison: String) -> (EvidenceDifference, String) {
    match (base.as_str(), comparison.as_str()) {
        ("pass", "pass") => (EvidenceDifference::NoChange,
            "Both configurations passed on this fixture".to_string()),
        ("fail", "fail") => (EvidenceDifference::NoChange,
            "Both configurations failed on this fixture (different failure modes possible)".to_string()),
        ("degraded", "degraded") => (EvidenceDifference::NoChange,
            "Both configurations showed degraded output".to_string()),
        ("not_tested", "not_tested") => (EvidenceDifference::NoChange,
            "Both configurations did not test this fixture".to_string()),
        ("unstable", _) => (EvidenceDifference::DifferentFailures,
            format!("Base showed unstable output; comparison showed {}", comparison)),
        (_, "unstable") => (EvidenceDifference::DifferentFailures,
            format!("Base showed {}; comparison showed unstable output", base)),
        ("pass", "fail") => (EvidenceDifference::BasePassComparisonFail,
            format!("Base passed; comparison failed under {}", comparison)),
        ("fail", "pass") => (EvidenceDifference::BaseFailComparisonPass,
            format!("Base failed; comparison passed under {}", comparison)),
        ("pass", "degraded") => (EvidenceDifference::BothPassComparisonDegraded,
            "Base passed cleanly; comparison produced degraded output".to_string()),
        ("degraded", "pass") => (EvidenceDifference::BothPassComparisonDegraded,
            "Base showed degraded output; comparison passed cleanly".to_string()),
        _ => (EvidenceDifference::DifferentFailures,
              format!("Base: {}, comparison: {}", base, comparison)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_base() -> RunConfig {
        RunConfig {
            model_id: "m1".to_string(),
            model_sha256: "sha-256".to_string(),
            quantization: "Q8_K_M".to_string(),
            runtime_build: "build-1".to_string(),
            hardware_lane: "RX 570".to_string(),
            label: "base".to_string(),
        }
    }

    fn make_comparison() -> RunConfig {
        RunConfig {
            model_id: "m1".to_string(),
            model_sha256: "sha-256".to_string(),
            quantization: "Q4_K_M".to_string(),
            runtime_build: "build-1".to_string(),
            hardware_lane: "RX 570".to_string(),
            label: "comparison".to_string(),
        }
    }

    fn make_evidence(fixture_id: &str, result_str: &str) -> CapabilityEvidence {
        CapabilityEvidence {
            evidence_id: format!("evt-{}", fixture_id),
            model_identity: ModelIdentity {
                model_id: "m1".to_string(),
                model_sha256: "sha-256".to_string(),
                quantization: "Q8_K_M".to_string(),
                model_version: "1.0.0".to_string(),
            },
            runtime_configuration: RuntimeConfig {
                model_sha256: "sha-256".to_string(),
                quantization: "Q8_K_M".to_string(),
                runtime_build: "build-1".to_string(),
                hardware_lane: "RX 570".to_string(),
                fixture_version: "1.0.0".to_string(),
            },
            evaluator_identity: EvaluatorIdentity {
                evaluator_id: "mqr".to_string(),
                evaluator_version: "1.0.0".to_string(),
                upstream_project: "MQR".to_string(),
            },
            fixture_identity: FixtureIdentity {
                fixture_id: fixture_id.to_string(),
                fixture_version: "1.0.0".to_string(),
            },
            execution_context: ExecutionContext {
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                hardware_lane: "RX 570".to_string(),
                runtime_build: "build-1".to_string(),
            },
            result: match result_str {
                "pass" => CapabilityResult::Pass,
                "fail" => CapabilityResult::Fail,
                "unstable" => CapabilityResult::Unstable,
                "not_tested" => CapabilityResult::NotTested,
                "degraded" => CapabilityResult::Degraded,
                _ => CapabilityResult::Fail,
            },
            failures: vec![],
            provenance_reference: ProvenanceReference {
                lineage_hash: None,
                lifecycle_event_id: None,
                model_identity_hash: "sha-256".to_string(),
            },
            evidence_hash: String::new(),
        }
    }

    #[test]
    fn test_qd_t1_deterministic_content_hash() {
        let d1 = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "pass")],
        );
        let d2 = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "pass")],
        );
        assert_eq!(d1.content_hash, d2.content_hash);
    }

    #[test]
    fn test_qd_t2_no_change_classification() {
        let d = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "pass")],
        );
        assert_eq!(d.differentials[0].difference, EvidenceDifference::NoChange);
        assert_eq!(d.no_change_count(), 1);
    }

    #[test]
    fn test_qd_t3_degradation_classification() {
        let d = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "degraded")],
        );
        assert_eq!(d.differentials[0].difference, EvidenceDifference::BothPassComparisonDegraded);
        assert_eq!(d.degradation_count(), 1);
    }

    #[test]
    fn test_qd_t4_improvement_classification() {
        let d = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "fail")],
            &[make_evidence("F-1", "pass")],
        );
        assert_eq!(d.differentials[0].difference, EvidenceDifference::BaseFailComparisonPass);
        assert_eq!(d.improvement_count(), 1);
    }

    #[test]
    fn test_qd_t5_failure_introduction() {
        let d = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "fail")],
        );
        assert_eq!(d.differentials[0].difference, EvidenceDifference::BasePassComparisonFail);
        assert_eq!(d.degradation_count(), 1);
    }

    #[test]
    fn test_qd_t6_authority_boundary() {
        let d = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "fail")],
        );
        let json = serde_json::to_value(&d).unwrap();
        assert!(json.get("score").is_none());
        assert!(json.get("ranking").is_none());
        assert!(json.get("winner").is_none());
        assert!(json.get("loser").is_none());
        assert!(json.get("percentage_diff").is_none());
        assert!(json.get("improvement_percent").is_none());
    }

    #[test]
    fn test_qd_t7_assertion_precondition() {
        let mut bad = make_base();
        bad.model_id = "different".to_string();
        let result = std::panic::catch_unwind(|| {
            QuantizationDifferentialTool::compute(
                bad, make_comparison(),
                &[make_evidence("F-1", "pass")],
                &[make_evidence("F-1", "pass")],
            )
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_qd_t8_observation_descriptive() {
        let d = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "fail")],
        );
        let obs = &d.differentials[0].observation;
        assert!(!obs.contains("12%"));
        assert!(!obs.contains("worse"));
        assert!(!obs.contains("better"));
        assert!(!obs.contains("score"));
    }

    #[test]
    fn test_qd_t9_deterministic_no_time_dependency() {
        let d1 = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "fail")],
        );
        let d2 = QuantizationDifferentialTool::compute(
            make_base(), make_comparison(),
            &[make_evidence("F-1", "pass")],
            &[make_evidence("F-1", "fail")],
        );
        assert_eq!(d1.content_hash, d2.content_hash);
    }
}
