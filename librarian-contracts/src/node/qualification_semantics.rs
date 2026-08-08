//! M1-D1 — Qualification semantics types
//!
//! Types representing governed qualification state and transition descriptions.
//! These types represent state; they do NOT execute transitions.
//!
//! **First-class invariants (mechanically protected):**
//! - `QUALIFIED ≠ AUTHORIZED` — qualification does not grant permission
//! - `QUALIFIED ≠ AVAILABLE` — qualification does not control availability
//! - `QUALIFIED ≠ EXECUTING` — qualification does not enable execution
//!
//! **Contract lineage:**
//! - `QUALIFICATION-TRANSITION-CONTRACT-001` (transition authority)
//! - `QUALIFICATION-EVIDENCE-CONTRACT-001` (evidence requirements)
//! - `QUALIFICATION-AUTHORITY-CONTRACT-001` (authorization boundaries)
//! - `QUALIFICATION-AVAILABILITY-BOUNDARY-CONTRACT-001` (axis independence)

use serde::{Deserialize, Serialize};

use super::capability_registry::{
    CapabilityId, QualificationAxis, QualificationRecord, QualificationState,
    SecurityClassification, TransitionType, TransitionerRole,
};

// ---------------------------------------------------------------------------
// Transition Result — outcome of a transition attempt
// ---------------------------------------------------------------------------

/// Transition result — the outcome of attempting a qualification transition.
///
/// **Represents the result; it does NOT execute transitions.**
///
/// Transitions are atomic: either succeed completely or fail completely.
/// No partial persistence is permitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionResult {
    /// Whether the transition succeeded.
    pub success: bool,
    /// The qualification state after transition (if successful).
    pub new_state: Option<QualificationState>,
    /// Error code if transition failed.
    pub error_code: Option<String>,
    /// Human-readable error message if transition failed.
    pub error_message: Option<String>,
    /// The lifecycle event produced by this transition (if successful).
    pub lifecycle_event_id: Option<String>,
}

/// Valid transition matrix — which transitions are permitted.
///
/// This is the contract-defined set of valid transitions from
/// `QUALIFICATION-TRANSITION-CONTRACT-001` §2.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidTransition {
    /// Unreviewed → Reviewed (requires owner or delegated evaluator)
    UnreviewedToReviewed,
    /// Reviewed → Qualified (requires owner or delegated approver + evaluator)
    ReviewedToQualified,
    /// Qualified → Deprecated (requires owner or pre-authorized policy)
    QualifiedToDeprecated,
    /// Qualified → Revoked (requires owner or pre-authorized security policy)
    QualifiedToRevoked,
    /// Deprecated → Revoked (requires owner or pre-authorized policy)
    DeprecatedToRevoked,
}

impl ValidTransition {
    /// All valid transitions.
    pub const ALL: [ValidTransition; 5] = [
        ValidTransition::UnreviewedToReviewed,
        ValidTransition::ReviewedToQualified,
        ValidTransition::QualifiedToDeprecated,
        ValidTransition::QualifiedToRevoked,
        ValidTransition::DeprecatedToRevoked,
    ];

    /// The source state for this transition.
    pub fn from_state(self) -> QualificationState {
        match self {
            ValidTransition::UnreviewedToReviewed => QualificationState::Unreviewed,
            ValidTransition::ReviewedToQualified => QualificationState::Reviewed,
            ValidTransition::QualifiedToDeprecated => QualificationState::Qualified,
            ValidTransition::QualifiedToRevoked => QualificationState::Qualified,
            ValidTransition::DeprecatedToRevoked => QualificationState::Deprecated,
        }
    }

    /// The target state for this transition.
    pub fn to_state(self) -> QualificationState {
        match self {
            ValidTransition::UnreviewedToReviewed => QualificationState::Reviewed,
            ValidTransition::ReviewedToQualified => QualificationState::Qualified,
            ValidTransition::QualifiedToDeprecated => QualificationState::Deprecated,
            ValidTransition::QualifiedToRevoked => QualificationState::Revoked,
            ValidTransition::DeprecatedToRevoked => QualificationState::Revoked,
        }
    }
}

// ---------------------------------------------------------------------------
// Transition Authority — who authorized a transition
// ---------------------------------------------------------------------------

/// Transition authority — who is permitted to cause a transition.
///
/// **Describes authority; it does NOT grant authority.**
///
/// Authority is verified at transition time, not assumed. The authority
/// source must not accidentally become the Rust runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionAuthority {
    /// The role performing the transition.
    pub role: TransitionerRole,
    /// Identity of the transitioner (human or system).
    pub identity: String,
    /// Evidence of authority (signature, approval record, policy ID).
    pub authority_evidence_id: Option<String>,
    /// Whether this authority is pre-authorized (for automated transitions).
    pub pre_authorized: bool,
    /// Policy ID if pre-authorized (for automated transitions).
    pub policy_id: Option<String>,
}

/// Authority requirement for a specific transition.
///
/// Defines what authority is required for each transition type.
/// This is contract-defined, not implementation-defined.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionAuthorityRequirement {
    /// The transition this requirement applies to.
    pub transition: ValidTransition,
    /// Required role(s) that can perform this transition.
    pub required_roles: Vec<TransitionerRole>,
    /// Whether evidence of authority is required.
    pub requires_evidence: bool,
    /// Whether this transition can be automated.
    pub can_be_automated: bool,
    /// Whether owner notification is required for automated transitions.
    pub requires_owner_notification: bool,
}

// ---------------------------------------------------------------------------
// Qualification Profile — what evidence is required
// ---------------------------------------------------------------------------

/// Qualification profile — defines what evidence is required for qualification.
///
/// **Profile validity is contract-defined, not implementation-defined.**
///
/// Profiles define:
/// - Which evidence dimensions are required
/// - What confidence thresholds apply
/// - What is the validity period
/// - Can profiles change after qualification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualificationProfile {
    /// Profile identity.
    pub profile_id: String,
    /// Profile name.
    pub name: String,
    /// Profile description.
    pub description: String,
    /// Required evidence dimensions (at minimum: identity, capability, qualification).
    pub required_dimensions: Vec<String>,
    /// Minimum confidence threshold (0.0–1.0).
    pub min_confidence: Option<f32>,
    /// Profile validity period in days (None = does not expire).
    pub validity_days: Option<u32>,
    /// Whether this profile can be used for automated qualification.
    pub allows_automated: bool,
    /// Security classification requirement (None = any classification).
    pub security_classification_requirement: Option<SecurityClassification>,
}

// ---------------------------------------------------------------------------
// Qualification Validation — structural validation result
// ---------------------------------------------------------------------------

/// Qualification validation result — the outcome of validating a qualification.
///
/// **Represents validation; it does NOT execute transitions.**
///
/// Validation checks:
/// - Evidence completeness (all required dimensions have evidence)
/// - Evidence validity (evidence is not expired, not revoked)
/// - Authority validity (authority is verified)
/// - Profile validity (profile exists and is applicable)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualificationValidationResult {
    /// Whether the qualification is valid.
    pub valid: bool,
    /// Validation errors (if invalid).
    pub errors: Vec<QualificationValidationError>,
    /// Validation warnings (valid but with concerns).
    pub warnings: Vec<String>,
}

/// Qualification validation error — specific validation failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualificationValidationError {
    /// Error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// The dimension or field that failed validation (if applicable).
    pub field: Option<String>,
}

// ---------------------------------------------------------------------------
// Axis Independence — types enforcing QUALIFIED ≠ AUTHORIZED ≠ AVAILABLE ≠ EXECUTING
// ---------------------------------------------------------------------------

/// Capability governance state — independent axes that must not be conflated.
///
/// **First-class invariant:**
/// - `QUALIFIED ≠ AUTHORIZED` — qualification does not grant permission
/// - `QUALIFIED ≠ AVAILABLE` — qualification does not control availability
/// - `QUALIFIED ≠ EXECUTING` — qualification does not enable execution
///
/// This struct represents the governance state across all independent axes.
/// It does NOT imply any relationships between axes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityGovernanceState {
    /// The capability this state belongs to.
    pub capability_id: String,
    /// Qualification state (lifecycle).
    pub qualification_state: QualificationState,
    /// Qualification axis (verification status).
    pub qualification_axis: QualificationAxis,
    /// Whether this capability is authorized (separate from qualification).
    pub authorized: bool,
    /// Whether this capability is available (separate from qualification).
    pub available: bool,
    /// Whether this capability is currently executing (separate from qualification).
    pub executing: bool,
}

impl CapabilityGovernanceState {
    /// Check if this capability is qualified.
    pub fn is_qualified(&self) -> bool {
        self.qualification_state == QualificationState::Qualified
    }

    /// Check if this capability is authorized.
    ///
    /// **Note:** Authorization is independent of qualification.
    /// A qualified capability is not automatically authorized.
    pub fn is_authorized(&self) -> bool {
        self.authorized
    }

    /// Check if this capability is available.
    ///
    /// **Note:** Availability is independent of qualification.
    /// A qualified capability is not automatically available.
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Check if this capability is executing.
    ///
    /// **Note:** Execution requires all four conditions:
    /// qualification, availability, permission, and operational mode.
    pub fn is_executing(&self) -> bool {
        self.executing
    }

    /// Check if this capability can be executed.
    ///
    /// **Note:** This is a convenience method, not an authority grant.
    /// Actual execution requires additional authorization checks.
    pub fn can_execute(&self) -> bool {
        self.is_qualified() && self.is_authorized() && self.is_available() && !self.is_executing()
    }
}

// ---------------------------------------------------------------------------
// Evidence Completeness — types for evidence validation
// ---------------------------------------------------------------------------

/// Evidence completeness status — whether all required evidence exists.
///
/// **Evidence is contract-defined, not implementation-defined.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceCompleteness {
    /// All required dimensions have valid evidence.
    Complete,
    /// One or more required dimensions lack evidence.
    Incomplete,
    /// One or more evidence records are expired or invalid.
    Invalid,
    /// Evidence exists but does not meet profile requirements.
    Insufficient,
}

impl EvidenceCompleteness {
    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceCompleteness::Complete => "complete",
            EvidenceCompleteness::Incomplete => "incomplete",
            EvidenceCompleteness::Invalid => "invalid",
            EvidenceCompleteness::Insufficient => "insufficient",
        }
    }
}

/// Evidence freshness status — whether evidence is current.
///
/// **Freshness policy is contract-defined, not implementation-defined.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceFreshnessStatus {
    /// All evidence is fresh (not expired).
    Fresh,
    /// One or more evidence records are stale (expired).
    Stale,
    /// No evidence exists.
    NoEvidence,
}

impl EvidenceFreshnessStatus {
    /// Serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceFreshnessStatus::Fresh => "fresh",
            EvidenceFreshnessStatus::Stale => "stale",
            EvidenceFreshnessStatus::NoEvidence => "no_evidence",
        }
    }
}

// ---------------------------------------------------------------------------
// Forbidden Transitions — types encoding what cannot happen
// ---------------------------------------------------------------------------

/// Forbidden transition — transitions that are contractually prohibited.
///
/// **These transitions are forbidden regardless of authority or evidence.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ForbiddenTransition {
    /// Unreviewed → Qualified (bypasses review step)
    UnreviewedToQualified,
    /// Unreviewed → Deprecated (no qualification to deprecate)
    UnreviewedToDeprecated,
    /// Unreviewed → Revoked (nothing to revoke)
    UnreviewedToRevoked,
    /// Reviewed → Unreviewed (regression not permitted)
    ReviewedToUnreviewed,
    /// Deprecated → Reviewed (deprecation is terminal)
    DeprecatedToReviewed,
    /// Deprecated → Qualified (deprecation is terminal)
    DeprecatedToQualified,
    /// Revoked → anything (revocation is terminal)
    RevokedToAnything,
}

impl ForbiddenTransition {
    /// All forbidden transitions.
    pub const ALL: [ForbiddenTransition; 7] = [
        ForbiddenTransition::UnreviewedToQualified,
        ForbiddenTransition::UnreviewedToDeprecated,
        ForbiddenTransition::UnreviewedToRevoked,
        ForbiddenTransition::ReviewedToUnreviewed,
        ForbiddenTransition::DeprecatedToReviewed,
        ForbiddenTransition::DeprecatedToQualified,
        ForbiddenTransition::RevokedToAnything,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify valid transition count matches contract.
    #[test]
    fn valid_transition_count_matches_contract() {
        assert_eq!(ValidTransition::ALL.len(), 5, "contract defines exactly 5 valid transitions");
    }

    /// Verify forbidden transition count matches contract.
    #[test]
    fn forbidden_transition_count_matches_contract() {
        assert_eq!(ForbiddenTransition::ALL.len(), 7, "contract defines exactly 7 forbidden transitions");
    }

    /// Verify valid transitions produce correct state pairs.
    #[test]
    fn valid_transitions_produce_correct_states() {
        for transition in ValidTransition::ALL {
            let from = transition.from_state();
            let to = transition.to_state();
            assert_ne!(from, to, "transition must change state");
        }
    }

    /// Verify axis independence: qualified does not imply authorized.
    #[test]
    fn qualified_does_not_imply_authorized() {
        let state = CapabilityGovernanceState {
            capability_id: "test".to_string(),
            qualification_state: QualificationState::Qualified,
            qualification_axis: QualificationAxis::Passed,
            authorized: false,
            available: true,
            executing: false,
        };
        assert!(state.is_qualified(), "capability is qualified");
        assert!(!state.is_authorized(), "qualified does NOT imply authorized");
        assert!(!state.can_execute(), "cannot execute without authorization");
    }

    /// Verify axis independence: qualified does not imply available.
    #[test]
    fn qualified_does_not_imply_available() {
        let state = CapabilityGovernanceState {
            capability_id: "test".to_string(),
            qualification_state: QualificationState::Qualified,
            qualification_axis: QualificationAxis::Passed,
            authorized: true,
            available: false,
            executing: false,
        };
        assert!(state.is_qualified(), "capability is qualified");
        assert!(!state.is_available(), "qualified does NOT imply available");
        assert!(!state.can_execute(), "cannot execute without availability");
    }

    /// Verify axis independence: qualified does not imply executing.
    #[test]
    fn qualified_does_not_imply_executing() {
        let state = CapabilityGovernanceState {
            capability_id: "test".to_string(),
            qualification_state: QualificationState::Qualified,
            qualification_axis: QualificationAxis::Passed,
            authorized: true,
            available: true,
            executing: false,
        };
        assert!(state.is_qualified(), "capability is qualified");
        assert!(!state.is_executing(), "qualified does NOT imply executing");
        assert!(state.can_execute(), "can execute when all conditions met");
    }

    /// Verify can_execute requires all four conditions.
    #[test]
    fn can_execute_requires_all_conditions() {
        // Missing qualification
        let state = CapabilityGovernanceState {
            capability_id: "test".to_string(),
            qualification_state: QualificationState::Unreviewed,
            qualification_axis: QualificationAxis::NotTested,
            authorized: true,
            available: true,
            executing: false,
        };
        assert!(!state.can_execute(), "cannot execute without qualification");

        // Missing authorization
        let state = CapabilityGovernanceState {
            capability_id: "test".to_string(),
            qualification_state: QualificationState::Qualified,
            qualification_axis: QualificationAxis::Passed,
            authorized: false,
            available: true,
            executing: false,
        };
        assert!(!state.can_execute(), "cannot execute without authorization");

        // Missing availability
        let state = CapabilityGovernanceState {
            capability_id: "test".to_string(),
            qualification_state: QualificationState::Qualified,
            qualification_axis: QualificationAxis::Passed,
            authorized: true,
            available: false,
            executing: false,
        };
        assert!(!state.can_execute(), "cannot execute without availability");

        // Already executing
        let state = CapabilityGovernanceState {
            capability_id: "test".to_string(),
            qualification_state: QualificationState::Qualified,
            qualification_axis: QualificationAxis::Passed,
            authorized: true,
            available: true,
            executing: true,
        };
        assert!(!state.can_execute(), "cannot execute while already executing");
    }

    /// Evidence completeness is contract-defined.
    #[test]
    fn evidence_completeness_variants_match_contract() {
        assert_eq!(EvidenceCompleteness::Complete.as_str(), "complete");
        assert_eq!(EvidenceCompleteness::Incomplete.as_str(), "incomplete");
        assert_eq!(EvidenceCompleteness::Invalid.as_str(), "invalid");
        assert_eq!(EvidenceCompleteness::Insufficient.as_str(), "insufficient");
    }

    /// Evidence freshness is contract-defined.
    #[test]
    fn evidence_freshness_variants_match_contract() {
        assert_eq!(EvidenceFreshnessStatus::Fresh.as_str(), "fresh");
        assert_eq!(EvidenceFreshnessStatus::Stale.as_str(), "stale");
        assert_eq!(EvidenceFreshnessStatus::NoEvidence.as_str(), "no_evidence");
    }
}
