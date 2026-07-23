use serde::{Deserialize, Serialize};

use super::receipt_envelope::ReceiptEnvelope;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceQuery {
    pub node_id: Option<String>,
    pub receipt_type: Option<String>,
    pub from_timestamp: Option<String>,
    pub to_timestamp: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceResult {
    pub envelope: ReceiptEnvelope,
    pub receipt_type: String,
    pub receipt_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceGraph {
    pub node_id: String,
    pub envelopes: Vec<ReceiptEnvelope>,
    pub relationships: Vec<ProvenanceLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceLink {
    pub from_envelope_id: String,
    pub to_envelope_id: String,
    pub relationship: String,
}
