//! Integration tests for the Rust router.
//!
//! Tests handler shape, refusal behavior, auth middleware, body limits,
//! profile serialization, and catch-all 404 response.
//!
//! Does NOT require a running llama-server or GPU — all tests use axum's
//! tower::ServiceExt::oneshot() against a constructed router with no
//! actual backend processes.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;
use librarian_node::node::{AllocationService, AnomalyDetectionService, BootstrapService, CapabilityEvidenceBridge, CoreIntegrationService, CustodyService, EvidenceClassificationService, EvidenceIntelligenceService, FleetService, FleetTrustService, ModelRuntimeService, NodeStateMachine, OperationsService, OwnerAllocationService, OwnerWorkflowService, PatternEscalationService, PolicyService, ReconciliationService, RecoveryCustodyService, RegistryApplyService, RegistrationService, SessionService, WorkloadLifecycleService, WorkloadSessionService};
use librarian_node::server::{build_router, AppState};
use librarian_node::config::{ProfileManager, RouterConfig};
use librarian_node::db::RuntimeDatabase;
use librarian_node::evidence::EvidenceWriter;
use librarian_node::operator;
use librarian_node::platform::create_detector;
use librarian_node::residency::{ModelResidencySupervisor, SupervisorConfig, RuntimeStopStrategy};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::Mutex;
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal RouterConfig for testing.
fn test_config() -> RouterConfig {
    RouterConfig {
        router_host: "127.0.0.1".to_string(),
        router_port: 9130,
        backend_port_base: 9120,
        auth_token: None,
        require_auth: false,
        max_body_bytes: 1024 * 1024,
        profile_config_path: None,
        backend_binary_path: None,
        evidence_path: None,
        log_path: None,
        health_timeout_secs: 1,
        health_check_timeout_secs: 1,
        health_poll_interval_secs: 1,
    }
}

/// Build a RouterConfig with auth enabled.
fn test_config_with_auth(token: &str) -> RouterConfig {
    let mut c = test_config();
    c.auth_token = Some(token.to_string());
    c.require_auth = true;
    c
}

/// Build a RouterConfig with a custom body limit.
fn test_config_with_body_limit(max_bytes: usize) -> RouterConfig {
    let mut c = test_config();
    c.max_body_bytes = max_bytes;
    c
}

/// Setup app state with a temporary profile config file.
/// Returns (Arc<AppState>, NamedTempFile, tempfile::TempDir) — the temp files/dirs are kept alive
/// for the duration of the test to prevent path invalidation.
async fn setup_app(config: RouterConfig) -> (Arc<AppState>, NamedTempFile, tempfile::TempDir) {

    let profiles_json = serde_json::json!({
        "profiles": [
            {
                "alias": "test-model",
                "model_file": "test.gguf",
                "model_path": "test.gguf",
                "port": 12345,
                "backend": "vulkan",
                "context": 4096,
                "ngl": 99,
                "task_classes": ["test-task"],
                "verified_status": "verified",
                "evidence_path": "fixtures/test-evidence.json",
                "limitations": "Test-only profile for integration tests",
            }
        ]
    });

    let mut temp_file = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut temp_file, &profiles_json).unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let mut test_config = config.clone();
    test_config.profile_config_path = Some(temp_path);

    let pm = ProfileManager::load_from_config(&test_config)
        .expect("Failed to load test profiles");

    let backends = Mutex::new(HashMap::new());
    let evidence_writer = EvidenceWriter::new();
    let start_time = std::time::Instant::now();
    let health_poller_handle = Mutex::new(None);

    // Use a unique temp directory for each test to avoid file collisions
    let test_dir = tempfile::tempdir().unwrap();
    let dir = |name: &str| -> PathBuf { test_dir.path().join(name) };

    let db_path = dir("test.db");
    let db = RuntimeDatabase::open(db_path).expect("Failed to open test DB");
    db.migrate().expect("Failed to migrate test DB");

    let state = Arc::new(AppState {
        profile_manager: pm,
        config: test_config,
        backends,
        evidence_writer,
        start_time,
        health_poller_handle,
        db: db.clone(),
        supervisor: ModelResidencySupervisor::new(
            SupervisorConfig {
                stop_strategy: RuntimeStopStrategy::ProcessKill,
                baseline_free_vram_mb: 3433,
                release_tolerance_mb: 100,
                process_exit_timeout: std::time::Duration::from_secs(5),
                health_timeout: std::time::Duration::from_secs(10),
                health_poll_interval: std::time::Duration::from_millis(100),
            },
            db,
        ),
        operator: std::sync::Arc::new(tokio::sync::Mutex::new(operator::OperatorService::new())),
        node_identity_service: Arc::new(librarian_node::node::NodeIdentityService::new(
            dir("test-node-identity.json"),
        )),
        node_state: Mutex::new(NodeStateMachine::new()),
        registration_service: Mutex::new(RegistrationService::new(
            dir("test-node-registration.json"),
        )),
        capability_evidence_bridge: Mutex::new(CapabilityEvidenceBridge::new(
            dir("test-capability-evidence.json"),
        )),
        session_service: Mutex::new(SessionService::new(
            dir(&format!("test-sessions-{}.json", uuid::Uuid::new_v4())),
        )),
        bootstrap_service: Mutex::new(BootstrapService::new(
            dir("test-bootstrap.json"),
            Arc::new(librarian_node::node::NodeIdentityService::new(
                dir("test-bootstrap-identity.json"),
            )),
            Arc::new(std::sync::Mutex::new(CapabilityEvidenceBridge::new(
                dir("test-bootstrap-bridge.json"),
            ))),
            Arc::new(create_detector()),
        )),
        custody_service: std::sync::Arc::new(std::sync::Mutex::new(CustodyService::new(
            dir("test-custody.json"),
        ))),
        core_integration_service: Mutex::new(CoreIntegrationService::new(
            dir("test-core-integration.json"),
            None,
        )),
        operations_service: OperationsService::new(),
        owner_workflow_service: Mutex::new(OwnerWorkflowService::new(
            dir("test-owner-workflows.json"),
        )),
        fleet_service: Mutex::new(FleetService::new(
            dir("test-fleet-inventory.json"),
        )),
        fleet_trust_service: Mutex::new(FleetTrustService::new(
            dir("test-fleet-trust.json"),
        )),
        allocation_service: Mutex::new(AllocationService::new(
            dir("test-allocation.json"),
        )),
        owner_allocation_service: Mutex::new(OwnerAllocationService::new(
            dir("test-owner-allocation.json"),
        )),
        workload_session_service: Mutex::new(WorkloadSessionService::new(
            dir("test-workload-sessions.json"),
        )),
        workload_lifecycle_service: WorkloadLifecycleService,
        evidence_intelligence_service: EvidenceIntelligenceService,
        evidence_classification_service: tokio::sync::Mutex::new(
            EvidenceClassificationService::new(dir("test-classification.json")),
        ),
        anomaly_detection_service: tokio::sync::Mutex::new(
            AnomalyDetectionService::new(dir("test-anomaly.json")),
        ),
        pattern_escalation_service: tokio::sync::Mutex::new(
            PatternEscalationService::new(dir("test-pattern-escalation.json")),
        ),
        reconciliation_service: tokio::sync::Mutex::new(
            ReconciliationService::new(dir("test-reconciliation.json"))
                .with_custody(std::sync::Arc::new(std::sync::Mutex::new(CustodyService::new(
                    dir("test-reconciliation-custody.json"),
                )))),
        ),
        recovery_custody_service: std::sync::Arc::new(std::sync::Mutex::new(
            RecoveryCustodyService::new(dir("test-recovery-custody.json"))
                .with_custody(std::sync::Arc::new(std::sync::Mutex::new(CustodyService::new(
                    dir("test-recovery-custody-custody.json"),
                )))),
        )),
        policy_service: std::sync::Arc::new(tokio::sync::Mutex::new(
            PolicyService::new(dir("test-policy.json")),
        )),
        registry_candidate_service: tokio::sync::Mutex::new(
            librarian_node::node::RegistryCandidateService::new(dir("test-registry-candidates.json")),
        ),
        registry_enforcement_service: tokio::sync::Mutex::new(
            librarian_node::node::RegistryEnforcementService::new(dir("test-registry-enforcement.json")),
        ),
        model_runtime_service: tokio::sync::Mutex::new(ModelRuntimeService::new()),
        registry_mcp_service: tokio::sync::Mutex::new(
            librarian_node::node::RegistryMcpService::new(dir("test-registry-mcp.json")),
        ),
        registry_owner_service: tokio::sync::Mutex::new(
            librarian_node::node::RegistryOwnerService::new(dir("test-registry-owner.json")),
        ),
        registry_apply_service: tokio::sync::Mutex::new(
            RegistryApplyService::new(dir("test-registry-apply.json")),
        ),
    });

    // Register the node so session endpoints can pass enforcement checks
    {
        let identity = state.node_identity_service.get_identity().clone();
        let mut reg = state.registration_service.lock().await;
        reg.submit_registration(&identity, None);
        let receipt = librarian_contracts::node::RegistrationReceipt {
            registration_id: uuid::Uuid::new_v4().to_string(),
            node_id: identity.node_id.clone(),
            status: "registered".to_string(),
            registered_at: chrono::Utc::now().to_rfc3339(),
            previous_state: Some("registration_requested".to_string()),
        };
        reg.confirm_registration(&receipt);
    }

    (state, temp_file, test_dir)
}

// ---------------------------------------------------------------------------
// Existing Auth & Body Tests (preserved from original)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_auth_middleware_success() {
    let config = test_config_with_auth("secret-token");
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // Auth middleware expects exact token match (not "Bearer " prefix)
    let req = Request::builder()
        .uri("/backend/status")
        .header(header::AUTHORIZATION, "secret-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_middleware_failure() {
    let config = test_config_with_auth("secret-token");
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/status")
        .header(header::AUTHORIZATION, "wrong-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_disabled() {
    let config = test_config_with_auth("secret-token");
    let mut disabled = config.clone();
    disabled.require_auth = false;
    let (state, _file, _db_file) = setup_app(disabled).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_max_body_bytes() {
    let config = test_config_with_body_limit(10); // Very small limit
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .method("POST")
        .uri("/backend/select")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("this is too long"))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

// ---------------------------------------------------------------------------
// New: Profile serialization shape tests (#8)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_profiles_contains_all_fields() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/profiles")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let profiles = body.get("profiles")
        .and_then(|p| p.as_array())
        .expect("profiles should be an array");

    let profile = profiles.first().expect("should have at least one profile");

    // Existing fields (must still be present)
    assert!(profile.get("alias").is_some(), "alias field missing");
    assert!(profile.get("task_classes").is_some(), "task_classes field missing");
    assert!(profile.get("verified").is_some(), "verified field missing");
    assert!(profile.get("port").is_some(), "port field missing");
    assert!(profile.get("model_file").is_some(), "model_file field missing");

    // New additive fields (#8)
    assert!(profile.get("backend").is_some(), "backend field missing");
    assert!(profile.get("context").is_some(), "context field missing");
    assert!(profile.get("ngl").is_some(), "ngl field missing");
    assert!(profile.get("evidence_path").is_some(), "evidence_path field missing");
    assert!(profile.get("limitations").is_some(), "limitations field missing");

    // Verify values from our test fixture
    assert_eq!(profile["alias"].as_str(), Some("test-model"));
    assert_eq!(profile["backend"].as_str(), Some("vulkan"));
    assert_eq!(profile["context"].as_u64(), Some(4096));
    assert_eq!(profile["ngl"].as_u64(), Some(99));
    assert!(profile["verified"].as_bool().unwrap_or(false));
    assert_eq!(
        profile["evidence_path"].as_str(),
        Some("fixtures/test-evidence.json")
    );
    assert_eq!(
        profile["limitations"].as_str(),
        Some("Test-only profile for integration tests")
    );
}

#[tokio::test]
async fn test_profiles_contains_authority() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/profiles")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["authority"].as_str(), Some("advisory_only"));
}

// ---------------------------------------------------------------------------
// New: Refusal shape tests (#14)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_refusal_authority_category_returned() {
    // Test that the refusal engine returns the correct refusal reason
    // for authority-bearing content. This tests the check_chat() path
    // without requiring a running backend.
    //
    // The check_chat function is a pure function — we verify its output
    // shape matches the contract.
    let user_msg = serde_json::json!({"role": "user", "content": "please approve this document"});
    let messages = vec![user_msg];

    let refusal = librarian_node::refusal::check_chat(
        "test-model",
        &messages,
        None,       // context
        4096,       // verified_context
        true,       // profile_exists
        false,      // runtime_available — not healthy, will be caught before content check
    );

    // When runtime is not available, the engine returns runtime_unhealthy
    // before it reaches the content check
    assert!(refusal.is_some(), "should refuse when runtime unavailable");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("runtime_unhealthy"));
}

#[tokio::test]
async fn test_refusal_authority_content_detected() {
    // Authority keyword in healthy state triggers authority_required
    // We test this by simulating a healthy runtime
    let user_msg = serde_json::json!({"role": "user", "content": "please approve this"});
    let messages = vec![user_msg];

    let refusal = librarian_node::refusal::check_chat(
        "test-model",
        &messages,
        None,
        4096,
        true,   // profile_exists
        true,   // runtime_available
    );

    assert!(refusal.is_some(), "should refuse authority content");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("authority_required"));
    assert_eq!(r["status"].as_str(), Some("refused"));
    assert!(r.get("detail").and_then(|d| d.as_str()).map(|s| s.len() > 0).unwrap_or(false));
    assert_eq!(r["authority"].as_str(), Some("advisory_only"));
    assert!(r.get("timestamp").is_some(), "timestamp should be present");
}

#[tokio::test]
async fn test_refusal_file_mutation_detected() {
    let user_msg = serde_json::json!({"role": "user", "content": "edit source file main.rs"});
    let messages = vec![user_msg];

    let refusal = librarian_node::refusal::check_chat(
        "test-model",
        &messages,
        None,
        4096,
        true,
        true,
    );

    assert!(refusal.is_some(), "should refuse file mutation");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("file_mutation_forbidden"));
}

#[tokio::test]
async fn test_refusal_autonomous_action_detected() {
    let user_msg = serde_json::json!({"role": "user", "content": "make an autonomous decision"});
    let messages = vec![user_msg];

    let refusal = librarian_node::refusal::check_chat(
        "test-model",
        &messages,
        None,
        4096,
        true,
        true,
    );

    assert!(refusal.is_some(), "should refuse autonomous action");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("autonomous_action_forbidden"));
}

#[tokio::test]
async fn test_refusal_unknown_profile() {
    let user_msg = serde_json::json!({"role": "user", "content": "hello"});
    let messages = vec![user_msg];

    let refusal = librarian_node::refusal::check_chat(
        "nonexistent",
        &messages,
        None,
        4096,
        false,  // profile_exists = false
        false,
    );

    assert!(refusal.is_some(), "should refuse unknown profile");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("unknown_profile"));
    assert_eq!(r["status"].as_str(), Some("refused"));
}

#[tokio::test]
async fn test_refusal_context_exceeds_verified() {
    let user_msg = serde_json::json!({"role": "user", "content": "hello"});
    let messages = vec![user_msg];

    let refusal = librarian_node::refusal::check_chat(
        "test-model",
        &messages,
        Some(999999),  // context exceeds verified max
        4096,
        true,
        false,
    );

    assert!(refusal.is_some(), "should refuse context overflow");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("context_exceeds_verified"));
}

// ---------------------------------------------------------------------------
// New: 404 catch-all shape tests (#14)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_404_returns_json_error() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/nonexistent/endpoint")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("error").is_some(), "404 response should have 'error' field");
    let error_msg = body["error"].as_str().unwrap_or("");
    assert!(error_msg.contains("/nonexistent/endpoint"),
        "404 error should contain the original path");
}

// ---------------------------------------------------------------------------
// New: BackendStatus serialization shape (#14)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_status_contains_contract_fields() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Contract-required fields
    assert!(body.get("status").is_some());
    assert!(body.get("profiles_registered").is_some());
    assert!(body.get("runtimes_alive").is_some());
    assert!(body.get("uptime_seconds").is_some());
    assert!(body.get("authority").is_some());
    assert!(body.get("profiles").is_some());

    assert_eq!(body["profiles_registered"].as_u64(), Some(1));
    assert_eq!(body["runtimes_alive"].as_u64(), Some(0));
    assert_eq!(body["authority"].as_str(), Some("advisory_only"));
}

// ---------------------------------------------------------------------------
// New: Node identity / status / capabilities endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_node_identity_endpoint_returns_valid_json() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/node/identity")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("node_id").is_some(), "node_id field missing");
    assert!(body.get("display_name").is_some(), "display_name field missing");
    assert!(body.get("platform").is_some(), "platform field missing");
    assert!(body.get("runtime_version").is_some(), "runtime_version field missing");
    assert!(body.get("contract_version").is_some(), "contract_version field missing");
    assert!(body.get("first_seen_at").is_some(), "first_seen_at field missing");
    assert_eq!(body["contract_version"].as_str(), Some("1"));
}

#[tokio::test]
async fn test_node_status_endpoint_returns_identity_and_state() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/node/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("identity").is_some(), "identity field missing");
    assert!(body.get("state").is_some(), "state field missing");
    assert!(body.get("uptime_seconds").is_some(), "uptime_seconds field missing");
    assert!(body.get("last_state_change").is_some(), "last_state_change field missing");

    let identity = &body["identity"];
    assert!(identity.get("node_id").is_some(), "identity.node_id missing");
    assert!(body["state"].as_str().is_some(), "state should be present");
}

#[tokio::test]
async fn test_node_capabilities_endpoint_returns_manifest() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/node/capabilities")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("node_id").is_some(), "node_id field missing");
    assert!(body.get("capabilities").is_some(), "capabilities field missing");

    let capabilities = body["capabilities"].as_array().expect("capabilities should be an array");
    assert!(!capabilities.is_empty(), "capabilities should not be empty");

    let types: Vec<&str> = capabilities.iter()
        .filter_map(|c| c.get("capability_type").and_then(|t| t.as_str()))
        .collect();
    assert!(types.contains(&"llm.inference"), "should have llm.inference capability");
    assert!(types.contains(&"hardware"), "should have hardware capability");
    assert!(types.contains(&"runtime"), "should have runtime capability");
    assert!(types.contains(&"qualification"), "should have qualification capability");
    assert!(types.contains(&"evidence-generation"), "should have evidence-generation capability");
}

#[tokio::test]
async fn test_node_endpoints_under_auth() {
    let config = test_config_with_auth("test-token");
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // Without auth token — should be unauthorized
    let req = Request::builder()
        .uri("/node/identity")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // With correct auth token — should succeed
    let req = Request::builder()
        .uri("/node/identity")
        .header(header::AUTHORIZATION, "test-token")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Registration endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_node_registration_returns_record() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/node/registration")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["registration_status"].as_str(), Some("registered"));
    assert!(body.get("node_id").is_some());
    assert!(body.get("display_name").is_some());
}

#[tokio::test]
async fn test_post_node_register_returns_receipt_and_transitions_state() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // POST /node/register
    let req = Request::builder()
        .method("POST")
        .uri("/node/register")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("registration_id").is_some(), "registration_id missing");
    assert!(body.get("node_id").is_some(), "node_id missing");
    assert_eq!(body["status"].as_str(), Some("registration_requested"));
    assert!(body.get("registered_at").is_some(), "registered_at missing");
}

#[tokio::test]
async fn test_post_node_register_confirm_transitions_to_registered() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // First, register
    let req = Request::builder()
        .method("POST")
        .uri("/node/register")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let confirm_body = serde_json::json!({
        "registration_id": receipt["registration_id"],
        "node_id": receipt["node_id"],
        "status": "registered",
        "registered_at": receipt["registered_at"],
        "previous_state": "registration_requested",
    });

    // Confirm registration
    let req = Request::builder()
        .method("POST")
        .uri("/node/register/confirm")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&confirm_body).unwrap()))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["status"].as_str(), Some("registered"));

    // Verify via GET /node/registration
    let req = Request::builder()
        .uri("/node/registration")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let record: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(record["registration_status"].as_str(), Some("registered"));
}

#[tokio::test]
async fn test_double_register_returns_conflict() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // First register succeeds
    let req = Request::builder()
        .method("POST")
        .uri("/node/register")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Second register should fail (wrong state transition)
    let req = Request::builder()
        .method("POST")
        .uri("/node/register")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

// ===========================================================================
// Session endpoint integration tests
// ===========================================================================

#[tokio::test]
async fn test_session_start_creates_session() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "node_id": "test-node",
                "agent_id": "test-agent",
                "requested_capabilities": ["llm.inference"],
                "context": "integration test session"
            }).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["state"].as_str(), Some("created"));
    assert_eq!(body["node_id"].as_str(), Some("test-node"));
    assert!(body.get("session_id").and_then(|v| v.as_str()).map(|s| s.len() > 0).unwrap_or(false));
    assert!(body.get("started_at").is_some());
    assert_eq!(body["agent_id"].as_str(), Some("test-agent"));
    assert_eq!(body["context"].as_str(), Some("integration test session"));
}

#[tokio::test]
async fn test_session_activate_and_close_lifecycle() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // Start session
    let req = Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"node_id": "test-node"}).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let session: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let session_id = session["session_id"].as_str().unwrap().to_string();

    // Activate
    let req = Request::builder()
        .method("POST")
        .uri(format!("/session/{}/activate", session_id))
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let activated: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(activated["state"].as_str(), Some("active"));

    // Close
    let req = Request::builder()
        .method("POST")
        .uri(format!("/session/{}/close", session_id))
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(receipt["session_id"].as_str(), Some(session_id.as_str()));
    assert!(receipt.get("receipt_id").is_some());
    assert_eq!(receipt["operations_executed"].as_u64(), Some(0));
}

#[tokio::test]
async fn test_session_expire_endpoint() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // Start session
    let req = Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"node_id": "test-node"}).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let session: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let session_id = session["session_id"].as_str().unwrap().to_string();

    // Expire
    let req = Request::builder()
        .method("POST")
        .uri(format!("/session/{}/expire", session_id))
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let expired: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(expired["state"].as_str(), Some("expired"));
}

#[tokio::test]
async fn test_get_session_endpoint() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // Start session
    let req = Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"node_id": "test-node"}).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let session: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let session_id = session["session_id"].as_str().unwrap().to_string();

    // GET session
    let req = Request::builder()
        .uri(format!("/session/{}", session_id))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let found: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(found["session_id"].as_str(), Some(session_id.as_str()));
}

#[tokio::test]
async fn test_sessions_list_endpoint() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // Start two sessions
    for _ in 0..2 {
        let req = Request::builder()
            .method("POST")
            .uri("/session/start")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"node_id": "test-node"}).to_string(),
            ))
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();
    }

    // List all
    let req = Request::builder()
        .uri("/sessions")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let sessions: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(sessions.as_array().map(|a| a.len()), Some(2));

    // List with state filter
    let req = Request::builder()
        .uri("/sessions?state=created")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let sessions: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(sessions.as_array().map(|a| a.len()), Some(2));
}

#[tokio::test]
async fn test_session_receipt_endpoint() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // Create and close a session
    let req = Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"node_id": "test-node"}).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let session: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let session_id = session["session_id"].as_str().unwrap().to_string();

    // Activate
    let req = Request::builder()
        .method("POST")
        .uri(format!("/session/{}/activate", session_id))
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // Close
    let req = Request::builder()
        .method("POST")
        .uri(format!("/session/{}/close", session_id))
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // Get receipt
    let req = Request::builder()
        .uri(format!("/session/{}/receipt", session_id))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(receipt["session_id"].as_str(), Some(session_id.as_str()));

    // Get all receipts
    let req = Request::builder()
        .uri("/sessions/receipts")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let receipts: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(receipts.as_array().map(|a| a.len()), Some(1));
}

#[tokio::test]
async fn test_session_not_found_returns_404() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/session/nonexistent-id")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_session_close_unactivated_returns_error() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .method("POST")
        .uri("/session/start")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"node_id": "test-node"}).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let session: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let session_id = session["session_id"].as_str().unwrap().to_string();

    // Try to close without activating
    let req = Request::builder()
        .method("POST")
        .uri(format!("/session/{}/close", session_id))
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ===========================================================================
// Operations endpoint tests (NODE-OPERATIONAL-SURFACE-1)
// ===========================================================================

#[tokio::test]
async fn test_ops_health_returns_all_components() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/ops/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("overall_status").is_some(), "overall_status missing");
    assert!(body.get("checked_at").is_some(), "checked_at missing");

    let components = body["components"].as_array().expect("components should be an array");
    let component_names: Vec<&str> = components.iter()
        .filter_map(|c| c["component"].as_str())
        .collect();

    assert!(component_names.contains(&"identity"), "should include identity");
    assert!(component_names.contains(&"registration"), "should include registration");
    assert!(component_names.contains(&"capabilities"), "should include capabilities");
    assert!(component_names.contains(&"sessions"), "should include sessions");
    assert!(component_names.contains(&"bootstrap"), "should include bootstrap");
    assert!(component_names.contains(&"custody"), "should include custody");
    assert!(component_names.contains(&"core_integration"), "should include core_integration");
}

#[tokio::test]
async fn test_ops_overview_returns_all_fields() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/ops/overview")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("node_id").is_some(), "node_id missing");
    assert!(body.get("display_name").is_some(), "display_name missing");
    assert!(body.get("status").is_some(), "status missing");
    assert!(body.get("uptime_seconds").is_some(), "uptime_seconds missing");
    assert!(body.get("state").is_some(), "state missing");
    assert!(body.get("registered").is_some(), "registered missing");
    assert!(body.get("session_count").is_some(), "session_count missing");
    assert!(body.get("active_session_count").is_some(), "active_session_count missing");
    assert!(body.get("capability_count").is_some(), "capability_count missing");
    assert!(body.get("verified_capability_count").is_some(), "verified_capability_count missing");
    assert!(body.get("bootstrap_completed").is_some(), "bootstrap_completed missing");
    assert!(body.get("custody_envelope_count").is_some(), "custody_envelope_count missing");
    assert!(body.get("core_connected").is_some(), "core_connected missing");
    assert!(body.get("observed_at").is_some(), "observed_at missing");
}

#[tokio::test]
async fn test_ops_diagnostics_returns_full_report() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/ops/diagnostics")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("report_id").is_some(), "report_id missing");
    assert!(body.get("requested_at").is_some(), "requested_at missing");
    assert!(body.get("health").is_some(), "health section missing");
    assert!(body.get("overview").is_some(), "overview section missing");
    assert!(body.get("sessions").is_some(), "sessions section missing");
    assert!(body.get("custody").is_some(), "custody section missing");

    let sessions = &body["sessions"];
    assert!(sessions.get("total_sessions").is_some(), "total_sessions missing");
    assert!(sessions.get("active_sessions").is_some(), "active_sessions missing");
    assert!(sessions.get("closed_sessions").is_some(), "closed_sessions missing");

    let custody = &body["custody"];
    assert!(custody.get("total_envelopes").is_some(), "total_envelopes missing");
    assert!(custody.get("integrity_verified").is_some(), "integrity_verified missing");
}

#[tokio::test]
async fn test_ops_health_summary_returns_counts() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/ops/health/summary")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("status").is_some(), "status missing");
    assert!(body.get("healthy_count").is_some(), "healthy_count missing");
    assert!(body.get("degraded_count").is_some(), "degraded_count missing");
    assert!(body.get("unhealthy_count").is_some(), "unhealthy_count missing");
    assert!(body.get("total_components").is_some(), "total_components missing");
}

#[tokio::test]
async fn test_ops_status_returns_text() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/ops/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let status_text = String::from_utf8_lossy(&body_bytes);
    assert!(!status_text.is_empty(), "status text should not be empty");
}

// ===========================================================================
// Dashboard data aggregation endpoint tests (DSH-1, DSH-6, DSH-7)
// ===========================================================================

#[tokio::test]
async fn test_dashboard_data_returns_all_domains() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/operator/dashboard/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // DSH-1: All Phase 1 + Phase 2 domains present
    assert!(body.get("node").is_some(), "node section missing");
    assert!(body.get("health").is_some(), "health section missing");
    assert!(body.get("sessions").is_some(), "sessions section missing");
    assert!(body.get("workloads").is_some(), "workloads section missing");
    assert!(body.get("capabilities").is_some(), "capabilities section missing");
    assert!(body.get("intelligence").is_some(), "intelligence section missing");
    assert!(body.get("pending_decisions").is_some(), "pending_decisions section missing");
    assert!(body.get("fleet").is_some(), "fleet section missing");
    assert!(body.get("reconciliation").is_some(), "reconciliation section missing");
    assert!(body.get("recovery").is_some(), "recovery section missing");
    assert!(body.get("custody").is_some(), "custody section missing");
    assert!(body.get("observed_at").is_some(), "observed_at missing");

    // DSH-6: No hardcoded values — verify node section has live fields
    let node = &body["node"];
    assert!(node.get("identity").is_some(), "node.identity missing");
    assert!(node.get("status").is_some(), "node.status missing");
    assert!(node.get("overview").is_some(), "node.overview missing");

    let identity = &node["identity"];
    assert!(identity.get("node_id").is_some(), "identity.node_id missing");
    assert!(identity.get("display_name").is_some(), "identity.display_name missing");
    assert!(identity.get("runtime_version").is_some(), "identity.runtime_version missing");

    // DSH-7: Dashboard data is read-only — no action/mutate fields
    assert!(body.get("actions").is_none(), "dashboard data must not contain actions");
    assert!(body.get("commands").is_none(), "dashboard data must not contain commands");
}

#[tokio::test]
async fn test_dashboard_data_node_identity_shape() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/operator/dashboard/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let node = &body["node"];
    let overview = &node["overview"];

    // DSH-2: Overview tab fields
    assert!(overview.get("node_id").is_some(), "overview.node_id missing");
    assert!(overview.get("display_name").is_some(), "overview.display_name missing");
    assert!(overview.get("status").is_some(), "overview.status missing");
    assert!(overview.get("uptime_seconds").is_some(), "overview.uptime_seconds missing");
    assert!(overview.get("session_count").is_some(), "overview.session_count missing");
    assert!(overview.get("active_session_count").is_some(), "overview.active_session_count missing");
    assert!(overview.get("capability_count").is_some(), "overview.capability_count missing");
    assert!(overview.get("verified_capability_count").is_some(), "overview.verified_capability_count missing");

    let health = &body["health"];
    assert!(health.get("overall_status").is_some(), "health.overall_status missing");
    let components = health["components"].as_array().expect("health.components should be array");
    assert!(components.len() >= 7, "health.components should have at least 7 entries");
}

#[tokio::test]
async fn test_dashboard_data_intelligence_shape() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/operator/dashboard/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // DSH-3: Intelligence tab fields
    let intel = &body["intelligence"];
    assert!(intel.get("findings_summary").is_some(), "intel.findings_summary missing");
    assert!(intel.get("findings_count").is_some(), "intel.findings_count missing");
    assert!(intel.get("active_anomalies").is_some(), "intel.active_anomalies missing");
    assert!(intel.get("pattern_summary").is_some(), "intel.pattern_summary missing");
}

#[tokio::test]
async fn test_dashboard_data_operations_shape() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/operator/dashboard/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // DSH-4: Operations tab fields
    let workloads = &body["workloads"];
    assert!(workloads.get("total").is_some(), "workloads.total missing");
    assert!(workloads.get("active").is_some(), "workloads.active missing");
    assert!(workloads.get("completed").is_some(), "workloads.completed missing");
    assert!(workloads.get("failed").is_some(), "workloads.failed missing");
    assert!(workloads.get("pending").is_some(), "workloads.pending missing");
    assert!(workloads.get("cancelled").is_some(), "workloads.cancelled missing");

    let sessions = &body["sessions"];
    assert!(sessions.get("total").is_some(), "sessions.total missing");
    assert!(sessions.get("active").is_some(), "sessions.active missing");
}

#[tokio::test]
async fn test_dashboard_data_governance_shape() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/operator/dashboard/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // DSH-5: Governance tab fields
    let pending = &body["pending_decisions"];
    assert!(pending.get("total_pending").is_some(), "pending.total_pending missing");
    assert!(pending.get("items").is_some(), "pending.items missing");

    let reconciliation = &body["reconciliation"];
    assert!(reconciliation.get("total_receipts").is_some(), "reconciliation.total_receipts missing");

    let recovery = &body["recovery"];
    assert!(recovery.get("active").is_some(), "recovery.active missing");

    let custody = &body["custody"];
    assert!(custody.get("envelope_count").is_some(), "custody.envelope_count missing");
    assert!(custody.get("integrity_verified").is_some(), "custody.integrity_verified missing");
}

#[tokio::test]
async fn test_dashboard_data_read_only_no_privileged_actions() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // DSH-7: Dashboard data endpoint is GET-only (read-only by HTTP method)
    let req = Request::builder()
        .method("POST")
        .uri("/operator/dashboard/data")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED,
        "dashboard/data must reject POST to prevent privileged actions");
}

#[tokio::test]
async fn test_dashboard_html_renders() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/operator/dashboard")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let html = String::from_utf8_lossy(&body_bytes);

    // Dashboard HTML contains all 4 tab sections
    assert!(html.contains("view-overview"), "HTML must contain overview view");
    assert!(html.contains("view-intelligence"), "HTML must contain intelligence view");
    assert!(html.contains("view-operations"), "HTML must contain operations view");
    assert!(html.contains("view-governance"), "HTML must contain governance view");

    // Dashboard contains all 4 tab buttons
    assert!(html.contains("data-tab=\"overview\""), "HTML must contain overview tab");
    assert!(html.contains("data-tab=\"intelligence\""), "HTML must contain intelligence tab");
    assert!(html.contains("data-tab=\"operations\""), "HTML must contain operations tab");
    assert!(html.contains("data-tab=\"governance\""), "HTML must contain governance tab");

    // Dashboard JS is embedded
    assert!(html.contains("renderOverview"), "HTML must contain JS renderOverview function");
    assert!(html.contains("renderIntelligence"), "HTML must contain JS renderIntelligence function");
    assert!(html.contains("renderOperations"), "HTML must contain JS renderOperations function");
    assert!(html.contains("renderGovernance"), "HTML must contain JS renderGovernance function");

    // Dashboard uses live data endpoint (no hardcoded governance values)
    assert!(!html.contains("42 Sprints Sealed"), "No hardcoded sprint count");
    assert!(!html.contains("980 Tests Passing"), "No hardcoded test count");
    assert!(html.contains("/operator/dashboard/data"), "HTML must use live data endpoint");
}

// DSH-8: Verify no existing backend behavior changed — test that existing ops endpoints still work
#[tokio::test]
async fn test_existing_ops_endpoints_unchanged() {
    let config = test_config();
    let (state, _file, _db_file) = setup_app(config).await;
    let app = build_router(state);

    // /ops/overview
    let req = Request::builder().uri("/ops/overview").body(Body::empty()).unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body.get("node_id").is_some(), "/ops/overview still returns node_id");

    // /ops/health
    let req = Request::builder().uri("/ops/health").body(Body::empty()).unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body.get("overall_status").is_some(), "/ops/health still returns overall_status");

    // /ops/diagnostics
    let req = Request::builder().uri("/ops/diagnostics").body(Body::empty()).unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body.get("report_id").is_some(), "/ops/diagnostics still returns report_id");

    // /node/identity
    let req = Request::builder().uri("/node/identity").body(Body::empty()).unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body.get("node_id").is_some(), "/node/identity still returns node_id");
}

// ============================================================================
// NODE-REGISTRY-OPERATIONAL-HARDENING-1: Hardening endpoints & offline tests
// ============================================================================

#[tokio::test]
async fn test_registry_health_endpoint_responds() {
    let config = test_config();
    let (state, _file, _dir) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/registry/health")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["status"], "healthy");
    assert!(body["components"]["file_integrity"].as_bool().unwrap());
    assert!(body["components"]["candidate_count"].as_u64().is_some());
}

#[tokio::test]
async fn test_registry_version_endpoint_responds() {
    let config = test_config();
    let (state, _file, _dir) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/registry/version")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["registry_schema_version"], 4);
}

#[tokio::test]
async fn test_registry_cleanup_endpoint_returns_summary() {
    let config = test_config();
    let (state, _file, _dir) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/registry/cleanup")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["status"], "cleanup_completed");
    assert!(body["summary"]["expired_candidates"].as_u64().is_some());
    assert!(body["summary"]["evidence_purged"].as_u64().is_some());
}

#[tokio::test]
async fn test_registry_services_work_without_core_endpoint() {
    // CoreIntegrationService is constructed with None endpoint in setup_app
    let config = test_config();
    let (state, _file, _dir) = setup_app(config).await;
    let app = build_router(state);

    // Core status should show offline
    let req = Request::builder()
        .uri("/core/status")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(!body["online"].as_bool().unwrap(), "Core must be offline when no endpoint configured");

    // Registry endpoints still work independently
    let req = Request::builder()
        .uri("/registry/health")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let req = Request::builder()
        .uri("/registry/version")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Candidate discovery works without Core
    let req = Request::builder()
        .uri("/registry/candidates")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_candidate_flow_completes_without_external_deps() {
    let config = test_config();
    let (state, _file, _dir) = setup_app(config).await;
    let app = build_router(state);

    // Candidate discovery
    let req = Request::builder()
        .uri("/registry/candidate/discover")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"node_id": "offline-node-1", "display_name": "Offline Node", "discovery_method": "manual"}"#,
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // List candidates
    let req = Request::builder()
        .uri("/registry/candidates")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body["candidates"].as_array().map(|a| a.len() > 0).unwrap_or(false),
        "Should have at least one candidate after discovery");
}

#[tokio::test]
async fn test_registry_owner_receipt_replay_protection() {
    let config = test_config();
    let (state, _file, _dir) = setup_app(config).await;

    // Test replay protection at service level
    let mut owner_svc = state.registry_owner_service.lock().await;

    let id1 = owner_svc.generate_receipt_id().unwrap();
    assert!(owner_svc.used_receipt_ids().contains(&id1));

    // Second call must produce a different unique ID
    let id2 = owner_svc.generate_receipt_id().unwrap();
    assert_ne!(id1, id2);
    assert!(owner_svc.used_receipt_ids().contains(&id2));
    assert_eq!(owner_svc.used_receipt_ids().len(), 2);
}

#[tokio::test]
async fn test_evidence_retention_on_cleanup() {
    let config = test_config();
    let (state, _file, _dir) = setup_app(config).await;

    // Add a candidate with evidence that has a very short retention
    {
        let mut candidate_svc = state.registry_candidate_service.lock().await;

        let candidate = candidate_svc.discover(
            "retention-test-node",
            "Retention Test",
            librarian_contracts::registry::DiscoveryMethod::Manual,
        );

        // Manually add evidence with 0-day retention (immediately expired)
        candidate_svc.get_all_evidence_mut().push(
            librarian_contracts::registry::CandidateEvidence {
                evidence_id: "expired-evidence-001".to_string(),
                candidate_id: candidate.candidate_id.clone(),
                evidence_type: librarian_contracts::registry::EvidenceType::OwnerNote,
                payload: serde_json::json!({"test": true}),
                collected_at: (chrono::Utc::now() - chrono::Duration::days(100)).to_rfc3339(),
                retention_days: 0,
            }
        );

        // Also add evidence with long retention (should survive)
        candidate_svc.get_all_evidence_mut().push(
            librarian_contracts::registry::CandidateEvidence {
                evidence_id: "retained-evidence-001".to_string(),
                candidate_id: candidate.candidate_id.clone(),
                evidence_type: librarian_contracts::registry::EvidenceType::OwnerNote,
                payload: serde_json::json!({"test": true}),
                collected_at: chrono::Utc::now().to_rfc3339(),
                retention_days: 365,
            }
        );
    }

    // Run cleanup
    let app = build_router(state.clone());
    let req = Request::builder()
        .uri("/registry/cleanup")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Should have purged the 0-retention evidence
    assert_eq!(body["summary"]["evidence_purged"].as_u64().unwrap_or(0), 1,
        "Should have purged the expired 0-retention evidence");

    // Verify retained evidence still exists
    let candidate_svc = state.registry_candidate_service.lock().await;
    let all_evidence = candidate_svc.get_all_evidence();
    assert!(all_evidence.iter().any(|e| e.evidence_id == "retained-evidence-001"),
        "retained-evidence-001 should survive cleanup");
    assert!(!all_evidence.iter().any(|e| e.evidence_id == "expired-evidence-001"),
        "expired-evidence-001 should have been purged");
}

