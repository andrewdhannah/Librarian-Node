use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadAllocationLink {
    pub workload_id: String,
    pub allocation_recommendation_id: String,
    pub allocation_decision_id: String,
    pub allocation_receipt_id: String,
    pub session_id: Option<String>,
    pub linked_at: String,
}
