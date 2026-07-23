use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::custody::CustodyMetadata;
use librarian_contracts::session::{Session, SessionReceipt, SessionStartRequest};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use uuid::Uuid;

use super::{CapabilityEvidenceBridge, CustodyService};

const DEFAULT_EXPIRY_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    sessions: Vec<Session>,
    receipts: Vec<SessionReceipt>,
    operation_counts: HashMap<String, u32>,
    evidence_by_session: HashMap<String, Vec<String>>,
}

pub struct SessionService {
    sessions: Vec<Session>,
    receipts: Vec<SessionReceipt>,
    operation_counts: HashMap<String, u32>,
    evidence_by_session: HashMap<String, Vec<String>>,
    persistence_path: PathBuf,
    capability_evidence_bridge: Option<Arc<std::sync::Mutex<CapabilityEvidenceBridge>>>,
    custody_service: Option<Arc<std::sync::Mutex<CustodyService>>>,
}

impl SessionService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (sessions, receipts, operation_counts, evidence_by_session) =
            if persistence_path.exists() {
                match std::fs::read_to_string(&persistence_path) {
                    Ok(content) => {
                        match serde_json::from_str::<PersistedState>(&content) {
                            Ok(state) => (
                                state.sessions,
                                state.receipts,
                                state.operation_counts,
                                state.evidence_by_session,
                            ),
                            Err(_) => (Vec::new(), Vec::new(), HashMap::new(), HashMap::new()),
                        }
                    }
                    Err(_) => (Vec::new(), Vec::new(), HashMap::new(), HashMap::new()),
                }
            } else {
                (Vec::new(), Vec::new(), HashMap::new(), HashMap::new())
            };

        SessionService {
            sessions,
            receipts,
            operation_counts,
            evidence_by_session,
            persistence_path,
            capability_evidence_bridge: None,
            custody_service: None,
        }
    }

    pub fn with_custody(mut self, custody: Arc<std::sync::Mutex<CustodyService>>) -> Self {
        self.custody_service = Some(custody);
        self
    }

    pub fn with_bridge(mut self, bridge: Arc<std::sync::Mutex<CapabilityEvidenceBridge>>) -> Self {
        self.capability_evidence_bridge = Some(bridge);
        self
    }

    pub fn create_session(&mut self, request: SessionStartRequest) -> Session {
        let session_id = Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();

        let capability_snapshot = self
            .capability_evidence_bridge
            .as_ref()
            .map(|bridge| {
                let bridge = bridge.lock().unwrap();
                let state = bridge.get_verification_state(&request.node_id);
                serde_json::to_string(&state).ok()
            })
            .flatten();

        let session = Session {
            session_id: session_id.clone(),
            node_id: request.node_id.clone(),
            agent_id: request.agent_id,
            state: "created".to_string(),
            started_at: started_at.clone(),
            closed_at: None,
            capability_snapshot,
            context: request.context,
        };

        self.sessions.push(session.clone());
        self.operation_counts.insert(session_id.clone(), 0);
        self.evidence_by_session
            .insert(session_id.clone(), Vec::new());
        self.persist();
        session
    }

    pub fn activate_session(&mut self, session_id: &str) -> Result<Session, String> {
        let idx = self
            .sessions
            .iter()
            .position(|s| s.session_id == session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        if self.sessions[idx].state != "created" {
            return Err(format!(
                "Session {} cannot be activated from state '{}'",
                session_id, self.sessions[idx].state
            ));
        }

        self.sessions[idx].state = "active".to_string();
        let result = self.sessions[idx].clone();
        self.check_expired_sessions();
        self.persist();
        Ok(result)
    }

    pub fn close_session(&mut self, session_id: &str) -> Result<SessionReceipt, String> {
        let idx = self
            .sessions
            .iter()
            .position(|s| s.session_id == session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        if self.sessions[idx].state != "active" {
            return Err(format!(
                "Session {} cannot be closed from state '{}'",
                session_id, self.sessions[idx].state
            ));
        }

        let closed_at = chrono::Utc::now().to_rfc3339();
        self.sessions[idx].state = "closed".to_string();
        self.sessions[idx].closed_at = Some(closed_at.clone());

        let operations_executed = self.operation_counts.get(session_id).copied().unwrap_or(0);
        let evidence_ids = self
            .evidence_by_session
            .get(session_id)
            .cloned()
            .unwrap_or_default();

        let node_id = self.sessions[idx].node_id.clone();
        let started_at = self.sessions[idx].started_at.clone();
        let capability_snapshot_hash = self.sessions[idx]
            .capability_snapshot
            .as_ref()
            .map(|snapshot| {
                let hash = sha2::Sha256::digest(snapshot.as_bytes());
                format!("{:x}", hash)
            });

        let receipt = SessionReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            node_id,
            started_at,
            closed_at,
            operations_executed,
            evidence_ids,
            capability_snapshot_hash,
        };

        self.receipts.push(receipt.clone());
        self.check_expired_sessions();
        self.persist();

        if let Some(ref custody) = self.custody_service {
            let node_id = receipt.node_id.clone();
            let payload = serde_json::to_value(&receipt).unwrap_or_default();
            let metadata = CustodyMetadata {
                source: "node".to_string(),
                version: "1".to_string(),
                notes: Some("Auto-custodied on session close".to_string()),
            };
            let mut guard = custody.lock().unwrap();
            guard.append_receipt(
                &node_id,
                "session",
                &receipt.receipt_id,
                payload,
                Some(metadata),
            );
        }

        Ok(receipt)
    }

    pub fn expire_session(&mut self, session_id: &str) -> Result<Session, String> {
        let idx = self
            .sessions
            .iter()
            .position(|s| s.session_id == session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        if self.sessions[idx].state == "expired" {
            return Err(format!("Session {} is already expired", session_id));
        }

        self.sessions[idx].state = "expired".to_string();
        if self.sessions[idx].closed_at.is_none() {
            self.sessions[idx].closed_at = Some(chrono::Utc::now().to_rfc3339());
        }

        let result = self.sessions[idx].clone();
        self.persist();
        Ok(result)
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        self.sessions.iter().find(|s| s.session_id == session_id).cloned()
    }

    pub fn list_sessions(&self, state_filter: Option<&str>) -> Vec<Session> {
        match state_filter {
            Some(filter) => self
                .sessions
                .iter()
                .filter(|s| s.state == filter)
                .cloned()
                .collect(),
            None => self.sessions.clone(),
        }
    }

    pub fn get_receipt(&self, session_id: &str) -> Option<SessionReceipt> {
        self.receipts
            .iter()
            .find(|r| r.session_id == session_id)
            .cloned()
    }

    pub fn get_receipts(&self) -> Vec<SessionReceipt> {
        self.receipts.clone()
    }

    pub fn record_operation(&mut self, session_id: &str) -> Result<(), String> {
        if !self.sessions.iter().any(|s| s.session_id == session_id && s.state == "active") {
            return Err(format!(
                "No active session found for {}",
                session_id
            ));
        }
        let count = self.operation_counts.entry(session_id.to_string()).or_insert(0);
        *count += 1;
        self.persist();
        Ok(())
    }

    pub fn record_evidence(&mut self, session_id: &str, evidence_id: &str) -> Result<(), String> {
        if !self.sessions.iter().any(|s| s.session_id == session_id) {
            return Err(format!("Session {} not found", session_id));
        }
        self.evidence_by_session
            .entry(session_id.to_string())
            .or_default()
            .push(evidence_id.to_string());
        self.persist();
        Ok(())
    }

    pub fn get_expired_sessions(&self, timeout_secs: Option<u64>) -> Vec<Session> {
        let timeout = timeout_secs.unwrap_or(DEFAULT_EXPIRY_TIMEOUT_SECS);
        let now = chrono::Utc::now();
        self.sessions
            .iter()
            .filter(|s| {
                if s.state != "created" {
                    return false;
                }
                let started = match chrono::DateTime::parse_from_rfc3339(&s.started_at) {
                    Ok(t) => t,
                    Err(_) => return false,
                };
                let elapsed = now.signed_duration_since(started);
                elapsed.num_seconds() >= timeout as i64
            })
            .cloned()
            .collect()
    }

    pub fn check_expired_sessions(&mut self) {
        let expired_ids: Vec<String> = self
            .get_expired_sessions(Some(DEFAULT_EXPIRY_TIMEOUT_SECS))
            .into_iter()
            .map(|s| s.session_id.clone())
            .collect();

        for id in expired_ids {
            let _ = self.expire_session(&id);
        }
    }

    pub fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            sessions: self.sessions.clone(),
            receipts: self.receipts.clone(),
            operation_counts: self.operation_counts.clone(),
            evidence_by_session: self.evidence_by_session.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }
}

impl Serialize for SessionService {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PersistedState {
            sessions: self.sessions.clone(),
            receipts: self.receipts.clone(),
            operation_counts: self.operation_counts.clone(),
            evidence_by_session: self.evidence_by_session.clone(),
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_service() -> SessionService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        SessionService::new(path)
    }

    fn test_request(node_id: &str) -> SessionStartRequest {
        SessionStartRequest {
            node_id: node_id.to_string(),
            agent_id: Some("test-agent".to_string()),
            requested_capabilities: Some(vec!["llm.inference".to_string()]),
            context: Some("test session".to_string()),
        }
    }

    #[test]
    fn test_create_session() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        assert_eq!(session.state, "created");
        assert_eq!(session.node_id, "test-node");
        assert!(session.session_id.len() > 0);
        assert!(session.capability_snapshot.is_none());
    }

    #[test]
    fn test_activate_session() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        let activated = service.activate_session(&session.session_id).unwrap();
        assert_eq!(activated.state, "active");
    }

    #[test]
    fn test_activate_invalid_state() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        service.activate_session(&session.session_id).unwrap();
        // Cannot activate again
        assert!(service.activate_session(&session.session_id).is_err());
    }

    #[test]
    fn test_close_session() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        service.activate_session(&session.session_id).unwrap();
        let receipt = service.close_session(&session.session_id).unwrap();
        assert_eq!(receipt.session_id, session.session_id);
        assert_eq!(receipt.operations_executed, 0);
        assert!(receipt.closed_at.len() > 0);
    }

    #[test]
    fn test_close_unactivated_session() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        assert!(service.close_session(&session.session_id).is_err());
    }

    #[test]
    fn test_expire_session() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        let expired = service.expire_session(&session.session_id).unwrap();
        assert_eq!(expired.state, "expired");
        assert!(expired.closed_at.is_some());
    }

    #[test]
    fn test_double_expire_fails() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        service.expire_session(&session.session_id).unwrap();
        assert!(service.expire_session(&session.session_id).is_err());
    }

    #[test]
    fn test_get_session() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        let found = service.get_session(&session.session_id).unwrap();
        assert_eq!(found.session_id, session.session_id);
    }

    #[test]
    fn test_get_session_not_found() {
        let service = test_service();
        assert!(service.get_session("nonexistent").is_none());
    }

    #[test]
    fn test_list_sessions() {
        let mut service = test_service();
        service.create_session(test_request("node-1"));
        service.create_session(test_request("node-2"));
        assert_eq!(service.list_sessions(None).len(), 2);
        assert_eq!(service.list_sessions(Some("created")).len(), 2);
        assert_eq!(service.list_sessions(Some("active")).len(), 0);
    }

    #[test]
    fn test_get_receipt() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        service.activate_session(&session.session_id).unwrap();
        let receipt = service.close_session(&session.session_id).unwrap();

        let found = service.get_receipt(&session.session_id).unwrap();
        assert_eq!(found.receipt_id, receipt.receipt_id);
    }

    #[test]
    fn test_get_receipts() {
        let mut service = test_service();
        let s1 = service.create_session(test_request("node-1"));
        service.activate_session(&s1.session_id).unwrap();
        service.close_session(&s1.session_id).unwrap();

        let s2 = service.create_session(test_request("node-2"));
        service.activate_session(&s2.session_id).unwrap();
        service.close_session(&s2.session_id).unwrap();

        assert_eq!(service.get_receipts().len(), 2);
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_sessions.json");
        {
            let mut service = SessionService::new(path.clone());
            let session = service.create_session(test_request("test-node"));
            service.activate_session(&session.session_id).unwrap();
            service.close_session(&session.session_id).unwrap();
        }
        {
            let service = SessionService::new(path.clone());
            assert_eq!(service.sessions.len(), 1);
            assert_eq!(service.receipts.len(), 1);
            assert_eq!(service.sessions[0].state, "closed");
        }
    }

    #[test]
    fn test_record_operation() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        // Cannot record operation on non-active session
        assert!(service.record_operation(&session.session_id).is_err());

        service.activate_session(&session.session_id).unwrap();
        service.record_operation(&session.session_id).unwrap();
        service.record_operation(&session.session_id).unwrap();

        let receipt = service.close_session(&session.session_id).unwrap();
        assert_eq!(receipt.operations_executed, 2);
    }

    #[test]
    fn test_record_evidence() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        service.record_evidence(&session.session_id, "evt-001").unwrap();

        let receipt = {
            service.activate_session(&session.session_id).unwrap();
            service.close_session(&session.session_id).unwrap()
        };
        assert_eq!(receipt.evidence_ids, vec!["evt-001".to_string()]);
    }

    #[test]
    fn test_get_expired_sessions() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        // With 0 timeout, the session should be expired
        let expired = service.get_expired_sessions(Some(0));
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].session_id, session.session_id);
    }

    #[test]
    fn test_active_sessions_not_expired() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        service.activate_session(&session.session_id).unwrap();

        let expired = service.get_expired_sessions(Some(0));
        assert!(expired.is_empty());
    }

    #[test]
    fn test_created_session_eligible_for_expiry_after_timeout() {
        let mut service = test_service();
        let session = service.create_session(test_request("test-node"));
        let expired = service.get_expired_sessions(Some(0));
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].session_id, session.session_id);
    }

    #[test]
    fn test_check_expired_sessions_marks_expired() {
        let mut service = test_service();
        service.create_session(test_request("test-node"));
        let expired = service.get_expired_sessions(Some(0));
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].state, "created");
        // Manually expire
        service.expire_session(&expired[0].session_id).unwrap();
        assert_eq!(service.sessions[0].state, "expired");
    }
}
