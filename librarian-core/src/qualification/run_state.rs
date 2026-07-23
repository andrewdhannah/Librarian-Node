//! Run state — deterministic lifecycle states for qualification runs.
//!
//! The runner progresses through exactly one path:
//! - Received → FixtureResolved → Executing → Completed | RunnerFailed | ModelFailed | RuntimeFailed | Timeout
//!
//! No state implies capability, qualification, or role assignment.

use serde::{Deserialize, Serialize};

/// Deterministic run states for qualification execution.
/// Each state is recorded as lifecycle evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunState {
    /// QualificationRequest received and validated.
    #[serde(rename = "received")]
    Received,
    /// Task pack fixture loaded and hash-verified.
    #[serde(rename = "fixture_resolved")]
    FixtureResolved,
    /// Runtime is loading the model (calling start_run).
    #[serde(rename = "loading_runtime")]
    LoadingRuntime,
    /// Model is executing the prompt (generation in progress).
    #[serde(rename = "executing")]
    Executing,
    /// Run completed successfully (raw output preserved).
    #[serde(rename = "completed")]
    Completed,
    /// Runner infrastructure failure (fixture not found, timeout setup, internal error).
    #[serde(rename = "runner_failed")]
    RunnerFailed,
    /// Model failed to generate (empty output, process crash, HTTP error from runtime).
    #[serde(rename = "model_failed")]
    ModelFailed,
    /// Runtime infrastructure failed (could not load model, port unavailable).
    #[serde(rename = "runtime_failed")]
    RuntimeFailed,
    /// Run exceeded timeout_seconds.
    #[serde(rename = "timeout")]
    Timeout,
}

impl RunState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Received => "received",
            Self::FixtureResolved => "fixture_resolved",
            Self::LoadingRuntime => "loading_runtime",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::RunnerFailed => "runner_failed",
            Self::ModelFailed => "model_failed",
            Self::RuntimeFailed => "runtime_failed",
            Self::Timeout => "timeout",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "received" => Some(Self::Received),
            "fixture_resolved" => Some(Self::FixtureResolved),
            "loading_runtime" => Some(Self::LoadingRuntime),
            "executing" => Some(Self::Executing),
            "completed" => Some(Self::Completed),
            "runner_failed" => Some(Self::RunnerFailed),
            "model_failed" => Some(Self::ModelFailed),
            "runtime_failed" => Some(Self::RuntimeFailed),
            "timeout" => Some(Self::Timeout),
            _ => None,
        }
    }

    /// Whether this state indicates the run completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Whether this state indicates a terminal failure.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::RunnerFailed | Self::ModelFailed | Self::RuntimeFailed | Self::Timeout)
    }

    /// Whether this is a terminal state (success or failure).
    pub fn is_terminal(&self) -> bool {
        self.is_success() || self.is_failure()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_states_have_string_repr() {
        let states = vec![
            RunState::Received,
            RunState::FixtureResolved,
            RunState::LoadingRuntime,
            RunState::Executing,
            RunState::Completed,
            RunState::RunnerFailed,
            RunState::ModelFailed,
            RunState::RuntimeFailed,
            RunState::Timeout,
        ];
        for state in &states {
            let s = state.as_str();
            assert!(!s.is_empty());
            assert_eq!(RunState::from_str(s), Some(state.clone()));
        }
    }

    #[test]
    fn test_success_state() {
        assert!(RunState::Completed.is_success());
        assert!(!RunState::Received.is_success());
        assert!(!RunState::ModelFailed.is_success());
    }

    #[test]
    fn test_failure_states() {
        assert!(RunState::RunnerFailed.is_failure());
        assert!(RunState::ModelFailed.is_failure());
        assert!(RunState::RuntimeFailed.is_failure());
        assert!(RunState::Timeout.is_failure());
        assert!(!RunState::Completed.is_failure());
        assert!(!RunState::Received.is_failure());
    }

    #[test]
    fn test_terminal_states() {
        assert!(RunState::Completed.is_terminal());
        assert!(RunState::RunnerFailed.is_terminal());
        assert!(RunState::ModelFailed.is_terminal());
        assert!(RunState::RuntimeFailed.is_terminal());
        assert!(RunState::Timeout.is_terminal());
        assert!(!RunState::Received.is_terminal());
        assert!(!RunState::FixtureResolved.is_terminal());
        assert!(!RunState::LoadingRuntime.is_terminal());
        assert!(!RunState::Executing.is_terminal());
    }

    #[test]
    fn test_from_str_unknown() {
        assert_eq!(RunState::from_str("unknown"), None);
        assert_eq!(RunState::from_str(""), None);
    }
}
