use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationAccuracy {
    pub recommendation_id: String,
    pub workload_id: Option<String>,
    pub selected_node_id: String,
    pub recommended: bool,
    pub workload_successful: bool,
    pub allocation_correct: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationAccuracyAnalysis {
    pub total_recommendations: u32,
    pub accepted_recommendations: u32,
    pub successful_workloads: u32,
    pub failed_workloads: u32,
    pub overall_accuracy: Option<f64>,
    pub entries: Vec<AllocationAccuracy>,
    pub generated_at: String,
}
