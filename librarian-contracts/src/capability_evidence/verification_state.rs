use serde::{Deserialize, Serialize};

use super::evidence_reference::EvidenceReference;
use super::lifecycle::CapabilityState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityVerificationState {
    pub node_id: String,
    pub capabilities: Vec<VerifiedCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifiedCapability {
    pub capability_type: String,
    pub claim_id: String,
    pub verification_status: String,
    pub last_verified_at: Option<String>,
    pub evidence_references: Vec<EvidenceReference>,
    #[serde(default)]
    pub state: CapabilityState,
}
