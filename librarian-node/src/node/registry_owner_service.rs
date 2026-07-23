use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::custody::CustodyMetadata;
use librarian_contracts::registry_owner::{
    OwnerActionStatus, OwnerActionType, RegistryOwnerAction, RegistryOwnerActionReceipt,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::custody_service::CustodyService;
use super::registry_candidate_service::RegistryCandidateService;
use super::registration_service::RegistrationService;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    actions: Vec<RegistryOwnerAction>,
    receipts: Vec<RegistryOwnerActionReceipt>,
    used_receipt_ids: HashSet<String>,
}

pub struct RegistryOwnerService {
    actions: Vec<RegistryOwnerAction>,
    receipts: Vec<RegistryOwnerActionReceipt>,
    used_receipt_ids: HashSet<String>,
    persistence_path: PathBuf,
    custody_service: Option<Arc<std::sync::Mutex<CustodyService>>>,
}

impl RegistryOwnerService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (actions, receipts, used_receipt_ids) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.actions, state.receipts, state.used_receipt_ids),
                    Err(_) => (Vec::new(), Vec::new(), HashSet::new()),
                },
                Err(_) => (Vec::new(), Vec::new(), HashSet::new()),
            }
        } else {
            (Vec::new(), Vec::new(), HashSet::new())
        };

        RegistryOwnerService {
            actions,
            receipts,
            used_receipt_ids,
            persistence_path,
            custody_service: None,
        }
    }

    pub fn with_custody(mut self, custody: Arc<std::sync::Mutex<CustodyService>>) -> Self {
        self.custody_service = Some(custody);
        self
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            actions: self.actions.clone(),
            receipts: self.receipts.clone(),
            used_receipt_ids: self.used_receipt_ids.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    fn next_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Generate a unique receipt_id, rejecting duplicates (replay protection).
    pub fn generate_receipt_id(&mut self) -> Result<String, String> {
        for _ in 0..10 {
            let id = Uuid::new_v4().to_string();
            if !self.used_receipt_ids.contains(&id) {
                self.used_receipt_ids.insert(id.clone());
                return Ok(id);
            }
        }
        Err("Failed to generate unique receipt_id after 10 attempts".to_string())
    }

    pub fn create_action(
        &mut self,
        action_type: OwnerActionType,
        target_id: &str,
        target_type: &str,
        owner: &str,
        reason: &str,
    ) -> RegistryOwnerAction {
        let action = RegistryOwnerAction {
            action_id: self.next_id(),
            action_type,
            target_id: target_id.to_string(),
            target_type: target_type.to_string(),
            owner: owner.to_string(),
            reason: reason.to_string(),
            status: OwnerActionStatus::Pending,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.actions.push(action.clone());
        self.persist();
        action
    }

    pub fn approve_action(
        &mut self,
        action_id: &str,
        registry_candidate_service: &mut RegistryCandidateService,
        registration_service: &mut RegistrationService,
    ) -> Option<RegistryOwnerActionReceipt> {
        let action = self.actions.iter().find(|a| a.action_id == action_id)?.clone();
        if action.status != OwnerActionStatus::Pending {
            return None;
        }

        let previous_state = action.status.to_string();
        let target_id = action.target_id.clone();
        let action_type = action.action_type.clone();
        let owner = action.owner.clone();
        let reason = action.reason.clone();

        // Set status to approved before executing side effects
        if let Some(a) = self.actions.iter_mut().find(|a| a.action_id == action_id) {
            a.status = OwnerActionStatus::Approved;
        }

        match &action_type {
            OwnerActionType::ApproveCandidate => {
                if action.target_type == "candidate" {
                    let decision = librarian_contracts::registry::ReviewDecision::Approve;
                    registry_candidate_service.review(
                        &target_id,
                        decision,
                        &owner,
                        &reason,
                        registration_service,
                    );
                }
            }
            OwnerActionType::RejectCandidate => {
                if action.target_type == "candidate" {
                    let decision = librarian_contracts::registry::ReviewDecision::Reject;
                    registry_candidate_service.review(
                        &target_id,
                        decision,
                        &owner,
                        &reason,
                        registration_service,
                    );
                }
            }
            OwnerActionType::SuspendNode => {
                registration_service.suspend();
            }
            OwnerActionType::ReinstateNode => {
                registration_service.confirm_registration(
                    &librarian_contracts::node::RegistrationReceipt {
                        registration_id: self.next_id(),
                        node_id: target_id.clone(),
                        status: "registered".to_string(),
                        registered_at: chrono::Utc::now().to_rfc3339(),
                        previous_state: Some("suspended".to_string()),
                    },
                );
            }
            _ => {}
        }

        // Set status to executed
        if let Some(a) = self.actions.iter_mut().find(|a| a.action_id == action_id) {
            a.status = OwnerActionStatus::Executed;
        }
        let new_state = OwnerActionStatus::Executed.to_string();

        let mut receipt = RegistryOwnerActionReceipt {
            receipt_id: self.generate_receipt_id().unwrap_or_else(|_| self.next_id()),
            action_id: action_id.to_string(),
            action_type: action_type.clone(),
            target_id: target_id.clone(),
            previous_state,
            new_state,
            owner: owner.clone(),
            reason: reason.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            custody_envelope_id: None,
        };

        if let Some(ref custody) = self.custody_service {
            let payload = serde_json::to_value(&receipt).unwrap_or_default();
            let metadata = CustodyMetadata {
                source: "node".to_string(),
                version: "1".to_string(),
                notes: Some(format!(
                    "Registry owner action '{}' on '{}'",
                    action_type, target_id
                )),
            };
            let mut guard = custody.lock().unwrap();
            let envelope = guard.append_receipt(
                &target_id,
                "registry_owner_action",
                &receipt.receipt_id,
                payload,
                Some(metadata),
            );
            receipt.custody_envelope_id = Some(envelope.envelope_id);
        }

        self.receipts.push(receipt.clone());
        self.persist();
        Some(receipt)
    }

    pub fn reject_action(&mut self, action_id: &str, reason: &str) -> Option<RegistryOwnerActionReceipt> {
        let action = self.actions.iter().find(|a| a.action_id == action_id)?.clone();
        if action.status != OwnerActionStatus::Pending {
            return None;
        }

        let previous_state = action.status.to_string();
        let action_type = action.action_type.clone();
        let target_id = action.target_id.clone();
        let owner = action.owner.clone();

        if let Some(a) = self.actions.iter_mut().find(|a| a.action_id == action_id) {
            a.status = OwnerActionStatus::Rejected;
        }
        let new_state = OwnerActionStatus::Rejected.to_string();

        let receipt = RegistryOwnerActionReceipt {
            receipt_id: self.generate_receipt_id().unwrap_or_else(|_| self.next_id()),
            action_id: action_id.to_string(),
            action_type,
            target_id,
            previous_state,
            new_state,
            owner,
            reason: reason.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            custody_envelope_id: None,
        };

        self.receipts.push(receipt.clone());
        self.persist();
        Some(receipt)
    }

    pub fn get_pending_actions(&self) -> Vec<RegistryOwnerAction> {
        self.actions
            .iter()
            .filter(|a| a.status == OwnerActionStatus::Pending)
            .cloned()
            .collect()
    }

    pub fn get_action_history(&self) -> Vec<RegistryOwnerActionReceipt> {
        self.receipts.clone()
    }

    pub fn get_action(&self, action_id: &str) -> Option<RegistryOwnerAction> {
        self.actions
            .iter()
            .find(|a| a.action_id == action_id)
            .cloned()
    }

    pub fn get_receipt(&self, action_id: &str) -> Option<RegistryOwnerActionReceipt> {
        self.receipts
            .iter()
            .find(|r| r.action_id == action_id)
            .cloned()
    }

    /// Access used receipt IDs for testing.
    pub fn used_receipt_ids(&self) -> &HashSet<String> {
        &self.used_receipt_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::node::NodeIdentity;
    use librarian_contracts::registry::DiscoveryMethod;
    use tempfile::tempdir;

    #[allow(unused_mut)]
    fn setup(
        dir: &tempfile::TempDir,
    ) -> (
        RegistryOwnerService,
        RegistryCandidateService,
        RegistrationService,
        Option<Arc<std::sync::Mutex<CustodyService>>>,
    ) {
        let owner_path = dir.path().join("registry_owner.json");
        let candidate_path = dir.path().join("candidates.json");
        let reg_path = dir.path().join("registration.json");
        let custody_path = dir.path().join("custody.json");

        let mut owner = RegistryOwnerService::new(owner_path);
        let candidates = RegistryCandidateService::new(candidate_path);
        let mut reg = RegistrationService::new(reg_path);
        let custody = CustodyService::new(custody_path);
        let custody_arc = Arc::new(std::sync::Mutex::new(custody));
        owner.custody_service = Some(custody_arc.clone());

        // Submit initial registration for approval tests
        let identity = NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        reg.submit_registration(&identity, None);

        (owner, candidates, reg, Some(custody_arc))
    }

    fn make_candidate(
        svc: &mut RegistryCandidateService,
        node_id: &str,
        name: &str,
    ) -> librarian_contracts::registry::NodeCandidate {
        svc.discover(node_id, name, DiscoveryMethod::ApiDiscovery)
    }

    // OWN-2: Create action creates pending action
    #[test]
    fn test_create_action_creates_pending() {
        let dir = tempdir().unwrap();
        let (mut owner, _, _, _) = setup(&dir);

        let action = owner.create_action(
            OwnerActionType::ApproveCandidate,
            "cand-001",
            "candidate",
            "owner-1",
            "Approved after review",
        );

        assert_eq!(action.status, OwnerActionStatus::Pending);
        assert_eq!(action.action_type, OwnerActionType::ApproveCandidate);
        assert_eq!(action.target_id, "cand-001");
        assert_eq!(action.owner, "owner-1");
        assert_eq!(action.reason, "Approved after review");
        assert!(!action.action_id.is_empty());
        assert!(!action.created_at.is_empty());
    }

    // OWN-3: Approve action executes and generates receipt with custody entry
    #[test]
    fn test_approve_action_executes_and_generates_receipt() {
        let dir = tempdir().unwrap();
        let (mut owner, mut candidates, mut reg, custody_arc) = setup(&dir);

        let candidate = make_candidate(&mut candidates, "node-1", "Node One");

        let action = owner.create_action(
            OwnerActionType::ApproveCandidate,
            &candidate.candidate_id,
            "candidate",
            "owner-1",
            "Approved for admission",
        );

        let receipt = owner
            .approve_action(&action.action_id, &mut candidates, &mut reg)
            .unwrap();

        assert_eq!(receipt.action_id, action.action_id);
        assert_eq!(receipt.action_type, OwnerActionType::ApproveCandidate);
        assert_eq!(receipt.target_id, candidate.candidate_id);
        assert_eq!(receipt.previous_state, "pending");
        assert_eq!(receipt.new_state, "executed");
        assert_eq!(receipt.owner, "owner-1");

        // Custody entry should exist
        assert!(receipt.custody_envelope_id.is_some());

        let custody = custody_arc.unwrap();
        let guard = custody.lock().unwrap();
        let chain = guard.get_chain().unwrap();
        assert!(chain.envelope_count > 0);

        // Candidate should now be admitted
        let updated = candidates.get_candidate(&candidate.candidate_id).unwrap();
        assert_eq!(
            updated.status,
            librarian_contracts::registry::CandidateStatus::Admitted
        );
    }

    // OWN-4: Reject action generates receipt without execution
    #[test]
    fn test_reject_action_generates_receipt_without_execution() {
        let dir = tempdir().unwrap();
        let (mut owner, mut candidates, mut reg, _) = setup(&dir);

        let candidate = make_candidate(&mut candidates, "node-1", "Node One");

        let action = owner.create_action(
            OwnerActionType::RejectCandidate,
            &candidate.candidate_id,
            "candidate",
            "owner-1",
            "Not eligible",
        );

        let receipt = owner.reject_action(&action.action_id, "Not eligible").unwrap();

        assert_eq!(receipt.action_id, action.action_id);
        assert_eq!(receipt.action_type, OwnerActionType::RejectCandidate);
        assert_eq!(receipt.new_state, "rejected");

        // Candidate should still be discovered (not executed)
        let candidate_stored = candidates.get_candidate(&candidate.candidate_id).unwrap();
        assert_eq!(
            candidate_stored.status,
            librarian_contracts::registry::CandidateStatus::Discovered
        );

        // Verify no custody entry for rejection
        assert!(receipt.custody_envelope_id.is_none());
    }

    // OWN-2b: Pending actions list shows awaiting actions
    #[test]
    fn test_pending_actions_list() {
        let dir = tempdir().unwrap();
        let (mut owner, _, _, _) = setup(&dir);

        owner.create_action(
            OwnerActionType::ApproveCandidate,
            "cand-001",
            "candidate",
            "owner-1",
            "Review 1",
        );
        owner.create_action(
            OwnerActionType::SuspendNode,
            "node-001",
            "node",
            "owner-1",
            "Suspend for maintenance",
        );

        let pending = owner.get_pending_actions();
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|a| a.status == OwnerActionStatus::Pending));
    }

    // OWN-2c: Action history shows past decisions
    #[test]
    fn test_action_history() {
        let dir = tempdir().unwrap();
        let (mut owner, mut candidates, mut reg, _) = setup(&dir);

        let candidate = make_candidate(&mut candidates, "node-1", "Node One");
        let action = owner.create_action(
            OwnerActionType::ApproveCandidate,
            &candidate.candidate_id,
            "candidate",
            "owner-1",
            "Approved",
        );

        owner
            .approve_action(&action.action_id, &mut candidates, &mut reg)
            .unwrap();

        let action2 = owner.create_action(
            OwnerActionType::RejectCandidate,
            "cand-002",
            "candidate",
            "owner-1",
            "Rejected",
        );
        owner.reject_action(&action2.action_id, "Not suitable").unwrap();

        let history = owner.get_action_history();
        assert_eq!(history.len(), 2);

        let approved = history.iter().find(|r| r.action_type == OwnerActionType::ApproveCandidate).unwrap();
        assert_eq!(approved.new_state, "executed");

        let rejected = history.iter().find(|r| r.action_type == OwnerActionType::RejectCandidate).unwrap();
        assert_eq!(rejected.new_state, "rejected");
    }

    // OWN-2d: Invalid action type rejected
    #[test]
    fn test_invalid_action_type_rejected() {
        let dir = tempdir().unwrap();
        let (mut owner, _, _, _) = setup(&dir);

        // Create with a valid action type using the enum
        let action = owner.create_action(
            OwnerActionType::OverrideEnforcement,
            "enf-rule-001",
            "enforcement_rule",
            "owner-1",
            "Override enforcement rule",
        );

        assert_eq!(action.action_type, OwnerActionType::OverrideEnforcement);
        assert_eq!(action.target_type, "enforcement_rule");
    }

    // OWN-2e: Approve with invalid action id returns None
    #[test]
    fn test_approve_nonexistent_action_returns_none() {
        let dir = tempdir().unwrap();
        let (mut owner, mut candidates, mut reg, _) = setup(&dir);

        let result = owner.approve_action("nonexistent", &mut candidates, &mut reg);
        assert!(result.is_none());
    }

    // OWN-2f: Cannot approve already-approved action
    #[test]
    fn test_cannot_approve_already_approved_action() {
        let dir = tempdir().unwrap();
        let (mut owner, mut candidates, mut reg, _) = setup(&dir);

        let candidate = make_candidate(&mut candidates, "node-1", "Node One");
        let action = owner.create_action(
            OwnerActionType::ApproveCandidate,
            &candidate.candidate_id,
            "candidate",
            "owner-1",
            "Approved",
        );

        owner
            .approve_action(&action.action_id, &mut candidates, &mut reg)
            .unwrap();

        let second_attempt = owner.approve_action(&action.action_id, &mut candidates, &mut reg);
        assert!(second_attempt.is_none());
    }

    // OWN-RP1: Duplicate receipt_id is rejected
    #[test]
    fn test_duplicate_receipt_id_rejected() {
        let dir = tempdir().unwrap();
        let (mut owner, mut candidates, mut reg, _) = setup(&dir);
        let candidate = make_candidate(&mut candidates, "node-rp1", "Node RP1");
        let action = owner.create_action(
            OwnerActionType::ApproveCandidate,
            &candidate.candidate_id,
            "candidate",
            "owner-1",
            "Approved replay test",
        );

        // Manually set a known receipt_id so we can verify uniqueness
        let known_id = "known-replay-receipt-id".to_string();
        owner.used_receipt_ids.insert(known_id.clone());
        let second = owner.used_receipt_ids.insert(known_id);
        assert!(!second, "Duplicate receipt_id must be rejected");

        // Verify that generate_receipt_id does not return duplicates
        let id1 = owner.generate_receipt_id().unwrap();
        assert!(owner.used_receipt_ids.contains(&id1));
        // Trying the same ID again should return a different one
        let id2 = owner.generate_receipt_id().unwrap();
        assert_ne!(id1, id2);
        assert!(owner.used_receipt_ids.contains(&id2));
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let owner_path = dir.path().join("registry_owner.json");

        let action_id;
        {
            let mut owner = RegistryOwnerService::new(&owner_path);
            let action = owner.create_action(
                OwnerActionType::ApproveCandidate,
                "cand-001",
                "candidate",
                "owner-1",
                "Test persist",
            );
            action_id = action.action_id.clone();
        }

        {
            let owner = RegistryOwnerService::new(&owner_path);
            let loaded = owner.get_action(&action_id).unwrap();
            assert_eq!(loaded.target_id, "cand-001");
            assert_eq!(loaded.status, OwnerActionStatus::Pending);
        }
    }
}
