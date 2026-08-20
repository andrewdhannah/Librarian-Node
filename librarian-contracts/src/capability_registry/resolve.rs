//! # Capability Registry — Resolution Contract
//!
//! Request/response types for `capability_registry_resolve`.
//!
//! **Contract invariant (CR-R-002):** Resolution does NOT deliver capability content.
//! Use `load` after successful resolution.

use serde::{Deserialize, Serialize};

/// Request for capability_registry_resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveRequest {
    /// Capability identifier (e.g., "frontend-design").
    pub capability_id: String,
    /// Optional specific version. Defaults to active_version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
}

/// Result of a capability resolution preflight check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResolution {
    /// Capability identifier.
    pub capability_id: String,
    /// Human-readable name.
    pub name: String,
    /// Capability type.
    #[serde(rename = "type")]
    pub cap_type: String,
    /// Resolved version number.
    pub version: i32,
    /// SHA-256 content hash of the resolved version.
    pub content_hash: String,
    /// Current lifecycle status.
    pub status: String,
    /// Resolution outcome: "approved" or "rejected".
    pub resolution: String,
    /// Why resolution failed (null when approved).
    pub rejection_reason: Option<String>,
    /// Dependency capability IDs.
    pub dependencies: Vec<String>,
    /// Dependency resolution status.
    pub dependency_resolution: String,
}

/// Receipt emitted for every successful resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveReceipt {
    /// Always "CAPABILITY_RESOLVED".
    pub event: String,
    /// Capability identifier.
    pub capability_id: String,
    /// Resolved version number.
    pub version: i32,
    /// SHA-256 content hash.
    pub content_hash: String,
    /// Resolution outcome.
    pub resolution: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Response for capability_registry_resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveResponse {
    /// Response schema identifier.
    pub response_schema: String,
    /// Resolution result.
    pub resolution: CapabilityResolution,
    /// Resolution receipt.
    pub receipt: ResolveReceipt,
}
