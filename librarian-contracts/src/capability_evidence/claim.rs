use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityClaim {
    pub claim_id: String,
    pub node_id: String,
    pub capability_type: String,
    pub runtime: Option<String>,
    pub model_id: Option<String>,
    pub claim_version: String,
    pub claimed_at: String,
    pub status: String,
}
