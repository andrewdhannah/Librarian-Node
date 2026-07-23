use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryAction {
    pub action_id: String,
    pub recovery_id: String,
    pub action_type: String,
    pub affected_differences: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryActionReceipt {
    pub receipt_id: String,
    pub action_id: String,
    pub recovery_id: String,
    pub action_type: String,
    pub previous_state: String,
    pub new_state: String,
    pub affected_differences: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub timestamp: String,
    pub custody_envelope_id: Option<String>,
}
