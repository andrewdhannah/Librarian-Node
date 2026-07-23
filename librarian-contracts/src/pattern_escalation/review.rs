use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternReviewAction {
    pub action_id: String,
    pub pattern_id: String,
    pub action: String,
    pub note: Option<String>,
    pub acted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternReviewReceipt {
    pub receipt_id: String,
    pub action_id: String,
    pub pattern_id: String,
    pub previous_status: String,
    pub new_status: String,
    pub action: String,
    pub note: Option<String>,
    pub acted_at: String,
}
