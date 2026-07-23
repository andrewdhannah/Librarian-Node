use serde::{Deserialize, Serialize};

use crate::evidence_classification::FindingSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsightDashboard {
    pub node_id: String,
    pub generated_at: String,
    pub findings_summary: FindingSummary,
    pub active_anomalies: u32,
    pub workload_trend: WorkloadTrendSummary,
    pub capability_health: CapabilityHealthSummary,
    pub allocation_quality: AllocationQualitySummary,
    pub recent_activity: RecentActivitySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadTrendSummary {
    pub total_workloads: u32,
    pub overall_success_rate: f64,
    pub avg_duration_seconds: f64,
    pub trend_direction: String,
    pub comparison_window: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityHealthSummary {
    pub total_capabilities: u32,
    pub healthy: u32,
    pub degraded: u32,
    pub untested: u32,
    pub capabilities_with_anomalies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllocationQualitySummary {
    pub total_recommendations: u32,
    pub accepted: u32,
    pub successful: u32,
    pub accuracy_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentActivitySummary {
    pub recent_workloads: u32,
    pub recent_findings: u32,
    pub recent_anomalies: u32,
    pub recent_owner_actions: u32,
    pub since_timestamp: String,
}
