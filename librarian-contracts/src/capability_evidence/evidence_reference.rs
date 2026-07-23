use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceReference {
    pub reference_id: String,
    pub claim_id: String,
    pub evidence_packet_id: String,
    pub qualification_run_id: String,
    pub verification_status: String,
    pub verified_at: Option<String>,
    pub evidence_hash: Option<String>,
}
