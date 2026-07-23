//! Lifecycle state transitions — validation and enforcement.
//!
//! Only Owner decisions can promote to Approved, Deprecated, or Retired.
//! Evidence, observability, provenance, and review packages have no
//! authority to change lifecycle state.

use super::models::{LifecycleAuthority, LifecycleEvent, LifecycleRecord, LifecycleState};

/// Error returned when a transition is invalid.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionError {
    /// Transition is not allowed by the state machine.
    InvalidTransition {
        from: LifecycleState,
        to: LifecycleState,
        reason: String,
    },
    /// Transition requires Owner decision authority.
    RequiresOwnerDecision {
        from: LifecycleState,
        to: LifecycleState,
    },
    /// Model already in the target state.
    AlreadyInState(LifecycleState),
    /// Record is in a terminal state (Retired).
    TerminalState(LifecycleState),
}

/// A validated lifecycle transition.
#[derive(Debug, Clone)]
pub struct LifecycleTransition {
    /// The event produced by this transition.
    pub event: LifecycleEvent,
    /// The resulting state.
    pub new_state: LifecycleState,
}

impl LifecycleTransition {
    /// Attempt to transition a lifecycle record to a new state.
    ///
    /// Validates:
    /// 1. The transition is allowed by the state machine
    /// 2. The authority is sufficient for the target state
    /// 3. The record is not in a terminal state
    pub fn apply(
        record: &LifecycleRecord,
        target_state: LifecycleState,
        authority: LifecycleAuthority,
        reason: String,
        evidence_refs: Vec<String>,
        review_refs: Vec<String>,
    ) -> Result<Self, TransitionError> {
        let from_state = &record.current_state;

        // Check terminal state
        if *from_state == LifecycleState::Retired {
            return Err(TransitionError::TerminalState(from_state.clone()));
        }

        // Check already in state
        if *from_state == target_state {
            return Err(TransitionError::AlreadyInState(from_state.clone()));
        }

        // Validate transition rules
        Self::validate_transition(from_state, &target_state, &authority)?;

        // Build event
        let now = chrono::Utc::now().to_rfc3339();
        let event_id = LifecycleEvent::compute_event_id(
            &record.model_id,
            Some(from_state),
            &target_state,
            &now,
        );

        let mut event = LifecycleEvent {
            event_id,
            model_id: record.model_id.clone(),
            from_state: Some(from_state.clone()),
            to_state: target_state.clone(),
            authority,
            reason,
            evidence_refs,
            review_refs,
            content_hash: String::new(),
            timestamp: now,
        };

        event.content_hash = event.compute_content_hash();

        Ok(LifecycleTransition {
            event,
            new_state: target_state,
        })
    }

    /// Validate a state transition.
    fn validate_transition(
        from: &LifecycleState,
        to: &LifecycleState,
        authority: &LifecycleAuthority,
    ) -> Result<(), TransitionError> {
        // Allowed transitions and required authorities
        match (from, to) {
            // System transitions (no Owner decision needed)
            (LifecycleState::Discovered, LifecycleState::Candidate) => Ok(()),
            (LifecycleState::Candidate, LifecycleState::Qualified) => Ok(()),
            (LifecycleState::Approved, LifecycleState::Active) => Ok(()),

            // Owner decision required
            (LifecycleState::Qualified, LifecycleState::Approved) => {
                Self::require_owner(from, to, authority)
            }
            (LifecycleState::Active, LifecycleState::Deprecated) => {
                Self::require_owner(from, to, authority)
            }
            (LifecycleState::Deprecated, LifecycleState::Retired) => {
                Self::require_owner(from, to, authority)
            }

            // Any state can go to Deprecated or Retired with Owner decision
            (_, LifecycleState::Deprecated) => Self::require_owner(from, to, authority),
            (_, LifecycleState::Retired) => Self::require_owner(from, to, authority),

            // All other transitions are invalid
            _ => Err(TransitionError::InvalidTransition {
                from: from.clone(),
                to: to.clone(),
                reason: format!("Transition from {:?} to {:?} is not allowed", from, to),
            }),
        }
    }

    fn require_owner(
        from: &LifecycleState,
        to: &LifecycleState,
        authority: &LifecycleAuthority,
    ) -> Result<(), TransitionError> {
        match authority {
            LifecycleAuthority::OwnerDecision(_) => Ok(()),
            _ => Err(TransitionError::RequiresOwnerDecision {
                from: from.clone(),
                to: to.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_record(state: LifecycleState) -> LifecycleRecord {
        LifecycleRecord {
            model_id: "model-a".to_string(),
            model_sha256: "sha256-a".to_string(),
            current_state: state,
            events: vec![],
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
            content_hash: String::new(),
        }
    }

    fn owner_authority() -> LifecycleAuthority {
        LifecycleAuthority::OwnerDecision("dec-001".to_string())
    }

    // LIFE-T1: Discovered → Candidate (system)
    #[test]
    fn test_discovered_to_candidate() {
        let r = test_record(LifecycleState::Discovered);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Candidate,
            LifecycleAuthority::System,
            "Discovery scan".to_string(), vec![], vec![],
        );
        assert!(result.is_ok());
        let t = result.unwrap();
        assert_eq!(t.new_state, LifecycleState::Candidate);
    }

    // LIFE-T2: Candidate → Qualified (system)
    #[test]
    fn test_candidate_to_qualified() {
        let r = test_record(LifecycleState::Candidate);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Qualified,
            LifecycleAuthority::System,
            "Qualification passed".to_string(), vec![], vec![],
        );
        assert!(result.is_ok());
    }

    // LIFE-T3: Qualified → Approved requires Owner
    #[test]
    fn test_qualified_to_approved_requires_owner() {
        let r = test_record(LifecycleState::Qualified);
        // Without Owner — fails
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Approved,
            LifecycleAuthority::System,
            "auto-approve".to_string(), vec![], vec![],
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), TransitionError::RequiresOwnerDecision {
            from: LifecycleState::Qualified,
            to: LifecycleState::Approved,
        });

        // With Owner — succeeds
        let result2 = LifecycleTransition::apply(
            &r, LifecycleState::Approved,
            owner_authority(),
            "Owner approved".to_string(), vec![], vec![],
        );
        assert!(result2.is_ok());
    }

    // LIFE-T4: Approved → Active (system)
    #[test]
    fn test_approved_to_active() {
        let r = test_record(LifecycleState::Approved);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Active,
            LifecycleAuthority::System,
            "Projection created".to_string(), vec![], vec![],
        );
        assert!(result.is_ok());
    }

    // LIFE-T5: Active → Deprecated requires Owner
    #[test]
    fn test_active_to_deprecated_requires_owner() {
        let r = test_record(LifecycleState::Active);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Deprecated,
            LifecycleAuthority::System, "auto".to_string(), vec![], vec![],
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransitionError::RequiresOwnerDecision { .. }));
    }

    // LIFE-T6: Deprecated → Retired requires Owner
    #[test]
    fn test_deprecated_to_retired_requires_owner() {
        let r = test_record(LifecycleState::Deprecated);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Retired,
            LifecycleAuthority::System, "auto".to_string(), vec![], vec![],
        );
        assert!(result.is_err());

        let result2 = LifecycleTransition::apply(
            &r, LifecycleState::Retired,
            owner_authority(), "Owner retired".to_string(), vec![], vec![],
        );
        assert!(result2.is_ok());
    }

    // LIFE-T7: Retired is terminal
    #[test]
    fn test_retired_is_terminal() {
        let r = test_record(LifecycleState::Retired);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Discovered,
            owner_authority(), "revive".to_string(), vec![], vec![],
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransitionError::TerminalState(_)));
    }

    // LIFE-T8: Already in state returns error
    #[test]
    fn test_already_in_state() {
        let r = test_record(LifecycleState::Active);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Active,
            LifecycleAuthority::System, "noop".to_string(), vec![], vec![],
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransitionError::AlreadyInState(_)));
    }

    // LIFE-T9: Invalid transitions
    #[test]
    fn test_invalid_transitions() {
        // Discovered → Approved (skips Candidate, Qualified)
        let r = test_record(LifecycleState::Discovered);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Approved,
            owner_authority(), "skip".to_string(), vec![], vec![],
        );
        assert!(result.is_err());

        // Candidate → Active (skips Qualified, Approved)
        let r2 = test_record(LifecycleState::Candidate);
        let result2 = LifecycleTransition::apply(
            &r2, LifecycleState::Active,
            owner_authority(), "skip".to_string(), vec![], vec![],
        );
        assert!(result2.is_err());
    }

    // LIFE-T10: System cannot bypass Owner to Approved
    #[test]
    fn test_system_cannot_approve() {
        let r = test_record(LifecycleState::Qualified);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Approved,
            LifecycleAuthority::System,
            "System attempt to approve".to_string(), vec![], vec![],
        );
        assert!(result.is_err());
    }

    // LIFE-T11: Event has correct fields after transition
    #[test]
    fn test_event_fields_after_transition() {
        let r = test_record(LifecycleState::Qualified);
        let result = LifecycleTransition::apply(
            &r, LifecycleState::Approved,
            owner_authority(),
            "Owner approved model".to_string(),
            vec!["ev-001".to_string()],
            vec!["rv-001".to_string()],
        ).unwrap();

        assert_eq!(result.event.from_state, Some(LifecycleState::Qualified));
        assert_eq!(result.event.to_state, LifecycleState::Approved);
        assert!(result.event.evidence_refs.contains(&"ev-001".to_string()));
        assert!(result.event.review_refs.contains(&"rv-001".to_string()));
        assert!(!result.event.content_hash.is_empty());
    }
}
