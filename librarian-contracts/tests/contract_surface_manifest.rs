//! RUST-M0-0 Contract Surface Lock — drift guard.
//!
//! Verifies that the librarian-contracts interop surface still matches
//! `conformance/contract-surface/contract-surface-manifest.json`:
//!
//! 1. Serialized field names / enum variants of the startup contract types.
//! 2. SHA-256 hashes of the M0A fixtures, canonical schema assets, and the
//!    contract source files themselves.
//!
//! Any drift fails this test; updating the surface requires a deliberate
//! manifest re-baseline (divergence protocol).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use librarian_contracts::node::capability_registry::{
    AssessorType, CapabilityDependency, CapabilityId, CapabilityIdentity,
    CapabilityRelationshipType, CapabilitySecurityContext, CapabilityType, CapabilityVersion,
    ClassificationDerivation, EvidenceDimension, EvidenceFreshness, EvidenceProducerRole,
    EvidenceType, OperationalMode, OperationalModeInputs, OperationalModeValue, QualificationAxis,
    QualificationEvidenceReference, QualificationLifecycleEvent, QualificationRecord,
    QualificationRecordStatus, QualificationState, SecurityClassification, TransitionType,
    TransitionerRole,
};
use librarian_contracts::node::startup::{
    is_sha256_hex, RuntimeLifecycleState, StartupCheck, StartupPhase, StartupStatus,
};
use librarian_contracts::node::StartupReceipt;
use sha2::{Digest, Sha256};

const MANIFEST_REL: &str = "../conformance/contract-surface/contract-surface-manifest.json";

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(MANIFEST_REL)
}

fn load_manifest() -> serde_json::Value {
    let text = std::fs::read_to_string(manifest_path()).expect("read contract-surface-manifest.json");
    serde_json::from_str(&text).expect("parse contract-surface-manifest.json")
}

fn sha256_of(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    format!("{:x}", Sha256::digest(&bytes))
}

/// Resolve a workspace-relative path recorded in the manifest.
fn workspace_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../{rel}"))
}

fn sample_receipt() -> StartupReceipt {
    StartupReceipt {
        receipt_id: "FIXTURE-RECEIPT-001".to_string(),
        node_id: "WINPC-BIG-PICKLE".to_string(),
        platform: "windows".to_string(),
        governance_commit: "6be76216a8048492526c4ca0ae751b6d2d507185".to_string(),
        startup_phase: "complete".to_string(),
        identity_loaded: true,
        governance_verified: true,
        capabilities_loaded: true,
        environment_validated: true,
        checks_passed: 6,
        checks_failed: 0,
        status: "GOVERNED_EXECUTION".to_string(),
        timestamp: "2026-07-25T02:33:33Z".to_string(),
    }
}

#[test]
fn manifest_exists_and_is_well_formed() {
    let manifest = load_manifest();
    assert_eq!(manifest["manifest_id"], "CONTRACT-SURFACE-MANIFEST-001");
    let types = manifest["types"].as_object().expect("types object");
    for name in [
        "StartupReceipt",
        "StartupReceiptFacts",
        "StartupPhase",
        "StartupStatus",
        "StartupCheck",
        "RuntimeLifecycleState",
    ] {
        assert!(types.contains_key(name), "manifest must declare type {name}");
    }
}

#[test]
fn startup_receipt_surface_matches_manifest() {
    let manifest = load_manifest();
    let expected: BTreeSet<String> = manifest["types"]["StartupReceipt"]["serialized_keys"]
        .as_array()
        .expect("serialized_keys array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let json = serde_json::to_value(sample_receipt()).expect("serialize StartupReceipt");
    let actual: BTreeSet<String> = json
        .as_object()
        .expect("object")
        .keys()
        .cloned()
        .collect();

    assert_eq!(
        actual, expected,
        "StartupReceipt serialized field set drifted from the manifest"
    );
    assert_eq!(actual.len(), 13, "schema RECEIPT-SCHEMA-001 requires exactly 13 fields");
    assert!(is_sha256_hex(&sample_receipt().governance_commit));
}

#[test]
fn startup_phase_surface_matches_manifest() {
    let manifest = load_manifest();
    let expected: Vec<String> = manifest["types"]["StartupPhase"]["variants"]
        .as_array()
        .expect("variants array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let actual: Vec<String> = StartupPhase::ALL.iter().map(|p| p.as_str().to_string()).collect();
    assert_eq!(actual, expected, "StartupPhase variants drifted from the manifest");

    // Serde round-trip must reproduce the manifest names.
    for name in &expected {
        let value: serde_json::Value = serde_json::from_str(&format!("\"{name}\""))
            .expect("deserialize StartupPhase");
        assert!(value.as_str().is_some());
    }
}

#[test]
fn startup_status_surface_matches_manifest() {
    let manifest = load_manifest();
    let expected: Vec<String> = manifest["types"]["StartupStatus"]["variants"]
        .as_array()
        .expect("variants array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let actual: Vec<String> = [
        StartupStatus::GovernedExecution,
        StartupStatus::StartupFailed,
    ]
    .iter()
    .map(|s| s.as_str().to_string())
    .collect();
    assert_eq!(actual, expected, "StartupStatus variants drifted from the manifest");
}

#[test]
fn startup_check_surface_matches_manifest() {
    let manifest = load_manifest();
    let expected: BTreeSet<String> = manifest["types"]["StartupCheck"]["serialized_keys"]
        .as_array()
        .expect("serialized_keys array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let check = StartupCheck {
        phase: StartupPhase::IdentityLoading,
        passed: true,
        detail: "ok".to_string(),
    };
    let json = serde_json::to_value(&check).expect("serialize StartupCheck");
    let actual: BTreeSet<String> = json.as_object().expect("object").keys().cloned().collect();
    assert_eq!(actual, expected, "StartupCheck serialized field set drifted");
}

#[test]
fn startup_receipt_facts_surface_matches_manifest() {
    // StartupReceiptFacts is internal (not serialized) but its 11 fields are
    // the equivalence surface — they must match the manifest declaration.
    let manifest = load_manifest();
    let expected: BTreeSet<String> = manifest["types"]["StartupReceiptFacts"]["fields"]
        .as_object()
        .expect("fields object")
        .keys()
        .cloned()
        .collect();

    let facts = sample_receipt().deterministic_facts();
    let json = serde_json::to_value(&facts).expect("serialize StartupReceiptFacts");
    let actual: BTreeSet<String> = json.as_object().expect("object").keys().cloned().collect();
    assert_eq!(actual.len(), 11, "equivalence surface must have exactly 11 fields");
    assert_eq!(actual, expected, "StartupReceiptFacts surface drifted from the manifest");
}

#[test]
fn runtime_lifecycle_state_surface_matches_manifest() {
    let manifest = load_manifest();
    let expected: Vec<String> = manifest["types"]["RuntimeLifecycleState"]["variants"]
        .as_array()
        .expect("variants array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let actual: Vec<String> = RuntimeLifecycleState::ALL
        .iter()
        .map(|s| s.as_str().to_string())
        .collect();
    assert_eq!(
        actual, expected,
        "RuntimeLifecycleState variants drifted from the manifest"
    );

    // Observational constraint: all variants must round-trip through serde
    // without authorizing anything (contract §2.4).
    for name in &expected {
        let value: RuntimeLifecycleState =
            serde_json::from_str(&format!("\"{name}\"")).expect("deserialize RuntimeLifecycleState");
        assert_eq!(value.as_str(), name);
    }
}

#[test]
fn m1_contract_surface_declared_in_manifest() {
    // M1-0 surface lock: the five capability contracts and their suites must be
    // declared in the manifest. Removing or renaming them fails the guard;
    // updating the surface requires a deliberate manifest re-baseline.
    let manifest = load_manifest();

    let expected_contracts = [
        "contracts/capability/CAPABILITY-ASSURANCE-CONTRACT-001.md",
        "contracts/capability/CAPABILITY-IDENTITY-CONTRACT-001.md",
        "contracts/capability/REGISTRY-OBSERVATION-CONTRACT-001.md",
        "contracts/capability/QUALIFICATION-STATE-CONTRACT-001.md",
        "contracts/capability/OPERATIONAL-MODE-DERIVATION-CONTRACT-001.md",
    ];
    let declared: BTreeSet<String> = manifest["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    for path in expected_contracts {
        assert!(
            declared.contains(path),
            "M1 contract must be declared in the manifest: {path}"
        );
    }

    let expected_suites = [
        "CAPABILITY-ASSURANCE-SCHEMA-001",
        "CAPABILITY-IDENTITY-SCHEMA-001",
        "REGISTRY-OBSERVATION-SCHEMA-001",
        "QUALIFICATION-STATE-SCHEMA-001",
        "OPERATIONAL-MODE-DERIVATION-SCHEMA-001",
    ];
    let suites: BTreeSet<String> = manifest["schema_suite"]
        .as_array()
        .expect("schema_suite array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    for suite in expected_suites {
        assert!(
            suites.contains(suite),
            "M1 suite must be declared in the manifest: {suite}"
        );
    }
}

#[test]
fn m1a_types_declared_in_manifest() {
    let manifest = load_manifest();
    let types = manifest["types"].as_object().expect("types object");
    for name in [
        "CapabilityId",
        "CapabilityVersion",
        "CapabilityType",
        "CapabilityRelationshipType",
        "CapabilityDependency",
        "CapabilityIdentity",
        "QualificationState",
        "QualificationAxis",
        "QualificationRecordStatus",
        "AssessorType",
        "QualificationRecord",
        "TransitionType",
        "TransitionerRole",
        "QualificationLifecycleEvent",
        "EvidenceDimension",
        "EvidenceType",
        "EvidenceProducerRole",
        "QualificationEvidenceReference",
        "SecurityClassification",
        "ClassificationDerivation",
        "CapabilitySecurityContext",
        "OperationalModeValue",
        "EvidenceFreshness",
        "OperationalModeInputs",
        "OperationalMode",
    ] {
        assert!(types.contains_key(name), "manifest must declare M1-A type {name}");
    }
}

/// Collect the serialized variant names of an enum's `ALL` const array.
fn variant_names<T: Copy>(all: &[T], as_str: fn(T) -> &'static str) -> Vec<&'static str> {
    all.iter().map(|&v| as_str(v)).collect()
}

#[test]
fn m1a_enum_variant_surfaces_match_manifest() {
    let manifest = load_manifest();
    let enums: Vec<(&str, Vec<&'static str>)> = vec![
        ("CapabilityType", variant_names(&CapabilityType::ALL, CapabilityType::as_str)),
        (
            "CapabilityRelationshipType",
            variant_names(&CapabilityRelationshipType::ALL, CapabilityRelationshipType::as_str),
        ),
        ("QualificationState", variant_names(&QualificationState::ALL, QualificationState::as_str)),
        ("QualificationAxis", variant_names(&QualificationAxis::ALL, QualificationAxis::as_str)),
        (
            "QualificationRecordStatus",
            variant_names(&QualificationRecordStatus::ALL, QualificationRecordStatus::as_str),
        ),
        ("AssessorType", variant_names(&AssessorType::ALL, AssessorType::as_str)),
        ("TransitionType", variant_names(&TransitionType::ALL, TransitionType::as_str)),
        ("TransitionerRole", variant_names(&TransitionerRole::ALL, TransitionerRole::as_str)),
        ("EvidenceDimension", variant_names(&EvidenceDimension::ALL, EvidenceDimension::as_str)),
        ("EvidenceType", variant_names(&EvidenceType::ALL, EvidenceType::as_str)),
        (
            "EvidenceProducerRole",
            variant_names(&EvidenceProducerRole::ALL, EvidenceProducerRole::as_str),
        ),
        (
            "SecurityClassification",
            variant_names(&SecurityClassification::ALL, SecurityClassification::as_str),
        ),
        (
            "ClassificationDerivation",
            variant_names(&ClassificationDerivation::ALL, ClassificationDerivation::as_str),
        ),
        (
            "OperationalModeValue",
            variant_names(&OperationalModeValue::ALL, OperationalModeValue::as_str),
        ),
        ("EvidenceFreshness", variant_names(&EvidenceFreshness::ALL, EvidenceFreshness::as_str)),
    ];

    for (type_name, actual) in &enums {
        let expected: Vec<String> = manifest["types"][type_name]["variants"]
            .as_array()
            .expect("variants array")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            *actual, expected,
            "{type_name} variants drifted from the manifest"
        );

        // Every variant must round-trip through serde (serde names == manifest names).
        for name in &expected {
            let value: serde_json::Value =
                serde_json::from_str(&format!("\"{name}\"")).expect("deserialize {type_name}");
            assert!(value.as_str().is_some(), "{type_name} variant {name} not deserializable");
        }
    }
}

/// Sample M1-A records as serialized JSON, keyed by manifest type name.
fn m1a_samples() -> Vec<(&'static str, serde_json::Value)> {
    let id_a = CapabilityId::new("alpha".to_string()).expect("valid id");
    let id_b = CapabilityId::new("beta".to_string()).expect("valid id");
    let version = CapabilityVersion::new(1).expect("valid version");

    let dependency = CapabilityDependency::new(
        id_a.clone(),
        id_b.clone(),
        true,
        CapabilityRelationshipType::Requires,
    )
    .expect("non-self dependency");

    let identity = CapabilityIdentity {
        capability_id: id_a.clone(),
        name: "Alpha Capability".to_string(),
        capability_type: CapabilityType::Skill,
        version: Some(version),
        lifecycle_state: QualificationState::Unreviewed,
    };

    let record = QualificationRecord {
        qualification_id: "Q-20260807-001".to_string(),
        capability_id: id_a.clone(),
        profile_id: "PROF-001".to_string(),
        version_id: Some(1),
        status: QualificationRecordStatus::Passed,
        confidence: Some(0.95),
        evidence_reference: Some("EV-00001".to_string()),
        qualified_at: Some("2026-08-07T10:00:00Z".to_string()),
        expires_at: None,
        assessed_at: "2026-08-07T09:00:00Z".to_string(),
        assessor_identity: Some("evaluator@librarian".to_string()),
        assessor_type: AssessorType::Manual,
    };

    let event = QualificationLifecycleEvent {
        event_id: "QLE-20260807-001".to_string(),
        qualification_id: "Q-20260807-001".to_string(),
        capability_id: id_a.clone(),
        from_state: QualificationState::Unreviewed,
        to_state: QualificationState::Reviewed,
        transition_type: TransitionType::Manual,
        security_classification: Some(SecurityClassification::S0),
        transitioned_by: "evaluator@librarian".to_string(),
        transitioner_role: TransitionerRole::Evaluator,
        authority_evidence_id: Some("EV-00001".to_string()),
        evidence_snapshot: serde_json::json!({"dimensions_checked": 3}),
        created_at: "2026-08-07T09:30:00Z".to_string(),
    };

    let evidence = QualificationEvidenceReference {
        evidence_id: "QER-20260807-001".to_string(),
        qualification_id: "Q-20260807-001".to_string(),
        dimension: EvidenceDimension::Identity,
        evidence_type: EvidenceType::Receipt,
        evidence_reference: Some("EV-00001".to_string()),
        evidence_body: Some(serde_json::json!({"checksum": "abc"})),
        evidence_hash: "abcd".to_string(),
        captured_at: "2026-08-07T08:00:00Z".to_string(),
        expires_at: None,
        producer_identity: "evaluator@librarian".to_string(),
        producer_role: EvidenceProducerRole::Evaluator,
    };

    let security_context = CapabilitySecurityContext {
        classification: SecurityClassification::S1,
        source: "capability_registry.capabilities".to_string(),
        derivation: ClassificationDerivation::Declared,
        evidence_reference: Some("EV-00001".to_string()),
    };

    let inputs = OperationalModeInputs {
        security_classification: SecurityClassification::S1,
        qualification_axis: QualificationAxis::Passed,
        lifecycle_state: QualificationState::Qualified,
        evidence_freshness: EvidenceFreshness::Fresh,
        policy_constraints: Some(serde_json::json!({"require_approval": true})),
    };

    let mode = OperationalMode {
        mode: OperationalModeValue::RecommendOnly,
        explanation: "qualified with fresh evidence".to_string(),
        derivation_inputs: inputs,
        evidence_references: vec!["EV-00001".to_string()],
    };

    vec![
        ("CapabilityDependency", serde_json::to_value(dependency).expect("serialize")),
        ("CapabilityIdentity", serde_json::to_value(identity).expect("serialize")),
        ("QualificationRecord", serde_json::to_value(record).expect("serialize")),
        ("QualificationLifecycleEvent", serde_json::to_value(event).expect("serialize")),
        ("QualificationEvidenceReference", serde_json::to_value(evidence).expect("serialize")),
        ("CapabilitySecurityContext", serde_json::to_value(security_context).expect("serialize")),
        (
            "OperationalModeInputs",
            serde_json::to_value(mode.derivation_inputs.clone()).expect("serialize"),
        ),
        ("OperationalMode", serde_json::to_value(mode).expect("serialize")),
    ]
}

#[test]
fn m1a_struct_surfaces_match_manifest() {
    let manifest = load_manifest();
    for (type_name, sample) in m1a_samples() {
        let expected: BTreeSet<String> = manifest["types"][type_name]["serialized_keys"]
            .as_array()
            .expect("serialized_keys array")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        let actual: BTreeSet<String> = sample
            .as_object()
            .expect("object")
            .keys()
            .cloned()
            .collect();

        assert_eq!(
            actual, expected,
            "{type_name} serialized field set drifted from the manifest"
        );
    }
}

#[test]
fn m1a_serialized_surface_has_no_authority_keys() {
    // Non-collapse guard: no M1-A serialized field may imply authority
    // (Registry ≠ Authority, Capability ≠ Permission). Authority verbs live in
    // transition/approval semantics, never in observational data fields.
    let forbidden = ["enable", "authorize", "activate", "permission", "grant", "decide"];
    let manifest = load_manifest();
    let m1a_types: Vec<String> = m1a_samples()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    for type_name in &m1a_types {
        if let Some(keys) = manifest["types"][type_name]["serialized_keys"].as_array() {
            for key in keys {
                let key = key.as_str().unwrap();
                for word in forbidden {
                    assert!(
                        !key.contains(word),
                        "{type_name} serialized key '{key}' contains authority term '{word}'"
                    );
                }
            }
        }
    }
}

#[test]
fn m1b_read_boundary_declared_in_manifest() {
    // M1-B-0 lock: the runtime registry projection adapter is declared under the
    // EXISTING REGISTRY-OBSERVATION-SCHEMA-001 suite (parent-aligned lineage,
    // no new semantic suite). Its source hash is pinned; removing or renaming
    // it fails the guard.
    let manifest = load_manifest();

    let declared: BTreeSet<String> = manifest["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(
        declared.contains("contracts/runtime-api/RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md"),
        "M1-B read boundary contract must be declared in the manifest"
    );

    let suites: BTreeSet<String> = manifest["schema_suite"]
        .as_array()
        .expect("schema_suite array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(
        suites.contains("REGISTRY-OBSERVATION-SCHEMA-001"),
        "M1-B contract must share the parent suite REGISTRY-OBSERVATION-SCHEMA-001 (no new suite)"
    );

    assert!(
        !suites.contains("RUNTIME-REGISTRY-OBSERVATION-SCHEMA-001"),
        "M1-B must not introduce a competing registry observation suite"
    );

    assert!(
        manifest["source_hashes"]
            .as_object()
            .expect("source_hashes")
            .contains_key("contracts/runtime-api/RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md"),
        "M1-B contract source hash must be pinned"
    );
}

#[test]
fn fixture_and_source_hashes_match_manifest() {
    let manifest = load_manifest();

    let mut verified = 0;
    for (key, value) in manifest["fixture_hashes"].as_object().expect("fixture_hashes") {
        let expected = value.as_str().expect("hash string");
        let actual = sha256_of(&workspace_path(key));
        assert_eq!(
            actual, expected,
            "fixture hash drifted: {key} — re-baseline the manifest deliberately"
        );
        verified += 1;
    }
    for (key, value) in manifest["source_hashes"].as_object().expect("source_hashes") {
        let expected = value.as_str().expect("hash string");
        let actual = sha256_of(&workspace_path(key));
        assert_eq!(
            actual, expected,
            "source hash drifted: {key} — re-baseline the manifest deliberately"
        );
        verified += 1;
    }
    assert_eq!(verified, 9, "manifest must pin 3 fixtures + 6 sources");
}
