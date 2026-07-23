use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeOverview {
    pub node_id: String,
    pub display_name: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub state: String,
    pub registered: bool,
    pub session_count: u32,
    pub active_session_count: u32,
    pub capability_count: u32,
    pub verified_capability_count: u32,
    pub bootstrap_completed: bool,
    pub custody_envelope_count: u32,
    pub core_connected: bool,
    pub last_sync_at: Option<String>,
    pub observed_at: String,
}
