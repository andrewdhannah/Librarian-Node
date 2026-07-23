use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeProjection {
    pub projection_id: String,
    pub node_id: String,
    pub generated_at: String,
    pub node_version: String,
    pub identity: serde_json::Value,
    pub registration: Option<serde_json::Value>,
    pub capabilities: Option<serde_json::Value>,
    pub capabilities_verified: bool,
    pub session_count: u32,
    pub bootstrap_completed: bool,
    pub custody_envelope_count: u32,
    pub last_integrity_hash: Option<String>,
    pub status: String,
}
