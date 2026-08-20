//! # Capability Registry — Shared Types
//!
//! Shared contract types for Capability Registry MCP tools.
//! Mirrors the canonical contract defined in
//! `docs/contracts/CAPABILITY-REGISTRY-MCP-CONTRACT.md`.
//!
//! These types are the contract — every runtime (Swift, Rust, etc.)
//! must serialize/deserialize to the same JSON representation.

use serde::{Deserialize, Serialize};

// ── Lifecycle Status ───────────────────────────────────────────────

/// Lifecycle status of a capability.
/// See CR-I-004 for transition rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    /// Newly imported, not yet reviewed.
    Unreviewed,
    /// Reviewed but not yet qualified.
    Reviewed,
    /// Verified and ready for use.
    Qualified,
    /// Superseded; still loadable with warning.
    Deprecated,
    /// Security issue; cannot be loaded.
    Revoked,
}

impl std::fmt::Display for CapabilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreviewed => write!(f, "unreviewed"),
            Self::Reviewed => write!(f, "reviewed"),
            Self::Qualified => write!(f, "qualified"),
            Self::Deprecated => write!(f, "deprecated"),
            Self::Revoked => write!(f, "revoked"),
        }
    }
}

// ── Capability Type ────────────────────────────────────────────────

/// Type of capability in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityType {
    /// Agent skill (e.g., frontend-design, security-review).
    Skill,
    /// Workflow (e.g., release-validation, migration-plan).
    Workflow,
    /// Policy (e.g., privacy-boundary, evidence-required).
    Policy,
    /// Validator (e.g., architecture-check, test-quality-check).
    Validator,
    /// Template (e.g., project-scaffold, component-pattern).
    Template,
}

impl std::fmt::Display for CapabilityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skill => write!(f, "skill"),
            Self::Workflow => write!(f, "workflow"),
            Self::Policy => write!(f, "policy"),
            Self::Validator => write!(f, "validator"),
            Self::Template => write!(f, "template"),
        }
    }
}

impl std::str::FromStr for CapabilityType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "skill" => Ok(Self::Skill),
            "workflow" => Ok(Self::Workflow),
            "policy" => Ok(Self::Policy),
            "validator" => Ok(Self::Validator),
            "template" => Ok(Self::Template),
            _ => Err(format!("unknown capability type: {s}")),
        }
    }
}

// ── Security Classification ────────────────────────────────────────

/// Security classification level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityClassification {
    /// Public — no restrictions.
    Green,
    /// Internal — limited distribution.
    Yellow,
    /// Restricted — explicit authorization required.
    Red,
}

impl std::fmt::Display for SecurityClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Green => write!(f, "green"),
            Self::Yellow => write!(f, "yellow"),
            Self::Red => write!(f, "red"),
        }
    }
}

// ── Source Provenance ──────────────────────────────────────────────

/// Source provenance classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Shipped with the Librarian.
    Builtin,
    /// Imported from an external SKILL.md.
    Imported,
    /// From Anthropic's ecosystem.
    Anthropic,
    /// Community-contributed.
    Community,
    /// User-authored.
    User,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin => write!(f, "builtin"),
            Self::Imported => write!(f, "imported"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::Community => write!(f, "community"),
            Self::User => write!(f, "user"),
        }
    }
}

// ── Capability Summary (used in search/list responses) ────────────

/// Metadata summary of a capability — never contains the instruction body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySummary {
    /// Capability identifier (e.g., "frontend-design").
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Capability type.
    #[serde(rename = "type")]
    pub cap_type: String,
    /// Agent-facing description.
    pub description: String,
    /// Short label for search results.
    pub summary: Option<String>,
    /// Lifecycle status.
    pub status: String,
    /// Current active version number.
    pub active_version: Option<i32>,
    /// Provenance source type.
    pub source_type: Option<String>,
    /// Security classification.
    pub security_classification: String,
    /// Tags for discovery.
    pub tags: Option<Vec<String>>,
    /// Optional grouping category.
    pub category: Option<String>,
}

// ── Action Result ──────────────────────────────────────────────────

/// Result of an import or mutation action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportActionResult {
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
