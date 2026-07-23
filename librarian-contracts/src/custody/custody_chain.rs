use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustodyChain {
    pub chain_id: String,
    pub node_id: String,
    pub created_at: String,
    pub envelope_count: u32,
    pub first_envelope_id: String,
    pub last_envelope_id: String,
    pub last_chain_hash: String,
    pub status: String,
}
