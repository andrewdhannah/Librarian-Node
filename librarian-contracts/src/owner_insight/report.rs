use serde::{Deserialize, Serialize};

use super::dashboard::InsightDashboard;
use crate::anomaly_detection::AnomalyFinding;
use crate::evidence_classification::ClassifiedFinding;
use crate::evidence_intelligence::{AllocationAccuracyAnalysis, CapabilityEffectivenessAnalysis, WorkloadOutcomeAnalysis};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsightReport {
    pub report_id: String,
    pub node_id: String,
    pub generated_at: String,
    pub report_period: String,
    pub dashboard: InsightDashboard,
    pub detailed_findings: Vec<ClassifiedFinding>,
    pub detailed_anomalies: Vec<AnomalyFindingSummary>,
    pub workload_breakdown: WorkloadOutcomeAnalysis,
    pub capability_breakdown: CapabilityEffectivenessAnalysis,
    pub allocation_breakdown: AllocationAccuracyAnalysis,
    pub recommendations: Vec<OwnerRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnomalyFindingSummary {
    pub anomaly_id: String,
    pub metric_name: String,
    pub context: String,
    pub deviation_factor: f64,
    pub severity: String,
    pub status: String,
    pub detected_at: String,
}

impl From<AnomalyFinding> for AnomalyFindingSummary {
    fn from(a: AnomalyFinding) -> Self {
        AnomalyFindingSummary {
            anomaly_id: a.anomaly_id,
            metric_name: a.observation.metric_name,
            context: a.observation.context,
            deviation_factor: a.observation.deviation_factor,
            severity: a.severity,
            status: "open".to_string(),
            detected_at: a.generated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnerRecommendation {
    pub recommendation_id: String,
    pub category: String,
    pub priority: String,
    pub title: String,
    pub description: String,
    pub supporting_evidence_count: u32,
    pub generated_at: String,
}
