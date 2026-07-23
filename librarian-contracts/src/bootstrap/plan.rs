use serde::{Deserialize, Serialize};

use super::assessment::BootstrapRecommendation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootstrapPlan {
    pub plan_id: String,
    pub node_id: String,
    pub session_id: String,
    pub created_at: String,
    pub status: String,
    pub recommendations: Vec<BootstrapRecommendation>,
    pub owner_approved: bool,
    pub approved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootstrapReceipt {
    pub receipt_id: String,
    pub plan_id: String,
    pub node_id: String,
    pub session_id: String,
    pub completed_at: String,
    pub actions_taken: u32,
    pub actions_skipped: u32,
    pub evidence_ids: Vec<String>,
    pub result: String,
}
