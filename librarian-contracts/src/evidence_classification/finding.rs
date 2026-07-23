use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindingCategory {
    pub category: String,
    pub display_name: String,
    pub description: String,
    pub severity_default: String,
    pub requires_owner_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassifiedFinding {
    pub finding_id: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub confidence: String,
    pub detection_method: String,
    pub affected_entity_type: String,
    pub affected_entity_id: Option<String>,
    pub evidence_references: Vec<String>,
    pub owner_review_status: String,
    pub generated_at: String,
}
