//! Lifecycle history — durable storage for lifecycle records.
//!
//! Manages lifecycle records for all tracked models. The history is
//! append-only for events — once a transition is recorded, it cannot
//! be modified or removed.

use super::models::{LifecycleRecord, LifecycleState};
use super::transitions::{LifecycleTransition, TransitionError};

/// Lifecycle history manager.
#[derive(Debug, Clone)]
pub struct LifecycleHistory {
    /// All lifecycle records, indexed by model_id.
    records: Vec<LifecycleRecord>,
}

impl LifecycleHistory {
    /// Create an empty history store.
    pub fn new() -> Self {
        Self { records: vec![] }
    }

    /// Create from existing records (e.g., loaded from registry).
    pub fn from_records(records: Vec<LifecycleRecord>) -> Self {
        Self { records }
    }

    /// Get all records.
    pub fn records(&self) -> &[LifecycleRecord] {
        &self.records
    }

    /// Find a record by model_id.
    pub fn find(&self, model_id: &str) -> Option<&LifecycleRecord> {
        self.records.iter().find(|r| r.model_id == model_id)
    }

    /// Find a record by model_id (mutable).
    pub fn find_mut(&mut self, model_id: &str) -> Option<&mut LifecycleRecord> {
        self.records.iter_mut().find(|r| r.model_id == model_id)
    }

    /// Get or create a record for the given model.
    pub fn get_or_create(&mut self, model_id: &str, model_sha256: &str) -> &mut LifecycleRecord {
        let idx = self.records.iter().position(|r| r.model_id == model_id);
        if let Some(i) = idx {
            &mut self.records[i]
        } else {
            self.records.push(LifecycleRecord::new(model_id, model_sha256));
            self.records.last_mut().unwrap()
        }
    }

    /// Apply a validated transition to the history.
    ///
    /// Returns the applied transition if successful.
    /// Records are created on demand for new models.
    pub fn apply_transition(
        &mut self,
        model_id: &str,
        model_sha256: &str,
        target_state: LifecycleState,
        authority: super::models::LifecycleAuthority,
        reason: String,
        evidence_refs: Vec<String>,
        review_refs: Vec<String>,
    ) -> Result<LifecycleTransition, TransitionError> {
        let record = self.get_or_create(model_id, model_sha256);
        let transition = LifecycleTransition::apply(
            record,
            target_state,
            authority,
            reason,
            evidence_refs,
            review_refs,
        )?;

        // Apply the transition
        record.current_state = transition.new_state.clone();
        record.events.push(transition.event.clone());
        record.updated_at = chrono::Utc::now().to_rfc3339();

        Ok(transition)
    }

    /// Get the event timeline for a model.
    pub fn timeline(&self, model_id: &str) -> Vec<&super::models::LifecycleEvent> {
        self.find(model_id)
            .map(|r| r.events.iter().collect())
            .unwrap_or_default()
    }

    /// Count records in a given state.
    pub fn count_by_state(&self, state: &LifecycleState) -> usize {
        self.records.iter().filter(|r| r.current_state == *state).count()
    }

    /// Total number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl Default for LifecycleHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::models::LifecycleAuthority;

    // LIFE-H1: Empty history
    #[test]
    fn test_empty_history() {
        let h = LifecycleHistory::new();
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
    }

    // LIFE-H2: Get or create creates new record
    #[test]
    fn test_get_or_create_creates() {
        let mut h = LifecycleHistory::new();
        let r = h.get_or_create("model-a", "sha256-a");
        assert_eq!(r.model_id, "model-a");
        assert_eq!(r.current_state, LifecycleState::Discovered);
    }

    // LIFE-H3: Get or create returns existing
    #[test]
    fn test_get_or_create_existing() {
        let mut h = LifecycleHistory::new();
        let r1 = h.get_or_create("model-a", "sha256-a");
        r1.current_state = LifecycleState::Approved;
        let r2 = h.get_or_create("model-a", "sha256-a");
        assert_eq!(r2.current_state, LifecycleState::Approved);
    }

    // LIFE-H4: Apply transition succeeds and records event
    #[test]
    fn test_apply_transition() {
        let mut h = LifecycleHistory::new();
        let result = h.apply_transition(
            "model-a", "sha256-a",
            LifecycleState::Candidate,
            LifecycleAuthority::System,
            "Discovery".to_string(), vec![], vec![],
        );
        assert!(result.is_ok());

        let record = h.find("model-a").unwrap();
        assert_eq!(record.current_state, LifecycleState::Candidate);
        assert_eq!(record.events.len(), 1);
    }

    // LIFE-H5: Full lifecycle round-trip
    #[test]
    fn test_full_lifecycle() {
        let mut h = LifecycleHistory::new();
        let mid = "model-full";
        let sha = "sha256-full";
        let owner = LifecycleAuthority::OwnerDecision("dec-001".to_string());

        // Discovered → Candidate
        h.apply_transition(mid, sha, LifecycleState::Candidate, LifecycleAuthority::System,
            "discovered".to_string(), vec![], vec![]).unwrap();
        assert_eq!(h.find(mid).unwrap().current_state, LifecycleState::Candidate);

        // Candidate → Qualified
        h.apply_transition(mid, sha, LifecycleState::Qualified, LifecycleAuthority::System,
            "qualified".to_string(), vec!["ev-001".to_string()], vec![]).unwrap();
        assert_eq!(h.find(mid).unwrap().current_state, LifecycleState::Qualified);

        // Qualified → Approved (Owner)
        h.apply_transition(mid, sha, LifecycleState::Approved, owner.clone(),
            "approved".to_string(), vec![], vec!["rv-001".to_string()]).unwrap();
        assert_eq!(h.find(mid).unwrap().current_state, LifecycleState::Approved);

        // Approved → Active
        h.apply_transition(mid, sha, LifecycleState::Active, LifecycleAuthority::System,
            "activated".to_string(), vec![], vec![]).unwrap();
        assert_eq!(h.find(mid).unwrap().current_state, LifecycleState::Active);

        // Active → Deprecated (Owner)
        h.apply_transition(mid, sha, LifecycleState::Deprecated, owner.clone(),
            "deprecated".to_string(), vec![], vec![]).unwrap();
        assert_eq!(h.find(mid).unwrap().current_state, LifecycleState::Deprecated);

        // Deprecated → Retired (Owner)
        h.apply_transition(mid, sha, LifecycleState::Retired, owner.clone(),
            "retired".to_string(), vec![], vec![]).unwrap();
        assert_eq!(h.find(mid).unwrap().current_state, LifecycleState::Retired);

        // 6 events in timeline
        assert_eq!(h.timeline(mid).len(), 6);
    }

    // LIFE-H6: Count by state
    #[test]
    fn test_count_by_state() {
        let mut h = LifecycleHistory::new();

        // m1: Discovered → Candidate
        h.apply_transition("m1", "s1", LifecycleState::Candidate, LifecycleAuthority::System,
            "".to_string(), vec![], vec![]).unwrap();
        assert_eq!(h.count_by_state(&LifecycleState::Candidate), 1);

        // m2: Discovered → Candidate → Qualified
        h.apply_transition("m2", "s2", LifecycleState::Candidate, LifecycleAuthority::System,
            "".to_string(), vec![], vec![]).unwrap();
        h.apply_transition("m2", "s2", LifecycleState::Qualified, LifecycleAuthority::System,
            "".to_string(), vec![], vec![]).unwrap();

        assert_eq!(h.count_by_state(&LifecycleState::Candidate), 1);
        assert_eq!(h.count_by_state(&LifecycleState::Qualified), 1);
    }

    // LIFE-H7: Authority boundary — system cannot approve
    #[test]
    fn test_system_cannot_approve_in_history() {
        let mut h = LifecycleHistory::new();
        h.apply_transition("m1", "s1", LifecycleState::Candidate, LifecycleAuthority::System,
            "".to_string(), vec![], vec![]).unwrap();
        h.apply_transition("m1", "s1", LifecycleState::Qualified, LifecycleAuthority::System,
            "".to_string(), vec![], vec![]).unwrap();

        let result = h.apply_transition("m1", "s1", LifecycleState::Approved,
            LifecycleAuthority::System, "auto".to_string(), vec![], vec![]);
        assert!(result.is_err());
    }
}
