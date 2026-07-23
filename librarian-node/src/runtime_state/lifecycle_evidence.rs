//! Lifecycle evidence — append-only event log for runtime observations.

use serde::{Deserialize, Serialize};

/// Event types for lifecycle evidence.
/// Sprint 1 schema supports all types; not all are generated yet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LifecycleEventType {
    // Runtime lifecycle
    #[serde(rename = "runtime_startup")]
    RuntimeStartup,
    #[serde(rename = "database_opened")]
    DatabaseOpened,
    #[serde(rename = "migration_applied")]
    MigrationApplied,

    // Model loading
    #[serde(rename = "model_load_requested")]
    ModelLoadRequested,
    #[serde(rename = "process_spawned")]
    ProcessSpawned,
    #[serde(rename = "health_starting")]
    HealthStarting,
    #[serde(rename = "health_healthy")]
    HealthHealthy,
    #[serde(rename = "health_degraded")]
    HealthDegraded,
    #[serde(rename = "health_failed")]
    HealthFailed,

    // Lease lifecycle
    #[serde(rename = "lease_acquired")]
    LeaseAcquired,
    #[serde(rename = "run_started")]
    RunStarted,
    #[serde(rename = "run_completed")]
    RunCompleted,
    #[serde(rename = "run_failed")]
    RunFailed,
    #[serde(rename = "lease_released")]
    LeaseReleased,

    // Model unloading
    #[serde(rename = "model_unload_requested")]
    ModelUnloadRequested,
    #[serde(rename = "process_stop_requested")]
    ProcessStopRequested,
    #[serde(rename = "process_exited")]
    ProcessExited,
    #[serde(rename = "process_killed")]
    ProcessKilled,
    #[serde(rename = "release_verified")]
    ReleaseVerified,

    // Recovery
    #[serde(rename = "runtime_reconciled")]
    RuntimeReconciled,
    #[serde(rename = "orphan_process_detected")]
    OrphanProcessDetected,
    #[serde(rename = "stale_lease_detected")]
    StaleLeaseDetected,

    // Sprint 3: GPU and run lifecycle
    #[serde(rename = "gpu_release_verified")]
    GpuReleaseVerified,
    #[serde(rename = "run_interrupted")]
    RunInterrupted,
    #[serde(rename = "qualification_started")]
    QualificationStarted,
    #[serde(rename = "qualification_completed")]
    QualificationCompleted,
}

impl LifecycleEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RuntimeStartup => "runtime_startup",
            Self::DatabaseOpened => "database_opened",
            Self::MigrationApplied => "migration_applied",
            Self::ModelLoadRequested => "model_load_requested",
            Self::ProcessSpawned => "process_spawned",
            Self::HealthStarting => "health_starting",
            Self::HealthHealthy => "health_healthy",
            Self::HealthDegraded => "health_degraded",
            Self::HealthFailed => "health_failed",
            Self::LeaseAcquired => "lease_acquired",
            Self::RunStarted => "run_started",
            Self::RunCompleted => "run_completed",
            Self::RunFailed => "run_failed",
            Self::LeaseReleased => "lease_released",
            Self::ModelUnloadRequested => "model_unload_requested",
            Self::ProcessStopRequested => "process_stop_requested",
            Self::ProcessExited => "process_exited",
            Self::ProcessKilled => "process_killed",
            Self::ReleaseVerified => "release_verified",
            Self::RuntimeReconciled => "runtime_reconciled",
            Self::OrphanProcessDetected => "orphan_process_detected",
            Self::StaleLeaseDetected => "stale_lease_detected",
            Self::GpuReleaseVerified => "gpu_release_verified",
            Self::RunInterrupted => "run_interrupted",
            Self::QualificationStarted => "qualification_started",
            Self::QualificationCompleted => "qualification_completed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "runtime_startup" => Some(Self::RuntimeStartup),
            "database_opened" => Some(Self::DatabaseOpened),
            "migration_applied" => Some(Self::MigrationApplied),
            "model_load_requested" => Some(Self::ModelLoadRequested),
            "process_spawned" => Some(Self::ProcessSpawned),
            "health_starting" => Some(Self::HealthStarting),
            "health_healthy" => Some(Self::HealthHealthy),
            "health_degraded" => Some(Self::HealthDegraded),
            "health_failed" => Some(Self::HealthFailed),
            "lease_acquired" => Some(Self::LeaseAcquired),
            "run_started" => Some(Self::RunStarted),
            "run_completed" => Some(Self::RunCompleted),
            "run_failed" => Some(Self::RunFailed),
            "lease_released" => Some(Self::LeaseReleased),
            "model_unload_requested" => Some(Self::ModelUnloadRequested),
            "process_stop_requested" => Some(Self::ProcessStopRequested),
            "process_exited" => Some(Self::ProcessExited),
            "process_killed" => Some(Self::ProcessKilled),
            "release_verified" => Some(Self::ReleaseVerified),
            "runtime_reconciled" => Some(Self::RuntimeReconciled),
            "orphan_process_detected" => Some(Self::OrphanProcessDetected),
            "stale_lease_detected" => Some(Self::StaleLeaseDetected),
            "gpu_release_verified" => Some(Self::GpuReleaseVerified),
            "run_interrupted" => Some(Self::RunInterrupted),
            "qualification_started" => Some(Self::QualificationStarted),
            "qualification_completed" => Some(Self::QualificationCompleted),
            _ => None,
        }
    }
}

/// An append-only lifecycle evidence record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvidence {
    pub evidence_id: String,
    pub event_type: LifecycleEventType,
    pub model_id: Option<String>,
    pub profile_id: Option<String>,
    pub lease_id: Option<String>,
    pub run_id: Option<String>,
    pub process_id: Option<i32>,
    pub observed_state: Option<String>,
    pub observation_json: String,
    pub occurred_at: String,
    pub recorded_at: String,
}

impl LifecycleEvidence {
    pub fn new(
        evidence_id: String,
        event_type: LifecycleEventType,
        observation_json: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            evidence_id,
            event_type,
            model_id: None,
            profile_id: None,
            lease_id: None,
            run_id: None,
            process_id: None,
            observed_state: None,
            observation_json,
            occurred_at: now.clone(),
            recorded_at: now,
        }
    }
}
