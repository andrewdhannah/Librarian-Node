use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeInventoryEntry {
    pub node_id: String,
    pub display_name: String,
    pub status: String,
    pub last_seen_at: Option<String>,
    pub runtime_version: String,
    pub platform: String,
    pub capability_count: u32,
    pub verified_capability_count: u32,
    pub session_count: u32,
    pub custody_envelope_count: u32,
    pub registered: bool,
    pub bootstrap_completed: bool,
    pub last_health_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetInventory {
    pub nodes: Vec<NodeInventoryEntry>,
    pub total_count: u32,
    pub online_count: u32,
    pub offline_count: u32,
    pub generated_at: String,
}
