use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryAnnouncement {
    pub node_id: String,
    pub display_name: String,
    pub node_version: String,
    pub announced_at: String,
    pub available: bool,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryResponse {
    pub node_id: String,
    pub status: String,
    pub core_version: Option<String>,
    pub contracts_version: Option<String>,
}
