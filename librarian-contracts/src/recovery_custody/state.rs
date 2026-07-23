use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryState {
    Healthy,
    Suspect,
    Reconciling,
    OwnerReview,
    Recovered,
    Failed,
}

impl RecoveryState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecoveryState::Healthy => "healthy",
            RecoveryState::Suspect => "suspect",
            RecoveryState::Reconciling => "reconciling",
            RecoveryState::OwnerReview => "owner_review",
            RecoveryState::Recovered => "recovered",
            RecoveryState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryStatus {
    pub recovery_id: String,
    pub node_id: String,
    pub state: String,
    pub previous_state: Option<String>,
    pub entered_at: String,
    pub reconciliation_report_id: Option<String>,
}
