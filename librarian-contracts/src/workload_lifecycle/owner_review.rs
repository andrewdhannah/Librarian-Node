use serde::{Deserialize, Serialize};

use super::timeline::WorkloadTimeline;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadReview {
    pub workload_id: String,
    pub workload_type: String,
    pub description: String,
    pub state: String,
    pub node_id: String,
    pub created_at: String,
    pub duration_seconds: Option<u64>,
    pub evidence_count: Option<u32>,
    pub timeline: Option<WorkloadTimeline>,
    pub decision_chain: Option<WorkloadDecisionChain>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadDecisionChain {
    pub allocation_recommendation_id: Option<String>,
    pub allocation_decision_id: Option<String>,
    pub owner_decision_summary: Option<String>,
}
