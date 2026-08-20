//! # Capability Registry — Load Contract
//!
//! Request/response types for `capability_registry_load`.
//!
//! **Contract invariants:**
//! - `identity.content_hash` MUST equal SHA-256 of `instructions.body`
//! - `receipt.agent` MUST match the request `agent_identity`
//! - `receipt.task_id` MUST match the request `task_id`

use serde::{Deserialize, Serialize};

/// Request for capability_registry_load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadRequest {
    /// Capability identifier (e.g., "frontend-design").
    pub capability_id: String,
    /// Optional specific version. Defaults to active_version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
    /// Attributable task identifier.
    pub task_id: String,
    /// Requesting agent identity.
    pub agent_identity: String,
    /// Human-readable reason for loading.
    pub reason: String,
}

/// Identity envelope — frozen at load time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityIdentity {
    /// Capability identifier.
    pub capability_id: String,
    /// Version number.
    pub version: i32,
    /// SHA-256 hash of the instruction body.
    pub content_hash: String,
}

/// The instruction body of the capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInstructions {
    /// Full instruction content.
    pub body: String,
}

/// Governance metadata for the loaded capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGovernance {
    /// Current lifecycle status.
    pub status: String,
    /// Security classification.
    pub security_classification: String,
}

/// Receipt emitted for every successful capability load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadReceipt {
    /// Always "CAPABILITY_LOADED".
    pub event: String,
    /// Capability identifier.
    pub capability_id: String,
    /// Version number.
    pub version: i32,
    /// SHA-256 content hash.
    pub content_hash: String,
    /// Agent that loaded the capability.
    pub agent: String,
    /// Task identifier.
    pub task_id: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Full capability context returned by a successful load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityContext {
    /// Identity envelope.
    pub identity: CapabilityIdentity,
    /// Instruction body.
    pub instructions: CapabilityInstructions,
    /// Constraint tags (e.g., ["WCAG AA", "responsive"]).
    pub constraints: Vec<String>,
    /// Resolved dependency capability IDs.
    pub dependencies: Vec<String>,
    /// Governance metadata.
    pub governance: CapabilityGovernance,
    /// Attributable load receipt.
    pub receipt: LoadReceipt,
}

/// Response for capability_registry_load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResponse {
    /// Response schema identifier.
    pub response_schema: String,
    /// Capability context with identity, instructions, and receipt.
    pub context: CapabilityContext,
}
