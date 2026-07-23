use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeCapability {
    pub runtime_type: String,
    pub runtime_version: String,
    pub backend: String,
    pub hardware_requirements: Vec<String>,
    pub evidence_packet_ids: Vec<String>,
    pub qualification_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRuntimeProfile {
    pub model_id: String,
    pub runtime_capabilities: Vec<RuntimeCapability>,
    pub last_qualified_at: Option<String>,
    pub qualification_summary: Option<String>,
}
