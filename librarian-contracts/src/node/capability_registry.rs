//! # Capability Registry Contract Types (M1-A)
//!
//! Representations of the locked M1 capability contract set
//! (`contracts/capability/`, sealed suites `CAPABILITY-*-SCHEMA-001`):
//!
//! - `CAPABILITY-ASSURANCE-SCHEMA-001` — assurance axes semantics
//! - `CAPABILITY-IDENTITY-SCHEMA-001` — identity model
//! - `REGISTRY-OBSERVATION-SCHEMA-001` — read-only registry projection
//! - `QUALIFICATION-STATE-SCHEMA-001` — qualification lifecycle + security context
//! - `OPERATIONAL-MODE-DERIVATION-SCHEMA-001` — operational mode projection output
//!
//! These types are **observational representations of frozen registry
//! semantics**. They do not grant, transition, or authorize anything:
//!
//! ```text
//! Registry        ≠ Authority      — projecting state does not create authority
//! Capability      ≠ Permission     — presence grants nothing
//! Qualification   ≠ Authorization  — a passed qualification is not approval
//! Evidence        ≠ Approval       — evidence is proof, not permission
//! ```
//!
//! Deliberate design rules (enforced by construction; checked by tests):
//!
//! 1. **No authority methods.** No `enable()`, `authorize()`, `activate()`,
//!    `qualify()`, `revoke()` — those would collapse contract layers.
//! 2. **`OperationalMode` is a projection OUTPUT.** The derivation function
//!    lives in the projection layer (M1-D), never on this data type
//!    (`OPERATIONAL-MODE-DERIVATION-CONTRACT-001` §1).
//! 3. **`CapabilitySecurityContext` carries provenance only.** It does not know
//!    whether classification came from a database column, policy evaluation,
//!    an inherited relationship, or a derived calculation — that is provenance
//!    (`QUALIFICATION-STATE-CONTRACT-001` §5).
//! 4. **Identity carries no qualification fields.** `CapabilityIdentity` holds
//!    lifecycle state but no qualification records, axis status, or evidence
//!    (`CAPABILITY-IDENTITY-CONTRACT-001` §1.2 invariant).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Capability identity family (CAPABILITY-IDENTITY-SCHEMA-001)
// ---------------------------------------------------------------------------

/// Immutable capability identity key.
///
/// Schema pattern `^[A-Za-z0-9_-]+$` (examples: `frontend-design`). The
/// newtype keeps identity distinct from every other string field — identity
/// cannot be silently substituted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityId(String);

/// Whether a string is a valid capability identity key
/// (schema pattern `^[A-Za-z0-9_-]+$`).
pub fn is_valid_capability_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl CapabilityId {
    /// Construct a validated capability id. `None` for ids that violate the
    /// schema pattern. Validation, not authority.
    pub fn new(id: String) -> Option<Self> {
        if is_valid_capability_id(&id) {
            Some(Self(id))
        } else {
            None
        }
    }

    /// The id string (read-only).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Capability version — positive integer (`CHECK (version > 0)`,
/// CR-I-001 append-only). The newtype prevents version/pointer confusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityVersion(u32);

impl CapabilityVersion {
    /// Construct a validated version. `None` for `0` (schema `CHECK (version > 0)`).
    pub fn new(version: u32) -> Option<Self> {
        if version > 0 {
            Some(Self(version))
        } else {
            None
        }
    }

    /// The version integer (read-only).
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Capability taxonomy — a capability has exactly one type
/// (`capability_types`; `capabilities.type` default `skill`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityType {
    /// Skill (default).
    Skill,
    /// Workflow.
    Workflow,
    /// Policy.
    Policy,
    /// Validator.
    Validator,
    /// Template.
    Template,
}

impl CapabilityType {
    /// All capability types (taxonomy order).
    pub const ALL: [CapabilityType; 5] = [
        CapabilityType::Skill,
        CapabilityType::Workflow,
        CapabilityType::Policy,
        CapabilityType::Validator,
        CapabilityType::Template,
    ];

    /// Serialized form (matches `serde(rename_all = "snake_case")`).
    pub fn as_str(self) -> &'static str {
        match self {
            CapabilityType::Skill => "skill",
            CapabilityType::Workflow => "workflow",
            CapabilityType::Policy => "policy",
            CapabilityType::Validator => "validator",
            CapabilityType::Template => "template",
        }
    }
}

/// Dependency relationship type (`capability_dependencies.relationship_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRelationshipType {
    /// Requires (default).
    Requires,
    /// Extends.
    Extends,
    /// Refines.
    Refines,
    /// Conflicts.
    Conflicts,
}

impl CapabilityRelationshipType {
    /// All relationship types.
    pub const ALL: [CapabilityRelationshipType; 4] = [
        CapabilityRelationshipType::Requires,
        CapabilityRelationshipType::Extends,
        CapabilityRelationshipType::Refines,
        CapabilityRelationshipType::Conflicts,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            CapabilityRelationshipType::Requires => "requires",
            CapabilityRelationshipType::Extends => "extends",
            CapabilityRelationshipType::Refines => "refines",
            CapabilityRelationshipType::Conflicts => "conflicts",
        }
    }
}

/// Dependency record — dependencies are REFERENCES, not embedded objects
/// (CR-I-005: cycles rejected at resolution; no self-dependency).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDependency {
    /// The capability that has the dependency.
    pub capability_id: CapabilityId,
    /// The capability it depends on.
    pub dependency_id: CapabilityId,
    /// Whether the dependency is required (`1`) or optional (`0`).
    pub required: bool,
    /// Relationship type (`requires` | `extends` | `refines` | `conflicts`).
    pub relationship_type: CapabilityRelationshipType,
}

impl CapabilityDependency {
    /// Construct a dependency record, rejecting self-dependency
    /// (schema `CHECK (capability_id != dependency_id)`). Validation, not authority.
    pub fn new(
        capability_id: CapabilityId,
        dependency_id: CapabilityId,
        required: bool,
        relationship_type: CapabilityRelationshipType,
    ) -> Option<Self> {
        if capability_id == dependency_id {
            return None;
        }
        Some(Self {
            capability_id,
            dependency_id,
            required,
            relationship_type,
        })
    }
}

/// Capability identity — the registry's answer to "what does Librarian know
/// exists" (`CAPABILITY-IDENTITY-CONTRACT-001` §1.1).
///
/// Holds identity + lifecycle state ONLY. **No qualification fields, no
/// permission fields, no evidence** — identity persists independently of
/// qualification state (contract §1.2 explicit invariant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityIdentity {
    /// Immutable identity key.
    pub capability_id: CapabilityId,
    /// Human-readable display name (mutable presentation, not identity).
    pub name: String,
    /// Capability type (exactly one).
    pub capability_type: CapabilityType,
    /// Active version (or none — `NULL` active_version is valid).
    pub version: Option<CapabilityVersion>,
    /// Governance lifecycle state (observational).
    pub lifecycle_state: QualificationState,
}

// ---------------------------------------------------------------------------
// Lifecycle / qualification family (QUALIFICATION-STATE-SCHEMA-001)
// ---------------------------------------------------------------------------

/// Governance lifecycle state — `capabilities.status` axis
/// (`CAPABILITY-ASSURANCE-CONTRACT-001` §1.5; `QUALIFICATION-STATE-CONTRACT-001`
/// §1.1).
///
/// **Observational.** Represents the state a transition record moved between;
/// it does not execute transitions (no `qualify()` / `revoke()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationState {
    /// Registered; not yet reviewed.
    Unreviewed,
    /// Reviewed.
    Reviewed,
    /// Qualified (requires the CR-I-007 assurance chain).
    Qualified,
    /// Deprecated (terminal governance state).
    Deprecated,
    /// Revoked (terminal governance state).
    Revoked,
}

impl QualificationState {
    /// All lifecycle states, in transition order.
    pub const ALL: [QualificationState; 5] = [
        QualificationState::Unreviewed,
        QualificationState::Reviewed,
        QualificationState::Qualified,
        QualificationState::Deprecated,
        QualificationState::Revoked,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            QualificationState::Unreviewed => "unreviewed",
            QualificationState::Reviewed => "reviewed",
            QualificationState::Qualified => "qualified",
            QualificationState::Deprecated => "deprecated",
            QualificationState::Revoked => "revoked",
        }
    }
}

/// Qualification axis — `capabilities.qualification`
/// (`CAPABILITY-ASSURANCE-CONTRACT-001` §1.3). Independent of the governance
/// status lifecycle and of the authority axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationAxis {
    /// No qualification evaluation has occurred.
    NotTested,
    /// Qualification evaluation in progress.
    Qualifying,
    /// Qualification passed against a profile (requires evidence chain).
    Passed,
    /// Qualification failed.
    Failed,
    /// Qualification out of date (e.g., profile version increment, CR-I-008).
    Stale,
    /// Qualification suspended pending re-evaluation.
    Suspended,
}

impl QualificationAxis {
    /// All qualification axis states.
    pub const ALL: [QualificationAxis; 6] = [
        QualificationAxis::NotTested,
        QualificationAxis::Qualifying,
        QualificationAxis::Passed,
        QualificationAxis::Failed,
        QualificationAxis::Stale,
        QualificationAxis::Suspended,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            QualificationAxis::NotTested => "not_tested",
            QualificationAxis::Qualifying => "qualifying",
            QualificationAxis::Passed => "passed",
            QualificationAxis::Failed => "failed",
            QualificationAxis::Stale => "stale",
            QualificationAxis::Suspended => "suspended",
        }
    }
}

/// Qualification record status — `capability_qualifications.qualification_status`
/// (schema value set; distinct from the axis enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationRecordStatus {
    /// Evaluation in progress.
    Qualifying,
    /// Passed.
    Passed,
    /// Failed.
    Failed,
    /// Out of date.
    Stale,
    /// Superseded by a newer evaluation.
    Superseded,
}

impl QualificationRecordStatus {
    /// All record statuses.
    pub const ALL: [QualificationRecordStatus; 5] = [
        QualificationRecordStatus::Qualifying,
        QualificationRecordStatus::Passed,
        QualificationRecordStatus::Failed,
        QualificationRecordStatus::Stale,
        QualificationRecordStatus::Superseded,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            QualificationRecordStatus::Qualifying => "qualifying",
            QualificationRecordStatus::Passed => "passed",
            QualificationRecordStatus::Failed => "failed",
            QualificationRecordStatus::Stale => "stale",
            QualificationRecordStatus::Superseded => "superseded",
        }
    }
}

/// Assessor type — who performed the qualification
/// (`capability_qualifications.assessor_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessorType {
    /// Manual assessment.
    Manual,
    /// Automated assessment.
    Automated,
    /// External assessment.
    External,
}

impl AssessorType {
    /// All assessor types.
    pub const ALL: [AssessorType; 3] = [
        AssessorType::Manual,
        AssessorType::Automated,
        AssessorType::External,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            AssessorType::Manual => "manual",
            AssessorType::Automated => "automated",
            AssessorType::External => "external",
        }
    }
}

/// Qualification record — the result of a qualification evaluation against a
/// profile (`QUALIFICATION-STATE-CONTRACT-001` §2.2). Evidence is a REFERENCE,
/// never inline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualificationRecord {
    /// Record identity (`Q-YYYYMMDD-001`).
    pub qualification_id: String,
    /// Qualified capability.
    pub capability_id: CapabilityId,
    /// Qualification profile applied.
    pub profile_id: String,
    /// Capability version evaluated (if version-specific).
    pub version_id: Option<u32>,
    /// Record status.
    pub status: QualificationRecordStatus,
    /// Confidence score 0.0–1.0.
    pub confidence: Option<f32>,
    /// Evidence reference (`EV-XXXXX`) — the Evidence Plane owns proof.
    pub evidence_reference: Option<String>,
    /// When qualification passed (ISO 8601).
    pub qualified_at: Option<String>,
    /// When qualification expires (ISO 8601; NULL = does not expire).
    pub expires_at: Option<String>,
    /// When the assessment occurred (ISO 8601).
    pub assessed_at: String,
    /// Who/what performed the qualification.
    pub assessor_identity: Option<String>,
    /// Assessor type.
    pub assessor_type: AssessorType,
}

/// Transition type — `qualification_lifecycle_events.transition_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    /// Automatic transition.
    Automatic,
    /// Manual transition.
    Manual,
}

impl TransitionType {
    /// All transition types.
    pub const ALL: [TransitionType; 2] = [TransitionType::Automatic, TransitionType::Manual];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            TransitionType::Automatic => "automatic",
            TransitionType::Manual => "manual",
        }
    }
}

/// Transitioner role — `qualification_lifecycle_events.transitioner_role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionerRole {
    /// System.
    System,
    /// Evaluator.
    Evaluator,
    /// Approver.
    Approver,
    /// Owner.
    Owner,
}

impl TransitionerRole {
    /// All transitioner roles.
    pub const ALL: [TransitionerRole; 4] = [
        TransitionerRole::System,
        TransitionerRole::Evaluator,
        TransitionerRole::Approver,
        TransitionerRole::Owner,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            TransitionerRole::System => "system",
            TransitionerRole::Evaluator => "evaluator",
            TransitionerRole::Approver => "approver",
            TransitionerRole::Owner => "owner",
        }
    }
}

/// Qualification lifecycle event — append-only audit trail for every
/// qualification state transition (`QUALIFICATION-STATE-CONTRACT-001` §2.3).
///
/// **Represents the transition record; it does not execute transitions.**
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualificationLifecycleEvent {
    /// Event identity (`QLE-YYYYMMDD-001`).
    pub event_id: String,
    /// Qualification record this event belongs to.
    pub qualification_id: String,
    /// Capability affected.
    pub capability_id: CapabilityId,
    /// Previous qualification state.
    pub from_state: QualificationState,
    /// New qualification state.
    pub to_state: QualificationState,
    /// Automatic or manual.
    pub transition_type: TransitionType,
    /// FROZEN security classification at transition time (S0–S5 or
    /// unclassified; audit replay reads this, not current registry state).
    pub security_classification: Option<SecurityClassification>,
    /// Identity who performed the transition.
    pub transitioned_by: String,
    /// Role of the transitioner.
    pub transitioner_role: TransitionerRole,
    /// Evidence reference for manual transitions (`EV-XXXXX`).
    pub authority_evidence_id: Option<String>,
    /// Evidence state at transition time (JSON).
    pub evidence_snapshot: serde_json::Value,
    /// Event timestamp (ISO 8601).
    pub created_at: String,
}

/// Evidence dimension — one of the five qualification evidence dimensions
/// (`qualification_evidence_records.dimension`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDimension {
    /// Identity.
    Identity,
    /// Capability.
    Capability,
    /// Security level.
    SecurityLevel,
    /// Qualification.
    Qualification,
    /// Constraints.
    Constraints,
}

impl EvidenceDimension {
    /// All evidence dimensions.
    pub const ALL: [EvidenceDimension; 5] = [
        EvidenceDimension::Identity,
        EvidenceDimension::Capability,
        EvidenceDimension::SecurityLevel,
        EvidenceDimension::Qualification,
        EvidenceDimension::Constraints,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceDimension::Identity => "identity",
            EvidenceDimension::Capability => "capability",
            EvidenceDimension::SecurityLevel => "security_level",
            EvidenceDimension::Qualification => "qualification",
            EvidenceDimension::Constraints => "constraints",
        }
    }
}

/// Evidence type — `qualification_evidence_records.evidence_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    /// Test result.
    TestResult,
    /// Review approval.
    ReviewApproval,
    /// Benchmark.
    Benchmark,
    /// Audit log.
    AuditLog,
    /// Receipt.
    Receipt,
}

impl EvidenceType {
    /// All evidence types.
    pub const ALL: [EvidenceType; 5] = [
        EvidenceType::TestResult,
        EvidenceType::ReviewApproval,
        EvidenceType::Benchmark,
        EvidenceType::AuditLog,
        EvidenceType::Receipt,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceType::TestResult => "test_result",
            EvidenceType::ReviewApproval => "review_approval",
            EvidenceType::Benchmark => "benchmark",
            EvidenceType::AuditLog => "audit_log",
            EvidenceType::Receipt => "receipt",
        }
    }
}

/// Evidence producer role — `qualification_evidence_records.producer_role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceProducerRole {
    /// Evaluator (default).
    Evaluator,
    /// System.
    System,
    /// Automated harness.
    AutomatedHarness,
    /// External.
    External,
}

impl EvidenceProducerRole {
    /// All producer roles.
    pub const ALL: [EvidenceProducerRole; 4] = [
        EvidenceProducerRole::Evaluator,
        EvidenceProducerRole::System,
        EvidenceProducerRole::AutomatedHarness,
        EvidenceProducerRole::External,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceProducerRole::Evaluator => "evaluator",
            EvidenceProducerRole::System => "system",
            EvidenceProducerRole::AutomatedHarness => "automated_harness",
            EvidenceProducerRole::External => "external",
        }
    }
}

/// Qualification evidence reference — per-dimension evidence record
/// (`QUALIFICATION-STATE-CONTRACT-001` §2.4). Proof is owned by the Evidence
/// Plane; this type carries the reference and its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualificationEvidenceReference {
    /// Evidence record identity (`QER-YYYYMMDD-001`).
    pub evidence_id: String,
    /// Qualification record this evidence belongs to.
    pub qualification_id: String,
    /// Evidence dimension (one of five).
    pub dimension: EvidenceDimension,
    /// Evidence type.
    pub evidence_type: EvidenceType,
    /// Evidence reference (`EV-XXXXX` or external).
    pub evidence_reference: Option<String>,
    /// Evidence payload (JSON).
    pub evidence_body: Option<serde_json::Value>,
    /// SHA-256 of `evidence_body` (non-empty).
    pub evidence_hash: String,
    /// When captured (ISO 8601).
    pub captured_at: String,
    /// When evidence expires (ISO 8601; NULL = does not expire).
    pub expires_at: Option<String>,
    /// Who produced this evidence.
    pub producer_identity: String,
    /// Producer role.
    pub producer_role: EvidenceProducerRole,
}

// ---------------------------------------------------------------------------
// Security context family (QUALIFICATION-STATE-SCHEMA-001 §5)
// ---------------------------------------------------------------------------

/// Security classification — S0–S5 (frozen meanings, PI-001) or unclassified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityClassification {
    /// S0 (default when no classification).
    #[serde(rename = "S0")]
    S0,
    /// S1.
    #[serde(rename = "S1")]
    S1,
    /// S2.
    #[serde(rename = "S2")]
    S2,
    /// S3.
    #[serde(rename = "S3")]
    S3,
    /// S4.
    #[serde(rename = "S4")]
    S4,
    /// S5.
    #[serde(rename = "S5")]
    S5,
    /// No classification.
    #[serde(rename = "unclassified")]
    Unclassified,
}

impl SecurityClassification {
    /// All classifications.
    pub const ALL: [SecurityClassification; 7] = [
        SecurityClassification::S0,
        SecurityClassification::S1,
        SecurityClassification::S2,
        SecurityClassification::S3,
        SecurityClassification::S4,
        SecurityClassification::S5,
        SecurityClassification::Unclassified,
    ];

    /// Serialized form (`S0`–`S5`, `unclassified`).
    pub fn as_str(self) -> &'static str {
        match self {
            SecurityClassification::S0 => "S0",
            SecurityClassification::S1 => "S1",
            SecurityClassification::S2 => "S2",
            SecurityClassification::S3 => "S3",
            SecurityClassification::S4 => "S4",
            SecurityClassification::S5 => "S5",
            SecurityClassification::Unclassified => "unclassified",
        }
    }
}

/// Classification derivation — how the classification fact was obtained
/// (`QUALIFICATION-STATE-CONTRACT-001` §5 provenance table).
///
/// The projection MUST preserve the distinction between these kinds whatever
/// layout stores them (storage independence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationDerivation {
    /// Classification asserted at registration.
    Declared,
    /// Classification computed from registry facts.
    Derived,
    /// Classification inherited from version/type context.
    Inherited,
    /// Classification derived from policy constraints.
    PolicyConstraint,
}

impl ClassificationDerivation {
    /// All derivation kinds.
    pub const ALL: [ClassificationDerivation; 4] = [
        ClassificationDerivation::Declared,
        ClassificationDerivation::Derived,
        ClassificationDerivation::Inherited,
        ClassificationDerivation::PolicyConstraint,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            ClassificationDerivation::Declared => "declared",
            ClassificationDerivation::Derived => "derived",
            ClassificationDerivation::Inherited => "inherited",
            ClassificationDerivation::PolicyConstraint => "policy_constraint",
        }
    }
}

/// Capability security context — observational boundary
/// (`QUALIFICATION-STATE-CONTRACT-001` §5).
///
/// **Storage-independent provenance.** This type does not know whether
/// classification came from a database column, policy evaluation, an inherited
/// relationship, or a derived calculation — that is `derivation`. Classification
/// provenance must remain observable even if storage representation changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySecurityContext {
    /// Security classification (S0–S5 or unclassified).
    pub classification: SecurityClassification,
    /// Where the classification fact originates.
    pub source: String,
    /// How it was obtained (declared / derived / inherited / policy_constraint).
    pub derivation: ClassificationDerivation,
    /// Evidence reference (`EV-XXXXX`) — PI-004: no self-attestation.
    pub evidence_reference: Option<String>,
}

// ---------------------------------------------------------------------------
// Operational mode family (OPERATIONAL-MODE-DERIVATION-SCHEMA-001)
// ---------------------------------------------------------------------------

/// Operational mode — projection OUTPUT only.
///
/// **The derivation function lives in the projection layer (M1-D), never on
/// this data type** (`OPERATIONAL-MODE-DERIVATION-CONTRACT-001` §1). This type
/// carries the derived result and its derivation inputs for reproducibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationalMode {
    /// Derived mode.
    pub mode: OperationalModeValue,
    /// Why this mode (deterministic, from the derivation).
    pub explanation: String,
    /// The facts consumed by the derivation (reproducibility).
    pub derivation_inputs: OperationalModeInputs,
    /// Evidence supporting the derived mode.
    pub evidence_references: Vec<String>,
}

/// Derived operational mode value
/// (`view_operational_mode_derivation`; contract §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalModeValue {
    /// Revoked/stale evidence; explain only.
    ExplainOnly,
    /// Requires human review.
    ReviewAssist,
    /// Recommendations only.
    RecommendOnly,
    /// Autonomous operation within security boundary.
    AutonomousAssist,
}

impl OperationalModeValue {
    /// All mode values (degraded → autonomous).
    pub const ALL: [OperationalModeValue; 4] = [
        OperationalModeValue::ExplainOnly,
        OperationalModeValue::ReviewAssist,
        OperationalModeValue::RecommendOnly,
        OperationalModeValue::AutonomousAssist,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            OperationalModeValue::ExplainOnly => "explain_only",
            OperationalModeValue::ReviewAssist => "review_assist",
            OperationalModeValue::RecommendOnly => "recommend_only",
            OperationalModeValue::AutonomousAssist => "autonomous_assist",
        }
    }
}

/// Evidence freshness — input to the derivation
/// (`view_qualification_resolvability`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFreshness {
    /// All present evidence dimensions non-expired.
    Fresh,
    /// At least one evidence dimension expired.
    Stale,
    /// No evidence recorded.
    NoEvidence,
}

impl EvidenceFreshness {
    /// All freshness states.
    pub const ALL: [EvidenceFreshness; 3] = [
        EvidenceFreshness::Fresh,
        EvidenceFreshness::Stale,
        EvidenceFreshness::NoEvidence,
    ];

    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceFreshness::Fresh => "fresh",
            EvidenceFreshness::Stale => "stale",
            EvidenceFreshness::NoEvidence => "no_evidence",
        }
    }
}

/// Derivation inputs — the facts consumed to derive an operational mode
/// (`OPERATIONAL-MODE-DERIVATION-CONTRACT-001` §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalModeInputs {
    /// Security classification (S0–S5; unclassified → S0 default).
    pub security_classification: SecurityClassification,
    /// Qualification axis state.
    pub qualification_axis: QualificationAxis,
    /// Governance lifecycle state.
    pub lifecycle_state: QualificationState,
    /// Evidence freshness.
    pub evidence_freshness: EvidenceFreshness,
    /// Policy constraints (JSON; passed through as `constraints`).
    pub policy_constraints: Option<serde_json::Value>,
}
