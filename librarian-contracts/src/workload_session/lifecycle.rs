use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkloadSessionState {
    Pending,
    Active,
    Completed,
    Failed,
    Cancelled,
}

impl WorkloadSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkloadSessionState::Pending => "pending",
            WorkloadSessionState::Active => "active",
            WorkloadSessionState::Completed => "completed",
            WorkloadSessionState::Failed => "failed",
            WorkloadSessionState::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(WorkloadSessionState::Pending),
            "active" => Some(WorkloadSessionState::Active),
            "completed" => Some(WorkloadSessionState::Completed),
            "failed" => Some(WorkloadSessionState::Failed),
            "cancelled" => Some(WorkloadSessionState::Cancelled),
            _ => None,
        }
    }
}
