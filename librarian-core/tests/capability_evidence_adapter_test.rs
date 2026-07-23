//! MQR-CAPABILITY-EVIDENCE-ADAPTER-FOUNDATION-1 — Adapter framework integration tests.
//!
//! Verifies the EvaluatorAdapter trait, AdapterError, and AdapterRegistry
//! preserve the MQR authority boundary: adapters produce capability
//! evidence, they do not create or mutate authority.

use librarian_core::capability_evidence::{
    AdapterError, AdapterRegistry, CapabilityFixture, CapabilityResult, EvaluatorAdapter,
    ValidationMethod,
};

struct CountAdapter {
    id: String,
    count: usize,
}

impl EvaluatorAdapter for CountAdapter {
    fn evaluator_id(&self) -> &str { &self.id }
    fn evaluator_version(&self) -> &str { "1.0.0" }
    fn upstream_project(&self) -> &str { "MQR-test" }
    fn fixture_count(&self) -> usize { self.count }
    fn fixture_at(&self, index: usize) -> Result<CapabilityFixture, AdapterError> {
        if index >= self.count {
            return Err(AdapterError::FixtureIndexOutOfBounds {
                evaluator_id: self.id.clone(),
                index,
                total: self.count,
            });
        }
        Ok(CapabilityFixture {
            fixture_id: format!("{}-{}", self.id, index),
            version: "1.0.0".to_string(),
            category: "test".to_string(),
            description: format!("Test fixture {}", index),
            prompt: "p".to_string(),
            expected_outcome: "ok".to_string(),
            validation: ValidationMethod::Contains { expected: "ok".to_string() },
        })
    }
    fn evaluate_fixture(
        &self,
        _fixture: &CapabilityFixture,
        output: &str,
    ) -> CapabilityResult {
        if output.contains("ok") {
            CapabilityResult::Pass
        } else {
            CapabilityResult::Fail
        }
    }
}

fn make_adapter(id: &str, count: usize) -> Box<dyn EvaluatorAdapter> {
    Box::new(CountAdapter { id: id.to_string(), count })
}

// ADAPTER-T1: Empty registry has zero adapters
#[test]
fn test_adapter_t1_empty_registry() {
    let r = AdapterRegistry::new();
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
    assert_eq!(r.total_fixture_count(), 0);
}

// ADAPTER-T2: Register and lookup adapter
#[test]
fn test_adapter_t2_register_and_lookup() {
    let mut r = AdapterRegistry::new();
    r.register(make_adapter("lm-eval", 10));
    assert!(r.contains("lm-eval"));
    let a = r.get("lm-eval").unwrap();
    assert_eq!(a.evaluator_id(), "lm-eval");
    assert_eq!(a.evaluator_version(), "1.0.0");
    assert_eq!(a.upstream_project(), "MQR-test");
    assert_eq!(a.fixture_count(), 10);
}

// ADAPTER-T3: Multiple adapters coexist
#[test]
fn test_adapter_t3_multiple_adapters() {
    let mut r = AdapterRegistry::new();
    r.register(make_adapter("a", 5));
    r.register(make_adapter("b", 10));
    r.register(make_adapter("c", 3));
    assert_eq!(r.len(), 3);
    assert_eq!(r.total_fixture_count(), 18);
    assert_eq!(r.list_evaluators(), vec!["a", "b", "c"]);
}

// ADAPTER-T4: Duplicate ID ignored (first registration wins)
#[test]
fn test_adapter_t4_duplicate_id_ignored() {
    let mut r = AdapterRegistry::new();
    assert!(r.register(make_adapter("dup", 5)));
    assert!(!r.register(make_adapter("dup", 10)));
    assert_eq!(r.len(), 1);
    assert_eq!(r.get("dup").unwrap().fixture_count(), 5);
}

// ADAPTER-T5: Fixture iteration
#[test]
fn test_adapter_t5_fixture_iteration() {
    let a = CountAdapter { id: "test".to_string(), count: 3 };
    for i in 0..3 {
        let f = a.fixture_at(i).unwrap();
        assert_eq!(f.fixture_id, format!("test-{}", i));
    }
}

// ADAPTER-T6: Out of bounds index returns AdapterError
#[test]
fn test_adapter_t6_out_of_bounds() {
    let a = CountAdapter { id: "test".to_string(), count: 3 };
    let err = a.fixture_at(10).unwrap_err();
    match err {
        AdapterError::FixtureIndexOutOfBounds { evaluator_id, index, total } => {
            assert_eq!(evaluator_id, "test");
            assert_eq!(index, 10);
            assert_eq!(total, 3);
        }
        _ => panic!("Expected FixtureIndexOutOfBounds"),
    }
}

// ADAPTER-T7: Deterministic fixture enumeration
#[test]
fn test_adapter_t7_deterministic_fixtures() {
    let a = CountAdapter { id: "det".to_string(), count: 5 };
    let f1 = a.fixture_at(2).unwrap();
    let f2 = a.fixture_at(2).unwrap();
    assert_eq!(f1.fixture_id, f2.fixture_id);
    assert_eq!(f1.fixture_id, "det-2");
}

// ADAPTER-T8: Fixture evaluation passes
#[test]
fn test_adapter_t8_evaluation_pass() {
    let a = CountAdapter { id: "test".to_string(), count: 1 };
    let f = a.fixture_at(0).unwrap();
    let r = a.evaluate_fixture(&f, "this is ok output");
    assert_eq!(r, CapabilityResult::Pass);
}

// ADAPTER-T9: Fixture evaluation fails
#[test]
fn test_adapter_t9_evaluation_fail() {
    let a = CountAdapter { id: "test".to_string(), count: 1 };
    let f = a.fixture_at(0).unwrap();
    let r = a.evaluate_fixture(&f, "this fails definitely");
    assert_eq!(r, CapabilityResult::Fail);
}

// ADAPTER-T10: Adapter contract produces no authority fields
#[test]
fn test_adapter_t10_no_authority_in_evidence() {
    let a = CountAdapter { id: "test".to_string(), count: 1 };
    let f = a.fixture_at(0).unwrap();
    let r = a.evaluate_fixture(&f, "ok");
    assert!(matches!(r, CapabilityResult::Pass));
    let json = serde_json::to_value(&r).unwrap();
    // No approval or authority fields in result
    assert!(json.get("approved").is_none());
    assert!(json.get("manifest_id").is_none());
    assert!(json.get("decision_id").is_none());
    assert!(json.get("score").is_none());
}

// ADAPTER-T11: Deterministic contract — same inputs produce same outputs
#[test]
fn test_adapter_t11_deterministic_contract() {
    let a = CountAdapter { id: "test".to_string(), count: 2 };
    for i in 0..2 {
        let f1 = a.fixture_at(i).unwrap();
        let f2 = a.fixture_at(i).unwrap();
        assert_eq!(f1.fixture_id, f2.fixture_id);
        assert_eq!(f1.validation, f2.validation);
        let r1 = a.evaluate_fixture(&f1, "input");
        let r2 = a.evaluate_fixture(&f2, "input");
        assert_eq!(r1, r2);
    }
}

// ADAPTER-T12: Registry lookup for non-existent adapter
#[test]
fn test_adapter_t12_lookup_nonexistent() {
    let r = AdapterRegistry::new();
    assert!(r.get("nope").is_none());
    assert!(!r.contains("nope"));
}

// ADAPTER-T13: AdapterError display formats correctly
#[test]
fn test_adapter_t13_error_display() {
    let e1 = AdapterError::FixtureIndexOutOfBounds {
        evaluator_id: "test".to_string(),
        index: 5,
        total: 2,
    };
    let s1 = format!("{}", e1);
    assert!(s1.contains("Fixture index 5"));
    assert!(s1.contains("test"));

    let e2 = AdapterError::EvaluatorNotFound {
        evaluator_id: "missing".to_string(),
    };
    let s2 = format!("{}", e2);
    assert!(s2.contains("missing"));

    let e3 = AdapterError::DuplicateRegistration {
        evaluator_id: "dup".to_string(),
    };
    let s3 = format!("{}", e3);
    assert!(s3.contains("dup"));

    let e4 = AdapterError::EvaluationFailed {
        evaluator_id: "test".to_string(),
        reason: "something went wrong".to_string(),
    };
    let s4 = format!("{}", e4);
    assert!(s4.contains("something went wrong"));
}

// ADAPTER-T14: CapabilityRunner accepts custom evaluator identity
#[test]
fn test_adapter_t14_runner_with_custom_evaluator() {
    use librarian_core::capability_evidence::runner::CapabilityRunner;
    use librarian_core::capability_evidence::models::RuntimeConfig;

    let f = CapabilityFixture {
        fixture_id: "f-001".to_string(),
        version: "1.0.0".to_string(),
        category: "test".to_string(),
        description: "Test fixture".to_string(),
        prompt: "p".to_string(),
        expected_outcome: "ok".to_string(),
        validation: ValidationMethod::Contains { expected: "ok".to_string() },
    };
    let runtime = RuntimeConfig {
        model_sha256: "sha-256".to_string(),
        quantization: "Q4_K_M".to_string(),
        runtime_build: "build-1".to_string(),
        hardware_lane: "RX 570".to_string(),
        fixture_version: "1.0.0".to_string(),
    };
    let e = CapabilityRunner::evaluate_with_evaluator(
        &f, "ok", "m1", &runtime,
        "lm-eval-harness", "0.4.0", "EleutherAI/lm-evaluation-harness",
    );
    assert_eq!(e.evaluator_identity.evaluator_id, "lm-eval-harness");
    assert_eq!(e.evaluator_identity.evaluator_version, "0.4.0");
    assert_eq!(e.evaluator_identity.upstream_project, "EleutherAI/lm-evaluation-harness");
}
