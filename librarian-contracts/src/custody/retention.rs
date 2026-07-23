use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetentionPolicy {
    pub policy_id: String,
    pub node_id: String,
    pub max_envelopes: Option<u32>,
    pub retention_days: Option<u32>,
    pub auto_archive: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetentionResult {
    pub policy_id: String,
    pub envelopes_before: u32,
    pub envelopes_after: u32,
    pub archived: u32,
    pub deleted: u32,
    pub applied_at: String,
}
