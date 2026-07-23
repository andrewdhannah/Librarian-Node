use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationReviewRequest {
    pub request_id: String,
    pub session_id: String,
    pub filter_status: Option<String>,
    pub requested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationReviewResult {
    pub result_id: String,
    pub request_id: String,
    pub pending_count: u32,
    pub total_count: u32,
    pub recommendations: Vec<AllocationRecommendationSummary>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationRecommendationSummary {
    pub recommendation_id: String,
    pub workload_description: String,
    pub recommended_node_id: String,
    pub recommended_node_name: String,
    pub score: f64,
    pub evidence_verified: bool,
    pub key_reasoning: Vec<String>,
    pub status: String,
    pub generated_at: String,
}
