use serde::{Deserialize, Serialize};

use super::capability_match::CapabilityRequirement;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationReceipt {
    pub receipt_id: String,
    pub recommendation_id: String,
    pub workload_id: String,
    pub node_id: String,
    pub decided_by: String,
    pub decision: String,
    pub decided_at: String,
    pub session_id: Option<String>,
    pub custody_envelope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationRequest {
    pub request_id: String,
    pub workload_description: String,
    pub requirements: Vec<CapabilityRequirement>,
    pub preferred_nodes: Option<Vec<String>>,
    pub requested_at: String,
}
