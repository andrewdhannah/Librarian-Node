use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadTimelineEntry {
    pub event_id: String,
    pub workload_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub details: Option<String>,
    pub associated_receipt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadTimeline {
    pub workload_id: String,
    pub node_id: String,
    pub session_id: String,
    pub entries: Vec<WorkloadTimelineEntry>,
    pub generated_at: String,
}
