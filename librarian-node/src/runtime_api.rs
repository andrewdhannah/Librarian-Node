//! Runtime API module (RUST-MIGRATION-M0B).
//!
//! Implements `RUNTIME-API-CONTRACT-001` (suite `RUNTIME-API-SCHEMA-001`):
//! read-only projections over the **sealed** [`StartupOutcome`].
//!
//! Boundary invariants (contract §1.4 / §5.3):
//! - Handlers are read-only projections; there is no write path.
//! - No request path invokes the startup engine (startup runs once, pre-bind).
//! - [`RuntimeLifecycleState`] is an observational representation derived from
//!   the sealed receipt; the API reads it and never writes it (contract §2.4).
//! - [`RuntimeApiState::from_outcome`] derives the lifecycle: only a
//!   `GOVERNED_EXECUTION` receipt yields `SERVABLE_RUNTIME` (contract §2.2).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use librarian_contracts::node::RuntimeLifecycleState;
use librarian_core::startup::StartupOutcome;
use serde::Serialize;

/// Sealed runtime observation state. Immutable after construction.
///
/// The lifecycle is **derived** from the sealed receipt at construction time;
/// there is no setter and no request path that can change it (contract §2.4).
#[derive(Clone)]
pub struct RuntimeApiState {
    outcome: Arc<StartupOutcome>,
    lifecycle: RuntimeLifecycleState,
}

impl RuntimeApiState {
    /// Seal a startup outcome and derive the lifecycle state.
    ///
    /// `GOVERNED_EXECUTION` → `SERVABLE_RUNTIME` (all §2.2 preconditions hold
    /// by construction of the receipt); any other status → `STARTUP_FAILED`.
    /// The transition itself is owned by the startup state machine — this is
    /// only the observational projection of the outcome it sealed.
    pub fn from_outcome(outcome: StartupOutcome) -> Self {
        let lifecycle = if outcome.receipt.status == "GOVERNED_EXECUTION" {
            RuntimeLifecycleState::ServableRuntime
        } else {
            RuntimeLifecycleState::StartupFailed
        };
        RuntimeApiState {
            outcome: Arc::new(outcome),
            lifecycle,
        }
    }

    /// The observed lifecycle state (read-only; never mutated by the API).
    pub fn lifecycle(&self) -> RuntimeLifecycleState {
        self.lifecycle
    }

    /// The sealed startup outcome this API projects.
    pub fn outcome(&self) -> &StartupOutcome {
        &self.outcome
    }
}

/// Contract §4.1 — `GET /health` response.
#[derive(Serialize)]
pub struct HealthResponse {
    node_id: String,
    runtime_state: String,
    health: String,
    observed_at: String,
}

/// Contract §4.2 — `GET /runtime/status` response.
///
/// Deterministic fields derive from the sealed receipt and MUST match the M0A
/// evidence receipts field-for-field; `observed_at` is the only variable field.
#[derive(Serialize)]
pub struct RuntimeStatusResponse {
    node_id: String,
    runtime_state: String,
    startup_receipt_id: String,
    governance_commit: String,
    startup_status: String,
    checks_passed: u32,
    checks_failed: u32,
    observed_at: String,
}

/// RFC 3339 UTC observation timestamp, seconds precision with `Z` designator
/// (the only variable response field; same form as the receipt timestamp).
fn observed_at() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Contract §3.1 — health is governed availability, not process liveness.
async fn get_health(
    State(state): State<Arc<RuntimeApiState>>,
) -> (StatusCode, Json<HealthResponse>) {
    let servable = state.lifecycle.is_servable();
    let status = if servable {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(HealthResponse {
            node_id: state.outcome.receipt.node_id.clone(),
            runtime_state: state.lifecycle.as_str().to_string(),
            health: if servable { "ok" } else { "unavailable" }.to_string(),
            observed_at: observed_at(),
        }),
    )
}

/// Contract §4.2 — governed availability status with provenance.
async fn get_status(
    State(state): State<Arc<RuntimeApiState>>,
) -> (StatusCode, Json<RuntimeStatusResponse>) {
    let servable = state.lifecycle.is_servable();
    let status = if servable {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let receipt = &state.outcome.receipt;
    (
        status,
        Json(RuntimeStatusResponse {
            node_id: receipt.node_id.clone(),
            runtime_state: state.lifecycle.as_str().to_string(),
            startup_receipt_id: receipt.receipt_id.clone(),
            governance_commit: receipt.governance_commit.clone(),
            startup_status: receipt.status.clone(),
            checks_passed: receipt.checks_passed,
            checks_failed: receipt.checks_failed,
            observed_at: observed_at(),
        }),
    )
}

/// Contract §4.3 — the receipt observed from the sealed startup outcome.
///
/// Served exactly as sealed: same `receipt_id`, same timestamp, same bytes.
/// No regeneration, transformation, or normalization — a regenerated receipt
/// would create a second evidence event.
async fn get_receipt(
    State(state): State<Arc<RuntimeApiState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let servable = state.lifecycle.is_servable();
    let status = if servable {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    // Serialize the sealed receipt as-is; the response body IS the receipt.
    let receipt =
        serde_json::to_value(&state.outcome.receipt).expect("sealed receipt must serialize");
    (status, Json(receipt))
}

/// Build the runtime API router (contract §3) over a sealed state.
///
/// Returns `Router<()>` so it can be merged into the serving app in `main.rs`.
/// Unknown paths → 404 and state-changing verbs → 405 with `Allow: GET` are
/// handled by axum's default routing semantics.
pub fn router(state: Arc<RuntimeApiState>) -> Router {
    Router::new()
        .route("/health", get(get_health))
        .route("/runtime/status", get(get_status))
        .route("/runtime/receipt", get(get_receipt))
        .with_state(state)
}
