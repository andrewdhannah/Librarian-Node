//! # Capability Registry — Import Contract
//!
//! Request/response types for `capability_registry_import`.
//!
//! **Contract invariants (CR-I-007, CR-I-010):**
//! - Status is always "unreviewed" after import
//! - Same content hash produces action "unchanged" (idempotent)
//! - Changed content produces action "new_version" with incremented version

use serde::{Deserialize, Serialize};

/// Request for capability_registry_import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    /// Absolute path to the SKILL.md file to import.
    pub path: String,
    /// Provenance classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    /// URL or origin reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<String>,
}

/// Response for capability_registry_import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResponse {
    /// Response schema identifier.
    pub response_schema: String,
    /// Action taken: "created", "new_version", "unchanged".
    pub action: String,
    /// Capability identifier.
    pub capability_id: String,
    /// New or existing version number.
    pub version: i32,
    /// SHA-256 content hash.
    pub content_hash: String,
    /// Current status (always "unreviewed" after import).
    pub status: String,
    /// Source type.
    pub source_type: String,
}
