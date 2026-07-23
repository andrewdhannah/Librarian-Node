use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CandidateStatus {
    Discovered,
    Candidate,
    EvidenceCollection,
    UnderReview,
    Approved,
    Rejected,
    Admitted,
}

impl std::fmt::Display for CandidateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidateStatus::Discovered => write!(f, "discovered"),
            CandidateStatus::Candidate => write!(f, "candidate"),
            CandidateStatus::EvidenceCollection => write!(f, "evidence_collection"),
            CandidateStatus::UnderReview => write!(f, "under_review"),
            CandidateStatus::Approved => write!(f, "approved"),
            CandidateStatus::Rejected => write!(f, "rejected"),
            CandidateStatus::Admitted => write!(f, "admitted"),
        }
    }
}

impl From<&str> for CandidateStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "discovered" => CandidateStatus::Discovered,
            "candidate" => CandidateStatus::Candidate,
            "evidence_collection" => CandidateStatus::EvidenceCollection,
            "under_review" => CandidateStatus::UnderReview,
            "approved" => CandidateStatus::Approved,
            "rejected" => CandidateStatus::Rejected,
            "admitted" => CandidateStatus::Admitted,
            _ => CandidateStatus::Discovered,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscoveryMethod {
    LocalStartup,
    ApiDiscovery,
    Manual,
}

impl std::fmt::Display for DiscoveryMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryMethod::LocalStartup => write!(f, "local_startup"),
            DiscoveryMethod::ApiDiscovery => write!(f, "api_discovery"),
            DiscoveryMethod::Manual => write!(f, "manual"),
        }
    }
}

impl From<&str> for DiscoveryMethod {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "local_startup" => DiscoveryMethod::LocalStartup,
            "api_discovery" => DiscoveryMethod::ApiDiscovery,
            "manual" => DiscoveryMethod::Manual,
            _ => DiscoveryMethod::ApiDiscovery,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeCandidate {
    pub candidate_id: String,
    pub node_id: String,
    pub display_name: String,
    pub status: CandidateStatus,
    pub first_seen_at: String,
    pub last_updated_at: String,
    pub discovery_method: DiscoveryMethod,
}
