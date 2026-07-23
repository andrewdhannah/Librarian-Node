use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::custody::CustodyMetadata;
use librarian_contracts::node::RegistrationReceipt;
use librarian_contracts::owner_workflows::{
    DecisionReceipt, OwnerActionEntry, OwnerActionHistory, OwnerDecision, PendingApprovalItem,
    PendingApprovalsSummary, ReviewResult,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::custody_service::CustodyService;
use super::{
    BootstrapService, CapabilityEvidenceBridge, NodeIdentityService, RegistrationService,
    SessionService,
};
use crate::db::RuntimeDatabase;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    action_history: Vec<OwnerActionEntry>,
}

pub struct OwnerWorkflowService {
    action_history: Vec<OwnerActionEntry>,
    persistence_path: PathBuf,
    custody_service: Option<Arc<std::sync::Mutex<CustodyService>>>,
}

impl OwnerWorkflowService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let action_history = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => state.action_history,
                    Err(_) => Vec::new(),
                },
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        OwnerWorkflowService {
            action_history,
            persistence_path,
            custody_service: None,
        }
    }

    pub fn with_custody(mut self, custody: Arc<std::sync::Mutex<CustodyService>>) -> Self {
        self.custody_service = Some(custody);
        self
    }

    pub fn review_node_state(
        &self,
        _session_id: &str,
        identity_service: &NodeIdentityService,
        registration_service: &RegistrationService,
        bridge: &CapabilityEvidenceBridge,
        session_service: &SessionService,
        bootstrap_service: &BootstrapService,
        db: &RuntimeDatabase,
    ) -> ReviewResult {
        let identity = identity_service.get_identity();
        let node_id = identity.node_id.clone();
        let reg_record = registration_service.get_record();

        let manifest =
            crate::node::capabilities::detect_capabilities(db, &node_id, Some(bridge), None);

        let sessions = session_service.list_sessions(None);
        let active_count = sessions.iter().filter(|s| s.state == "active").count();

        let bootstrap_receipt_count = bootstrap_service.get_receipts().len();

        let data = serde_json::json!({
            "node_id": node_id,
            "display_name": identity.display_name,
            "platform": identity.platform,
            "runtime_version": identity.runtime_version,
            "registration_status": reg_record.registration_status,
            "capability_count": manifest.capabilities.len(),
            "total_sessions": sessions.len(),
            "active_session_count": active_count,
            "bootstrap_receipt_count": bootstrap_receipt_count,
        });

        ReviewResult {
            result_id: Uuid::new_v4().to_string(),
            request_id: String::new(),
            review_type: "node_state".to_string(),
            summary: format!("Node state review for {}", node_id),
            data,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn review_capabilities(
        &self,
        _session_id: &str,
        identity_service: &NodeIdentityService,
        bridge: &CapabilityEvidenceBridge,
        db: &RuntimeDatabase,
    ) -> ReviewResult {
        let node_id = identity_service.get_identity().node_id.clone();
        let manifest =
            crate::node::capabilities::detect_capabilities(db, &node_id, Some(bridge), None);
        let verification_state = bridge.get_verification_state(&node_id);

        let verified_count = manifest
            .capabilities
            .iter()
            .filter(|c| c.verification_status.as_deref() == Some("verified"))
            .count();
        let unverified_count = manifest
            .capabilities
            .iter()
            .filter(|c| {
                c.verification_status
                    .as_deref()
                    .map(|s| s != "verified")
                    .unwrap_or(true)
            })
            .count();

        let data = serde_json::json!({
            "capabilities": manifest.capabilities,
            "verification_state": verification_state,
            "verified_count": verified_count,
            "unverified_count": unverified_count,
            "total_count": manifest.capabilities.len(),
        });

        ReviewResult {
            result_id: Uuid::new_v4().to_string(),
            request_id: String::new(),
            review_type: "capabilities".to_string(),
            summary: format!(
                "Capability review: {} verified, {} unverified",
                verified_count, unverified_count
            ),
            data,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn review_sessions(
        &self,
        _session_id: &str,
        session_service: &SessionService,
    ) -> ReviewResult {
        let sessions = session_service.list_sessions(None);
        let receipts = session_service.get_receipts();

        let active_count = sessions.iter().filter(|s| s.state == "active").count();
        let closed_count = sessions.iter().filter(|s| s.state == "closed").count();
        let expired_count = sessions.iter().filter(|s| s.state == "expired").count();

        let data = serde_json::json!({
            "sessions": sessions,
            "receipts": receipts,
            "active_count": active_count,
            "closed_count": closed_count,
            "expired_count": expired_count,
            "total_count": sessions.len(),
        });

        ReviewResult {
            result_id: Uuid::new_v4().to_string(),
            request_id: String::new(),
            review_type: "sessions".to_string(),
            summary: format!(
                "Session review: {} active, {} closed, {} expired",
                active_count, closed_count, expired_count
            ),
            data,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn review_custody(
        &self,
        _session_id: &str,
        custody_service: &CustodyService,
    ) -> ReviewResult {
        let chain = custody_service.get_chain();
        let integrity = custody_service.verify_integrity();
        let envelopes = custody_service.get_envelopes_by_time_range(None, None);
        let envelope_count = envelopes.len();

        let data = serde_json::json!({
            "chain": chain,
            "integrity_report": integrity,
            "envelopes": envelopes,
            "envelope_count": envelope_count,
        });

        ReviewResult {
            result_id: Uuid::new_v4().to_string(),
            request_id: String::new(),
            review_type: "custody".to_string(),
            summary: format!(
                "Custody review: {} envelopes, integrity verified: {}",
                envelope_count,
                integrity.verified
            ),
            data,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn review_bootstrap_history(
        &self,
        _session_id: &str,
        bootstrap_service: &BootstrapService,
    ) -> ReviewResult {
        let receipts = bootstrap_service.get_receipts();
        let receipt_count = receipts.len();

        let data = serde_json::json!({
            "receipts": receipts,
            "receipt_count": receipt_count,
        });

        ReviewResult {
            result_id: Uuid::new_v4().to_string(),
            request_id: String::new(),
            review_type: "bootstrap_history".to_string(),
            summary: format!("Bootstrap history: {} receipts", receipt_count),
            data,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_pending_approvals(
        &self,
        bootstrap_service: &BootstrapService,
        registration_service: &RegistrationService,
        bridge: &CapabilityEvidenceBridge,
    ) -> PendingApprovalsSummary {
        let mut items = Vec::new();

        for plan in bootstrap_service.get_plans() {
            if plan.owner_approved || plan.status != "draft" {
                continue;
            }
            let has_high_impact = plan
                .recommendations
                .iter()
                .any(|r| r.impact == "high" && r.owner_approval_required);
            if !has_high_impact {
                continue;
            }
            items.push(PendingApprovalItem {
                item_id: plan.plan_id.clone(),
                item_type: "bootstrap_action".to_string(),
                description: format!(
                    "Bootstrap plan with {} recommendations requiring approval",
                    plan.recommendations.len()
                ),
                requested_at: plan.created_at.clone(),
                session_id: plan.session_id.clone(),
                details: serde_json::to_value(plan).unwrap_or_default(),
                impact: "high".to_string(),
            });
        }

        let record = registration_service.get_record();
        if record.registration_status == "registration_requested" {
            items.push(PendingApprovalItem {
                item_id: record.node_id.clone(),
                item_type: "registration".to_string(),
                description: format!(
                    "Registration request for node '{}' pending confirmation",
                    record.display_name
                ),
                requested_at: record
                    .last_seen_at
                    .clone()
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                session_id: String::new(),
                details: serde_json::to_value(record).unwrap_or_default(),
                impact: "high".to_string(),
            });
        }

        let unverified_claims = bridge.get_unverified_claims();
        for claim in &unverified_claims {
            items.push(PendingApprovalItem {
                item_id: claim.claim_id.clone(),
                item_type: "capability_claim".to_string(),
                description: format!(
                    "Unverified capability claim '{}'",
                    claim.capability_type
                ),
                requested_at: claim.claimed_at.clone(),
                session_id: String::new(),
                details: serde_json::to_value(claim).unwrap_or_default(),
                impact: "medium".to_string(),
            });
        }

        PendingApprovalsSummary {
            total_pending: items.len() as u32,
            items,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn submit_decision(
        &mut self,
        decision: OwnerDecision,
        bootstrap_service: &mut BootstrapService,
        registration_service: &mut RegistrationService,
    ) -> DecisionReceipt {
        let receipt_id = Uuid::new_v4().to_string();
        let mut previous_state: Option<String> = None;
        let mut new_state: Option<String> = None;
        let item_type: String;

        match decision.item_type.as_str() {
            "bootstrap_action" => {
                item_type = "bootstrap_action".to_string();
                previous_state = Some("pending_approval".to_string());
                match decision.decision.as_str() {
                    "approved" => {
                        match bootstrap_service.approve_plan(&decision.item_id) {
                            Ok(plan) => {
                                new_state = Some(format!("approved (status: {})", plan.status));
                            }
                            Err(e) => {
                                new_state = Some(format!("approval_failed: {}", e));
                            }
                        }
                    }
                    "rejected" => {
                        let plans = bootstrap_service.get_plans_mut();
                        if let Some(plan) = plans.iter_mut().find(|p| p.plan_id == decision.item_id) {
                            previous_state =
                                Some(format!("draft (owner_approved: {})", plan.owner_approved));
                            plan.status = "rejected".to_string();
                            new_state = Some("rejected".to_string());
                        }
                    }
                    "deferred" => {
                        new_state = Some("deferred".to_string());
                    }
                    _ => {
                        new_state = Some("unknown_decision".to_string());
                    }
                }
            }
            "registration" => {
                item_type = "registration".to_string();
                previous_state =
                    Some(registration_service.get_record().registration_status.clone());
                match decision.decision.as_str() {
                    "approved" => {
                        let receipt = RegistrationReceipt {
                            registration_id: Uuid::new_v4().to_string(),
                            node_id: decision.item_id.clone(),
                            status: "registered".to_string(),
                            registered_at: chrono::Utc::now().to_rfc3339(),
                            previous_state: Some("registration_requested".to_string()),
                        };
                        registration_service.confirm_registration(&receipt);
                        new_state = Some("registered".to_string());
                    }
                    "rejected" => {
                        let record = registration_service.get_record_mut();
                        previous_state = Some(record.registration_status.clone());
                        record.registration_status = "registration_rejected".to_string();
                        new_state = Some("registration_rejected".to_string());
                    }
                    "deferred" => {
                        new_state = Some("deferred".to_string());
                    }
                    _ => {
                        new_state = Some("unknown_decision".to_string());
                    }
                }
            }
            _ => {
                item_type = decision.item_type.clone();
                new_state = Some(format!("decision_recorded: {}", decision.decision));
            }
        }

        let mut receipt = DecisionReceipt {
            receipt_id: receipt_id.clone(),
            decision_id: decision.decision_id.clone(),
            item_id: decision.item_id.clone(),
            item_type: item_type.clone(),
            decision: decision.decision.clone(),
            decided_at: decision.decided_at.clone(),
            previous_state,
            new_state,
            custody_envelope_id: None,
        };

        if let Some(ref custody) = self.custody_service {
            let payload = serde_json::to_value(&receipt).unwrap_or_default();
            let metadata = CustodyMetadata {
                source: "node".to_string(),
                version: "1".to_string(),
                notes: Some(format!(
                    "Owner decision '{}' on '{}'",
                    decision.decision, decision.item_id
                )),
            };
            let mut guard = custody.lock().unwrap();
            let envelope = guard.append_receipt(
                &String::new(),
                "owner_decision",
                &receipt_id,
                payload,
                Some(metadata),
            );
            receipt.custody_envelope_id = Some(envelope.envelope_id);
        }

        self.log_action(
            match decision.decision.as_str() {
                "approved" => "approve",
                "rejected" => "reject",
                "deferred" => "defer",
                _ => "decide",
            },
            &item_type,
            &format!(
                "Owner {} item '{}'",
                decision.decision, decision.item_id
            ),
            Some(&decision.session_id),
            Some(&receipt_id),
        );

        self.persist();
        receipt
    }

    pub fn get_action_history(
        &self,
        identity_service: &NodeIdentityService,
    ) -> OwnerActionHistory {
        OwnerActionHistory {
            node_id: identity_service.get_identity().node_id.clone(),
            actions: self.action_history.clone(),
            total_count: self.action_history.len() as u32,
        }
    }

    pub fn log_action(
        &mut self,
        action_type: &str,
        item_type: &str,
        summary: &str,
        session_id: Option<&str>,
        receipt_id: Option<&str>,
    ) {
        self.action_history.push(OwnerActionEntry {
            action_id: Uuid::new_v4().to_string(),
            action_type: action_type.to_string(),
            item_type: item_type.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            summary: summary.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            receipt_id: receipt_id.map(|r| r.to_string()),
        });
        self.persist();
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            action_history: self.action_history.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeIdentityService;
    use crate::platform::create_detector;
    use librarian_contracts::owner_workflows::OwnerDecision;
    use tempfile::tempdir;

    fn test_setup() -> (
        OwnerWorkflowService,
        NodeIdentityService,
        RegistrationService,
        BootstrapService,
        CapabilityEvidenceBridge,
        SessionService,
        tempfile::TempDir,
    ) {
        let dir = tempdir().unwrap();
        let ow_path = dir.path().join("owner_workflows.json");
        let identity_path = dir.path().join("identity.json");
        let reg_path = dir.path().join("registration.json");
        let bridge_path = dir.path().join("bridge.json");
        let bootstrap_path = dir.path().join("bootstrap.json");
        let session_path = dir.path().join("sessions.json");

        let identity_service = NodeIdentityService::new(identity_path);
        let identity_arc = Arc::new(NodeIdentityService::new(dir.path().join("id2.json")));
        let registration_service = RegistrationService::new(reg_path);
        let bridge = CapabilityEvidenceBridge::new(bridge_path);
        let bridge_arc = Arc::new(std::sync::Mutex::new(
            CapabilityEvidenceBridge::new(dir.path().join("bridge2.json")),
        ));
        let bootstrap_service = BootstrapService::new(
            bootstrap_path,
            identity_arc,
            bridge_arc,
            Arc::new(create_detector()),
        );
        let session_service = SessionService::new(session_path);

        let service = OwnerWorkflowService::new(ow_path);

        (service, identity_service, registration_service, bootstrap_service, bridge, session_service, dir)
    }

    fn open_test_db(dir: &tempfile::TempDir) -> crate::db::RuntimeDatabase {
        let db_path = dir.path().join("test.db");
        let db = crate::db::RuntimeDatabase::open(db_path).unwrap();
        db.migrate().unwrap();
        db
    }

    #[test]
    fn test_review_node_state() {
        let (service, identity, reg, bootstrap, bridge, session, dir) = test_setup();
        let db = open_test_db(&dir);

        let result = service.review_node_state(
            "session-1", &identity, &reg, &bridge, &session, &bootstrap, &db,
        );
        assert_eq!(result.review_type, "node_state");
        assert!(result.data["node_id"].is_string());
        assert!(result.data["registration_status"].is_string());
    }

    #[test]
    fn test_review_capabilities() {
        let (service, identity, _reg, _bootstrap, bridge, _session, dir) = test_setup();
        let _ = &bridge;
        let db = open_test_db(&dir);

        let result = service.review_capabilities("session-1", &identity, &bridge, &db);
        assert_eq!(result.review_type, "capabilities");
        assert!(result.data["total_count"].as_u64().unwrap_or(0) > 0);
    }

    #[test]
    fn test_review_sessions() {
        let (service, _identity, _reg, _bootstrap, _bridge, session, _dir) = test_setup();

        let result = service.review_sessions("session-1", &session);
        assert_eq!(result.review_type, "sessions");
        assert_eq!(result.data["total_count"].as_u64().unwrap_or(0), 0);
    }

    #[test]
    fn test_review_custody() {
        let (service, _identity, _reg, _bootstrap, _bridge, _session, dir) = test_setup();
        let custody_path = dir.path().join("custody.json");
        let custody = CustodyService::new(custody_path);

        let result = service.review_custody("session-1", &custody);
        assert_eq!(result.review_type, "custody");
        assert_eq!(result.data["envelope_count"].as_u64().unwrap_or(0), 0);
    }

    #[test]
    fn test_review_bootstrap_history() {
        let (service, _identity, _reg, bootstrap, _bridge, _session, _dir) = test_setup();

        let result = service.review_bootstrap_history("session-1", &bootstrap);
        assert_eq!(result.review_type, "bootstrap_history");
        assert_eq!(result.data["receipt_count"].as_u64().unwrap_or(0), 0);
    }

    #[test]
    fn test_pending_approvals_empty_when_nothing_pending() {
        let (service, _identity, reg, bootstrap, bridge, _session, _dir) = test_setup();
        let approvals = service.get_pending_approvals(&bootstrap, &reg, &bridge);
        assert_eq!(approvals.total_pending, 0);
        assert!(approvals.items.is_empty());
    }

    #[test]
    fn test_pending_approvals_collects_from_all_services() {
        let (service, _identity, mut reg, mut bootstrap, mut bridge, _session, _dir) = test_setup();

        // Create a bootstrap assessment and plan with high-impact items
        let assessment = bootstrap.assess("session-1");
        let rec_ids: Vec<String> = assessment
            .recommendations
            .iter()
            .filter(|r| r.impact == "high")
            .map(|r| r.recommendation_id.clone())
            .collect();
        if !rec_ids.is_empty() {
            bootstrap.create_plan("session-1", &assessment.assessment_id, &rec_ids).unwrap();
        }

        // Submit a registration request
        let node_identity = librarian_contracts::node::NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        let _request = reg.submit_registration(&node_identity, None);

        // Create unverified claim
        bridge.register_claim("test-node", "llm.inference", Some("llama.cpp".to_string()), None);

        let approvals = service.get_pending_approvals(&bootstrap, &reg, &bridge);
        assert!(approvals.total_pending > 0);

        let types: Vec<&str> = approvals.items.iter().map(|i| i.item_type.as_str()).collect();
        assert!(types.contains(&"registration"));
    }

    #[test]
    fn test_owner_decision_approve_generates_receipt() {
        let (mut service, _identity, mut reg, mut bootstrap, _bridge, _session, _dir) = test_setup();

        // Create a pending registration
        let node_identity = librarian_contracts::node::NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        let _request = reg.submit_registration(&node_identity, None);
        assert_eq!(reg.get_record().registration_status, "registration_requested");

        let decision = OwnerDecision {
            decision_id: "dec-001".to_string(),
            item_id: "test-node-uuid".to_string(),
            item_type: "registration".to_string(),
            session_id: "session-1".to_string(),
            decision: "approved".to_string(),
            reason: Some("Looks good".to_string()),
            decided_at: chrono::Utc::now().to_rfc3339(),
            owner_identity: Some("owner-1".to_string()),
        };

        let receipt = service.submit_decision(decision, &mut bootstrap, &mut reg);
        assert_eq!(receipt.decision, "approved");
        assert_eq!(receipt.item_type, "registration");
        assert_eq!(receipt.new_state, Some("registered".to_string()));
        assert_eq!(receipt.previous_state, Some("registration_requested".to_string()));
        assert_eq!(reg.get_record().registration_status, "registered");
    }

    #[test]
    fn test_owner_decision_reject_generates_receipt() {
        let (mut service, _identity, mut reg, mut bootstrap, _bridge, _session, _dir) = test_setup();

        let node_identity = librarian_contracts::node::NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        let _request = reg.submit_registration(&node_identity, None);

        let decision = OwnerDecision {
            decision_id: "dec-002".to_string(),
            item_id: "test-node-uuid".to_string(),
            item_type: "registration".to_string(),
            session_id: "session-1".to_string(),
            decision: "rejected".to_string(),
            reason: Some("Not ready".to_string()),
            decided_at: chrono::Utc::now().to_rfc3339(),
            owner_identity: Some("owner-1".to_string()),
        };

        let receipt = service.submit_decision(decision, &mut bootstrap, &mut reg);
        assert_eq!(receipt.decision, "rejected");
        assert_eq!(receipt.new_state, Some("registration_rejected".to_string()));
        assert_eq!(reg.get_record().registration_status, "registration_rejected");
    }

    #[test]
    fn test_decision_receipt_appended_to_custody_chain() {
        let (mut service, _identity, mut reg, mut bootstrap, _bridge, _session, dir) = test_setup();

        // Wire custody to the service
        let custody_path = dir.path().join("custody.json");
        let custody = CustodyService::new(custody_path);
        let custody_arc = Arc::new(std::sync::Mutex::new(custody));
        service.custody_service = Some(custody_arc.clone());

        // Seed identity
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

        let node_identity = librarian_contracts::node::NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        let _request = reg.submit_registration(&node_identity, None);

        let decision = OwnerDecision {
            decision_id: "dec-003".to_string(),
            item_id: "test-node-uuid".to_string(),
            item_type: "registration".to_string(),
            session_id: "session-1".to_string(),
            decision: "approved".to_string(),
            reason: None,
            decided_at: chrono::Utc::now().to_rfc3339(),
            owner_identity: None,
        };

        let receipt = service.submit_decision(decision, &mut bootstrap, &mut reg);
        assert!(receipt.custody_envelope_id.is_some());

        // Verify the envelope is in the custody chain
        let guard = custody_arc.lock().unwrap();
        let chain = guard.get_chain().unwrap();
        assert_eq!(chain.envelope_count, 2); // identity + decision
    }

    #[test]
    fn test_owner_action_history_records_decisions() {
        let (mut service, _identity, mut reg, mut bootstrap, _bridge, _session, dir) = test_setup();

        let node_identity = librarian_contracts::node::NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        let _request = reg.submit_registration(&node_identity, None);

        let decision = OwnerDecision {
            decision_id: "dec-004".to_string(),
            item_id: "test-node-uuid".to_string(),
            item_type: "registration".to_string(),
            session_id: "session-1".to_string(),
            decision: "approved".to_string(),
            reason: None,
            decided_at: chrono::Utc::now().to_rfc3339(),
            owner_identity: None,
        };
        service.submit_decision(decision, &mut bootstrap, &mut reg);

        let identity_service = NodeIdentityService::new(dir.path().join("identity.json"));
        let history = service.get_action_history(&identity_service);
        assert_eq!(history.total_count, 1);
        assert_eq!(history.actions[0].action_type, "approve");
        assert_eq!(history.actions[0].item_type, "registration");
    }

    #[test]
    fn test_owner_action_history_queryable() {
        let (mut service, _identity, mut reg, mut bootstrap, _bridge, _session, dir) = test_setup();

        let node_identity = librarian_contracts::node::NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        let _request = reg.submit_registration(&node_identity, None);

        service.submit_decision(
            OwnerDecision {
                decision_id: "dec-a".to_string(),
                item_id: "test-node-uuid".to_string(),
                item_type: "registration".to_string(),
                session_id: "session-1".to_string(),
                decision: "approved".to_string(),
                reason: None,
                decided_at: chrono::Utc::now().to_rfc3339(),
                owner_identity: None,
            },
            &mut bootstrap,
            &mut reg,
        );

        service.submit_decision(
            OwnerDecision {
                decision_id: "dec-b".to_string(),
                item_id: "test-node-uuid".to_string(),
                item_type: "registration".to_string(),
                session_id: "session-1".to_string(),
                decision: "rejected".to_string(),
                reason: Some("Not suitable".to_string()),
                decided_at: chrono::Utc::now().to_rfc3339(),
                owner_identity: None,
            },
            &mut bootstrap,
            &mut reg,
        );

        let identity_service = NodeIdentityService::new(dir.path().join("identity.json"));
        let history = service.get_action_history(&identity_service);
        assert_eq!(history.total_count, 2);
        assert_eq!(history.actions[1].action_type, "reject");

        // Verify timestamps are present
        for action in &history.actions {
            assert!(!action.timestamp.is_empty());
            assert!(!action.action_id.is_empty());
            assert!(action.receipt_id.is_some());
        }
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let ow_path = dir.path().join("owner_workflows.json");

        {
            let mut service = OwnerWorkflowService::new(&ow_path);
            service.log_action("review", "node_state", "Initial review", None, None);
            assert_eq!(service.action_history.len(), 1);
        }

        {
            let service = OwnerWorkflowService::new(&ow_path);
            assert_eq!(service.action_history.len(), 1);
            assert_eq!(service.action_history[0].action_type, "review");
        }
    }

    #[test]
    fn test_bootstrap_decision_approve() {
        let (mut service, _identity, mut reg, mut bootstrap, _bridge, _session, _dir) = test_setup();

        // Create assessment and plan
        let assessment = bootstrap.assess("session-1");
        let high_impact_ids: Vec<String> = assessment
            .recommendations
            .iter()
            .filter(|r| r.impact == "high")
            .map(|r| r.recommendation_id.clone())
            .collect();

        if !high_impact_ids.is_empty() {
            let plan = bootstrap.create_plan("session-1", &assessment.assessment_id, &high_impact_ids).unwrap();
            assert!(!plan.owner_approved);

            let decision = OwnerDecision {
                decision_id: "dec-bootstrap".to_string(),
                item_id: plan.plan_id.clone(),
                item_type: "bootstrap_action".to_string(),
                session_id: "session-1".to_string(),
                decision: "approved".to_string(),
                reason: Some("Approved".to_string()),
                decided_at: chrono::Utc::now().to_rfc3339(),
                owner_identity: Some("owner-1".to_string()),
            };

            let receipt = service.submit_decision(decision, &mut bootstrap, &mut reg);
            assert_eq!(receipt.decision, "approved");
            assert_eq!(receipt.item_type, "bootstrap_action");
            assert!(receipt.new_state.unwrap_or_default().contains("approved"));

            // Verify plan was actually approved
            let updated_plan = bootstrap.get_plan(&plan.plan_id).unwrap();
            assert!(updated_plan.owner_approved);
        }
    }

    #[test]
    fn test_custody_service_not_required() {
        // Service works without custody service
        let (mut service, _identity, mut reg, mut bootstrap, _bridge, _session, _dir) = test_setup();
        assert!(service.custody_service.is_none());

        let node_identity = librarian_contracts::node::NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        let _request = reg.submit_registration(&node_identity, None);

        let decision = OwnerDecision {
            decision_id: "dec-nocustody".to_string(),
            item_id: "test-node-uuid".to_string(),
            item_type: "registration".to_string(),
            session_id: "session-1".to_string(),
            decision: "approved".to_string(),
            reason: None,
            decided_at: chrono::Utc::now().to_rfc3339(),
            owner_identity: None,
        };
        let receipt = service.submit_decision(decision, &mut bootstrap, &mut reg);
        assert!(receipt.custody_envelope_id.is_none());
        assert_eq!(reg.get_record().registration_status, "registered");
    }
}
