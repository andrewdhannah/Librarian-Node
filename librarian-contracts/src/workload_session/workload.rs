use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadDescriptor {
    pub workload_id: String,
    pub workload_type: String,
    pub description: String,
    pub requirements: Option<Vec<String>>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadSession {
    pub workload_session_id: String,
    pub workload_id: String,
    pub session_id: String,
    pub node_id: String,
    pub allocation_recommendation_id: Option<String>,
    pub allocation_decision_id: Option<String>,
    pub state: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub receipt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadSessionReceipt {
    pub receipt_id: String,
    pub workload_session_id: String,
    pub workload_id: String,
    pub session_id: String,
    pub node_id: String,
    pub allocation_decision_id: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub state: String,
    pub operations_executed: u32,
    pub evidence_ids: Vec<String>,
}
