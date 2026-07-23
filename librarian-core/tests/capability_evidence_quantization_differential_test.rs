//! MQR-CAPABILITY-EVIDENCE-QUANTIZATION-DIFFERENTIAL-1 — Differential comparison tests.
//!
//! Verifies that the quantization differential tool produces descriptive
//! evidence comparing model variants under identical capability fixtures,
//! preserving the authority boundary (no scores, no rankings).

use librarian_core::capability_evidence::{
    EvidenceDifference, QuantizationDifferential, QuantizationDifferentialTool, RunConfig,
};
use librarian_core::capability_evidence::models::{
    CapabilityEvidence, CapabilityResult, EvaluatorIdentity, ExecutionContext, FixtureIdentity,
    ModelIdentity, ProvenanceReference, RuntimeConfig,
};

fn make_config(quant: &str, label: &str) -> RunConfig {
    RunConfig {
        model_id: "m1".to_string(),
        model_sha256: "sha-256".to_string(),
        quantization: quant.to_string(),
        runtime_build: "build-1".to_string(),
        hardware_lane: "RX 570".to_string(),
        label: label.to_string(),
    }
}

fn make_evidence(fixture_id: &str, quant: &str, result_str: &str) -> CapabilityEvidence {
    CapabilityEvidence {
        evidence_id: format!("evt-{}", fixture_id),
        model_identity: ModelIdentity {
            model_id: "m1".to_string(),
            model_sha256: "sha-256".to_string(),
            quantization: quant.to_string(),
            model_version: "1.0.0".to_string(),
        },
        runtime_configuration: RuntimeConfig {
            model_sha256: "sha-256".to_string(),
            quantization: quant.to_string(),
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

// QD-T1: Differential preserves no_change for matching results
#[test]
fn test_qd_t1_no_change_preserved() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("STRUCT-001", "Q8_K_M", "pass")],
        &[make_evidence("STRUCT-001", "Q4_K_M", "pass")],
    );
    assert_eq!(d.differentials[0].difference, EvidenceDifference::NoChange);
    assert_eq!(d.no_change_count(), 1);
}

// QD-T2: Differential preserves degradation
#[test]
fn test_qd_t2_degradation_preserved() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("FACT-002", "Q8_K_M", "pass")],
        &[make_evidence("FACT-002", "Q4_K_M", "fail")],
    );
    assert_eq!(d.differentials[0].difference, EvidenceDifference::BasePassComparisonFail);
    assert_eq!(d.degradation_count(), 1);
}

// QD-T3: Differential preserves improvement
#[test]
fn test_qd_t3_improvement_preserved() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("FACT-001", "Q8_K_M", "fail")],
        &[make_evidence("FACT-001", "Q4_K_M", "pass")],
    );
    assert_eq!(d.differentials[0].difference, EvidenceDifference::BaseFailComparisonPass);
    assert_eq!(d.improvement_count(), 1);
}

// QD-T4: Differential preserves BothPassComparisonDegraded
#[test]
fn test_qd_t4_both_pass_comparison_degraded() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("CTX-001", "Q8_K_M", "pass")],
        &[make_evidence("CTX-001", "Q4_K_M", "degraded")],
    );
    assert_eq!(d.differentials[0].difference, EvidenceDifference::BothPassComparisonDegraded);
}

// QD-T5: Content hash is deterministic
#[test]
fn test_qd_t5_content_hash_deterministic() {
    let d1 = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q4_K_M", "fail")],
    );
    let d2 = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q4_K_M", "fail")],
    );
    assert_eq!(d1.content_hash, d2.content_hash);
}

// QD-T6: Authority boundary - no scores
#[test]
fn test_qd_t6_no_scores_in_json() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q4_K_M", "fail")],
    );
    let json = serde_json::to_value(&d).unwrap();
    assert!(json.get("score").is_none());
    assert!(json.get("ranking").is_none());
    assert!(json.get("winner").is_none());
    assert!(json.get("loser").is_none());
    assert!(json.get("percentage_diff").is_none());
    assert!(json.get("improvement_percent").is_none());
    assert!(json.get("score_difference").is_none());
    assert!(json.get("quality_score").is_none());
}

// QD-T7: Observation is descriptive, not numerical
#[test]
fn test_qd_t7_observation_descriptive() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("FACT-002", "Q8_K_M", "pass")],
        &[make_evidence("FACT-002", "Q4_K_M", "fail")],
    );
    let obs = &d.differentials[0].observation;
    assert!(!obs.contains("12%"));
    assert!(!obs.contains("worse"));
    assert!(!obs.contains("better"));
    assert!(!obs.contains("score"));
    assert!(!obs.contains("dropped"));
    assert!(!obs.contains("quality"));
}

// QD-T8: Counts match expectations
#[test]
fn test_qd_t8_counts_match() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[
            make_evidence("F-1", "Q8_K_M", "pass"),
            make_evidence("F-2", "Q8_K_M", "fail"),
            make_evidence("F-3", "Q8_K_M", "pass"),
        ],
        &[
            make_evidence("F-1", "Q4_K_M", "pass"),
            make_evidence("F-2", "Q4_K_M", "pass"),
            make_evidence("F-3", "Q4_K_M", "degraded"),
        ],
    );
    assert_eq!(d.no_change_count(), 1);
    assert_eq!(d.improvement_count(), 1);
    assert_eq!(d.degradation_count(), 1);
    assert_eq!(d.different_failure_count(), 0);
}

// QD-T9: Configurations are preserved
#[test]
fn test_qd_t9_configurations_preserved() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q4_K_M", "pass")],
    );
    assert_eq!(d.base.quantization, "Q8_K_M");
    assert_eq!(d.comparison.quantization, "Q4_K_M");
    assert_eq!(d.base.label, "base");
    assert_eq!(d.comparison.label, "comparison");
}

// QD-T10: Content hash includes quantization labels
#[test]
fn test_qd_t10_content_hash_includes_quantization() {
    let d_q8 = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q4_K_M", "pass")],
    );
    let d_q4 = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q5_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q5_K_M", "pass")],
    );
    assert_ne!(d_q8.content_hash, d_q4.content_hash);
}

// QD-T11: Multiple fixture differentials
#[test]
fn test_qd_t11_multiple_fixtures() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[
            make_evidence("STRUCT-001", "Q8_K_M", "pass"),
            make_evidence("FACT-002", "Q8_K_M", "pass"),
            make_evidence("CTX-001", "Q8_K_M", "pass"),
        ],
        &[
            make_evidence("STRUCT-001", "Q4_K_M", "pass"),
            make_evidence("FACT-002", "Q4_K_M", "degraded"),
            make_evidence("CTX-001", "Q4_K_M", "pass"),
        ],
    );
    assert_eq!(d.differentials.len(), 3);
    assert_eq!(d.no_change_count(), 2);
    assert_eq!(d.degradation_count(), 1);
}

// QD-T12: Comparison-only fixtures are marked
#[test]
fn test_qd_t12_comparison_only_fixture() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[
            make_evidence("F-1", "Q4_K_M", "pass"),
            make_evidence("F-2", "Q4_K_M", "pass"),
        ],
    );
    assert_eq!(d.differentials.len(), 2);
    let f2 = d.differentials.iter().find(|x| x.fixture_id == "F-2").unwrap();
    assert_eq!(f2.difference, EvidenceDifference::ComparisonNotTested);
}

// QD-T13: No rank, no winner
#[test]
fn test_qd_t13_no_rank_no_winner() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q4_K_M", "fail")],
    );
    let json = serde_json::to_value(&d).unwrap();
    assert!(json.get("winner_quantization").is_none());
    assert!(json.get("loser_quantization").is_none());
    assert!(json.get("recommended").is_none());
    assert!(json.get("approval").is_none());
    assert!(json.get("recommendation").is_none());
}

// QD-T14: No ability to mark as approved or rejected
#[test]
fn test_qd_t14_no_approval_or_rejection() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q4_K_M", "fail")],
    );
    let json = serde_json::to_value(&d).unwrap();
    assert!(json.get("approved").is_none());
    assert!(json.get("rejected").is_none());
    assert!(json.get("recommended").is_none());
    assert!(json.get("should_use_base").is_none());
    assert!(json.get("should_use_comparison").is_none());
}

// QD-T15: Generated_at is set
#[test]
fn test_qd_t15_generated_at_set() {
    let d = QuantizationDifferentialTool::compute(
        make_config("Q8_K_M", "base"),
        make_config("Q4_K_M", "comparison"),
        &[make_evidence("F-1", "Q8_K_M", "pass")],
        &[make_evidence("F-1", "Q4_K_M", "pass")],
    );
    assert!(!d.generated_at.is_empty());
    assert!(d.generated_at.contains("T"));
}
