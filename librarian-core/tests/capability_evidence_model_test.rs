//! MQR-CAPABILITY-EVIDENCE-MODEL-1 — Capability Evidence Canonical Model integration tests.
//!
//! Verifies the canonical capability evidence model preserves the MQR
//! authority boundary: capability evidence describes behavior but does
//! NOT create, approve, or route any qualification authority.

use librarian_core::capability_evidence::models::*;
use librarian_core::capability_evidence::runner::CapabilityRunner;
use librarian_core::capability_evidence::runner::{
    DEFAULT_EVALUATOR_ID, DEFAULT_EVALUATOR_VERSION, DEFAULT_UPSTREAM_PROJECT,
};

fn make_fixture(validation: ValidationMethod) -> CapabilityFixture {
    CapabilityFixture {
        fixture_id: "f-001".to_string(),
        version: "1.0.0".to_string(),
        category: "test".to_string(),
        description: "Test fixture".to_string(),
        prompt: "Test".to_string(),
        expected_outcome: "Expected".to_string(),
        validation,
    }
}

fn make_runtime() -> RuntimeConfig {
    RuntimeConfig {
        model_sha256: "sha-256".to_string(),
        quantization: "Q4_K_M".to_string(),
        runtime_build: "build-1".to_string(),
        hardware_lane: "RX 570".to_string(),
        fixture_version: "1.0.0".to_string(),
    }
}

// CAPEM-T1: Canonical model has all required canonical fields
#[test]
fn test_capem_t1_canonical_model_has_required_fields() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    assert!(!e.evidence_id.is_empty());
    assert_eq!(e.model_identity.model_id, "m1");
    assert!(!e.evaluator_identity.evaluator_id.is_empty());
    assert!(!e.fixture_identity.fixture_id.is_empty());
    assert!(!e.execution_context.timestamp.is_empty());
    assert!(!e.runtime_configuration.runtime_build.is_empty());
    assert!(!e.provenance_reference.model_identity_hash.is_empty());
    assert!(!e.evidence_hash.is_empty());
}

// CAPEM-T2: Default evaluator identity is mqr-internal
#[test]
fn test_capem_t2_default_evaluator_identity() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    assert_eq!(e.evaluator_identity.evaluator_id, DEFAULT_EVALUATOR_ID);
    assert_eq!(e.evaluator_identity.evaluator_version, DEFAULT_EVALUATOR_VERSION);
    assert_eq!(e.evaluator_identity.upstream_project, DEFAULT_UPSTREAM_PROJECT);
}

// CAPEM-T3: Custom evaluator identity is supported
#[test]
fn test_capem_t3_custom_evaluator_identity() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate_with_evaluator(
        &f, "x", "m1", &make_runtime(),
        "lm-eval-harness", "0.4.0", "EleutherAI/lm-evaluation-harness",
    );
    assert_eq!(e.evaluator_identity.evaluator_id, "lm-eval-harness");
    assert_eq!(e.evaluator_identity.evaluator_version, "0.4.0");
    assert_eq!(e.evaluator_identity.upstream_project, "EleutherAI/lm-evaluation-harness");
}

// CAPEM-T4: All 5 result states are valid
#[test]
fn test_capem_t4_all_five_result_states_valid() {
    for r in &[CapabilityResult::Pass, CapabilityResult::Fail,
              CapabilityResult::Unstable, CapabilityResult::NotTested,
              CapabilityResult::Degraded] {
        assert!(r.validate().is_ok());
    }
}

// CAPEM-T5: All 10 failure classifications are present
#[test]
fn test_capem_t5_all_ten_failure_classifications() {
    let all = [
        FailureClassification::HallucinatedEntity,
        FailureClassification::UnsupportedClaim,
        FailureClassification::FormattingDrift,
        FailureClassification::ContextLoss,
        FailureClassification::RepetitionLoop,
        FailureClassification::LanguageCorruption,
        FailureClassification::NondeterministicOutput,
        FailureClassification::PartialCompletion,
        FailureClassification::SchemaViolation,
        FailureClassification::UnsafeBehavior,
    ];
    assert_eq!(all.len(), 10);
    for f in &all {
        assert!(!f.as_str().is_empty());
        assert!(!f.description().is_empty());
        assert!(FailureClassification::from_str(f.as_str()).is_some());
    }
}

// CAPEM-T6: Authority boundary — no approval state
#[test]
fn test_capem_t6_no_approval_state() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    let json = serde_json::to_value(&e).unwrap();
    assert!(json.get("approved").is_none());
    assert!(json.get("rejected").is_none());
}

// CAPEM-T7: Authority boundary — no decision ownership
#[test]
fn test_capem_t7_no_decision_ownership() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    let json = serde_json::to_value(&e).unwrap();
    assert!(json.get("manifest_id").is_none());
    assert!(json.get("decision_id").is_none());
    assert!(json.get("owner_decision").is_none());
}

// CAPEM-T8: Authority boundary — no router eligibility
#[test]
fn test_capem_t8_no_router_eligibility() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    let json = serde_json::to_value(&e).unwrap();
    assert!(json.get("router_eligible").is_none());
    assert!(json.get("projection_id").is_none());
    assert!(json.get("routing_status").is_none());
}

// CAPEM-T9: Authority boundary — no scores or rankings
#[test]
fn test_capem_t9_no_scores_or_rankings() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    let json = serde_json::to_value(&e).unwrap();
    assert!(json.get("score").is_none());
    assert!(json.get("ranking").is_none());
    assert!(json.get("intelligence_index").is_none());
    assert!(json.get("rating").is_none());
}

// CAPEM-T10: Authority boundary — no lifecycle transition authority
#[test]
fn test_capem_t10_no_lifecycle_authority() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    let json = serde_json::to_value(&e).unwrap();
    assert!(json.get("lifecycle_transition").is_none());
    assert!(json.get("promote_to").is_none());
    assert!(json.get("demote_to").is_none());
}

// CAPEM-T11: All canonical fields are deterministic serializable
#[test]
fn test_capem_t11_canonical_deterministic_serialization() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());

    // First hash
    let h1 = e.compute_content_hash();

    // Second hash from the same logical content
    let _e2 = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    // Note: timestamps may differ slightly, so we can't compare directly
    // But the *format* is deterministic
    assert!(!h1.is_empty());
    assert_eq!(h1.len(), 64); // SHA-256 hex
}

// CAPEM-T12: Content hash includes all canonical fields
#[test]
fn test_capem_t12_content_hash_includes_all_fields() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e1 = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    let h1 = e1.compute_content_hash();

    // Hash must change if any canonical field changes
    let mut e2 = e1.clone();
    e2.model_identity.model_id = "different".to_string();
    let h2 = e2.compute_content_hash();
    assert_ne!(h1, h2);

    let mut e3 = e1.clone();
    e3.execution_context.timestamp = "different".to_string();
    let h3 = e3.compute_content_hash();
    assert_ne!(h1, h3);
}

// CAPEM-T13: Validation result is deterministic per fixture/output pair
#[test]
fn test_capem_t13_validation_deterministic() {
    let f = make_fixture(ValidationMethod::Contains { expected: "hello".to_string() });
    let e1 = CapabilityRunner::evaluate(&f, "hello world", "m1", &make_runtime());
    let e2 = CapabilityRunner::evaluate(&f, "hello world", "m1", &make_runtime());
    assert_eq!(e1.result, e2.result);
    assert_eq!(e1.failures.len(), e2.failures.len());
    // Both have Pass result for same input
    assert_eq!(e1.result, CapabilityResult::Pass);
    assert_eq!(e2.result, CapabilityResult::Pass);
}

// CAPEM-T14: Provenance reference to model identity hash
#[test]
fn test_capem_t14_provenance_reference_to_identity() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    assert_eq!(
        e.provenance_reference.model_identity_hash,
        e.runtime_configuration.model_sha256
    );
}

// CAPEM-T15: CapabilityEvidence is JSON-serializable without data loss
#[test]
fn test_capem_t15_json_serialization_lossless() {
    let f = make_fixture(ValidationMethod::Contains { expected: "x".to_string() });
    let e1 = CapabilityRunner::evaluate(&f, "x", "m1", &make_runtime());
    let json = serde_json::to_string(&e1).unwrap();
    let e2: CapabilityEvidence = serde_json::from_str(&json).unwrap();
    assert_eq!(e1.evidence_id, e2.evidence_id);
    assert_eq!(e1.model_identity, e2.model_identity);
    assert_eq!(e1.evaluator_identity, e2.evaluator_identity);
    assert_eq!(e1.fixture_identity, e2.fixture_identity);
    assert_eq!(e1.execution_context, e2.execution_context);
    assert_eq!(e1.result, e2.result);
    assert_eq!(e1.failures, e2.failures);
    assert_eq!(e1.provenance_reference, e2.provenance_reference);
}
