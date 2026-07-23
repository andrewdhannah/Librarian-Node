use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnforcementEvent {
    pub event_id: String,
    pub rule_id: String,
    pub scope: String,
    pub target_id: String,
    pub violation_detail: String,
    pub action_taken: String,
    pub timestamp: String,
}
