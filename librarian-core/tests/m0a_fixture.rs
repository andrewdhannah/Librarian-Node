//! M0A fixture-driven conformance test (RUST-M0-1 → RUST-M0-8 path).
//!
//! Given `conformance/fixtures/startup/canonical-startup-input.json`, the
//! `StartupEngine` must produce a startup receipt whose deterministic facts
//! match `conformance/fixtures/startup/expected-startup-receipt.json`
//! (per `conformance/fixtures/startup/README.md` field policy).

use librarian_contracts::node::{StartupReceipt, StartupStatus};
use librarian_core::startup::{
    canonical_schema, CapabilitiesFile, DatabaseContext, GovernanceSync, NodeIdentityFile,
    StartupContext, StartupEngine,
};

/// Workspace-relative fixture paths (anchored at the crate manifest).
const FIXTURE_INPUT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../conformance/fixtures/startup/canonical-startup-input.json"
);
const FIXTURE_EXPECTED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../conformance/fixtures/startup/expected-startup-receipt.json"
);

fn load_fixture_input() -> serde_json::Value {
    let text = std::fs::read_to_string(FIXTURE_INPUT).expect("read canonical-startup-input.json");
    serde_json::from_str(&text).expect("parse canonical-startup-input.json")
}

fn build_context(fixture: &serde_json::Value, db_path: std::path::PathBuf) -> StartupContext {
    let node_dir = &fixture["node_directory"];
    let identity: NodeIdentityFile =
        serde_json::from_value(node_dir["node-identity.json"].clone()).expect("parse node identity");
    let governance: GovernanceSync =
        serde_json::from_value(node_dir["governance-sync.json"].clone()).expect("parse governance");
    let capabilities: CapabilitiesFile =
        serde_json::from_value(node_dir["capabilities.json"].clone()).expect("parse capabilities");

    StartupContext {
        identity,
        governance,
        capabilities,
        expected_platform: fixture["platform"].as_str().expect("platform").to_string(),
        expected_governance_commit: fixture["expected_governance_commit"]
            .as_str()
            .expect("governance commit")
            .to_string(),
        database: DatabaseContext {
            path: db_path,
            schema_sql: canonical_schema(),
        },
    }
}

#[test]
fn m0a_canonical_input_produces_canonical_receipt() {
    let fixture = load_fixture_input();
    let tmp = tempfile::tempdir().expect("temp dir");
    let context = build_context(&fixture, tmp.path().join("runtime-node.db"));

    let outcome = StartupEngine::run(&context).expect("startup engine must not fail");
    let receipt = &outcome.receipt;

    // Schema conformance.
    receipt.validate().expect("receipt must conform to RECEIPT-SCHEMA-001");

    // Deterministic equivalence: actual receipt facts == expected receipt facts.
    let expected: StartupReceipt =
        serde_json::from_str(&std::fs::read_to_string(FIXTURE_EXPECTED).expect("read expected"))
            .expect("parse expected-startup-receipt.json");
    assert_eq!(
        receipt.deterministic_facts(),
        expected.deterministic_facts(),
        "deterministic facts must match the expected receipt exactly"
    );

    // Contract outcomes declared by the fixture's startup_contract section.
    let contract = &fixture["startup_contract"];
    assert_eq!(receipt.checks_passed, contract["expected_checks_passed"].as_u64().unwrap() as u32);
    assert_eq!(receipt.checks_failed, contract["expected_checks_failed"].as_u64().unwrap() as u32);
    assert_eq!(receipt.status, contract["expected_status"].as_str().unwrap());
    assert_eq!(receipt.status, StartupStatus::GovernedExecution.as_str());
    assert_eq!(receipt.startup_phase, "complete");

    // Reference-format variable fields.
    assert!(receipt.receipt_id.starts_with("WINDOWS-STARTUP-"));
    assert!(!receipt.timestamp.is_empty());

    // Audit trail: exactly one check per phase, all passed.
    assert_eq!(outcome.checks.len(), 6);
    assert!(outcome.checks.iter().all(|c| c.passed), "all phases must pass");
}

#[test]
fn m0a_database_applies_canonical_schema() {
    // The environment phase must produce a database with the full canonical
    // schema (9 Phase-2 + 2 Phase-3 tables).
    let fixture = load_fixture_input();
    let tmp = tempfile::tempdir().expect("temp dir");
    let db_path = tmp.path().join("runtime-node.db");
    let context = build_context(&fixture, db_path.clone());

    let outcome = StartupEngine::run(&context).expect("startup engine must not fail");
    assert!(outcome.receipt.environment_validated);

    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let table_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .expect("count tables");
    assert_eq!(table_count, 11, "canonical schema must create exactly 11 tables");

    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("integrity check");
    assert_eq!(integrity, "ok");
}

#[test]
fn m0a_engine_is_deterministic_across_runs() {
    // Same input, fresh database → identical deterministic facts. The variable
    // fields (receipt_id, timestamp) are second-precision per the reference
    // format, so two runs within the same second MAY coincide — the conformance
    // claim is that the deterministic surface never varies.
    let fixture = load_fixture_input();

    let tmp_a = tempfile::tempdir().expect("temp dir a");
    let ctx_a = build_context(&fixture, tmp_a.path().join("runtime-node.db"));
    let receipt_a = StartupEngine::execute(&ctx_a).expect("run a");

    let tmp_b = tempfile::tempdir().expect("temp dir b");
    let ctx_b = build_context(&fixture, tmp_b.path().join("runtime-node.db"));
    let receipt_b = StartupEngine::execute(&ctx_b).expect("run b");

    assert_eq!(receipt_a.deterministic_facts(), receipt_b.deterministic_facts());
    // Variable fields stay format-valid in both runs.
    for receipt in [&receipt_a, &receipt_b] {
        receipt.validate().expect("receipt must conform to RECEIPT-SCHEMA-001");
        assert!(receipt.receipt_id.starts_with("WINDOWS-STARTUP-"));
        assert!(!receipt.timestamp.is_empty());
    }
}
