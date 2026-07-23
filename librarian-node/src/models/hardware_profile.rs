//! Hardware profile — observed GPU and system characteristics.

use serde::{Deserialize, Serialize};

/// Measured hardware characteristics of this runtime node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub hw_profile_id: String,
    pub device_name: Option<String>,
    pub vulkan_device: Option<String>,
    pub total_vram_mb: Option<i32>,
    pub available_vram_mb: Option<i32>,
    pub driver_version: Option<String>,
    pub measured_at: String,
}

impl HardwareProfile {
    pub fn new(hw_profile_id: String) -> Self {
        Self {
            hw_profile_id,
            device_name: None,
            vulkan_device: None,
            total_vram_mb: None,
            available_vram_mb: None,
            driver_version: None,
            measured_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
