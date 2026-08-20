//! # Capability Registry — Search/List Contracts
//!
//! Request/response types for `capability_registry_search` and
//! `capability_registry_list` MCP tools.
//!
//! **Security constraint (CR-R-001):** Responses never contain instruction bodies.

use serde::{Deserialize, Serialize};
use crate::capability_registry::types::CapabilitySummary;

// ── Search ─────────────────────────────────────────────────────────

/// Request for capability_registry_search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Keyword search against capability names, descriptions, and IDs.
    pub query: String,
    /// Optional filter by capability type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_type: Option<String>,
    /// Optional filter by lifecycle status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Maximum results (1–100, default 20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

/// Response for capability_registry_search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Response schema identifier.
    pub response_schema: String,
    /// Total number of results returned.
    pub total_results: usize,
    /// Capability metadata results.
    pub results: Vec<CapabilitySummary>,
}

// ── List ───────────────────────────────────────────────────────────

/// Request for capability_registry_list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRequest {
    /// Optional filter by capability type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_type: Option<String>,
    /// Optional filter by lifecycle status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Optional filter by source type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
}

/// Response for capability_registry_list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    /// Response schema identifier.
    pub response_schema: String,
    /// Total number of results returned.
    pub total_results: usize,
    /// Capability metadata results.
    pub results: Vec<CapabilitySummary>,
}
