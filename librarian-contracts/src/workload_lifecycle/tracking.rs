use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadSummary {
    pub workload_id: String,
    pub workload_type: String,
    pub description: String,
    pub state: String,
    pub node_id: String,
    pub node_name: String,
    pub session_id: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub duration_seconds: Option<u64>,
    pub operations_executed: Option<u32>,
    pub evidence_count: Option<u32>,
    pub has_receipt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadInventory {
    pub total: u32,
    pub active: u32,
    pub completed: u32,
    pub failed: u32,
    pub pending: u32,
    pub cancelled: u32,
    pub workloads: Vec<WorkloadSummary>,
    pub generated_at: String,
}
