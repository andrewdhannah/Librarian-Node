use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptEnvelope {
    pub envelope_id: String,
    pub node_id: String,
    pub receipt_type: String,
    pub receipt_id: String,
    pub receipt_payload: serde_json::Value,
    pub receipt_hash: String,
    pub previous_envelope_id: Option<String>,
    pub previous_envelope_hash: Option<String>,
    pub chain_hash: String,
    pub timestamp: String,
    pub metadata: Option<CustodyMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustodyMetadata {
    pub source: String,
    pub version: String,
    pub notes: Option<String>,
}
