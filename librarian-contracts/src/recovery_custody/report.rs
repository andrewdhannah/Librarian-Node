use serde::{Deserialize, Serialize};

use super::action::RecoveryActionReceipt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryTransition {
    pub from_state: String,
    pub to_state: String,
    pub triggered_by: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryReport {
    pub report_id: String,
    pub recovery_id: String,
    pub node_id: String,
    pub actions_taken: Vec<RecoveryActionReceipt>,
    pub state_transitions: Vec<RecoveryTransition>,
    pub summary: String,
    pub generated_at: String,
}
