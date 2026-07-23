//! Model lease — records a residency lease on the GPU.

use serde::{Deserialize, Serialize};

/// Lease states matching the Sprint 1 schema.
/// The full 8-state machine is implemented in Sprint 3.
/// Sprint 1 persists the schema and basic transitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeaseState {
    #[serde(rename = "unloaded")]
    Unloaded,
    #[serde(rename = "loading")]
    Loading,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "draining")]
    Draining,
    #[serde(rename = "unloading")]
    Unloading,
    #[serde(rename = "verifying_release")]
    VerifyingRelease,
    #[serde(rename = "failed")]
    Failed,
}

impl LeaseState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unloaded => "unloaded",
            Self::Loading => "loading",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Draining => "draining",
            Self::Unloading => "unloading",
            Self::VerifyingRelease => "verifying_release",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "unloaded" => Some(Self::Unloaded),
            "loading" => Some(Self::Loading),
            "ready" => Some(Self::Ready),
            "running" => Some(Self::Running),
            "draining" => Some(Self::Draining),
            "unloading" => Some(Self::Unloading),
            "verifying_release" => Some(Self::VerifyingRelease),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// A residency lease — one model occupies the GPU under this lease.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLease {
    pub lease_id: String,
    pub model_id: String,
    pub profile_id: Option<String>,
    pub port: Option<i32>,
    pub process_id: Option<i32>,
    pub state: LeaseState,
    pub loaded_at: Option<String>,
    pub released_at: Option<String>,
    pub vram_allocated_mb: Option<i32>,
    pub vram_released_at: Option<String>,
}

impl ModelLease {
    pub fn new(lease_id: String, model_id: String) -> Self {
        Self {
            lease_id,
            model_id,
            profile_id: None,
            port: None,
            process_id: None,
            state: LeaseState::Unloaded,
            loaded_at: None,
            released_at: None,
            vram_allocated_mb: None,
            vram_released_at: None,
        }
    }
}
