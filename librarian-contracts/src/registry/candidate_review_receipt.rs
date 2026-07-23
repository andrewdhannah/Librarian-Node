use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReviewDecision {
    Approve,
    Reject,
    RequestInfo,
}

impl std::fmt::Display for ReviewDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewDecision::Approve => write!(f, "approve"),
            ReviewDecision::Reject => write!(f, "reject"),
            ReviewDecision::RequestInfo => write!(f, "request_info"),
        }
    }
}

impl From<&str> for ReviewDecision {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "approve" => ReviewDecision::Approve,
            "reject" => ReviewDecision::Reject,
            "request_info" => ReviewDecision::RequestInfo,
            _ => ReviewDecision::RequestInfo,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateReviewReceipt {
    pub receipt_id: String,
    pub candidate_id: String,
    pub decision: ReviewDecision,
    pub reviewer: String,
    pub reason: String,
    pub decided_at: String,
    pub previous_status: String,
    pub new_status: String,
}
