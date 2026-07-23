//! Ledger governance data models — authorization records, receipts, state.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Valid sprint governance states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SprintState {
    /// Sprint has been authorized but not yet begun.
    Authorized,
    /// Sprint is actively being worked.
    Active,
    /// Sprint has been completed, verified, and sealed.
    Sealed,
    /// Sprint was rejected during review.
    Rejected,
    /// Sprint was cancelled before completion.
    Cancelled,
}

impl SprintState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Authorized => "authorized",
            Self::Active => "active",
            Self::Sealed => "sealed",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "authorized" => Some(Self::Authorized),
            "active" => Some(Self::Active),
            "sealed" => Some(Self::Sealed),
            "rejected" => Some(Self::Rejected),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
    /// Whether this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Sealed | Self::Rejected | Self::Cancelled)
    }
}

/// Sprint authorization — proof that work was authorized before execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SprintAuthorization {
    pub authorization_id: String,
    pub sprint_id: String,
    pub epic_id: String,
    pub authorized_at: String,
    pub authorizing_body: String,
    pub scope_summary: String,
    pub content_hash: String,
}

impl SprintAuthorization {
    pub fn compute_content_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.authorization_id.as_bytes());
        h.update(self.sprint_id.as_bytes());
        h.update(self.epic_id.as_bytes());
        h.update(self.authorized_at.as_bytes());
        h.update(self.authorizing_body.as_bytes());
        format!("{:x}", h.finalize())
    }
}

/// Sprint completion receipt — proof that a sprint was completed and sealed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SprintReceipt {
    pub receipt_id: String,
    pub sprint_id: String,
    pub epic_id: String,
    pub previous_state: SprintState,
    pub new_state: SprintState,
    pub completed_at: String,
    pub test_count: usize,
    pub failure_count: usize,
    pub release_build_status: String,
    pub authority_proofs: Vec<String>,
    pub content_hash: String,
}

impl SprintReceipt {
    pub fn compute_content_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.receipt_id.as_bytes());
        h.update(self.sprint_id.as_bytes());
        h.update(self.epic_id.as_bytes());
        h.update(self.new_state.as_str().as_bytes());
        h.update(self.completed_at.as_bytes());
        format!("{:x}", h.finalize())
    }
}

/// Governance receipt — unified record linking authorization to completion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GovernanceReceipt {
    pub authorization: SprintAuthorization,
    pub receipt: Option<SprintReceipt>,
    pub current_state: SprintState,
    pub content_hash: String,
}

impl GovernanceReceipt {
    pub fn compute_content_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.authorization.content_hash.as_bytes());
        if let Some(ref r) = self.receipt {
            h.update(r.content_hash.as_bytes());
        }
        h.update(self.current_state.as_str().as_bytes());
        format!("{:x}", h.finalize())
    }
}

/// Complete ledger snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SprintLedger {
    pub records: Vec<GovernanceReceipt>,
    pub content_hash: String,
}

impl SprintLedger {
    pub fn compute_content_hash(&self) -> String {
        let mut h = Sha256::new();
        for r in &self.records {
            h.update(r.content_hash.as_bytes());
        }
        format!("{:x}", h.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth() -> SprintAuthorization {
        SprintAuthorization {
            authorization_id: "AUTH-01".into(), sprint_id: "S-001".into(),
            epic_id: "EPIC-1".into(), authorized_at: "2026-01-01".into(),
            authorizing_body: "Owner".into(), scope_summary: "Test".into(), content_hash: String::new(),
        }
    }

    #[test] fn test_state_roundtrip() {
        for s in &[SprintState::Authorized, SprintState::Active, SprintState::Sealed, SprintState::Rejected, SprintState::Cancelled] {
            assert_eq!(SprintState::from_str(s.as_str()), Some(s.clone()));
        }
    }

    #[test] fn test_terminal_states() {
        assert!(SprintState::Sealed.is_terminal());
        assert!(SprintState::Rejected.is_terminal());
        assert!(SprintState::Cancelled.is_terminal());
        assert!(!SprintState::Authorized.is_terminal());
    }

    #[test] fn test_auth_hash_deterministic() {
        let a1 = auth(); let a2 = auth();
        assert_eq!(a1.compute_content_hash(), a2.compute_content_hash());
    }

    #[test] fn test_receipt_hash() {
        let r = SprintReceipt {
            receipt_id: "R-1".into(), sprint_id: "S-1".into(), epic_id: "E-1".into(),
            previous_state: SprintState::Active, new_state: SprintState::Sealed,
            completed_at: "2026-01-01".into(), test_count: 100, failure_count: 0,
            release_build_status: "pass".into(), authority_proofs: vec![], content_hash: String::new(),
        };
        let h1 = r.compute_content_hash();
        let h2 = r.compute_content_hash();
        assert_eq!(h1, h2);
    }
}
