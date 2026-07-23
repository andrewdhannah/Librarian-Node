use std::path::PathBuf;

use librarian_contracts::core_integration::{
    DiscoveryAnnouncement, DiscoveryResponse, NodeProjection, SyncReceipt, SyncRequest,
};
use librarian_contracts::node::NodeIdentity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    last_sync_at: Option<String>,
    discovery_registered: bool,
    sync_attempts: Vec<SyncHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHistoryEntry {
    pub request_id: String,
    pub attempted_at: String,
    pub receipt: Option<SyncReceipt>,
}

pub struct CoreIntegrationService {
    last_sync_at: Option<String>,
    core_endpoint: Option<String>,
    discovery_registered: bool,
    persistence_path: PathBuf,
    sync_attempts: Vec<SyncHistoryEntry>,
}

impl CoreIntegrationService {
    pub fn new(
        persistence_path: impl Into<PathBuf>,
        core_endpoint: Option<String>,
    ) -> Self {
        let persistence_path = persistence_path.into();
        let (last_sync_at, discovery_registered, sync_attempts) =
            if persistence_path.exists() {
                match std::fs::read_to_string(&persistence_path) {
                    Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                        Ok(state) => {
                            (state.last_sync_at, state.discovery_registered, state.sync_attempts)
                        }
                        Err(_) => (None, false, Vec::new()),
                    },
                    Err(_) => (None, false, Vec::new()),
                }
            } else {
                (None, false, Vec::new())
            };

        CoreIntegrationService {
            last_sync_at,
            core_endpoint,
            discovery_registered,
            persistence_path,
            sync_attempts,
        }
    }

    pub fn generate_projection(
        &self,
        identity: &NodeIdentity,
        registration: Option<serde_json::Value>,
        capabilities: Option<serde_json::Value>,
        capabilities_verified: bool,
        session_count: u32,
        bootstrap_completed: bool,
        custody_envelope_count: u32,
        last_integrity_hash: Option<String>,
    ) -> NodeProjection {
        let status = if self.core_endpoint.is_some() {
            "online"
        } else {
            "offline"
        };

        NodeProjection {
            projection_id: Uuid::new_v4().to_string(),
            node_id: identity.node_id.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            node_version: identity.runtime_version.clone(),
            identity: serde_json::to_value(identity).unwrap_or_default(),
            registration,
            capabilities,
            capabilities_verified,
            session_count,
            bootstrap_completed,
            custody_envelope_count,
            last_integrity_hash,
            status: status.to_string(),
        }
    }

    pub fn prepare_sync(&mut self, projection: NodeProjection, identity: &NodeIdentity) -> SyncRequest {
        let request = SyncRequest {
            request_id: Uuid::new_v4().to_string(),
            node_id: identity.node_id.clone(),
            node_version: identity.runtime_version.clone(),
            last_sync_at: self.last_sync_at.clone(),
            projection,
            requested_at: chrono::Utc::now().to_rfc3339(),
        };

        self.record_sync_attempt(&request);
        request
    }

    pub fn process_sync_receipt(&mut self, receipt: SyncReceipt) -> Result<(), String> {
        self.last_sync_at = Some(receipt.processed_at.clone());

        if let Some(entry) = self
            .sync_attempts
            .iter_mut()
            .find(|e| e.request_id == receipt.request_id)
        {
            entry.receipt = Some(receipt);
        }

        self.persist();
        Ok(())
    }

    pub fn record_sync_attempt(&mut self, sync_request: &SyncRequest) {
        self.sync_attempts.push(SyncHistoryEntry {
            request_id: sync_request.request_id.clone(),
            attempted_at: sync_request.requested_at.clone(),
            receipt: None,
        });
        self.persist();
    }

    pub fn record_sync_result(&mut self, receipt: SyncReceipt) {
        self.last_sync_at = Some(receipt.processed_at.clone());
        if let Some(entry) = self
            .sync_attempts
            .iter_mut()
            .find(|e| e.request_id == receipt.request_id)
        {
            entry.receipt = Some(receipt);
        }
        self.persist();
    }

    pub fn create_announcement(&self, identity: &NodeIdentity) -> DiscoveryAnnouncement {
        DiscoveryAnnouncement {
            node_id: identity.node_id.clone(),
            display_name: identity.display_name.clone(),
            node_version: identity.runtime_version.clone(),
            announced_at: chrono::Utc::now().to_rfc3339(),
            available: self.core_endpoint.is_some(),
            endpoint: self.core_endpoint.clone(),
        }
    }

    pub fn process_discovery_response(&mut self, response: DiscoveryResponse) -> Result<(), String> {
        match response.status.as_str() {
            "known" | "new" => {
                self.discovery_registered = true;
                self.persist();
                Ok(())
            }
            "rejected" => {
                self.discovery_registered = false;
                self.persist();
                Err("Discovery rejected by Core".to_string())
            }
            _ => Err(format!("Unknown discovery status: {}", response.status)),
        }
    }

    pub fn is_online(&self) -> bool {
        self.core_endpoint.is_some()
    }

    pub fn get_unsynced_envelope_count(&self) -> u32 {
        0 // Envelope count is provided externally via generate_projection
    }

    pub fn get_last_sync_at(&self) -> Option<String> {
        self.last_sync_at.clone()
    }

    pub fn get_discovery_registered(&self) -> bool {
        self.discovery_registered
    }

    pub fn get_sync_attempts(&self) -> &[SyncHistoryEntry] {
        &self.sync_attempts
    }

    pub fn set_core_endpoint(&mut self, endpoint: Option<String>) {
        self.core_endpoint = endpoint;
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            last_sync_at: self.last_sync_at.clone(),
            discovery_registered: self.discovery_registered,
            sync_attempts: self.sync_attempts.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }
}

#[cfg(test)]
fn test_identity() -> NodeIdentity {
    NodeIdentity {
        node_id: "test-node-uuid".to_string(),
        display_name: "test-host".to_string(),
        platform: "test".to_string(),
        runtime_version: "0.1.0".to_string(),
        contract_version: "1".to_string(),
        first_seen_at: "2026-07-15T12:00:00Z".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::core_integration::SyncError;
    use tempfile::tempdir;

    fn test_service() -> CoreIntegrationService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("core-integration.json");
        CoreIntegrationService::new(path, None)
    }

    fn test_service_online() -> CoreIntegrationService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("core-integration.json");
        CoreIntegrationService::new(path, Some("http://core:8080".to_string()))
    }

    #[test]
    fn test_new_service_offline_by_default() {
        let service = test_service();
        assert!(!service.is_online());
        assert!(service.get_last_sync_at().is_none());
        assert!(!service.get_discovery_registered());
        assert!(service.get_sync_attempts().is_empty());
    }

    #[test]
    fn test_new_service_online_when_configured() {
        let service = test_service_online();
        assert!(service.is_online());
    }

    #[test]
    fn test_generate_projection_contains_all_fields() {
        let service = test_service();
        let identity = test_identity();

        let projection = service.generate_projection(
            &identity,
            Some(serde_json::json!({"status": "registered"})),
            Some(serde_json::json!({"capabilities": [{"type": "llm.inference", "available": true}]})),
            true,
            3,
            true,
            5,
            Some("abc123".to_string()),
        );

        assert_eq!(projection.node_id, "test-node-uuid");
        assert_eq!(projection.node_version, "0.1.0");
        assert_eq!(projection.status, "offline");
        assert!(projection.capabilities_verified);
        assert_eq!(projection.session_count, 3);
        assert!(projection.bootstrap_completed);
        assert_eq!(projection.custody_envelope_count, 5);
        assert_eq!(projection.last_integrity_hash, Some("abc123".to_string()));
        assert!(projection.registration.is_some());
        assert!(projection.capabilities.is_some());
        assert!(!projection.projection_id.is_empty());
        assert!(!projection.generated_at.is_empty());
    }

    #[test]
    fn test_generate_projection_offline_when_no_core_endpoint() {
        let service = test_service();
        let projection = service.generate_projection(
            &test_identity(),
            None, None, false, 0, false, 0, None,
        );
        assert_eq!(projection.status, "offline");
    }

    #[test]
    fn test_generate_projection_online_when_core_endpoint_configured() {
        let service = test_service_online();
        let projection = service.generate_projection(
            &test_identity(),
            None, None, false, 0, false, 0, None,
        );
        assert_eq!(projection.status, "online");
    }

    #[test]
    fn test_prepare_sync_creates_request_with_projection() {
        let mut service = test_service();
        let identity = test_identity();
        let projection = service.generate_projection(
            &identity, None, None, false, 0, false, 0, None,
        );

        let request = service.prepare_sync(projection.clone(), &identity);

        assert_eq!(request.node_id, "test-node-uuid");
        assert_eq!(request.node_version, "0.1.0");
        assert!(request.last_sync_at.is_none());
        assert_eq!(request.projection.projection_id, projection.projection_id);
        assert!(!request.request_id.is_empty());
        assert!(!request.requested_at.is_empty());
    }

    #[test]
    fn test_prepare_sync_records_sync_attempt() {
        let mut service = test_service();
        let identity = test_identity();
        let projection = service.generate_projection(
            &identity, None, None, false, 0, false, 0, None,
        );

        assert!(service.get_sync_attempts().is_empty());
        service.prepare_sync(projection, &identity);
        assert_eq!(service.get_sync_attempts().len(), 1);

        let attempt = &service.get_sync_attempts()[0];
        assert!(!attempt.request_id.is_empty());
        assert!(attempt.receipt.is_none());
    }

    #[test]
    fn test_process_sync_receipt_updates_last_sync_at() {
        let mut service = test_service();
        let identity = test_identity();
        let projection = service.generate_projection(
            &identity, None, None, false, 0, false, 0, None,
        );

        let request = service.prepare_sync(projection, &identity);

        let receipt = SyncReceipt {
            receipt_id: "receipt-001".to_string(),
            request_id: request.request_id.clone(),
            node_id: "test-node-uuid".to_string(),
            status: "accepted".to_string(),
            accepted_envelopes: 3,
            rejected_envelopes: 0,
            errors: vec![],
            processed_at: "2026-07-15T12:30:00Z".to_string(),
        };

        let result = service.process_sync_receipt(receipt);
        assert!(result.is_ok());
        assert_eq!(service.get_last_sync_at(), Some("2026-07-15T12:30:00Z".to_string()));

        // Verify receipt is attached to the attempt
        let attempt = &service.get_sync_attempts()[0];
        assert!(attempt.receipt.is_some());
        assert_eq!(attempt.receipt.as_ref().unwrap().status, "accepted");
    }

    #[test]
    fn test_process_sync_receipt_with_errors() {
        let mut service = test_service();
        let identity = test_identity();
        let projection = service.generate_projection(
            &identity, None, None, false, 0, false, 0, None,
        );

        let request = service.prepare_sync(projection, &identity);

        let receipt = SyncReceipt {
            receipt_id: "receipt-002".to_string(),
            request_id: request.request_id.clone(),
            node_id: "test-node-uuid".to_string(),
            status: "partial".to_string(),
            accepted_envelopes: 2,
            rejected_envelopes: 1,
            errors: vec![
                SyncError {
                    envelope_id: "env-003".to_string(),
                    reason: "integrity_failure".to_string(),
                },
            ],
            processed_at: "2026-07-15T12:31:00Z".to_string(),
        };

        let result = service.process_sync_receipt(receipt);
        assert!(result.is_ok());
        assert_eq!(service.get_last_sync_at(), Some("2026-07-15T12:31:00Z".to_string()));
    }

    #[test]
    fn test_record_sync_result_updates_state() {
        let mut service = test_service();
        let identity = test_identity();
        let projection = service.generate_projection(
            &identity, None, None, false, 0, false, 0, None,
        );

        let request = service.prepare_sync(projection, &identity);

        let receipt = SyncReceipt {
            receipt_id: "receipt-003".to_string(),
            request_id: request.request_id.clone(),
            node_id: "test-node-uuid".to_string(),
            status: "accepted".to_string(),
            accepted_envelopes: 5,
            rejected_envelopes: 0,
            errors: vec![],
            processed_at: "2026-07-15T12:32:00Z".to_string(),
        };

        service.record_sync_result(receipt);
        assert_eq!(service.get_last_sync_at(), Some("2026-07-15T12:32:00Z".to_string()));
    }

    #[test]
    fn test_create_announcement_from_identity() {
        let service = test_service();
        let identity = test_identity();
        let announcement = service.create_announcement(&identity);

        assert_eq!(announcement.node_id, "test-node-uuid");
        assert_eq!(announcement.display_name, "test-host");
        assert_eq!(announcement.node_version, "0.1.0");
        assert!(!announcement.announced_at.is_empty());
        assert!(!announcement.available); // no endpoint configured
        assert!(announcement.endpoint.is_none());
    }

    #[test]
    fn test_create_announcement_with_endpoint() {
        let service = test_service_online();
        let identity = test_identity();
        let announcement = service.create_announcement(&identity);

        assert!(announcement.available);
        assert_eq!(announcement.endpoint, Some("http://core:8080".to_string()));
    }

    #[test]
    fn test_process_discovery_response_known() {
        let mut service = test_service();
        let response = DiscoveryResponse {
            node_id: "test-node-uuid".to_string(),
            status: "known".to_string(),
            core_version: Some("1.0.0".to_string()),
            contracts_version: Some("1".to_string()),
        };

        let result = service.process_discovery_response(response);
        assert!(result.is_ok());
        assert!(service.get_discovery_registered());
    }

    #[test]
    fn test_process_discovery_response_new() {
        let mut service = test_service();
        let response = DiscoveryResponse {
            node_id: "test-node-uuid".to_string(),
            status: "new".to_string(),
            core_version: Some("1.0.0".to_string()),
            contracts_version: Some("1".to_string()),
        };

        let result = service.process_discovery_response(response);
        assert!(result.is_ok());
        assert!(service.get_discovery_registered());
    }

    #[test]
    fn test_process_discovery_response_rejected() {
        let mut service = test_service();
        let response = DiscoveryResponse {
            node_id: "test-node-uuid".to_string(),
            status: "rejected".to_string(),
            core_version: None,
            contracts_version: None,
        };

        let result = service.process_discovery_response(response);
        assert!(result.is_err());
        assert!(!service.get_discovery_registered());
    }

    #[test]
    fn test_process_discovery_response_unknown_status() {
        let mut service = test_service();
        let response = DiscoveryResponse {
            node_id: "test-node-uuid".to_string(),
            status: "pending".to_string(),
            core_version: None,
            contracts_version: None,
        };

        let result = service.process_discovery_response(response);
        assert!(result.is_err());
        assert!(!service.get_discovery_registered());
    }

    #[test]
    fn test_offline_behavior_works_without_core() {
        let mut service = test_service();
        let identity = test_identity();

        // All methods work locally without Core
        let projection = service.generate_projection(
            &identity, None, None, false, 0, false, 0, None,
        );
        assert_eq!(projection.status, "offline");

        let request = service.prepare_sync(projection, &identity);
        assert_eq!(request.node_id, "test-node-uuid");

        // Sync receipt still processes fine
        let receipt = SyncReceipt {
            receipt_id: "offline-receipt".to_string(),
            request_id: request.request_id.clone(),
            node_id: "test-node-uuid".to_string(),
            status: "accepted".to_string(),
            accepted_envelopes: 0,
            rejected_envelopes: 0,
            errors: vec![],
            processed_at: "2026-07-15T13:00:00Z".to_string(),
        };
        assert!(service.process_sync_receipt(receipt).is_ok());

        // Discovery works offline
        let announcement = service.create_announcement(&identity);
        assert!(!announcement.announced_at.is_empty());
    }

    #[test]
    fn test_set_core_endpoint_changes_online_status() {
        let mut service = test_service();
        assert!(!service.is_online());

        service.set_core_endpoint(Some("http://core:9090".to_string()));
        assert!(service.is_online());

        service.set_core_endpoint(None);
        assert!(!service.is_online());
    }

    #[test]
    fn test_multiple_sync_attempts_tracked() {
        let mut service = test_service();
        let identity = test_identity();

        let p1 = service.generate_projection(&identity, None, None, false, 0, false, 0, None);
        let p2 = service.generate_projection(&identity, None, None, false, 0, false, 0, None);

        service.prepare_sync(p1, &identity);
        service.prepare_sync(p2, &identity);

        assert_eq!(service.get_sync_attempts().len(), 2);
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("core-integration-persist.json");
        let identity = test_identity();

        let request_id;
        {
            let mut service = CoreIntegrationService::new(path.clone(), None);
            let projection = service.generate_projection(
                &identity, None, None, false, 0, false, 0, None,
            );
            let request = service.prepare_sync(projection, &identity);
            request_id = request.request_id.clone();

            let receipt = SyncReceipt {
                receipt_id: "persist-receipt".to_string(),
                request_id: request_id.clone(),
                node_id: "test-node-uuid".to_string(),
                status: "accepted".to_string(),
                accepted_envelopes: 1,
                rejected_envelopes: 0,
                errors: vec![],
                processed_at: "2026-07-15T14:00:00Z".to_string(),
            };
            service.process_sync_receipt(receipt).unwrap();
        }

        {
            let service = CoreIntegrationService::new(path.clone(), None);
            assert_eq!(service.get_last_sync_at(), Some("2026-07-15T14:00:00Z".to_string()));
            assert_eq!(service.get_sync_attempts().len(), 1);
            assert_eq!(service.get_sync_attempts()[0].request_id, request_id);
            assert!(service.get_sync_attempts()[0].receipt.is_some());
        }
    }

    #[test]
    fn test_identity_fields_included_in_projection() {
        let service = test_service();
        let identity = test_identity();
        let projection = service.generate_projection(
            &identity, None, None, false, 0, false, 0, None,
        );

        let ident_val = projection.identity.as_object().unwrap();
        assert_eq!(ident_val["node_id"], "test-node-uuid");
        assert_eq!(ident_val["display_name"], "test-host");
        assert_eq!(ident_val["platform"], "test");
        assert_eq!(ident_val["runtime_version"], "0.1.0");
    }

    #[test]
    fn test_prepare_sync_includes_last_sync_at() {
        let mut service = test_service();
        let identity = test_identity();

        let p1 = service.generate_projection(&identity, None, None, false, 0, false, 0, None);
        let r1 = service.prepare_sync(p1, &identity);

        // Process receipt to set last_sync_at
        let receipt = SyncReceipt {
            receipt_id: "sync-1-receipt".to_string(),
            request_id: r1.request_id.clone(),
            node_id: "test-node-uuid".to_string(),
            status: "accepted".to_string(),
            accepted_envelopes: 0,
            rejected_envelopes: 0,
            errors: vec![],
            processed_at: "2026-07-15T15:00:00Z".to_string(),
        };
        service.process_sync_receipt(receipt).unwrap();

        let p2 = service.generate_projection(&identity, None, None, false, 0, false, 0, None);
        let r2 = service.prepare_sync(p2, &identity);
        assert_eq!(r2.last_sync_at, Some("2026-07-15T15:00:00Z".to_string()));
    }

    #[test]
    fn test_get_unsynced_envelope_count_returns_zero() {
        let service = test_service();
        assert_eq!(service.get_unsynced_envelope_count(), 0);
    }
}
