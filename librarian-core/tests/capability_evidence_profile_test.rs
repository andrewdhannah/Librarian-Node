use librarian_core::capability_evidence::{
    AggregatedResult, CapabilityProfile, ProfileAssembler, ProfileWarningSeverity,
};
use librarian_core::capability_evidence::models::*;

fn ev(fid: &str, result: CapabilityResult, eval: &str) -> CapabilityEvidence {
    CapabilityEvidence {
        evidence_id: format!("e-{}", fid),
        model_identity: ModelIdentity { model_id: "m1".into(), model_sha256: "s".into(), quantization: "Q4".into(), model_version: "1".into() },
        runtime_configuration: RuntimeConfig { model_sha256: "s".into(), quantization: "Q4".into(), runtime_build: "b".into(), hardware_lane: "RX".into(), fixture_version: "1".into() },
        evaluator_identity: EvaluatorIdentity { evaluator_id: eval.into(), evaluator_version: "1".into(), upstream_project: "MQR".into() },
        fixture_identity: FixtureIdentity { fixture_id: fid.into(), fixture_version: "1".into() },
        execution_context: ExecutionContext { timestamp: "2026-01-01".into(), hardware_lane: "RX".into(), runtime_build: "b".into() },
        result, failures: vec![],
        provenance_reference: ProvenanceReference { lineage_hash: None, lifecycle_event_id: None, model_identity_hash: "s".into() },
        evidence_hash: String::new(),
    }
}

#[test] fn test_pf_t1_empty_evidence() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[]);
    assert!(p.domain_profiles.is_empty());
    assert!(p.sources.is_empty());
    assert!(p.warnings.is_empty());
    assert_eq!(p.model_id, "m1");
}

#[test] fn test_pf_t2_all_pass() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[
        ev("struct", CapabilityResult::Pass, "ope"),
        ev("struct", CapabilityResult::Pass, "ope"),
    ]);
    assert_eq!(p.domain_profiles[0].overall_result, AggregatedResult::Pass);
}

#[test] fn test_pf_t3_all_fail() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[
        ev("struct", CapabilityResult::Fail, "ope"),
        ev("struct", CapabilityResult::Fail, "ope"),
    ]);
    assert_eq!(p.domain_profiles[0].overall_result, AggregatedResult::Fail);
}

#[test] fn test_pf_t4_some_degraded() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[
        ev("f", CapabilityResult::Pass, "ope"),
        ev("f", CapabilityResult::Degraded, "ope"),
    ]);
    assert_eq!(p.domain_profiles[0].overall_result, AggregatedResult::Degraded);
}

#[test] fn test_pf_t5_multiple_domains() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[
        ev("struct", CapabilityResult::Pass, "ope"),
        ev("fact", CapabilityResult::Pass, "ope"),
    ]);
    assert_eq!(p.domain_profiles.len(), 2);
}

#[test] fn test_pf_t6_mixed_results() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[
        ev("a", CapabilityResult::Pass, "ope"),
        ev("a", CapabilityResult::Pass, "ope"),
        ev("b", CapabilityResult::Fail, "ope"),
        ev("b", CapabilityResult::Fail, "ope"),
    ]);
    assert_eq!(p.domain_profiles[0].overall_result, AggregatedResult::Pass);
    assert_eq!(p.domain_profiles[1].overall_result, AggregatedResult::Fail);
}

#[test] fn test_pf_t7_content_hash_deterministic() {
    let e = vec![ev("x", CapabilityResult::Pass, "ope")];
    let p1 = ProfileAssembler::assemble("m1", "s", "Q4", &e);
    let p2 = ProfileAssembler::assemble("m1", "s", "Q4", &e);
    assert_eq!(p1.content_hash, p2.content_hash);
}

#[test] fn test_pf_t8_warnings_generated() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[
        ev("x", CapabilityResult::Fail, "ope"),
        ev("x", CapabilityResult::Fail, "ope"),
    ]);
    assert!(!p.warnings.is_empty());
    assert!(p.warnings.iter().any(|w| matches!(w.severity, ProfileWarningSeverity::Critical)));
}

#[test] fn test_pf_t9_no_authority_fields() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[ev("x", CapabilityResult::Pass, "ope")]);
    let j = serde_json::to_value(&p).unwrap();
    assert!(j.get("manifest_id").is_none());
    assert!(j.get("decision_id").is_none());
    assert!(j.get("approved").is_none());
    assert!(j.get("score").is_none());
    assert!(j.get("ranking").is_none());
    assert!(j.get("recommendation").is_none());
    assert!(j.get("qualified").is_none());
}

#[test] fn test_pf_t10_not_tested_empty() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[]);
    assert!(p.domain_profiles.is_empty());
}

#[test] fn test_pf_t11_evidence_refs_populated() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[ev("x", CapabilityResult::Pass, "ope")]);
    assert!(!p.domain_profiles[0].evidence_refs.is_empty());
    assert!(p.domain_profiles[0].evidence_refs[0].contains("e-"));
}

#[test] fn test_pf_t12_quantization_preserved() {
    let p = ProfileAssembler::assemble("m1", "s", "Q8_K_M", &[ev("x", CapabilityResult::Pass, "ope")]);
    assert_eq!(p.quantization, "Q8_K_M");
}

#[test] fn test_pf_t13_generated_at_set() {
    let p = ProfileAssembler::assemble("m1", "s", "Q4", &[ev("x", CapabilityResult::Pass, "ope")]);
    assert!(!p.generated_at.is_empty());
}
