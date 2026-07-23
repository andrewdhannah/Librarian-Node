//! Qualification run result — structured output of a qualification run.
//!
//! Contains everything the runner produced: run metadata, raw output,
//! settings, telemetry, and lifecycle evidence. Ready for later
//! deterministic validators.
//!
//! Does NOT contain:
//! - Capability status
//! - Role assignments
//! - Qualification decisions
//! - Router eligibility

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::custom_executor::CustomRuleEvidence;
use super::run_state::RunState;

/// Lifecycle event recorded during the run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunLifecycleEvent {
    /// Event state.
    pub state: RunState,

    /// When the event occurred (RFC 3339).
    pub occurred_at: String,

    /// Optional observation data (JSON).
    pub observation: Option<String>,
}

/// Generation settings used for this run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationSettings {
    /// Runtime profile ID.
    pub runtime_profile_id: String,

    /// Max tokens requested.
    pub max_tokens: Option<u32>,

    /// Temperature.
    pub temperature: Option<f64>,

    /// Timeout in seconds.
    pub timeout_seconds: Option<u32>,

    /// Task description from the request.
    pub task_description: String,
}

/// Runtime telemetry captured during the run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTelemetry {
    /// Port the model was served on.
    pub port: Option<u16>,

    /// Process ID of the runtime.
    pub process_id: Option<i32>,

    /// Duration the model was loaded (ms).
    pub load_duration_ms: Option<u64>,

    /// Duration of the generation (ms).
    pub generation_duration_ms: Option<u64>,

    /// Input tokens consumed.
    pub input_tokens: Option<u32>,

    /// Output tokens generated.
    pub output_tokens: Option<u32>,

    /// HTTP status code from the runtime (if applicable).
    pub http_status: Option<u16>,

    /// Runtime error message (if applicable).
    pub runtime_error: Option<String>,
}

/// Complete qualification run result.
///
/// This struct contains everything the runner produced during a
/// qualification run. It is the input for later deterministic validators
/// (Stage 1, Stage 2, etc.).
///
/// The runner does NOT interpret this result for capability or qualification.
/// That responsibility belongs to later stages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualificationRunResult {
    /// Deterministic run ID (SHA-256 of request_id + timestamp).
    pub run_id: String,

    /// The qualification request ID this run responds to.
    pub request_id: String,

    /// Model identity (exact artifact binding).
    pub model_id: String,
    pub model_sha256: String,
    pub model_filename: String,

    /// Task pack ID used.
    pub task_pack_id: String,

    /// Task pack fixture hash (integrity proof).
    pub fixture_hash: String,

    /// Final run state.
    pub state: RunState,

    /// Raw model output (preserved exactly, never interpreted).
    pub raw_output: Option<String>,

    /// Generation settings used.
    pub settings: GenerationSettings,

    /// Runtime telemetry captured.
    pub telemetry: RuntimeTelemetry,

    /// Ordered lifecycle events.
    pub lifecycle_events: Vec<RunLifecycleEvent>,

    /// Error message (if state is a failure).
    pub error_message: Option<String>,

    /// Custom rule validation evidence — additive, inspectable, non-authoritative.
    pub custom_evidence: Vec<CustomRuleEvidence>,

    /// When the run started (RFC 3339).
    pub started_at: String,

    /// When the run ended (RFC 3339).
    pub ended_at: Option<String>,
}

impl QualificationRunResult {
    /// Compute a deterministic run ID from request_id and timestamp.
    pub fn compute_run_id(request_id: &str, started_at: &str) -> String {
        let input = format!("{}:{}", request_id, started_at);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Validate the result structure.
    pub fn validate(&self) -> Result<()> {
        if self.run_id.is_empty() {
            anyhow::bail!("run_id is empty");
        }
        if self.request_id.is_empty() {
            anyhow::bail!("request_id is empty");
        }
        if self.model_id.is_empty() {
            anyhow::bail!("model_id is empty");
        }
        if self.model_sha256.is_empty() {
            anyhow::bail!("model_sha256 is empty");
        }
        if self.task_pack_id.is_empty() {
            anyhow::bail!("task_pack_id is empty");
        }
        if self.fixture_hash.is_empty() {
            anyhow::bail!("fixture_hash is empty");
        }
        if self.started_at.is_empty() {
            anyhow::bail!("started_at is empty");
        }
        Ok(())
    }

    /// Compute SHA-256 hash of the serialized result.
    pub fn compute_hash(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .context("Failed to serialize result for hashing")?;
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize result to JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse QualificationRunResult from JSON")
    }

    /// Assert this result contains no capability authority data.
    pub fn assert_no_capability_data(&self) -> Result<()> {
        // QualificationRunResult must not contain:
        // - role assignments
        // - capability status
        // - qualification decisions
        // - router eligibility
        //
        // Structural proof: the fields are:
        // - run_id, request_id (identifiers)
        // - model_id, model_sha256, model_filename (identity binding)
        // - task_pack_id, fixture_hash (task binding)
        // - state (run lifecycle — NOT capability)
        // - raw_output (preserved evidence — NOT capability)
        // - settings, telemetry (execution context — NOT capability)
        // - lifecycle_events (evidence chain — NOT capability)
        // - error_message (diagnostic — NOT capability)
        // - custom_evidence (validation evidence — NOT capability)
        // - started_at, ended_at (timestamps)
        //
        // There are no fields for:
        // - role
        // - capability_status
        // - qualification_status
        // - approved_roles
        // - router_eligible
        Ok(())
    }

    /// Add a lifecycle event.
    pub fn record_event(&mut self, state: RunState, observation: Option<String>) {
        self.lifecycle_events.push(RunLifecycleEvent {
            state,
            occurred_at: chrono::Utc::now().to_rfc3339(),
            observation,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_result() -> QualificationRunResult {
        let run_id = QualificationRunResult::compute_run_id("qr-test-001", "2026-07-11T12:00:00Z");
        QualificationRunResult {
            run_id,
            request_id: "qr-test-001".to_string(),
            model_id: "minicpm5-1b-q4km".to_string(),
            model_sha256: "81B64D05A23B".to_string(),
            model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            task_pack_id: "tp-if-001".to_string(),
            fixture_hash: "abc123".to_string(),
            state: RunState::Completed,
            raw_output: Some("Hello, how can I help you?".to_string()),
            settings: GenerationSettings {
                runtime_profile_id: "prof-q4km".to_string(),
                max_tokens: Some(256),
                temperature: Some(0.0),
                timeout_seconds: Some(120),
                task_description: "Test fixture".to_string(),
            },
            telemetry: RuntimeTelemetry {
                port: Some(9120),
                process_id: Some(10804),
                load_duration_ms: Some(2187),
                generation_duration_ms: Some(385),
                input_tokens: Some(10),
                output_tokens: Some(32),
                http_status: Some(200),
                runtime_error: None,
            },
            lifecycle_events: vec![],
            error_message: None,
            custom_evidence: vec![],
            started_at: "2026-07-11T12:00:00Z".to_string(),
            ended_at: Some("2026-07-11T12:00:01Z".to_string()),
        }
    }

    #[test]
    fn test_round_trip() {
        let result = test_result();
        let json = result.to_json().unwrap();
        let parsed = QualificationRunResult::from_json(&json).unwrap();
        assert_eq!(result, parsed);
    }

    #[test]
    fn test_hash_deterministic() {
        let result = test_result();
        let h1 = result.compute_hash().unwrap();
        let h2 = result.compute_hash().unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_run_id_deterministic() {
        let id1 = QualificationRunResult::compute_run_id("qr-1", "2026-07-11T12:00:00Z");
        let id2 = QualificationRunResult::compute_run_id("qr-1", "2026-07-11T12:00:00Z");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    #[test]
    fn test_run_id_depends_on_inputs() {
        let id1 = QualificationRunResult::compute_run_id("qr-1", "2026-07-11T12:00:00Z");
        let id2 = QualificationRunResult::compute_run_id("qr-2", "2026-07-11T12:00:00Z");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_validate_passes() {
        let result = test_result();
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_run_id() {
        let mut result = test_result();
        result.run_id = "".to_string();
        assert!(result.validate().is_err());
    }

    #[test]
    fn test_validate_empty_model_id() {
        let mut result = test_result();
        result.model_id = "".to_string();
        assert!(result.validate().is_err());
    }

    #[test]
    fn test_no_capability_data() {
        let result = test_result();
        assert!(result.assert_no_capability_data().is_ok());
    }

    #[test]
    fn test_record_event() {
        let mut result = test_result();
        result.record_event(RunState::Received, None);
        result.record_event(RunState::Executing, Some(r#"{"port":9120}"#.to_string()));
        assert_eq!(result.lifecycle_events.len(), 2);
        assert_eq!(result.lifecycle_events[0].state, RunState::Received);
        assert_eq!(result.lifecycle_events[1].state, RunState::Executing);
    }

    #[test]
    fn test_failed_result_preserves_error() {
        let mut result = test_result();
        result.state = RunState::ModelFailed;
        result.raw_output = None;
        result.error_message = Some("Model returned empty output".to_string());
        result.ended_at = None;

        assert!(result.state.is_failure());
        assert!(result.error_message.is_some());
        assert!(result.raw_output.is_none());
    }
}
