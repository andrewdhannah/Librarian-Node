use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionStartRequest {
    pub node_id: String,
    pub agent_id: Option<String>,
    pub requested_capabilities: Option<Vec<String>>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub session_id: String,
    pub node_id: String,
    pub agent_id: Option<String>,
    pub state: String,
    pub started_at: String,
    pub closed_at: Option<String>,
    pub capability_snapshot: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionReceipt {
    pub receipt_id: String,
    pub session_id: String,
    pub node_id: String,
    pub started_at: String,
    pub closed_at: String,
    pub operations_executed: u32,
    pub evidence_ids: Vec<String>,
    pub capability_snapshot_hash: Option<String>,
}
