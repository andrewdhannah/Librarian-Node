use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadOutcomeSummary {
    pub workload_type: String,
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub success_rate: f64,
    pub avg_duration_seconds: Option<f64>,
    pub evidence_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadOutcomeAnalysis {
    pub summaries: Vec<WorkloadOutcomeSummary>,
    pub total_workloads: u32,
    pub overall_success_rate: f64,
    pub generated_at: String,
}
