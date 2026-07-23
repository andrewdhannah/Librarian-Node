use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OwnerActionType {
    ApproveCandidate,
    RejectCandidate,
    SuspendNode,
    ReinstateNode,
    ExpireEvidence,
    OverrideEnforcement,
}

impl std::fmt::Display for OwnerActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnerActionType::ApproveCandidate => write!(f, "approve_candidate"),
            OwnerActionType::RejectCandidate => write!(f, "reject_candidate"),
            OwnerActionType::SuspendNode => write!(f, "suspend_node"),
            OwnerActionType::ReinstateNode => write!(f, "reinstate_node"),
            OwnerActionType::ExpireEvidence => write!(f, "expire_evidence"),
            OwnerActionType::OverrideEnforcement => write!(f, "override_enforcement"),
        }
    }
}

impl From<&str> for OwnerActionType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "approve_candidate" => OwnerActionType::ApproveCandidate,
            "reject_candidate" => OwnerActionType::RejectCandidate,
            "suspend_node" => OwnerActionType::SuspendNode,
            "reinstate_node" => OwnerActionType::ReinstateNode,
            "expire_evidence" => OwnerActionType::ExpireEvidence,
            "override_enforcement" => OwnerActionType::OverrideEnforcement,
            _ => OwnerActionType::OverrideEnforcement,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OwnerActionStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

impl std::fmt::Display for OwnerActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnerActionStatus::Pending => write!(f, "pending"),
            OwnerActionStatus::Approved => write!(f, "approved"),
            OwnerActionStatus::Rejected => write!(f, "rejected"),
            OwnerActionStatus::Executed => write!(f, "executed"),
        }
    }
}

impl From<&str> for OwnerActionStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => OwnerActionStatus::Pending,
            "approved" => OwnerActionStatus::Approved,
            "rejected" => OwnerActionStatus::Rejected,
            "executed" => OwnerActionStatus::Executed,
            _ => OwnerActionStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistryOwnerAction {
    pub action_id: String,
    pub action_type: OwnerActionType,
    pub target_id: String,
    pub target_type: String,
    pub owner: String,
    pub reason: String,
    pub status: OwnerActionStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistryOwnerActionReceipt {
    pub receipt_id: String,
    pub action_id: String,
    pub action_type: OwnerActionType,
    pub target_id: String,
    pub previous_state: String,
    pub new_state: String,
    pub owner: String,
    pub reason: String,
    pub timestamp: String,
    pub custody_envelope_id: Option<String>,
}
