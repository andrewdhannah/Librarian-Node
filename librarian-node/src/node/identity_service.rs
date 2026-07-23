use librarian_contracts::node::NodeIdentity;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub struct NodeIdentityService {
    identity: NodeIdentity,
    #[allow(dead_code)]
    config_path: PathBuf,
}

impl NodeIdentityService {
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        let config_path = config_path.into();
        let identity = load_or_create_identity(&config_path);
        info!(
            "Node identity: {} ({}) on {}",
            identity.display_name, identity.node_id, identity.platform
        );
        NodeIdentityService { identity, config_path }
    }

    pub fn get_identity(&self) -> &NodeIdentity {
        &self.identity
    }
}

fn load_or_create_identity(path: &Path) -> NodeIdentity {
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<NodeIdentity>(&content) {
                Ok(identity) => {
                    info!("Loaded existing node identity from {}", path.display());
                    return identity;
                }
                Err(e) => {
                    warn!(
                        "Corrupted node identity file at {}: {}. Regenerating.",
                        path.display(),
                        e
                    );
                }
            },
            Err(e) => {
                warn!(
                    "Failed to read node identity file at {}: {}. Regenerating.",
                    path.display(),
                    e
                );
            }
        }
    }

    let identity = generate_identity();
    persist_identity(path, &identity);
    info!("Generated new node identity: {}", identity.node_id);
    identity
}

fn generate_identity() -> NodeIdentity {
    let hostname = detect_hostname();
    NodeIdentity {
        node_id: uuid::Uuid::new_v4().to_string(),
        display_name: hostname,
        platform: std::env::consts::OS.to_string(),
        runtime_version: env!("CARGO_PKG_VERSION").to_string(),
        contract_version: "1".to_string(),
        first_seen_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn persist_identity(path: &Path, identity: &NodeIdentity) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(identity).expect("Failed to serialize identity");
    match std::fs::write(path, &json) {
        Ok(_) => info!("Persisted node identity to {}", path.display()),
        Err(e) => warn!("Failed to persist node identity to {}: {}", path.display(), e),
    }
}

fn detect_hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_identity_generates_uuid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-identity.json");
        let service = NodeIdentityService::new(&path);
        let identity = service.get_identity();
        assert!(!identity.node_id.is_empty());
        assert_eq!(identity.platform, std::env::consts::OS);
        assert_eq!(identity.contract_version, "1");
    }

    #[test]
    fn test_identity_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-identity.json");

        let service1 = NodeIdentityService::new(&path);
        let id1 = service1.get_identity().node_id.clone();

        let service2 = NodeIdentityService::new(&path);
        let id2 = service2.get_identity().node_id.clone();

        assert_eq!(id1, id2, "Identity must survive restart");
    }

    #[test]
    fn test_corrupted_identity_regenerates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-identity.json");

        std::fs::write(&path, "not valid json").unwrap();

        let service = NodeIdentityService::new(&path);
        let identity = service.get_identity();
        assert!(!identity.node_id.is_empty());
        // File should now contain valid JSON
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: NodeIdentity = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.node_id, identity.node_id);
    }

    #[test]
    fn test_display_name_defaults_to_hostname() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("node-identity.json");
        let service = NodeIdentityService::new(&path);
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown-host".to_string());
        assert_eq!(service.get_identity().display_name, hostname);
    }
}
