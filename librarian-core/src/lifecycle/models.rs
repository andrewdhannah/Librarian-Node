//! Lifecycle data models — states, events, records.
//!
//! Each model has explicit authority tracking. Evidence, observability,
//! provenance, and review packages cannot create lifecycle events.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Lifecycle state for a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LifecycleState {
    /// Model file discovered but not yet qualified.
    Discovered,
    /// Queued for qualification.
    Candidate,
    /// Passed qualification pipeline (evidence exists).
    Qualified,
    /// Owner-approved for routing.
    Approved,
    /// Currently routable (projection exists).
    Active,
    /// No longer recommended; existing routes may complete.
    Deprecated,
    /// Permanently removed from routing consideration.
    Retired,
}

impl LifecycleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Candidate => "candidate",
            Self::Qualified => "qualified",
            Self::Approved => "approved",
            Self::Active => "active",
            Self::Deprecated => "deprecated",
            Self::Retired => "retired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "discovered" => Some(Self::Discovered),
            "candidate" => Some(Self::Candidate),
            "qualified" => Some(Self::Qualified),
            "approved" => Some(Self::Approved),
            "active" => Some(Self::Active),
            "deprecated" => Some(Self::Deprecated),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}

/// Who or what authorized a lifecycle state transition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LifecycleAuthority {
    /// Owner decision (decision_id reference).
    OwnerDecision(String),
    /// Automatic system action (model discovery, migration rules).
    System,
}

/// A single lifecycle state transition event (immutable once recorded).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifecycleEvent {
    /// Unique event ID (deterministic).
    pub event_id: String,

    /// Model ID.
    pub model_id: String,

    /// Previous state (None for initial event).
    pub from_state: Option<LifecycleState>,

    /// New state.
    pub to_state: LifecycleState,

    /// Authority that authorized this transition.
    pub authority: LifecycleAuthority,

    /// Human-readable reason.
    pub reason: String,

    /// References to qualification evidence.
    pub evidence_refs: Vec<String>,

    /// References to review packages.
    pub review_refs: Vec<String>,

    /// Content hash for tamper detection.
    pub content_hash: String,

    /// When the event occurred.
    pub timestamp: String,
}

impl LifecycleEvent {
    /// Compute a deterministic event ID.
    pub fn compute_event_id(
        model_id: &str,
        from_state: Option<&LifecycleState>,
        to_state: &LifecycleState,
        timestamp: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(model_id.as_bytes());
        if let Some(fs) = from_state {
            hasher.update(fs.as_str().as_bytes());
        }
        hasher.update(b"->");
        hasher.update(to_state.as_str().as_bytes());
        hasher.update(b"@");
        hasher.update(timestamp.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute content hash for this event.
    pub fn compute_content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.event_id.as_bytes());
        hasher.update(self.model_id.as_bytes());
        if let Some(fs) = &self.from_state {
            hasher.update(fs.as_str().as_bytes());
        }
        hasher.update(self.to_state.as_str().as_bytes());
        hasher.update(self.reason.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Complete lifecycle record for a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifecycleRecord {
    /// Model ID.
    pub model_id: String,

    /// Model SHA-256.
    pub model_sha256: String,

    /// Current lifecycle state.
    pub current_state: LifecycleState,

    /// Immutable, append-only event list.
    pub events: Vec<LifecycleEvent>,

    /// When the record was created.
    pub created_at: String,

    /// When the record was last updated.
    pub updated_at: String,

    /// Content hash.
    pub content_hash: String,
}

impl LifecycleRecord {
    /// Create a new lifecycle record for a model at Discovered state.
    pub fn new(model_id: &str, model_sha256: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            model_id: model_id.to_string(),
            model_sha256: model_sha256.to_string(),
            current_state: LifecycleState::Discovered,
            events: vec![],
            created_at: now.clone(),
            updated_at: now,
            content_hash: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // LIFE-U1: LifecycleState string round-trip
    #[test]
    fn test_state_string_roundtrip() {
        let states = [
            LifecycleState::Discovered,
            LifecycleState::Candidate,
            LifecycleState::Qualified,
            LifecycleState::Approved,
            LifecycleState::Active,
            LifecycleState::Deprecated,
            LifecycleState::Retired,
        ];
        for s in &states {
            assert_eq!(LifecycleState::from_str(s.as_str()), Some(s.clone()));
        }
        assert_eq!(LifecycleState::from_str("unknown"), None);
    }

    // LIFE-U2: Event ID is deterministic
    #[test]
    fn test_event_id_deterministic() {
        let id1 = LifecycleEvent::compute_event_id("m1", Some(&LifecycleState::Discovered), &LifecycleState::Candidate, "2026-01-01");
        let id2 = LifecycleEvent::compute_event_id("m1", Some(&LifecycleState::Discovered), &LifecycleState::Candidate, "2026-01-01");
        assert_eq!(id1, id2);
    }

    // LIFE-U3: Event ID changes with different inputs
    #[test]
    fn test_event_id_changes() {
        let id1 = LifecycleEvent::compute_event_id("m1", Some(&LifecycleState::Discovered), &LifecycleState::Candidate, "2026-01-01");
        let id2 = LifecycleEvent::compute_event_id("m2", Some(&LifecycleState::Discovered), &LifecycleState::Candidate, "2026-01-01");
        assert_ne!(id1, id2);
    }

    // LIFE-U4: Content hash is deterministic
    #[test]
    fn test_content_hash_deterministic() {
        let e = LifecycleEvent {
            event_id: "evt-001".to_string(),
            model_id: "m1".to_string(),
            from_state: Some(LifecycleState::Discovered),
            to_state: LifecycleState::Candidate,
            authority: LifecycleAuthority::System,
            reason: "Model discovered".to_string(),
            evidence_refs: vec![],
            review_refs: vec![],
            content_hash: String::new(),
            timestamp: "2026-01-01".to_string(),
        };
        let h1 = e.compute_content_hash();
        let h2 = e.compute_content_hash();
        assert_eq!(h1, h2);
    }

    // LIFE-U5: LifecycleRecord creation
    #[test]
    fn test_record_creation() {
        let r = LifecycleRecord::new("model-a", "sha256-a");
        assert_eq!(r.current_state, LifecycleState::Discovered);
        assert!(r.events.is_empty());
    }
}
