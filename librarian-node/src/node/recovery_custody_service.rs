use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::custody::CustodyMetadata;
use librarian_contracts::recovery_custody::{
    RecoveryAction, RecoveryActionReceipt, RecoveryReport, RecoveryState, RecoveryStatus,
    RecoveryTransition,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::custody_service::CustodyService;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PersistedState {
    status: Option<RecoveryStatus>,
    receipts: Vec<RecoveryActionReceipt>,
    transitions: Vec<RecoveryTransition>,
    actions: Vec<RecoveryAction>,
}

pub struct RecoveryCustodyService {
    persistence_path: PathBuf,
    status: Option<RecoveryStatus>,
    receipts: Vec<RecoveryActionReceipt>,
    transitions: Vec<RecoveryTransition>,
    actions: Vec<RecoveryAction>,
    custody_service: Option<Arc<std::sync::Mutex<CustodyService>>>,
}

impl RecoveryCustodyService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (status, receipts, transitions, actions) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.status, state.receipts, state.transitions, state.actions),
                    Err(_) => (None, Vec::new(), Vec::new(), Vec::new()),
                },
                Err(_) => (None, Vec::new(), Vec::new(), Vec::new()),
            }
        } else {
            (None, Vec::new(), Vec::new(), Vec::new())
        };

        RecoveryCustodyService {
            persistence_path,
            status,
            receipts,
            transitions,
            actions,
            custody_service: None,
        }
    }

    pub fn with_custody(mut self, custody: Arc<std::sync::Mutex<CustodyService>>) -> Self {
        self.custody_service = Some(custody);
        self
    }

    pub fn initiate_recovery(
        &mut self,
        node_id: &str,
        reconciliation_report_id: &str,
    ) -> RecoveryStatus {
        let recovery_id = Uuid::new_v4().to_string();
        let status = RecoveryStatus {
            recovery_id: recovery_id.clone(),
            node_id: node_id.to_string(),
            state: RecoveryState::Reconciling.as_str().to_string(),
            previous_state: Some(RecoveryState::Suspect.as_str().to_string()),
            entered_at: chrono::Utc::now().to_rfc3339(),
            reconciliation_report_id: Some(reconciliation_report_id.to_string()),
        };

        let transition = RecoveryTransition {
            from_state: RecoveryState::Suspect.as_str().to_string(),
            to_state: RecoveryState::Reconciling.as_str().to_string(),
            triggered_by: "initiate_recovery".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.status = Some(status.clone());
        self.transitions.push(transition);
        self.append_to_custody("recovery_initiated", &recovery_id, None);
        self.persist();
        status
    }

    pub fn apply_action(
        &mut self,
        action: RecoveryAction,
        _node_id: &str,
    ) -> Result<RecoveryActionReceipt, String> {
        let status = self
            .status
            .as_ref()
            .map(|s| s.clone())
            .ok_or_else(|| "No active recovery".to_string())?;

        if status.recovery_id != action.recovery_id {
            return Err("Action recovery_id does not match active recovery".to_string());
        }

        if status.state == RecoveryState::Recovered.as_str()
            || status.state == RecoveryState::Failed.as_str()
        {
            return Err(format!(
                "Cannot apply action in terminal state '{}'",
                status.state
            ));
        }

        if action.action_type == "owner_override"
            && status.state != RecoveryState::OwnerReview.as_str()
        {
            return Err("Owner override only allowed in OwnerReview state".to_string());
        }

        let previous_state = status.state.clone();
        let new_state = self.compute_next_state(&action.action_type)?;

        let receipt_id = Uuid::new_v4().to_string();
        let mut receipt = RecoveryActionReceipt {
            receipt_id: receipt_id.clone(),
            action_id: action.action_id.clone(),
            recovery_id: action.recovery_id.clone(),
            action_type: action.action_type.clone(),
            previous_state: previous_state.clone(),
            new_state: new_state.clone(),
            affected_differences: action.affected_differences.clone(),
            evidence_ids: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            custody_envelope_id: None,
        };

        let transition = RecoveryTransition {
            from_state: previous_state.clone(),
            to_state: new_state.clone(),
            triggered_by: format!("action:{}", action.action_type),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.actions.push(action);
        self.receipts.push(receipt.clone());
        self.transitions.push(transition);

        let custody_note = format!(
            "Recovery action '{}' from '{}' to '{}'",
            receipt.action_type, receipt.previous_state, receipt.new_state
        );
        let envelope_id = self.append_to_custody(
            &format!("recovery_action_{}", receipt.action_type),
            &receipt_id,
            Some(custody_note.as_str()),
        );
        receipt.custody_envelope_id = envelope_id;

        let mut updated_status = status;
        updated_status.state = new_state;
        updated_status.previous_state = Some(previous_state);
        self.status = Some(updated_status);

        self.persist();
        Ok(receipt)
    }

    pub fn request_owner_review(&mut self) -> Result<RecoveryStatus, String> {
        let current = self
            .status
            .as_ref()
            .map(|s| s.clone())
            .ok_or_else(|| "No active recovery".to_string())?;

        if current.state != RecoveryState::Reconciling.as_str() {
            return Err(format!(
                "Cannot request owner review from state '{}'; must be reconciling",
                current.state
            ));
        }

        let previous_state = current.state.clone();
        let mut updated = current.clone();
        updated.state = RecoveryState::OwnerReview.as_str().to_string();
        updated.previous_state = Some(previous_state.clone());
        self.status = Some(updated.clone());

        let transition = RecoveryTransition {
            from_state: previous_state,
            to_state: RecoveryState::OwnerReview.as_str().to_string(),
            triggered_by: "request_owner_review".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.transitions.push(transition);

        self.append_to_custody(
            "recovery_owner_review_requested",
            &current.recovery_id,
            None,
        );
        self.persist();
        Ok(updated)
    }

    pub fn complete_recovery(&mut self, decision: &str) -> Result<RecoveryReport, String> {
        let current = self
            .status
            .as_ref()
            .map(|s| s.clone())
            .ok_or_else(|| "No active recovery".to_string())?;

        if current.state != RecoveryState::OwnerReview.as_str()
            && current.state != RecoveryState::Reconciling.as_str()
        {
            return Err(format!(
                "Cannot complete recovery from state '{}'",
                current.state
            ));
        }

        let previous_state = current.state.clone();
        let mut updated = current.clone();
        updated.state = RecoveryState::Recovered.as_str().to_string();
        updated.previous_state = Some(previous_state.clone());
        self.status = Some(updated.clone());

        let transition = RecoveryTransition {
            from_state: previous_state,
            to_state: RecoveryState::Recovered.as_str().to_string(),
            triggered_by: format!("complete_recovery:{}", decision),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.transitions.push(transition);

        self.append_to_custody("recovery_completed", &current.recovery_id, None);

        let report = RecoveryReport {
            report_id: Uuid::new_v4().to_string(),
            recovery_id: current.recovery_id.clone(),
            node_id: current.node_id.clone(),
            actions_taken: self.receipts.clone(),
            state_transitions: self.transitions.clone(),
            summary: format!(
                "Recovery {} completed with decision '{}'. {} action(s) taken.",
                current.recovery_id,
                decision,
                self.receipts.len()
            ),
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        self.persist();
        Ok(report)
    }

    pub fn fail_recovery(&mut self, reason: &str) -> RecoveryReport {
        let recovery_id = self
            .status
            .as_ref()
            .map(|s| s.recovery_id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let node_id = self
            .status
            .as_ref()
            .map(|s| s.node_id.clone())
            .unwrap_or_default();

        let previous_state = self
            .status
            .as_ref()
            .map(|s| s.state.clone())
            .unwrap_or_else(|| RecoveryState::Healthy.as_str().to_string());

        let updated_status = RecoveryStatus {
            recovery_id: recovery_id.clone(),
            node_id: node_id.clone(),
            state: RecoveryState::Failed.as_str().to_string(),
            previous_state: Some(previous_state.clone()),
            entered_at: chrono::Utc::now().to_rfc3339(),
            reconciliation_report_id: self
                .status
                .as_ref()
                .and_then(|s| s.reconciliation_report_id.clone()),
        };
        self.status = Some(updated_status);

        let transition = RecoveryTransition {
            from_state: previous_state,
            to_state: RecoveryState::Failed.as_str().to_string(),
            triggered_by: format!("fail_recovery:{}", reason),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.transitions.push(transition);

        self.append_to_custody("recovery_failed", &recovery_id, None);

        let report = RecoveryReport {
            report_id: Uuid::new_v4().to_string(),
            recovery_id,
            node_id,
            actions_taken: self.receipts.clone(),
            state_transitions: self.transitions.clone(),
            summary: format!("Recovery failed: {}", reason),
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        self.persist();
        report
    }

    pub fn get_status(&self) -> Option<RecoveryStatus> {
        self.status.clone()
    }

    pub fn get_report(&self, recovery_id: &str) -> Option<RecoveryReport> {
        if self
            .status
            .as_ref()
            .map(|s| s.recovery_id.as_str() == recovery_id)
            .unwrap_or(false)
        {
            let status = self.status.as_ref()?;
            Some(RecoveryReport {
                report_id: Uuid::new_v4().to_string(),
                recovery_id: status.recovery_id.clone(),
                node_id: status.node_id.clone(),
                actions_taken: self.receipts.clone(),
                state_transitions: self.transitions.clone(),
                summary: format!(
                    "Recovery {} in state '{}'",
                    status.recovery_id, status.state
                ),
                generated_at: chrono::Utc::now().to_rfc3339(),
            })
        } else {
            None
        }
    }

    // -- private helpers --

    fn compute_next_state(
        &self,
        action_type: &str,
    ) -> Result<String, String> {
        match action_type {
            "accept_sync" => Ok(RecoveryState::Reconciling.as_str().to_string()),
            "reject_change" => Ok(RecoveryState::Reconciling.as_str().to_string()),
            "quarantine" => Ok(RecoveryState::OwnerReview.as_str().to_string()),
            "rollback_request" => Ok(RecoveryState::OwnerReview.as_str().to_string()),
            "owner_override" => Ok(RecoveryState::Recovered.as_str().to_string()),
            _ => Err(format!("Unknown action type '{}'", action_type)),
        }
    }

    fn append_to_custody(
        &mut self,
        receipt_type: &str,
        receipt_id: &str,
        note: Option<&str>,
    ) -> Option<String> {
        if let Some(ref custody) = self.custody_service {
            let node_id = self
                .status
                .as_ref()
                .map(|s| s.node_id.clone())
                .unwrap_or_default();
            let metadata = CustodyMetadata {
                source: "node".to_string(),
                version: "1".to_string(),
                notes: note.map(|s| s.to_string()),
            };
            let mut guard = custody.lock().unwrap();
            let envelope = guard.append_receipt(
                &node_id,
                receipt_type,
                receipt_id,
                serde_json::json!({
                    "recovery_id": self.status.as_ref().map(|s| s.recovery_id.clone()),
                    "receipt_type": receipt_type,
                }),
                Some(metadata),
            );
            return Some(envelope.envelope_id);
        }
        None
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            status: self.status.clone(),
            receipts: self.receipts.clone(),
            transitions: self.transitions.clone(),
            actions: self.actions.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn service_with_custody(dir: &tempfile::TempDir) -> RecoveryCustodyService {
        let custody_path = dir.path().join("custody.json");
        let custody = CustodyService::new(&custody_path);
        let custody_arc = Arc::new(std::sync::Mutex::new(custody));
        let path = dir.path().join("recovery_custody.json");
        RecoveryCustodyService::new(path).with_custody(custody_arc)
    }

    #[test]
    fn test_initiate_recovery_creates_status() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");
        assert_eq!(status.node_id, "test-node");
        assert_eq!(status.state, "reconciling");
        assert_eq!(
            status.reconciliation_report_id,
            Some("report-001".to_string())
        );
        assert!(!status.recovery_id.is_empty());
    }

    #[test]
    fn test_accept_sync_action() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-001".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "accept_sync".to_string(),
            affected_differences: vec!["diff-001".to_string()],
            reason: Some("Accepted differences".to_string()),
        };

        let receipt = service.apply_action(action, "test-node").unwrap();
        assert_eq!(receipt.action_type, "accept_sync");
        assert_eq!(receipt.previous_state, "reconciling");
        assert_eq!(receipt.new_state, "reconciling");
        assert_eq!(receipt.affected_differences, vec!["diff-001"]);
        assert!(receipt.custody_envelope_id.is_some());
    }

    #[test]
    fn test_reject_change_action() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-002".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "reject_change".to_string(),
            affected_differences: vec!["diff-002".to_string()],
            reason: Some("Rejected change".to_string()),
        };

        let receipt = service.apply_action(action, "test-node").unwrap();
        assert_eq!(receipt.action_type, "reject_change");
        assert_eq!(receipt.previous_state, "reconciling");
        assert_eq!(receipt.new_state, "reconciling");
    }

    #[test]
    fn test_quarantine_action_transitions_to_owner_review() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-003".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "quarantine".to_string(),
            affected_differences: vec!["diff-003".to_string()],
            reason: Some("Quarantined".to_string()),
        };

        let receipt = service.apply_action(action, "test-node").unwrap();
        assert_eq!(receipt.action_type, "quarantine");
        assert_eq!(receipt.previous_state, "reconciling");
        assert_eq!(receipt.new_state, "owner_review");
        assert_eq!(service.get_status().unwrap().state, "owner_review");
    }

    #[test]
    fn test_rollback_request_transitions_to_owner_review() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-roll".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "rollback_request".to_string(),
            affected_differences: vec!["diff-roll".to_string()],
            reason: Some("Rollback requested".to_string()),
        };

        let receipt = service.apply_action(action, "test-node").unwrap();
        assert_eq!(receipt.action_type, "rollback_request");
        assert_eq!(receipt.new_state, "owner_review");
    }

    #[test]
    fn test_owner_override_resolves_quarantine() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        // Quarantine first
        let q_action = RecoveryAction {
            action_id: "act-q".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "quarantine".to_string(),
            affected_differences: vec!["diff-q".to_string()],
            reason: Some("Quarantined".to_string()),
        };
        service.apply_action(q_action, "test-node").unwrap();
        assert_eq!(service.get_status().unwrap().state, "owner_review");

        // Owner override
        let o_action = RecoveryAction {
            action_id: "act-o".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "owner_override".to_string(),
            affected_differences: vec!["diff-q".to_string()],
            reason: Some("Owner overrode quarantine".to_string()),
        };
        let receipt = service.apply_action(o_action, "test-node").unwrap();
        assert_eq!(receipt.action_type, "owner_override");
        assert_eq!(receipt.previous_state, "owner_review");
        assert_eq!(receipt.new_state, "recovered");
        assert_eq!(service.get_status().unwrap().state, "recovered");
    }

    #[test]
    fn test_owner_override_not_allowed_from_reconciling() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-bad".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "owner_override".to_string(),
            affected_differences: vec![],
            reason: Some("Bad override".to_string()),
        };

        let result = service.apply_action(action, "test-node");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("only allowed in OwnerReview")
        );
    }

    #[test]
    fn test_request_owner_review() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        service.initiate_recovery("test-node", "report-001");

        let status = service.request_owner_review().unwrap();
        assert_eq!(status.state, "owner_review");
        assert_eq!(status.previous_state, Some("reconciling".to_string()));
        assert_eq!(service.get_status().unwrap().state, "owner_review");
    }

    #[test]
    fn test_complete_recovery_generates_report() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-001".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "accept_sync".to_string(),
            affected_differences: vec!["diff-001".to_string()],
            reason: None,
        };
        service.apply_action(action, "test-node").unwrap();

        service.request_owner_review().unwrap();
        let report = service.complete_recovery("approved").unwrap();
        assert_eq!(report.recovery_id, status.recovery_id);
        assert_eq!(report.node_id, "test-node");
        assert_eq!(report.actions_taken.len(), 1);
        assert!(!report.report_id.is_empty());
        assert!(report.summary.contains("completed"));
    }

    #[test]
    fn test_fail_recovery_generates_failure_report() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        service.initiate_recovery("test-node", "report-001");

        let report = service.fail_recovery("unrecoverable divergence");
        assert_eq!(report.node_id, "test-node");
        assert!(report.summary.contains("failed"));
        assert!(report.summary.contains("unrecoverable divergence"));
        assert_eq!(service.get_status().unwrap().state, "failed");
    }

    #[test]
    fn test_state_transitions_recorded() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-t".to_string(),
            recovery_id: service.get_status().unwrap().recovery_id.clone(),
            action_type: "quarantine".to_string(),
            affected_differences: vec!["diff-t".to_string()],
            reason: None,
        };
        service.apply_action(action, "test-node").unwrap();

        let report = service
            .complete_recovery("owner_approved")
            .unwrap();
        assert_eq!(report.state_transitions.len(), 3);
        assert!(report
            .state_transitions
            .iter()
            .any(|t| t.from_state == "suspect" && t.to_state == "reconciling"));
        assert!(report
            .state_transitions
            .iter()
            .any(|t| t.from_state == "reconciling" && t.to_state == "owner_review"));
        assert!(report
            .state_transitions
            .iter()
            .any(|t| t.from_state == "owner_review" && t.to_state == "recovered"));
    }

    #[test]
    fn test_get_report_returns_none_for_unknown() {
        let dir = tempdir().unwrap();
        let service = service_with_custody(&dir);
        assert!(service.get_report("nonexistent").is_none());
    }

    #[test]
    fn test_get_report_returns_some_for_active() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");
        let report = service.get_report(&status.recovery_id);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report.recovery_id, status.recovery_id);
    }

    #[test]
    fn test_no_action_in_terminal_state() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        // Quarantine -> OwnerReview
        let q = RecoveryAction {
            action_id: "act-q".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "quarantine".to_string(),
            affected_differences: vec![],
            reason: None,
        };
        service.apply_action(q, "test-node").unwrap();

        // Override -> Recovered
        let o = RecoveryAction {
            action_id: "act-o".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "owner_override".to_string(),
            affected_differences: vec![],
            reason: None,
        };
        service.apply_action(o, "test-node").unwrap();
        assert_eq!(service.get_status().unwrap().state, "recovered");

        // Try action in terminal state
        let a = RecoveryAction {
            action_id: "act-after".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "accept_sync".to_string(),
            affected_differences: vec![],
            reason: None,
        };
        let result = service.apply_action(a, "test-node");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("terminal"));
    }

    #[test]
    fn test_receipt_has_custody_envelope() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-env".to_string(),
            recovery_id: status.recovery_id.clone(),
            action_type: "accept_sync".to_string(),
            affected_differences: vec!["diff-env".to_string()],
            reason: None,
        };
        let receipt = service.apply_action(action, "test-node").unwrap();
        assert!(receipt.custody_envelope_id.is_some());
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("recovery_persist.json");
        let custody_path = dir.path().join("custody.json");
        {
            let custody = CustodyService::new(&custody_path);
            let custody_arc = Arc::new(std::sync::Mutex::new(custody));
            let mut service =
                RecoveryCustodyService::new(path.clone()).with_custody(custody_arc);
            let status = service.initiate_recovery("test-node", "report-001");
            assert_eq!(status.state, "reconciling");
        }
        {
            let service = RecoveryCustodyService::new(path.clone());
            let loaded = service.get_status().unwrap();
            assert_eq!(loaded.state, "reconciling");
            assert_eq!(loaded.node_id, "test-node");
        }
    }

    #[test]
    fn test_wrong_recovery_id_rejected() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-wrong".to_string(),
            recovery_id: "wrong-recovery-id".to_string(),
            action_type: "accept_sync".to_string(),
            affected_differences: vec![],
            reason: None,
        };
        let result = service.apply_action(action, "test-node");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_status_none_when_not_started() {
        let dir = tempdir().unwrap();
        let service = service_with_custody(&dir);
        assert!(service.get_status().is_none());
    }

    #[test]
    fn test_direct_complete_recovery_from_reconciling() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.initiate_recovery("test-node", "report-001");
        let report = service.complete_recovery("auto_approved").unwrap();
        assert_eq!(report.recovery_id, status.recovery_id);
        assert_eq!(service.get_status().unwrap().state, "recovered");
    }

    #[test]
    fn test_recovery_never_deletes_history() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        service.initiate_recovery("test-node", "report-001");

        let action = RecoveryAction {
            action_id: "act-hist".to_string(),
            recovery_id: service.get_status().unwrap().recovery_id.clone(),
            action_type: "accept_sync".to_string(),
            affected_differences: vec!["diff-hist".to_string()],
            reason: None,
        };
        service.apply_action(action, "test-node").unwrap();

        let receipt_count_before = service.receipts.len();
        let transition_count_before = service.transitions.len();

        service.complete_recovery("ok").unwrap();
        service.fail_recovery("test fail");

        // History is preserved, not deleted
        assert_eq!(service.receipts.len(), receipt_count_before);
        assert!(service.transitions.len() > transition_count_before);
    }
}
