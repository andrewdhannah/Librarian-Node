use serde::{Deserialize, Serialize};

use super::tracking::WorkloadSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadHistoryQuery {
    pub node_id: Option<String>,
    pub state: Option<String>,
    pub workload_type: Option<String>,
    pub from_timestamp: Option<String>,
    pub to_timestamp: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadHistoryResult {
    pub total: u32,
    pub returned: u32,
    pub workloads: Vec<WorkloadSummary>,
    pub generated_at: String,
}
