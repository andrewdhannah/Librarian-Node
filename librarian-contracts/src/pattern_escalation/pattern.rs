use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternFinding {
    pub pattern_id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub status: String,
    pub affected_node_id: String,
    pub affected_entity_type: String,
    pub affected_entity_id: Option<String>,
    pub constituent_finding_ids: Vec<String>,
    pub constituent_anomaly_ids: Vec<String>,
    pub first_detected_at: String,
    pub last_observed_at: String,
    pub finding_count: u32,
    pub time_window_hours: u32,
    pub confidence: String,
    pub provenance: PatternProvenance,
    pub owner_review_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternProvenance {
    pub evidence_references: Vec<String>,
    pub workload_ids: Vec<String>,
    pub session_ids: Vec<String>,
    pub custody_envelope_ids: Vec<String>,
}
