use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeIdentity {
    pub node_id: String,
    pub display_name: String,
    pub platform: String,
    pub runtime_version: String,
    pub contract_version: String,
    pub first_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeStatus {
    pub identity: NodeIdentity,
    pub state: String,
    pub uptime_seconds: u64,
    pub last_state_change: String,
}
