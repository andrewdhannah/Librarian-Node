use librarian_core::capability_evidence::{CapabilityRegressionDetector, RegressionResult, CapabilityReplay};
use librarian_core::capability_evidence::models::*;

fn ev(fid: &str, r: CapabilityResult, v: &str) -> CapabilityEvidence {
    CapabilityEvidence {
        evidence_id: format!("e-{}", fid), model_identity: ModelIdentity { model_id: "m1".into(), model_sha256: "s".into(), quantization: "Q4".into(), model_version: "1".into() },
        runtime_configuration: RuntimeConfig { model_sha256: "s".into(), quantization: "Q4".into(), runtime_build: "b".into(), hardware_lane: "RX".into(), fixture_version: v.into() },
        evaluator_identity: EvaluatorIdentity { evaluator_id: "ope".into(), evaluator_version: "1".into(), upstream_project: "MQR".into() },
        fixture_identity: FixtureIdentity { fixture_id: fid.into(), fixture_version: v.into() },
        execution_context: ExecutionContext { timestamp: "1".into(), hardware_lane: "RX".into(), runtime_build: "b".into() },
        result: r, failures: vec![], provenance_reference: ProvenanceReference { lineage_hash: None, lifecycle_event_id: None, model_identity_hash: "s".into() }, evidence_hash: String::new(),
    }
}

#[test] fn test_rp_t1_no_change() {
    let r = CapabilityRegressionDetector::compare("d1", "d2", &[ev("f1", CapabilityResult::Pass, "1")], &[ev("f1", CapabilityResult::Pass, "1")]);
    assert_eq!(r.comparisons[0].regression, RegressionResult::NoChange);
    assert_eq!(r.regressions, 0);
}

#[test] fn test_rp_t2_regression() {
    let r = CapabilityRegressionDetector::compare("d1", "d2", &[ev("f1", CapabilityResult::Pass, "1")], &[ev("f1", CapabilityResult::Fail, "1")]);
    assert_eq!(r.regressions, 1);
}

#[test] fn test_rp_t3_improvement() {
    let r = CapabilityRegressionDetector::compare("d1", "d2", &[ev("f1", CapabilityResult::Fail, "1")], &[ev("f1", CapabilityResult::Pass, "1")]);
    assert_eq!(r.improvements, 1);
}

#[test] fn test_rp_t4_multiple() {
    let p = vec![ev("f1", CapabilityResult::Pass, "1"), ev("f2", CapabilityResult::Pass, "1")];
    let c = vec![ev("f1", CapabilityResult::Fail, "1"), ev("f2", CapabilityResult::Pass, "1")];
    let r = CapabilityRegressionDetector::compare("d1", "d2", &p, &c);
    assert_eq!(r.regressions, 1);
}

#[test] fn test_rp_t5_deterministic() {
    let p = vec![ev("f1", CapabilityResult::Pass, "1")];
    let c = vec![ev("f1", CapabilityResult::Fail, "1")];
    let a = CapabilityRegressionDetector::compare("d1", "d2", &p, &c);
    let b = CapabilityRegressionDetector::compare("d1", "d2", &p, &c);
    assert_eq!(a.content_hash, b.content_hash);
}

#[test] fn test_rp_t6_authority_boundary() {
    let r = CapabilityRegressionDetector::compare("d1", "d2", &[ev("f1", CapabilityResult::Pass, "1")], &[ev("f1", CapabilityResult::Fail, "1")]);
    let j = serde_json::to_value(&r).unwrap();
    assert!(j.get("approve").is_none()); assert!(j.get("reject").is_none());
    assert!(j.get("recommendation").is_none()); assert!(j.get("score").is_none());
}

#[test] fn test_rp_t7_labels_preserved() {
    let r = CapabilityRegressionDetector::compare("2026-01-01", "2026-07-01", &[ev("f1", CapabilityResult::Pass, "1")], &[ev("f1", CapabilityResult::Fail, "1")]);
    assert_eq!(r.previous_label, "2026-01-01");
    assert_eq!(r.current_label, "2026-07-01");
}
