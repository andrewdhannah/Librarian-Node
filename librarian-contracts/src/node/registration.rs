use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistrationRequest {
    pub node_id: String,
    pub display_name: String,
    pub hostname: String,
    pub platform: String,
    pub runtime_version: String,
    pub capabilities_hash: Option<String>,
    pub requested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistrationReceipt {
    pub registration_id: String,
    pub node_id: String,
    pub status: String,
    pub registered_at: String,
    pub previous_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeRecord {
    pub node_id: String,
    pub display_name: String,
    pub hostname: String,
    pub platform: String,
    pub runtime_version: String,
    pub registration_status: String,
    pub first_registered_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub capabilities_snapshot: Option<String>,
}
