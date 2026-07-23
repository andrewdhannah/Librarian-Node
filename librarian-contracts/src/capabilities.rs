//! # Capability Contract Types
//!
//! Capability declarations for the Librarian platform.
//! Defines what a node can do and under what authority.

use serde::{Deserialize, Serialize};

/// Schema version for capability contracts.
pub const CAPABILITY_CONTRACT_VERSION: &str = "1.0.0";

/// A declared capability — what a node can do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Unique capability identifier.
    pub capability_id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this capability provides.
    pub description: String,
    /// Category of capability.
    pub category: CapabilityCategory,
    /// Whether this capability requires explicit owner authorization.
    pub requires_authorization: bool,
    /// Whether this capability is currently enabled.
    pub enabled: bool,
    /// Schema version.
    pub schema_version: String,
}

/// Category of capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCategory {
    /// Model execution (running AI models).
    ModelExecution,
    /// File system access.
    FileSystem,
    /// Network access.
    Network,
    /// Governance operations.
    Governance,
    /// Evidence generation.
    Evidence,
    /// Receipt generation.
    Receipt,
    /// Custody operations.
    Custody,
    /// Identity management.
    Identity,
    /// Hardware qualification.
    HardwareQualification,
    /// Communication with other nodes.
    InterNodeCommunication,
    /// Information processing (ingestion, indexing, search, knowledge management).
    InformationProcessing,
}

/// A capability registry entry — complete set of capabilities for a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRegistry {
    /// Node these capabilities belong to.
    pub node_id: String,
    /// List of declared capabilities.
    pub capabilities: Vec<Capability>,
    /// ISO 8601 timestamp of last update.
    pub last_updated: String,
    /// Schema version.
    pub schema_version: String,
}

impl CapabilityRegistry {
    /// Get a capability by ID.
    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|c| c.capability_id == id)
    }

    /// Check if a capability is enabled.
    pub fn is_enabled(&self, id: &str) -> bool {
        self.get(id).map(|c| c.enabled).unwrap_or(false)
    }

    /// Count enabled capabilities.
    pub fn enabled_count(&self) -> usize {
        self.capabilities.iter().filter(|c| c.enabled).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_registry() {
        let capabilities = vec![
            Capability {
                capability_id: "model-exec".into(),
                name: "Model Execution".into(),
                description: "Run AI model inference".into(),
                category: CapabilityCategory::ModelExecution,
                requires_authorization: true,
                enabled: true,
                schema_version: CAPABILITY_CONTRACT_VERSION.into(),
            },
            Capability {
                capability_id: "file-read".into(),
                name: "File Read".into(),
                description: "Read files from local filesystem".into(),
                category: CapabilityCategory::FileSystem,
                requires_authorization: false,
                enabled: true,
                schema_version: CAPABILITY_CONTRACT_VERSION.into(),
            },
        ];

        let registry = CapabilityRegistry {
            node_id: "node-001".into(),
            capabilities,
            last_updated: "2026-07-23T00:00:00Z".into(),
            schema_version: CAPABILITY_CONTRACT_VERSION.into(),
        };

        assert!(registry.is_enabled("model-exec"));
        assert_eq!(registry.enabled_count(), 2);
        assert!(registry.get("model-exec").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_capability_serde() {
        let cap = Capability {
            capability_id: "test-cap".into(),
            name: "Test Capability".into(),
            description: "A test capability".into(),
            category: CapabilityCategory::Evidence,
            requires_authorization: false,
            enabled: true,
            schema_version: CAPABILITY_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap.capability_id, deserialized.capability_id);
        assert_eq!(cap.category, deserialized.category);
    }

    #[test]
    fn test_information_processing_category() {
        let cap = Capability {
            capability_id: "conversation.import".into(),
            name: "Import Conversations".into(),
            description: "Import Claude/ChatGPT conversation exports".into(),
            category: CapabilityCategory::InformationProcessing,
            requires_authorization: true,
            enabled: true,
            schema_version: CAPABILITY_CONTRACT_VERSION.into(),
        };
        assert!(matches!(cap.category, CapabilityCategory::InformationProcessing));
        let json = serde_json::to_string(&cap).unwrap();
        assert!(json.contains("information_processing"));
    }
}
