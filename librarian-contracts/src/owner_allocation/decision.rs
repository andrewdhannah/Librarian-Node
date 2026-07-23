use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationDecision {
    pub decision_id: String,
    pub recommendation_id: String,
    pub session_id: String,
    pub decision: String,
    pub alternative_node_id: Option<String>,
    pub reason: Option<String>,
    pub decided_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationDecisionReceipt {
    pub receipt_id: String,
    pub decision_id: String,
    pub recommendation_id: String,
    pub decision: String,
    pub workload_description: String,
    pub selected_node_id: Option<String>,
    pub decided_at: String,
    pub session_id: String,
    pub custody_envelope_id: Option<String>,
}
