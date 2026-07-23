use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CapabilityState {
    Discovered,
    PendingVerification,
    Verified,
    Active,
    Degraded,
    Retired,
    Superseded,
}

impl CapabilityState {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityState::Discovered => "discovered",
            CapabilityState::PendingVerification => "pending_verification",
            CapabilityState::Verified => "verified",
            CapabilityState::Active => "active",
            CapabilityState::Degraded => "degraded",
            CapabilityState::Retired => "retired",
            CapabilityState::Superseded => "superseded",
        }
    }

    pub fn valid_transitions(&self) -> Vec<CapabilityState> {
        match self {
            CapabilityState::Discovered => vec![CapabilityState::PendingVerification],
            CapabilityState::PendingVerification => vec![CapabilityState::Verified],
            CapabilityState::Verified => vec![
                CapabilityState::Active,
                CapabilityState::Degraded,
                CapabilityState::Retired,
                CapabilityState::Superseded,
            ],
            CapabilityState::Active => vec![CapabilityState::Degraded, CapabilityState::Retired],
            CapabilityState::Degraded => vec![CapabilityState::Verified, CapabilityState::Retired],
            CapabilityState::Retired => vec![],
            CapabilityState::Superseded => vec![],
        }
    }
}

impl Default for CapabilityState {
    fn default() -> Self {
        CapabilityState::Discovered
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityStateChangeReceipt {
    pub receipt_id: String,
    pub capability_type: String,
    pub previous_state: CapabilityState,
    pub new_state: CapabilityState,
    pub reason: String,
    pub changed_at: String,
}
