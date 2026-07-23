use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityRequirement {
    pub requirement_id: String,
    pub capability_type: String,
    pub required: bool,
    pub constraints: Option<Vec<RequirementConstraint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequirementConstraint {
    pub constraint_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityMatch {
    pub node_id: String,
    pub requirement_id: String,
    pub matches: bool,
    pub evidence_verified: bool,
    pub match_confidence: String,
    pub details: Option<String>,
}
