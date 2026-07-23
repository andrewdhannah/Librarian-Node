use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeState {
    Unregistered,
    RegistrationRequested,
    Registered,
    Suspended,
    Retired,
    Connected,
    Authorized,
    Executing,
    EvidencePending,
    Reconciling,
    Failed,
}

impl NodeState {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeState::Unregistered => "unregistered",
            NodeState::RegistrationRequested => "registration_requested",
            NodeState::Registered => "registered",
            NodeState::Suspended => "suspended",
            NodeState::Retired => "retired",
            NodeState::Connected => "connected",
            NodeState::Authorized => "authorized",
            NodeState::Executing => "executing",
            NodeState::EvidencePending => "evidence_pending",
            NodeState::Reconciling => "reconciling",
            NodeState::Failed => "failed",
        }
    }
}
