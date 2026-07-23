use librarian_core::capability_evidence::CapabilityReviewPackage;
use librarian_core::capability_evidence::models::*;

fn ev(fid: &str, r: CapabilityResult) -> CapabilityEvidence {
    CapabilityEvidence {
        evidence_id: format!("e-{}", fid), model_identity: ModelIdentity { model_id: "m1".into(), model_sha256: "s".into(), quantization: "Q4".into(), model_version: "1".into() },
        runtime_configuration: RuntimeConfig { model_sha256: "s".into(), quantization: "Q4".into(), runtime_build: "b".into(), hardware_lane: "RX".into(), fixture_version: "1".into() },
        evaluator_identity: EvaluatorIdentity { evaluator_id: "ope".into(), evaluator_version: "1".into(), upstream_project: "MQR".into() },
        fixture_identity: FixtureIdentity { fixture_id: fid.into(), fixture_version: "1".into() },
        execution_context: ExecutionContext { timestamp: "1".into(), hardware_lane: "RX".into(), runtime_build: "b".into() },
        result: r, failures: vec![], provenance_reference: ProvenanceReference { lineage_hash: None, lifecycle_event_id: None, model_identity_hash: "s".into() }, evidence_hash: String::new(),
    }
}

#[test] fn test_rv_t1_empty() {
    let p = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &[]);
    assert_eq!(p.evidence_count, 0);
}

#[test] fn test_rv_t2_counts() {
    let p = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &[ev("f1", CapabilityResult::Pass), ev("f2", CapabilityResult::Fail), ev("f3", CapabilityResult::Degraded)]);
    assert_eq!(p.evidence_count, 3); assert_eq!(p.passes, 1); assert_eq!(p.failures, 2); assert_eq!(p.degraded, 1);
}

#[test] fn test_rv_t3_deterministic() {
    let e = vec![ev("f1", CapabilityResult::Pass)];
    let a = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &e);
    let b = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &e);
    assert_eq!(a.content_hash, b.content_hash);
}

#[test] fn test_rv_t4_no_authority() {
    let p = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &[ev("f1", CapabilityResult::Pass)]);
    let j = serde_json::to_value(&p).unwrap();
    assert!(j.get("approve").is_none()); assert!(j.get("reject").is_none());
    assert!(j.get("recommendation").is_none()); assert!(j.get("score").is_none());
}

#[test] fn test_rv_t5_observations_populated() {
    let p = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &[ev("f1", CapabilityResult::Pass)]);
    assert_eq!(p.observations.len(), 1);
    assert_eq!(p.observations[0].fixture_id, "f1");
    assert_eq!(p.observations[0].result, "pass");
}
