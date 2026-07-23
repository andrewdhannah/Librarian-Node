//! # Lifecycle Contract Types
//!
//! Lifecycle states, cursors, and transitions for the Librarian platform.
//! Maps to Swift lifecycle cursor model (LIFECYCLE-PLATFORM-CONTRACT.md).
//!
//! This module defines states only — it does not implement transitions.
//! Transition logic belongs in the implementation layer (librarian-core).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Schema version for lifecycle contracts.
pub const LIFECYCLE_CONTRACT_VERSION: &str = "1.1.0";

// ============================================================================
// Core Lifecycle States
// ============================================================================

/// Core lifecycle phases for a Librarian component.
///
/// Maps to the lifecycle cursor phases in LIFECYCLE-PLATFORM-CONTRACT.md.
/// The lifecycle flows: INSTALL → INITIALIZE → QUALIFY → IDENTITY → READY
/// → DISCOVERED → CANDIDATE → ADMITTED → OPERATIONAL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// Software installed on the target system.
    Install,
    /// First-run initialization complete.
    Initialize,
    /// Hardware and environment qualification passed.
    Qualify,
    /// Node identity generated and registered.
    Identity,
    /// Ready for discovery (accepts incoming connections).
    Ready,
    /// Discovered by the Librarian Core or another node.
    Discovered,
    /// Under evaluation for admission into the platform.
    Candidate,
    /// Admitted into the platform. May begin operations.
    Admitted,
    /// Fully operational. Authorized for production workloads.
    Operational,
    /// Suspended for maintenance or investigation.
    Suspended,
    /// Retired from the platform. No longer operational.
    Retired,
}

impl LifecycleState {
    /// All defined lifecycle states.
    pub const ALL: &'static [LifecycleState] = &[
        LifecycleState::Install,
        LifecycleState::Initialize,
        LifecycleState::Qualify,
        LifecycleState::Identity,
        LifecycleState::Ready,
        LifecycleState::Discovered,
        LifecycleState::Candidate,
        LifecycleState::Admitted,
        LifecycleState::Operational,
        LifecycleState::Suspended,
        LifecycleState::Retired,
    ];

    /// Valid transitions from this state.
    /// Returns the set of states that can be transitioned to.
    pub fn valid_transitions(&self) -> &'static [LifecycleState] {
        match self {
            LifecycleState::Install => &[LifecycleState::Initialize],
            LifecycleState::Initialize => &[LifecycleState::Qualify],
            LifecycleState::Qualify => &[LifecycleState::Identity],
            LifecycleState::Identity => &[LifecycleState::Ready],
            LifecycleState::Ready => &[LifecycleState::Discovered],
            LifecycleState::Discovered => &[LifecycleState::Candidate],
            LifecycleState::Candidate => &[LifecycleState::Admitted, LifecycleState::Suspended],
            LifecycleState::Admitted => &[LifecycleState::Operational, LifecycleState::Suspended],
            LifecycleState::Operational => &[LifecycleState::Suspended, LifecycleState::Retired],
            LifecycleState::Suspended => &[LifecycleState::Candidate, LifecycleState::Admitted, LifecycleState::Retired],
            LifecycleState::Retired => &[],
        }
    }

    /// Whether a transition from `self` to `target` is valid.
    pub fn can_transition_to(&self, target: &LifecycleState) -> bool {
        self.valid_transitions().contains(target)
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            LifecycleState::Install => "Install",
            LifecycleState::Initialize => "Initialize",
            LifecycleState::Qualify => "Qualify",
            LifecycleState::Identity => "Identity",
            LifecycleState::Ready => "Ready",
            LifecycleState::Discovered => "Discovered",
            LifecycleState::Candidate => "Candidate",
            LifecycleState::Admitted => "Admitted",
            LifecycleState::Operational => "Operational",
            LifecycleState::Suspended => "Suspended",
            LifecycleState::Retired => "Retired",
        }
    }

    /// Whether this state is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, LifecycleState::Retired)
    }

    /// Whether this state is a suspension state.
    pub fn is_suspended(&self) -> bool {
        matches!(self, LifecycleState::Suspended)
    }
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ============================================================================
// Lifecycle Cursor
// ============================================================================

/// A lifecycle cursor records the current position in the lifecycle state machine.
/// Maps to Swift `lifecycle-cursor.json` structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleCursor {
    /// The project or component this cursor belongs to.
    pub project_id: String,
    /// Current lifecycle state.
    pub current_state: LifecycleState,
    /// Current cycle number (for cyclic lifecycles).
    pub cycle: u32,
    /// Cursor position counter (increments on each transition).
    pub cursor_position: u64,
    /// ISO 8601 timestamp of last transition.
    pub last_transition_at: String,
    /// ISO 8601 timestamp of last reconciliation.
    pub last_reconciled_at: Option<String>,
    /// Reason for the current state.
    pub reason: Option<String>,
    /// Schema version.
    pub schema_version: String,
}

/// A single lifecycle transition record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleTransition {
    /// Source state before transition.
    pub from_state: LifecycleState,
    /// Target state after transition.
    pub to_state: LifecycleState,
    /// ISO 8601 timestamp.
    pub transitioned_at: String,
    /// Reason for the transition.
    pub reason: Option<String>,
    /// Who or what initiated the transition.
    pub initiated_by: Option<String>,
}

/// A lifecycle branch state (a-z).
/// Used for parallel tracks within a phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchState(pub char);

impl BranchState {
    /// Create a new branch state. Validates a-z.
    pub fn new(c: char) -> Result<Self, String> {
        if c.is_ascii_lowercase() {
            Ok(BranchState(c))
        } else {
            Err(format!("Branch state must be a-z, got '{}'", c))
        }
    }
}

impl fmt::Display for BranchState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Governance stages (for presentation purposes).
/// Maps internal phase numbers to qualified labels per
/// LIFECYCLE-CURSOR-PRESENTATION.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceStage(pub u32);

impl GovernanceStage {
    /// Create a governance stage.
    pub fn new(n: u32) -> Self {
        GovernanceStage(n)
    }
}

impl fmt::Display for GovernanceStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Governance Stage: {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        // Valid: Install -> Initialize
        assert!(LifecycleState::Install.can_transition_to(&LifecycleState::Initialize));
        // Invalid: Install -> Operational (skips too many)
        assert!(!LifecycleState::Install.can_transition_to(&LifecycleState::Operational));
        // Terminal: Retired has no outgoing transitions
        assert!(LifecycleState::Retired.valid_transitions().is_empty());
    }

    #[test]
    fn test_suspended_not_terminal() {
        assert!(!LifecycleState::Suspended.is_terminal());
        assert!(LifecycleState::Retired.is_terminal());
    }

    #[test]
    fn test_branch_state_valid() {
        assert!(BranchState::new('a').is_ok());
        assert!(BranchState::new('z').is_ok());
        assert!(BranchState::new('A').is_err());
        assert!(BranchState::new('1').is_err());
    }

    #[test]
    fn test_lifecycle_cursor_serde() {
        let cursor = LifecycleCursor {
            project_id: "librarian".into(),
            current_state: LifecycleState::Operational,
            cycle: 1,
            cursor_position: 37,
            last_transition_at: "2026-07-23T00:00:00Z".into(),
            last_reconciled_at: None,
            reason: Some("Initial deployment".into()),
            schema_version: LIFECYCLE_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string(&cursor).unwrap();
        let deserialized: LifecycleCursor = serde_json::from_str(&json).unwrap();
        assert_eq!(cursor.project_id, deserialized.project_id);
        assert_eq!(cursor.current_state, deserialized.current_state);
    }

    #[test]
    fn test_all_states_have_labels() {
        for state in LifecycleState::ALL {
            assert!(!state.label().is_empty());
        }
    }
}
