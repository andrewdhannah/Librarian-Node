//! # Residency State Contract
//!
//! Operational state of a running instance — orthogonal to lifecycle state.
//!
//! ## Relationship to LifecycleState
//!
//! These two state machines represent independent concerns:
//!
//! | Concern | Type | Question Answered |
//! |---------|------|------------------|
//! | Component trust | `LifecycleState` | Is this component trusted to operate? |
//! | Resource occupation | `ResidencyState` | Is there an active instance consuming resources? |
//!
//! They are orthogonal:
//!
//! - A component can be `LifecycleState::Admitted` (trusted) with
//!   `ResidencyState::Loading` (not yet occupying compute).
//! - A component can be `LifecycleState::Suspended` (not trusted) with
//!   `ResidencyState::Active` (needs forced release).
//! - A component can be `LifecycleState::Operational` (trusted) with
//!   `ResidencyState::Released` (not currently running).
//!
//! ## Enforcement Crossings
//!
//! Rules between the two state machines (not additional states):
//!
//! - `LifecycleState::Operational` permits `ResidencyState::Active`
//! - `LifecycleState::Retired` prohibits transitions to `ResidencyState::Loading`
//! - `LifecycleState::Suspended` requires transition from `Active` to `Releasing`
//!
//! ## Applicability
//!
//! ResidencyState applies to any governed execution component:
//! - Local AI models (model loaded in GPU memory)
//! - Runtime services (daemon occupying a port)
//! - Plugins (extension consuming CPU time)
//! - Future capability providers

use serde::{Deserialize, Serialize};
use std::fmt;

/// Schema version for residency contracts.
pub const RESIDENCY_CONTRACT_VERSION: &str = "1.0.0";

/// Operational state of a running instance.
///
/// Describes resource occupation, not trust status.
/// Use `LifecycleState` for trust/authority concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidencyState {
    /// Instance creation requested but not yet started.
    Requested,
    /// Instance is starting (allocating resources).
    Loading,
    /// Instance is loaded and ready.
    Loaded,
    /// Instance is actively processing.
    Active,
    /// Instance is shutting down (releasing resources).
    Releasing,
    /// Instance has fully stopped. No resources consumed.
    Released,
    /// Instance failed to start or encountered a runtime error.
    Failed,
    /// Instance was blocked from starting (policy or resource constraint).
    Blocked,
}

impl ResidencyState {
    /// All defined residency states.
    pub const ALL: &'static [ResidencyState] = &[
        ResidencyState::Requested,
        ResidencyState::Loading,
        ResidencyState::Loaded,
        ResidencyState::Active,
        ResidencyState::Releasing,
        ResidencyState::Released,
        ResidencyState::Failed,
        ResidencyState::Blocked,
    ];

    /// Valid transitions from this state.
    pub fn valid_transitions(&self) -> &'static [ResidencyState] {
        match self {
            ResidencyState::Requested => &[ResidencyState::Loading, ResidencyState::Blocked],
            ResidencyState::Loading => &[ResidencyState::Loaded, ResidencyState::Failed],
            ResidencyState::Loaded => &[ResidencyState::Active, ResidencyState::Releasing],
            ResidencyState::Active => &[ResidencyState::Releasing, ResidencyState::Failed],
            ResidencyState::Releasing => &[ResidencyState::Released, ResidencyState::Failed],
            ResidencyState::Released => &[ResidencyState::Requested],
            ResidencyState::Failed => &[ResidencyState::Requested, ResidencyState::Releasing],
            ResidencyState::Blocked => &[ResidencyState::Requested],
        }
    }

    /// Whether a transition to `target` is valid.
    pub fn can_transition_to(&self, target: &ResidencyState) -> bool {
        self.valid_transitions().contains(target)
    }

    /// Whether this state represents actively consuming resources.
    pub fn is_occupying_resources(&self) -> bool {
        matches!(
            self,
            ResidencyState::Loading | ResidencyState::Loaded | ResidencyState::Active
        )
    }

    /// Whether this state is a terminal (non-recoverable without external action).
    pub fn is_terminal(&self) -> bool {
        matches!(self, ResidencyState::Released | ResidencyState::Failed | ResidencyState::Blocked)
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            ResidencyState::Requested => "Requested",
            ResidencyState::Loading => "Loading",
            ResidencyState::Loaded => "Loaded",
            ResidencyState::Active => "Active",
            ResidencyState::Releasing => "Releasing",
            ResidencyState::Released => "Released",
            ResidencyState::Failed => "Failed",
            ResidencyState::Blocked => "Blocked",
        }
    }
}

impl fmt::Display for ResidencyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A residency record — tracks the current operational state of a component instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidencyRecord {
    /// Unique identifier for this residency record.
    pub record_id: String,
    /// The component or resource this residency tracks.
    pub component_id: String,
    /// Current residency state.
    pub current_state: ResidencyState,
    /// ISO 8601 timestamp of last state change.
    pub last_transition_at: String,
    /// Node hosting this instance.
    pub host_node: String,
    /// Schema version.
    pub schema_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_states_defined() {
        assert_eq!(ResidencyState::ALL.len(), 8);
    }

    #[test]
    fn test_valid_transitions() {
        // Requested → Loading
        assert!(ResidencyState::Requested.can_transition_to(&ResidencyState::Loading));
        // Requested → Blocked
        assert!(ResidencyState::Requested.can_transition_to(&ResidencyState::Blocked));
        // Loading → Loaded
        assert!(ResidencyState::Loading.can_transition_to(&ResidencyState::Loaded));
        // Loading → Failed
        assert!(ResidencyState::Loading.can_transition_to(&ResidencyState::Failed));
        // Loaded → Active
        assert!(ResidencyState::Loaded.can_transition_to(&ResidencyState::Active));
        // Active → Releasing
        assert!(ResidencyState::Active.can_transition_to(&ResidencyState::Releasing));
        // Releasing → Released
        assert!(ResidencyState::Releasing.can_transition_to(&ResidencyState::Released));
        // Released → Requested (restart cycle)
        assert!(ResidencyState::Released.can_transition_to(&ResidencyState::Requested));
    }

    #[test]
    fn test_invalid_transitions() {
        // Cannot skip Requested → Active
        assert!(!ResidencyState::Requested.can_transition_to(&ResidencyState::Active));
        // Cannot go from Released → Loading (must go through Requested)
        assert!(!ResidencyState::Released.can_transition_to(&ResidencyState::Loading));
        // Cannot go from Loaded → Failed
        assert!(!ResidencyState::Loaded.can_transition_to(&ResidencyState::Failed));
    }

    #[test]
    fn test_resource_occupation() {
        assert!(ResidencyState::Loading.is_occupying_resources());
        assert!(ResidencyState::Loaded.is_occupying_resources());
        assert!(ResidencyState::Active.is_occupying_resources());
        assert!(!ResidencyState::Released.is_occupying_resources());
        assert!(!ResidencyState::Requested.is_occupying_resources());
    }

    #[test]
    fn test_terminal_states() {
        assert!(ResidencyState::Released.is_terminal());
        assert!(ResidencyState::Failed.is_terminal());
        assert!(ResidencyState::Blocked.is_terminal());
        assert!(!ResidencyState::Active.is_terminal());
    }

    #[test]
    fn test_serde_round_trip() {
        let states = ResidencyState::ALL;
        for state in states {
            let json = serde_json::to_string(state).unwrap();
            let deserialized: ResidencyState = serde_json::from_str(&json).unwrap();
            assert_eq!(*state, deserialized, "Failed to round-trip {:?}", state);
        }
    }

    #[test]
    fn test_residency_record() {
        let record = ResidencyRecord {
            record_id: "res-001".into(),
            component_id: "model-phi4".into(),
            current_state: ResidencyState::Active,
            last_transition_at: "2026-07-23T00:00:00Z".into(),
            host_node: "node-windows-1".into(),
            schema_version: RESIDENCY_CONTRACT_VERSION.into(),
        };
        assert_eq!(record.current_state, ResidencyState::Active);
        assert!(record.current_state.is_occupying_resources());

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: ResidencyRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.record_id, deserialized.record_id);
    }

    #[test]
    fn test_labels_not_empty() {
        for state in ResidencyState::ALL {
            assert!(!state.label().is_empty());
        }
    }

    #[test]
    fn test_transition_cycle() {
        // Full lifecycle: Requested → Loading → Loaded → Active → Releasing → Released
        let cycle = vec![
            ResidencyState::Requested,
            ResidencyState::Loading,
            ResidencyState::Loaded,
            ResidencyState::Active,
            ResidencyState::Releasing,
            ResidencyState::Released,
        ];
        for pair in cycle.windows(2) {
            assert!(pair[0].can_transition_to(&pair[1]),
                "Expected valid transition: {:?} → {:?}", pair[0], pair[1]);
        }
    }
}
