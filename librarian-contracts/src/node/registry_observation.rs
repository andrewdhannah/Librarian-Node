//! # Registry Observation Projection Types (M1-B)
//!
//! Node-side representations of the locked M1-B read boundary
//! (`contracts/runtime-api/RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md`,
//! Canonical, `232edfb`; parent suite `REGISTRY-OBSERVATION-SCHEMA-001`).
//!
//! These types carry **observation envelopes and projection payloads** over
//! the governed capability-registry database. They are projections of frozen
//! registry semantics — they confer nothing:
//!
//! ```text
//! Registry   ≠ Authority   — projecting state does not create authority
//! Projection ≠ Ownership   — reporting state is not owning state
//! Observation ≠ Mutation   — the payload is read from a read-only snapshot
//! ```
//!
//! Deliberate design rules (enforced by construction; checked by tests):
//!
//! 1. **No authority methods.** No write, transition, approval, or evaluation
//!    method exists on any projection type — the projection module is a read
//!    surface (contract §5.2).
//! 2. **Payloads reuse the locked M1-A types where possible.**
//!    `CapabilityObservation` embeds the five `CapabilityIdentity` fields;
//!    `CapabilityVersionRecord` uses `CapabilityId`/`CapabilityVersion`;
//!    dependencies are the M1-A `CapabilityDependency` verbatim. The nine
//!    M1-B types add only what the observation surface requires.
//! 3. **`registry_identity` is structured provenance** (`value`, `source`,
//!    `derivation`) — never an unbound field, never an invented source
//!    (contract §3.1; fail-closed when identity cannot be established).
//! 4. **`projection_observed_at` is the only variable envelope field.**
//!    The projection payload is deterministic: same registry state → same
//!    bytes across runs and implementations (contract §3.1).
//! 5. **Naming guard:** the `capability_types` TABLE (projected by
//!    `CapabilityTypeDefinition`) is NOT the `CapabilityType` M1-A enum
//!    (`capabilities.type` column value set). Two distinct concepts; the
//!    projection returns taxonomy rows, the enum types a capability's single
//!    type.
//! 6. **Overview groups are fixed and zero-count-present.** Group keys use the
//!    serialized enum names in `ALL` order; a fixed group set with zero counts
//!    is the stronger observation contract (owner-approved).

use serde::{Deserialize, Serialize};

use super::capability_registry::{
    CapabilityId, CapabilityType, CapabilityVersion, QualificationAxis, QualificationState,
};

// ---------------------------------------------------------------------------
// Assurance axis enums (capabilities.availability / capabilities.authority)
// ---------------------------------------------------------------------------

/// Availability axis — `capabilities.availability`
/// (`CAPABILITY-ASSURANCE-CONTRACT-001` §1.3). NOT NULL with default
/// `'registered'` in the governed schema; the axis columns carry no CHECK
/// constraints, so unknown persisted values fail closed at projection time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityAxis {
    /// Discovered but not yet registered.
    Discovered,
    /// Registered (default).
    Registered,
    /// Disabled.
    Disabled,
    /// Removed.
    Removed,
}

impl AvailabilityAxis {
    /// All availability axis states (taxonomy order).
    pub const ALL: [AvailabilityAxis; 4] = [
        AvailabilityAxis::Discovered,
        AvailabilityAxis::Registered,
        AvailabilityAxis::Disabled,
        AvailabilityAxis::Removed,
    ];

    /// Serialized form (matches `serde(rename_all = "snake_case")`).
    pub fn as_str(self) -> &'static str {
        match self {
            AvailabilityAxis::Discovered => "discovered",
            AvailabilityAxis::Registered => "registered",
            AvailabilityAxis::Disabled => "disabled",
            AvailabilityAxis::Removed => "removed",
        }
    }
}

/// Authority axis — `capabilities.authority`
/// (`CAPABILITY-ASSURANCE-CONTRACT-001` §1.3). NOT NULL with default
/// `'not_submitted'`. Observational: reports the axis value; the projection
/// does not grant, review, or revoke authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityAxis {
    /// Not submitted for review (default).
    NotSubmitted,
    /// Pending review.
    PendingReview,
    /// Approved.
    Approved,
    /// Rejected.
    Rejected,
    /// Revoked.
    Revoked,
}

impl AuthorityAxis {
    /// All authority axis states.
    pub const ALL: [AuthorityAxis; 5] = [
        AuthorityAxis::NotSubmitted,
        AuthorityAxis::PendingReview,
        AuthorityAxis::Approved,
        AuthorityAxis::Rejected,
        AuthorityAxis::Revoked,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            AuthorityAxis::NotSubmitted => "not_submitted",
            AuthorityAxis::PendingReview => "pending_review",
            AuthorityAxis::Approved => "approved",
            AuthorityAxis::Rejected => "rejected",
            AuthorityAxis::Revoked => "revoked",
        }
    }
}

// ---------------------------------------------------------------------------
// Taxonomy category (capability_types.category)
// ---------------------------------------------------------------------------

/// Type taxonomy category — `capability_types.category` (default
/// `'standard'`). Distinct from the `CapabilityType` enum: `TypeCategory`
/// classifies taxonomy ROWS; `CapabilityType` types a capability's single
/// type (naming guard, owner-approved).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeCategory {
    /// Standard (default).
    Standard,
    /// System.
    System,
    /// Experimental.
    Experimental,
    /// External.
    External,
}

impl TypeCategory {
    /// All taxonomy categories.
    pub const ALL: [TypeCategory; 4] = [
        TypeCategory::Standard,
        TypeCategory::System,
        TypeCategory::Experimental,
        TypeCategory::External,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            TypeCategory::Standard => "standard",
            TypeCategory::System => "system",
            TypeCategory::Experimental => "experimental",
            TypeCategory::External => "external",
        }
    }
}

// ---------------------------------------------------------------------------
// Projection payloads
// ---------------------------------------------------------------------------

/// Capability projection payload — identity + lifecycle + assurance axes
/// (contract §1.1 / §3, `capabilities` row).
///
/// Embeds the five `CapabilityIdentity` fields (identity + lifecycle state
/// ONLY, `CAPABILITY-IDENTITY-CONTRACT-001` §1.2) and adds the three
/// assurance axes read directly from the `capabilities` columns
/// (availability / qualification / authority). No content, policy, or
/// provenance material is exposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityObservation {
    /// Immutable identity key.
    pub capability_id: CapabilityId,
    /// Human-readable display name.
    pub name: String,
    /// Capability type (exactly one).
    pub capability_type: CapabilityType,
    /// Active version (or none — `NULL` active_version is valid).
    pub version: Option<CapabilityVersion>,
    /// Governance lifecycle state (observational).
    pub lifecycle_state: QualificationState,
    /// Availability axis (column read).
    pub availability: AvailabilityAxis,
    /// Qualification axis (column read).
    pub qualification: QualificationAxis,
    /// Authority axis (column read).
    pub authority: AuthorityAxis,
}

/// Version projection payload — append-only version history
/// (`capability_versions` rows, ascending by integer, CR-I-001).
///
/// `body` is deliberately NOT exposed (owner-approved): the observation
/// exposes existence + the `content_hash` evidence anchor (CR-I-002), not the
/// instruction payload. `qualification_evidence_id` is an evidence REFERENCE
/// only — proof is owned by the Evidence Plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityVersionRecord {
    /// Owning capability.
    pub capability_id: CapabilityId,
    /// Version number (`CHECK (version > 0)`).
    pub version: CapabilityVersion,
    /// SHA-256 hex content hash (evidence anchor, non-empty).
    pub content_hash: String,
    /// Changelog note (optional).
    pub changelog: Option<String>,
    /// Author (optional).
    pub author: Option<String>,
    /// Review notes (optional).
    pub review_notes: Option<String>,
    /// Evidence reference `EV-XXXXX` (optional; Evidence Plane owns proof).
    pub qualification_evidence_id: Option<String>,
    /// Qualification profile reference (optional; no profile evaluation).
    pub profile_id: Option<String>,
    /// Registry provenance timestamp (ISO 8601).
    pub created_at: String,
}

/// Type taxonomy projection payload — `capability_types` rows, ordered by
/// `capability_type_id`. Rows are NOT the `CapabilityType` enum (naming
/// guard, §2.4 of the source map).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityTypeDefinition {
    /// Taxonomy row id (PK).
    pub capability_type_id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Category (row classification).
    pub category: TypeCategory,
    /// Default qualification profile reference (optional).
    pub default_profile_id: Option<String>,
    /// Default policy reference (optional).
    pub default_policy_id: Option<String>,
}

/// Registry overview projection payload — deterministic counts over the
/// same snapshot (contract §3, `registry_overview()`).
///
/// Group fields are `(group_name, count)` pairs in the enum `ALL` order, with
/// zero-count groups PRESENT (owner-approved): the same registry snapshot
/// always yields the same group set. Group names are the serialized enum
/// values. No qualification/policy/evidence counts (M1-C), no operational
/// mode or resolvability facts (M1-D / F-3), no `security_classification`
/// facts (F-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryOverview {
    /// Total capability rows.
    pub capability_count: u64,
    /// Counts by `capabilities.status` (`QualificationState::ALL` order).
    pub by_status: Vec<(String, u64)>,
    /// Counts by `capabilities.availability` (`AvailabilityAxis::ALL` order).
    pub by_availability: Vec<(String, u64)>,
    /// Counts by `capabilities.qualification` (`QualificationAxis::ALL` order).
    pub by_qualification: Vec<(String, u64)>,
    /// Counts by `capabilities.authority` (`AuthorityAxis::ALL` order).
    pub by_authority: Vec<(String, u64)>,
    /// Counts by `capabilities.type` (`CapabilityType::ALL` order).
    pub by_type: Vec<(String, u64)>,
    /// Total version rows.
    pub version_count: u64,
    /// Total dependency rows.
    pub dependency_count: u64,
    /// Counts by `capability_dependencies.relationship_type`
    /// (`CapabilityRelationshipType::ALL` order).
    pub dependency_by_relationship: Vec<(String, u64)>,
    /// Total taxonomy rows.
    pub type_count: u64,
    /// Counts by `capability_types.category` (`TypeCategory::ALL` order).
    pub types_by_category: Vec<(String, u64)>,
}

// ---------------------------------------------------------------------------
// Observation envelope
// ---------------------------------------------------------------------------

/// `registry_identity` — structured provenance for the observed registry
/// state (contract §3.1). `value` is implementation-defined but deterministic
/// and content-derived; `source` names the authoritative origin (e.g.
/// `"capability_registry_meta"`); `derivation` names how the value was
/// obtained. Never a node-generated, process-derived, timestamp-derived, or
/// transport-generated identity; a projection fails closed if identity
/// cannot be established.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegistryIdentity {
    /// The identity value (deterministic, content-derived).
    pub value: String,
    /// Authoritative origin of the identity.
    pub source: String,
    /// How the value was obtained.
    pub derivation: String,
}

/// Observation envelope — carried by every projection response
/// (contract §3.1).
///
/// `projection_observed_at` is the ONLY variable field (observation time; it
/// does not represent registry mutation time and does not imply freshness of
/// registry state). The `projection` payload is fully deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryObservationEnvelope<T> {
    /// Observing node.
    pub node_id: String,
    /// Registry identity provenance (fail-closed, see [`RegistryIdentity`]).
    pub registry_identity: RegistryIdentity,
    /// RFC 3339 UTC observation timestamp (variable field).
    pub projection_observed_at: String,
    /// Deterministic projection payload.
    pub projection: T,
}
