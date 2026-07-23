use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuitabilityScore {
    pub node_id: String,
    pub score: f64,
    pub requirement_matches: u32,
    pub requirement_total: u32,
    pub constraints_satisfied: u32,
    pub constraints_total: u32,
    pub evidence_verified: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationRecommendation {
    pub recommendation_id: String,
    pub workload_id: String,
    pub node_id: String,
    pub score: SuitabilityScore,
    pub reasoning: Vec<String>,
    pub generated_at: String,
    pub status: String,
}
