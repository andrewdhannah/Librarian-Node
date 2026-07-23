//! Runtime profile — a specific GPU/context configuration for a model.

use serde::{Deserialize, Serialize};

/// How to run a specific model on a specific device configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfile {
    pub profile_id: String,
    pub model_id: String,
    pub device_backend: String,
    pub gpu_layers: Option<i32>,
    pub context_tokens: Option<i32>,
    pub estimated_vram_mb: Option<i32>,
    pub measured_vram_mb: Option<i32>,
    pub measured_tokens_per_sec: Option<f64>,
    pub practical_context_tokens: Option<i32>,
    pub profile_priority: i32,
    pub enabled: bool,
}

impl RuntimeProfile {
    pub fn new(profile_id: String, model_id: String, device_backend: String) -> Self {
        Self {
            profile_id,
            model_id,
            device_backend,
            gpu_layers: None,
            context_tokens: None,
            estimated_vram_mb: None,
            measured_vram_mb: None,
            measured_tokens_per_sec: None,
            practical_context_tokens: None,
            profile_priority: 0,
            enabled: true,
        }
    }
}
