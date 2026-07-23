//! # Runtime Adapter Interface
//!
//! Platform-agnostic adapter trait for process/runtime supervision.
//! Platform-specific implementations (Windows, Linux, macOS) provide
//! the actual process management — this trait defines the events
//! that the governance layer consumes.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Process-level state as reported by the platform adapter.
/// These correspond to observable OS-level states, not governance states.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessState {
    /// Process is not running.
    Stopped,
    /// Process is starting (between exec and first health check).
    Starting,
    /// Process is running and healthy.
    Running,
    /// Process is running but degraded (e.g., high memory, slow responses).
    Degraded,
    /// Process has exited unexpectedly.
    Crashed,
    /// Process was blocked from starting.
    Blocked,
}

/// Events emitted by a platform runtime adapter.
/// These are the raw events that get mapped to governance primitives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessEvent {
    /// Start has been requested.
    StartRequested,
    /// Process has started and is accepting signals.
    Started,
    /// Health check passed.
    HealthCheckPassed,
    /// Health check failed (process still running).
    HealthCheckFailed,
    /// Process performance has degraded.
    Degraded,
    /// Stop has been requested.
    StopRequested,
    /// Process has stopped cleanly.
    Stopped,
    /// Process crashed unexpectedly.
    Crashed,
    /// Process was blocked from starting.
    Blocked,
}

impl ProcessEvent {
    /// Convert to the corresponding process state.
    pub fn to_process_state(&self) -> ProcessState {
        match self {
            ProcessEvent::StartRequested => ProcessState::Stopped,
            ProcessEvent::Started => ProcessState::Running,
            ProcessEvent::HealthCheckPassed => ProcessState::Running,
            ProcessEvent::HealthCheckFailed => ProcessState::Running,
            ProcessEvent::Degraded => ProcessState::Degraded,
            ProcessEvent::StopRequested => ProcessState::Running,
            ProcessEvent::Stopped => ProcessState::Stopped,
            ProcessEvent::Crashed => ProcessState::Crashed,
            ProcessEvent::Blocked => ProcessState::Blocked,
        }
    }
}

impl fmt::Display for ProcessEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// The runtime adapter trait. Platform implementations provide this.
///
/// This is intentionally minimal — the adapter translates OS-level events
/// into `ProcessEvent` values that the supervisor maps to governance.
pub trait RuntimeAdapter: Send + Sync {
    /// Start a component. Returns Ok when the process is launched.
    fn start(&self, component_id: &str) -> Result<(), String>;

    /// Stop a component. Returns Ok when the process has exited.
    fn stop(&self, component_id: &str) -> Result<(), String>;

    /// Check if a component is running.
    fn is_running(&self, component_id: &str) -> Result<bool, String>;

    /// Get the current process state.
    fn get_state(&self, component_id: &str) -> Result<ProcessState, String>;
}

/// A no-op adapter for testing and platforms without a runtime.
pub struct NoopAdapter;

impl RuntimeAdapter for NoopAdapter {
    fn start(&self, _component_id: &str) -> Result<(), String> {
        Ok(())
    }

    fn stop(&self, _component_id: &str) -> Result<(), String> {
        Ok(())
    }

    fn is_running(&self, _component_id: &str) -> Result<bool, String> {
        Ok(false)
    }

    fn get_state(&self, _component_id: &str) -> Result<ProcessState, String> {
        Ok(ProcessState::Stopped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_to_state() {
        assert_eq!(ProcessEvent::Started.to_process_state(), ProcessState::Running);
        assert_eq!(ProcessEvent::Stopped.to_process_state(), ProcessState::Stopped);
        assert_eq!(ProcessEvent::Crashed.to_process_state(), ProcessState::Crashed);
        assert_eq!(ProcessEvent::Degraded.to_process_state(), ProcessState::Degraded);
    }

    #[test]
    fn test_noop_adapter() {
        let adapter = NoopAdapter;
        assert!(adapter.start("test").is_ok());
        assert!(adapter.stop("test").is_ok());
        assert!(!adapter.is_running("test").unwrap());
        assert_eq!(adapter.get_state("test").unwrap(), ProcessState::Stopped);
    }
}
