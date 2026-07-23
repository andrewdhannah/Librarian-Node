//! Runtime run — records a single inference execution under a lease.

use serde::{Deserialize, Serialize};

/// A single inference run executed under a lease.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRun {
    pub run_id: String,
    pub lease_id: String,
    pub packet_id: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub load_duration_ms: Option<i32>,
    pub generation_duration_ms: Option<i32>,
    pub exit_status: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

impl RuntimeRun {
    pub fn new(run_id: String, lease_id: String) -> Self {
        Self {
            run_id,
            lease_id,
            packet_id: None,
            input_tokens: None,
            output_tokens: None,
            load_duration_ms: None,
            generation_duration_ms: None,
            exit_status: None,
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
        }
    }
}
