use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationDecision {
    pub decision_id: String,
    pub reconciliation_id: String,
    pub difference_id: String,
    pub node_id: String,
    pub decision: String,
    pub reason: Option<String>,
    pub decided_at: String,
    pub actor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationReceipt {
    pub receipt_id: String,
    pub reconciliation_id: String,
    pub node_id: String,
    pub receipt_type: String,
    pub previous_phase: Option<String>,
    pub new_phase: Option<String>,
    pub decision_id: Option<String>,
    pub difference_ids: Vec<String>,
    pub payload: serde_json::Value,
    pub generated_at: String,
}
