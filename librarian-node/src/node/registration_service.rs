use std::sync::Arc;

use librarian_contracts::custody::CustodyMetadata;
use librarian_contracts::node::{NodeRecord, RegistrationReceipt, RegistrationRequest};
use sha2::Digest;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use super::CustodyService;

pub struct RegistrationService {
    node_record: NodeRecord,
    persistence_path: PathBuf,
    custody_service: Option<Arc<std::sync::Mutex<CustodyService>>>,
}

impl RegistrationService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let node_record = load_or_create_record(&persistence_path);
        info!(
            "Registration service initialized: status={}",
            node_record.registration_status
        );
        RegistrationService {
            node_record,
            persistence_path,
            custody_service: None,
        }
    }

    pub fn with_custody(mut self, custody: Arc<std::sync::Mutex<CustodyService>>) -> Self {
        self.custody_service = Some(custody);
        self
    }

    pub fn get_record(&self) -> &NodeRecord {
        &self.node_record
    }

    pub fn get_record_mut(&mut self) -> &mut NodeRecord {
        &mut self.node_record
    }

    pub fn submit_registration(
        &mut self,
        identity: &librarian_contracts::node::NodeIdentity,
        capabilities_json: Option<String>,
    ) -> RegistrationRequest {
        let requested_at = chrono::Utc::now().to_rfc3339();
        let capabilities_hash = capabilities_json.as_ref().map(|cj| {
            let hash = sha2::Sha256::digest(cj.as_bytes());
            format!("{:x}", hash)
        });

        let hostname = detect_hostname();
        self.node_record.node_id = identity.node_id.clone();
        self.node_record.display_name = identity.display_name.clone();
        self.node_record.hostname = hostname.clone();
        self.node_record.platform = identity.platform.clone();
        self.node_record.runtime_version = identity.runtime_version.clone();
        self.node_record.registration_status = "registration_requested".to_string();
        self.node_record.last_seen_at = Some(requested_at.clone());

        let request = RegistrationRequest {
            node_id: identity.node_id.clone(),
            display_name: identity.display_name.clone(),
            hostname,
            platform: identity.platform.clone(),
            runtime_version: identity.runtime_version.clone(),
            capabilities_hash,
            requested_at: requested_at.clone(),
        };

        self.persist();
        info!("Registration submitted for node {}", identity.node_id);
        request
    }

    pub fn confirm_registration(&mut self, receipt: &RegistrationReceipt) {
        self.node_record.registration_status = receipt.status.clone();
        self.node_record.first_registered_at = self
            .node_record
            .first_registered_at
            .take()
            .or(Some(receipt.registered_at.clone()));
        self.node_record.last_seen_at = Some(receipt.registered_at.clone());
        self.persist();
        info!(
            "Registration confirmed for node {}: status={}",
            self.node_record.node_id, receipt.status
        );

        if let Some(ref custody) = self.custody_service {
            let node_id = receipt.node_id.clone();
            let payload = serde_json::to_value(receipt).unwrap_or_default();
            let metadata = CustodyMetadata {
                source: "node".to_string(),
                version: "1".to_string(),
                notes: Some("Auto-custodied on registration confirmation".to_string()),
            };
            let mut guard = custody.lock().unwrap();
            guard.append_receipt(
                &node_id,
                "registration",
                &receipt.registration_id,
                payload,
                Some(metadata),
            );
        }
    }

    pub fn suspend(&mut self) {
        self.node_record.registration_status = "suspended".to_string();
        self.node_record.last_seen_at = Some(chrono::Utc::now().to_rfc3339());
        self.persist();
        info!("Node {} suspended", self.node_record.node_id);
    }

    pub fn retire(&mut self) {
        self.node_record.registration_status = "retired".to_string();
        self.node_record.last_seen_at = Some(chrono::Utc::now().to_rfc3339());
        self.persist();
        info!("Node {} retired", self.node_record.node_id);
    }

    pub fn update_capabilities_snapshot(&mut self, snapshot: String) {
        self.node_record.capabilities_snapshot = Some(snapshot);
        self.node_record.last_seen_at = Some(chrono::Utc::now().to_rfc3339());
        self.persist();
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json =
            serde_json::to_string_pretty(&self.node_record).expect("Failed to serialize record");
        match std::fs::write(&self.persistence_path, &json) {
            Ok(_) => {
                tracing::trace!("Persisted node registration to {}", self.persistence_path.display())
            }
            Err(e) => warn!(
                "Failed to persist node registration to {}: {}",
                self.persistence_path.display(),
                e
            ),
        }
    }
}

fn load_or_create_record(path: &Path) -> NodeRecord {
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<NodeRecord>(&content) {
                Ok(record) => {
                    info!("Loaded existing node registration record from {}", path.display());
                    return record;
                }
                Err(e) => {
                    warn!(
                        "Corrupted node registration record at {}: {}. Creating new record.",
                        path.display(),
                        e
                    );
                }
            },
            Err(e) => {
                warn!(
                    "Failed to read node registration record at {}: {}. Creating new record.",
                    path.display(),
                    e
                );
            }
        }
    }

    let record = NodeRecord {
        node_id: String::new(),
        display_name: String::new(),
        hostname: String::new(),
        platform: String::new(),
        runtime_version: String::new(),
        registration_status: "unregistered".to_string(),
        first_registered_at: None,
        last_seen_at: None,
        capabilities_snapshot: None,
    };

    // Persist the empty record so the file exists
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(&record).expect("Failed to serialize record");
    let _ = std::fs::write(path, &json);

    info!("Created new unregistered node registration record at {}", path.display());
    record
}

fn detect_hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::node::NodeIdentity;
    use tempfile::tempdir;

    fn test_identity() -> NodeIdentity {
        NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_new_service_creates_unregistered_record() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-registration.json");
        let service = RegistrationService::new(&path);
        assert_eq!(service.get_record().registration_status, "unregistered");
    }

    #[test]
    fn test_submit_registration_creates_request() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-registration.json");
        let mut service = RegistrationService::new(&path);
        let identity = test_identity();

        let request = service.submit_registration(&identity, None);
        assert_eq!(request.node_id, "test-node-uuid");
        assert_eq!(request.display_name, "test-host");
        assert!(request.requested_at.len() > 0);
        assert_eq!(service.get_record().registration_status, "registration_requested");
    }

    #[test]
    fn test_confirm_registration_transitions_to_registered() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-registration.json");
        let mut service = RegistrationService::new(&path);
        let identity = test_identity();

        service.submit_registration(&identity, None);

        let receipt = RegistrationReceipt {
            registration_id: "reg-001".to_string(),
            node_id: "test-node-uuid".to_string(),
            status: "registered".to_string(),
            registered_at: chrono::Utc::now().to_rfc3339(),
            previous_state: Some("registration_requested".to_string()),
        };

        service.confirm_registration(&receipt);
        assert_eq!(service.get_record().registration_status, "registered");
        assert!(service.get_record().first_registered_at.is_some());
    }

    #[test]
    fn test_suspend_and_retire() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-registration.json");
        let mut service = RegistrationService::new(&path);
        let identity = test_identity();

        service.submit_registration(&identity, None);
        let receipt = RegistrationReceipt {
            registration_id: "reg-001".to_string(),
            node_id: "test-node-uuid".to_string(),
            status: "registered".to_string(),
            registered_at: chrono::Utc::now().to_rfc3339(),
            previous_state: Some("registration_requested".to_string()),
        };
        service.confirm_registration(&receipt);

        service.suspend();
        assert_eq!(service.get_record().registration_status, "suspended");

        // Cannot transition from suspended to retired via registration service (no direct method)
        // But we can check that the service allows setting to retired
        service.retire();
        assert_eq!(service.get_record().registration_status, "retired");
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-registration.json");

        let mut service1 = RegistrationService::new(&path);
        let identity = test_identity();
        service1.submit_registration(&identity, None);
        let receipt = RegistrationReceipt {
            registration_id: "reg-001".to_string(),
            node_id: "test-node-uuid".to_string(),
            status: "registered".to_string(),
            registered_at: chrono::Utc::now().to_rfc3339(),
            previous_state: Some("registration_requested".to_string()),
        };
        service1.confirm_registration(&receipt);
        assert_eq!(service1.get_record().registration_status, "registered");

        // Simulate restart
        let service2 = RegistrationService::new(&path);
        assert_eq!(service2.get_record().registration_status, "registered");
        assert_eq!(service2.get_record().node_id, "test-node-uuid");
    }

    #[test]
    fn test_capabilities_snapshot() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-registration.json");
        let mut service = RegistrationService::new(&path);

        let snapshot = r#"{"capabilities":[{"type":"llm.inference","available":true}]}"#;
        service.update_capabilities_snapshot(snapshot.to_string());
        assert!(service.get_record().capabilities_snapshot.is_some());
        assert_eq!(
            service.get_record().capabilities_snapshot.as_deref(),
            Some(snapshot)
        );
    }
}
