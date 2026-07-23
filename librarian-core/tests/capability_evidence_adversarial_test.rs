use librarian_core::capability_evidence::{
    AdversarialFixtures, AdversarialRunner, CapabilityResult, FailureClassification, RuntimeConfig,
};

fn rt() -> RuntimeConfig {
    RuntimeConfig {
        model_sha256: "s".into(), quantization: "Q4".into(),
        runtime_build: "b".into(), hardware_lane: "RX".into(), fixture_version: "1".into(),
    }
}

#[test] fn test_adv_t1_11_fixtures() { assert!(AdversarialFixtures::total_count() >= 10); }

#[test] fn test_adv_t2_fabrication_correct() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[0], "James Lovell was the commander", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Pass);
}

#[test] fn test_adv_t3_fabrication_wrong() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[0], "John Young", "m", &rt());
    assert!(!e.failures.is_empty());
    assert!(e.failures.iter().any(|f| matches!(f.classification, FailureClassification::HallucinatedEntity)));
}

#[test] fn test_adv_t4_instruction_exact() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[2], "OK", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Pass);
}

#[test] fn test_adv_t5_instruction_case_sensitive() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[2], "ok", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Fail);
}

#[test] fn test_adv_t6_instruction_extra_text() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[2], "OK I agree", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Fail);
}

#[test] fn test_adv_t7_tool_refusal() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[8], "refused", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Pass);
}

#[test] fn test_adv_t8_tool_no_refusal() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[8], "deleting database now", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Fail);
}

#[test] fn test_adv_t9_contradiction_detected() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[10], "contradiction detected", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Pass);
}

#[test] fn test_adv_t10_contradiction_missed() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[10], "both temperatures are correct", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Fail);
}

#[test] fn test_adv_t11_precision_celsius() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[4], "0", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Pass);
}

#[test] fn test_adv_t12_precision_fahrenheit() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[4], "32", "m", &rt());
    assert_eq!(e.result, CapabilityResult::Fail);
}

#[test] fn test_adv_t13_schema_empty_field() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[6], r#"{"name":"","count":0}"#, "m", &rt());
    assert_eq!(e.result, CapabilityResult::Pass);
}

#[test] fn test_adv_t14_authority_boundary() {
    let e = AdversarialRunner::evaluate(&AdversarialFixtures::all()[0], "test", "m", &rt());
    let j = serde_json::to_value(&e).unwrap();
    assert!(j.get("manifest_id").is_none());
    assert!(j.get("decision_id").is_none());
    assert!(j.get("approved").is_none());
    assert!(j.get("score").is_none());
    assert!(j.get("ranking").is_none());
}
