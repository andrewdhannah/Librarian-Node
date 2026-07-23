use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityComparison {
    pub capability_type: String,
    pub nodes_with_capability: Vec<String>,
    pub total_nodes_with_capability: u32,
    pub total_nodes_verified: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetCapabilityView {
    pub comparisons: Vec<CapabilityComparison>,
    pub generated_at: String,
}
