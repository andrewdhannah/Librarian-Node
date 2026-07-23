//! EPIC-RELEASE-TRUST-FOUNDATION — end-to-end pipeline integration tests.
//!
//! Validates the full Release Trust pipeline:
//! ReleaseManifest → ReleaseValidation → ReleaseProvenance → ReleaseTrustPackage

use librarian_core::release::{
    ReleaseComponent, ReleaseManifest, ReleaseValidation, ReleaseVersion,
    ReleaseProvenance, ReleaseTrustPackage,
};

fn manifest() -> ReleaseManifest {
    ReleaseManifest {
        release_id: "R-001".into(), version: ReleaseVersion { major: 1, minor: 0, patch: 0 },
        components: vec![ReleaseComponent { component_id: "C-001".into(), component_type: "model".into(), version: "1.0".into(), sprint_id: "S-001".into(), content_hash: "abc".into() }],
        governance_receipt_refs: vec!["GR-001".into()], included_sprint_ids: vec!["S-001".into()],
        created_at: "2026-01-01".into(), content_hash: String::new(),
    }
}

// E2E-T1: Happy path — valid manifest validates and produces trust package
#[test] fn test_e2e_t1_happy_path() {
    let m = manifest();
    let vr = ReleaseValidation::validate(&m, &["S-001".into()], &["GR-001".into()]);
    assert!(vr.valid);
    let vs = ReleaseValidation::summary(&vr);
    let prov = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["abc".into()]);
    let tp = ReleaseTrustPackage::build("R-001", "1.0.0", vr, vs, prov);
    assert!(!tp.integrity_hash.is_empty());
    assert!(tp.validation.valid);
}

// E2E-T2: Missing evidence — not all sprints sealed
#[test] fn test_e2e_t2_missing_evidence() {
    let m = manifest();
    let vr = ReleaseValidation::validate(&m, &[], &["GR-001".into()]);
    assert!(!vr.valid);
    assert!(vr.issues.iter().any(|i| i.code == "SPRINT_NOT_SEALED"));
}

// E2E-T3: Broken hashes — validation catches empty content hash
#[test] fn test_e2e_t3_broken_hashes() {
    let mut m = manifest();
    m.components[0].content_hash = String::new();
    let vr = ReleaseValidation::validate(&m, &["S-001".into()], &["GR-001".into()]);
    assert!(vr.issues.iter().any(|i| i.code == "EMPTY_CONTENT_HASH"));
}

// E2E-T4: Duplicate sprint — manifest lists a sprint twice
#[test] fn test_e2e_t4_duplicate_sprint() {
    let mut m = manifest();
    m.included_sprint_ids.push("S-001".into());
    assert_eq!(m.included_sprint_ids.len(), 2);
    // Validation should still pass since both copies are sealed
    let vr = ReleaseValidation::validate(&m, &["S-001".into()], &["GR-001".into()]);
    assert!(vr.valid);
}

// E2E-T5: Missing receipt — governance reference not found
#[test] fn test_e2e_t5_missing_receipt() {
    let m = manifest();
    let vr = ReleaseValidation::validate(&m, &["S-001".into()], &[]);
    assert!(!vr.valid);
    assert!(vr.issues.iter().any(|i| i.code == "MISSING_GOVERNANCE_REF"));
}

// E2E-T6: Non-sealed sprint — sprint not in sealed list
#[test] fn test_e2e_t6_non_sealed_sprint() {
    let m = manifest();
    let vr = ReleaseValidation::validate(&m, &["S-999".into()], &["GR-001".into()]);
    assert!(!vr.valid);
    assert!(vr.issues.iter().any(|i| i.code == "SPRINT_NOT_SEALED"));
}

// E2E-T7: Provenance chain completeness
#[test] fn test_e2e_t7_provenance_chain() {
    let p = ReleaseProvenance::build("R-001", &["S-001".into(), "S-002".into()], &["GR-001".into(), "GR-002".into()], &["h1".into(), "h2".into()]);
    assert_eq!(p.chain.len(), 5); // release + 2 sprint + 2 governance
    assert_eq!(p.evidence_refs.len(), 2);
    assert!(p.verify());
}

// E2E-T8: Tampered provenance detected
#[test] fn test_e2e_t8_tampered_provenance() {
    let mut p = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["h1".into()]);
    p.chain[1].content_hash = "tampered".into();
    assert!(!p.verify());
}

// E2E-T9: Complete pipeline with validation + provenance + trust package
#[test] fn test_e2e_t9_complete_pipeline() {
    let m = manifest();
    let vr = ReleaseValidation::validate(&m, &["S-001".into()], &["GR-001".into()]);
    assert!(vr.valid);
    let vs = ReleaseValidation::summary(&vr);
    let prov = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["abc".into()]);
    let tp = ReleaseTrustPackage::build("R-001", "1.0.0", vr, vs, prov);
    assert!(tp.validation.valid);
    assert!(tp.provenance.verify());
    // Verify no authority fields in the entire package
    let j = serde_json::to_value(&tp).unwrap();
    assert!(j.get("approve").is_none()); assert!(j.get("recommend").is_none());
    assert!(j.get("deploy").is_none()); assert!(j.get("risk").is_none());
}

// E2E-T10: ReleaseTrustPackage metadata populated correctly
#[test] fn test_e2e_t10_trust_metadata() {
    let m = manifest();
    let vr = ReleaseValidation::validate(&m, &["S-001".into()], &["GR-001".into()]);
    let vs = ReleaseValidation::summary(&vr);
    let prov = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["abc".into()]);
    let tp = ReleaseTrustPackage::build("R-001", "1.0.0", vr, vs, prov);
    assert_eq!(tp.metadata.release_id, "R-001");
    assert_eq!(tp.metadata.version, "1.0.0");
    assert_eq!(tp.metadata.total_sprints, 1);
    assert_eq!(tp.metadata.total_evidence_refs, 1);
}
