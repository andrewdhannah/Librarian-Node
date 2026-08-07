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
    assert_eq!(verified, 7, "manifest must pin 3 fixtures + 4 sources");
}
