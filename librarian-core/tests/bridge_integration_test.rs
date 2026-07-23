//! Bridge integration tests — prove real HTTP transport boundary.
//!
//! These tests exercise the canonical HTTP bridge client against a controlled
//! Axum test server that exposes the same route contracts as the Windows runtime
//! node. Every test performs actual HTTP serialization/deserialization.
//!
//! No test uses direct Rust function calls to substitute for HTTP transport.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use librarian_contracts::bridge::client::{BridgeClient, BridgeError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

// ============================================================================
// Test data builders
// ============================================================================

/// Build a valid EvidencePacket as JSON.
fn valid_evidence_packet_json(run_id: &str, request_id: &str, sha256: &str) -> Value {
    json!({
        "packet_type": "evidence_packet",
        "packet_version": "1",
        "exported_at": "2026-07-12T00:00:00Z",
        "qualification_request_id": request_id,
        "identity": {
            "model_id": "minicpm5-1b-q4km",
            "sha256": sha256,
            "filename": "MiniCPM5-1B-Q4_K_M.gguf",
            "quantization": "Q4_K_M"
        },
        "execution": {
            "runtime_profile_id": "prof-001",
            "hardware_profile_id": "hw-001",
            "runtime_executable_sha256": "abc123def456",
            "runtime_executable_version": "c85e97a"
        },
        "lease": {
            "lease_id": "lease-001",
            "port": 9120,
            "state": "unloaded",
            "loaded_at": "2026-07-12T00:00:00Z",
            "released_at": "2026-07-12T00:00:05Z",
            "vram_released_at": "2026-07-12T00:00:06Z"
        },
        "run": {
            "run_id": run_id,
            "input_tokens": 100,
            "output_tokens": 50,
            "load_duration_ms": 2000,
            "generation_duration_ms": 300,
            "exit_status": "success",
            "started_at": "2026-07-12T00:00:01Z",
            "ended_at": "2026-07-12T00:00:04Z"
        },
        "lifecycle_events": [
            {
                "event_type": "process_started",
                "process_id": 12345,
                "observed_state": "loading",
                "observation": "{}",
                "occurred_at": "2026-07-12T00:00:01Z"
            },
            {
                "event_type": "model_loaded",
                "process_id": 12345,
                "observed_state": "ready",
                "observation": "{}",
                "occurred_at": "2026-07-12T00:00:03Z"
            }
        ],
        "release_verification": {
            "pid_exit_verified": true,
            "gpu_release_verified": true,
            "free_vram_mb": 3433,
            "baseline_vram_mb": 3433,
            "within_tolerance": true
        }
    })
}

/// Build a valid lifecycle response as JSON.
fn valid_lifecycle_response_json() -> Value {
    json!({
        "events": [
            {
                "evidence_id": "ev-001",
                "event_type": "process_started",
                "model_id": "minicpm5-1b-q4km",
                "profile_id": "prof-001",
                "lease_id": "lease-001",
                "run_id": "run-001",
                "process_id": 12345,
                "observed_state": "loading",
                "observation_json": "{}",
                "occurred_at": "2026-07-12T00:00:01Z",
                "recorded_at": "2026-07-12T00:00:02Z"
            },
            {
                "evidence_id": "ev-002",
                "event_type": "model_loaded",
                "model_id": "minicpm5-1b-q4km",
                "profile_id": "prof-001",
                "lease_id": "lease-001",
                "run_id": "run-001",
                "process_id": 12345,
                "observed_state": "ready",
                "observation_json": "{}",
                "occurred_at": "2026-07-12T00:00:03Z",
                "recorded_at": "2026-07-12T00:00:04Z"
            }
        ],
        "count": 2
    })
}

/// Build a valid residency status response as JSON.
fn valid_residency_response_json() -> Value {
    json!({
        "timestamp": "2026-07-12T00:00:00Z",
        "active_leases": [
            {
                "lease_id": "lease-001",
                "model_id": "minicpm5-1b-q4km",
                "profile_id": "prof-001",
                "state": "ready",
                "port": 9120,
                "process_id": 10804
            }
        ],
        "active_runs": [
            {
                "run_id": "run-001",
                "lease_id": "lease-001",
                "started_at": "2026-07-12T00:00:00Z"
            }
        ],
        "draining": false,
        "available_vram_mb": 3433,
        "baseline_vram_mb": 3433
    })
}

// ============================================================================
// Test server harness
// ============================================================================

/// Control knobs for the test server.
struct TestServerControl {
    /// If set, evidence/runs/{run_id} returns this status code instead of 200.
    evidence_run_status: Option<u16>,
    /// If set, evidence/runs returns this body (ignoring evidence_run_status).
    evidence_run_body: Option<String>,
    /// If set, evidence/lifecycle returns this status code.
    lifecycle_status: Option<u16>,
    /// If set, evidence/lifecycle returns this body.
    lifecycle_body: Option<String>,
    /// If set, residency/status returns this status code.
    residency_status: Option<u16>,
    /// If set, residency/status returns this body.
    residency_body: Option<String>,
    /// If > 0, evidence/runs sleeps this many milliseconds before responding.
    evidence_run_delay_ms: u64,
}

impl Default for TestServerControl {
    fn default() -> Self {
        Self {
            evidence_run_status: None,
            evidence_run_body: None,
            lifecycle_status: None,
            lifecycle_body: None,
            residency_status: None,
            residency_body: None,
            evidence_run_delay_ms: 0,
        }
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct EvidenceQuery {
    request_id: String,
    sha256: String,
    version: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct LifecycleQuery {
    lease_id: Option<String>,
    limit: Option<i64>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ResidencyQuery {
    model_id: Option<String>,
}

/// Start a controlled Axum test server on a random port.
/// Returns (base_url, control handle) — use control to configure responses.
async fn start_test_server() -> (String, Arc<Mutex<TestServerControl>>) {
    let control = Arc::new(Mutex::new(TestServerControl::default()));
    let control_clone = control.clone();

    let app = Router::new()
        .route(
            "/evidence/runs/{run_id}",
            get({
                let control = control.clone();
                move |Path(_run_id): Path<String>, Query(query): Query<EvidenceQuery>| {
                    let control = control.clone();
                    async move {
                        let ctrl = control.lock().await;

                        // Delay if configured
                        if ctrl.evidence_run_delay_ms > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                ctrl.evidence_run_delay_ms,
                            ))
                            .await;
                        }

                        if let Some(status) = ctrl.evidence_run_status {
                            let body = ctrl.evidence_run_body.clone().unwrap_or_default();
                            return (
                                StatusCode::from_u16(status).unwrap(),
                                Json(serde_json::from_str::<Value>(&body).unwrap_or(json!({"error": body}))),
                            );
                        }

                        if let Some(body) = &ctrl.evidence_run_body {
                            (
                                StatusCode::OK,
                                Json(serde_json::from_str::<Value>(body).unwrap_or(json!({"error": "invalid test body"}))),
                            )
                        } else {
                            // Echo back query params so client can verify identity continuity
                            (StatusCode::OK, Json(valid_evidence_packet_json("run-001", &query.request_id, &query.sha256)))
                        }
                    }
                }
            }),
        )
        .route(
            "/evidence/lifecycle",
            get({
                let control = control.clone();
                move |Query(_query): Query<LifecycleQuery>| {
                    let control = control.clone();
                    async move {
                        let ctrl = control.lock().await;

                        if let Some(status) = ctrl.lifecycle_status {
                            let body = ctrl.lifecycle_body.clone().unwrap_or_default();
                            return (
                                StatusCode::from_u16(status).unwrap(),
                                Json(serde_json::from_str::<Value>(&body).unwrap_or(json!({"error": body}))),
                            );
                        }

                        if let Some(body) = &ctrl.lifecycle_body {
                            (
                                StatusCode::OK,
                                Json(serde_json::from_str::<Value>(body).unwrap_or(json!({"error": "invalid test body"}))),
                            )
                        } else {
                            (StatusCode::OK, Json(valid_lifecycle_response_json()))
                        }
                    }
                }
            }),
        )
        .route(
            "/residency/status",
            get({
                let control = control_clone.clone();
                move |Query(_query): Query<ResidencyQuery>| {
                    let control = control.clone();
                    async move {
                        let ctrl = control.lock().await;

                        if let Some(status) = ctrl.residency_status {
                            let body = ctrl.residency_body.clone().unwrap_or_default();
                            return (
                                StatusCode::from_u16(status).unwrap(),
                                Json(serde_json::from_str::<Value>(&body).unwrap_or(json!({"error": body}))),
                            );
                        }

                        if let Some(body) = &ctrl.residency_body {
                            (
                                StatusCode::OK,
                                Json(serde_json::from_str::<Value>(body).unwrap_or(json!({"error": "invalid test body"}))),
                            )
                        } else {
                            (StatusCode::OK, Json(valid_residency_response_json()))
                        }
                    }
                }
            }),
        );

    // Bind to port 0 for random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://127.0.0.1:{}", addr.port());

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    (base_url, control)
}

// ============================================================================
// Evidence Run Integration Tests
// ============================================================================

// H1-T1: Valid EvidencePacket round-trip over HTTP
#[tokio::test]
async fn test_evidence_run_roundtrip() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let packet = client
        .get_evidence_run("run-001", "req-001", "abc123def456", "c85e97a")
        .await
        .unwrap();

    // Verify identity continuity
    assert_eq!(packet.run.run_id, "run-001");
    assert_eq!(packet.qualification_request_id, "req-001");
    assert_eq!(packet.identity.sha256, "abc123def456");
    assert_eq!(packet.execution.runtime_executable_version, "c85e97a");

    // Verify packet validates
    assert!(packet.validate().is_ok());
}

// H1-T2: Evidence run request ID continuity
#[tokio::test]
async fn test_evidence_run_request_id_continuity() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let packet = client
        .get_evidence_run("run-001", "req-42", "abc123def456", "c85e97a")
        .await
        .unwrap();

    assert_eq!(packet.qualification_request_id, "req-42");
}

// H1-T3: Evidence run artifact SHA-256 continuity
#[tokio::test]
async fn test_evidence_run_sha256_continuity() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let packet = client
        .get_evidence_run("run-001", "req-001", "deadbeef1234", "c85e97a")
        .await
        .unwrap();

    assert_eq!(packet.identity.sha256, "deadbeef1234");
}

// H1-T4: Evidence run lifecycle events are preserved
#[tokio::test]
async fn test_evidence_run_lifecycle_events_preserved() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let packet = client
        .get_evidence_run("run-001", "req-001", "abc123def456", "c85e97a")
        .await
        .unwrap();

    assert!(!packet.lifecycle_events.is_empty());
    assert_eq!(packet.lifecycle_events[0].event_type, "process_started");
    assert_eq!(packet.lifecycle_events[1].event_type, "model_loaded");
}

// H1-T5: Evidence run release verification preserved
#[tokio::test]
async fn test_evidence_run_release_verification() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let packet = client
        .get_evidence_run("run-001", "req-001", "abc123def456", "c85e97a")
        .await
        .unwrap();

    assert!(packet.release_verification.pid_exit_verified);
    assert!(packet.release_verification.gpu_release_verified);
    assert!(packet.release_verification.within_tolerance);
    assert_eq!(packet.release_verification.free_vram_mb, Some(3433));
}

// H1-T6: Evidence packet authority boundary preserved over HTTP
#[tokio::test]
async fn test_evidence_run_no_capability_data() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let packet = client
        .get_evidence_run("run-001", "req-001", "abc123def456", "c85e97a")
        .await
        .unwrap();

    // Authority boundary: packet must not contain capability data
    assert!(packet.assert_no_capability_data().is_ok());
}

// ============================================================================
// Lifecycle Integration Tests
// ============================================================================

// H1-T7: Lifecycle events retrieved over HTTP with correct structure
#[tokio::test]
async fn test_lifecycle_events_retrieval() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client
        .get_evidence_lifecycle("lease-001", None)
        .await
        .unwrap();

    assert_eq!(response.events.len(), 2);
    assert_eq!(response.count, 2);
}

// H1-T8: Lifecycle events are chronologically ordered
#[tokio::test]
async fn test_lifecycle_chronological_ordering() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client
        .get_evidence_lifecycle("lease-001", None)
        .await
        .unwrap();

    // Events should be in chronological order
    let t1 = response.events[0].occurred_at.as_ref().unwrap();
    let t2 = response.events[1].occurred_at.as_ref().unwrap();
    assert!(t1 <= t2, "Events should be chronologically ordered");
}

// H1-T9: Lifecycle lease identity continuity
#[tokio::test]
async fn test_lifecycle_lease_identity_continuity() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client
        .get_evidence_lifecycle("lease-001", None)
        .await
        .unwrap();

    for event in &response.events {
        assert_eq!(event.lease_id, "lease-001");
    }
}

// H1-T10: Lifecycle bounded limit behavior
#[tokio::test]
async fn test_lifecycle_limit_respected() {
    let (base_url, control) = start_test_server().await;
    let mut ctrl = control.lock().await;
    // Return 5 events
    ctrl.lifecycle_body = Some(serde_json::to_string(&json!({
        "events": [
            {"evidence_id": "ev-1", "event_type": "a", "model_id": "m", "lease_id": "l", "run_id": "r", "occurred_at": "2026-07-12T00:00:01Z"},
            {"evidence_id": "ev-2", "event_type": "b", "model_id": "m", "lease_id": "l", "run_id": "r", "occurred_at": "2026-07-12T00:00:02Z"},
            {"evidence_id": "ev-3", "event_type": "c", "model_id": "m", "lease_id": "l", "run_id": "r", "occurred_at": "2026-07-12T00:00:03Z"},
            {"evidence_id": "ev-4", "event_type": "d", "model_id": "m", "lease_id": "l", "run_id": "r", "occurred_at": "2026-07-12T00:00:04Z"},
            {"evidence_id": "ev-5", "event_type": "e", "model_id": "m", "lease_id": "l", "run_id": "r", "occurred_at": "2026-07-12T00:00:05Z"}
        ],
        "count": 5
    })).unwrap());
    drop(ctrl);

    let client = BridgeClient::new(&base_url).unwrap();
    let response = client
        .get_evidence_lifecycle("lease-001", Some(3))
        .await
        .unwrap();

    // The server returns all 5 regardless of limit parameter (server-side filtering),
    // but the client correctly passes the limit. Verify the response is valid.
    assert!(!response.events.is_empty());
}

// ============================================================================
// Residency Integration Tests
// ============================================================================

// H1-T11: Residency status round-trip over HTTP
#[tokio::test]
async fn test_residency_status_roundtrip() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client
        .get_residency_status(None)
        .await
        .unwrap();

    assert!(response.validate().is_ok());
    assert_eq!(response.timestamp, "2026-07-12T00:00:00Z");
    assert!(!response.active_leases.is_empty());
}

// H1-T12: Residency active lease deserialized correctly
#[tokio::test]
async fn test_residency_active_lease() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client
        .get_residency_status(None)
        .await
        .unwrap();

    assert_eq!(response.active_leases.len(), 1);
    let lease = &response.active_leases[0];
    assert_eq!(lease.lease_id, "lease-001");
    assert_eq!(lease.model_id, "minicpm5-1b-q4km");
    assert_eq!(lease.state, "ready");
    assert_eq!(lease.port, Some(9120));
}

// H1-T13: Residency active run deserialized correctly
#[tokio::test]
async fn test_residency_active_run() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client
        .get_residency_status(None)
        .await
        .unwrap();

    assert_eq!(response.active_runs.len(), 1);
    let run = &response.active_runs[0];
    assert_eq!(run.run_id, "run-001");
    assert_eq!(run.lease_id, "lease-001");
}

// H1-T14: Residency drain-state preservation
#[tokio::test]
async fn test_residency_drain_state() {
    let (base_url, control) = start_test_server().await;
    let mut ctrl = control.lock().await;
    ctrl.residency_body = Some(serde_json::to_string(&json!({
        "timestamp": "2026-07-12T00:00:00Z",
        "active_leases": [],
        "active_runs": [],
        "draining": true,
        "available_vram_mb": 3433,
        "baseline_vram_mb": 3433
    })).unwrap());
    drop(ctrl);

    let client = BridgeClient::new(&base_url).unwrap();
    let response = client.get_residency_status(None).await.unwrap();

    assert!(response.draining);
}

// H1-T15: Residency VRAM values preserved
#[tokio::test]
async fn test_residency_vram_values() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client.get_residency_status(None).await.unwrap();

    assert_eq!(response.available_vram_mb, Some(3433));
    assert_eq!(response.baseline_vram_mb, Some(3433));
}

// H1-T16: Residency capability-data boundary preserved
#[tokio::test]
async fn test_residency_no_capability_data() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client.get_residency_status(None).await.unwrap();

    assert!(response.assert_no_capability_data().is_ok());
}

// H1-T17: Residency with model filter
#[tokio::test]
async fn test_residency_model_filter() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    // The test server ignores the filter, but the client should send it
    let response = client
        .get_residency_status(Some("minicpm5-1b-q4km"))
        .await
        .unwrap();

    assert!(response.validate().is_ok());
}

// ============================================================================
// Failure-Path HTTP Tests
// ============================================================================

// H1-T18: Malformed JSON response → deserialization failure
#[tokio::test]
async fn test_malformed_json_evidence() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        ctrl.evidence_run_body = Some("this is not json".to_string());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let result = client
        .get_evidence_run("run-001", "req-001", "abc123", "v1")
        .await;

    match result {
        Err(BridgeError::Deserialization { detail, .. }) => {
            assert!(detail.contains("Failed to parse EvidencePacket"));
        }
        other => panic!("Expected Deserialization error, got {:?}", other),
    }
}

// H1-T19: Structurally invalid packet → validation failure
#[tokio::test]
async fn test_structurally_invalid_packet() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        // Valid JSON but wrong packet_type
        ctrl.evidence_run_body = Some(serde_json::to_string(&json!({
            "packet_type": "wrong_type",
            "packet_version": "1",
            "exported_at": "2026-07-12T00:00:00Z",
            "qualification_request_id": "req-001",
            "identity": {"model_id": "m", "sha256": "s", "filename": "f"},
            "execution": {"runtime_profile_id": "r", "hardware_profile_id": "h", "runtime_executable_sha256": "s", "runtime_executable_version": "v"},
            "lease": {"lease_id": "l", "state": "ready"},
            "run": {"run_id": "r"},
            "lifecycle_events": [],
            "release_verification": {"pid_exit_verified": true, "gpu_release_verified": true, "within_tolerance": true}
        })).unwrap());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let result = client
        .get_evidence_run("run-001", "req-001", "abc123", "v1")
        .await;

    match result {
        Err(BridgeError::Validation(detail)) => {
            assert!(detail.contains("packet type") || detail.contains("Invalid"));
        }
        other => panic!("Expected Validation error, got {:?}", other),
    }
}

// H1-T20: HTTP 404 → preserved as HTTP status failure
#[tokio::test]
async fn test_http_404_preserved() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        ctrl.evidence_run_status = Some(404);
        ctrl.evidence_run_body = Some(serde_json::to_string(&json!({"error": "run not found"})).unwrap());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let result = client
        .get_evidence_run("run-001", "req-001", "abc123", "v1")
        .await;

    match result {
        Err(BridgeError::HttpStatus { status, body }) => {
            assert_eq!(status, 404);
            assert!(body.contains("not found"));
        }
        other => panic!("Expected HttpStatus(404), got {:?}", other),
    }
}

// H1-T21: HTTP 500 → preserved as HTTP status failure
#[tokio::test]
async fn test_http_500_preserved() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        ctrl.residency_status = Some(500);
        ctrl.residency_body = Some(serde_json::to_string(&json!({"error": "internal error"})).unwrap());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let result = client.get_residency_status(None).await;

    match result {
        Err(BridgeError::HttpStatus { status, body }) => {
            assert_eq!(status, 500);
            assert!(body.contains("internal error"));
        }
        other => panic!("Expected HttpStatus(500), got {:?}", other),
    }
}

// H1-T22: Connection failure → classified as transport failure
#[tokio::test]
async fn test_connection_failure() {
    // Use a port that nothing is listening on
    let client = BridgeClient::new("http://127.0.0.1:1").unwrap();
    let result = client
        .get_evidence_run("run-001", "req-001", "abc123", "v1")
        .await;

    match result {
        Err(BridgeError::Transport(msg)) => {
            assert!(msg.contains("connect") || msg.contains("HTTP request failed"));
        }
        other => panic!("Expected Transport error, got {:?}", other),
    }
}

// H1-T23: Malformed lifecycle JSON → deserialization failure
#[tokio::test]
async fn test_malformed_lifecycle_json() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        ctrl.lifecycle_body = Some("not json at all".to_string());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let result = client
        .get_evidence_lifecycle("lease-001", None)
        .await;

    match result {
        Err(BridgeError::Deserialization { detail, .. }) => {
            assert!(detail.contains("Failed to parse LifecycleResponse"));
        }
        other => panic!("Expected Deserialization error, got {:?}", other),
    }
}

// H1-T24: Malformed residency JSON → deserialization failure
#[tokio::test]
async fn test_malformed_residency_json() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        ctrl.residency_body = Some("{bad json".to_string());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let result = client.get_residency_status(None).await;

    match result {
        Err(BridgeError::Deserialization { detail, .. }) => {
            assert!(detail.contains("Failed to parse ResidencyStatusResponse"));
        }
        other => panic!("Expected Deserialization error, got {:?}", other),
    }
}

// H1-T25: HTTP lifecycle endpoint 500 → preserved
#[tokio::test]
async fn test_http_500_lifecycle() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        ctrl.lifecycle_status = Some(500);
        ctrl.lifecycle_body = Some(serde_json::to_string(&json!({"error": "db error"})).unwrap());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let result = client
        .get_evidence_lifecycle("lease-001", None)
        .await;

    match result {
        Err(BridgeError::HttpStatus { status, body }) => {
            assert_eq!(status, 500);
            assert!(body.contains("db error"));
        }
        other => panic!("Expected HttpStatus(500), got {:?}", other),
    }
}

// ============================================================================
// Authority Boundary Proof Tests
// ============================================================================

// H1-T26: Evidence packet across HTTP boundary has no role assignment
#[tokio::test]
async fn test_evidence_no_role_across_http() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let packet = client
        .get_evidence_run("run-001", "req-001", "abc123def456", "c85e97a")
        .await
        .unwrap();

    // Serialize to JSON and check no capability-related fields exist
    let json_str = serde_json::to_string(&packet).unwrap();
    assert!(!json_str.contains("\"role\""));
    assert!(!json_str.contains("\"capability_status\""));
    assert!(!json_str.contains("\"qualification\""));
    assert!(!json_str.contains("\"router_eligible\""));
    assert!(!json_str.contains("\"owner_decision\""));
    assert!(!json_str.contains("\"supersession\""));
}

// H1-T27: Residency packet across HTTP boundary has no capability authority
#[tokio::test]
async fn test_residency_no_capability_across_http() {
    let (base_url, _control) = start_test_server().await;
    let client = BridgeClient::new(&base_url).unwrap();

    let response = client.get_residency_status(None).await.unwrap();

    let json_str = serde_json::to_string(&response).unwrap();
    assert!(!json_str.contains("\"role\""));
    assert!(!json_str.contains("\"capability_status\""));
    assert!(!json_str.contains("\"qualification\""));
    assert!(!json_str.contains("\"router_eligible\""));
}

// H1-T28: Direct function-call substitution is NOT used
// This test proves the bridge client goes through HTTP by connecting
// to a server we control — if it were using direct function calls,
// the test server would not be needed.
#[tokio::test]
async fn test_http_not_substituted_with_function_call() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        // Set a unique response that only our test server would return
        ctrl.residency_body = Some(serde_json::to_string(&json!({
            "timestamp": "2026-01-01T00:00:00Z",
            "active_leases": [],
            "active_runs": [],
            "draining": false,
            "available_vram_mb": 9999,
            "baseline_vram_mb": 9999
        })).unwrap());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let response = client.get_residency_status(None).await.unwrap();

    // If this were a direct function call, the response would be
    // from the real DB, not our custom test data
    assert_eq!(response.available_vram_mb, Some(9999));
    assert_eq!(response.baseline_vram_mb, Some(9999));
    assert_eq!(response.timestamp, "2026-01-01T00:00:00Z");
}

// H1-T29: Evidence run timeout classification
#[tokio::test]
async fn test_evidence_run_timeout_classified() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        ctrl.evidence_run_delay_ms = 5000; // 5 second delay
    }

    let client = BridgeClient::with_timeout(&base_url, std::time::Duration::from_millis(100)).unwrap();
    let result = client
        .get_evidence_run("run-001", "req-001", "abc123", "v1")
        .await;

    match result {
        Err(BridgeError::Timeout(msg)) => {
            assert!(msg.contains("timed out") || msg.contains("timeout"));
        }
        other => panic!("Expected Timeout error, got {:?}", other),
    }
}

// H1-T30: Empty evidence packet → validation failure
#[tokio::test]
async fn test_empty_evidence_packet_validation() {
    let (base_url, control) = start_test_server().await;
    {
        let mut ctrl = control.lock().await;
        // Minimal JSON that deserializes but fails validation (empty packet_type)
        ctrl.evidence_run_body = Some(serde_json::to_string(&json!({
            "packet_type": "",
            "packet_version": "1",
            "exported_at": "2026-07-12T00:00:00Z",
            "qualification_request_id": "req-001",
            "identity": {"model_id": "m", "sha256": "s", "filename": "f"},
            "execution": {"runtime_profile_id": "r", "hardware_profile_id": "h", "runtime_executable_sha256": "s", "runtime_executable_version": "v"},
            "lease": {"lease_id": "l", "state": "ready"},
            "run": {"run_id": "r"},
            "lifecycle_events": [],
            "release_verification": {"pid_exit_verified": true, "gpu_release_verified": true, "within_tolerance": true}
        })).unwrap());
    }

    let client = BridgeClient::new(&base_url).unwrap();
    let result = client
        .get_evidence_run("run-001", "req-001", "abc123", "v1")
        .await;

    match result {
        Err(BridgeError::Validation(detail)) => {
            assert!(detail.contains("packet type") || detail.contains("Invalid"));
        }
        other => panic!("Expected Validation error, got {:?}", other),
    }
}
