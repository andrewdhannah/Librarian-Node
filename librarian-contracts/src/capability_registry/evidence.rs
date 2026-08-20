//! # Capability Registry — Evidence Query Contracts
//!
//! Request/response types for capability evidence MCP tools:
//!   - `capability_evidence_query`
//!   - `capability_evidence_task_history`
//!   - `capability_evidence_agent_usage`
//!   - `capability_evidence_revoke_impact`
//!
//! **Contract invariant (CR-E-005):** Historical evidence is never invalidated
//! by status changes. Responses always report historical usage alongside
//! current status.

use serde::{Deserialize, Serialize};

// ── Evidence Event ─────────────────────────────────────────────────

/// A single capability evidence event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEvidenceEvent {
    /// Database ID of the event.
    pub id: Option<i64>,
    /// Event type: "CAPABILITY_RESOLVED" or "CAPABILITY_LOADED".
    pub event_type: String,
    /// Capability identifier.
    pub capability_id: String,
    /// Version number.
    pub version: i32,
    /// SHA-256 content hash.
    pub content_hash: String,
    /// Agent identity.
    pub agent_identity: Option<String>,
    /// Task identifier.
    pub task_id: Option<String>,
    /// Resolution outcome (for resolved events).
    pub resolution: Option<String>,
    /// Reason for the event.
    pub reason: Option<String>,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

// ── Query ──────────────────────────────────────────────────────────

/// Request for capability_evidence_query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceQueryRequest {
    /// Filter by capability ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    /// Filter by event type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    /// Filter by agent identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_identity: Option<String>,
    /// Filter by task ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Filter by version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
    /// Maximum results (default 50, max 200).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

/// Response for capability_evidence_query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceQueryResponse {
    pub response_schema: String,
    pub total_events: usize,
    pub events: Vec<CapabilityEvidenceEvent>,
}

// ── Task History ───────────────────────────────────────────────────

/// Request for capability_evidence_task_history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryRequest {
    /// Task identifier to look up.
    pub task_id: String,
}

/// Response for capability_evidence_task_history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryResponse {
    pub response_schema: String,
    pub task_id: String,
    pub total_capabilities: usize,
    pub events: Vec<CapabilityEvidenceEvent>,
}

// ── Agent Usage ────────────────────────────────────────────────────

/// Request for capability_evidence_agent_usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsageRequest {
    /// Agent identity to look up.
    pub agent_identity: String,
    /// If true, return summary only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<bool>,
}

/// Summary of capability usage by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsageSummary {
    pub agent_identity: String,
    pub total_events: usize,
    pub capabilities_used: Vec<String>,
    pub last_event: String,
}

/// Response for capability_evidence_agent_usage (detailed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsageResponse {
    pub response_schema: String,
    pub agent_identity: String,
    pub total_events: usize,
    pub events: Vec<CapabilityEvidenceEvent>,
}

/// Response for capability_evidence_agent_usage (summary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSummaryResponse {
    pub response_schema: String,
    pub summary: AgentUsageSummary,
}

// ── Revoke Impact ──────────────────────────────────────────────────

/// Request for capability_evidence_revoke_impact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeImpactRequest {
    /// Capability identifier to analyze.
    pub capability_id: String,
    /// Version number to check.
    pub version: i32,
}

/// Revocation impact analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeImpact {
    /// Capability identifier.
    pub capability_id: String,
    /// Version number.
    pub version: i32,
    /// Current lifecycle status of the capability.
    pub current_status: String,
    /// Number of historical load events for this version.
    pub historical_events: usize,
    /// Tasks that used this capability version.
    pub affected_tasks: Vec<String>,
    /// Agents that used this capability version.
    pub affected_agents: Vec<String>,
    /// Human-readable note.
    pub note: String,
}

/// Response for capability_evidence_revoke_impact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeImpactResponse {
    pub response_schema: String,
    pub impact: RevokeImpact,
}
