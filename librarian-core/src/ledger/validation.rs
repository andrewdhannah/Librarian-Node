//! Ledger governance validation — allowed sprint state transitions,
//! authorization boundaries, and plan/execution drift detection.

use super::models::SprintState;

/// Error for invalid state transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionError {
    InvalidTransition { from: SprintState, to: SprintState },
    TerminalState(SprintState),
    AlreadyInState,
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => write!(f, "Cannot transition from {:?} to {:?}", from, to),
            Self::TerminalState(s) => write!(f, "Cannot transition from terminal state {:?}", s),
            Self::AlreadyInState => write!(f, "Already in target state"),
        }
    }
}

/// Validates sprint state transitions.
pub struct LedgerValidation;

impl LedgerValidation {
    /// Validates that a state transition is allowed by governance rules.
    pub fn validate_transition(from: &SprintState, to: &SprintState) -> Result<(), TransitionError> {
        if from == to { return Err(TransitionError::AlreadyInState); }
        if from.is_terminal() { return Err(TransitionError::TerminalState(from.clone())); }
        match (from, to) {
            (SprintState::Authorized, SprintState::Active) => Ok(()),
            (SprintState::Active, SprintState::Sealed) => Ok(()),
            (SprintState::Authorized, SprintState::Cancelled) => Ok(()),
            (SprintState::Active, SprintState::Cancelled) => Ok(()),
            _ => Err(TransitionError::InvalidTransition { from: from.clone(), to: to.clone() }),
        }
    }

    /// Detect drift between planned and actual scope.
    /// Returns a list of discrepancies.
    pub fn detect_scope_drift(planned_summary: &str, actual_entry: &str) -> Vec<String> {
        let mut issues = Vec::new();
        if actual_entry.is_empty() {
            issues.push("No actual scope recorded".to_string());
        }
        if planned_summary.len() > actual_entry.len() * 3 {
            issues.push("Actual scope is significantly shorter than planned scope".to_string());
        }
        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_authorized_to_active() {
        assert!(LedgerValidation::validate_transition(&SprintState::Authorized, &SprintState::Active).is_ok());
    }

    #[test] fn test_active_to_sealed() {
        assert!(LedgerValidation::validate_transition(&SprintState::Active, &SprintState::Sealed).is_ok());
    }

    #[test] fn test_sealed_to_active_rejected() {
        let r = LedgerValidation::validate_transition(&SprintState::Sealed, &SprintState::Active);
        assert!(r.is_err());
    }

    #[test] fn test_cancelled() {
        assert!(LedgerValidation::validate_transition(&SprintState::Authorized, &SprintState::Cancelled).is_ok());
        assert!(LedgerValidation::validate_transition(&SprintState::Active, &SprintState::Cancelled).is_ok());
    }

    #[test] fn test_skip_to_sealed_rejected() {
        let r = LedgerValidation::validate_transition(&SprintState::Authorized, &SprintState::Sealed);
        assert!(matches!(r, Err(TransitionError::InvalidTransition { .. })));
    }

    #[test] fn test_already_in_state() {
        let r = LedgerValidation::validate_transition(&SprintState::Sealed, &SprintState::Sealed);
        assert!(matches!(r, Err(TransitionError::AlreadyInState)));
    }

    #[test] fn test_terminal_state() {
        let r = LedgerValidation::validate_transition(&SprintState::Rejected, &SprintState::Active);
        assert!(matches!(r, Err(TransitionError::TerminalState(_))));
    }

    #[test] fn test_scope_drift_detected() {
        let issues = LedgerValidation::detect_scope_drift("Very long planned scope. Multiple pages.", "done");
        assert!(!issues.is_empty());
    }

    #[test] fn test_no_drift() {
        let issues = LedgerValidation::detect_scope_drift("Short scope", "Actual scope done as planned.");
        assert!(issues.is_empty());
    }
}
