use serde::{Deserialize, Serialize};

/// OwnerDecision — an owner's decision on a pending item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnerDecision {
    pub decision_id: String,
    pub item_id: String,
    pub item_type: String,
    pub session_id: String,
    pub decision: String,
    pub reason: Option<String>,
    pub decided_at: String,
    pub owner_identity: Option<String>,
}

/// DecisionReceipt — evidence that an owner decision was made.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionReceipt {
    pub receipt_id: String,
    pub decision_id: String,
    pub item_id: String,
    pub item_type: String,
    pub decision: String,
    pub decided_at: String,
    pub previous_state: Option<String>,
    pub new_state: Option<String>,
    pub custody_envelope_id: Option<String>,
}
