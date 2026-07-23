use std::path::PathBuf;

use librarian_contracts::workload_session::{
    WorkloadAllocationLink, WorkloadDescriptor, WorkloadSession, WorkloadSessionReceipt,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CustodyService, SessionService};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    workload_sessions: Vec<WorkloadSession>,
    receipts: Vec<WorkloadSessionReceipt>,
    links: Vec<WorkloadAllocationLink>,
}

pub struct WorkloadSessionService {
    workload_sessions: Vec<WorkloadSession>,
    receipts: Vec<WorkloadSessionReceipt>,
    links: Vec<WorkloadAllocationLink>,
    persistence_path: PathBuf,
}

impl WorkloadSessionService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (workload_sessions, receipts, links) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.workload_sessions, state.receipts, state.links),
                    Err(_) => (Vec::new(), Vec::new(), Vec::new()),
                },
                Err(_) => (Vec::new(), Vec::new(), Vec::new()),
            }
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        WorkloadSessionService {
            workload_sessions,
            receipts,
            links,
            persistence_path,
        }
    }

    pub fn create_workload_session(
        &mut self,
        workload: WorkloadDescriptor,
        decision_receipt_id: &str,
        node_id: &str,
        allocation_recommendation_id: Option<String>,
        allocation_decision_id: Option<String>,
        session_service: &mut SessionService,
        custody_service: Option<&mut CustodyService>,
    ) -> Result<WorkloadSession, String> {
        let workload_session_id = Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        let req = librarian_contracts::session::SessionStartRequest {
            node_id: node_id.to_string(),
            agent_id: None,
            requested_capabilities: None,
            context: Some(format!("workload_session:{}", workload_session_id)),
        };
        let session = session_service.create_session(req);

        let ws = WorkloadSession {
            workload_session_id: workload_session_id.clone(),
            workload_id: workload.workload_id.clone(),
            session_id: session.session_id.clone(),
            node_id: node_id.to_string(),
            allocation_recommendation_id: allocation_recommendation_id.clone(),
            allocation_decision_id: allocation_decision_id.clone(),
            state: "created".to_string(),
            created_at: created_at.clone(),
            completed_at: None,
            receipt_id: None,
        };

        self.workload_sessions.push(ws.clone());

        let link = WorkloadAllocationLink {
            workload_id: workload.workload_id.clone(),
            allocation_recommendation_id: allocation_recommendation_id.unwrap_or_default(),
            allocation_decision_id: allocation_decision_id.unwrap_or_default(),
            allocation_receipt_id: decision_receipt_id.to_string(),
            session_id: Some(session.session_id),
            linked_at: created_at.clone(),
        };

        self.links.push(link.clone());

        if let Some(custody) = custody_service {
            let payload = serde_json::to_value(&link).unwrap_or_default();
            let metadata = librarian_contracts::custody::CustodyMetadata {
                source: "node".to_string(),
                version: "1".to_string(),
                notes: Some("Workload allocation link custodied on session creation".to_string()),
            };
            custody.append_receipt(
                node_id,
                "workload_allocation_link",
                &link.allocation_decision_id,
                payload,
                Some(metadata),
            );
        }

        self.persist();
        Ok(ws)
    }

    pub fn activate_workload_session(
        &mut self,
        workload_session_id: &str,
        session_service: &mut SessionService,
    ) -> Result<WorkloadSession, String> {
        let idx = self
            .workload_sessions
            .iter()
            .position(|ws| ws.workload_session_id == workload_session_id)
            .ok_or_else(|| format!("WorkloadSession {} not found", workload_session_id))?;

        if self.workload_sessions[idx].state != "created" {
            return Err(format!(
                "WorkloadSession {} cannot be activated from state '{}'",
                workload_session_id, self.workload_sessions[idx].state
            ));
        }

        session_service
            .activate_session(&self.workload_sessions[idx].session_id)
            .map_err(|e| format!("Failed to activate underlying session: {}", e))?;

        self.workload_sessions[idx].state = "active".to_string();
        let result = self.workload_sessions[idx].clone();
        self.persist();
        Ok(result)
    }

    pub fn complete_workload_session(
        &mut self,
        workload_session_id: &str,
        operations_executed: u32,
        evidence_ids: Vec<String>,
        session_service: &mut SessionService,
    ) -> Result<WorkloadSessionReceipt, String> {
        let idx = self
            .workload_sessions
            .iter()
            .position(|ws| ws.workload_session_id == workload_session_id)
            .ok_or_else(|| format!("WorkloadSession {} not found", workload_session_id))?;

        if self.workload_sessions[idx].state != "active" {
            return Err(format!(
                "WorkloadSession {} cannot be completed from state '{}'",
                workload_session_id, self.workload_sessions[idx].state
            ));
        }

        let completed_at = chrono::Utc::now().to_rfc3339();

        session_service
            .close_session(&self.workload_sessions[idx].session_id)
            .map_err(|e| format!("Failed to close underlying session: {}", e))?;

        self.workload_sessions[idx].state = "completed".to_string();
        self.workload_sessions[idx].completed_at = Some(completed_at.clone());

        let receipt = WorkloadSessionReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            workload_session_id: workload_session_id.to_string(),
            workload_id: self.workload_sessions[idx].workload_id.clone(),
            session_id: self.workload_sessions[idx].session_id.clone(),
            node_id: self.workload_sessions[idx].node_id.clone(),
            allocation_decision_id: self.workload_sessions[idx].allocation_decision_id.clone(),
            created_at: self.workload_sessions[idx].created_at.clone(),
            completed_at: Some(completed_at.clone()),
            state: "completed".to_string(),
            operations_executed,
            evidence_ids: evidence_ids.clone(),
        };

        self.workload_sessions[idx].receipt_id = Some(receipt.receipt_id.clone());
        self.receipts.push(receipt.clone());
        self.persist();
        Ok(receipt)
    }

    pub fn fail_workload_session(
        &mut self,
        workload_session_id: &str,
        _reason: &str,
        session_service: &mut SessionService,
    ) -> Result<WorkloadSessionReceipt, String> {
        let idx = self
            .workload_sessions
            .iter()
            .position(|ws| ws.workload_session_id == workload_session_id)
            .ok_or_else(|| format!("WorkloadSession {} not found", workload_session_id))?;

        if self.workload_sessions[idx].state != "active" {
            return Err(format!(
                "WorkloadSession {} cannot be failed from state '{}'",
                workload_session_id, self.workload_sessions[idx].state
            ));
        }

        let completed_at = chrono::Utc::now().to_rfc3339();

        session_service
            .close_session(&self.workload_sessions[idx].session_id)
            .map_err(|e| format!("Failed to close underlying session: {}", e))?;

        self.workload_sessions[idx].state = "failed".to_string();
        self.workload_sessions[idx].completed_at = Some(completed_at.clone());

        let receipt = WorkloadSessionReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            workload_session_id: workload_session_id.to_string(),
            workload_id: self.workload_sessions[idx].workload_id.clone(),
            session_id: self.workload_sessions[idx].session_id.clone(),
            node_id: self.workload_sessions[idx].node_id.clone(),
            allocation_decision_id: self.workload_sessions[idx].allocation_decision_id.clone(),
            created_at: self.workload_sessions[idx].created_at.clone(),
            completed_at: Some(completed_at.clone()),
            state: "failed".to_string(),
            operations_executed: 0,
            evidence_ids: Vec::new(),
        };

        self.workload_sessions[idx].receipt_id = Some(receipt.receipt_id.clone());
        self.receipts.push(receipt.clone());
        self.persist();
        Ok(receipt)
    }

    pub fn get_workload_session(
        &self,
        workload_session_id: &str,
    ) -> Option<WorkloadSession> {
        self.workload_sessions
            .iter()
            .find(|ws| ws.workload_session_id == workload_session_id)
            .cloned()
    }

    pub fn get_workload_sessions_by_node(&self, node_id: &str) -> Vec<WorkloadSession> {
        self.workload_sessions
            .iter()
            .filter(|ws| ws.node_id == node_id)
            .cloned()
            .collect()
    }

    pub fn get_workload_sessions_by_state(&self, state: &str) -> Vec<WorkloadSession> {
        self.workload_sessions
            .iter()
            .filter(|ws| ws.state == state)
            .cloned()
            .collect()
    }

    pub fn list_workload_sessions(&self) -> Vec<WorkloadSession> {
        self.workload_sessions.clone()
    }

    pub fn get_link(&self, workload_id: &str) -> Option<WorkloadAllocationLink> {
        self.links
            .iter()
            .find(|l| l.workload_id == workload_id)
            .cloned()
    }

    pub fn get_receipts(&self) -> Vec<WorkloadSessionReceipt> {
        self.receipts.clone()
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            workload_sessions: self.workload_sessions.clone(),
            receipts: self.receipts.clone(),
            links: self.links.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }
}

impl Serialize for WorkloadSessionService {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PersistedState {
            workload_sessions: self.workload_sessions.clone(),
            receipts: self.receipts.clone(),
            links: self.links.clone(),
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::session::SessionStartRequest;
    use tempfile::tempdir;

    fn test_workload() -> WorkloadDescriptor {
        WorkloadDescriptor {
            workload_id: "wl-001".to_string(),
            workload_type: "inference".to_string(),
            description: "Test inference workload".to_string(),
            requirements: Some(vec!["llm.inference".to_string()]),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn setup() -> (WorkloadSessionService, SessionService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let session_path = dir.path().join("sessions.json");
        let session_service = SessionService::new(session_path);
        let ws_path = dir.path().join("workload_sessions.json");
        let ws_service = WorkloadSessionService::new(ws_path);
        (ws_service, session_service, dir)
    }

    #[test]
    fn test_create_workload_session_from_approved_decision() {
        let (mut ws_service, mut session_service, _dir) = setup();

        let ws = ws_service
            .create_workload_session(
                test_workload(),
                "receipt-001",
                "node-001",
                Some("rec-001".to_string()),
                Some("dec-001".to_string()),
                &mut session_service,
                None,
            )
            .expect("create should succeed");

        assert_eq!(ws.state, "created");
        assert_eq!(ws.node_id, "node-001");
        assert_eq!(ws.workload_id, "wl-001");
        assert_eq!(
            ws.allocation_recommendation_id,
            Some("rec-001".to_string())
        );
        assert_eq!(ws.allocation_decision_id, Some("dec-001".to_string()));
        assert!(!ws.session_id.is_empty());
        assert!(ws.receipt_id.is_none());

        let link = ws_service.get_link("wl-001").expect("link should exist");
        assert_eq!(link.allocation_receipt_id, "receipt-001");
        assert_eq!(link.allocation_recommendation_id, "rec-001");
        assert_eq!(link.allocation_decision_id, "dec-001");
        assert!(link.session_id.is_some());
    }

    #[test]
    fn test_workload_lifecycle_create_activate_complete() {
        let (mut ws_service, mut session_service, _dir) = setup();

        let ws = ws_service
            .create_workload_session(
                test_workload(),
                "receipt-002",
                "node-001",
                None,
                None,
                &mut session_service,
                None,
            )
            .expect("create should succeed");
        assert_eq!(ws.state, "created");

        let activated = ws_service
            .activate_workload_session(&ws.workload_session_id, &mut session_service)
            .expect("activate should succeed");
        assert_eq!(activated.state, "active");

        let receipt = ws_service
            .complete_workload_session(
                &ws.workload_session_id,
                5,
                vec!["evt-001".to_string(), "evt-002".to_string()],
                &mut session_service,
            )
            .expect("complete should succeed");

        assert_eq!(receipt.state, "completed");
        assert_eq!(receipt.workload_session_id, ws.workload_session_id);
        assert_eq!(receipt.operations_executed, 5);
        assert_eq!(receipt.evidence_ids.len(), 2);
        assert!(receipt.completed_at.is_some());

        let stored = ws_service
            .get_workload_session(&ws.workload_session_id)
            .unwrap();
        assert_eq!(stored.state, "completed");
        assert!(stored.completed_at.is_some());
        assert_eq!(stored.receipt_id, Some(receipt.receipt_id.clone()));
    }

    #[test]
    fn test_workload_lifecycle_create_activate_fail() {
        let (mut ws_service, mut session_service, _dir) = setup();

        let ws = ws_service
            .create_workload_session(
                test_workload(),
                "receipt-003",
                "node-001",
                None,
                None,
                &mut session_service,
                None,
            )
            .expect("create should succeed");

        ws_service
            .activate_workload_session(&ws.workload_session_id, &mut session_service)
            .expect("activate should succeed");

        let receipt = ws_service
            .fail_workload_session(&ws.workload_session_id, "Execution error", &mut session_service)
            .expect("fail should succeed");

        assert_eq!(receipt.state, "failed");
        assert_eq!(receipt.workload_session_id, ws.workload_session_id);
        assert!(receipt.completed_at.is_some());

        let stored = ws_service
            .get_workload_session(&ws.workload_session_id)
            .unwrap();
        assert_eq!(stored.state, "failed");
        assert_eq!(stored.receipt_id, Some(receipt.receipt_id.clone()));
    }

    #[test]
    fn test_workload_sessions_listable_by_node_and_state() {
        let (mut ws_service, mut session_service, _dir) = setup();

        ws_service
            .create_workload_session(
                WorkloadDescriptor {
                    workload_id: "wl-001".to_string(),
                    ..test_workload()
                },
                "r1",
                "node-a",
                None,
                None,
                &mut session_service,
                None,
            )
            .unwrap();

        ws_service
            .create_workload_session(
                WorkloadDescriptor {
                    workload_id: "wl-002".to_string(),
                    ..test_workload()
                },
                "r2",
                "node-b",
                None,
                None,
                &mut session_service,
                None,
            )
            .unwrap();

        ws_service
            .create_workload_session(
                WorkloadDescriptor {
                    workload_id: "wl-003".to_string(),
                    ..test_workload()
                },
                "r3",
                "node-a",
                None,
                None,
                &mut session_service,
                None,
            )
            .unwrap();

        let node_a_sessions = ws_service.get_workload_sessions_by_node("node-a");
        assert_eq!(node_a_sessions.len(), 2);

        let node_b_sessions = ws_service.get_workload_sessions_by_node("node-b");
        assert_eq!(node_b_sessions.len(), 1);

        let created_sessions = ws_service.get_workload_sessions_by_state("created");
        assert_eq!(created_sessions.len(), 3);

        let active_sessions = ws_service.get_workload_sessions_by_state("active");
        assert_eq!(active_sessions.len(), 0);
    }

    #[test]
    fn test_allocation_link_is_queryable() {
        let (mut ws_service, mut session_service, _dir) = setup();

        ws_service
            .create_workload_session(
                test_workload(),
                "receipt-link",
                "node-001",
                Some("rec-link".to_string()),
                Some("dec-link".to_string()),
                &mut session_service,
                None,
            )
            .expect("create should succeed");

        let link = ws_service.get_link("wl-001").expect("link should exist");
        assert_eq!(link.workload_id, "wl-001");
        assert_eq!(link.allocation_recommendation_id, "rec-link");
        assert_eq!(link.allocation_decision_id, "dec-link");
        assert_eq!(link.allocation_receipt_id, "receipt-link");
        assert!(link.session_id.is_some());
        assert!(!link.linked_at.is_empty());
    }

    #[test]
    fn test_receipts_collected() {
        let (mut ws_service, mut session_service, _dir) = setup();

        let ws = ws_service
            .create_workload_session(test_workload(), "receipt-004", "node-001", None, None, &mut session_service, None)
            .unwrap();
        ws_service
            .activate_workload_session(&ws.workload_session_id, &mut session_service)
            .unwrap();
        ws_service
            .complete_workload_session(&ws.workload_session_id, 3, vec![], &mut session_service)
            .unwrap();

        ws_service
            .create_workload_session(
                WorkloadDescriptor {
                    workload_id: "wl-002".to_string(),
                    ..test_workload()
                },
                "receipt-005",
                "node-001",
                None,
                None,
                &mut session_service,
                None,
            )
            .unwrap();
        let ws2_id = ws_service
            .get_workload_sessions_by_node("node-001")
            .iter()
            .find(|s| s.workload_id == "wl-002")
            .unwrap()
            .workload_session_id
            .clone();
        ws_service
            .activate_workload_session(&ws2_id, &mut session_service)
            .unwrap();
        let r2 = ws_service
            .fail_workload_session(&ws2_id, "error", &mut session_service)
            .unwrap();

        let receipts = ws_service.get_receipts();
        assert_eq!(receipts.len(), 2);

        let failed = receipts.iter().find(|r| r.state == "failed").unwrap();
        assert_eq!(failed.workload_id, "wl-002");
        assert_eq!(failed.receipt_id, r2.receipt_id);
    }

    #[test]
    fn test_invalid_state_transitions_rejected() {
        let (mut ws_service, mut session_service, _dir) = setup();

        let ws = ws_service
            .create_workload_session(test_workload(), "receipt-006", "node-001", None, None, &mut session_service, None)
            .unwrap();

        assert!(ws_service
            .complete_workload_session(&ws.workload_session_id, 1, vec![], &mut session_service)
            .is_err());

        assert!(ws_service
            .fail_workload_session(&ws.workload_session_id, "reason", &mut session_service)
            .is_err());

        ws_service
            .activate_workload_session(&ws.workload_session_id, &mut session_service)
            .unwrap();
        ws_service
            .complete_workload_session(&ws.workload_session_id, 1, vec![], &mut session_service)
            .unwrap();

        assert!(ws_service
            .activate_workload_session(&ws.workload_session_id, &mut session_service)
            .is_err());
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let ws_path = dir.path().join("ws_persist.json");
        let session_path = dir.path().join("sessions.json");

        {
            let mut session_service = SessionService::new(session_path.clone());
            let mut ws = WorkloadSessionService::new(ws_path.clone());
            let ws1 = ws
                .create_workload_session(test_workload(), "r1", "node-001", None, None, &mut session_service, None)
                .unwrap();
            ws.activate_workload_session(&ws1.workload_session_id, &mut session_service)
                .unwrap();
            ws.complete_workload_session(&ws1.workload_session_id, 2, vec!["e1".to_string()], &mut session_service)
                .unwrap();
        }

        {
            let ws = WorkloadSessionService::new(ws_path.clone());
            assert_eq!(ws.workload_sessions.len(), 1);
            assert_eq!(ws.receipts.len(), 1);
            assert_eq!(ws.workload_sessions[0].state, "completed");
            assert_eq!(ws.workload_sessions[0].workload_id, "wl-001");
        }
    }
}
