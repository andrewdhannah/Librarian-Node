//! # Identity Contract Types
//!
//! Node and platform identity types for the Librarian platform.
//! Maps to Swift `NodeRole`, platform identity, and node registration models.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Schema version for identity contracts.
pub const IDENTITY_CONTRACT_VERSION: &str = "1.0.0";

/// Known node roles from the node-role authority schema.
/// Maps to Swift `NodeRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    /// The Librarian Core — canonical authority.
    LibrarianAuthority,
    /// Client node — consumes services.
    Client,
    /// Worker node — executes tasks.
    Worker,
    /// Runtime node — hosts model execution.
    Runtime,
    /// Router bridge — proxies between nodes.
    RouterBridge,
    /// Verifier node — validates evidence and receipts.
    Verifier,
    /// Receipt producer — generates governance receipts.
    ReceiptProducer,
}

impl NodeRole {
    /// All known node roles.
    pub const ALL: &'static [NodeRole] = &[
        NodeRole::LibrarianAuthority,
        NodeRole::Client,
        NodeRole::Worker,
        NodeRole::Runtime,
        NodeRole::RouterBridge,
        NodeRole::Verifier,
        NodeRole::ReceiptProducer,
    ];

    /// Human-readable label for this role.
    pub fn label(&self) -> &'static str {
        match self {
            NodeRole::LibrarianAuthority => "Librarian Authority",
            NodeRole::Client => "Client",
            NodeRole::Worker => "Worker",
            NodeRole::Runtime => "Runtime",
            NodeRole::RouterBridge => "Router Bridge",
            NodeRole::Verifier => "Verifier",
            NodeRole::ReceiptProducer => "Receipt Producer",
        }
    }
}

impl fmt::Display for NodeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Target platform for a component.
/// Maps to Swift platform identification (macOS, Windows, Linux).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformId {
    /// macOS (Apple Silicon or Intel)
    MacOS,
    /// Windows (x86_64)
    Windows,
    /// Linux (x86_64, aarch64)
    Linux,
}

impl PlatformId {
    /// All known platforms.
    pub const ALL: &'static [PlatformId] = &[
        PlatformId::MacOS,
        PlatformId::Windows,
        PlatformId::Linux,
    ];

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            PlatformId::MacOS => "macOS",
            PlatformId::Windows => "Windows",
            PlatformId::Linux => "Linux",
        }
    }
}

impl fmt::Display for PlatformId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Hardware architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    /// Apple Silicon
    Arm64,
    /// 64-bit x86
    X8664,
}

/// A unique node identifier.
/// Maps to Swift `NodeId` / UUID-based identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new NodeId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        NodeId(id.into())
    }

    /// Reference the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        NodeId(s)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        NodeId(s.to_string())
    }
}

/// Full node identity.
/// Maps to Swift node registration / project profile models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeIdentity {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable node name.
    pub display_name: String,
    /// Node's role in the platform.
    pub role: NodeRole,
    /// Target platform.
    pub platform: PlatformId,
    /// Hardware architecture.
    pub architecture: Architecture,
    /// Software version.
    pub version: String,
    /// Contract version this node implements.
    pub contract_version: String,
}

/// Identity claim — a signed identity assertion for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityClaim {
    /// The node making the claim.
    pub node_id: NodeId,
    /// Claimed role.
    pub role: NodeRole,
    /// Claimed platform.
    pub platform: PlatformId,
    /// Timestamp of claim (ISO 8601).
    pub claimed_at: String,
    /// Signature (hex-encoded).
    pub signature: String,
    /// Schema version.
    pub schema_version: String,
}

impl Default for IdentityClaim {
    fn default() -> Self {
        Self {
            node_id: NodeId::new(""),
            role: NodeRole::Client,
            platform: PlatformId::MacOS,
            claimed_at: String::new(),
            signature: String::new(),
            schema_version: IDENTITY_CONTRACT_VERSION.to_string(),
        }
    }
}

/// A registered project profile (from Swift `ProjectProfile`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProfile {
    pub project_id: String,
    pub project_name: String,
    pub profile_id: Option<String>,
    pub source_kind: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_role_all() {
        assert!(NodeRole::ALL.contains(&NodeRole::LibrarianAuthority));
        assert!(NodeRole::ALL.contains(&NodeRole::Runtime));
        assert_eq!(NodeRole::ALL.len(), 7);
    }

    #[test]
    fn test_platform_all() {
        assert!(PlatformId::ALL.contains(&PlatformId::MacOS));
        assert!(PlatformId::ALL.contains(&PlatformId::Windows));
        assert!(PlatformId::ALL.contains(&PlatformId::Linux));
    }

    #[test]
    fn test_node_identity_serde() {
        let id = NodeIdentity {
            node_id: NodeId::new("test-node-001"),
            display_name: "Test Node".into(),
            role: NodeRole::Runtime,
            platform: PlatformId::Windows,
            architecture: Architecture::X8664,
            version: "0.1.0".into(),
            contract_version: IDENTITY_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: NodeIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(id.node_id, deserialized.node_id);
        assert_eq!(id.role, deserialized.role);
        assert_eq!(id.platform, deserialized.platform);
    }

    #[test]
    fn test_identity_claim_default() {
        let claim = IdentityClaim::default();
        assert_eq!(claim.schema_version, IDENTITY_CONTRACT_VERSION);
    }
}
