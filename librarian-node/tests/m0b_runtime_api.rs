//! RUST-M0B integration tests — Runtime API over the sealed startup outcome.
//!
//! Validates `RUNTIME-API-CONTRACT-001` (suite `RUNTIME-API-SCHEMA-001`):
//!
//! - RUST-M0B-2: read-only runtime projections (health/status/receipt).
//! - RUST-M0B-3: `STARTUP_COMPLETE → SERVABLE_RUNTIME` — a failed startup is
//!   never servable (defensive 503 branch; the process exits pre-bind in the
//!   real binary).
//! - RUST-M0B-4: responses conform to the contract (provenance fields,
//!   deterministic facts, governed-availability health, unmodified receipt).
//! - RUST-M0B-6: evidence-backed validation — the receipt served is the exact
//!   receipt sealed at startup (no regeneration / second evidence event).

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use librarian_node::runtime_api::{RuntimeApiState, router};
use librarian_node::startup::{NodeStartupOptions, run_node_startup};
use serde_json::Value;
use tower::ServiceExt;

const FIXTURE_INPUT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../conformance/fixtures/startup/canonical-startup-input.json"
);
const GOVERNED_COMMIT: &str = "6be76216a8048492526c4ca0ae751b6d2d507185";

fn load_fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE_INPUT).expect("read fixture input"))
        .expect("parse canonical-startup-input.json")
}

/// Write the canonical fixture node files into a temp node directory and
/// return `NodeStartupOptions` pointing at it.
fn fixture_options(tmp: &Path, governance_commit: &str) -> NodeStartupOptions {
    let fixture = load_fixture();
    let node_dir_value = &fixture["node_directory"];
    let node_dir = tmp.join("node");
    std::fs::create_dir_all(&node_dir).expect("create node dir");
    for name in ["node-identity.json", "governance-sync.json", "capabilities.json"] {
        let value = &node_dir_value[name];
        std::fs::write(
            node_dir.join(name),
            serde_json::to_string_pretty(value).expect("serialize fixture file"),
        )
        .expect("write fixture file");
    }
    NodeStartupOptions {
        node_dir: node_dir.clone(),
        governance_sync: node_dir.join("governance-sync.json"),
        capability_db: tmp.join("capability-registry.sqlite"),
        evidence_dir: tmp.join("evidence"),
        platform: "windows".to_string(),
        governance_commit: governance_commit.to_string(),
    }
}

async fn get_json(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .expect("build GET request"),
        )
        .await
        .expect("send GET request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("read response body");
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// Contract format: RFC 3339 UTC, seconds precision, `Z` designator
/// (e.g. `2026-08-06T22:40:00Z`).
fn is_rfc3339(value: &str) -> bool {
    value.len() == 20 && value.ends_with('Z') && value.as_bytes().get(10) == Some(&b'T')
}

#[test]
fn m0b_health_reports_governed_availability() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_node_startup(&fixture_options(tmp.path(), GOVERNED_COMMIT)).expect("startup");
    assert_eq!(outcome.receipt.status, "GOVERNED_EXECUTION");

    let app = router(std::sync::Arc::new(RuntimeApiState::from_outcome(outcome)));

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let (status, body) = rt.block_on(get_json(&app, "/health"));

    // Contract §3.1: healthy only in governed availability state.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["health"], "ok");
    assert_eq!(body["runtime_state"], "SERVABLE_RUNTIME");
    assert_eq!(body["node_id"], "WINPC-BIG-PICKLE");
    assert!(is_rfc3339(body["observed_at"].as_str().expect("observed_at")));
}

#[test]
fn m0b_status_serves_provenance_from_sealed_receipt() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_node_startup(&fixture_options(tmp.path(), GOVERNED_COMMIT)).expect("startup");
    let receipt = outcome.receipt.clone();

    let app = router(std::sync::Arc::new(RuntimeApiState::from_outcome(outcome)));

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let (status, body) = rt.block_on(get_json(&app, "/runtime/status"));

    // Contract §4.2: deterministic fields MUST match the sealed receipt.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["node_id"], receipt.node_id);
    assert_eq!(body["runtime_state"], "SERVABLE_RUNTIME");
    assert_eq!(body["startup_receipt_id"], receipt.receipt_id);
    assert_eq!(body["governance_commit"], receipt.governance_commit);
    assert_eq!(body["startup_status"], receipt.status);
    assert_eq!(body["checks_passed"], receipt.checks_passed);
    assert_eq!(body["checks_failed"], receipt.checks_failed);
    assert!(is_rfc3339(body["observed_at"].as_str().expect("observed_at")));
    assert_eq!(body.as_object().expect("object").len(), 8, "contract §4.2 has exactly 8 fields");
}

#[test]
fn m0b_receipt_returns_exact_sealed_receipt() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_node_startup(&fixture_options(tmp.path(), GOVERNED_COMMIT)).expect("startup");
    let sealed = serde_json::to_value(&outcome.receipt).expect("serialize sealed receipt");

    let app = router(std::sync::Arc::new(RuntimeApiState::from_outcome(outcome)));

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let (status, body) = rt.block_on(get_json(&app, "/runtime/receipt"));

    // Contract §4.3: the API returns the receipt it observed — no wrapping,
    // no regeneration (same receipt_id, same timestamp, same bytes).
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, sealed, "receipt served must be the exact sealed receipt");
}

#[test]
fn m0b_status_queries_differ_only_in_observed_at() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_node_startup(&fixture_options(tmp.path(), GOVERNED_COMMIT)).expect("startup");
    let app = router(std::sync::Arc::new(RuntimeApiState::from_outcome(outcome)));

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let (s1, mut b1) = rt.block_on(get_json(&app, "/runtime/status"));
    let (s2, mut b2) = rt.block_on(get_json(&app, "/runtime/status"));
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);

    // Contract refinement: two status queries can differ only in observation
    // metadata — every deterministic field must be identical.
    let o1 = b1["observed_at"].as_str().expect("observed_at 1").to_string();
    let o2 = b2["observed_at"].as_str().expect("observed_at 2").to_string();
    b1.as_object_mut().expect("obj1").remove("observed_at");
    b2.as_object_mut().expect("obj2").remove("observed_at");
    assert_eq!(b1, b2, "queries must differ only in observed_at");
    assert!(is_rfc3339(&o1) && is_rfc3339(&o2), "observed_at must be RFC 3339 UTC");
}

#[test]
fn m0b_state_changing_verbs_rejected() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_node_startup(&fixture_options(tmp.path(), GOVERNED_COMMIT)).expect("startup");
    let app = router(std::sync::Arc::new(RuntimeApiState::from_outcome(outcome)));

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    for uri in ["/health", "/runtime/status", "/runtime/receipt"] {
        let response = rt
            .block_on(app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .body(Body::empty())
                    .expect("build POST request"),
            ))
            .expect("send POST request");
        // Contract §3: state-changing verbs → 405 with Allow: GET.
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "POST {uri}");
        let allow = response
            .headers()
            .get("allow")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(allow.contains("GET"), "Allow header must advertise GET, got {allow:?}");
    }
}

#[test]
fn m0b_unknown_paths_are_404() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_node_startup(&fixture_options(tmp.path(), GOVERNED_COMMIT)).expect("startup");
    let app = router(std::sync::Arc::new(RuntimeApiState::from_outcome(outcome)));

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let (status, _) = rt.block_on(get_json(&app, "/runtime/nonexistent"));
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[test]
fn m0b_failed_startup_is_never_servable() {
    // RUST-M0B-3: the boundary transition requires GOVERNED_EXECUTION. A
    // receipt with any failed check must NOT be servable — the API reports the
    // defensive 503 (the real process exits pre-bind before this can be hit).
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_node_startup(&fixture_options(
        tmp.path(),
        "0000000000000000000000000000000000000000", // wrong commit → governance fails
    ))
    .expect("startup engine returns an outcome even on failure");
    assert_ne!(outcome.receipt.status, "GOVERNED_EXECUTION");

    let state = RuntimeApiState::from_outcome(outcome);
    assert!(!state.lifecycle().is_servable());

    let app = router(std::sync::Arc::new(state));
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let (status, body) = rt.block_on(get_json(&app, "/health"));
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["runtime_state"], "STARTUP_FAILED");
    assert_eq!(body["health"], "unavailable");

    let (status, _) = rt.block_on(get_json(&app, "/runtime/status"));
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    let (status, _) = rt.block_on(get_json(&app, "/runtime/receipt"));
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn m0b_evidence_receipt_matches_api_receipt() {
    // RUST-M0B-6: evidence-backed validation — the receipt the API serves is
    // the exact artifact written to the evidence directory at startup (one
    // evidence event, no regeneration).
    let tmp = tempfile::tempdir().expect("temp dir");
    let options = fixture_options(tmp.path(), GOVERNED_COMMIT);
    let outcome = run_node_startup(&options).expect("startup");
    let app = router(std::sync::Arc::new(RuntimeApiState::from_outcome(outcome)));

    let evidence_files: Vec<PathBuf> = std::fs::read_dir(&options.evidence_dir)
        .expect("read evidence dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    assert_eq!(evidence_files.len(), 1, "startup must write exactly one receipt");

    let evidence: Value =
        serde_json::from_str(&std::fs::read_to_string(&evidence_files[0]).expect("read evidence"))
            .expect("parse evidence receipt");

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let (status, body) = rt.block_on(get_json(&app, "/runtime/receipt"));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body, evidence,
        "API receipt must equal the evidence artifact sealed at startup"
    );
}
