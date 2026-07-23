use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntelligenceFinding {
    pub finding_id: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub supporting_data: serde_json::Value,
    pub source_references: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntelligenceReport {
    pub report_id: String,
    pub generated_at: String,
    pub workload_analysis: WorkloadOutcomeAnalysis,
    pub capability_analysis: CapabilityEffectivenessAnalysis,
    pub allocation_analysis: AllocationAccuracyAnalysis,
    pub findings: Vec<IntelligenceFinding>,
}

use super::allocation_accuracy::AllocationAccuracyAnalysis;
use super::capability_effectiveness::CapabilityEffectivenessAnalysis;
use super::workload_outcomes::WorkloadOutcomeAnalysis;
