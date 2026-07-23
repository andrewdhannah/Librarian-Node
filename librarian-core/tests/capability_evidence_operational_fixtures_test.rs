//! MQR-CAPABILITY-EVIDENCE-OPERATIONAL-FIXTURES-1 — Operational fixtures tests.
//!
//! Verifies that the operational fixtures and runner produce canonical
//! capability evidence with correct failure classification, preserving
//! the authority boundary.

use librarian_core::capability_evidence::{
    CapabilityResult, FailureClassification, OperationalDomain, OperationalFixtures,
    OperationalRunner, RuntimeConfig,
};

fn make_runtime() -> RuntimeConfig {
    RuntimeConfig {
        model_sha256: "sha-256".to_string(),
        quantization: "Q4_K_M".to_string(),
        runtime_build: "build-1".to_string(),
        hardware_lane: "RX 570".to_string(),
        fixture_version: "1.0.0".to_string(),
    }
}

// OPF-T1: Operational fixtures cover all 5 domains
#[test]
fn test_opf_t1_all_domains_covered() {
    let all = OperationalFixtures::all();
    let domains: std::collections::HashSet<_> = all.iter().map(|f| f.category.clone()).collect();
    assert!(domains.contains("structured_output"));
    assert!(domains.contains("tool_interaction"));
    assert!(domains.contains("deterministic_replay"));
    assert!(domains.contains("factual_integrity"));
    assert!(domains.contains("context_preservation"));
}

// OPF-T2: Operational fixtures are versioned
#[test]
fn test_opf_t2_fixtures_versioned() {
    for f in OperationalFixtures::all() {
        assert!(!f.version.is_empty(), "Fixture {} has no version", f.fixture_id);
    }
}

// OPF-T3: Operational fixtures are deterministic
#[test]
fn test_opf_t3_fixtures_deterministic() {
    for f in OperationalFixtures::all() {
        assert!(!f.fixture_id.is_empty());
        assert!(!f.prompt.is_empty());
        assert!(!f.description.is_empty());
    }
}

// OPF-T4: Structured output runner passes for valid JSON
#[test]
fn test_opf_t4_structured_output_pass() {
    let evidence = OperationalRunner::evaluate(
        "m1",
        r#"{"id": 1, "name": "Alice", "status": "ok"}"#,
        &make_runtime(),
        &OperationalDomain::StructuredOutput,
    );
    assert_eq!(evidence.result, CapabilityResult::Pass);
    assert!(evidence.failures.is_empty());
}

// OPF-T5: Structured output runner fails for non-JSON
#[test]
fn test_opf_t5_structured_output_fail() {
    let evidence = OperationalRunner::evaluate(
        "m1",
        "this is not JSON at all",
        &make_runtime(),
        &OperationalDomain::StructuredOutput,
    );
    assert_eq!(evidence.result, CapabilityResult::Fail);
    assert!(!evidence.failures.is_empty());
}

// OPF-T6: Tool interaction runner passes for valid call
#[test]
fn test_opf_t6_tool_interaction_pass() {
    let evidence = OperationalRunner::evaluate(
        "m1",
        r#"{"name": "search", "arguments": {"q": "MQR"}}"#,
        &make_runtime(),
        &OperationalDomain::ToolInteraction,
    );
    assert_eq!(evidence.result, CapabilityResult::Pass);
}

// OPF-T7: Tool interaction runner fails for missing name
#[test]
fn test_opf_t7_tool_interaction_fail() {
    let evidence = OperationalRunner::evaluate(
        "m1",
        "I'll do it manually",
        &make_runtime(),
        &OperationalDomain::ToolInteraction,
    );
    assert_eq!(evidence.result, CapabilityResult::Fail);
}

// OPF-T8: Deterministic replay runner passes for exact output
#[test]
fn test_opf_t8_deterministic_replay_pass() {
    let evidence = OperationalRunner::evaluate(
        "m1", "4", &make_runtime(), &OperationalDomain::DeterministicReplay,
    );
    assert_eq!(evidence.result, CapabilityResult::Pass);
}

// OPF-T9: Deterministic replay runner fails for empty output
#[test]
fn test_opf_t9_deterministic_replay_fail() {
    let evidence = OperationalRunner::evaluate(
        "m1", "", &make_runtime(), &OperationalDomain::DeterministicReplay,
    );
    assert_eq!(evidence.result, CapabilityResult::Fail);
}

// OPF-T10: Factual integrity runner passes for correct answer
#[test]
fn test_opf_t10_factual_integrity_pass() {
    let evidence = OperationalRunner::evaluate(
        "m1", "Neil Armstrong", &make_runtime(), &OperationalDomain::FactualIntegrity,
    );
    assert_eq!(evidence.result, CapabilityResult::Pass);
}

// OPF-T11: Factual integrity runner fails for uncertain answer
#[test]
fn test_opf_t11_factual_integrity_fail() {
    let evidence = OperationalRunner::evaluate(
        "m1", "I think maybe someone", &make_runtime(), &OperationalDomain::FactualIntegrity,
    );
    assert_eq!(evidence.result, CapabilityResult::Fail);
}

// OPF-T12: Context preservation runner passes for correct context
#[test]
fn test_opf_t12_context_preservation_pass() {
    let evidence = OperationalRunner::evaluate(
        "m1", r#"{"project": "MQR"}"#, &make_runtime(), &OperationalDomain::ContextPreservation,
    );
    assert_eq!(evidence.result, CapabilityResult::Pass);
}

// OPF-T13: Context preservation runner fails for context loss
#[test]
fn test_opf_t13_context_preservation_fail() {
    let evidence = OperationalRunner::evaluate(
        "m1", r#"{"project": "something else"}"#, &make_runtime(), &OperationalDomain::ContextPreservation,
    );
    assert_eq!(evidence.result, CapabilityResult::Fail);
}

// OPF-T14: Operational evidence has full provenance
#[test]
fn test_opf_t14_evidence_has_full_provenance() {
    let evidence = OperationalRunner::evaluate(
        "m1", "4", &make_runtime(), &OperationalDomain::DeterministicReplay,
    );
    assert!(!evidence.evidence_id.is_empty());
    assert!(!evidence.model_identity.model_id.is_empty());
    assert!(!evidence.evaluator_identity.evaluator_id.is_empty());
    assert!(!evidence.fixture_identity.fixture_id.is_empty());
    assert!(!evidence.evidence_hash.is_empty());
    assert!(!evidence.provenance_reference.model_identity_hash.is_empty());
}

// OPF-T15: Failure classification maps to CAPE taxonomy
#[test]
fn test_opf_t15_failure_classification_taxonomy() {
    // Structured output fail
    let obs = OperationalFixtures::classify_failure(
        "not json", &OperationalDomain::StructuredOutput,
    );
    assert!(matches!(obs[0].classification, FailureClassification::SchemaViolation));

    // Tool interaction fail
    let obs = OperationalFixtures::classify_failure(
        "no tool call", &OperationalDomain::ToolInteraction,
    );
    assert!(matches!(obs[0].classification, FailureClassification::SchemaViolation));

    // Factual integrity fail
    let obs = OperationalFixtures::classify_failure(
        "I think maybe", &OperationalDomain::FactualIntegrity,
    );
    assert!(matches!(obs[0].classification, FailureClassification::UnsupportedClaim));

    // Context preservation fail
    let obs = OperationalFixtures::classify_failure(
        "wrong context", &OperationalDomain::ContextPreservation,
    );
    assert!(matches!(obs[0].classification, FailureClassification::ContextLoss));

    // Deterministic replay fail
    let obs = OperationalFixtures::classify_failure(
        "", &OperationalDomain::DeterministicReplay,
    );
    assert!(matches!(obs[0].classification, FailureClassification::NondeterministicOutput));
}

// OPF-T16: Operational evidence has no authority fields
#[test]
fn test_opf_t16_no_authority_fields() {
    let evidence = OperationalRunner::evaluate(
        "m1", "4", &make_runtime(), &OperationalDomain::DeterministicReplay,
    );
    let json = serde_json::to_value(&evidence).unwrap();
    assert!(json.get("manifest_id").is_none());
    assert!(json.get("decision_id").is_none());
    assert!(json.get("approved").is_none());
    assert!(json.get("router_eligible").is_none());
    assert!(json.get("score").is_none());
    assert!(json.get("ranking").is_none());
    assert!(json.get("intelligence_index").is_none());
}

// OPF-T17: Operational fixtures are reproducible
#[test]
fn test_opf_t17_fixtures_reproducible() {
    let a = OperationalFixtures::all();
    let b = OperationalFixtures::all();
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.fixture_id, y.fixture_id);
        assert_eq!(x.version, y.version);
        assert_eq!(x.category, y.category);
    }
}

// OPF-T18: No fixture creates scores or rankings
#[test]
fn test_opf_t18_no_scores_or_rankings() {
    let all = OperationalFixtures::all();
    for f in &all {
        let json = serde_json::to_value(f).unwrap();
        assert!(json.get("score").is_none());
        assert!(json.get("ranking").is_none());
        assert!(json.get("intelligence_index").is_none());
    }
}

// OPF-T19: Capability runner has no authority
#[test]
fn test_opf_t19_runner_authority_boundary() {
    let evidence = OperationalRunner::evaluate(
        "m1", "4", &make_runtime(), &OperationalDomain::DeterministicReplay,
    );
    assert!(evidence.assert_no_authority_fields());
    assert!(evidence.assert_no_authority_fields_in_json());
}

// OPF-T20: Each domain has at least one fixture
#[test]
fn test_opf_t20_each_domain_has_fixture() {
    let all = OperationalFixtures::all();
    let domains: std::collections::HashSet<_> = all.iter().map(|f| f.category.clone()).collect();
    let expected = vec![
        "structured_output",
        "tool_interaction",
        "deterministic_replay",
        "factual_integrity",
        "context_preservation",
    ];
    for d in expected {
        assert!(domains.contains(d), "Domain {} has no fixtures", d);
    }
}
