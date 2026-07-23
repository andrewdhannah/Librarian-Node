use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictSeverity {
    Critical,
    High,
    Medium,
}

impl ConflictSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictSeverity::Critical => "critical",
            ConflictSeverity::High => "high",
            ConflictSeverity::Medium => "medium",
        }
    }

    pub fn from_classification(classification: &str) -> Self {
        match classification {
            "divergent_hash" => ConflictSeverity::Critical,
            "missing_envelope" | "incomplete_receipt" => ConflictSeverity::High,
            "orphan_session" | "state_mismatch" => ConflictSeverity::Medium,
            _ => ConflictSeverity::Medium,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassifiedDifference {
    pub difference_id: String,
    pub classification: String,
    pub artifact_type: String,
    pub artifact_id: String,
    pub severity: ConflictSeverity,
    pub expected_state: serde_json::Value,
    pub actual_state: serde_json::Value,
    pub field_path: Option<String>,
    pub details: String,
    pub detected_at: String,
}
