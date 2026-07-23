use serde::{Deserialize, Serialize};

use super::node_projection::NodeProjection;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoreNodeRecord {
    pub node_id: String,
    pub first_seen_at: String,
    pub last_sync_at: String,
    pub last_projection: Option<NodeProjection>,
    pub sync_count: u32,
    pub status: String,
}
