use std::path::PathBuf;

use librarian_contracts::registry_apply::{
    ChangeStatus, RegistryStateChange, StateChangeReceipt, TransitionType,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    changes: Vec<RegistryStateChange>,
    receipts: Vec<StateChangeReceipt>,
}

pub struct RegistryApplyService {
    changes: Vec<RegistryStateChange>,
    receipts: Vec<StateChangeReceipt>,
    persistence_path: PathBuf,
}

impl RegistryApplyService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (changes, receipts) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.changes, state.receipts),
                    Err(_) => (Vec::new(), Vec::new()),
                },
                Err(_) => (Vec::new(), Vec::new()),
            }
        } else {
            (Vec::new(), Vec::new())
        };

        RegistryApplyService {
            changes,
            receipts,
            persistence_path,
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            changes: self.changes.clone(),
            receipts: self.receipts.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    fn next_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    fn add_receipt(
        &mut self,
        change_id: &str,
        transition: TransitionType,
        previous_status: &str,
        new_status: &str,
        triggered_by: &str,
    ) -> StateChangeReceipt {
        let receipt = StateChangeReceipt {
            receipt_id: self.next_id(),
            change_id: change_id.to_string(),
            transition,
            previous_status: previous_status.to_string(),
            new_status: new_status.to_string(),
            triggered_by: triggered_by.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.receipts.push(receipt.clone());
        receipt
    }

    pub fn propose_change(
        &mut self,
        target_type: &str,
        target_id: &str,
        proposed_state: serde_json::Value,
    ) -> RegistryStateChange {
        let now = chrono::Utc::now().to_rfc3339();
        let change = RegistryStateChange {
            change_id: self.next_id(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            proposed_state,
            approved_state: None,
            applied_state: None,
            status: ChangeStatus::Proposed,
            receipts: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        self.changes.push(change.clone());
        self.persist();
        change
    }

    pub fn approve_change(&mut self, change_id: &str, triggered_by: &str) -> Option<RegistryStateChange> {
        let idx = self.changes.iter().position(|c| c.change_id == change_id)?;
        let status = self.changes[idx].status.clone();
        if status != ChangeStatus::Proposed {
            return None;
        }

        let previous_status = status.to_string();
        let receipt = self.add_receipt(change_id, TransitionType::Approved, &previous_status, "approved", triggered_by);

        let c = &mut self.changes[idx];
        c.status = ChangeStatus::Approved;
        c.updated_at = chrono::Utc::now().to_rfc3339();
        c.receipts.push(receipt.receipt_id);

        self.persist();
        Some(self.changes[idx].clone())
    }

    pub fn apply_change<F>(
        &mut self,
        change_id: &str,
        triggered_by: &str,
        apply_fn: F,
    ) -> Option<RegistryStateChange>
    where
        F: FnOnce(&RegistryStateChange) -> Result<serde_json::Value, String>,
    {
        let idx = self.changes.iter().position(|c| c.change_id == change_id)?;
        let status = self.changes[idx].status.clone();
        if status != ChangeStatus::Approved {
            return None;
        }

        let change = self.changes[idx].clone();
        match apply_fn(&change) {
            Ok(applied_state) => {
                let previous_status = status.to_string();
                let receipt = self.add_receipt(change_id, TransitionType::Applied, &previous_status, "applied", triggered_by);

                let c = &mut self.changes[idx];
                c.status = ChangeStatus::Applied;
                c.applied_state = Some(applied_state);
                c.updated_at = chrono::Utc::now().to_rfc3339();
                c.receipts.push(receipt.receipt_id);

                self.persist();
                Some(self.changes[idx].clone())
            }
            Err(_err) => {
                let previous_status = status.to_string();
                let receipt = self.add_receipt(change_id, TransitionType::Applied, &previous_status, "failed", triggered_by);

                let c = &mut self.changes[idx];
                c.status = ChangeStatus::Failed;
                c.updated_at = chrono::Utc::now().to_rfc3339();
                c.receipts.push(receipt.receipt_id);

                self.persist();
                None
            }
        }
    }

    pub fn verify_change(&mut self, change_id: &str, triggered_by: &str) -> Option<RegistryStateChange> {
        let idx = self.changes.iter().position(|c| c.change_id == change_id)?;
        let status = self.changes[idx].status.clone();
        if status != ChangeStatus::Applied {
            return None;
        }

        let previous_status = status.to_string();
        let receipt = self.add_receipt(change_id, TransitionType::Verified, &previous_status, "verified", triggered_by);

        let c = &mut self.changes[idx];
        c.status = ChangeStatus::Verified;
        c.updated_at = chrono::Utc::now().to_rfc3339();
        c.receipts.push(receipt.receipt_id);

        self.persist();
        Some(self.changes[idx].clone())
    }

    pub fn reject_change(&mut self, change_id: &str, triggered_by: &str) -> Option<RegistryStateChange> {
        let idx = self.changes.iter().position(|c| c.change_id == change_id)?;
        let status = self.changes[idx].status.clone();
        if status != ChangeStatus::Proposed && status != ChangeStatus::Approved {
            return None;
        }

        let previous_status = status.to_string();
        let receipt = self.add_receipt(change_id, TransitionType::Proposed, &previous_status, "rejected", triggered_by);

        let c = &mut self.changes[idx];
        c.status = ChangeStatus::Rejected;
        c.updated_at = chrono::Utc::now().to_rfc3339();
        c.receipts.push(receipt.receipt_id);

        self.persist();
        Some(self.changes[idx].clone())
    }

    pub fn get_change(&self, change_id: &str) -> Option<RegistryStateChange> {
        self.changes.iter().find(|c| c.change_id == change_id).cloned()
    }

    pub fn get_pending_changes(&self) -> Vec<RegistryStateChange> {
        self.changes
            .iter()
            .filter(|c| c.status == ChangeStatus::Proposed || c.status == ChangeStatus::Approved)
            .cloned()
            .collect()
    }

    pub fn get_change_history(&self) -> Vec<StateChangeReceipt> {
        self.receipts.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup(dir: &tempfile::TempDir) -> RegistryApplyService {
        let path = dir.path().join("registry-apply.json");
        RegistryApplyService::new(path)
    }

    fn propose_test_change(svc: &mut RegistryApplyService) -> RegistryStateChange {
        svc.propose_change(
            "candidate",
            "cand-001",
            serde_json::json!({"status": "admitted"}),
        )
    }

    // APB-2: Propose creates change with "proposed" status
    #[test]
    fn test_propose_creates_proposed() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        assert_eq!(change.status, ChangeStatus::Proposed);
        assert_eq!(change.target_type, "candidate");
        assert_eq!(change.target_id, "cand-001");
        assert_eq!(change.proposed_state, serde_json::json!({"status": "admitted"}));
        assert!(change.approved_state.is_none());
        assert!(change.applied_state.is_none());
        assert!(!change.change_id.is_empty());
        assert!(!change.created_at.is_empty());
    }

    // APB-3: Approve transitions to "approved", does not apply
    #[test]
    fn test_approve_transitions_to_approved() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);

        let approved = svc.approve_change(&change.change_id, "owner-1").unwrap();
        assert_eq!(approved.status, ChangeStatus::Approved);
        assert!(approved.approved_state.is_none());
        assert!(approved.applied_state.is_none());
        assert_eq!(approved.receipts.len(), 1);
    }

    // APB-7: Cannot apply without approval — proposed→applied blocked
    #[test]
    fn test_proposed_to_applied_without_approval_rejected() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);

        let result = svc.apply_change(&change.change_id, "system", |_| {
            Ok(serde_json::json!({"applied": true}))
        });
        assert!(result.is_none(), "Must not allow applying a proposed change");

        let stored = svc.get_change(&change.change_id).unwrap();
        assert_eq!(stored.status, ChangeStatus::Proposed);
    }

    // APB-8: Cannot approve already-applied change
    #[test]
    fn test_cannot_approve_already_applied() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);

        svc.approve_change(&change.change_id, "owner-1");
        svc.apply_change(&change.change_id, "system", |_| {
            Ok(serde_json::json!({"applied": true}))
        });

        let result = svc.approve_change(&change.change_id, "owner-2");
        assert!(result.is_none(), "Must not allow approving an applied change");
    }

    // APB-9: Cannot apply rejected change
    #[test]
    fn test_cannot_apply_rejected() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);

        svc.reject_change(&change.change_id, "owner-1");

        let result = svc.apply_change(&change.change_id, "system", |_| {
            Ok(serde_json::json!({"applied": true}))
        });
        assert!(result.is_none(), "Must not allow applying a rejected change");

        let stored = svc.get_change(&change.change_id).unwrap();
        assert_eq!(stored.status, ChangeStatus::Rejected);
    }

    // APB-4: Apply transitions to "applied", calls service method
    #[test]
    fn test_approve_then_apply_succeeds() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        let approved = svc.approve_change(&change_id, "owner-1").unwrap();
        assert_eq!(approved.status, ChangeStatus::Approved);

        let mut fn_called = false;
        let applied = svc
            .apply_change(&change_id, "system", |c| {
                fn_called = true;
                assert_eq!(c.target_id, "cand-001");
                Ok(serde_json::json!({"applied": true, "target": c.target_id}))
            })
            .unwrap();
        assert!(fn_called, "Apply function must be called");
        assert_eq!(applied.status, ChangeStatus::Applied);
        assert_eq!(
            applied.applied_state,
            Some(serde_json::json!({"applied": true, "target": "cand-001"}))
        );
    }

    // APB-5: Verify confirms change was applied
    #[test]
    fn test_verify_confirms_applied() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        svc.approve_change(&change_id, "owner-1");
        svc.apply_change(&change_id, "system", |_| {
            Ok(serde_json::json!({"applied": true}))
        });

        let verified = svc.verify_change(&change_id, "verifier-1").unwrap();
        assert_eq!(verified.status, ChangeStatus::Verified);
        assert_eq!(verified.receipts.len(), 3);
    }

    // APB-6: Reject transitions to "rejected"
    #[test]
    fn test_reject_proposed_change() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);

        let rejected = svc.reject_change(&change.change_id, "owner-1").unwrap();
        assert_eq!(rejected.status, ChangeStatus::Rejected);
        assert_eq!(rejected.receipts.len(), 1);
    }

    // Full lifecycle: propose → approve → apply → verify
    #[test]
    fn test_full_lifecycle_succeeds() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        let c1 = svc.get_change(&change_id).unwrap();
        assert_eq!(c1.status, ChangeStatus::Proposed);

        svc.approve_change(&change_id, "owner-1");
        let c2 = svc.get_change(&change_id).unwrap();
        assert_eq!(c2.status, ChangeStatus::Approved);

        svc.apply_change(&change_id, "system", |_| {
            Ok(serde_json::json!({"applied": true}))
        });
        let c3 = svc.get_change(&change_id).unwrap();
        assert_eq!(c3.status, ChangeStatus::Applied);

        svc.verify_change(&change_id, "verifier-1");
        let c4 = svc.get_change(&change_id).unwrap();
        assert_eq!(c4.status, ChangeStatus::Verified);
        assert_eq!(c4.receipts.len(), 3);
    }

    // Rejection: propose → reject
    #[test]
    fn test_propose_then_reject() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        svc.reject_change(&change_id, "owner-1");
        let stored = svc.get_change(&change_id).unwrap();
        assert_eq!(stored.status, ChangeStatus::Rejected);

        let history = svc.get_change_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].new_status, "rejected");
    }

    // APB-1: Apply boundary contract types exist (structural test)
    #[test]
    fn test_registry_state_change_has_required_fields() {
        let change = RegistryStateChange {
            change_id: "test-id".to_string(),
            target_type: "candidate".to_string(),
            target_id: "cand-001".to_string(),
            proposed_state: serde_json::json!({"key": "value"}),
            approved_state: None,
            applied_state: None,
            status: ChangeStatus::Proposed,
            receipts: Vec::new(),
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        };
        assert_eq!(change.target_type, "candidate");
        assert_eq!(change.status, ChangeStatus::Proposed);
    }

    #[test]
    fn test_state_change_receipt_has_required_fields() {
        let receipt = StateChangeReceipt {
            receipt_id: "rec-001".to_string(),
            change_id: "change-001".to_string(),
            transition: TransitionType::Approved,
            previous_status: "proposed".to_string(),
            new_status: "approved".to_string(),
            triggered_by: "owner-1".to_string(),
            timestamp: "now".to_string(),
        };
        assert_eq!(receipt.transition, TransitionType::Approved);
        assert_eq!(receipt.previous_status, "proposed");
        assert_eq!(receipt.new_status, "approved");
    }

    // Cannot verify a proposed or approved change
    #[test]
    fn test_cannot_verify_proposed() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);

        let result = svc.verify_change(&change.change_id, "verifier-1");
        assert!(result.is_none());
    }

    #[test]
    fn test_cannot_verify_approved() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        svc.approve_change(&change.change_id, "owner-1");

        let result = svc.verify_change(&change.change_id, "verifier-1");
        assert!(result.is_none());
    }

    // Cannot reject a change that is already applied or verified
    #[test]
    fn test_cannot_reject_applied() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        svc.approve_change(&change_id, "owner-1");
        svc.apply_change(&change_id, "system", |_| Ok(serde_json::json!({"ok": true})));

        let result = svc.reject_change(&change_id, "owner-1");
        assert!(result.is_none());
    }

    // Pending changes returns only proposed and approved
    #[test]
    fn test_get_pending_changes() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);

        let c1 = propose_test_change(&mut svc);
        let c2 = propose_test_change(&mut svc);
        let c3 = propose_test_change(&mut svc);

        svc.approve_change(&c2.change_id, "owner-1");

        let pending = svc.get_pending_changes();
        assert_eq!(pending.len(), 3);
        assert!(pending.iter().any(|c| c.change_id == c1.change_id));
        assert!(pending.iter().any(|c| c.change_id == c2.change_id));
        assert!(pending.iter().any(|c| c.change_id == c3.change_id));
    }

    // Get change history returns all receipts
    #[test]
    fn test_get_change_history() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        svc.approve_change(&change_id, "owner-1");
        svc.apply_change(&change_id, "system", |_| Ok(serde_json::json!({"ok": true})));
        svc.verify_change(&change_id, "verifier-1");

        let history = svc.get_change_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].new_status, "approved");
        assert_eq!(history[1].new_status, "applied");
        assert_eq!(history[2].new_status, "verified");
    }

    // APB-10: Cannot auto-advance proposed to applied — negative test
    #[test]
    fn test_cannot_auto_advance_proposed_to_applied() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        let result = svc.apply_change(&change_id, "system", |_| {
            Ok(serde_json::json!({"applied": true}))
        });
        assert!(result.is_none(), "No code path should allow proposed→applied");

        let stored = svc.get_change(&change_id).unwrap();
        assert_eq!(stored.status, ChangeStatus::Proposed);
    }

    // Apply failure transitions to failed status
    #[test]
    fn test_apply_failure_transitions_to_failed() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        svc.approve_change(&change_id, "owner-1");
        let result = svc.apply_change(&change_id, "system", |_| {
            Err("simulated failure".to_string())
        });
        assert!(result.is_none());

        let stored = svc.get_change(&change_id).unwrap();
        assert_eq!(stored.status, ChangeStatus::Failed);

        let history = svc.get_change_history();
        assert_eq!(history.last().unwrap().new_status, "failed");
    }

    // Reject an approved change (still possible before apply)
    #[test]
    fn test_reject_approved_change() {
        let dir = tempdir().unwrap();
        let mut svc = setup(&dir);
        let change = propose_test_change(&mut svc);
        let change_id = change.change_id.clone();

        svc.approve_change(&change_id, "owner-1");
        let rejected = svc.reject_change(&change_id, "owner-1").unwrap();
        assert_eq!(rejected.status, ChangeStatus::Rejected);
    }

    // Persistence across restarts
    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-apply.json");
        let change_id;

        {
            let mut svc = RegistryApplyService::new(&path);
            let change = propose_test_change(&mut svc);
            change_id = change.change_id.clone();
            svc.approve_change(&change_id, "owner-1");
            svc.apply_change(&change_id, "system", |_| Ok(serde_json::json!({"ok": true})));
        }

        {
            let svc = RegistryApplyService::new(&path);
            let loaded = svc.get_change(&change_id).unwrap();
            assert_eq!(loaded.status, ChangeStatus::Applied);
            assert_eq!(loaded.target_id, "cand-001");

            let history = svc.get_change_history();
            assert_eq!(history.len(), 2);
        }
    }

    // Get change returns None for non-existent
    #[test]
    fn test_get_nonexistent_change() {
        let dir = tempdir().unwrap();
        let svc = setup(&dir);
        assert!(svc.get_change("nonexistent").is_none());
    }
}
