use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeTrustState {
    pub node_id: String,
    pub trust_level: String,
    pub score: f64,
    pub evidence_summary: String,
    pub last_assessed_at: String,
}

impl Default for NodeTrustState {
    fn default() -> Self {
        NodeTrustState {
            node_id: String::new(),
            trust_level: "unknown".to_string(),
            score: 0.0,
            evidence_summary: String::new(),
            last_assessed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustEvidence {
    pub evidence_id: String,
    pub node_id: String,
    pub metric: String,
    pub value: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustAssessmentReceipt {
    pub receipt_id: String,
    pub node_id: String,
    pub previous_score: f64,
    pub new_score: f64,
    pub factors: Vec<TrustFactor>,
    pub assessed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustFactor {
    pub name: String,
    pub weight: f64,
    pub value: f64,
    pub description: String,
}

pub fn trust_level_from_score(score: f64) -> String {
    if score >= 90.0 {
        "trusted".to_string()
    } else if score >= 70.0 {
        "onboarding".to_string()
    } else if score >= 50.0 {
        "degraded".to_string()
    } else if score >= 20.0 {
        "suspended".to_string()
    } else {
        "retired".to_string()
    }
}
