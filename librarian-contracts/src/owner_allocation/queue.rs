use serde::{Deserialize, Serialize};

use super::review::AllocationRecommendationSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingAllocationQueue {
    pub total_pending: u32,
    pub items: Vec<AllocationRecommendationSummary>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationActionReceipt {
    pub receipt_id: String,
    pub decision_id: String,
    pub recommendation_id: String,
    pub action: String,
    pub session_id: Option<String>,
    pub node_id: String,
    pub acted_at: String,
}
