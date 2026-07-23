//! Shared types for bridge packets.
//!
//! These types are used by both QualificationRequest (Mac→Windows) and
//! EvidencePacket (Windows→Mac). They represent the identity and execution
//! binding that crosses the authority boundary.

use serde::{Deserialize, Serialize};

/// Model identity reference — carried in bridge packets.
/// Binds the packet to an exact model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketModelIdentity {
    /// Model ID (references Windows local_models.model_id).
    pub model_id: String,

    /// SHA-256 hash of the model artifact (hex).
    pub sha256: String,

    /// Filename of the model artifact.
    pub filename: String,

    /// Quantization of the model artifact.
    pub quantization: Option<String>,
}

/// Execution configuration — carried in QualificationRequest.
/// Defines what the Windows node should execute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PacketExecutionConfig {
    /// Runtime profile ID to use (references Windows runtime_profiles.profile_id).
    pub runtime_profile_id: String,

    /// Task description (human-readable prompt or fixture reference).
    pub task_description: String,

    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,

    /// Temperature for generation (0.0 = deterministic).
    pub temperature: Option<f64>,

    /// Timeout in seconds for the entire run.
    pub timeout_seconds: Option<u32>,
}

/// Execution constraints — carried in QualificationRequest.
/// Defines bounds the Windows node must respect.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketConstraints {
    /// Whether release verification is required.
    pub require_release_proof: bool,

    /// Maximum VRAM the model may use (in MB).
    pub max_vram_mb: Option<u32>,
}

/// Execution identity — carried in EvidencePacket.
/// Binds the evidence to the exact runtime that executed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketExecutionIdentity {
    /// Runtime profile ID used (references Windows runtime_profiles.profile_id).
    pub runtime_profile_id: String,

    /// Hardware profile ID used (references Windows hardware_profiles.hw_profile_id).
    pub hardware_profile_id: String,

    /// SHA-256 of the runtime executable (hex).
    pub runtime_executable_sha256: String,

    /// Runtime executable version string.
    pub runtime_executable_version: String,
}

/// Lease lifecycle — carried in EvidencePacket.
/// Records the residency lifecycle for this execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketLeaseLifecycle {
    /// Lease ID (references Windows job_leases.lease_id).
    pub lease_id: String,

    /// Port the model was served on.
    pub port: Option<u16>,

    /// Final lease state.
    pub state: String,

    /// When the model was loaded.
    pub loaded_at: Option<String>,

    /// When the model was released.
    pub released_at: Option<String>,

    /// When GPU VRAM was verified released.
    pub vram_released_at: Option<String>,
}

/// Execution metrics — carried in EvidencePacket.
/// Records the execution lifecycle for this run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketExecutionMetrics {
    /// Run ID (references Windows runtime_runs.run_id).
    pub run_id: String,

    /// Input tokens consumed.
    pub input_tokens: Option<u32>,

    /// Output tokens generated.
    pub output_tokens: Option<u32>,

    /// Load duration in milliseconds.
    pub load_duration_ms: Option<u64>,

    /// Generation duration in milliseconds.
    pub generation_duration_ms: Option<u64>,

    /// Exit status of the process.
    pub exit_status: Option<String>,

    /// When the run started.
    pub started_at: Option<String>,

    /// When the run ended.
    pub ended_at: Option<String>,
}

/// Lifecycle event — carried in EvidencePacket.
/// Records a single event from the Windows lifecycle evidence chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketLifecycleEvent {
    /// Event type (matches Windows LifecycleEventType).
    pub event_type: String,

    /// Process ID that generated this event.
    pub process_id: Option<i32>,

    /// Observed state at the time of the event.
    pub observed_state: Option<String>,

    /// Observation data (JSON string).
    pub observation: Option<String>,

    /// When the event occurred.
    pub occurred_at: Option<String>,
}

/// GPU release verification — carried in EvidencePacket.
/// Records the VRAM release proof.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketReleaseVerification {
    /// Whether the process PID was verified exited.
    pub pid_exit_verified: bool,

    /// Whether GPU VRAM was verified released.
    pub gpu_release_verified: bool,

    /// Free VRAM after release (in MB).
    pub free_vram_mb: Option<u64>,

    /// Baseline free VRAM (in MB).
    pub baseline_vram_mb: Option<u64>,

    /// Whether the release was within tolerance.
    pub within_tolerance: bool,
}
