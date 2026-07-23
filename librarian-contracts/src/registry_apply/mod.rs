use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeStatus {
    Proposed,
    Approved,
    Applied,
    Verified,
    Rejected,
    Failed,
}

impl std::fmt::Display for ChangeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeStatus::Proposed => write!(f, "proposed"),
            ChangeStatus::Approved => write!(f, "approved"),
            ChangeStatus::Applied => write!(f, "applied"),
            ChangeStatus::Verified => write!(f, "verified"),
            ChangeStatus::Rejected => write!(f, "rejected"),
            ChangeStatus::Failed => write!(f, "failed"),
        }
    }
}

impl From<&str> for ChangeStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "proposed" => ChangeStatus::Proposed,
            "approved" => ChangeStatus::Approved,
            "applied" => ChangeStatus::Applied,
            "verified" => ChangeStatus::Verified,
            "rejected" => ChangeStatus::Rejected,
            "failed" => ChangeStatus::Failed,
            _ => ChangeStatus::Proposed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransitionType {
    Proposed,
    Approved,
    Applied,
    Verified,
}

impl std::fmt::Display for TransitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionType::Proposed => write!(f, "proposed"),
            TransitionType::Approved => write!(f, "approved"),
            TransitionType::Applied => write!(f, "applied"),
            TransitionType::Verified => write!(f, "verified"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistryStateChange {
    pub change_id: String,
    pub target_type: String,
    pub target_id: String,
    pub proposed_state: serde_json::Value,
    pub approved_state: Option<serde_json::Value>,
    pub applied_state: Option<serde_json::Value>,
    pub status: ChangeStatus,
    pub receipts: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateChangeReceipt {
    pub receipt_id: String,
    pub change_id: String,
    pub transition: TransitionType,
    pub previous_status: String,
    pub new_status: String,
    pub triggered_by: String,
    pub timestamp: String,
}
