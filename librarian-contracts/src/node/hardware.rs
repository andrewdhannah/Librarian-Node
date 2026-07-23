use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareProfile {
    pub cpu_model: Option<String>,
    pub cpu_cores: Option<u32>,
    pub total_ram_mb: Option<u64>,
    pub gpu_vendor: Option<String>,
    pub gpu_model: Option<String>,
    pub gpu_vram_mb: Option<u64>,
    pub os_platform: String,
}
