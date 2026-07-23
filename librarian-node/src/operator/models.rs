//! Operator surface — human-facing runtime models.

use serde::{Deserialize, Serialize};

/// Runtime state for the operator UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuntimeIndicator {
    Unavailable,
    Loading,
    Ready,
    Running,
    Stopping,
    Unloaded,
    Error(String),
}

impl RuntimeIndicator {
    pub fn as_str(&self) -> &'static str {
        match self { Self::Unavailable => "unavailable", Self::Loading => "loading", Self::Ready => "ready", Self::Running => "running", Self::Stopping => "stopping", Self::Unloaded => "unloaded", Self::Error(_) => "error" }
    }
}

/// A model entry shown to the operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelEntry {
    pub model_id: String,
    pub filename: String,
    pub quantization: String,
    pub qualified: bool,
    pub loaded: bool,
    pub active: bool,
    pub gpu_vram_mb: u64,
    pub context_length: u64,
}

/// Snapshot of the current runtime state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeSnapshot {
    pub status: RuntimeIndicator,
    pub active_model: Option<String>,
    pub process_id: Option<i32>,
    pub gpu_vram_used_mb: Option<u64>,
    pub gpu_vram_total_mb: Option<u64>,
    pub generation_speed: Option<f64>,
    pub uptime_seconds: Option<u64>,
    pub load_duration_ms: Option<u64>,
}

/// Operator event — recorded locally.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorEvent {
    pub event_id: String,
    pub event_type: String,
    pub model_id: Option<String>,
    pub message: String,
    pub timestamp: String,
}

/// Complete operator state for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorState {
    pub runtime: RuntimeSnapshot,
    pub models: Vec<ModelEntry>,
    pub events: Vec<OperatorEvent>,
    pub version: String,
}
