use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityManifest {
    pub node_id: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capability {
    pub capability_type: String,
    pub runtime: Option<String>,
    pub models: Option<Vec<ModelDescriptor>>,
    pub available: bool,
    pub verification_status: Option<String>,
    pub evidence_count: Option<u32>,
    #[serde(default)]
    pub runtime_qualification_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelDescriptor {
    pub model_id: String,
    pub quantization: Option<String>,
    pub family: Option<String>,
}
