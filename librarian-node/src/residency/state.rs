//! Eight-state residency machine with validated transitions.
//!
//! States:
//!   Unloaded → Loading → Ready → Running → Draining → Unloading → VerifyingRelease → Unloaded
//!
//! Failure transitions may enter `Failed` from any runtime state where process or
//! verification evidence invalidates the expected transition.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The eight residency states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidencyState {
    /// No model loaded. GPU available for new residency.
    Unloaded,
    /// Model process spawned, waiting for health readiness.
    Loading,
    /// Model healthy and ready for generation requests.
    Ready,
    /// Active generation in progress.
    Running,
    /// Drain initiated. No new generations accepted. Waiting for active generation to complete.
    Draining,
    /// Process termination requested. Waiting for PID exit.
    Unloading,
    /// PID exit verified. Checking GPU memory release.
    VerifyingRelease,
    /// An error condition occurred. Requires explicit recovery.
    Failed,
}

impl ResidencyState {
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

    /// Whether this state indicates the model is potentially GPU-resident.
    pub fn is_potentially_resident(&self) -> bool {
        matches!(
            self,
            Self::Loading | Self::Ready | Self::Running | Self::Draining | Self::Unloading
        )
    }

    /// Whether this state allows new generation requests.
    pub fn allows_generation(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Whether this state is considered "active" for lease enforcement.
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Unloaded | Self::Failed)
    }
}

impl fmt::Display for ResidencyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// How the runtime should be told to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStopStrategy {
    /// POST /health with {"stop":true} — currently unproven for prism.
    GracefulHttp,
    /// TerminateProcess (SIGTERM equivalent on Windows).
    ProcessTerminate,
    /// Kill the process immediately.
    ProcessKill,
}

impl RuntimeStopStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GracefulHttp => "graceful_http",
            Self::ProcessTerminate => "process_terminate",
            Self::ProcessKill => "process_kill",
        }
    }
}

/// Error returned for invalid state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransitionError {
    pub from: ResidencyState,
    pub to: ResidencyState,
    pub reason: &'static str,
}

impl fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid residency transition: {} → {} ({})",
            self.from, self.to, self.reason
        )
    }
}

impl std::error::Error for StateTransitionError {}

/// Legal transitions in the residency state machine.
///
/// Returns Ok(target) if the transition is valid, Err with details if not.
/// `to_failed` allows transition to Failed from any non-Failed state.
pub fn validate_transition(
    from: ResidencyState,
    to: ResidencyState,
) -> Result<ResidencyState, StateTransitionError> {
    // Any state can transition to Failed (error condition)
    if to == ResidencyState::Failed {
        if from == ResidencyState::Failed {
            return Err(StateTransitionError {
                from,
                to,
                reason: "Already in Failed state",
            });
        }
        return Ok(to);
    }

    let valid = match from {
        ResidencyState::Unloaded => matches!(to, ResidencyState::Loading),
        ResidencyState::Loading => {
            matches!(to, ResidencyState::Ready | ResidencyState::Failed)
        }
        ResidencyState::Ready => {
            matches!(
                to,
                ResidencyState::Running | ResidencyState::Draining | ResidencyState::Failed
            )
        }
        ResidencyState::Running => {
            matches!(to, ResidencyState::Ready | ResidencyState::Draining | ResidencyState::Failed)
        }
        ResidencyState::Draining => {
            matches!(
                to,
                ResidencyState::Unloading | ResidencyState::Ready | ResidencyState::Failed
            )
        }
        ResidencyState::Unloading => {
            matches!(
                to,
                ResidencyState::VerifyingRelease | ResidencyState::Failed
            )
        }
        ResidencyState::VerifyingRelease => {
            matches!(
                to,
                ResidencyState::Unloaded | ResidencyState::Failed
            )
        }
        ResidencyState::Failed => {
            // From Failed, only allow recovery to Unloaded
            matches!(to, ResidencyState::Unloaded)
        }
    };

    if valid {
        Ok(to)
    } else {
        Err(StateTransitionError {
            from,
            to,
            reason: "Transition not in legal set",
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legal_happy_path() {
        let transitions = vec![
            (ResidencyState::Unloaded, ResidencyState::Loading),
            (ResidencyState::Loading, ResidencyState::Ready),
            (ResidencyState::Ready, ResidencyState::Running),
            (ResidencyState::Running, ResidencyState::Ready),
            (ResidencyState::Ready, ResidencyState::Draining),
            (ResidencyState::Draining, ResidencyState::Unloading),
            (ResidencyState::Unloading, ResidencyState::VerifyingRelease),
            (ResidencyState::VerifyingRelease, ResidencyState::Unloaded),
        ];

        for (from, to) in transitions {
            assert!(
                validate_transition(from, to).is_ok(),
                "Expected valid: {} → {}",
                from,
                to
            );
        }
    }

    #[test]
    fn test_illegal_transitions() {
        let illegal = vec![
            (ResidencyState::Unloaded, ResidencyState::Ready),
            (ResidencyState::Unloaded, ResidencyState::Running),
            (ResidencyState::Loading, ResidencyState::Running),
            (ResidencyState::Loading, ResidencyState::Draining),
            (ResidencyState::Ready, ResidencyState::Loading),
            (ResidencyState::Running, ResidencyState::Loading),
            (ResidencyState::Draining, ResidencyState::Running),
            (ResidencyState::Unloading, ResidencyState::Running),
            (ResidencyState::VerifyingRelease, ResidencyState::Loading),
        ];

        for (from, to) in illegal {
            assert!(
                validate_transition(from, to).is_err(),
                "Expected illegal: {} → {}",
                from,
                to
            );
        }
    }

    #[test]
    fn test_any_state_can_fail() {
        let states = vec![
            ResidencyState::Unloaded,
            ResidencyState::Loading,
            ResidencyState::Ready,
            ResidencyState::Running,
            ResidencyState::Draining,
            ResidencyState::Unloading,
            ResidencyState::VerifyingRelease,
        ];

        for state in states {
            assert!(
                validate_transition(state, ResidencyState::Failed).is_ok(),
                "Expected {} → Failed to be valid",
                state
            );
        }
    }

    #[test]
    fn test_failed_can_only_recover_to_unloaded() {
        assert!(validate_transition(ResidencyState::Failed, ResidencyState::Unloaded).is_ok());
        assert!(validate_transition(ResidencyState::Failed, ResidencyState::Loading).is_err());
        assert!(validate_transition(ResidencyState::Failed, ResidencyState::Ready).is_err());
    }

    #[test]
    fn test_failed_to_failed_is_illegal() {
        assert!(validate_transition(ResidencyState::Failed, ResidencyState::Failed).is_err());
    }

    #[test]
    fn test_draining_can_go_to_ready() {
        // Draining → Ready is valid: if drain completes and we want to re-ready
        assert!(validate_transition(ResidencyState::Draining, ResidencyState::Ready).is_ok());
    }

    #[test]
    fn test_running_to_ready() {
        // Running → Ready: generation completed, still resident
        assert!(validate_transition(ResidencyState::Running, ResidencyState::Ready).is_ok());
    }

    #[test]
    fn test_potentially_resident() {
        assert!(!ResidencyState::Unloaded.is_potentially_resident());
        assert!(ResidencyState::Loading.is_potentially_resident());
        assert!(ResidencyState::Ready.is_potentially_resident());
        assert!(ResidencyState::Running.is_potentially_resident());
        assert!(ResidencyState::Draining.is_potentially_resident());
        assert!(ResidencyState::Unloading.is_potentially_resident());
        assert!(!ResidencyState::VerifyingRelease.is_potentially_resident());
        assert!(!ResidencyState::Failed.is_potentially_resident());
    }

    #[test]
    fn test_allows_generation() {
        assert!(ResidencyState::Ready.allows_generation());
        assert!(!ResidencyState::Running.allows_generation());
        assert!(!ResidencyState::Draining.allows_generation());
        assert!(!ResidencyState::Loading.allows_generation());
    }
}
