//! # Platform Qualification
//!
//! Qualification certificate and contract identity for the Librarian platform
//! specification. Every runtime implementation must pass qualification before
//! it is considered a valid Librarian runtime.
//!
//! This mirrors the existing qualification patterns:
//! - Models receive qualification.
//! - Capabilities receive qualification.
//! - Agents receive authorization.
//! - Runtimes now receive qualification certificates.
//!
//! ## Architecture
//!
//! ```text
//! Platform Specification (contracts crate)
//!         │
//!         ▼
//! Qualification Harness (tests/qualification)
//!         │
//!         ▼
//! Qualification Certificate (evidence artifact)
//!         │
//!         ▼
//! Runtime eligible for release
//! ```

use serde::{Deserialize, Serialize};

// ── Contract Identity ──────────────────────────────────────────────

/// Unique identifier for a platform contract specification version.
/// Format: "LPC-{nnn}" where {nnn} is a sequential number.
pub type ContractId = String;

/// Version of the platform specification.
/// Follows semver: MAJOR.MINOR.PATCH
pub type ContractVersion = String;

/// The platform contract identity — a unique, immutable reference to a
/// specific version of the platform specification.
///
/// Every qualification certificate, evidence record, and release receipt
/// references this identity for complete traceability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformContractIdentity {
    /// Contract identifier (e.g., "LPC-001").
    pub contract_id: ContractId,
    /// Semantic version (e.g., "1.0.0").
    pub version: ContractVersion,
    /// Human-readable contract name.
    pub name: String,
    /// Specification status.
    pub status: SpecificationStatus,
}

/// Specification lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecificationStatus {
    /// Initial draft — not yet frozen.
    Draft,
    /// Stable — changes require Owner authorization.
    Stable,
    /// Superseded by a newer contract version.
    Superseded,
    /// No longer in use.
    Retired,
}

// ── Qualification Level Results ────────────────────────────────────

/// Result of a single qualification level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualificationLevelResult {
    /// Qualification level identifier (Q-1 through Q-5).
    pub level: String,
    /// Level name.
    pub name: String,
    /// Whether this level passed.
    pub passed: bool,
    /// Optional failure details.
    pub details: Option<String>,
}

// ── Qualification Certificate ──────────────────────────────────────

/// A platform qualification certificate.
///
/// This is a first-class evidence artifact, not merely CI output.
/// A runtime that has not been qualified is not a valid Librarian runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformQualificationCertificate {
    /// Schema identifier.
    pub schema: String,
    /// Contract identity this qualification is against.
    pub contract: PlatformContractIdentity,
    /// Implementation being qualified.
    pub implementation: ImplementationIdentity,
    /// Results for each qualification level.
    pub levels: Vec<QualificationLevelResult>,
    /// Overall qualification result.
    pub qualified: bool,
    /// ISO 8601 timestamp of qualification.
    pub qualified_at: String,
    /// Optional evidence receipt reference.
    pub evidence_receipt: Option<String>,
}

/// Identity of the implementation being qualified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationIdentity {
    /// Runtime name (e.g., "Swift Runtime", "Rust Runtime").
    pub name: String,
    /// Implementation version or build number.
    pub version: String,
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_metadata: Option<String>,
}

// ── Current Contract Identity ──────────────────────────────────────

/// The current platform contract identity.
/// This is the specification that all runtimes must qualify against.
pub const CURRENT_CONTRACT: PlatformContractIdentity = PlatformContractIdentity {
    contract_id: String::new(), // "LPC-001"
    version: String::new(),     // "1.0.0"
    name: String::new(),        // will be set by `current_contract()`
    status: SpecificationStatus::Stable,
};

/// Get the current platform contract identity.
pub fn current_contract() -> PlatformContractIdentity {
    PlatformContractIdentity {
        contract_id: "LPC-001".to_string(),
        version: "1.0.0".to_string(),
        name: "Capability Registry Platform Contract".to_string(),
        status: SpecificationStatus::Stable,
    }
}

/// Create a qualification certificate with the given level results.
pub fn create_certificate(
    implementation: ImplementationIdentity,
    levels: Vec<QualificationLevelResult>,
    evidence_receipt: Option<String>,
    qualified_at: String,
) -> PlatformQualificationCertificate {
    let qualified = levels.iter().all(|l| l.passed);

    PlatformQualificationCertificate {
        schema: "platform-qualification-certificate-v1".to_string(),
        contract: current_contract(),
        implementation,
        levels,
        qualified,
        qualified_at,
        evidence_receipt,
    }
}

/// Create a qualification certificate with all results pre-filled.
/// Each result has `passed = true` by default — the harness sets them.
pub fn default_certificate(implementation: ImplementationIdentity) -> PlatformQualificationCertificate {
    let levels = vec![
        QualificationLevelResult {
            level: "Q-1".to_string(),
            name: "Structural".to_string(),
            passed: false,
            details: None,
        },
        QualificationLevelResult {
            level: "Q-2".to_string(),
            name: "Representational".to_string(),
            passed: false,
            details: None,
        },
        QualificationLevelResult {
            level: "Q-3".to_string(),
            name: "Behavioral".to_string(),
            passed: false,
            details: None,
        },
        QualificationLevelResult {
            level: "Q-4".to_string(),
            name: "Deterministic".to_string(),
            passed: false,
            details: None,
        },
        QualificationLevelResult {
            level: "Q-5".to_string(),
            name: "Evolution".to_string(),
            passed: false,
            details: None,
        },
    ];

    create_certificate(implementation, levels, None, String::new())
}
