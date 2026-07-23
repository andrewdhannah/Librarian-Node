//! HTTP server — contract endpoints for the Rust router.
//!
//! Preserves the same endpoint names and response shapes as the Python router:
//! - GET  /backend/status
//! - GET  /backend/profiles
//! - GET  /backend/health
//! - GET  /health
//! - POST /backend/select
//! - POST /backend/stop
//! - POST /backend/chat (internal) / POST /v1/chat/completions (OpenAI-compatible)
//!
//! Authority: advisory_only

use crate::config::ProfileManager;
use crate::db::RuntimeDatabase;
use crate::evidence::export::build_evidence_packet;
use crate::evidence::residency_status::build_residency_status;
use crate::evidence::EvidenceWriter;
use crate::node::{
    AllocationService, AnomalyDetectionService, BootstrapService, CapabilityEvidenceBridge,
    CoreIntegrationService, CustodyService, EvidenceClassificationService,
    EvidenceIntelligenceService, FleetService, FleetTrustService, NodeIdentityService,
    NodeStateMachine, OperationsService, OwnerAllocationService, OwnerInsightService,
    OwnerWorkflowService, ModelRuntimeService, PatternEscalationService, PolicyService,
    RecoveryCustodyService, ReconciliationService, RegistryApplyService,
    RegistryCandidateService, RegistryEnforcementService, RegistryMcpService,
    RegistryOwnerService, RegistrationService, SessionService, WorkloadLifecycleService,
    WorkloadSessionService, require_active_session,
};
use crate::process::{BackendProcess, BackendState};
use crate::refusal;
use crate::residency::ModelResidencySupervisor;
use axum::{
    extract::{State, DefaultBodyLimit},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::Json,
    routing::{get, post, put},
    Router,
    extract::Request,
};
use chrono::Utc;
use librarian_contracts::core_integration::{DiscoveryResponse, SyncReceipt};
use librarian_contracts::custody::{ProvenanceQuery, ReceiptEnvelope, RetentionPolicy};
use librarian_contracts::anomaly_detection::{
    AnomalyFinding, AnomalyThreshold, BaselineRecord,
};
use librarian_contracts::recovery_custody::{
    RecoveryAction, RecoveryReport, RecoveryStatus,
};
use librarian_contracts::reconciliation::{
    ReconciliationDecision, ReconciliationReceipt, ReconciliationRequest,
};
use librarian_contracts::evidence_classification::{
    ClassifiedFinding, FindingCatalog, FindingReviewAction, FindingReviewReceipt, FindingSummary,
};
use librarian_contracts::pattern_escalation::{
    PatternDetectionConfig, PatternFinding, PatternReviewReceipt, PatternSummary,
};
use librarian_contracts::node::{
    CapabilityManifest, NodeIdentity, NodeRecord, NodeState, NodeStatus, RegistrationReceipt,
};
use librarian_contracts::policy::PolicyChangeReceipt;
use librarian_contracts::model_runtime::{ModelRuntimeEvidenceLink, RuntimeCapability};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};
// info used in evidence writes

/// Shared application state.
pub struct AppState {
    pub profile_manager: ProfileManager,
    pub config: crate::config::RouterConfig,
    pub backends: Mutex<HashMap<String, Arc<BackendProcess>>>,
    pub evidence_writer: EvidenceWriter,
    pub start_time: std::time::Instant,
    /// Background health poller handle (for graceful shutdown)
    pub health_poller_handle: Mutex<Option<JoinHandle<()>>>,
    /// Operational database for runtime state persistence
    pub db: RuntimeDatabase,
    /// Model residency supervisor — enforces single-GPU residency
    pub supervisor: ModelResidencySupervisor,
    /// Operator surface — human-facing runtime state
    pub operator: Arc<Mutex<crate::operator::OperatorService>>,
    /// Node identity service — persistent node identity
    pub node_identity_service: Arc<NodeIdentityService>,
    /// Node state machine — current lifecycle state
    pub node_state: Mutex<NodeStateMachine>,
    /// Node registration service — registration lifecycle management
    pub registration_service: Mutex<RegistrationService>,
    /// Capability evidence bridge — links capability claims to qualification evidence
    pub capability_evidence_bridge: Mutex<CapabilityEvidenceBridge>,
    /// Session service — manages execution session lifecycle
    pub session_service: Mutex<SessionService>,
    /// Bootstrap service — runtime adaptation assessment and planning
    pub bootstrap_service: Mutex<BootstrapService>,
    /// Custody service — evidence chain of custody
    pub custody_service: std::sync::Arc<std::sync::Mutex<CustodyService>>,
    /// Core integration service — optional Core synchronization
    pub core_integration_service: tokio::sync::Mutex<CoreIntegrationService>,
    /// Operations service — stateless operational query surface
    pub operations_service: OperationsService,
    /// Owner workflow service — human authority review and decision workflows
    pub owner_workflow_service: tokio::sync::Mutex<OwnerWorkflowService>,
    /// Fleet service — multi-node inventory, health, capability comparison
    pub fleet_service: tokio::sync::Mutex<FleetService>,
    /// Fleet trust service — evidence-based trust assessment and scoring
    pub fleet_trust_service: tokio::sync::Mutex<FleetTrustService>,
    /// Allocation service — capability matching, suitability scoring, recommendations
    pub allocation_service: tokio::sync::Mutex<AllocationService>,
    /// Owner allocation service — review and decide on allocation recommendations
    pub owner_allocation_service: tokio::sync::Mutex<OwnerAllocationService>,
    /// Workload session service — links allocation decisions to workload execution lifecycle
    pub workload_session_service: tokio::sync::Mutex<WorkloadSessionService>,
    /// Workload lifecycle service — stateless read-only tracking and history surface
    pub workload_lifecycle_service: WorkloadLifecycleService,
    /// Evidence intelligence service — stateless read-only analysis over existing evidence
    pub evidence_intelligence_service: EvidenceIntelligenceService,
    /// Evidence classification service — controlled vocabulary classification, review, persistence
    pub evidence_classification_service: tokio::sync::Mutex<EvidenceClassificationService>,
    /// Anomaly detection service — baseline computation, deviation detection, finding generation
    pub anomaly_detection_service: tokio::sync::Mutex<AnomalyDetectionService>,
    /// Pattern escalation service — pattern detection, review lifecycle, expiration
    pub pattern_escalation_service: tokio::sync::Mutex<PatternEscalationService>,
    /// Reconciliation service — detect divergence, compare state, owner review, receipts
    pub reconciliation_service: tokio::sync::Mutex<ReconciliationService>,
    /// Recovery custody service — controlled recovery after reconciliation outcomes
    pub recovery_custody_service: std::sync::Arc<std::sync::Mutex<RecoveryCustodyService>>,
    /// Policy service — governed configuration policy objects
    pub policy_service: std::sync::Arc<tokio::sync::Mutex<PolicyService>>,
    /// Registry candidate service — candidate admission lifecycle management
    pub registry_candidate_service: tokio::sync::Mutex<RegistryCandidateService>,
    /// Registry enforcement service — enforcement of registry rules
    pub registry_enforcement_service: tokio::sync::Mutex<RegistryEnforcementService>,
    /// Model runtime service — runtime qualification evidence querying and linking
    pub model_runtime_service: tokio::sync::Mutex<ModelRuntimeService>,
    /// Registry MCP service — MCP tool catalog and execution for registry operations
    pub registry_mcp_service: tokio::sync::Mutex<RegistryMcpService>,
    /// Registry owner service — owner action lifecycle for registry state changes
    pub registry_owner_service: tokio::sync::Mutex<RegistryOwnerService>,
    /// Registry apply service — apply boundary state machine for registry operations
    pub registry_apply_service: tokio::sync::Mutex<RegistryApplyService>,
}

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SelectRequest {
    pub profile: String,
    pub task_class: Option<String>,
    pub context: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StopRequest {
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RestartRequest {
    pub profile: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub profile: String,
    pub messages: Option<Vec<Value>>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub context: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct V1ChatRequest {
    pub model: Option<String>,
    pub messages: Option<Vec<Value>>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

// ============================================================================
// Evidence export query types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct EvidenceRunQuery {
    /// Qualification request ID this evidence responds to.
    pub request_id: String,
    /// Runtime executable SHA-256 (hex).
    pub sha256: String,
    /// Runtime executable version string.
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct EvidenceLifecycleQuery {
    /// Filter by lease ID (optional).
    pub lease_id: Option<String>,
    /// Maximum events to return (optional, default 100).
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ResidencyStatusQuery {
    /// Filter by model ID (optional).
    pub model_id: Option<String>,
}

// ============================================================================
// Middleware
// ============================================================================

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    if state.config.require_auth {
        let auth_header = req.headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        match auth_header {
            Some(token) if Some(token) == state.config.auth_token.as_ref().map(|x| x.as_str()) => {
                Ok(next.run(req).await)
            }
            _ => {
                warn!("Unauthorized request attempt");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    } else {
        Ok(next.run(req).await)
    }
}

// ============================================================================
// GET /backend/status
// ============================================================================

async fn handle_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let backends = state.backends.lock().await;
    let mut profiles_status = json!({});
    let mut active_profile: Option<String> = None;
    let mut healthy_count = 0u32;

    for (alias, bp) in backends.iter() {
        let status = bp.get_status().await;
        if status.state == "healthy" {
            healthy_count += 1;
            if active_profile.is_none() {
                active_profile = Some(alias.clone());
            }
        }
        profiles_status[alias] = serde_json::to_value(&status).unwrap_or_default();
    }

    let overall = if healthy_count > 0 { "ok" } else { "degraded" };

    let response = json!({
        "status": overall,
        "active_profile": active_profile,
        "profiles_registered": state.profile_manager.len(),
        "runtimes_alive": healthy_count,
        "uptime_seconds": state.start_time.elapsed().as_secs(),
        "authority": "advisory_only",
        "profiles": profiles_status,
    });

    state.evidence_writer.write("status.json", &response);
    Json(response)
}

// ============================================================================
// GET /backend/profiles
// ============================================================================

async fn handle_profiles(State(state): State<Arc<AppState>>) -> Json<Value> {
    let response = json!({
        "profiles": state.profile_manager.list_all(),
        "authority": "advisory_only",
    });
    state.evidence_writer.write("profiles.json", &response);
    Json(response)
}

// ============================================================================
// GET /backend/health
// ============================================================================

async fn handle_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let backends = state.backends.lock().await;
    let mut profiles_health = json!({});
    let mut all_healthy = true;
    let mut active_profile: Option<String> = None;

    for (alias, bp) in backends.iter() {
        let s = bp.get_state().await;
        let h = bp.check_health().await;
        let health_status = if s == BackendState::Healthy { "ok" } else { "degraded" };
        if !h { all_healthy = false; }
        if s == BackendState::Healthy && active_profile.is_none() {
            active_profile = Some(alias.clone());
        }
        profiles_health[alias] = json!({
            "status": health_status,
            "state": s.as_str(),
            "identity_verified": s == BackendState::Healthy,
            "port": bp.profile.port,
        });
    }

    let response = json!({
        "status": if all_healthy { "ok" } else { "degraded" },
        "active_profile": active_profile,
        "profiles": profiles_health,
        "authority": "advisory_only",
    });
    state.evidence_writer.write("health.json", &response);
    Json(response)
}

// ============================================================================
// GET /health (legacy)
// ============================================================================

async fn handle_health_legacy(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Get backend aliases without holding the lock across await
    let aliases: Vec<String> = {
        let backends = state.backends.lock().await;
        backends.keys().cloned().collect()
    };

    // Check each backend with brief health poll
    let mut active: Option<String> = None;
    for alias in &aliases {
        let bp = {
            let backends = state.backends.lock().await;
            backends.get(alias).cloned()
        };
        if let Some(bp) = bp {
            if bp.check_health().await && bp.get_state().await.is_healthy() {
                active = Some(alias.clone());
                break;
            }
        }
    }

    let response = json!({
        "status": if active.is_some() { "ok" } else { "degraded" },
        "router": "ok",
        "active_profile": active,
        "authority": "advisory_only",
    });
    Json(response)
}

// ============================================================================
// POST /backend/select
// ============================================================================

async fn handle_select(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<SelectRequest>,
) -> (StatusCode, Json<Value>) {
    let alias = &body.profile;
    let profile = state.profile_manager.get(alias);

    // Check refusal conditions
    if let Some(refusal) = refusal::check_select(
        alias,
        body.task_class.as_deref(),
        profile.is_some(),
        profile.map(|p| p.verified_status == "verified").unwrap_or(false),
        false, // runtime_failed is checked below
    ) {
        // Handle task_class case specially
        if refusal.get("reason") == Some(&json!("task_class_check_needed")) {
            // Check task classes from the profile
            if let Some(p) = profile {
                if let Some(ref tc) = body.task_class {
                    if !p.task_classes.contains(tc) {
                        let resp = json!({
                            "status": "refused",
                            "reason": "unknown_profile",
                            "detail": format!(
                                "Task class '{}' not declared for profile '{}'. Declared: {:?}",
                                tc, alias, p.task_classes
                            ),
                            "authority": "advisory_only",
                            "timestamp": Utc::now().to_rfc3339(),
                        });
                        state.evidence_writer.write("select-invalid.json", &resp);
                        return (StatusCode::FORBIDDEN, Json(resp));
                    }
                }
            }
        } else {
            state.evidence_writer.write("select-invalid.json", &refusal);
            return (StatusCode::FORBIDDEN, Json(refusal));
        }
    }

    // Get or create backend process
    let bp = {
        let mut backends = state.backends.lock().await;
        if let Some(existing) = backends.get(alias) {
            existing.clone()
        } else if let Some(profile_data) = state.profile_manager.get(alias) {
            let bp = Arc::new(BackendProcess::new(profile_data.clone(), state.config.clone()));
            backends.insert(alias.clone(), bp.clone());
            bp
        } else {
            let resp = json!({
                "status": "refused",
                "reason": "unknown_profile",
                "detail": format!("No profile registered with alias '{}'", alias),
                "authority": "advisory_only",
                "timestamp": Utc::now().to_rfc3339(),
            });
            state.evidence_writer.write("select-invalid.json", &resp);
            return (StatusCode::FORBIDDEN, Json(resp));
        }
    };

    // Start the backend if not running
    let current_state = bp.get_state().await;
    if current_state == BackendState::Stopped || current_state == BackendState::Failed {
        if let Err(e) = bp.start().await {
            let resp = json!({
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": format!("Backend launch failed for '{}': {}", alias, e),
                "authority": "advisory_only",
            });
            state.evidence_writer.write("select-invalid.json", &resp);
            return (StatusCode::SERVICE_UNAVAILABLE, Json(resp));
        }
    }

    // Brief wait for health if still starting
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while std::time::Instant::now() < deadline && !bp.get_state().await.is_healthy() {
        bp.check_health().await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    let port = state.profile_manager.get(alias).map(|p| p.port).unwrap_or(0);
    let response = json!({
        "status": "selected",
        "profile": alias,
        "port": port,
        "authority": "advisory_only",
        "task_class": body.task_class,
    });
    state.evidence_writer.write("select-valid.json", &response);
    (StatusCode::OK, Json(response))
}

// ============================================================================
// POST /backend/stop
// ============================================================================

async fn handle_stop(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<StopRequest>,
) -> (StatusCode, Json<Value>) {
    let backends = state.backends.lock().await;

    let aliases_to_stop: Vec<String> = if let Some(ref profile) = body.profile {
        vec![profile.clone()]
    } else {
        backends.keys().cloned().collect()
    };

    if aliases_to_stop.is_empty() {
        let resp = json!({
            "status": "error",
            "detail": "No backends running",
        });
        return (StatusCode::BAD_REQUEST, Json(resp));
    }

    let mut stopped = Vec::new();
    let mut not_found = Vec::new();

    for alias in &aliases_to_stop {
        if let Some(bp) = backends.get(alias) {
            bp.stop().await;
            stopped.push(alias.clone());
        } else {
            not_found.push(alias.clone());
        }
    }

    let response = json!({
        "status": "stopped",
        "stopped": stopped,
        "not_found": not_found,
        "authority": "advisory_only",
    });
    state.evidence_writer.write("stop-result.json", &response);
    (StatusCode::OK, Json(response))
}

// ============================================================================
// POST /backend/restart
// ============================================================================

async fn handle_restart(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<RestartRequest>,
) -> (StatusCode, Json<Value>) {
    let alias = &body.profile;

    // Check if profile exists
    let profile = state.profile_manager.get(alias);
    if profile.is_none() {
        let resp = json!({
            "status": "refused",
            "reason": "unknown_profile",
            "detail": format!("No profile registered with alias '{}'", alias),
            "authority": "advisory_only",
            "timestamp": Utc::now().to_rfc3339(),
        });
        state.evidence_writer.write("restart-invalid.json", &resp);
        return (StatusCode::FORBIDDEN, Json(resp));
    }

    // Get the backend process
    let bp = {
        let backends = state.backends.lock().await;
        backends.get(alias).cloned()
    };

    let bp = match bp {
        Some(bp) => bp,
        None => {
            let resp = json!({
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": format!("No runtime for profile '{}'. Select it first.", alias),
                "authority": "advisory_only",
                "timestamp": Utc::now().to_rfc3339(),
            });
            state.evidence_writer.write("restart-invalid.json", &resp);
            return (StatusCode::SERVICE_UNAVAILABLE, Json(resp));
        }
    };

    // Get old PID before restart
    let old_pid = bp.get_status().await.pid;

    // Perform restart (stop -> start -> wait healthy)
    let result = bp.restart().await;

    let new_pid = bp.get_status().await.pid;

    // Write process-before-after.txt (matching Python router)
    let timestamp = Utc::now().to_rfc3339();
    state.evidence_writer.write_text(
        "process-before-after.txt",
        &format!(
            "Before: PID={}\nAfter: PID={}\nProfile: {}\nTimestamp: {}\n",
            old_pid.unwrap_or(0),
            new_pid.unwrap_or(0),
            alias,
            timestamp,
        ),
    );

    match result {
        Ok(()) => {
            let response = json!({
                "status": "restarted",
                "profile": alias,
                "old_pid": old_pid,
                "new_pid": new_pid,
                "authority": "advisory_only",
            });
            state.evidence_writer.write("restart-result.json", &response);
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let resp = json!({
                "status": "failed",
                "profile": alias,
                "old_pid": old_pid,
                "new_pid": new_pid,
                "error": e,
                    "authority": "advisory_only",
            });
            state.evidence_writer.write("restart-result.json", &resp);
            (StatusCode::SERVICE_UNAVAILABLE, Json(resp))
        }
    }
}

// ============================================================================
// POST /backend/chat (internal router endpoint)
// ============================================================================

async fn handle_chat(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<ChatRequest>,
) -> (StatusCode, Json<Value>) {
    let alias = &body.profile;
    let messages = body.messages.unwrap_or_default();

    if alias.is_empty() {
        let resp = json!({"status": "error", "error": "Missing 'profile' field"});
        return (StatusCode::BAD_REQUEST, Json(resp));
    }

    if messages.is_empty() {
        let resp = json!({"status": "error", "error": "Missing 'messages' field"});
        return (StatusCode::BAD_REQUEST, Json(resp));
    }

    let profile = state.profile_manager.get(alias);
    let verified_context = profile.map(|p| p.context).unwrap_or(1024);

    // Check refusal
    let (is_healthy, bp_for_identity) = {
        let backends = state.backends.lock().await;
        match backends.get(alias) {
            Some(bp) => (bp.get_state().await.is_healthy(), Some(bp.clone())),
            None => (false, None),
        }
    };

    if let Some(refusal) = refusal::check_chat(
        alias,
        &messages,
        body.context,
        verified_context,
        profile.is_some(),
        is_healthy,
    ) {
        state.evidence_writer.write("chat-refusal-authority.json", &refusal);
        return (StatusCode::FORBIDDEN, Json(refusal));
    }

    // Identity verification: if backend is healthy, verify model identity
    // before proxying (matches Python router's verify_identity flow)
    if let Some(ref bp) = bp_for_identity {
        if is_healthy {
            let (identity_ok, identity_detail) = bp.verify_identity().await;
            if !identity_ok {
                let resp = json!({
                    "status": "refused",
                    "reason": "identity_mismatch",
                    "detail": format!("Identity verification failed: {}", identity_detail),
                    "profile": alias,
                    "authority": "advisory_only",
                    "timestamp": Utc::now().to_rfc3339(),
                });
                state.evidence_writer.write("chat-refusal-authority.json", &resp);
                return (StatusCode::FORBIDDEN, Json(resp));
            }
        }
    }

    // Proxy to backend
    let max_tokens = body.max_tokens.unwrap_or(256);
    let temperature = body.temperature.unwrap_or(0.7);

    let bp = {
        let backends = state.backends.lock().await;
        backends.get(alias).cloned()
    };

    match bp {
        Some(process) => {
            match process.proxy_chat(&messages, max_tokens, temperature).await {
                Ok(backend_response) => {
                    // Extract content from OpenAI-compatible response
                    let choices = backend_response.get("choices").and_then(|c| c.as_array());
                    let content = choices
                        .and_then(|c| c.first())
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    let finish_reason = choices
                        .and_then(|c| c.first())
                        .and_then(|c| c.get("finish_reason"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("stop");

                    let response = json!({
                        "status": "ok",
                        "content": content,
                        "finish_reason": finish_reason,
                        "profile": alias,
                        "authority": "advisory_only",
                    });
                    state.evidence_writer.write("chat-valid.json", &response);
                    (StatusCode::OK, Json(response))
                }
                Err(e) => {
                    let resp = json!({
                        "status": "error",
                        "error": e,
                        "profile": alias,
                        "authority": "advisory_only",
                    });
                    (StatusCode::BAD_GATEWAY, Json(resp))
                }
            }
        }
        None => {
            let resp = json!({
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": format!("No runtime for profile '{}'. Select it first.", alias),
                "authority": "advisory_only",
            });
            (StatusCode::SERVICE_UNAVAILABLE, Json(resp))
        }
    }
}

// ============================================================================
// POST /v1/chat/completions (OpenAI-compatible endpoint)
// ============================================================================

async fn handle_v1_chat(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<V1ChatRequest>,
) -> (StatusCode, Json<Value>) {
    let model = body.model.clone().unwrap_or_default();

    // Find the target backend process
    let target_bp: Option<Arc<BackendProcess>> = {
        let backends = state.backends.lock().await;

        if !model.is_empty() {
            // Try exact match with model alias
            backends.get(&model).cloned()
        } else {
            // Return the first backend (any)
            backends.values().next().cloned()
        }
    };

    let process = match target_bp {
        Some(p) => p,
        None => {
            let resp = json!({
                "error": "No active backend. Use /backend/select first.",
                "authority": "advisory_only",
            });
            return (StatusCode::SERVICE_UNAVAILABLE, Json(resp));
        }
    };

    // Identity verification before proxying
    let backend_state = process.get_state().await;
    if backend_state.is_healthy() {
        let (identity_ok, identity_detail) = process.verify_identity().await;
        if !identity_ok {
            let resp = json!({
                "error": format!("Identity verification failed: {}", identity_detail),
                "authority": "advisory_only",
            });
            return (StatusCode::FORBIDDEN, Json(resp));
        }
    }

    let messages = body.messages.unwrap_or_default();
    if messages.is_empty() {
        let resp = json!({"error": "Missing 'messages' field"});
        return (StatusCode::BAD_REQUEST, Json(resp));
    }

    let max_tokens = body.max_tokens.unwrap_or(256);
    let temperature = body.temperature.unwrap_or(0.7);

    match process.proxy_chat(&messages, max_tokens, temperature).await {
        Ok(backend_response) => (StatusCode::OK, Json(backend_response)),
        Err(e) => {
            let resp = json!({"error": e});
            (StatusCode::BAD_GATEWAY, Json(resp))
        }
    }
}

// ============================================================================
// GET /v1/models (OpenAI-compatible identity endpoint)
// ============================================================================

async fn handle_v1_models(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    // Return configured available profiles as OpenAI-compatible models list
    // Does not expose local model file paths
    let models: Vec<Value> = state.profile_manager.iter().map(|p| {
        json!({
            "id": p.alias,
            "object": "model",
            "created": chrono::Utc::now().timestamp(),
            "owned_by": "librarian-runtime-node",
            "permission": [],
            "root": p.alias,
            "parent": null,
        })
    }).collect();

    let response = json!({
        "object": "list",
        "data": models,
        "authority": "advisory_only",
    });
    state.evidence_writer.write("v1-models.json", &response);
    Json(response)
}

// ============================================================================
// GET /evidence/runs/:run_id
// ============================================================================

async fn handle_evidence_run(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<EvidenceRunQuery>,
) -> (StatusCode, Json<Value>) {
    match build_evidence_packet(
        &state.db,
        &run_id,
        &query.request_id,
        &query.sha256,
        &query.version,
    ) {
        Ok(packet) => {
            // Validate the constructed packet
            if let Err(e) = packet.validate() {
                let resp = json!({
                    "error": format!("Invalid packet: {}", e),
                    "run_id": run_id,
                });
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(resp));
            }

            // Verify authority boundary
            if let Err(e) = packet.assert_no_capability_data() {
                let resp = json!({
                    "error": format!("Authority boundary violation: {}", e),
                    "run_id": run_id,
                });
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(resp));
            }

            match serde_json::to_value(&packet) {
                Ok(val) => (StatusCode::OK, Json(val)),
                Err(e) => {
                    let resp = json!({
                        "error": format!("Failed to serialize packet: {}", e),
                        "run_id": run_id,
                    });
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(resp))
                }
            }
        }
        Err(e) => {
            let resp = json!({
                "error": format!("Failed to build evidence packet: {}", e),
                "run_id": run_id,
            });
            (StatusCode::NOT_FOUND, Json(resp))
        }
    }
}

// ============================================================================
// GET /evidence/lifecycle
// ============================================================================

async fn handle_evidence_lifecycle(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<EvidenceLifecycleQuery>,
) -> (StatusCode, Json<Value>) {
    let limit = query.limit.unwrap_or(100);
    match state.db.list_lifecycle_evidence(query.lease_id.as_deref(), Some(limit)) {
        Ok(events) => {
            let events_json: Vec<Value> = events.iter().map(|e| {
                json!({
                    "evidence_id": e.evidence_id,
                    "event_type": e.event_type.as_str(),
                    "model_id": e.model_id,
                    "profile_id": e.profile_id,
                    "lease_id": e.lease_id,
                    "run_id": e.run_id,
                    "process_id": e.process_id,
                    "observed_state": e.observed_state,
                    "observation_json": e.observation_json,
                    "occurred_at": e.occurred_at,
                    "recorded_at": e.recorded_at,
                })
            }).collect();

            let response = json!({
                "events": events_json,
                "count": events_json.len(),
            });
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let resp = json!({
                "error": format!("Failed to query lifecycle evidence: {}", e),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(resp))
        }
    }
}

// ============================================================================
// GET /residency/status
// ============================================================================

async fn handle_residency_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<ResidencyStatusQuery>,
) -> (StatusCode, Json<Value>) {
    match build_residency_status(&state.db, query.model_id.as_deref()) {
        Ok(response) => {
            // Validate the constructed response
            if let Err(e) = response.validate() {
                let resp = json!({
                    "error": format!("Invalid residency status: {}", e),
                });
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(resp));
            }

            // Verify authority boundary
            if let Err(e) = response.assert_no_capability_data() {
                let resp = json!({
                    "error": format!("Authority boundary violation: {}", e),
                });
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(resp));
            }

            match serde_json::to_value(&response) {
                Ok(val) => (StatusCode::OK, Json(val)),
                Err(e) => {
                    let resp = json!({
                        "error": format!("Failed to serialize response: {}", e),
                    });
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(resp))
                }
            }
        }
        Err(e) => {
            let resp = json!({
                "error": format!("Failed to build residency status: {}", e),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(resp))
        }
    }
}

// ============================================================================
// GET /node/identity
// ============================================================================

async fn handle_node_identity(State(state): State<Arc<AppState>>) -> Json<NodeIdentity> {
    Json(state.node_identity_service.get_identity().clone())
}

// ============================================================================
// GET /node/status
// ============================================================================

async fn handle_node_status(State(state): State<Arc<AppState>>) -> Json<NodeStatus> {
    let node_state = state.node_state.lock().await;
    Json(NodeStatus {
        identity: state.node_identity_service.get_identity().clone(),
        state: node_state.current().as_str().to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        last_state_change: node_state.last_change().to_string(),
    })
}

// ============================================================================
// GET /node/capabilities
// ============================================================================

async fn handle_node_capabilities(State(state): State<Arc<AppState>>) -> Json<CapabilityManifest> {
    // Apply enforcement degradation before manifest generation
    let mut enforcement = state.registry_enforcement_service.lock().await;
    let mut bridge = state.capability_evidence_bridge.lock().await;
    let reg = state.registration_service.lock().await;
    let policy = state.policy_service.lock().await;
    enforcement.check_capability_validity(&bridge, &reg);
    let is_registered = reg.get_record().registration_status == "registered"
        || reg.get_record().registration_status == "admitted_via_candidate";
    bridge.degrade_if_not_registered(&reg.get_record().node_id, is_registered);
    drop(enforcement);
    drop(reg);
    drop(policy);

    let mrs = state.model_runtime_service.lock().await;
    let manifest = crate::node::capabilities::detect_capabilities(
        &state.db,
        &state.node_identity_service.get_identity().node_id,
        Some(&bridge),
        Some(&*mrs),
    );
    Json(manifest)
}

// ============================================================================
// GET /node/capabilities/evidence
// ============================================================================

async fn handle_capability_evidence(State(state): State<Arc<AppState>>) -> Json<Value> {
    let bridge = state.capability_evidence_bridge.lock().await;
    let state_val = bridge.get_verification_state(
        &state.node_identity_service.get_identity().node_id,
    );
    Json(serde_json::to_value(&state_val).unwrap_or_default())
}

// ============================================================================
// POST /node/capabilities/evidence/link
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LinkEvidenceRequest {
    pub claim_id: String,
    pub evidence_packet_id: String,
    pub qualification_run_id: String,
}

async fn handle_link_evidence(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<LinkEvidenceRequest>,
) -> (StatusCode, Json<Value>) {
    let mut bridge = state.capability_evidence_bridge.lock().await;
    match bridge.link_evidence(&body.claim_id, &body.evidence_packet_id, &body.qualification_run_id) {
        Some(reference) => {
            if let Err(e) = bridge.persist() {
                warn!("Failed to persist evidence bridge state: {}", e);
            }
            (StatusCode::OK, Json(json!(reference)))
        }
        None => {
            let resp = json!({
                "error": format!("Claim '{}' not found", body.claim_id),
            });
            (StatusCode::NOT_FOUND, Json(resp))
        }
    }
}

// ============================================================================
// PUT /node/capabilities/{type}/state
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TransitionStateRequest {
    pub state: String,
    pub reason: String,
}

async fn handle_capability_transition_state(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(cap_type): axum::extract::Path<String>,
    axum::Json(body): axum::Json<TransitionStateRequest>,
) -> (StatusCode, Json<Value>) {
    use librarian_contracts::capability_evidence::CapabilityState;

    let target_state = match body.state.to_lowercase().as_str() {
        "active" => CapabilityState::Active,
        "degraded" => CapabilityState::Degraded,
        "retired" => CapabilityState::Retired,
        "superseded" => CapabilityState::Superseded,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!("Invalid target state '{}'. Valid: active, degraded, retired, superseded", body.state)
                })),
            );
        }
    };

    let mut bridge = state.capability_evidence_bridge.lock().await;
    match bridge.transition_state(&cap_type, target_state, &body.reason) {
        Ok(receipt) => {
            if let Err(e) = bridge.persist() {
                warn!("Failed to persist evidence bridge: {}", e);
            }
            (StatusCode::OK, Json(json!(receipt)))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    }
}

// ============================================================================
// GET /node/capabilities/lifecycle
// ============================================================================

async fn handle_capability_lifecycle(State(state): State<Arc<AppState>>) -> Json<Value> {
    let bridge = state.capability_evidence_bridge.lock().await;
    let lifecycle = bridge.get_capability_lifecycle();
    Json(json!({
        "lifecycle": lifecycle,
        "count": lifecycle.len(),
    }))
}

// ============================================================================
// GET /node/capabilities/unverified
// ============================================================================

async fn handle_unverified_claims(State(state): State<Arc<AppState>>) -> Json<Value> {
    let bridge = state.capability_evidence_bridge.lock().await;
    let claims: Vec<&librarian_contracts::capability_evidence::CapabilityClaim> =
        bridge.get_unverified_claims();
    Json(json!({
        "node_id": state.node_identity_service.get_identity().node_id,
        "unverified_claims": claims,
        "count": claims.len(),
    }))
}

// ============================================================================
// POST /node/capabilities/evidence/verify
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct VerifyEvidenceRequest {
    pub claim_id: String,
    pub evidence_packet_json: String,
}

async fn handle_verify_evidence(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<VerifyEvidenceRequest>,
) -> (StatusCode, Json<Value>) {
    let packet = match librarian_contracts::evidence_packet::EvidencePacket::from_json(&body.evidence_packet_json) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Invalid evidence packet JSON: {}", e) })),
            );
        }
    };

    let mut bridge = state.capability_evidence_bridge.lock().await;
    match bridge.verify_claim(&body.claim_id, &packet) {
        Ok(status) => {
            if let Err(e) = bridge.persist() {
                warn!("Failed to persist evidence bridge state: {}", e);
            }
            (StatusCode::OK, Json(json!({ "status": status })))
        }
        Err(e) => {
            if let Err(pe) = bridge.persist() {
                warn!("Failed to persist evidence bridge state: {}", pe);
            }
            (StatusCode::BAD_REQUEST, Json(json!({ "error": e })))
        }
    }
}

// ============================================================================
// GET /node/registration
// ============================================================================

async fn handle_node_registration(State(state): State<Arc<AppState>>) -> Json<NodeRecord> {
    let reg = state.registration_service.lock().await;
    Json(reg.get_record().clone())
}

// ============================================================================
// POST /node/register
// ============================================================================

async fn handle_node_register(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let identity = state.node_identity_service.get_identity().clone();

    // Build capabilities snapshot
    let capabilities_snapshot = {
        let bridge = state.capability_evidence_bridge.lock().await;
        let manifest = crate::node::capabilities::detect_capabilities(
            &state.db,
            &identity.node_id,
            Some(&bridge),
            None,
        );
        serde_json::to_string(&manifest).ok()
    };

    let mut node_state = state.node_state.lock().await;
    let mut reg = state.registration_service.lock().await;

    // Transition state
    if let Err(e) = node_state.transition(NodeState::RegistrationRequested) {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "status": "error",
                "error": format!("Invalid state transition: {}", e),
                "current_state": node_state.current().as_str(),
            })),
        );
    }

    let request = reg.submit_registration(&identity, capabilities_snapshot);
    let receipt = RegistrationReceipt {
        registration_id: uuid::Uuid::new_v4().to_string(),
        node_id: request.node_id.clone(),
        status: "registration_requested".to_string(),
        registered_at: request.requested_at.clone(),
        previous_state: Some("unregistered".to_string()),
    };

    (
        StatusCode::OK,
        Json(json!(receipt)),
    )
}

// ============================================================================
// POST /node/register/confirm
// ============================================================================

async fn handle_node_register_confirm(
    State(state): State<Arc<AppState>>,
    axum::Json(receipt): axum::Json<RegistrationReceipt>,
) -> (StatusCode, Json<Value>) {
    let mut node_state = state.node_state.lock().await;
    let mut reg = state.registration_service.lock().await;

    // Transition state
    if let Err(e) = node_state.transition(NodeState::Registered) {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "status": "error",
                "error": format!("Invalid state transition: {}", e),
                "current_state": node_state.current().as_str(),
            })),
        );
    }

    reg.confirm_registration(&receipt);

    (
        StatusCode::OK,
        Json(json!({
            "status": "registered",
            "node_id": receipt.node_id,
            "registration_id": receipt.registration_id,
            "registered_at": receipt.registered_at,
        })),
    )
}

// ============================================================================
// Session Management Endpoints
// ============================================================================

/// POST /session/start
async fn handle_session_start(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<librarian_contracts::session::SessionStartRequest>,
) -> (StatusCode, Json<Value>) {
    let mut enforcement = state.registry_enforcement_service.lock().await;
    let reg = state.registration_service.lock().await;
    let policy = state.policy_service.lock().await;
    if let Err(e) = enforcement.check_session_allowed(&reg, &policy) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e, "reason": "registration_required" })),
        );
    }
    drop(enforcement);
    drop(reg);
    drop(policy);

    let mut session_service = state.session_service.lock().await;
    let session = session_service.create_session(body);
    (StatusCode::OK, Json(json!(session)))
}

/// POST /session/{session_id}/activate
async fn handle_session_activate(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let mut session_service = state.session_service.lock().await;
    match session_service.activate_session(&session_id) {
        Ok(session) => (StatusCode::OK, Json(json!(session))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// POST /session/{session_id}/close
async fn handle_session_close(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let mut session_service = state.session_service.lock().await;
    match session_service.close_session(&session_id) {
        Ok(receipt) => (StatusCode::OK, Json(json!(receipt))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// POST /session/{session_id}/expire
async fn handle_session_expire(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let mut session_service = state.session_service.lock().await;
    match session_service.expire_session(&session_id) {
        Ok(session) => (StatusCode::OK, Json(json!(session))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// GET /session/{session_id}
async fn handle_session_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let session_service = state.session_service.lock().await;
    match session_service.get_session(&session_id) {
        Some(session) => (StatusCode::OK, Json(json!(session))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session {} not found", session_id) })),
        ),
    }
}

/// GET /sessions?state=active
#[derive(Debug, Deserialize)]
pub struct SessionListQuery {
    pub state: Option<String>,
}

async fn handle_sessions_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<SessionListQuery>,
) -> Json<Value> {
    let session_service = state.session_service.lock().await;
    let sessions = session_service.list_sessions(query.state.as_deref());
    Json(json!(sessions))
}

/// GET /session/{session_id}/receipt
async fn handle_session_receipt(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let session_service = state.session_service.lock().await;
    match session_service.get_receipt(&session_id) {
        Some(receipt) => (StatusCode::OK, Json(json!(receipt))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Receipt for session {} not found", session_id) })),
        ),
    }
}

// ============================================================================
// Bootstrap Request Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BootstrapAssessRequest {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapPlanRequest {
    pub session_id: String,
    pub assessment_id: String,
    pub approved_recommendation_ids: Vec<String>,
}

// ============================================================================
// Bootstrap Endpoints
// ============================================================================

/// POST /bootstrap/assess
async fn handle_bootstrap_assess(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<BootstrapAssessRequest>,
) -> (StatusCode, Json<Value>) {
    let session_service = state.session_service.lock().await;
    if let Err(e) = require_active_session(&session_service, &body.session_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e })),
        );
    }
    drop(session_service);

    let mut bootstrap = state.bootstrap_service.lock().await;
    let assessment = bootstrap.assess(&body.session_id);
    (StatusCode::OK, Json(json!(assessment)))
}

/// POST /bootstrap/plan
async fn handle_bootstrap_create_plan(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<BootstrapPlanRequest>,
) -> (StatusCode, Json<Value>) {
    let session_service = state.session_service.lock().await;
    if let Err(e) = require_active_session(&session_service, &body.session_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e })),
        );
    }
    drop(session_service);

    let mut bootstrap = state.bootstrap_service.lock().await;
    match bootstrap.create_plan(&body.session_id, &body.assessment_id, &body.approved_recommendation_ids) {
        Ok(plan) => (StatusCode::OK, Json(json!(plan))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// POST /bootstrap/plan/{plan_id}/execute
async fn handle_bootstrap_execute_plan(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let mut bootstrap = state.bootstrap_service.lock().await;
    match bootstrap.execute_plan(&plan_id) {
        Ok(receipt) => (StatusCode::OK, Json(json!(receipt))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// POST /bootstrap/plan/{plan_id}/approve
async fn handle_bootstrap_approve_plan(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let mut bootstrap = state.bootstrap_service.lock().await;
    match bootstrap.approve_plan(&plan_id) {
        Ok(plan) => (StatusCode::OK, Json(json!(plan))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// GET /bootstrap/plan/{plan_id}
async fn handle_bootstrap_get_plan(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let bootstrap = state.bootstrap_service.lock().await;
    match bootstrap.get_plan(&plan_id) {
        Some(plan) => (StatusCode::OK, Json(json!(plan))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Plan {} not found", plan_id) })),
        ),
    }
}

/// GET /bootstrap/assessment/{assessment_id}
async fn handle_bootstrap_get_assessment(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(assessment_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let bootstrap = state.bootstrap_service.lock().await;
    match bootstrap.get_assessment(&assessment_id) {
        Some(assessment) => (StatusCode::OK, Json(json!(assessment))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Assessment {} not found", assessment_id) })),
        ),
    }
}

/// GET /sessions/receipts
async fn handle_sessions_receipts(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let session_service = state.session_service.lock().await;
    let receipts = session_service.get_receipts();
    Json(json!(receipts))
}

// ============================================================================
// Core Integration Endpoints
// ============================================================================

/// GET /core/projection — complete snapshot of current node state for Core consumption
async fn handle_core_projection(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Collect all data without holding core lock
    let identity = state.node_identity_service.get_identity().clone();

    let registration;
    let capabilities;
    let capabilities_verified;
    let session_count;
    let bootstrap_completed;
    let custody_envelope_count;
    let last_integrity_hash;
    {
        let reg = state.registration_service.lock().await;
        registration = serde_json::to_value(reg.get_record()).ok();
    }
    {
        let bridge = state.capability_evidence_bridge.lock().await;
        let manifest = crate::node::capabilities::detect_capabilities(
            &state.db,
            &identity.node_id,
            Some(&bridge),
            None,
        );
        capabilities_verified = manifest.capabilities.iter().all(|c| c.verification_status.as_deref() == Some("verified"));
        capabilities = serde_json::to_value(&manifest).ok();
    }
    {
        let sessions = state.session_service.lock().await;
        session_count = sessions.list_sessions(None).len() as u32;
    }
    {
        let bootstrap = state.bootstrap_service.lock().await;
        bootstrap_completed = bootstrap.get_receipts().len() > 0;
    }
    {
        let custody = state.custody_service.lock().unwrap();
        let chain = custody.get_chain();
        custody_envelope_count = chain.as_ref().map(|c| c.envelope_count).unwrap_or(0);
        last_integrity_hash = chain.map(|c| c.last_chain_hash);
    }

    let core = state.core_integration_service.lock().await;
    let projection = core.generate_projection(
        &identity,
        registration,
        capabilities,
        capabilities_verified,
        session_count,
        bootstrap_completed,
        custody_envelope_count,
        last_integrity_hash,
    );
    Json(json!(projection))
}

/// POST /core/sync/prepare — prepares a sync payload (for sending to Core or export)
async fn handle_core_sync_prepare(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Collect all data without holding core lock
    let identity = state.node_identity_service.get_identity().clone();

    let registration;
    let capabilities;
    let capabilities_verified;
    let session_count;
    let bootstrap_completed;
    let custody_envelope_count;
    let last_integrity_hash;
    {
        let reg = state.registration_service.lock().await;
        registration = serde_json::to_value(reg.get_record()).ok();
    }
    {
        let bridge = state.capability_evidence_bridge.lock().await;
        let manifest = crate::node::capabilities::detect_capabilities(
            &state.db,
            &identity.node_id,
            Some(&bridge),
            None,
        );
        capabilities_verified = manifest.capabilities.iter().all(|c| c.verification_status.as_deref() == Some("verified"));
        capabilities = serde_json::to_value(&manifest).ok();
    }
    {
        let sessions = state.session_service.lock().await;
        session_count = sessions.list_sessions(None).len() as u32;
    }
    {
        let bootstrap = state.bootstrap_service.lock().await;
        bootstrap_completed = bootstrap.get_receipts().len() > 0;
    }
    {
        let custody = state.custody_service.lock().unwrap();
        let chain = custody.get_chain();
        custody_envelope_count = chain.as_ref().map(|c| c.envelope_count).unwrap_or(0);
        last_integrity_hash = chain.map(|c| c.last_chain_hash);
    }

    let mut core = state.core_integration_service.lock().await;
    let projection = core.generate_projection(
        &identity,
        registration,
        capabilities,
        capabilities_verified,
        session_count,
        bootstrap_completed,
        custody_envelope_count,
        last_integrity_hash,
    );
    let request = core.prepare_sync(projection, &identity);
    Json(json!(request))
}

/// POST /core/sync/receipt — processes a SyncReceipt from Core
async fn handle_core_sync_receipt(
    State(state): State<Arc<AppState>>,
    axum::Json(receipt): axum::Json<SyncReceipt>,
) -> (StatusCode, Json<Value>) {
    let mut core = state.core_integration_service.lock().await;
    match core.process_sync_receipt(receipt) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "status": "processed" })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// POST /core/discover — announces node presence
async fn handle_core_discover(State(state): State<Arc<AppState>>) -> Json<Value> {
    let core = state.core_integration_service.lock().await;
    let identity = state.node_identity_service.get_identity();
    let announcement = core.create_announcement(identity);
    Json(json!(announcement))
}

/// POST /core/discover/response — processes a DiscoveryResponse
async fn handle_core_discover_response(
    State(state): State<Arc<AppState>>,
    axum::Json(response): axum::Json<DiscoveryResponse>,
) -> (StatusCode, Json<Value>) {
    let mut core = state.core_integration_service.lock().await;
    match core.process_discovery_response(response) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "status": "acknowledged" })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// GET /core/status — Core connection status
async fn handle_core_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let core = state.core_integration_service.lock().await;
    let custody = state.custody_service.lock().unwrap();
    let chain = custody.get_chain();
    let unsynced = chain.as_ref().map(|c| c.envelope_count).unwrap_or(0);
    Json(json!({
        "online": core.is_online(),
        "discovery_registered": core.get_discovery_registered(),
        "last_sync_at": core.get_last_sync_at(),
        "unsynced_envelope_count": unsynced,
        "sync_attempts": core.get_sync_attempts().len(),
    }))
}

// ============================================================================
// Fleet Management Endpoints
// ============================================================================

/// GET /fleet/inventory
async fn handle_fleet_inventory(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::fleet::FleetInventory> {
    let fleet = state.fleet_service.lock().await;
    Json(fleet.get_inventory())
}

/// GET /fleet/inventory/{node_id}
async fn handle_fleet_inventory_node(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(node_id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let fleet = state.fleet_service.lock().await;
    match fleet.get_node(&node_id) {
        Some(entry) => (StatusCode::OK, Json(serde_json::to_value(entry).unwrap_or_default())),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("Node {} not found in fleet inventory", node_id)}))),
    }
}

/// GET /fleet/health
async fn handle_fleet_health(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::fleet::FleetHealth> {
    let fleet = state.fleet_service.lock().await;
    Json(fleet.get_fleet_health())
}

/// GET /fleet/health/breakdown
async fn handle_fleet_health_breakdown(State(state): State<Arc<AppState>>) -> Json<Vec<librarian_contracts::fleet::FleetHealthBreakdown>> {
    let fleet = state.fleet_service.lock().await;
    Json(fleet.get_health_breakdown())
}

/// GET /fleet/capabilities
async fn handle_fleet_capabilities(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::fleet::FleetCapabilityView> {
    let fleet = state.fleet_service.lock().await;
    Json(fleet.get_fleet_capability_view())
}

/// GET /fleet/overview
async fn handle_fleet_overview(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::fleet::FleetOverview> {
    let fleet = state.fleet_service.lock().await;
    Json(fleet.get_fleet_overview())
}

/// POST /fleet/discover
async fn handle_fleet_discover(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<librarian_contracts::fleet::DiscoveryScanResult>,
) -> Json<librarian_contracts::fleet::FleetInventory> {
    let mut fleet = state.fleet_service.lock().await;
    fleet.process_scan_result(body);
    Json(fleet.get_inventory())
}

/// POST /fleet/nodes
async fn handle_fleet_nodes_post(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<librarian_contracts::fleet::NodeInventoryEntry>,
) -> Json<librarian_contracts::fleet::FleetInventory> {
    let mut fleet = state.fleet_service.lock().await;
    fleet.add_or_update_node(body);
    Json(fleet.get_inventory())
}

// ============================================================================
// Fleet Trust Endpoints
// ============================================================================

/// GET /fleet/trust — all trust states
async fn handle_fleet_trust(State(state): State<Arc<AppState>>) -> Json<Vec<librarian_contracts::fleet_trust::NodeTrustState>> {
    let trust = state.fleet_trust_service.lock().await;
    Json(trust.get_all_trust_states())
}

/// GET /fleet/trust/{node_id} — single node trust state
async fn handle_fleet_trust_node(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(node_id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let trust = state.fleet_trust_service.lock().await;
    match trust.get_node_trust(&node_id) {
        Some(state_val) => (StatusCode::OK, Json(serde_json::to_value(state_val).unwrap_or_default())),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("Trust state for node {} not found", node_id)}))),
    }
}

/// POST /fleet/trust/assess — assess trust for all nodes
async fn handle_fleet_trust_assess(State(state): State<Arc<AppState>>) -> Json<Vec<librarian_contracts::fleet_trust::NodeTrustState>> {
    let (chain_exists, integrity_verified, envelope_count) = {
        let custody = state.custody_service.lock().unwrap();
        let chain = custody.get_chain();
        let exists = chain.is_some();
        let integrity = if exists { custody.verify_integrity().verified } else { false };
        let count = chain.map(|c| c.envelope_count).unwrap_or(0);
        (exists, integrity, count)
    };

    let results = {
        let fleet = state.fleet_service.lock().await;
        let anomaly = state.anomaly_detection_service.lock().await;
        let pattern = state.pattern_escalation_service.lock().await;
        let mut trust = state.fleet_trust_service.lock().await;
        trust.assess_all_nodes_ext(&fleet, &anomaly, &pattern, chain_exists, integrity_verified, envelope_count)
    };

    {
        let mut fleet = state.fleet_service.lock().await;
        let trust = state.fleet_trust_service.lock().await;
        trust.publish_trust_to_fleet(&mut fleet);
    }

    Json(results)
}

/// GET /fleet/trust/receipts — trust assessment receipts
async fn handle_fleet_trust_receipts(State(state): State<Arc<AppState>>) -> Json<Vec<librarian_contracts::fleet_trust::TrustAssessmentReceipt>> {
    let trust = state.fleet_trust_service.lock().await;
    Json(trust.get_receipts())
}

// ============================================================================
// Allocation Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct EvaluateRequest {
    pub requirements: Vec<librarian_contracts::allocation::CapabilityRequirement>,
}

#[derive(Debug, Deserialize)]
pub struct ScoreRequest {
    pub requirements: Vec<librarian_contracts::allocation::CapabilityRequirement>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptRecommendationRequest {
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RejectRecommendationRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecommendationsQuery {
    pub status: Option<String>,
}

/// POST /allocation/evaluate
async fn handle_allocation_evaluate(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<EvaluateRequest>,
) -> Json<Vec<librarian_contracts::allocation::CapabilityMatch>> {
    let allocation = state.allocation_service.lock().await;
    let fleet = state.fleet_service.lock().await;
    let nodes = fleet.all_nodes().to_vec();
    let results = allocation.evaluate_requirements(body.requirements, nodes);
    Json(results)
}

/// POST /allocation/score
async fn handle_allocation_score(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<ScoreRequest>,
) -> Json<Vec<librarian_contracts::allocation::SuitabilityScore>> {
    let allocation = state.allocation_service.lock().await;
    let fleet = state.fleet_service.lock().await;
    let nodes = fleet.all_nodes().to_vec();
    let matches = allocation.evaluate_requirements(body.requirements, nodes.clone());
    let scores = allocation.score_nodes(matches, nodes);
    Json(scores)
}

/// POST /allocation/recommend
async fn handle_allocation_recommend(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<librarian_contracts::allocation::AllocationRequest>,
) -> Json<librarian_contracts::allocation::AllocationRecommendation> {
    let mut allocation = state.allocation_service.lock().await;
    let fleet = state.fleet_service.lock().await;
    let recommendation = allocation.generate_recommendation(body, &fleet);
    Json(recommendation)
}

/// POST /allocation/recommend/{id}/accept
async fn handle_allocation_accept(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<AcceptRecommendationRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut allocation = state.allocation_service.lock().await;
    match allocation.accept_recommendation(&id, body.session_id) {
        Some(receipt) => (StatusCode::OK, Json(serde_json::to_value(receipt).unwrap_or_default())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Recommendation {} not found", id)})),
        ),
    }
}

/// POST /allocation/recommend/{id}/reject
async fn handle_allocation_reject(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<RejectRecommendationRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut allocation = state.allocation_service.lock().await;
    match allocation.reject_recommendation(&id, body.reason) {
        Some(receipt) => (StatusCode::OK, Json(serde_json::to_value(receipt).unwrap_or_default())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Recommendation {} not found", id)})),
        ),
    }
}

/// GET /allocation/recommendations
async fn handle_allocation_recommendations(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<RecommendationsQuery>,
) -> Json<Vec<librarian_contracts::allocation::AllocationRecommendation>> {
    let allocation = state.allocation_service.lock().await;
    let results = allocation.get_recommendations(query.status.as_deref());
    Json(results)
}

/// GET /allocation/receipts
async fn handle_allocation_receipts(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<librarian_contracts::allocation::AllocationReceipt>> {
    let allocation = state.allocation_service.lock().await;
    Json(allocation.get_receipts())
}

// ============================================================================
// Owner Allocation Workflow Request Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OwnerAllocationReviewRequest {
    pub request_id: Option<String>,
    pub session_id: Option<String>,
    pub filter_status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OwnerAllocationDecisionRequest {
    pub decision_id: String,
    pub recommendation_id: String,
    pub session_id: String,
    pub decision: String,
    pub alternative_node_id: Option<String>,
    pub reason: Option<String>,
    pub decided_at: String,
}

// ============================================================================
// Owner Allocation Workflow Endpoints
// ============================================================================

/// GET /owner/allocation/pending
async fn handle_owner_allocation_pending(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::owner_allocation::PendingAllocationQueue> {
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    let queue = owner.get_pending_recommendations(&*allocation);
    Json(queue)
}

/// POST /owner/allocation/review
async fn handle_owner_allocation_review(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OwnerAllocationReviewRequest>,
) -> Json<librarian_contracts::owner_allocation::AllocationReviewResult> {
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    let result = owner.review_recommendations(&*allocation, body.filter_status.as_deref());
    Json(result)
}

/// GET /owner/allocation/recommendation/{id}
async fn handle_owner_allocation_recommendation_detail(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    match owner.get_recommendation_detail(&*allocation, &id) {
        Some(summary) => (StatusCode::OK, Json(serde_json::to_value(summary).unwrap_or_default())),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("Recommendation {} not found", id)}))),
    }
}

/// POST /owner/allocation/decide
async fn handle_owner_allocation_decide(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OwnerAllocationDecisionRequest>,
) -> Json<librarian_contracts::owner_allocation::AllocationDecisionReceipt> {
    let mut allocation = state.allocation_service.lock().await;
    let mut owner = state.owner_allocation_service.lock().await;
    let decision = librarian_contracts::owner_allocation::AllocationDecision {
        decision_id: body.decision_id,
        recommendation_id: body.recommendation_id,
        session_id: body.session_id,
        decision: body.decision,
        alternative_node_id: body.alternative_node_id,
        reason: body.reason,
        decided_at: body.decided_at,
    };
    let receipt = owner.submit_decision(&mut *allocation, decision);
    Json(receipt)
}

/// GET /owner/allocation/history
async fn handle_owner_allocation_history(State(state): State<Arc<AppState>>) -> Json<Vec<librarian_contracts::owner_allocation::AllocationDecisionReceipt>> {
    let owner = state.owner_allocation_service.lock().await;
    Json(owner.get_decision_history())
}

/// GET /owner/allocation/actions
async fn handle_owner_allocation_actions(State(state): State<Arc<AppState>>) -> Json<Vec<librarian_contracts::owner_allocation::AllocationActionReceipt>> {
    let owner = state.owner_allocation_service.lock().await;
    Json(owner.get_action_receipts())
}

// ============================================================================
// Workload Session Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateWorkloadSessionRequest {
    pub workload: librarian_contracts::workload_session::WorkloadDescriptor,
    pub decision_receipt_id: String,
    pub node_id: String,
    pub allocation_recommendation_id: Option<String>,
    pub allocation_decision_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteWorkloadSessionRequest {
    pub operations_executed: u32,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct FailWorkloadSessionRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ListWorkloadSessionsQuery {
    pub node_id: Option<String>,
    pub state: Option<String>,
}

/// POST /workload/session/create
async fn handle_workload_session_create(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<CreateWorkloadSessionRequest>,
) -> (StatusCode, Json<Value>) {
    let mut ws_service = state.workload_session_service.lock().await;
    let mut session_service = state.session_service.lock().await;
    let mut custody_service = state.custody_service.lock().unwrap();
    match ws_service.create_workload_session(
        body.workload,
        &body.decision_receipt_id,
        &body.node_id,
        body.allocation_recommendation_id,
        body.allocation_decision_id,
        &mut *session_service,
        Some(&mut *custody_service),
    ) {
        Ok(ws) => (StatusCode::OK, Json(json!(ws))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// POST /workload/session/{id}/activate
async fn handle_workload_session_activate(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let mut ws_service = state.workload_session_service.lock().await;
    let mut session_service = state.session_service.lock().await;
    match ws_service.activate_workload_session(&id, &mut *session_service) {
        Ok(ws) => (StatusCode::OK, Json(json!(ws))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// POST /workload/session/{id}/complete
async fn handle_workload_session_complete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<CompleteWorkloadSessionRequest>,
) -> (StatusCode, Json<Value>) {
    let mut ws_service = state.workload_session_service.lock().await;
    let mut session_service = state.session_service.lock().await;
    match ws_service.complete_workload_session(&id, body.operations_executed, body.evidence_ids, &mut *session_service) {
        Ok(receipt) => (StatusCode::OK, Json(json!(receipt))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// POST /workload/session/{id}/fail
async fn handle_workload_session_fail(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<FailWorkloadSessionRequest>,
) -> (StatusCode, Json<Value>) {
    let mut ws_service = state.workload_session_service.lock().await;
    let mut session_service = state.session_service.lock().await;
    match ws_service.fail_workload_session(&id, &body.reason, &mut *session_service) {
        Ok(receipt) => (StatusCode::OK, Json(json!(receipt))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        ),
    }
}

/// GET /workload/session/{id}
async fn handle_workload_session_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let ws_service = state.workload_session_service.lock().await;
    match ws_service.get_workload_session(&id) {
        Some(ws) => (StatusCode::OK, Json(json!(ws))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("WorkloadSession {} not found", id) })),
        ),
    }
}

/// GET /workload/sessions
async fn handle_workload_sessions_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<ListWorkloadSessionsQuery>,
) -> Json<Value> {
    let ws_service = state.workload_session_service.lock().await;
    let sessions = if let Some(ref node_id) = query.node_id {
        ws_service.get_workload_sessions_by_node(node_id)
    } else if let Some(ref state_filter) = query.state {
        ws_service.get_workload_sessions_by_state(state_filter)
    } else {
        ws_service.list_workload_sessions()
    };
    Json(json!(sessions))
}

/// GET /workload/session/{id}/link
async fn handle_workload_session_link(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let ws_service = state.workload_session_service.lock().await;
    match ws_service.get_link(&id) {
        Some(link) => (StatusCode::OK, Json(json!(link))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Link for workload {} not found", id) })),
        ),
    }
}

/// GET /workload/receipts
async fn handle_workload_receipts(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let ws_service = state.workload_session_service.lock().await;
    let receipts = ws_service.get_receipts();
    Json(json!(receipts))
}

// ============================================================================
// Workload Lifecycle Endpoints
// ============================================================================

/// GET /workload/inventory
async fn handle_workload_inventory(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::workload_lifecycle::WorkloadInventory> {
    let ws_service = state.workload_session_service.lock().await;
    Json(WorkloadLifecycleService::get_inventory(&ws_service))
}

/// GET /workload/timeline/{workload_id}
async fn handle_workload_timeline(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(workload_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let ws_service = state.workload_session_service.lock().await;
    match WorkloadLifecycleService::get_timeline(&ws_service, &workload_id) {
        Some(timeline) => (StatusCode::OK, Json(json!(timeline))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Workload {} not found", workload_id) })),
        ),
    }
}

/// POST /workload/history
async fn handle_workload_history(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<librarian_contracts::workload_lifecycle::WorkloadHistoryQuery>,
) -> Json<librarian_contracts::workload_lifecycle::WorkloadHistoryResult> {
    let ws_service = state.workload_session_service.lock().await;
    Json(WorkloadLifecycleService::query_history(&ws_service, body))
}

/// GET /workload/review/{workload_id}
async fn handle_workload_review(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(workload_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let ws_service = state.workload_session_service.lock().await;
    match WorkloadLifecycleService::get_review(&ws_service, &workload_id) {
        Some(review) => (StatusCode::OK, Json(json!(review))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Workload {} not found", workload_id) })),
        ),
    }
}

/// GET /workload/active
async fn handle_workload_active(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let ws_service = state.workload_session_service.lock().await;
    let inventory = WorkloadLifecycleService::get_inventory(&ws_service);
    let active: Vec<librarian_contracts::workload_lifecycle::WorkloadSummary> = inventory
        .workloads
        .into_iter()
        .filter(|w| w.state == "active")
        .collect();
    Json(json!(active))
}

/// GET /workload/failed
async fn handle_workload_failed(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let ws_service = state.workload_session_service.lock().await;
    let failed = WorkloadLifecycleService::get_failed_workloads(&ws_service);
    Json(json!(failed))
}

/// GET /workload/summary
async fn handle_workload_summary(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::workload_lifecycle::WorkloadInventory> {
    let ws_service = state.workload_session_service.lock().await;
    Json(WorkloadLifecycleService::get_inventory(&ws_service))
}

// ============================================================================
// Evidence Intelligence Endpoints
// ============================================================================

/// POST /intelligence/report — Generate complete intelligence report
async fn handle_intelligence_report(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::evidence_intelligence::IntelligenceReport> {
    let ws_service = state.workload_session_service.lock().await;
    let fleet = state.fleet_service.lock().await;
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    let bridge = state.capability_evidence_bridge.lock().await;
    Json(EvidenceIntelligenceService::generate_report(
        &ws_service, &fleet, &allocation, &owner, &bridge,
    ))
}

/// POST /intelligence/workloads — Workload outcome analysis
async fn handle_intelligence_workloads(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::evidence_intelligence::WorkloadOutcomeAnalysis> {
    let ws_service = state.workload_session_service.lock().await;
    Json(EvidenceIntelligenceService::analyze_workload_outcomes(
        &ws_service,
    ))
}

/// POST /intelligence/capabilities — Capability effectiveness analysis
async fn handle_intelligence_capabilities(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::evidence_intelligence::CapabilityEffectivenessAnalysis> {
    let ws_service = state.workload_session_service.lock().await;
    let fleet = state.fleet_service.lock().await;
    let bridge = state.capability_evidence_bridge.lock().await;
    Json(EvidenceIntelligenceService::analyze_capability_effectiveness(
        &ws_service, &fleet, &bridge,
    ))
}

/// POST /intelligence/allocation — Allocation recommendation accuracy
async fn handle_intelligence_allocation(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::evidence_intelligence::AllocationAccuracyAnalysis> {
    let ws_service = state.workload_session_service.lock().await;
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    Json(EvidenceIntelligenceService::analyze_allocation_accuracy(
        &ws_service, &allocation, &owner,
    ))
}

/// POST /intelligence/findings — Notable findings and patterns
async fn handle_intelligence_findings(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<librarian_contracts::evidence_intelligence::IntelligenceFinding>> {
    let ws_service = state.workload_session_service.lock().await;
    let fleet = state.fleet_service.lock().await;
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    let bridge = state.capability_evidence_bridge.lock().await;
    let outcome = EvidenceIntelligenceService::analyze_workload_outcomes(&ws_service);
    let capability =
        EvidenceIntelligenceService::analyze_capability_effectiveness(&ws_service, &fleet, &bridge);
    let alloc_analysis =
        EvidenceIntelligenceService::analyze_allocation_accuracy(&ws_service, &allocation, &owner);
    Json(EvidenceIntelligenceService::generate_findings(
        &ws_service, &fleet, &outcome, &capability, &alloc_analysis,
    ))
}

// ============================================================================
// Evidence Classification Endpoints
// ============================================================================

/// GET /intelligence/catalog — Get finding category definitions
async fn handle_intelligence_catalog(
    State(state): State<Arc<AppState>>,
) -> Json<FindingCatalog> {
    let svc = state.evidence_classification_service.lock().await;
    Json(svc.get_catalog().clone())
}

#[derive(Debug, Deserialize)]
pub struct ClassifyRequest {
    pub findings: Vec<librarian_contracts::evidence_intelligence::IntelligenceFinding>,
}

/// POST /intelligence/classify — Classify raw intelligence into structured findings
async fn handle_intelligence_classify(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<ClassifyRequest>,
) -> Json<Vec<ClassifiedFinding>> {
    let mut svc = state.evidence_classification_service.lock().await;
    let mut results = Vec::new();
    for raw in body.findings {
        results.push(svc.classify_finding(raw, None));
    }
    Json(results)
}

#[derive(Debug, Deserialize)]
pub struct FindingsQuery {
    pub status: Option<String>,
    pub category: Option<String>,
}

/// GET /intelligence/findings — Get findings with optional filters
async fn handle_intelligence_findings_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<FindingsQuery>,
) -> Json<Vec<ClassifiedFinding>> {
    let svc = state.evidence_classification_service.lock().await;
    Json(svc.get_findings(query.status.as_deref(), query.category.as_deref()))
}

/// GET /intelligence/findings/summary — Summary of findings by severity, category, review status
async fn handle_intelligence_findings_summary(
    State(state): State<Arc<AppState>>,
) -> Json<FindingSummary> {
    let svc = state.evidence_classification_service.lock().await;
    Json(svc.get_findings_summary())
}

/// POST /intelligence/findings/review — Acknowledge, resolve, or dismiss a finding
async fn handle_intelligence_findings_review(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<FindingReviewAction>,
) -> Json<FindingReviewReceipt> {
    let mut svc = state.evidence_classification_service.lock().await;
    Json(svc.review_finding(body))
}

/// GET /intelligence/findings/receipts — Review action receipts
async fn handle_intelligence_findings_receipts(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<FindingReviewReceipt>> {
    let svc = state.evidence_classification_service.lock().await;
    Json(svc.get_receipts())
}

// ============================================================================
// Anomaly Detection Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AnomalyCheckRequest {
    pub metric_name: String,
    pub context: String,
    pub observed_value: f64,
    pub workload_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaselineResetResponse {
    pub status: String,
    pub metric_name: String,
    pub context: String,
    pub reset_at: String,
}

#[derive(Debug, Deserialize)]
pub struct BaselineResetRequest {
    pub metric_name: String,
    pub context: String,
}

/// GET /anomaly/baselines — Current baselines
async fn handle_anomaly_baselines(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<BaselineRecord>> {
    let svc = state.anomaly_detection_service.lock().await;
    Json(svc.get_all_baselines())
}

/// POST /anomaly/baselines/compute — Compute baselines from history
async fn handle_anomaly_baselines_compute(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<BaselineRecord>> {
    let ws_service = state.workload_session_service.lock().await;
    let mut svc = state.anomaly_detection_service.lock().await;
    let records = svc.compute_baselines_from_history(&ws_service);
    Json(records)
}

/// POST /anomaly/baselines/reset — Reset baselines (owner action)
async fn handle_anomaly_baselines_reset(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<BaselineResetRequest>,
) -> Json<BaselineResetResponse> {
    let mut svc = state.anomaly_detection_service.lock().await;
    svc.reset_baseline(&body.metric_name, &body.context);
    Json(BaselineResetResponse {
        status: "reset".to_string(),
        metric_name: body.metric_name,
        context: body.context,
        reset_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// POST /anomaly/check — Check a single metric for anomaly
async fn handle_anomaly_check(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<AnomalyCheckRequest>,
) -> Json<Option<AnomalyFinding>> {
    let svc = state.anomaly_detection_service.lock().await;
    Json(svc.check_for_anomalies(
        &body.metric_name,
        &body.context,
        body.observed_value,
        body.workload_ids,
    ))
}

/// POST /anomaly/scan — Scan all metrics against baselines
async fn handle_anomaly_scan(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<AnomalyFinding>> {
    let ws_service = state.workload_session_service.lock().await;
    let svc = state.anomaly_detection_service.lock().await;
    Json(svc.scan_all_metrics(&ws_service))
}

/// GET /anomaly/thresholds — Current thresholds
async fn handle_anomaly_thresholds_get(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<AnomalyThreshold>> {
    let svc = state.anomaly_detection_service.lock().await;
    Json(svc.get_thresholds())
}

/// PUT /anomaly/thresholds — Update thresholds
async fn handle_anomaly_thresholds_put(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<Vec<AnomalyThreshold>>,
) -> Json<Vec<AnomalyThreshold>> {
    let mut svc = state.anomaly_detection_service.lock().await;
    for threshold in body {
        svc.set_threshold(threshold);
    }
    Json(svc.get_thresholds())
}

#[derive(Debug, Deserialize)]
pub struct ClassifyAnomaliesRequest {
    pub anomaly_findings: Vec<AnomalyFinding>,
}

/// POST /anomaly/classify — Classify anomalies into controlled vocabulary
async fn handle_anomaly_classify(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<ClassifyAnomaliesRequest>,
) -> Json<Vec<ClassifiedFinding>> {
    let mut class_svc = state.evidence_classification_service.lock().await;
    let mut findings = Vec::new();
    for anomaly in body.anomaly_findings {
        let raw = librarian_contracts::evidence_intelligence::IntelligenceFinding {
            finding_id: anomaly.anomaly_id.clone(),
            category: "node_health".to_string(),
            severity: anomaly.severity.clone(),
            title: format!(
                "Anomaly detected: {} for {}",
                anomaly.observation.metric_name, anomaly.observation.context
            ),
            description: format!(
                "Deviation factor {:.2} (threshold {:.1}), observed {}, expected {:.1} +/- {:.1}",
                anomaly.observation.deviation_factor,
                anomaly.threshold_exceeded,
                anomaly.observation.observed_value,
                anomaly.observation.baseline_mean,
                anomaly.observation.baseline_std_dev,
            ),
            supporting_data: serde_json::to_value(&anomaly).unwrap_or_default(),
            source_references: anomaly.observation.evidence_workload_ids.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        findings.push(class_svc.classify_finding(raw, None));
    }
    Json(findings)
}

// ============================================================================
// Pattern Escalation Endpoints
// ============================================================================

/// POST /patterns/detect — Run pattern detection
async fn handle_patterns_detect(State(state): State<Arc<AppState>>) -> Json<Vec<PatternFinding>> {
    let class_svc = state.evidence_classification_service.lock().await;
    let anomaly_svc = state.anomaly_detection_service.lock().await;
    let mut pattern_svc = state.pattern_escalation_service.lock().await;
    let results = pattern_svc.detect_patterns(&class_svc, &anomaly_svc);
    Json(results)
}

#[derive(Debug, Deserialize)]
pub struct PatternsQuery {
    pub status: Option<String>,
    pub category: Option<String>,
}

/// GET /patterns — List patterns (optional filters)
async fn handle_patterns_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<PatternsQuery>,
) -> Json<Vec<PatternFinding>> {
    let pattern_svc = state.pattern_escalation_service.lock().await;
    Json(pattern_svc.get_patterns(query.status.as_deref(), query.category.as_deref()))
}

/// GET /patterns/{id} — Get specific pattern
async fn handle_patterns_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let pattern_svc = state.pattern_escalation_service.lock().await;
    match pattern_svc.get_pattern(&id) {
        Some(p) => (StatusCode::OK, Json(serde_json::to_value(p).unwrap_or_default())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Pattern {} not found", id)})),
        ),
    }
}

/// GET /patterns/summary — Pattern overview counts
async fn handle_patterns_summary(State(state): State<Arc<AppState>>) -> Json<PatternSummary> {
    let pattern_svc = state.pattern_escalation_service.lock().await;
    Json(pattern_svc.get_summary())
}

/// POST /patterns/{id}/acknowledge — Acknowledge pattern
async fn handle_patterns_acknowledge(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    body: Option<axum::Json<serde_json::Value>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let note = body.and_then(|b| b.0.get("note").and_then(|v| v.as_str().map(|s| s.to_string())));
    let mut pattern_svc = state.pattern_escalation_service.lock().await;
    match pattern_svc.acknowledge_pattern(&id, note) {
        Some(receipt) => (StatusCode::OK, Json(serde_json::to_value(receipt).unwrap_or_default())),
        None => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Cannot acknowledge pattern: must be in pending status"})),
        ),
    }
}

/// POST /patterns/{id}/resolve — Resolve pattern
async fn handle_patterns_resolve(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    body: Option<axum::Json<serde_json::Value>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let note = body.and_then(|b| b.0.get("note").and_then(|v| v.as_str().map(|s| s.to_string())));
    let mut pattern_svc = state.pattern_escalation_service.lock().await;
    match pattern_svc.resolve_pattern(&id, note) {
        Some(receipt) => (StatusCode::OK, Json(serde_json::to_value(receipt).unwrap_or_default())),
        None => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Cannot resolve pattern: must be in acknowledged or monitoring status"})),
        ),
    }
}

/// POST /patterns/{id}/dismiss — Dismiss pattern
async fn handle_patterns_dismiss(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    body: Option<axum::Json<serde_json::Value>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let note = body.and_then(|b| b.0.get("note").and_then(|v| v.as_str().map(|s| s.to_string())));
    let mut pattern_svc = state.pattern_escalation_service.lock().await;
    match pattern_svc.dismiss_pattern(&id, note) {
        Some(receipt) => (StatusCode::OK, Json(serde_json::to_value(receipt).unwrap_or_default())),
        None => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Cannot dismiss pattern: must be in pending or acknowledged status"})),
        ),
    }
}

/// GET /patterns/receipts — Review action history
async fn handle_patterns_receipts(State(state): State<Arc<AppState>>) -> Json<Vec<PatternReviewReceipt>> {
    let pattern_svc = state.pattern_escalation_service.lock().await;
    Json(pattern_svc.get_receipts())
}

/// GET /patterns/config — Current detection config
async fn handle_patterns_config_get(State(state): State<Arc<AppState>>) -> Json<PatternDetectionConfig> {
    let pattern_svc = state.pattern_escalation_service.lock().await;
    Json(pattern_svc.get_config())
}

/// PUT /patterns/config — Update detection config
async fn handle_patterns_config_put(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<PatternDetectionConfig>,
) -> Json<PatternDetectionConfig> {
    let mut pattern_svc = state.pattern_escalation_service.lock().await;
    pattern_svc.update_config(body);
    Json(pattern_svc.get_config())
}

/// POST /patterns/expire — Expire old patterns
async fn handle_patterns_expire(State(state): State<Arc<AppState>>) -> Json<Vec<PatternFinding>> {
    let mut pattern_svc = state.pattern_escalation_service.lock().await;
    Json(pattern_svc.expire_old_patterns())
}

// ============================================================================
// Owner Insight Endpoints
// ============================================================================

/// POST /owner/insight/dashboard — Complete owner-facing overview
async fn handle_owner_insight_dashboard(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::owner_insight::InsightDashboard> {
    let classification = state.evidence_classification_service.lock().await;
    let anomaly = state.anomaly_detection_service.lock().await;
    let ws = state.workload_session_service.lock().await;
    let fleet = state.fleet_service.lock().await;
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    let bridge = state.capability_evidence_bridge.lock().await;
    let node_id = state.node_identity_service.get_identity().node_id.clone();
    Json(OwnerInsightService::get_dashboard(
        &classification, &anomaly, &EvidenceIntelligenceService, &WorkloadLifecycleService,
        &ws, &fleet, &allocation, &owner, &bridge, &node_id,
    ))
}

#[derive(Debug, Deserialize)]
pub struct InsightReportRequest {
    pub period: Option<String>,
}

/// POST /owner/insight/report — Comprehensive intelligence report
async fn handle_owner_insight_report(
    State(state): State<Arc<AppState>>,
    body: Option<axum::Json<InsightReportRequest>>,
) -> Json<librarian_contracts::owner_insight::InsightReport> {
    let period = body.map(|b| b.period.clone().unwrap_or_else(|| "last_24h".to_string())).unwrap_or_else(|| "last_24h".to_string());
    let classification = state.evidence_classification_service.lock().await;
    let anomaly = state.anomaly_detection_service.lock().await;
    let ws = state.workload_session_service.lock().await;
    let fleet = state.fleet_service.lock().await;
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    let bridge = state.capability_evidence_bridge.lock().await;
    let node_id = state.node_identity_service.get_identity().node_id.clone();
    Json(OwnerInsightService::get_report(
        &period, &classification, &anomaly, &EvidenceIntelligenceService, &WorkloadLifecycleService,
        &ws, &fleet, &allocation, &owner, &bridge, &node_id,
    ))
}

#[derive(Debug, Deserialize)]
pub struct ComparePeriodsRequest {
    pub period_a_label: String,
    pub period_a_data: librarian_contracts::evidence_intelligence::WorkloadOutcomeAnalysis,
    pub period_b_label: String,
    pub period_b_data: librarian_contracts::evidence_intelligence::WorkloadOutcomeAnalysis,
}

/// POST /owner/insight/trends — Compare two time periods
async fn handle_owner_insight_trends(
    State(_state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<ComparePeriodsRequest>,
) -> Json<Vec<librarian_contracts::owner_insight::InsightComparison>> {
    Json(OwnerInsightService::compare_periods(
        &body.period_a_label,
        &body.period_a_data,
        &body.period_b_label,
        &body.period_b_data,
    ))
}

/// POST /owner/insight/workloads — Workload success/duration trends
async fn handle_owner_insight_workloads(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::owner_insight::WorkloadTrendSummary> {
    let ws = state.workload_session_service.lock().await;
    Json(OwnerInsightService::get_workload_trend(&ws))
}

/// POST /owner/insight/capabilities — Capability health overview
async fn handle_owner_insight_capabilities(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::owner_insight::CapabilityHealthSummary> {
    let fleet = state.fleet_service.lock().await;
    let bridge = state.capability_evidence_bridge.lock().await;
    Json(OwnerInsightService::get_capability_health(&fleet, &bridge))
}

/// POST /owner/insight/allocation — Allocation accuracy overview
async fn handle_owner_insight_allocation(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::owner_insight::AllocationQualitySummary> {
    let allocation = state.allocation_service.lock().await;
    let owner = state.owner_allocation_service.lock().await;
    Json(OwnerInsightService::get_allocation_quality(&allocation, &owner))
}

// ============================================================================
// Reconciliation Endpoints
// ============================================================================

/// POST /reconciliation/start — begin a new reconciliation cycle
async fn handle_reconciliation_start(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    let mut service = state.reconciliation_service.lock().await;
    let node_id = state.node_identity_service.get_identity().node_id.clone();
    match service.initiate_reconciliation(&node_id, "owner") {
        Ok(request) => (StatusCode::OK, Json(json!(request))),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

/// POST /reconciliation/compare — compare local state against expected state
#[derive(Debug, Deserialize)]
pub struct CompareRequest {
    pub request: ReconciliationRequest,
    pub expected_state: serde_json::Value,
}

async fn handle_reconciliation_compare(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<CompareRequest>,
) -> (StatusCode, Json<Value>) {
    let mut service = state.reconciliation_service.lock().await;
    let report = service.compare_state(&body.request, body.expected_state);
    (StatusCode::OK, Json(json!(report)))
}

/// POST /reconciliation/decide — submit a decision on a difference
async fn handle_reconciliation_decide(
    State(state): State<Arc<AppState>>,
    axum::Json(decision): axum::Json<ReconciliationDecision>,
) -> (StatusCode, Json<Value>) {
    let mut service = state.reconciliation_service.lock().await;
    match service.submit_decision(decision) {
        Ok(receipt) => (StatusCode::OK, Json(json!(receipt))),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

/// GET /reconciliation/report/{id} — get a reconciliation report by id
async fn handle_reconciliation_report(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(report_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let service = state.reconciliation_service.lock().await;
    let history = service.get_reconciliation_history();
    for receipt in &history {
        if let Some(payload_report) = receipt.payload.get("report_id") {
            if payload_report.as_str() == Some(&report_id) {
                return (StatusCode::OK, Json(receipt.payload.clone()));
            }
        }
    }
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("Report {} not found", report_id) })),
    )
}

/// GET /reconciliation/receipts — get all reconciliation receipts
async fn handle_reconciliation_receipts(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ReconciliationReceipt>> {
    let service = state.reconciliation_service.lock().await;
    Json(service.get_reconciliation_history())
}

// ============================================================================
// Recovery Custody Endpoints
// ============================================================================

/// POST /recovery/initiate — start recovery from a reconciliation outcome
#[derive(Debug, Deserialize)]
pub struct InitiateRecoveryRequest {
    pub reconciliation_receipt_id: String,
}

async fn handle_recovery_initiate(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<InitiateRecoveryRequest>,
) -> Json<RecoveryStatus> {
    let node_id = state.node_identity_service.get_identity().node_id.clone();
    let mut service = state.recovery_custody_service.lock().unwrap();
    Json(service.initiate_recovery(&node_id, &body.reconciliation_receipt_id))
}

/// POST /recovery/action — apply a recovery action
async fn handle_recovery_action(
    State(state): State<Arc<AppState>>,
    axum::Json(action): axum::Json<RecoveryAction>,
) -> (StatusCode, Json<serde_json::Value>) {
    let node_id = state.node_identity_service.get_identity().node_id.clone();
    let mut service = state.recovery_custody_service.lock().unwrap();
    match service.apply_action(action, &node_id) {
        Ok(receipt) => (StatusCode::OK, Json(serde_json::to_value(receipt).unwrap_or_default())),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

/// POST /recovery/owner-review — transition to OwnerReview state
async fn handle_recovery_owner_review(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut service = state.recovery_custody_service.lock().unwrap();
    match service.request_owner_review() {
        Ok(status) => (StatusCode::OK, Json(serde_json::to_value(status).unwrap_or_default())),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

/// POST /recovery/complete — complete recovery with decision
#[derive(Debug, Deserialize)]
pub struct CompleteRecoveryRequest {
    pub decision: String,
}

async fn handle_recovery_complete(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<CompleteRecoveryRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut service = state.recovery_custody_service.lock().unwrap();
    match service.complete_recovery(&body.decision) {
        Ok(report) => (StatusCode::OK, Json(serde_json::to_value(report).unwrap_or_default())),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

/// POST /recovery/fail — fail recovery with reason
#[derive(Debug, Deserialize)]
pub struct FailRecoveryRequest {
    pub reason: String,
}

async fn handle_recovery_fail(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<FailRecoveryRequest>,
) -> Json<RecoveryReport> {
    let mut service = state.recovery_custody_service.lock().unwrap();
    Json(service.fail_recovery(&body.reason))
}

/// GET /recovery/status — get current recovery status
async fn handle_recovery_status(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let service = state.recovery_custody_service.lock().unwrap();
    match service.get_status() {
        Some(status) => (StatusCode::OK, Json(serde_json::to_value(status).unwrap_or_default())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No active recovery" })),
        ),
    }
}

/// GET /recovery/report/{id} — get recovery report by id
async fn handle_recovery_report(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(recovery_id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let service = state.recovery_custody_service.lock().unwrap();
    match service.get_report(&recovery_id) {
        Some(report) => (StatusCode::OK, Json(serde_json::to_value(report).unwrap_or_default())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Recovery report for {} not found", recovery_id) })),
        ),
    }
}

// ============================================================================
// Policy Endpoints
// ============================================================================

/// GET /policy — get all current policies
async fn handle_policy_get_all(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::policy::PolicyConfig> {
    let svc = state.policy_service.lock().await;
    Json(svc.get_policies())
}

/// GET /policy/{name} — get a single policy by name
async fn handle_policy_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let svc = state.policy_service.lock().await;
    match svc.get_policy(&name) {
        Some(entry) => (StatusCode::OK, Json(serde_json::to_value(&entry).unwrap_or_default())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Policy '{}' not found", name)})),
        ),
    }
}

/// PUT /policy/{name} — update a policy value, produces a receipt
#[derive(Debug, serde::Deserialize)]
pub struct UpdatePolicyBody {
    pub value: serde_json::Value,
    pub owner: String,
}

async fn handle_policy_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    axum::Json(body): axum::Json<UpdatePolicyBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut svc = state.policy_service.lock().await;
    match svc.update_policy(&name, body.value, &body.owner) {
        Some(receipt) => (StatusCode::OK, Json(serde_json::to_value(&receipt).unwrap_or_default())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Policy '{}' not found", name)})),
        ),
    }
}

/// GET /policy/receipts — get all policy change receipts
async fn handle_policy_receipts(State(state): State<Arc<AppState>>) -> Json<Vec<PolicyChangeReceipt>> {
    let svc = state.policy_service.lock().await;
    Json(svc.get_receipts())
}

// ============================================================================
// Model-Runtime Endpoints
// ============================================================================

/// GET /model-runtime/profiles — available runtime capabilities with qualification evidence
async fn handle_model_runtime_profiles(State(state): State<Arc<AppState>>) -> Json<Vec<RuntimeCapability>> {
    let svc = state.model_runtime_service.lock().await;
    Json(svc.get_runtime_profiles())
}

/// GET /model-runtime/{model_id} — get a model's runtime profile with evidence
async fn handle_model_runtime_profile(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(model_id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let svc = state.model_runtime_service.lock().await;
    match svc.get_model_runtime_profile(&model_id) {
        Some(profile) => (StatusCode::OK, Json(serde_json::to_value(&profile).unwrap_or_default())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("No runtime profile for model '{}'", model_id)})),
        ),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct LinkModelRuntimeRequest {
    pub model_id: String,
    pub runtime_type: String,
    pub evidence_packet_id: String,
    pub qualification_run_id: String,
}

/// POST /model-runtime/link — link qualification evidence to a model's runtime capability
async fn handle_model_runtime_link(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<LinkModelRuntimeRequest>,
) -> Json<ModelRuntimeEvidenceLink> {
    let mut svc = state.model_runtime_service.lock().await;
    let link = svc.link_evidence(
        &body.model_id,
        &body.runtime_type,
        &body.evidence_packet_id,
        &body.qualification_run_id,
    );
    Json(link)
}

/// GET /model-runtime/{model_id}/evidence — get evidence links for a model
async fn handle_model_runtime_evidence(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(model_id): axum::extract::Path<String>,
) -> Json<Vec<ModelRuntimeEvidenceLink>> {
    let svc = state.model_runtime_service.lock().await;
    Json(svc.get_evidence_links(&model_id))
}

// ============================================================================
// Registry Enforcement Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct EnforcementEventsQuery {
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnforcementPolicyRequest {
    pub rules: Vec<librarian_contracts::registry_enforcement::EnforcementRule>,
    pub version: u32,
}

/// GET /registry/enforcement/policy
async fn handle_registry_enforcement_policy_get(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::registry_enforcement::EnforcementPolicy> {
    let enf = state.registry_enforcement_service.lock().await;
    Json(enf.get_enforcement_policy())
}

/// PUT /registry/enforcement/policy
async fn handle_registry_enforcement_policy_put(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<librarian_contracts::registry_enforcement::EnforcementPolicy>,
) -> Json<librarian_contracts::registry_enforcement::EnforcementPolicy> {
    let mut enf = state.registry_enforcement_service.lock().await;
    enf.update_enforcement_policy(body);
    Json(enf.get_enforcement_policy())
}

/// GET /registry/enforcement/events?scope=
async fn handle_registry_enforcement_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<EnforcementEventsQuery>,
) -> Json<Vec<librarian_contracts::registry_enforcement::EnforcementEvent>> {
    let enf = state.registry_enforcement_service.lock().await;
    Json(enf.get_enforcement_events(query.scope.as_deref()))
}

// ============================================================================
// Registry Candidate Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DiscoverCandidateRequest {
    pub node_id: String,
    pub display_name: String,
    pub discovery_method: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewCandidateRequest {
    pub decision: String,
    pub reviewer: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ExpireCandidatesRequest {
    pub days_old: u32,
}

#[derive(Debug, Deserialize)]
pub struct CandidatesQuery {
    pub status: Option<String>,
}

/// POST /registry/candidate/discover
async fn handle_registry_candidate_discover(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<DiscoverCandidateRequest>,
) -> Json<librarian_contracts::registry::NodeCandidate> {
    let mut svc = state.registry_candidate_service.lock().await;
    let method = body
        .discovery_method
        .as_deref()
        .map(|m| m.into())
        .unwrap_or(librarian_contracts::registry::DiscoveryMethod::ApiDiscovery);
    let candidate = svc.discover(&body.node_id, &body.display_name, method);
    Json(candidate)
}

/// POST /registry/candidate/{id}/evidence/collect
async fn handle_registry_candidate_collect_evidence(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let identity = state.node_identity_service.get_identity().clone();
    let bridge_state = {
        let bridge = state.capability_evidence_bridge.lock().await;
        bridge.get_verification_state(&identity.node_id)
    };
    let custody_chain = {
        let custody = state.custody_service.lock().unwrap();
        custody.get_chain()
    };
    let fleet_entry = {
        let fleet = state.fleet_service.lock().await;
        fleet.get_node(&identity.node_id)
    };

    let mut svc = state.registry_candidate_service.lock().await;
    let evidence = svc.collect_evidence_simple(&id, &identity, &bridge_state, &custody_chain, &fleet_entry);
    if evidence.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Candidate {} not found", id) })),
        );
    }
    (StatusCode::OK, Json(json!(evidence)))
}

/// POST /registry/candidate/{id}/submit
async fn handle_registry_candidate_submit(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let mut svc = state.registry_candidate_service.lock().await;
    match svc.submit_for_review(&id) {
        Some(candidate) => (StatusCode::OK, Json(json!(candidate))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Candidate {} not found", id) })),
        ),
    }
}

/// POST /registry/candidate/{id}/review
async fn handle_registry_candidate_review(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<ReviewCandidateRequest>,
) -> (StatusCode, Json<Value>) {
    use librarian_contracts::registry_owner::OwnerActionType;

    let decision: librarian_contracts::registry::ReviewDecision = body.decision.as_str().into();
    let action_type = match decision {
        librarian_contracts::registry::ReviewDecision::Approve => OwnerActionType::ApproveCandidate,
        librarian_contracts::registry::ReviewDecision::Reject => OwnerActionType::RejectCandidate,
        _ => {
            // RequestInfo — execute directly, no owner action needed
            let mut reg = state.registration_service.lock().await;
            let mut svc = state.registry_candidate_service.lock().await;
            return match svc.review(&id, decision, &body.reviewer, &body.reason, &mut reg) {
                Some(receipt) => (StatusCode::OK, Json(json!(receipt))),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": format!("Candidate {} not found", id) })),
                ),
            };
        }
    };

    // Create owner action that must be approved before execution
    let mut owner_svc = state.registry_owner_service.lock().await;
    let action = owner_svc.create_action(
        action_type,
        &id,
        "candidate",
        &body.reviewer,
        &body.reason,
    );

    (StatusCode::OK, Json(json!({
        "status": "action_created",
        "action": action,
        "message": "Owner action created. Use POST /registry/owner/action/{id}/approve to execute.",
    })))
}

/// GET /registry/candidates?status=
async fn handle_registry_candidates_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<CandidatesQuery>,
) -> Json<Value> {
    let svc = state.registry_candidate_service.lock().await;
    let candidates = svc.get_candidates(query.status.as_deref());
    Json(json!({ "candidates": candidates, "count": candidates.len() }))
}

/// GET /registry/candidate/{id}
async fn handle_registry_candidate_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let svc = state.registry_candidate_service.lock().await;
    match svc.get_candidate(&id) {
        Some(candidate) => (StatusCode::OK, Json(json!(candidate))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Candidate {} not found", id) })),
        ),
    }
}

/// GET /registry/candidate/{id}/evidence
async fn handle_registry_candidate_evidence(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let svc = state.registry_candidate_service.lock().await;
    let evidence = svc.get_evidence(&id);
    if evidence.is_empty() && svc.get_candidate(&id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Candidate {} not found", id) })),
        );
    }
    (StatusCode::OK, Json(json!({ "evidence": evidence, "count": evidence.len() })))
}

/// POST /registry/candidate/expire
async fn handle_registry_candidate_expire(
    State(state): State<Arc<AppState>>,
    axum::Json(_body): axum::Json<ExpireCandidatesRequest>,
) -> Json<Value> {
    let mut enforcement = state.registry_enforcement_service.lock().await;
    let mut svc = state.registry_candidate_service.lock().await;
    let policy = state.policy_service.lock().await;
    let expired = enforcement.check_candidate_expiry(&mut svc, &policy);
    Json(json!({ "expired": expired, "count": expired.len() }))
}

// ============================================================================
// Registry MCP Endpoints
// ============================================================================

/// GET /registry/mcp/catalog — returns the full MCP tool catalog
async fn handle_registry_mcp_catalog(State(state): State<Arc<AppState>>) -> Json<librarian_contracts::registry_mcp::McpToolCatalog> {
    let mcp = state.registry_mcp_service.lock().await;
    Json(mcp.get_tool_catalog())
}

/// POST /registry/mcp/execute — execute an MCP tool request
async fn handle_registry_mcp_execute(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<librarian_contracts::registry_mcp::McpToolRequest>,
) -> (StatusCode, Json<librarian_contracts::registry_mcp::McpToolResponse>) {
    let mut mcp = state.registry_mcp_service.lock().await;
    let candidates = state.registry_candidate_service.lock().await;
    let identity = state.node_identity_service.clone();
    let node_state = state.node_state.lock().await;
    let mut ow = state.owner_workflow_service.lock().await;

    let response = mcp.execute_tool(body, &candidates, identity.as_ref(), &node_state, &mut ow);
    (StatusCode::OK, Json(response))
}

// ============================================================================
// Registry Owner Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateOwnerActionRequest {
    pub action_type: String,
    pub target_id: String,
    pub target_type: String,
    pub owner: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RejectOwnerActionRequest {
    pub reason: String,
}

/// POST /registry/owner/action — create a pending owner action
async fn handle_registry_owner_create_action(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<CreateOwnerActionRequest>,
) -> (
    StatusCode,
    Json<librarian_contracts::registry_owner::RegistryOwnerAction>,
) {
    let mut svc = state.registry_owner_service.lock().await;
    let action_type: librarian_contracts::registry_owner::OwnerActionType =
        body.action_type.as_str().into();
    let action = svc.create_action(
        action_type,
        &body.target_id,
        &body.target_type,
        &body.owner,
        &body.reason,
    );
    (StatusCode::OK, Json(action))
}

/// POST /registry/owner/action/{id}/approve — approve and execute an owner action
async fn handle_registry_owner_approve_action(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut svc = state.registry_owner_service.lock().await;
    let mut candidates = state.registry_candidate_service.lock().await;
    let mut reg = state.registration_service.lock().await;

    match svc.approve_action(&id, &mut candidates, &mut reg) {
        Some(receipt) => (
            StatusCode::OK,
            Json(serde_json::to_value(receipt).unwrap_or_default()),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Action {} not found or not pending", id)
            })),
        ),
    }
}

/// POST /registry/owner/action/{id}/reject — reject an owner action without execution
async fn handle_registry_owner_reject_action(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<RejectOwnerActionRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut svc = state.registry_owner_service.lock().await;

    match svc.reject_action(&id, &body.reason) {
        Some(receipt) => (
            StatusCode::OK,
            Json(serde_json::to_value(receipt).unwrap_or_default()),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Action {} not found or not pending", id)
            })),
        ),
    }
}

/// GET /registry/owner/actions/pending — list all pending owner actions
async fn handle_registry_owner_pending_actions(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let svc = state.registry_owner_service.lock().await;
    let actions = svc.get_pending_actions();
    Json(serde_json::json!({
        "actions": actions,
        "count": actions.len()
    }))
}

/// GET /registry/owner/actions/history — list all past owner action receipts
async fn handle_registry_owner_action_history(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let svc = state.registry_owner_service.lock().await;
    let history = svc.get_action_history();
    Json(serde_json::json!({
        "receipts": history,
        "count": history.len()
    }))
}

// ============================================================================
// Registry Apply endpoint handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ProposeChangeRequest {
    pub target_type: String,
    pub target_id: String,
    pub proposed_state: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct RejectChangeRequest {
    pub reason: String,
}

/// POST /registry/apply/propose — propose a registry state change
async fn handle_registry_apply_propose(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<ProposeChangeRequest>,
) -> (
    StatusCode,
    Json<librarian_contracts::registry_apply::RegistryStateChange>,
) {
    let mut svc = state.registry_apply_service.lock().await;
    let change = svc.propose_change(
        &body.target_type,
        &body.target_id,
        body.proposed_state,
    );
    (StatusCode::OK, Json(change))
}

/// POST /registry/apply/{id}/approve — approve a proposed change
async fn handle_registry_apply_approve(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut svc = state.registry_apply_service.lock().await;
    match svc.approve_change(&id, "api") {
        Some(change) => (
            StatusCode::OK,
            Json(serde_json::to_value(change).unwrap_or_default()),
        ),
        None => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Change {} not found or not in proposed state", id)
            })),
        ),
    }
}

/// POST /registry/apply/{id}/apply — apply an approved change
async fn handle_registry_apply_apply(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut svc = state.registry_apply_service.lock().await;
    let change = svc.get_change(&id);

    let change = match change {
        Some(c) => c,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Change {} not found", id)})),
            );
        }
    };

    if change.status != librarian_contracts::registry_apply::ChangeStatus::Approved {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!(
                    "Change {} must be approved before applying. Current status: {}",
                    id,
                    change.status
                )
            })),
        );
    }

    let target_type = change.target_type.clone();
    let target_id = change.target_id.clone();
    let proposed_state = change.proposed_state.clone();
    drop(change);

    let result = svc.apply_change(&id, "api", |_| {
        let result: Result<serde_json::Value, String> = Ok(serde_json::json!({
            "applied": true,
            "target_type": target_type,
            "target_id": target_id,
            "proposed_state": proposed_state,
        }));
        result
    });

    match result {
        Some(change) => (
            StatusCode::OK,
            Json(serde_json::to_value(change).unwrap_or_default()),
        ),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to apply change {}", id)})),
        ),
    }
}

/// POST /registry/apply/{id}/verify — verify an applied change
async fn handle_registry_apply_verify(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut svc = state.registry_apply_service.lock().await;
    match svc.verify_change(&id, "api") {
        Some(change) => (
            StatusCode::OK,
            Json(serde_json::to_value(change).unwrap_or_default()),
        ),
        None => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Change {} not found or not in applied state", id)
            })),
        ),
    }
}

/// POST /registry/apply/{id}/reject — reject a proposed or approved change
async fn handle_registry_apply_reject(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(body): axum::Json<RejectChangeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut svc = state.registry_apply_service.lock().await;
    match svc.reject_change(&id, &body.reason) {
        Some(change) => (
            StatusCode::OK,
            Json(serde_json::to_value(change).unwrap_or_default()),
        ),
        None => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!(
                    "Change {} not found or cannot be rejected (status must be proposed or approved)",
                    id
                )
            })),
        ),
    }
}

// ============================================================================
// Registry Hardening Endpoints
// ============================================================================

/// GET /registry/health — registry service health
async fn handle_registry_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let candidate_svc = state.registry_candidate_service.lock().await;
    let enforcement_svc = state.registry_enforcement_service.lock().await;
    let owner_svc = state.registry_owner_service.lock().await;

    let file_integrity = candidate_svc.verify_file_integrity();
    let candidate_count = candidate_svc.candidate_count();
    let evidence_count = candidate_svc.get_all_evidence().len();
    let enforcement_events = enforcement_svc.get_enforcement_events(None).len();
    let owner_receipts = owner_svc.get_action_history().len();

    Json(json!({
        "status": if file_integrity { "healthy" } else { "degraded" },
        "components": {
            "file_integrity": file_integrity,
            "candidate_count": candidate_count,
            "evidence_count": evidence_count,
            "enforcement_events_logged": enforcement_events,
            "owner_receipts_logged": owner_receipts,
        },
        "checked_at": chrono::Utc::now().to_rfc3339(),
    }))
}

/// POST /registry/cleanup — trigger stale candidate and evidence cleanup
async fn handle_registry_cleanup(State(state): State<Arc<AppState>>) -> Json<Value> {
    let mut candidate_svc = state.registry_candidate_service.lock().await;
    let mut enforcement_svc = state.registry_enforcement_service.lock().await;

    let summary = candidate_svc.cleanup_stale();

    // Log enforcement events for expired candidates
    for cand_id in &summary.expired_candidate_ids {
        enforcement_svc.log_event(
            "enf-cleanup-auto",
            "candidate",
            cand_id,
            "Candidate auto-expired via cleanup",
            "expire",
        );
    }

    // Log enforcement events for flagged stale reviews
    for cand_id in &summary.flagged_candidate_ids {
        enforcement_svc.log_event(
            "enf-review-stale",
            "candidate",
            cand_id,
            &format!("Candidate in under_review state >48h — flagging for review"),
            "flag",
        );
    }

    Json(json!({
        "status": "cleanup_completed",
        "summary": {
            "expired_candidates": summary.expired_candidates,
            "expired_candidate_ids": summary.expired_candidate_ids,
            "flagged_stale_review": summary.flagged_stale_review,
            "flagged_candidate_ids": summary.flagged_candidate_ids,
            "evidence_purged": summary.evidence_purged,
            "evidence_before_count": summary.evidence_before_count,
            "evidence_after_count": summary.evidence_after_count,
        },
        "completed_at": chrono::Utc::now().to_rfc3339(),
    }))
}

/// GET /registry/version — current registry schema version
async fn handle_registry_version() -> Json<Value> {
    Json(json!({
        "registry_schema_version": 4u32,
        "description": "Registry file format version (librarian-core REGISTRY_SCHEMA_VERSION)",
    }))
}

/// GET /registry/apply/pending — list pending changes
async fn handle_registry_apply_pending(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let svc = state.registry_apply_service.lock().await;
    let changes = svc.get_pending_changes();
    Json(serde_json::json!({
        "changes": changes,
        "count": changes.len()
    }))
}

/// GET /registry/apply/history — list change receipt history
async fn handle_registry_apply_history(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let svc = state.registry_apply_service.lock().await;
    let history = svc.get_change_history();
    Json(serde_json::json!({
        "receipts": history,
        "count": history.len()
    }))
}

// ============================================================================
// Router construction
// ============================================================================

/// Build the axum Router with all contract endpoints.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/backend/status", get(handle_status))
        .route("/backend/profiles", get(handle_profiles))
        .route("/backend/health", get(handle_health))
        .route("/health", get(handle_health_legacy))
        .route("/backend/select", post(handle_select))
        .route("/backend/stop", post(handle_stop))
        .route("/backend/restart", post(handle_restart))
        .route("/backend/chat", post(handle_chat))
        .route("/v1/chat/completions", post(handle_v1_chat))
        .route("/v1/models", get(handle_v1_models))
        .route("/evidence/runs/{run_id}", get(handle_evidence_run))
        .route("/evidence/lifecycle", get(handle_evidence_lifecycle))
        .route("/residency/status", get(handle_residency_status))
        .route("/operator/state", get(handle_operator_state))
        .route("/operator/events", get(handle_operator_events))
        .route("/operator/dashboard", get(handle_operator_dashboard))
        .route("/operator/dashboard/data", get(handle_operator_dashboard_data))
        .route("/node/identity", get(handle_node_identity))
        .route("/node/status", get(handle_node_status))
        .route("/node/capabilities", get(handle_node_capabilities))
        .route("/node/capabilities/evidence", get(handle_capability_evidence))
        .route("/node/capabilities/evidence/link", post(handle_link_evidence))
        .route("/node/capabilities/evidence/verify", post(handle_verify_evidence))
        .route("/node/capabilities/unverified", get(handle_unverified_claims))
        .route("/node/capabilities/{type}/state", put(handle_capability_transition_state))
        .route("/node/capabilities/lifecycle", get(handle_capability_lifecycle))
        .route("/node/registration", get(handle_node_registration))
        .route("/node/register", post(handle_node_register))
        .route("/node/register/confirm", post(handle_node_register_confirm))
        .route("/session/start", post(handle_session_start))
        .route("/session/{session_id}/activate", post(handle_session_activate))
        .route("/session/{session_id}/close", post(handle_session_close))
        .route("/session/{session_id}/expire", post(handle_session_expire))
        .route("/session/{session_id}", get(handle_session_get))
        .route("/sessions", get(handle_sessions_list))
        .route("/session/{session_id}/receipt", get(handle_session_receipt))
        .route("/sessions/receipts", get(handle_sessions_receipts))
        .route("/bootstrap/assess", post(handle_bootstrap_assess))
        .route("/bootstrap/plan", post(handle_bootstrap_create_plan))
        .route("/bootstrap/plan/{plan_id}/execute", post(handle_bootstrap_execute_plan))
        .route("/bootstrap/plan/{plan_id}/approve", post(handle_bootstrap_approve_plan))
        .route("/bootstrap/plan/{plan_id}", get(handle_bootstrap_get_plan))
        .route("/bootstrap/assessment/{assessment_id}", get(handle_bootstrap_get_assessment))
        .route("/custody/chain", get(handle_custody_chain))
        .route("/custody/envelopes", get(handle_custody_envelopes))
        .route("/custody/envelope/{envelope_id}", get(handle_custody_envelope))
        .route("/custody/provenance", get(handle_custody_provenance))
        .route("/custody/provenance/graph", get(handle_custody_provenance_graph))
        .route("/custody/verify", post(handle_custody_verify))
        .route("/custody/retention", post(handle_custody_retention))
        .route("/core/projection", get(handle_core_projection))
        .route("/core/sync/prepare", post(handle_core_sync_prepare))
        .route("/core/sync/receipt", post(handle_core_sync_receipt))
        .route("/core/discover", post(handle_core_discover))
        .route("/core/discover/response", post(handle_core_discover_response))
        .route("/core/status", get(handle_core_status))
        .route("/ops/health", get(handle_ops_health))
        .route("/ops/overview", get(handle_ops_overview))
        .route("/ops/diagnostics", get(handle_ops_diagnostics))
        .route("/ops/health/summary", get(handle_ops_health_summary))
        .route("/ops/status", get(handle_ops_status))
        .route("/owner/review/node", post(handle_owner_review_node))
        .route("/owner/review/capabilities", post(handle_owner_review_capabilities))
        .route("/owner/review/sessions", post(handle_owner_review_sessions))
        .route("/owner/review/custody", post(handle_owner_review_custody))
        .route("/owner/review/bootstrap", post(handle_owner_review_bootstrap))
        .route("/owner/pending", get(handle_owner_pending))
        .route("/owner/decide", post(handle_owner_decide))
        .route("/owner/history", get(handle_owner_history))
        .route("/fleet/inventory", get(handle_fleet_inventory))
        .route("/fleet/inventory/{node_id}", get(handle_fleet_inventory_node))
        .route("/fleet/health", get(handle_fleet_health))
        .route("/fleet/health/breakdown", get(handle_fleet_health_breakdown))
        .route("/fleet/capabilities", get(handle_fleet_capabilities))
        .route("/fleet/overview", get(handle_fleet_overview))
        .route("/fleet/discover", post(handle_fleet_discover))
        .route("/fleet/nodes", post(handle_fleet_nodes_post))
        .route("/fleet/trust", get(handle_fleet_trust))
        .route("/fleet/trust/{node_id}", get(handle_fleet_trust_node))
        .route("/fleet/trust/assess", post(handle_fleet_trust_assess))
        .route("/fleet/trust/receipts", get(handle_fleet_trust_receipts))
        .route("/allocation/evaluate", post(handle_allocation_evaluate))
        .route("/allocation/score", post(handle_allocation_score))
        .route("/allocation/recommend", post(handle_allocation_recommend))
        .route("/allocation/recommend/{id}/accept", post(handle_allocation_accept))
        .route("/allocation/recommend/{id}/reject", post(handle_allocation_reject))
        .route("/allocation/recommendations", get(handle_allocation_recommendations))
        .route("/allocation/receipts", get(handle_allocation_receipts))
        .route("/owner/allocation/pending", get(handle_owner_allocation_pending))
        .route("/owner/allocation/review", post(handle_owner_allocation_review))
        .route("/owner/allocation/recommendation/{id}", get(handle_owner_allocation_recommendation_detail))
        .route("/owner/allocation/decide", post(handle_owner_allocation_decide))
        .route("/owner/allocation/history", get(handle_owner_allocation_history))
        .route("/owner/allocation/actions", get(handle_owner_allocation_actions))
        .route("/workload/session/create", post(handle_workload_session_create))
        .route("/workload/session/{id}/activate", post(handle_workload_session_activate))
        .route("/workload/session/{id}/complete", post(handle_workload_session_complete))
        .route("/workload/session/{id}/fail", post(handle_workload_session_fail))
        .route("/workload/session/{id}", get(handle_workload_session_get))
        .route("/workload/sessions", get(handle_workload_sessions_list))
        .route("/workload/session/{id}/link", get(handle_workload_session_link))
        .route("/workload/receipts", get(handle_workload_receipts))
        .route("/workload/inventory", get(handle_workload_inventory))
        .route("/workload/timeline/{workload_id}", get(handle_workload_timeline))
        .route("/workload/history", post(handle_workload_history))
        .route("/workload/review/{workload_id}", get(handle_workload_review))
        .route("/workload/active", get(handle_workload_active))
        .route("/workload/failed", get(handle_workload_failed))
        .route("/workload/summary", get(handle_workload_summary))
        .route("/intelligence/report", post(handle_intelligence_report))
        .route("/intelligence/workloads", post(handle_intelligence_workloads))
        .route("/intelligence/capabilities", post(handle_intelligence_capabilities))
        .route("/intelligence/allocation", post(handle_intelligence_allocation))
        .route("/intelligence/findings", post(handle_intelligence_findings))
        .route("/intelligence/catalog", get(handle_intelligence_catalog))
        .route("/intelligence/classify", post(handle_intelligence_classify))
        .route("/intelligence/findings/summary", get(handle_intelligence_findings_summary))
        .route("/intelligence/findings/review", post(handle_intelligence_findings_review))
        .route("/intelligence/findings/receipts", get(handle_intelligence_findings_receipts))
        .route("/anomaly/baselines", get(handle_anomaly_baselines))
        .route("/anomaly/baselines/compute", post(handle_anomaly_baselines_compute))
        .route("/anomaly/baselines/reset", post(handle_anomaly_baselines_reset))
        .route("/anomaly/check", post(handle_anomaly_check))
        .route("/anomaly/scan", post(handle_anomaly_scan))
        .route("/anomaly/thresholds", get(handle_anomaly_thresholds_get).put(handle_anomaly_thresholds_put))
        .route("/anomaly/classify", post(handle_anomaly_classify))
        .route("/patterns/detect", post(handle_patterns_detect))
        .route("/patterns", get(handle_patterns_list))
        .route("/patterns/summary", get(handle_patterns_summary))
        .route("/patterns/receipts", get(handle_patterns_receipts))
        .route("/patterns/config", get(handle_patterns_config_get).put(handle_patterns_config_put))
        .route("/patterns/expire", post(handle_patterns_expire))
        .route("/patterns/{id}", get(handle_patterns_get))
        .route("/patterns/{id}/acknowledge", post(handle_patterns_acknowledge))
        .route("/patterns/{id}/resolve", post(handle_patterns_resolve))
        .route("/patterns/{id}/dismiss", post(handle_patterns_dismiss))
        .route("/owner/insight/dashboard", post(handle_owner_insight_dashboard))
        .route("/owner/insight/report", post(handle_owner_insight_report))
        .route("/owner/insight/trends", post(handle_owner_insight_trends))
        .route("/owner/insight/workloads", post(handle_owner_insight_workloads))
        .route("/owner/insight/capabilities", post(handle_owner_insight_capabilities))
        .route("/owner/insight/allocation", post(handle_owner_insight_allocation))
        .route("/reconciliation/start", post(handle_reconciliation_start))
        .route("/reconciliation/compare", post(handle_reconciliation_compare))
        .route("/reconciliation/decide", post(handle_reconciliation_decide))
        .route("/reconciliation/report/{id}", get(handle_reconciliation_report))
        .route("/reconciliation/receipts", get(handle_reconciliation_receipts))
        .route("/recovery/initiate", post(handle_recovery_initiate))
        .route("/recovery/action", post(handle_recovery_action))
        .route("/recovery/owner-review", post(handle_recovery_owner_review))
        .route("/recovery/complete", post(handle_recovery_complete))
        .route("/recovery/fail", post(handle_recovery_fail))
        .route("/recovery/status", get(handle_recovery_status))
        .route("/recovery/report/{id}", get(handle_recovery_report))
        .route("/policy", get(handle_policy_get_all))
        .route("/policy/{name}", get(handle_policy_get).put(handle_policy_update))
        .route("/policy/receipts", get(handle_policy_receipts))
        .route("/model-runtime/profiles", get(handle_model_runtime_profiles))
        .route("/model-runtime/{model_id}", get(handle_model_runtime_profile))
        .route("/model-runtime/{model_id}/evidence", get(handle_model_runtime_evidence))
        .route("/model-runtime/link", post(handle_model_runtime_link))
        .route("/registry/enforcement/policy", get(handle_registry_enforcement_policy_get).put(handle_registry_enforcement_policy_put))
        .route("/registry/enforcement/events", get(handle_registry_enforcement_events))
        .route("/registry/candidate/discover", post(handle_registry_candidate_discover))
        .route("/registry/candidate/{id}/evidence/collect", post(handle_registry_candidate_collect_evidence))
        .route("/registry/candidate/{id}/submit", post(handle_registry_candidate_submit))
        .route("/registry/candidate/{id}/review", post(handle_registry_candidate_review))
        .route("/registry/candidates", get(handle_registry_candidates_list))
        .route("/registry/candidate/{id}", get(handle_registry_candidate_get))
        .route("/registry/candidate/{id}/evidence", get(handle_registry_candidate_evidence))
        .route("/registry/candidate/expire", post(handle_registry_candidate_expire))
        .route("/registry/mcp/catalog", get(handle_registry_mcp_catalog))
        .route("/registry/mcp/execute", post(handle_registry_mcp_execute))
        .route("/registry/health", get(handle_registry_health))
        .route("/registry/cleanup", post(handle_registry_cleanup))
        .route("/registry/version", get(handle_registry_version))
        .route("/registry/owner/action", post(handle_registry_owner_create_action))
        .route("/registry/owner/action/{id}/approve", post(handle_registry_owner_approve_action))
        .route("/registry/owner/action/{id}/reject", post(handle_registry_owner_reject_action))
        .route("/registry/owner/actions/pending", get(handle_registry_owner_pending_actions))
        .route("/registry/owner/actions/history", get(handle_registry_owner_action_history))
        .route("/registry/apply/propose", post(handle_registry_apply_propose))
        .route("/registry/apply/{id}/approve", post(handle_registry_apply_approve))
        .route("/registry/apply/{id}/apply", post(handle_registry_apply_apply))
        .route("/registry/apply/{id}/verify", post(handle_registry_apply_verify))
        .route("/registry/apply/{id}/reject", post(handle_registry_apply_reject))
        .route("/registry/apply/pending", get(handle_registry_apply_pending))
        .route("/registry/apply/history", get(handle_registry_apply_history))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(DefaultBodyLimit::max(state.config.max_body_bytes))
        .layer(
            tower_http::cors::CorsLayer::permissive()
        )
        .fallback(handle_404)
        .with_state(state)
}

/// 404 catch-all handler matching Python router's JSON error response shape.
async fn handle_404(req: axum::extract::Request) -> (StatusCode, Json<Value>) {
    let path = req.uri().path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(req.uri().path());
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": format!("Not found: {}", path),
        })),
    )
}

/// Start the background health poller.
/// Polls all running backends at the given interval and updates their state.
/// Does NOT auto-restart backends - only updates state to degraded/failed.
pub async fn start_health_poller(state: Arc<AppState>, interval_secs: u64) {
    let state_for_spawn = state.clone();
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        info!("Health poller started (interval: {}s)", interval_secs);

        loop {
            interval.tick().await;

            // Get list of backend aliases to check
            let aliases: Vec<String> = {
                let backends = state_for_spawn.backends.lock().await;
                backends.keys().cloned().collect()
            };

            for alias in aliases {
                let bp = {
                    let backends = state_for_spawn.backends.lock().await;
                    backends.get(&alias).cloned()
                };

                if let Some(bp) = bp {
                    let state_val = bp.get_state().await;
                    // Only poll if not stopped/failed
                    if state_val != BackendState::Stopped && state_val != BackendState::Failed {
                        let healthy = bp.check_health().await;
                        if !healthy {
                            let new_state = bp.get_state().await;
                            if new_state == BackendState::Degraded {
                                warn!("[{}] health poller: backend degraded", alias);
                            } else if new_state == BackendState::Failed {
                                warn!("[{}] health poller: backend failed", alias);
                            }
                        }
                    }
                }
            }
        }
    });

    // Store the handle for graceful shutdown
    let mut handle_guard = state.health_poller_handle.lock().await;
    *handle_guard = Some(handle);
}

/// Stop the background health poller.
pub async fn stop_health_poller(state: &Arc<AppState>) {
    let mut handle_guard = state.health_poller_handle.lock().await;
    if let Some(handle) = handle_guard.take() {
        handle.abort();
        info!("Health poller stopped");
    }
}

// ============================================================================
// Custody Endpoints
// ============================================================================

/// GET /custody/chain
async fn handle_custody_chain(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let custody = state.custody_service.lock().unwrap();
    match custody.get_chain() {
        Some(chain) => (StatusCode::OK, Json(json!(chain))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No custody chain initialized" })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct CustodyEnvelopesQuery {
    pub receipt_type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<u32>,
}

/// GET /custody/envelopes
async fn handle_custody_envelopes(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<CustodyEnvelopesQuery>,
) -> Json<Value> {
    let custody = state.custody_service.lock().unwrap();
    let envelopes = match (&query.receipt_type, &query.from, &query.to) {
        (Some(rt), _, _) => custody.get_envelopes_by_type(rt),
        (_, Some(from), Some(to)) => {
            custody.get_envelopes_by_time_range(Some(from.as_str()), Some(to.as_str()))
        }
        (_, Some(from), None) => custody.get_envelopes_by_time_range(Some(from.as_str()), None),
        (_, None, Some(to)) => custody.get_envelopes_by_time_range(None, Some(to.as_str())),
        (None, None, None) => custody.get_envelopes_by_time_range(None, None),
    };
    let envelopes: Vec<ReceiptEnvelope> = match query.limit {
        Some(limit) => envelopes.into_iter().take(limit as usize).collect(),
        None => envelopes,
    };
    Json(json!({ "envelopes": envelopes, "count": envelopes.len() }))
}

/// GET /custody/envelope/{envelope_id}
async fn handle_custody_envelope(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(envelope_id): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    let custody = state.custody_service.lock().unwrap();
    match custody.get_envelope(&envelope_id) {
        Some(envelope) => (StatusCode::OK, Json(json!(envelope))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Envelope {} not found", envelope_id) })),
        ),
    }
}

/// GET /custody/provenance
async fn handle_custody_provenance(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<ProvenanceQuery>,
) -> Json<Value> {
    let custody = state.custody_service.lock().unwrap();
    let results = custody.query_provenance(&query);
    Json(json!({ "results": results, "count": results.len() }))
}

/// GET /custody/provenance/graph
async fn handle_custody_provenance_graph(State(state): State<Arc<AppState>>) -> Json<Value> {
    let custody = state.custody_service.lock().unwrap();
    let graph = custody.get_provenance_graph();
    Json(json!(graph))
}

/// POST /custody/verify
async fn handle_custody_verify(State(state): State<Arc<AppState>>) -> Json<Value> {
    let custody = state.custody_service.lock().unwrap();
    let report = custody.verify_integrity();
    Json(json!(report))
}

/// POST /custody/retention
async fn handle_custody_retention(
    State(state): State<Arc<AppState>>,
    axum::Json(policy): axum::Json<RetentionPolicy>,
) -> Json<Value> {
    let mut custody = state.custody_service.lock().unwrap();
    let result = custody.apply_retention(&policy);
    Json(json!(result))
}

// ============================================================================
// Operations Endpoints
// ============================================================================

/// GET /ops/health — per-component health check
async fn handle_ops_health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let node_id = state.node_identity_service.get_identity().node_id.clone();
    let reg_status = { let r = state.registration_service.lock().await; r.get_record().registration_status.clone() };
    let manifest = {
        let b = state.capability_evidence_bridge.lock().await;
        crate::node::capabilities::detect_capabilities(&state.db, &node_id, Some(&b), None)
    };
    let session_count = { let s = state.session_service.lock().await; s.list_sessions(None).len() };
    let bootstrap_done = { let b = state.bootstrap_service.lock().await; b.get_receipts().len() > 0 };
    let (chain_exists, integrity_ok) = {
        let c = state.custody_service.lock().unwrap();
        let chain = c.get_chain();
        let exists = chain.is_some();
        let ok = if exists { c.verify_integrity().verified } else { false };
        (exists, ok)
    };
    let core_online = { let c = state.core_integration_service.lock().await; c.is_online() };

    Json(serde_json::json!({
        "overall_status": "healthy",
        "components": [
            {"component": "identity", "status": "healthy", "details": null},
            {"component": "registration", "status": if reg_status == "registered" || reg_status == "registration_requested" { "healthy" } else { "degraded" }, "details": format!("status: {}", reg_status)},
            {"component": "capabilities", "status": if !manifest.capabilities.is_empty() { "healthy" } else { "degraded" }, "details": format!("{} capabilities", manifest.capabilities.len())},
            {"component": "sessions", "status": "healthy", "details": format!("{} total sessions", session_count)},
            {"component": "bootstrap", "status": if bootstrap_done { "healthy" } else { "degraded" }, "details": if bootstrap_done { "Bootstrap completed" } else { "Bootstrap not yet completed" }},
            {"component": "custody", "status": if chain_exists && integrity_ok { "healthy" } else if chain_exists { "degraded" } else { "not_available" }, "details": format!("chain_exists: {}, integrity_verified: {}", chain_exists, integrity_ok)},
            {"component": "core_integration", "status": if core_online { "healthy" } else { "not_available" }, "details": if core_online { "Core endpoint configured" } else { "No Core endpoint configured (offline mode)" }}
        ],
        "checked_at": chrono::Utc::now().to_rfc3339()
    }))
}

/// GET /ops/overview — complete single-endpoint node state summary
async fn handle_ops_overview(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let identity = state.node_identity_service.get_identity().clone();
    let reg_record;
    let manifest;
    let sessions;
    let bootstrap_done;
    let env_count;
    let core_online;
    let last_sync_at;
    {
        let r = state.registration_service.lock().await;
        reg_record = r.get_record().clone();
    }
    {
        let b = state.capability_evidence_bridge.lock().await;
        manifest = crate::node::capabilities::detect_capabilities(&state.db, &identity.node_id, Some(&b), None);
    }
    {
        let s = state.session_service.lock().await;
        sessions = s.list_sessions(None);
    }
    {
        let b = state.bootstrap_service.lock().await;
        bootstrap_done = b.get_receipts().len() > 0;
    }
    {
        let c = state.custody_service.lock().unwrap();
        let chain = c.get_chain();
        env_count = chain.as_ref().map(|ch| ch.envelope_count).unwrap_or(0);
    }
    {
        let core = state.core_integration_service.lock().await;
        core_online = core.is_online();
        last_sync_at = core.get_last_sync_at();
    }

    let capability_count = manifest.capabilities.len() as u32;
    let verified_count = manifest.capabilities.iter().filter(|c| c.verification_status.as_deref() == Some("verified")).count() as u32;
    let active_count = sessions.iter().filter(|s| s.state == "active").count() as u32;
    let registered = reg_record.registration_status == "registered" || reg_record.registration_status == "registration_requested";

    Json(serde_json::json!({
        "node_id": identity.node_id,
        "display_name": identity.display_name,
        "status": if registered { "online" } else { "offline" },
        "uptime_seconds": state.start_time.elapsed().as_secs(),
        "state": reg_record.registration_status,
        "registered": registered,
        "session_count": sessions.len() as u32,
        "active_session_count": active_count,
        "capability_count": capability_count,
        "verified_capability_count": verified_count,
        "bootstrap_completed": bootstrap_done,
        "custody_envelope_count": env_count,
        "core_connected": core_online,
        "last_sync_at": last_sync_at,
        "observed_at": chrono::Utc::now().to_rfc3339()
    }))
}

/// GET /ops/diagnostics — comprehensive system diagnostics
async fn handle_ops_diagnostics(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let health_val = handle_ops_health(State(state.clone())).await.0;
    let overview_val = handle_ops_overview(State(state.clone())).await.0;

    let session_summary;
    {
        let s = state.session_service.lock().await;
        let sessions = s.list_sessions(None);
        session_summary = serde_json::json!({
            "total_sessions": sessions.len(),
            "active_sessions": sessions.iter().filter(|sess| sess.state == "active").count(),
            "closed_sessions": sessions.iter().filter(|sess| sess.state == "closed").count(),
            "oldest_active": sessions.iter().filter(|sess| sess.state == "active").min_by_key(|sess| &sess.started_at).map(|sess| sess.started_at.clone()),
            "latest_closed": sessions.iter().filter(|sess| sess.state == "closed").max_by_key(|sess| sess.closed_at.as_deref().unwrap_or("")).and_then(|sess| sess.closed_at.clone())
        });
    }

    let custody_summary;
    {
        let c = state.custody_service.lock().unwrap();
        let chain = c.get_chain();
        let total_envelopes = chain.as_ref().map(|ch| ch.envelope_count).unwrap_or(0);
        custody_summary = serde_json::json!({
            "total_envelopes": total_envelopes,
            "integrity_verified": if total_envelopes > 0 { c.verify_integrity().verified } else { true },
            "first_envelope_at": c.get_envelopes_by_time_range(None, None).first().map(|e| e.timestamp.clone()),
            "latest_envelope_at": c.get_envelopes_by_time_range(None, None).last().map(|e| e.timestamp.clone())
        });
    }

    Json(serde_json::json!({
        "report_id": uuid::Uuid::new_v4().to_string(),
        "requested_at": chrono::Utc::now().to_rfc3339(),
        "health": health_val,
        "overview": overview_val,
        "sessions": session_summary,
        "custody": custody_summary
    }))
}

/// GET /ops/health/summary — concise health counts
async fn handle_ops_health_summary(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let health = handle_ops_health(State(state.clone())).await;
    let components = health["components"].as_array().cloned().unwrap_or_default();
    let mut healthy = 0u32;
    let mut degraded = 0u32;
    let mut unhealthy = 0u32;
    for c in &components {
        match c["status"].as_str() {
            Some("healthy") => healthy += 1,
            Some("degraded") | Some("not_available") => degraded += 1,
            _ => unhealthy += 1,
        }
    }
    Json(serde_json::json!({
        "status": health["overall_status"],
        "healthy_count": healthy,
        "degraded_count": degraded,
        "unhealthy_count": unhealthy,
        "total_components": components.len() as u32
    }))
}

/// GET /ops/status — simple text/plain health indicator for load balancers
async fn handle_ops_status(State(state): State<Arc<AppState>>) -> (axum::http::StatusCode, String) {
    let health = handle_ops_health(State(state.clone())).await;
    let overall = health["overall_status"].as_str().unwrap_or("unhealthy").to_string();
    let status_code = match overall.as_str() {
        "healthy" => axum::http::StatusCode::OK,
        _ => axum::http::StatusCode::SERVICE_UNAVAILABLE,
    };
    (status_code, overall)
}

// ============================================================================
// Operator Surface Handlers
// ============================================================================

/// GET /operator/state — returns current operator state as JSON.
async fn handle_operator_state(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let backends = state.backends.lock().await;
    let operator = state.operator.lock().await;
    let snapshot = operator.snapshot(&backends, &state.supervisor, &state.db);
    Json(serde_json::to_value(&snapshot).unwrap_or_default())
}

/// GET /operator/events — returns recent operator events.
async fn handle_operator_events(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let operator = state.operator.lock().await;
    let events: Vec<&crate::operator::OperatorEvent> = operator.events.recent(50);
    Json(serde_json::json!({ "events": events }))
}

/// GET /operator/dashboard — returns the operator dashboard HTML page.
async fn handle_operator_dashboard() -> (axum::http::StatusCode, [(&'static str, &'static str); 2], String) {
    let app_js = include_str!("../runtime-ui/js/app.js");
    let tokens_css = include_str!("../runtime-ui/styles/tokens.css");
    let librarian_css = include_str!("../runtime-ui/styles/librarian.css");
    let version = env!("CARGO_PKG_VERSION");

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Librarian Runtime Dashboard</title>
<style>{tokens}</style>
<style>{librarian}</style>
</head>
<body>
<div class="lr-shell">
<header class="lr-header">
<div class="lr-header-title">
<svg class="lr-logo" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="4"/><path d="M8 12h8M12 8v8" stroke-linecap="round"/></svg>
Librarian Runtime
</div>
<div class="lr-header-info"><span id="lr-header-meta">Loading...</span> &middot; <span id="lr-header-version">{version}</span></div>
</header>
<nav class="lr-tabs"><button class="lr-tab active" data-tab="overview">Overview</button><button class="lr-tab" data-tab="intelligence">Intelligence</button><button class="lr-tab" data-tab="operations">Operations</button><button class="lr-tab" data-tab="governance">Governance</button></nav>
<div id="view-overview" class="lr-view active"></div>
<div id="view-intelligence" class="lr-view"></div>
<div id="view-operations" class="lr-view"></div>
<div id="view-governance" class="lr-view"></div>
</div>
<script>{app_js}</script>
</body>
</html>"##,
        tokens = tokens_css,
        librarian = librarian_css,
        app_js = app_js,
        version = version,
    );
    (axum::http::StatusCode::OK, [("content-type", "text/html; charset=utf-8"), ("cache-control", "no-cache")], html)
}

/// GET /operator/dashboard/data — aggregated dashboard data from all Phase 1 + Phase 2 domains.
async fn handle_operator_dashboard_data(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let identity = state.node_identity_service.get_identity().clone();

    let node_state_val = state.node_state.lock().await;
    let node_status = json!({
        "state": node_state_val.current().as_str(),
        "uptime_seconds": state.start_time.elapsed().as_secs(),
        "last_state_change": node_state_val.last_change().to_string(),
    });
    drop(node_state_val);

    let registration_status = {
        let r = state.registration_service.lock().await;
        r.get_record().registration_status.clone()
    };

    let (manifest, capability_count, verified_count) = {
        let b = state.capability_evidence_bridge.lock().await;
        let mrs = state.model_runtime_service.lock().await;
        let m = crate::node::capabilities::detect_capabilities(&state.db, &identity.node_id, Some(&b), Some(&mrs));
        let total = m.capabilities.len() as u32;
        let verified = m.capabilities.iter().filter(|c| c.verification_status.as_deref() == Some("verified")).count() as u32;
        (serde_json::to_value(&m).ok(), total, verified)
    };

    let (session_total, session_active, sessions_json) = {
        let s = state.session_service.lock().await;
        let sessions = s.list_sessions(None);
        let total = sessions.len() as u32;
        let active = sessions.iter().filter(|s| s.state == "active").count() as u32;
        (total, active, serde_json::to_value(&sessions).ok())
    };

    let bootstrap_done = {
        let b = state.bootstrap_service.lock().await;
        b.get_receipts().len() > 0
    };

    let workload_inventory = {
        let ws = state.workload_session_service.lock().await;
        serde_json::to_value(&WorkloadLifecycleService::get_inventory(&ws)).ok()
    };

    let (findings_list, findings_summary, total_findings) = {
        let svc = state.evidence_classification_service.lock().await;
        let list = svc.get_findings(None, None);
        let summary = svc.get_findings_summary();
        let count = list.len() as u32;
        (serde_json::to_value(&list).ok(), serde_json::to_value(&summary).ok(), count)
    };

    let (anomaly_findings, anomalies_count) = {
        let ws_for_anomaly = state.workload_session_service.lock().await;
        let svc = state.anomaly_detection_service.lock().await;
        let findings = svc.scan_all_metrics(&ws_for_anomaly);
        let count = findings.len() as u32;
        (serde_json::to_value(&findings).ok(), count)
    };

    let (patterns, pattern_summary) = {
        let svc = state.pattern_escalation_service.lock().await;
        let pats = svc.get_patterns(None, None);
        (serde_json::to_value(&pats).ok(), serde_json::to_value(&svc.get_summary()).ok())
    };

    let report_available = findings_list.is_some() || anomaly_findings.is_some()
        || patterns.is_some() || workload_inventory.is_some();

    let pending_decisions = {
        let bootstrap = state.bootstrap_service.lock().await;
        let reg = state.registration_service.lock().await;
        let bridge = state.capability_evidence_bridge.lock().await;
        let ow = state.owner_workflow_service.lock().await;
        serde_json::to_value(&ow.get_pending_approvals(&bootstrap, &reg, &bridge)).ok()
    };

    let fleet = {
        let f = state.fleet_service.lock().await;
        let inventory = f.get_inventory();
        let node_count = inventory.nodes.len() as u32;
        (serde_json::to_value(&inventory).ok(), node_count)
    };

    let fleet_trust = {
        let t = state.fleet_trust_service.lock().await;
        serde_json::to_value(t.get_all_trust_states()).ok()
    };

    let reconciliation_receipts = {
        let r = state.reconciliation_service.lock().await;
        r.get_reconciliation_history().len() as u32
    };

    let recovery = {
        let r = state.recovery_custody_service.lock().unwrap();
        serde_json::to_value(&r.get_status()).ok()
    };
    let recovery_active = recovery.as_ref().and_then(|v| v.as_object()).is_some();

    let (custody_env_count, custody_integrity) = {
        let c = state.custody_service.lock().unwrap();
        let chain = c.get_chain();
        let count = chain.as_ref().map(|ch| ch.envelope_count).unwrap_or(0);
        let integrity = if count > 0 { c.verify_integrity().verified } else { true };
        (count, integrity)
    };

    let (core_online, last_sync_at) = {
        let core = state.core_integration_service.lock().await;
        (core.is_online(), core.get_last_sync_at())
    };

    let health_components = {
        let reg_status = registration_status.clone();
        serde_json::json!([
            {"component": "identity", "status": "healthy", "details": null},
            {"component": "registration", "status": if reg_status == "registered" || reg_status == "registration_requested" { "healthy" } else { "degraded" }, "details": format!("status: {}", reg_status)},
            {"component": "capabilities", "status": if capability_count > 0 { "healthy" } else { "degraded" }, "details": format!("{} capabilities", capability_count)},
            {"component": "sessions", "status": "healthy", "details": format!("{} total sessions", session_total)},
            {"component": "bootstrap", "status": if bootstrap_done { "healthy" } else { "degraded" }, "details": if bootstrap_done { "Bootstrap completed" } else { "Bootstrap not yet completed" }},
            {"component": "custody", "status": if custody_env_count > 0 && custody_integrity { "healthy" } else if custody_env_count > 0 { "degraded" } else { "not_available" }, "details": format!("chain_exists: {}, integrity_verified: {}", custody_env_count > 0, custody_integrity)},
            {"component": "core_integration", "status": if core_online { "healthy" } else { "not_available" }, "details": if core_online { "Core endpoint configured" } else { "No Core endpoint configured (offline mode)" }}
        ])
    };

    let registered = registration_status == "registered" || registration_status == "registration_requested";

    // Build insight-to-decision map: for each pending decision, attach the triggering finding info
    let decisions_with_insight = pending_decisions.as_ref().and_then(|pd| {
        let items = pd.get("items")?.as_array()?;
        let findings = findings_list.as_ref()?;
        let findings_arr = findings.as_array()?;
        let enriched: Vec<serde_json::Value> = items.iter().map(|item| {
            let item_id = item.get("item_id").and_then(|v| v.as_str()).unwrap_or("");
            let item_type = item.get("item_type").and_then(|v| v.as_str()).unwrap_or("");
            // Find matching finding by item_id or item_type
            let trigger = findings_arr.iter().find(|f| {
                f.get("finding_id").and_then(|v| v.as_str()) == Some(item_id)
                    || f.get("category").and_then(|v| v.as_str()) == Some(item_type)
                    || f.get("affected_entity_id").and_then(|v| v.as_str()) == Some(item_id)
            });
            let trigger_info = trigger.map(|f| json!({
                "finding_id": f.get("finding_id"),
                "title": f.get("title"),
                "category": f.get("category"),
                "severity": f.get("severity"),
                "confidence": f.get("confidence"),
                "detection_method": f.get("detection_method"),
                "evidence_references": f.get("evidence_references"),
                "description": f.get("description"),
            }));
            let mut enriched = item.clone();
            if let Some(ti) = trigger_info {
                enriched["triggering_insight"] = ti;
            }
            enriched
        }).collect();
        Some(serde_json::to_value(&enriched).ok())
    }).flatten();

    let (policy_config, policy_receipts) = {
        let svc = state.policy_service.lock().await;
        (serde_json::to_value(svc.get_policies()).ok(), serde_json::to_value(svc.get_receipts()).ok())
    };

    let capability_lifecycle = {
        let bridge = state.capability_evidence_bridge.lock().await;
        bridge.get_capability_lifecycle()
    };

    Json(serde_json::json!({
        "node": {
            "identity": identity,
            "status": node_status,
            "overview": {
                "node_id": identity.node_id,
                "display_name": identity.display_name,
                "status": if registered { "online" } else { "offline" },
                "uptime_seconds": node_status["uptime_seconds"],
                "state": registration_status,
                "registered": registered,
                "session_count": session_total,
                "active_session_count": session_active,
                "capability_count": capability_count,
                "verified_capability_count": verified_count,
                "bootstrap_completed": bootstrap_done,
                "custody_envelope_count": custody_env_count,
                "core_connected": core_online,
                "last_sync_at": last_sync_at
            }
        },
        "health": {
            "overall_status": if registered && capability_count > 0 { "healthy" } else { "degraded" },
            "components": health_components,
            "checked_at": chrono::Utc::now().to_rfc3339()
        },
        "sessions": {
            "total": session_total,
            "active": session_active,
            "sessions": sessions_json
        },
        "workloads": workload_inventory,
        "capabilities": {
            "total": capability_count,
            "verified": verified_count,
            "manifest": manifest
        },
        "intelligence": {
            "findings_summary": findings_summary,
            "findings_count": total_findings,
            "active_anomalies": anomalies_count,
            "pattern_summary": pattern_summary
        },
        "insight_findings": findings_list,
        "insight_anomalies": anomaly_findings,
        "insight_patterns": patterns,
        "insight_report_available": report_available,
        "pending_decisions_with_insight": decisions_with_insight,
        "pending_decisions": pending_decisions,
        "fleet": {
            "node_count": fleet.1,
            "inventory": fleet.0,
            "trust": fleet_trust
        },
        "reconciliation": {
            "total_receipts": reconciliation_receipts
        },
        "recovery": {
            "active": recovery_active,
            "status": recovery
        },
        "custody": {
            "envelope_count": custody_env_count,
            "integrity_verified": custody_integrity
        },
        "policy": {
            "config": policy_config,
            "receipts": policy_receipts
        },
        "capability_lifecycle": capability_lifecycle,
        "owner_history": null,
        "observed_at": chrono::Utc::now().to_rfc3339()
    }))
}


// ============================================================================
// Owner Workflow Request Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OwnerReviewRequest {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct OwnerDecideRequest {
    pub decision_id: String,
    pub item_id: String,
    pub item_type: String,
    pub session_id: String,
    pub decision: String,
    pub reason: Option<String>,
    pub decided_at: String,
    pub owner_identity: Option<String>,
}

// ============================================================================
// Owner Workflow Endpoints
// ============================================================================

/// POST /owner/review/node
async fn handle_owner_review_node(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OwnerReviewRequest>,
) -> Json<librarian_contracts::owner_workflows::ReviewResult> {
    let reg = state.registration_service.lock().await;
    let bridge = state.capability_evidence_bridge.lock().await;
    let session = state.session_service.lock().await;
    let bootstrap = state.bootstrap_service.lock().await;
    let ow = state.owner_workflow_service.lock().await;
    let result = ow.review_node_state(
        &body.session_id,
        &state.node_identity_service,
        &*reg,
        &*bridge,
        &*session,
        &*bootstrap,
        &state.db,
    );
    Json(result)
}

/// POST /owner/review/capabilities
async fn handle_owner_review_capabilities(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OwnerReviewRequest>,
) -> Json<librarian_contracts::owner_workflows::ReviewResult> {
    let bridge = state.capability_evidence_bridge.lock().await;
    let ow = state.owner_workflow_service.lock().await;
    let result = ow.review_capabilities(
        &body.session_id,
        &state.node_identity_service,
        &*bridge,
        &state.db,
    );
    Json(result)
}

/// POST /owner/review/sessions
async fn handle_owner_review_sessions(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OwnerReviewRequest>,
) -> Json<librarian_contracts::owner_workflows::ReviewResult> {
    let session = state.session_service.lock().await;
    let ow = state.owner_workflow_service.lock().await;
    let result = ow.review_sessions(
        &body.session_id,
        &*session,
    );
    Json(result)
}

/// POST /owner/review/custody
async fn handle_owner_review_custody(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OwnerReviewRequest>,
) -> Json<librarian_contracts::owner_workflows::ReviewResult> {
    let ow = state.owner_workflow_service.lock().await;
    let custody = state.custody_service.lock().unwrap();
    let result = ow.review_custody(
        &body.session_id,
        &*custody,
    );
    Json(result)
}

/// POST /owner/review/bootstrap
async fn handle_owner_review_bootstrap(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OwnerReviewRequest>,
) -> Json<librarian_contracts::owner_workflows::ReviewResult> {
    let bootstrap = state.bootstrap_service.lock().await;
    let ow = state.owner_workflow_service.lock().await;
    let result = ow.review_bootstrap_history(
        &body.session_id,
        &*bootstrap,
    );
    Json(result)
}

/// GET /owner/pending
async fn handle_owner_pending(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::owner_workflows::PendingApprovalsSummary> {
    let bootstrap = state.bootstrap_service.lock().await;
    let reg = state.registration_service.lock().await;
    let bridge = state.capability_evidence_bridge.lock().await;
    let ow = state.owner_workflow_service.lock().await;
    let result = ow.get_pending_approvals(
        &*bootstrap,
        &*reg,
        &*bridge,
    );
    Json(result)
}

/// POST /owner/decide
async fn handle_owner_decide(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<OwnerDecideRequest>,
) -> Json<librarian_contracts::owner_workflows::DecisionReceipt> {
    let decision = librarian_contracts::owner_workflows::OwnerDecision {
        decision_id: body.decision_id,
        item_id: body.item_id,
        item_type: body.item_type,
        session_id: body.session_id,
        decision: body.decision,
        reason: body.reason,
        decided_at: body.decided_at,
        owner_identity: body.owner_identity,
    };
    let mut ow = state.owner_workflow_service.lock().await;
    let mut bootstrap = state.bootstrap_service.lock().await;
    let mut reg = state.registration_service.lock().await;
    let receipt = ow.submit_decision(
        decision,
        &mut *bootstrap,
        &mut *reg,
    );
    Json(receipt)
}

/// GET /owner/history
async fn handle_owner_history(
    State(state): State<Arc<AppState>>,
) -> Json<librarian_contracts::owner_workflows::OwnerActionHistory> {
    let ow = state.owner_workflow_service.lock().await;
    let history = ow.get_action_history(&state.node_identity_service);
    Json(history)
}
