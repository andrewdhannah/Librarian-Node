//! Ledger store — in-memory store with integrity validation.
//!
//! Maintains a collection of governance receipts and provides
//! integrity checks via deterministic content hashing.

use super::models::{
    GovernanceReceipt, SprintAuthorization, SprintLedger, SprintReceipt, SprintState,
};
use super::validation::LedgerValidation;

/// In-memory ledger store with integrity checking.
pub struct LedgerStore {
    records: Vec<GovernanceReceipt>,
}

impl LedgerStore {
    pub fn new() -> Self { Self { records: vec![] } }

    pub fn from_records(records: Vec<GovernanceReceipt>) -> Self { Self { records } }

    pub fn records(&self) -> &[GovernanceReceipt] { &self.records }

    pub fn add_authorization(&mut self, auth: SprintAuthorization) {
        self.records.push(GovernanceReceipt {
            authorization: auth,
            receipt: None,
            current_state: SprintState::Authorized,
            content_hash: String::new(),
        });
        if let Some(last) = self.records.last_mut() {
            last.content_hash = last.compute_content_hash();
        }
    }

    pub fn activate_sprint(&mut self, sprint_id: &str) -> Result<(), String> {
        let idx = self.records.iter().position(|r| r.authorization.sprint_id == sprint_id);
        match idx {
            Some(i) => {
                let record = &mut self.records[i];
                LedgerValidation::validate_transition(&record.current_state, &SprintState::Active)
                    .map_err(|e| e.to_string())?;
                record.current_state = SprintState::Active;
                record.content_hash = record.compute_content_hash();
                Ok(())
            }
            None => Err(format!("No authorization found for sprint {}", sprint_id)),
        }
    }

    pub fn seal_sprint(
        &mut self,
        receipt: SprintReceipt,
    ) -> Result<(), String> {
        let idx = self.records.iter().position(|r| r.authorization.sprint_id == receipt.sprint_id);
        match idx {
            Some(i) => {
                let record = &mut self.records[i];
                LedgerValidation::validate_transition(&record.current_state, &receipt.new_state)
                    .map_err(|e| e.to_string())?;
                record.receipt = Some(receipt);
                record.current_state = SprintState::Sealed;
                record.content_hash = record.compute_content_hash();
                Ok(())
            }
            None => Err(format!("No authorization found for sprint {}", receipt.sprint_id)),
        }
    }

    pub fn by_sprint(&self, sprint_id: &str) -> Option<&GovernanceReceipt> {
        self.records.iter().find(|r| r.authorization.sprint_id == sprint_id)
    }

    pub fn by_epic(&self, epic_id: &str) -> Vec<&GovernanceReceipt> {
        self.records.iter().filter(|r| r.authorization.epic_id == epic_id).collect()
    }

    pub fn snapshot(&self) -> SprintLedger {
        let mut ledger = SprintLedger {
            records: self.records.clone(),
            content_hash: String::new(),
        };
        ledger.content_hash = ledger.compute_content_hash();
        ledger
    }

    pub fn count(&self) -> usize { self.records.len() }
    pub fn sealed_count(&self) -> usize {
        self.records.iter().filter(|r| r.current_state == SprintState::Sealed).count()
    }
}

impl Default for LedgerStore {
    fn default() -> Self { Self::new() }
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

    fn receipt() -> SprintReceipt {
        SprintReceipt {
            receipt_id: "R-01".into(), sprint_id: "S-001".into(), epic_id: "EPIC-1".into(),
            previous_state: SprintState::Active, new_state: SprintState::Sealed,
            completed_at: "2026-01-02".into(), test_count: 100, failure_count: 0,
            release_build_status: "pass".into(), authority_proofs: vec![], content_hash: String::new(),
        }
    }

    #[test] fn test_empty() { let s = LedgerStore::new(); assert_eq!(s.count(), 0); }

    #[test] fn test_authorize_activate_seal() {
        let mut s = LedgerStore::new();
        s.add_authorization(auth());
        assert_eq!(s.count(), 1);
        assert!(s.activate_sprint("S-001").is_ok());
        assert!(s.seal_sprint(receipt()).is_ok());
        assert_eq!(s.sealed_count(), 1);
    }

    #[test] fn test_snapshot_deterministic() {
        let mut s = LedgerStore::new();
        s.add_authorization(auth());
        let snap1 = s.snapshot().compute_content_hash();
        let snap2 = s.snapshot().compute_content_hash();
        assert_eq!(snap1, snap2);
    }

    #[test] fn test_missing_authorization_error() {
        let mut s = LedgerStore::new();
        let result = s.seal_sprint(receipt());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No authorization"));
    }

    #[test] fn test_by_epic() {
        let mut s = LedgerStore::new();
        s.add_authorization(auth());
        assert_eq!(s.by_epic("EPIC-1").len(), 1);
    }
}
