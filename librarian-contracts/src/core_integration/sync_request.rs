use serde::{Deserialize, Serialize};

use super::node_projection::NodeProjection;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncRequest {
    pub request_id: String,
    pub node_id: String,
    pub node_version: String,
    pub last_sync_at: Option<String>,
    pub projection: NodeProjection,
    pub requested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncReceipt {
    pub receipt_id: String,
    pub request_id: String,
    pub node_id: String,
    pub status: String,
    pub accepted_envelopes: u32,
    pub rejected_envelopes: u32,
    pub errors: Vec<SyncError>,
    pub processed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncError {
    pub envelope_id: String,
    pub reason: String,
}
