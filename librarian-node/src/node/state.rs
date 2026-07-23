use librarian_contracts::node::NodeState;
use std::fmt;

#[derive(Debug, Clone)]
pub struct StateTransitionError {
    pub from: NodeState,
    pub to: NodeState,
    pub reason: String,
}

impl fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid transition from {:?} to {:?}: {}",
            self.from, self.to, self.reason
        )
    }
}

impl std::error::Error for StateTransitionError {}

pub struct NodeStateMachine {
    current: NodeState,
    last_change: String,
}

impl NodeStateMachine {
    pub fn new() -> Self {
        NodeStateMachine {
            current: NodeState::Unregistered,
            last_change: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn current(&self) -> &NodeState {
        &self.current
    }

    pub fn last_change(&self) -> &str {
        &self.last_change
    }

    pub fn transition(&mut self, to: NodeState) -> Result<(), StateTransitionError> {
        validate_transition(&self.current, &to)?;
        self.current = to;
        self.last_change = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn set_state(&mut self, state: NodeState) {
        self.current = state;
        self.last_change = chrono::Utc::now().to_rfc3339();
    }
}

pub fn validate_transition(from: &NodeState, to: &NodeState) -> Result<(), StateTransitionError> {
    let allowed = match (from, to) {
        // Registration lifecycle
        (NodeState::Unregistered, NodeState::RegistrationRequested) => true,
        (NodeState::RegistrationRequested, NodeState::Registered) => true,
        (NodeState::Registered, NodeState::Suspended) => true,
        (NodeState::Registered, NodeState::Retired) => true,
        (NodeState::Suspended, NodeState::Registered) => true,
        // Error / recovery
        (NodeState::Unregistered, NodeState::Failed) => true,
        (NodeState::RegistrationRequested, NodeState::Failed) => true,
        (NodeState::Registered, NodeState::Failed) => true,
        (NodeState::Suspended, NodeState::Failed) => true,
        (NodeState::Retired, NodeState::Failed) => true,
        (NodeState::Failed, NodeState::Unregistered) => true,
        // Future state transitions (reserved, not yet active)
        (NodeState::Registered, NodeState::Connected) => true,
        (NodeState::Connected, NodeState::Authorized) => true,
        (NodeState::Connected, NodeState::Failed) => true,
        (NodeState::Connected, NodeState::Registered) => true,
        (NodeState::Authorized, NodeState::Executing) => true,
        (NodeState::Authorized, NodeState::Failed) => true,
        (NodeState::Authorized, NodeState::Registered) => true,
        (NodeState::Executing, NodeState::EvidencePending) => true,
        (NodeState::Executing, NodeState::Failed) => true,
        (NodeState::EvidencePending, NodeState::Reconciling) => true,
        (NodeState::EvidencePending, NodeState::Failed) => true,
        (NodeState::Reconciling, NodeState::Registered) => true,
        (NodeState::Reconciling, NodeState::Failed) => true,
        _ => false,
    };

    if allowed {
        Ok(())
    } else {
        Err(StateTransitionError {
            from: from.clone(),
            to: to.clone(),
            reason: format!("No valid transition from {:?} to {:?}", from, to),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_unregistered() {
        let sm = NodeStateMachine::new();
        assert_eq!(*sm.current(), NodeState::Unregistered);
    }

    #[test]
    fn test_valid_transition_unregistered_to_registration_requested() {
        let mut sm = NodeStateMachine::new();
        assert!(sm.transition(NodeState::RegistrationRequested).is_ok());
        assert_eq!(*sm.current(), NodeState::RegistrationRequested);
    }

    #[test]
    fn test_valid_transition_registration_requested_to_registered() {
        let mut sm = NodeStateMachine::new();
        sm.set_state(NodeState::RegistrationRequested);
        assert!(sm.transition(NodeState::Registered).is_ok());
        assert_eq!(*sm.current(), NodeState::Registered);
    }

    #[test]
    fn test_valid_transition_registered_to_suspended() {
        let mut sm = NodeStateMachine::new();
        sm.set_state(NodeState::Registered);
        assert!(sm.transition(NodeState::Suspended).is_ok());
        assert_eq!(*sm.current(), NodeState::Suspended);
    }

    #[test]
    fn test_valid_transition_suspended_to_registered() {
        let mut sm = NodeStateMachine::new();
        sm.set_state(NodeState::Suspended);
        assert!(sm.transition(NodeState::Registered).is_ok());
        assert_eq!(*sm.current(), NodeState::Registered);
    }

    #[test]
    fn test_valid_transition_registered_to_retired() {
        let mut sm = NodeStateMachine::new();
        sm.set_state(NodeState::Registered);
        assert!(sm.transition(NodeState::Retired).is_ok());
        assert_eq!(*sm.current(), NodeState::Retired);
    }

    #[test]
    fn test_valid_transition_unregistered_to_failed() {
        let mut sm = NodeStateMachine::new();
        assert!(sm.transition(NodeState::Failed).is_ok());
    }

    #[test]
    fn test_valid_transition_failed_to_unregistered() {
        let mut sm = NodeStateMachine::new();
        sm.set_state(NodeState::Failed);
        assert!(sm.transition(NodeState::Unregistered).is_ok());
    }

    #[test]
    fn test_invalid_transition_unregistered_to_registered() {
        let mut sm = NodeStateMachine::new();
        // Must go through RegistrationRequested first
        assert!(sm.transition(NodeState::Registered).is_err());
    }

    #[test]
    fn test_invalid_transition_unregistered_to_executing() {
        let mut sm = NodeStateMachine::new();
        assert!(sm.transition(NodeState::Executing).is_err());
    }

    #[test]
    fn test_invalid_transition_registered_to_unregistered() {
        let mut sm = NodeStateMachine::new();
        sm.set_state(NodeState::Registered);
        assert!(sm.transition(NodeState::Unregistered).is_err());
    }

    #[test]
    fn test_invalid_transition_registration_requested_to_suspended() {
        let mut sm = NodeStateMachine::new();
        sm.set_state(NodeState::RegistrationRequested);
        assert!(sm.transition(NodeState::Suspended).is_err());
    }

    #[test]
    fn test_set_state_skips_validation() {
        let mut sm = NodeStateMachine::new();
        // set_state bypasses validation
        sm.set_state(NodeState::Executing);
        assert_eq!(*sm.current(), NodeState::Executing);
    }

    #[test]
    fn test_last_change_updates() {
        let mut sm = NodeStateMachine::new();
        let first = sm.last_change().to_string();
        std::thread::sleep(std::time::Duration::from_millis(10));
        sm.transition(NodeState::RegistrationRequested).unwrap();
        let second = sm.last_change().to_string();
        assert_ne!(first, second);
    }
}
