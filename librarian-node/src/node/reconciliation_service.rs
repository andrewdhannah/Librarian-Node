use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::custody::CustodyMetadata;
use librarian_contracts::reconciliation::{
    ClassifiedDifference, ConflictSeverity, ReconciliationConfig, ReconciliationDecision,
    ReconciliationReceipt, ReconciliationReport, ReconciliationRequest,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::custody_service::CustodyService;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReconciliationPhase {
    Offline,
    Reconnecting,
    Comparing,
    Validating,
    Reviewing,
    Accepting,
    Exception,
    Quarantined,
}

impl ReconciliationPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReconciliationPhase::Offline => "offline",
            ReconciliationPhase::Reconnecting => "reconnecting",
            ReconciliationPhase::Comparing => "comparing",
            ReconciliationPhase::Validating => "validating",
            ReconciliationPhase::Reviewing => "reviewing",
            ReconciliationPhase::Accepting => "accepting",
            ReconciliationPhase::Exception => "exception",
            ReconciliationPhase::Quarantined => "quarantined",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationStatus {
    pub phase: String,
    pub current_request_id: Option<String>,
    pub current_report_id: Option<String>,
    pub pending_differences: u32,
    pub resolved_differences: u32,
    pub total_receipts: u32,
    pub node_id: String,
}

#[derive(Debug)]
pub enum ReconciliationError {
    CustodyUnavailable(String),
    IntegrityCheckFailed(String),
    LkgReferenceNotFound(String),
    ComparisonFailed(String),
    InvalidPhaseTransition { from: String, to: String },
    DuplicateDecision(String),
    UnknownDifference(String),
    PersistenceFailed(String),
    OwnerWorkflowDenied(String),
}

impl std::fmt::Display for ReconciliationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReconciliationError::CustodyUnavailable(msg) => {
                write!(f, "Custody unavailable: {}", msg)
            }
            ReconciliationError::IntegrityCheckFailed(msg) => {
                write!(f, "Integrity check failed: {}", msg)
            }
            ReconciliationError::LkgReferenceNotFound(msg) => {
                write!(f, "LKG reference not found: {}", msg)
            }
            ReconciliationError::ComparisonFailed(msg) => write!(f, "Comparison failed: {}", msg),
            ReconciliationError::InvalidPhaseTransition { from, to } => {
                write!(f, "Invalid phase transition from {} to {}", from, to)
            }
            ReconciliationError::DuplicateDecision(msg) => {
                write!(f, "Duplicate decision: {}", msg)
            }
            ReconciliationError::UnknownDifference(msg) => {
                write!(f, "Unknown difference: {}", msg)
            }
            ReconciliationError::PersistenceFailed(msg) => {
                write!(f, "Persistence failed: {}", msg)
            }
            ReconciliationError::OwnerWorkflowDenied(msg) => {
                write!(f, "Owner workflow denied: {}", msg)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    receipts: Vec<ReconciliationReceipt>,
    config: ReconciliationConfig,
    quarantined_differences: Vec<ClassifiedDifference>,
}

pub struct ReconciliationService {
    persistence_path: PathBuf,
    receipts: Vec<ReconciliationReceipt>,
    current_request: Option<ReconciliationRequest>,
    current_report: Option<ReconciliationReport>,
    pending_differences: Vec<ClassifiedDifference>,
    resolved_difference_ids: Vec<String>,
    config: ReconciliationConfig,
    quarantined_differences: Vec<ClassifiedDifference>,
    phase: ReconciliationPhase,
    node_id: String,
    custody_service: Option<Arc<std::sync::Mutex<CustodyService>>>,
}

impl ReconciliationService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (receipts, config, quarantined_differences) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.receipts, state.config, state.quarantined_differences),
                    Err(_) => (Vec::new(), ReconciliationConfig::default(), Vec::new()),
                },
                Err(_) => (Vec::new(), ReconciliationConfig::default(), Vec::new()),
            }
        } else {
            (Vec::new(), ReconciliationConfig::default(), Vec::new())
        };

        ReconciliationService {
            persistence_path,
            receipts,
            current_request: None,
            current_report: None,
            pending_differences: Vec::new(),
            resolved_difference_ids: Vec::new(),
            config,
            quarantined_differences,
            phase: ReconciliationPhase::Offline,
            node_id: String::new(),
            custody_service: None,
        }
    }

    pub fn with_custody(mut self, custody: Arc<std::sync::Mutex<CustodyService>>) -> Self {
        self.custody_service = Some(custody);
        self
    }

    /// Begin a new reconciliation cycle. Returns a ReconciliationRequest and
    /// produces a `reconciliation_started` receipt in the custody chain.
    pub fn initiate_reconciliation(
        &mut self,
        node_id: &str,
        initiated_by: &str,
    ) -> Result<ReconciliationRequest, ReconciliationError> {
        self.node_id = node_id.to_string();
        self.phase = ReconciliationPhase::Reconnecting;

        let custody_hash = self
            .custody_service
            .as_ref()
            .map(|c| {
                let guard = c.lock().unwrap();
                guard
                    .get_chain()
                    .map(|ch| ch.last_chain_hash)
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let request = ReconciliationRequest {
            reconciliation_id: Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            lkg_reference: custody_hash,
            initiated_at: chrono::Utc::now().to_rfc3339(),
            initiated_by: initiated_by.to_string(),
            phase: "reconnecting".to_string(),
        };

        self.current_request = Some(request.clone());
        let receipt = self.make_receipt(
            &request.reconciliation_id,
            "reconciliation_started",
            None,
            &[],
            serde_json::to_value(&request).unwrap_or_default(),
        );
        self.append_to_custody(&receipt, None);
        self.phase = ReconciliationPhase::Comparing;
        self.persist();
        Ok(request)
    }

    /// Compare local state against an expected_state JSON payload.
    /// Expected state should contain `sessions`, `registration_status`,
    /// `custody_envelopes` fields for comparison.
    pub fn compare_state(
        &mut self,
        request: &ReconciliationRequest,
        expected_state: serde_json::Value,
    ) -> ReconciliationReport {
        self.phase = ReconciliationPhase::Comparing;
        let mut differences = Vec::new();

        if let Some(expected_sessions) = expected_state.get("sessions").and_then(|v| v.as_array())
        {
            let local_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
            let expected_ids: std::collections::HashSet<String> = expected_sessions
                .iter()
                .filter_map(|s| s.get("session_id").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .collect();

            for sid in local_ids.difference(&expected_ids) {
                differences.push(self.make_diff(
                    "orphan_session",
                    "session",
                    sid,
                    ConflictSeverity::Medium,
                    serde_json::Value::Null,
                    serde_json::json!({"session_id": sid}),
                    None,
                    format!("Session {} exists locally but not in expected state", sid),
                ));
            }

            for sid in expected_ids.difference(&local_ids) {
                differences.push(self.make_diff(
                    "missing_envelope",
                    "session_receipt",
                    sid,
                    ConflictSeverity::High,
                    serde_json::json!({"session_id": sid}),
                    serde_json::Value::Null,
                    None,
                    format!("Session {} is expected but not found locally", sid),
                ));
            }
        }

        if let Some(expected_reg_status) = expected_state
            .get("registration_status")
            .and_then(|v| v.as_str())
        {
            let local_status = "unregistered";
            if local_status != expected_reg_status {
                differences.push(self.make_diff(
                    "state_mismatch",
                    "registration",
                    "registration_status",
                    ConflictSeverity::Medium,
                    serde_json::json!({"registration_status": expected_reg_status}),
                    serde_json::json!({"registration_status": local_status}),
                    Some("registration_status"),
                    format!(
                        "Registration status differs: local={}, expected={}",
                        local_status, expected_reg_status
                    ),
                ));
            }
        }

        if let Some(expected_custody) = expected_state
            .get("custody_envelopes")
            .and_then(|v| v.as_array())
        {
            let local_envelopes: Vec<librarian_contracts::custody::ReceiptEnvelope> = self
                .custody_service
                .as_ref()
                .map(|c| {
                    let guard = c.lock().unwrap();
                    guard.get_envelopes_by_time_range(None, None)
                })
                .unwrap_or_default();

            let local_ids: std::collections::HashSet<String> =
                local_envelopes.iter().map(|e| e.envelope_id.clone()).collect();
            let expected_ids: std::collections::HashSet<String> = expected_custody
                .iter()
                .filter_map(|e| e.get("envelope_id").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .collect();

            for eid in local_ids.difference(&expected_ids) {
                differences.push(self.make_diff(
                    "orphan_session",
                    "custody_envelope",
                    eid,
                    ConflictSeverity::Medium,
                    serde_json::Value::Null,
                    serde_json::json!({"envelope_id": eid}),
                    None,
                    format!("Custody envelope {} exists locally but not expected", eid),
                ));
            }
            for eid in expected_ids.difference(&local_ids) {
                differences.push(self.make_diff(
                    "missing_envelope",
                    "custody_envelope",
                    eid,
                    ConflictSeverity::High,
                    serde_json::json!({"envelope_id": eid}),
                    serde_json::Value::Null,
                    None,
                    format!("Custody envelope {} expected but not found locally", eid),
                ));
            }
        }

        let custody_snapshot = self
            .custody_service
            .as_ref()
            .map(|c| {
                let g = c.lock().unwrap();
                g.get_chain()
                    .map(|ch| ch.last_chain_hash)
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let report = ReconciliationReport {
            report_id: Uuid::new_v4().to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            node_id: self.node_id.clone(),
            lkg_reference: request.lkg_reference.clone(),
            custody_snapshot,
            total_differences: differences.len() as u32,
            differences,
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        };

        self.current_report = Some(report.clone());
        self.pending_differences = report.differences.clone();

        if report.total_differences == 0 {
            let receipt = self.make_receipt(
                &request.reconciliation_id,
                "reconciliation_complete",
                None,
                &[],
                serde_json::json!({"reason": "no_differences", "report_id": report.report_id}),
            );
            self.append_to_custody(&receipt, None);
            self.phase = ReconciliationPhase::Accepting;
        } else {
            let receipt = self.make_receipt(
                &request.reconciliation_id,
                "reconciliation_report",
                None,
                &report
                    .differences
                    .iter()
                    .map(|d| d.difference_id.clone())
                    .collect::<Vec<_>>(),
                serde_json::to_value(&report).unwrap_or_default(),
            );
            self.append_to_custody(&receipt, None);
            self.phase = ReconciliationPhase::Reviewing;
        }

        self.persist();
        report
    }

    /// Validate report: apply auto-accept rules and mark which differences
    /// require owner review. If all differences can be auto-accepted,
    /// transitions to Accepting.
    pub fn validate_report(
        &mut self,
        report: ReconciliationReport,
    ) -> Result<ReconciliationReport, ReconciliationError> {
        self.phase = ReconciliationPhase::Validating;
        let mut validated = report;
        let all_auto_acceptable = validated
            .differences
            .iter()
            .all(|d| !self.does_require_owner_review(d));

        validated.phase = "validated".to_string();

        if all_auto_acceptable && validated.total_differences > 0 {
            let request_id = validated.reconciliation_id.clone();
            let diff_ids: Vec<String> = validated
                .differences
                .iter()
                .map(|d| d.difference_id.clone())
                .collect();
            let receipt = self.make_receipt(
                &request_id,
                "reconciliation_accept",
                None,
                &diff_ids,
                serde_json::json!({"reason": "auto_accepted", "report_id": validated.report_id}),
            );
            self.append_to_custody(&receipt, None);
            self.resolved_difference_ids
                .extend(validated.differences.iter().map(|d| d.difference_id.clone()));
            self.pending_differences.clear();
            self.phase = ReconciliationPhase::Accepting;
            self.current_report = Some(validated.clone());
            self.persist();
            return Ok(validated);
        }

        if validated.total_differences > 0 {
            self.phase = ReconciliationPhase::Reviewing;
        }

        self.current_report = Some(validated.clone());
        self.persist();
        Ok(validated)
    }

    /// Submit an owner decision on a single difference. Produces a receipt,
    /// appends it to the custody chain, and transitions to Accepting when
    /// all differences are resolved.
    pub fn submit_decision(
        &mut self,
        decision: ReconciliationDecision,
    ) -> Result<ReconciliationReceipt, ReconciliationError> {
        if self.phase != ReconciliationPhase::Reviewing
            && self.phase != ReconciliationPhase::Accepting
        {
            return Err(ReconciliationError::InvalidPhaseTransition {
                from: self.phase.as_str().to_string(),
                to: "decision".to_string(),
            });
        }

        if self
            .resolved_difference_ids
            .contains(&decision.difference_id)
        {
            return Err(ReconciliationError::DuplicateDecision(format!(
                "Difference {} already resolved",
                decision.difference_id
            )));
        }

        let report = self
            .current_report
            .as_ref()
            .ok_or_else(|| ReconciliationError::UnknownDifference("No active report".into()))?;

        let diff = report
            .differences
            .iter()
            .find(|d| d.difference_id == decision.difference_id)
            .ok_or_else(|| {
                ReconciliationError::UnknownDifference(format!(
                    "Difference {} not found in current report",
                    decision.difference_id
                ))
            })?;

        let receipt_type = match decision.decision.as_str() {
            "accept" => "reconciliation_accept",
            "override" => "reconciliation_override",
            "quarantine" => "reconciliation_quarantine",
            _ => "reconciliation_accept",
        };

        self.resolved_difference_ids
            .push(decision.difference_id.clone());
        self.pending_differences
            .retain(|d| d.difference_id != decision.difference_id);

        if decision.decision == "quarantine" {
            self.quarantined_differences.push(diff.clone());
        }

        let receipt = self.make_receipt(
            &decision.reconciliation_id,
            receipt_type,
            Some(&decision.decision_id),
            &[decision.difference_id.clone()],
            serde_json::to_value(&decision).unwrap_or_default(),
        );

        let metadata = CustodyMetadata {
            source: "node".to_string(),
            version: "1".to_string(),
            notes: Some(format!(
                "Reconciliation decision '{}' on difference '{}'",
                decision.decision, decision.difference_id
            )),
        };
        self.append_to_custody(&receipt, Some(metadata));

        if self.pending_differences.is_empty() {
            let reason = if self.quarantined_differences.is_empty() {
                "all_differences_resolved"
            } else {
                "some_differences_quarantined"
            };
            let complete_receipt = self.make_receipt(
                &decision.reconciliation_id,
                "reconciliation_complete",
                None,
                &self.resolved_difference_ids.clone(),
                serde_json::json!({
                    "reason": reason,
                    "total_differences": self.resolved_difference_ids.len(),
                    "quarantined": self.quarantined_differences.len(),
                }),
            );
            self.append_to_custody(&complete_receipt, None);
            self.phase = ReconciliationPhase::Accepting;
        }

        self.persist();
        Ok(receipt)
    }

    /// Create a request manually (for the compare endpoint flow).
    pub fn create_request(
        &mut self,
        node_id: &str,
        _session_ids: &[String],
        custody_hash: &str,
        _last_known_good_state: serde_json::Value,
    ) -> ReconciliationRequest {
        self.node_id = node_id.to_string();
        let request = ReconciliationRequest {
            reconciliation_id: Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            lkg_reference: custody_hash.to_string(),
            initiated_at: chrono::Utc::now().to_rfc3339(),
            initiated_by: "owner".to_string(),
            phase: "comparing".to_string(),
        };
        self.current_request = Some(request.clone());
        self.phase = ReconciliationPhase::Comparing;
        self.persist();
        request
    }

    pub fn get_pending_differences(&self) -> Vec<ClassifiedDifference> {
        self.pending_differences.clone()
    }

    pub fn get_reconciliation_status(&self) -> ReconciliationStatus {
        ReconciliationStatus {
            phase: self.phase.as_str().to_string(),
            current_request_id: self
                .current_request
                .as_ref()
                .map(|r| r.reconciliation_id.clone()),
            current_report_id: self.current_report.as_ref().map(|r| r.report_id.clone()),
            pending_differences: self.pending_differences.len() as u32,
            resolved_differences: self.resolved_difference_ids.len() as u32,
            total_receipts: self.receipts.len() as u32,
            node_id: self.node_id.clone(),
        }
    }

    pub fn get_reconciliation_history(&self) -> Vec<ReconciliationReceipt> {
        self.receipts.clone()
    }

    pub fn get_config(&self) -> ReconciliationConfig {
        self.config.clone()
    }

    pub fn set_config(
        &mut self,
        config: ReconciliationConfig,
    ) -> Result<(), ReconciliationError> {
        self.config = config;
        self.persist();
        Ok(())
    }

    pub fn detect_reconnection(&self) -> bool {
        if self.phase != ReconciliationPhase::Offline {
            return false;
        }
        self.custody_service.as_ref().map_or(false, |c| {
            let guard = c.lock().unwrap();
            guard.get_chain().is_some()
        })
    }

    // -- private helpers --

    fn does_require_owner_review(&self, diff: &ClassifiedDifference) -> bool {
        match diff.classification.as_str() {
            "orphan_session" => {
                if diff.actual_state.get("state").and_then(|v| v.as_str())
                    == Some("closed")
                {
                    false
                } else {
                    true
                }
            }
            "state_mismatch" => {
                matches!(
                    diff.artifact_type.as_str(),
                    "registration" | "capability"
                )
            }
            "missing_envelope" | "divergent_hash" | "incomplete_receipt" => true,
            _ => true,
        }
    }

    fn make_diff(
        &self,
        classification: &str,
        artifact_type: &str,
        artifact_id: &str,
        severity: ConflictSeverity,
        expected_state: serde_json::Value,
        actual_state: serde_json::Value,
        field_path: Option<&str>,
        details: String,
    ) -> ClassifiedDifference {
        ClassifiedDifference {
            difference_id: Uuid::new_v4().to_string(),
            classification: classification.to_string(),
            artifact_type: artifact_type.to_string(),
            artifact_id: artifact_id.to_string(),
            severity,
            expected_state,
            actual_state,
            field_path: field_path.map(|s| s.to_string()),
            details,
            detected_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn make_receipt(
        &self,
        reconciliation_id: &str,
        receipt_type: &str,
        decision_id: Option<&str>,
        difference_ids: &[String],
        payload: serde_json::Value,
    ) -> ReconciliationReceipt {
        ReconciliationReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            reconciliation_id: reconciliation_id.to_string(),
            node_id: self.node_id.clone(),
            receipt_type: receipt_type.to_string(),
            previous_phase: None,
            new_phase: Some(self.phase.as_str().to_string()),
            decision_id: decision_id.map(|s| s.to_string()),
            difference_ids: difference_ids.to_vec(),
            payload,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn append_to_custody(
        &mut self,
        receipt: &ReconciliationReceipt,
        metadata: Option<CustodyMetadata>,
    ) {
        let meta = metadata.unwrap_or_else(|| CustodyMetadata {
            source: "node".to_string(),
            version: "1".to_string(),
            notes: Some(format!("Reconciliation receipt: {}", receipt.receipt_type)),
        });
        let payload = serde_json::to_value(receipt).unwrap_or_default();
        if let Some(ref custody) = self.custody_service {
            let mut guard = custody.lock().unwrap();
            guard.append_receipt(
                &self.node_id,
                &receipt.receipt_type,
                &receipt.receipt_id,
                payload,
                Some(meta),
            );
        }
        self.receipts.push(receipt.clone());
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            receipts: self.receipts.clone(),
            config: self.config.clone(),
            quarantined_differences: self.quarantined_differences.clone(),
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

    fn test_service() -> ReconciliationService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("reconciliation.json");
        ReconciliationService::new(path)
    }

    fn service_with_custody(dir: &tempfile::TempDir) -> ReconciliationService {
        let custody_path = dir.path().join("custody.json");
        let custody = CustodyService::new(custody_path);
        let custody_arc = Arc::new(std::sync::Mutex::new(custody));
        let path = dir.path().join("reconciliation.json");
        ReconciliationService::new(path).with_custody(custody_arc)
    }

    #[test]
    fn test_initiate_reconciliation_creates_request() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let request = service.initiate_reconciliation("test-node", "owner").unwrap();
        assert_eq!(request.node_id, "test-node");
        assert_eq!(request.initiated_by, "owner");
        assert!(!request.reconciliation_id.is_empty());
    }

    #[test]
    fn test_initiate_reconciliation_creates_receipt() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let request = service.initiate_reconciliation("test-node", "system").unwrap();
        let history = service.get_reconciliation_history();
        assert!(history.iter().any(|r| {
            r.receipt_type == "reconciliation_started"
                && r.reconciliation_id == request.reconciliation_id
        }));
    }

    #[test]
    fn test_comparison_no_differences_fresh_node() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let req = service.create_request("test-node", &[], "", serde_json::json!({}));
        let report = service.compare_state(
            &req,
            serde_json::json!({
                "sessions": [],
                "registration_status": "unregistered",
                "custody_envelopes": []
            }),
        );
        assert_eq!(report.total_differences, 0);
    }

    #[test]
    fn test_comparison_detects_missing_envelope() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let req = service.create_request("test-node", &[], "", serde_json::json!({}));
        let report = service.compare_state(
            &req,
            serde_json::json!({
                "sessions": [{"session_id": "sess-001", "state": "active"}],
                "registration_status": "unregistered",
                "custody_envelopes": []
            }),
        );
        assert!(report.total_differences > 0);
        let missing = report
            .differences
            .iter()
            .find(|d| d.classification == "missing_envelope");
        assert!(missing.is_some());
        assert_eq!(missing.unwrap().artifact_id, "sess-001");
    }

    #[test]
    fn test_comparison_detects_orphan_session() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);

        let req = service.create_request("test-node", &[], "", serde_json::json!({}));

        let report = service.compare_state(
            &req,
            serde_json::json!({
                "sessions": [],
                "registration_status": "unregistered",
                "custody_envelopes": [{"envelope_id": "env-lkg-001"}]
            }),
        );
        assert!(report.total_differences > 0);
        let missing = report
            .differences
            .iter()
            .find(|d| d.classification == "missing_envelope");
        assert!(missing.is_some());
        assert_eq!(missing.unwrap().artifact_id, "env-lkg-001");
    }

    #[test]
    fn test_comparison_detects_state_mismatch() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let req = service.create_request("test-node", &[], "", serde_json::json!({}));
        let report = service.compare_state(
            &req,
            serde_json::json!({
                "sessions": [],
                "registration_status": "registered",
                "custody_envelopes": []
            }),
        );
        assert!(report.total_differences > 0);
        let sm = report
            .differences
            .iter()
            .find(|d| d.classification == "state_mismatch");
        assert!(sm.is_some());
        assert_eq!(sm.unwrap().artifact_id, "registration_status");
    }

    #[test]
    fn test_generate_report_with_differences() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let req = service.create_request("test-node", &[], "", serde_json::json!({}));
        let report = service.compare_state(
            &req,
            serde_json::json!({
                "sessions": [{"session_id": "sess-001", "state": "active"}],
                "registration_status": "unregistered",
                "custody_envelopes": []
            }),
        );
        assert!(report.total_differences > 0);
        assert_eq!(report.phase, "final");
    }

    #[test]
    fn test_generate_report_no_differences() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let req = service.create_request("test-node", &[], "", serde_json::json!({}));
        let report = service.compare_state(
            &req,
            serde_json::json!({
                "sessions": [],
                "registration_status": "unregistered",
                "custody_envelopes": []
            }),
        );
        assert_eq!(report.total_differences, 0);
        assert_eq!(report.phase, "final");
    }

    #[test]
    fn test_submit_decision_accept() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let request = service.initiate_reconciliation("test-node", "owner").unwrap();

        let diffs = vec![ClassifiedDifference {
            difference_id: "diff-001".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "sess-001".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"session_id": "sess-001"}),
            field_path: None,
            details: "Test difference".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        }];

        let report = ReconciliationReport {
            report_id: "r-001".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            node_id: "test-node".to_string(),
            lkg_reference: request.lkg_reference.clone(),
            custody_snapshot: String::new(),
            total_differences: 1,
            differences: diffs,
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        };

        service.current_report = Some(report);
        service.phase = ReconciliationPhase::Validating;
        let _validated = service.validate_report(ReconciliationReport {
            report_id: "r-validated".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            node_id: "test-node".to_string(),
            lkg_reference: request.lkg_reference.clone(),
            custody_snapshot: String::new(),
            total_differences: 1,
            differences: vec![ClassifiedDifference {
                difference_id: "diff-001".to_string(),
                classification: "orphan_session".to_string(),
                artifact_type: "session".to_string(),
                artifact_id: "sess-001".to_string(),
                severity: ConflictSeverity::Medium,
                expected_state: serde_json::Value::Null,
                actual_state: serde_json::json!({"session_id": "sess-001"}),
                field_path: None,
                details: "Test difference".to_string(),
                detected_at: chrono::Utc::now().to_rfc3339(),
            }],
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        }).unwrap();
        assert_eq!(_validated.phase, "validated");
        assert_eq!(service.phase, ReconciliationPhase::Reviewing);

        let mut service2 = service_with_custody(&dir);
        let request2 = service2.initiate_reconciliation("test-node", "owner").unwrap();

        let diffs2 = vec![ClassifiedDifference {
            difference_id: "diff-accept".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "sess-accept".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"session_id": "sess-accept"}),
            field_path: None,
            details: "To be accepted".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        }];

        let report2 = ReconciliationReport {
            report_id: "r-accept".to_string(),
            reconciliation_id: request2.reconciliation_id.clone(),
            node_id: "test-node".to_string(),
            lkg_reference: request2.lkg_reference.clone(),
            custody_snapshot: String::new(),
            total_differences: 1,
            differences: diffs2,
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        };

        service2.current_report = Some(report2);
        service2.pending_differences.push(ClassifiedDifference {
            difference_id: "diff-accept".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "sess-accept".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"session_id": "sess-accept"}),
            field_path: None,
            details: "To be accepted".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        });
        service2.phase = ReconciliationPhase::Reviewing;

        let decision = ReconciliationDecision {
            decision_id: "dec-accept".to_string(),
            reconciliation_id: request2.reconciliation_id.clone(),
            difference_id: "diff-accept".to_string(),
            node_id: "test-node".to_string(),
            decision: "accept".to_string(),
            reason: Some("Accepted".to_string()),
            decided_at: chrono::Utc::now().to_rfc3339(),
            actor: "owner".to_string(),
        };

        let receipt = service2.submit_decision(decision).unwrap();
        assert_eq!(receipt.receipt_type, "reconciliation_accept");
        assert_eq!(receipt.difference_ids, vec!["diff-accept"]);
    }

    #[test]
    fn test_submit_decision_override() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let request = service.initiate_reconciliation("test-node", "owner").unwrap();

        service.pending_differences.push(ClassifiedDifference {
            difference_id: "diff-override".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "sess-override".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"session_id": "sess-override"}),
            field_path: None,
            details: "To be overridden".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        });
        service.current_report = Some(ReconciliationReport {
            report_id: "r-override".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            node_id: "test-node".to_string(),
            lkg_reference: request.lkg_reference.clone(),
            custody_snapshot: String::new(),
            total_differences: 1,
            differences: service.pending_differences.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        });
        service.phase = ReconciliationPhase::Reviewing;

        let decision = ReconciliationDecision {
            decision_id: "dec-override".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            difference_id: "diff-override".to_string(),
            node_id: "test-node".to_string(),
            decision: "override".to_string(),
            reason: Some("Overridden by owner".to_string()),
            decided_at: chrono::Utc::now().to_rfc3339(),
            actor: "owner".to_string(),
        };

        let receipt = service.submit_decision(decision).unwrap();
        assert_eq!(receipt.receipt_type, "reconciliation_override");
    }

    #[test]
    fn test_decision_generates_receipt_appended_to_custody() {
        let dir = tempdir().unwrap();
        let custody_path = dir.path().join("custody.json");
        let custody = CustodyService::new(&custody_path);
        let custody_arc = Arc::new(std::sync::Mutex::new(custody));
        {
            let mut guard = custody_arc.lock().unwrap();
            guard.seed_identity(
                "test-node",
                serde_json::json!({"node_id": "test-node"}),
                CustodyMetadata {
                    source: "test".to_string(),
                    version: "1".to_string(),
                    notes: None,
                },
            );
        }
        let mut service = ReconciliationService::new(dir.path().join("reconciliation.json"))
            .with_custody(custody_arc.clone());
        let request = service.initiate_reconciliation("test-node", "owner").unwrap();

        service.pending_differences.push(ClassifiedDifference {
            difference_id: "diff-cust".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "sess-cust".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"session_id": "sess-cust"}),
            field_path: None,
            details: "Custody test".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        });
        service.current_report = Some(ReconciliationReport {
            report_id: "r-cust".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            node_id: "test-node".to_string(),
            lkg_reference: request.lkg_reference.clone(),
            custody_snapshot: String::new(),
            total_differences: 1,
            differences: service.pending_differences.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        });
        service.phase = ReconciliationPhase::Reviewing;

        let decision = ReconciliationDecision {
            decision_id: "dec-cust".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            difference_id: "diff-cust".to_string(),
            node_id: "test-node".to_string(),
            decision: "accept".to_string(),
            reason: None,
            decided_at: chrono::Utc::now().to_rfc3339(),
            actor: "owner".to_string(),
        };

        service.submit_decision(decision).unwrap();

        let guard = custody_arc.lock().unwrap();
        let accept_envs = guard.get_envelopes_by_type("reconciliation_accept");
        assert!(!accept_envs.is_empty());
        let chain = guard.get_chain().unwrap();
        assert!(chain.envelope_count >= 3);
    }

    #[test]
    fn test_auto_accept_rules_work() {
        let service = test_service();

        let orphan_closed = ClassifiedDifference {
            difference_id: "d1".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "sess-closed".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"state": "closed"}),
            field_path: None,
            details: "Closed session without custody".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        };
        let orphan_active = ClassifiedDifference {
            difference_id: "d2".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "sess-active".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"state": "active"}),
            field_path: None,
            details: "Active session without custody".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        };
        let missing_env = ClassifiedDifference {
            difference_id: "d3".to_string(),
            classification: "missing_envelope".to_string(),
            artifact_type: "session_receipt".to_string(),
            artifact_id: "sess-missing".to_string(),
            severity: ConflictSeverity::High,
            expected_state: serde_json::json!({"session_id": "sess-missing"}),
            actual_state: serde_json::Value::Null,
            field_path: None,
            details: "Missing envelope".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        };

        assert!(!service.does_require_owner_review(&orphan_closed));
        assert!(service.does_require_owner_review(&orphan_active));
        assert!(service.does_require_owner_review(&missing_env));
    }

    #[test]
    fn test_get_reconciliation_history() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        assert!(service.get_reconciliation_history().is_empty());

        service.initiate_reconciliation("test-node", "owner").unwrap();
        let history = service.get_reconciliation_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].receipt_type, "reconciliation_started");
    }

    #[test]
    fn test_get_reconciliation_status() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let status = service.get_reconciliation_status();
        assert_eq!(status.phase, "offline");

        service.initiate_reconciliation("test-node", "owner").unwrap();
        let status = service.get_reconciliation_status();
        assert_eq!(status.phase, "comparing");
        assert!(status.current_request_id.is_some());
    }

    #[test]
    fn test_get_config() {
        let service = test_service();
        let config = service.get_config();
        assert!(config.auto_reconcile_on_reconnect);
        assert_eq!(config.version, "1");
    }

    #[test]
    fn test_set_config() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let mut config = service.get_config();
        config.auto_reconcile_on_reconnect = false;
        service.set_config(config.clone()).unwrap();
        assert!(!service.get_config().auto_reconcile_on_reconnect);
    }

    #[test]
    fn test_detect_reconnection() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        service.initiate_reconciliation("test-node", "owner").unwrap();
        assert!(!service.detect_reconnection());
    }

    #[test]
    fn test_no_auto_accept() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let req = service.create_request("test-node", &[], "", serde_json::json!({}));
        let report = service.compare_state(
            &req,
            serde_json::json!({
                "sessions": [],
                "registration_status": "unregistered",
                "custody_envelopes": []
            }),
        );
        assert_eq!(report.total_differences, 0);
    }

    #[test]
    fn test_create_request_returns_proper_request() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let req = service.create_request(
            "test-node",
            &["sess-1".to_string()],
            "abc123",
            serde_json::json!({"sessions": []}),
        );
        assert_eq!(req.node_id, "test-node");
        assert_eq!(req.lkg_reference, "abc123");
        assert_eq!(req.initiated_by, "owner");
    }

    #[test]
    fn test_duplicate_decision_rejected() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let request = service.initiate_reconciliation("test-node", "owner").unwrap();

        service.pending_differences.push(ClassifiedDifference {
            difference_id: "diff-dupe".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "sess-dupe".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"session_id": "sess-dupe"}),
            field_path: None,
            details: "Test dupe".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        });
        service.current_report = Some(ReconciliationReport {
            report_id: "r-dupe".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            node_id: "test-node".to_string(),
            lkg_reference: request.lkg_reference.clone(),
            custody_snapshot: String::new(),
            total_differences: 1,
            differences: service.pending_differences.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        });
        service.phase = ReconciliationPhase::Reviewing;

        let decision = ReconciliationDecision {
            decision_id: "dec-dupe".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            difference_id: "diff-dupe".to_string(),
            node_id: "test-node".to_string(),
            decision: "accept".to_string(),
            reason: None,
            decided_at: chrono::Utc::now().to_rfc3339(),
            actor: "owner".to_string(),
        };

        service.submit_decision(decision.clone()).unwrap();
        let result = service.submit_decision(decision);
        assert!(result.is_err());
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("reco_persist.json");
        {
            let custody_path = dir.path().join("custody.json");
            let custody = CustodyService::new(&custody_path);
            let custody_arc = Arc::new(std::sync::Mutex::new(custody));
            let mut service =
                ReconciliationService::new(path.clone()).with_custody(custody_arc);
            service.initiate_reconciliation("test-node", "owner").unwrap();
            assert_eq!(service.get_reconciliation_history().len(), 1);
        }
        {
            let service = ReconciliationService::new(path.clone());
            assert_eq!(service.get_reconciliation_history().len(), 1);
            let config = service.get_config();
            assert!(config.auto_reconcile_on_reconnect);
        }
    }

    #[test]
    fn test_validate_report_marks_owner_review_required() {
        let service = test_service();

        let orphan_closed = ClassifiedDifference {
            difference_id: "d1".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "s1".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"state": "closed"}),
            field_path: None,
            details: "Closed session".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        };
        assert!(!service.does_require_owner_review(&orphan_closed));

        let orphan_active = ClassifiedDifference {
            difference_id: "d2".to_string(),
            classification: "orphan_session".to_string(),
            artifact_type: "session".to_string(),
            artifact_id: "s2".to_string(),
            severity: ConflictSeverity::Medium,
            expected_state: serde_json::Value::Null,
            actual_state: serde_json::json!({"state": "active"}),
            field_path: None,
            details: "Active session".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        };
        assert!(service.does_require_owner_review(&orphan_active));

        let missing_env = ClassifiedDifference {
            difference_id: "d3".to_string(),
            classification: "missing_envelope".to_string(),
            artifact_type: "session_receipt".to_string(),
            artifact_id: "s3".to_string(),
            severity: ConflictSeverity::High,
            expected_state: serde_json::json!({"session_id": "s3"}),
            actual_state: serde_json::Value::Null,
            field_path: None,
            details: "Missing envelope".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        };
        assert!(service.does_require_owner_review(&missing_env));
    }

    #[test]
    fn test_no_silent_merge() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let request = service.initiate_reconciliation("test-node", "owner").unwrap();

        service.pending_differences.push(ClassifiedDifference {
            difference_id: "diff-merge".to_string(),
            classification: "missing_envelope".to_string(),
            artifact_type: "session_receipt".to_string(),
            artifact_id: "sess-merge".to_string(),
            severity: ConflictSeverity::High,
            expected_state: serde_json::json!({"session_id": "sess-merge"}),
            actual_state: serde_json::Value::Null,
            field_path: None,
            details: "Missing envelope".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        });
        service.current_report = Some(ReconciliationReport {
            report_id: "r-merge".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            node_id: "test-node".to_string(),
            lkg_reference: request.lkg_reference.clone(),
            custody_snapshot: String::new(),
            total_differences: 1,
            differences: service.pending_differences.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        });
        service.phase = ReconciliationPhase::Reviewing;

        let decision = ReconciliationDecision {
            decision_id: "dec-merge".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            difference_id: "diff-merge".to_string(),
            node_id: "test-node".to_string(),
            decision: "accept".to_string(),
            reason: Some("Owner explicit accept".to_string()),
            decided_at: chrono::Utc::now().to_rfc3339(),
            actor: "owner".to_string(),
        };

        let receipt = service.submit_decision(decision).unwrap();
        assert!(receipt.decision_id.is_some());
        assert_eq!(receipt.receipt_type, "reconciliation_accept");
    }

    #[test]
    fn test_quarantine_preserves_state() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);
        let request = service.initiate_reconciliation("test-node", "owner").unwrap();

        service.pending_differences.push(ClassifiedDifference {
            difference_id: "diff-quar".to_string(),
            classification: "divergent_hash".to_string(),
            artifact_type: "custody_envelope".to_string(),
            artifact_id: "env-quar".to_string(),
            severity: ConflictSeverity::Critical,
            expected_state: serde_json::json!({"hash": "abc"}),
            actual_state: serde_json::json!({"hash": "def"}),
            field_path: Some("chain_hash".to_string()),
            details: "Hash mismatch".to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
        });
        service.current_report = Some(ReconciliationReport {
            report_id: "r-quar".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            node_id: "test-node".to_string(),
            lkg_reference: request.lkg_reference.clone(),
            custody_snapshot: String::new(),
            total_differences: 1,
            differences: service.pending_differences.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "final".to_string(),
        });
        service.phase = ReconciliationPhase::Reviewing;

        let decision = ReconciliationDecision {
            decision_id: "dec-quar".to_string(),
            reconciliation_id: request.reconciliation_id.clone(),
            difference_id: "diff-quar".to_string(),
            node_id: "test-node".to_string(),
            decision: "quarantine".to_string(),
            reason: Some("Needs investigation".to_string()),
            decided_at: chrono::Utc::now().to_rfc3339(),
            actor: "owner".to_string(),
        };

        service.submit_decision(decision).unwrap();
        assert_eq!(service.quarantined_differences.len(), 1);
        assert_eq!(
            service.quarantined_differences[0].difference_id,
            "diff-quar"
        );
    }

    #[test]
    fn test_single_node_only() {
        let service = test_service();
        assert_eq!(service.node_id, "");
        assert!(service.custody_service.is_some() == false);
    }

    #[test]
    fn test_no_mutation_during_validate() {
        let dir = tempdir().unwrap();
        let mut service = service_with_custody(&dir);

        let report = ReconciliationReport {
            report_id: "r-val".to_string(),
            reconciliation_id: "recon-val".to_string(),
            node_id: "test-node".to_string(),
            lkg_reference: "lkg".to_string(),
            custody_snapshot: "snap".to_string(),
            total_differences: 2,
            differences: vec![
                ClassifiedDifference {
                    difference_id: "dv1".to_string(),
                    classification: "orphan_session".to_string(),
                    artifact_type: "session".to_string(),
                    artifact_id: "s1".to_string(),
                    severity: ConflictSeverity::Medium,
                    expected_state: serde_json::Value::Null,
                    actual_state: serde_json::json!({"state": "closed"}),
                    field_path: None,
                    details: "test".to_string(),
                    detected_at: chrono::Utc::now().to_rfc3339(),
                },
                ClassifiedDifference {
                    difference_id: "dv2".to_string(),
                    classification: "missing_envelope".to_string(),
                    artifact_type: "session_receipt".to_string(),
                    artifact_id: "s2".to_string(),
                    severity: ConflictSeverity::High,
                    expected_state: serde_json::json!({"session_id": "s2"}),
                    actual_state: serde_json::Value::Null,
                    field_path: None,
                    details: "test".to_string(),
                    detected_at: chrono::Utc::now().to_rfc3339(),
                },
            ],
            generated_at: chrono::Utc::now().to_rfc3339(),
            phase: "draft".to_string(),
        };

        let receipts_before = service.get_reconciliation_history().len();
        let validated = service.validate_report(report).unwrap();
        assert_eq!(validated.phase, "validated");
        assert_eq!(service.get_reconciliation_history().len(), receipts_before);
    }
}
