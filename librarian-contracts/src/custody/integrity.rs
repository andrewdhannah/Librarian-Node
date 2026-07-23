use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntegrityReport {
    pub chain_id: String,
    pub node_id: String,
    pub verified: bool,
    pub envelope_count: u32,
    pub envelopes_checked: u32,
    pub errors: Vec<IntegrityError>,
    pub verified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntegrityError {
    pub envelope_id: String,
    pub error_type: String,
    pub details: String,
}
