use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootstrapAssessment {
    pub assessment_id: String,
    pub node_id: String,
    pub session_id: String,
    pub assessed_at: String,
    pub hardware: HardwareSummary,
    pub runtime_status: RuntimeStatus,
    pub recommendations: Vec<BootstrapRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareSummary {
    pub gpu_available: bool,
    pub gpu_model: Option<String>,
    pub gpu_vram_mb: Option<u64>,
    pub ram_mb: u64,
    pub cpu_cores: u32,
    pub disk_space_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeStatus {
    pub runtime_installed: bool,
    pub runtime_version: Option<String>,
    pub backend_available: Option<String>,
    pub models_installed: u32,
    pub qualification_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootstrapRecommendation {
    pub recommendation_id: String,
    pub category: String,
    pub priority: String,
    pub description: String,
    pub action: String,
    pub impact: String,
    pub owner_approval_required: bool,
}
