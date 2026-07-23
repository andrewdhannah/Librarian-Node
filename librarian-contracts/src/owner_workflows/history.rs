use serde::{Deserialize, Serialize};

/// OwnerActionHistory — record of all owner decisions and reviews.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnerActionHistory {
    pub node_id: String,
    pub actions: Vec<OwnerActionEntry>,
    pub total_count: u32,
}

/// OwnerActionEntry — a single owner action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnerActionEntry {
    pub action_id: String,
    pub action_type: String,
    pub item_type: String,
    pub timestamp: String,
    pub summary: String,
    pub session_id: Option<String>,
    pub receipt_id: Option<String>,
}
