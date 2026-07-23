//! # Lifecycle Cursor Engine
//!
//! Portable lifecycle cursor engine. Advances a project's lifecycle cursor
//! through valid state transitions, recording each transition as evidence.
//!
//! The cursor engine enforces:
//! - Valid transitions only (from the transition table in `LifecycleState`)
//! - Prohibited transitions are rejected (e.g., skipping governance states)
//! - Each transition produces an evidence record
//! - The cursor is persisted to the governance database

use anyhow::Result;
use librarian_contracts::lifecycle::LifecycleState;
use librarian_contracts::prelude::*;

use super::db::GovernanceDb;

/// Errors that can occur during cursor operations.
#[derive(Debug, thiserror::Error)]
pub enum CursorError {
    #[error("Invalid transition: cannot go from {from:?} to {to:?}")]
    InvalidTransition {
        from: LifecycleState,
        to: LifecycleState,
    },
    #[error("Project not found: {0}")]
    ProjectNotFound(String),
    #[error("Transition would skip required governance state")]
    GovernanceSkip,
    #[error("Database error: {0}")]
    Database(String),
}

impl From<anyhow::Error> for CursorError {
    fn from(e: anyhow::Error) -> Self {
        CursorError::Database(e.to_string())
    }
}

/// The lifecycle cursor engine.
pub struct CursorEngine {
    db: GovernanceDb,
}

impl CursorEngine {
    /// Create a new cursor engine backed by the given database.
    pub fn new(db: GovernanceDb) -> Self {
        Self { db }
    }

    /// Get a reference to the database.
    pub fn db(&self) -> &GovernanceDb {
        &self.db
    }

    /// Initialize a lifecycle cursor for a project at the given state.
    pub fn initialize(&self, project_id: &str, initial_state: LifecycleState) -> Result<LifecycleCursor, CursorError> {
        // Check if cursor already exists
        if let Some(_) = self.db.load_cursor(project_id)? {
            return Err(CursorError::Database(
                format!("Cursor already exists for project '{}'", project_id)
            ));
        }

        let now = chrono::Utc::now().to_rfc3339();
        let cursor = LifecycleCursor {
            project_id: project_id.to_string(),
            current_state: initial_state,
            cycle: 1,
            cursor_position: 1,
            last_transition_at: now.clone(),
            last_reconciled_at: None,
            reason: Some("Lifecycle initialized".into()),
            schema_version: LIFECYCLE_CONTRACT_VERSION.into(),
        };

        self.db.save_cursor(&cursor)?;
        Ok(cursor)
    }

    /// Advance the cursor to a new state. Validates the transition first.
    pub fn advance(
        &self,
        project_id: &str,
        target_state: LifecycleState,
        reason: &str,
    ) -> Result<LifecycleCursor, CursorError> {
        let cursor = self.db
            .load_cursor(project_id)?
            .ok_or_else(|| CursorError::ProjectNotFound(project_id.to_string()))?;

        let from = cursor.current_state;
        let to = target_state;

        // Validate transition
        if !from.can_transition_to(&to) {
            return Err(CursorError::InvalidTransition { from, to });
        }

        // Special check: prevent governance skip transitions
        // (transitions that skip intermediate governance states)
        self.check_governance_skip(&from, &to)?;

        let now = chrono::Utc::now().to_rfc3339();
        let new_cursor = LifecycleCursor {
            project_id: project_id.to_string(),
            current_state: to,
            cycle: cursor.cycle,
            cursor_position: cursor.cursor_position + 1,
            last_transition_at: now,
            last_reconciled_at: None,
            reason: Some(reason.to_string()),
            schema_version: LIFECYCLE_CONTRACT_VERSION.into(),
        };

        self.db.save_cursor(&new_cursor)?;

        // Record a custody event for the transition
        let event = CustodyEvent {
            event_id: format!("trans-{}-{}", project_id, new_cursor.cursor_position),
            project_id: project_id.to_string(),
            mcp_session_id: String::new(),
            node_id: "librarian-core".into(),
            window_id: None,
            work_packet_id: None,
            tool_name: "cursor_engine".into(),
            authority_role: CustodyAuthorityRole::System,
            document_reference: format!("lifecycle://{}/cursor", project_id),
            custody_action: CustodyAction::Validate,
            previous_custody_mode: None,
            resulting_custody_mode: None,
            mutation_allowance: None,
            decision_reference: None,
            provenance_receipt: None,
            refusal_reason: None,
            target_project_id: None,
            target_session_id: None,
            target_node_id: None,
            timestamp: new_cursor.last_transition_at.clone(),
        };
        self.db.record_custody_event(&event)?;

        Ok(new_cursor)
    }

    /// Get the current cursor for a project.
    pub fn current(&self, project_id: &str) -> Result<Option<LifecycleCursor>, CursorError> {
        Ok(self.db.load_cursor(project_id)?)
    }

    /// Check whether a transition would skip required governance states.
    fn check_governance_skip(&self, from: &LifecycleState, to: &LifecycleState) -> Result<(), CursorError> {
        // Certain transitions that skip intermediate states are governance violations.
        // Example: Discovered → Operational (skips Candidate + Admitted)
        let skip_pairs = vec![
            (LifecycleState::Discovered, LifecycleState::Operational),
            (LifecycleState::Install, LifecycleState::Ready),
            (LifecycleState::Initialize, LifecycleState::Identity),
        ];
        for (skip_from, skip_to) in skip_pairs {
            if *from == skip_from && *to == skip_to {
                return Err(CursorError::GovernanceSkip);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_engine() -> CursorEngine {
        let db = GovernanceDb::open_in_memory().unwrap();
        CursorEngine::new(db)
    }

    #[test]
    fn test_initialize_cursor() {
        let engine = setup_engine();
        let cursor = engine.initialize("test-project", LifecycleState::Install).unwrap();
        assert_eq!(cursor.current_state, LifecycleState::Install);
        assert_eq!(cursor.cursor_position, 1);
    }

    #[test]
    fn test_advance_valid_transition() {
        let engine = setup_engine();
        engine.initialize("test-project", LifecycleState::Install).unwrap();
        let cursor = engine.advance("test-project", LifecycleState::Initialize, "Moving to init").unwrap();
        assert_eq!(cursor.current_state, LifecycleState::Initialize);
        assert_eq!(cursor.cursor_position, 2);
    }

    #[test]
    fn test_advance_invalid_transition() {
        let engine = setup_engine();
        engine.initialize("test-project", LifecycleState::Install).unwrap();
        let result = engine.advance("test-project", LifecycleState::Operational, "Skip everything");
        assert!(result.is_err());
        match result {
            Err(CursorError::GovernanceSkip) => {} // expected
            _ => panic!("Expected GovernanceSkip error"),
        }
    }

    #[test]
    fn test_duplicate_initialize_rejected() {
        let engine = setup_engine();
        engine.initialize("test-project", LifecycleState::Install).unwrap();
        let result = engine.initialize("test-project", LifecycleState::Install);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_not_found() {
        let engine = setup_engine();
        let result = engine.advance("nonexistent", LifecycleState::Operational, "test");
        assert!(result.is_err());
        match result {
            Err(CursorError::ProjectNotFound(_)) => {} // expected
            _ => panic!("Expected ProjectNotFound error"),
        }
    }
}
