use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityEffectiveness {
    pub capability_type: String,
    pub workloads_using: u32,
    pub successful_workloads: u32,
    pub failed_workloads: u32,
    pub success_rate: f64,
    pub avg_evidence_per_workload: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityEffectivenessAnalysis {
    pub entries: Vec<CapabilityEffectiveness>,
    pub total_capabilities: u32,
    pub generated_at: String,
}
