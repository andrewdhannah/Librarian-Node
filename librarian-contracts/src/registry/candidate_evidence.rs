use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvidenceType {
    Identity,
    Capability,
    Custody,
    Health,
    OwnerNote,
}

impl std::fmt::Display for EvidenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceType::Identity => write!(f, "identity"),
            EvidenceType::Capability => write!(f, "capability"),
            EvidenceType::Custody => write!(f, "custody"),
            EvidenceType::Health => write!(f, "health"),
            EvidenceType::OwnerNote => write!(f, "owner_note"),
        }
    }
}

impl From<&str> for EvidenceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "identity" => EvidenceType::Identity,
            "capability" => EvidenceType::Capability,
            "custody" => EvidenceType::Custody,
            "health" => EvidenceType::Health,
            "owner_note" => EvidenceType::OwnerNote,
            _ => EvidenceType::Identity,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateEvidence {
    pub evidence_id: String,
    pub candidate_id: String,
    pub evidence_type: EvidenceType,
    pub payload: Value,
    pub collected_at: String,
    pub retention_days: u32,
}
