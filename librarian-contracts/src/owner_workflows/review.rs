use serde::{Deserialize, Serialize};

/// ReviewRequest — an owner requests to review a specific aspect of node state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewRequest {
    pub request_id: String,
    pub session_id: String,
    pub review_type: String,
    pub requested_at: String,
}

/// ReviewResult — the data returned for an owner review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewResult {
    pub result_id: String,
    pub request_id: String,
    pub review_type: String,
    pub summary: String,
    pub data: serde_json::Value,
    pub generated_at: String,
}

/// PendingApprovalItem — something requiring owner decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingApprovalItem {
    pub item_id: String,
    pub item_type: String,
    pub description: String,
    pub requested_at: String,
    pub session_id: String,
    pub details: serde_json::Value,
    pub impact: String,
}

/// PendingApprovalsSummary — overview of items awaiting owner decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingApprovalsSummary {
    pub total_pending: u32,
    pub items: Vec<PendingApprovalItem>,
    pub generated_at: String,
}
