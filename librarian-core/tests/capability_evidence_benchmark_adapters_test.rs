//! MQR-CAPABILITY-EVIDENCE-STANDARD-BENCHMARK-ADAPTERS-1 — Standard benchmark adapter tests.
//!
//! Verifies that the lm-evaluation-harness and code-needle adapters
//! integrate correctly with the canonical capability evidence system
//! without creating authority.

use librarian_core::capability_evidence::{
    AdapterRegistry, AdapterError, CapabilityResult, EvaluatorAdapter,
};
use librarian_core::capability_evidence::code_needle_adapter::CodeNeedleAdapter;
use librarian_core::capability_evidence::lm_eval_adapter::LMEvalHarnessAdapter;

// SBADAPTER-T1: LMEvalHarnessAdapter identity
#[test]
fn test_sbadapter_t1_lm_eval_identity() {
    let a = LMEvalHarnessAdapter::new("0.4.0");
    assert_eq!(a.evaluator_id(), "lm-eval-harness");
    assert_eq!(a.evaluator_version(), "0.4.0");
    assert_eq!(a.upstream_project(), "EleutherAI/lm-evaluation-harness");
    assert!(a.fixture_count() > 0);
}

// SBADAPTER-T2: LMEvalHarnessAdapter fixtures are deterministic
#[test]
fn test_sbadapter_t2_lm_eval_deterministic() {
    let a = LMEvalHarnessAdapter::new("0.4.0");
    for i in 0..a.fixture_count() {
        let f1 = a.fixture_at(i).unwrap();
        let f2 = a.fixture_at(i).unwrap();
        assert_eq!(f1.fixture_id, f2.fixture_id);
        assert_eq!(f1.category, f2.category);
    }
}

// SBADAPTER-T3: LMEvalHarnessAdapter out-of-bounds returns AdapterError
#[test]
fn test_sbadapter_t3_lm_eval_out_of_bounds() {
    let a = LMEvalHarnessAdapter::new("0.4.0");
    let err = a.fixture_at(1000).unwrap_err();
    match err {
        AdapterError::FixtureIndexOutOfBounds { evaluator_id, index, total } => {
            assert_eq!(evaluator_id, "lm-eval-harness");
            assert_eq!(index, 1000);
            assert_eq!(total, a.fixture_count());
        }
        _ => panic!("Expected FixtureIndexOutOfBounds"),
    }
}

// SBADAPTER-T4: LMEvalHarnessAdapter result is a valid capability state
#[test]
fn test_sbadapter_t4_lm_eval_result_is_valid_state() {
    let a = LMEvalHarnessAdapter::new("0.4.0");
    let f = a.fixture_at(0).unwrap();
    let r1 = a.evaluate_fixture(&f, "expected answer here");
    let r2 = a.evaluate_fixture(&f, "wrong answer");
    assert!(matches!(r1, CapabilityResult::Pass | CapabilityResult::Fail
                   | CapabilityResult::Unstable | CapabilityResult::NotTested
                   | CapabilityResult::Degraded));
    assert!(matches!(r2, CapabilityResult::Pass | CapabilityResult::Fail
                   | CapabilityResult::Unstable | CapabilityResult::NotTested
                   | CapabilityResult::Degraded));
}

// SBADAPTER-T5: CodeNeedleAdapter identity
#[test]
fn test_sbadapter_t5_code_needle_identity() {
    let a = CodeNeedleAdapter::new("1.0.0");
    assert_eq!(a.evaluator_id(), "code-needle");
    assert_eq!(a.evaluator_version(), "1.0.0");
    assert_eq!(a.upstream_project(), "MQR-code-needle");
    assert!(a.fixture_count() > 0);
}

// SBADAPTER-T6: CodeNeedleAdapter fixtures are deterministic
#[test]
fn test_sbadapter_t6_code_needle_deterministic() {
    let a = CodeNeedleAdapter::new("1.0.0");
    for i in 0..a.fixture_count() {
        let f1 = a.fixture_at(i).unwrap();
        let f2 = a.fixture_at(i).unwrap();
        assert_eq!(f1.fixture_id, f2.fixture_id);
    }
}

// SBADAPTER-T7: Both adapters register in a single registry
#[test]
fn test_sbadapter_t7_both_adapters_in_registry() {
    let mut registry = AdapterRegistry::new();
    assert!(registry.register(Box::new(LMEvalHarnessAdapter::new("0.4.0"))));
    assert!(registry.register(Box::new(CodeNeedleAdapter::new("1.0.0"))));
    assert_eq!(registry.len(), 2);
    let ids = registry.list_evaluators();
    assert!(ids.contains(&"lm-eval-harness".to_string()));
    assert!(ids.contains(&"code-needle".to_string()));
}

// SBADAPTER-T8: Adapter result has no authority fields
#[test]
fn test_sbadapter_t8_result_no_authority() {
    let a = LMEvalHarnessAdapter::new("0.4.0");
    let f = a.fixture_at(0).unwrap();
    let r = a.evaluate_fixture(&f, "test");
    let json = serde_json::to_value(&r).unwrap();
    assert!(json.get("approved").is_none());
    assert!(json.get("manifest_id").is_none());
    assert!(json.get("decision_id").is_none());
    assert!(json.get("router_eligible").is_none());
    assert!(json.get("score").is_none());
    assert!(json.get("ranking").is_none());
}

// SBADAPTER-T9: Authority boundary preserved across both adapters
#[test]
fn test_sbadapter_t9_authority_boundary_both() {
    let lm = LMEvalHarnessAdapter::new("0.4.0");
    let code = CodeNeedleAdapter::new("1.0.0");
    let lm_f = lm.fixture_at(0).unwrap();
    let code_f = code.fixture_at(0).unwrap();
    let lm_r = lm.evaluate_fixture(&lm_f, "any");
    let code_r = code.evaluate_fixture(&code_f, "any");
    for r in &[lm_r, code_r] {
        let json = serde_json::to_value(r).unwrap();
        assert!(json.get("manifest_id").is_none());
        assert!(json.get("decision_id").is_none());
        assert!(json.get("approved").is_none());
    }
}

// SBADAPTER-T10: Evaluator versions are distinct
#[test]
fn test_sbadapter_t10_distinct_evaluator_versions() {
    let lm = LMEvalHarnessAdapter::new("0.4.0");
    let code = CodeNeedleAdapter::new("1.0.0");
    assert_ne!(lm.evaluator_id(), code.evaluator_id());
    assert_ne!(lm.evaluator_version(), code.evaluator_version());
    assert_ne!(lm.upstream_project(), code.upstream_project());
}

// SBADAPTER-T11: AdapterError display formatting
#[test]
fn test_sbadapter_t11_error_format() {
    let e1 = AdapterError::FixtureIndexOutOfBounds {
        evaluator_id: "lm-eval-harness".to_string(),
        index: 5,
        total: 3,
    };
    let s = format!("{}", e1);
    assert!(s.contains("Fixture index 5"));
    assert!(s.contains("lm-eval-harness"));
}
