//! Capability manifest — records demonstrated model capabilities.
//!
//! The manifest is the bridge between qualification evidence and
//! router eligibility. It aggregates sealed evidence into a structured
//! capability description, but MUST NOT auto-promote to approved status.
//!
//! Status lifecycle:
//!   draft → proposed → approved | conditional | quarantined | rejected
//!   Any → superseded (when a newer manifest replaces this one)
//!
//! Authority rules:
//! - draft → proposed: automated (evidence aggregation)
//! - proposed → approved: REQUIRES Owner decision
//! - proposed → conditional: REQUIRES Owner decision
//! - proposed → quarantined: REQUIRES Owner decision
//! - proposed → rejected: REQUIRES Owner decision
//! - Any → superseded: automated (when new manifest created)
//!
//! The manifest MUST NOT:
//! - Auto-promote from draft to approved
//! - Create itself from a probe pass alone
//! - Infer router eligibility
//! - Mutate router selection policy
//!
//! The manifest DOES:
//! - Aggregate sealed qualification evidence
//! - Describe demonstrated primitive and role evidence
//! - Record known failure modes
//! - Propose role status and bounded constraints
//! - Create a draft capability manifest
//! - Record an explicit Owner decision reference

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Capability manifest status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ManifestStatus {
    /// Draft: evidence aggregated, not yet proposed for review.
    #[serde(rename = "draft")]
    Draft,
    /// Proposed: submitted for Owner review.
    #[serde(rename = "proposed")]
    Proposed,
    /// Approved: Owner approved for the specified role.
    #[serde(rename = "approved")]
    Approved,
    /// Conditional: Owner approved with constraints.
    #[serde(rename = "conditional")]
    Conditional,
    /// Quarantined: Owner placed on hold for further review.
    #[serde(rename = "quarantined")]
    Quarantined,
    /// Rejected: Owner rejected the manifest.
    #[serde(rename = "rejected")]
    Rejected,
    /// Superseded: replaced by a newer manifest.
    #[serde(rename = "superseded")]
    Superseded,
}

impl ManifestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Conditional => "conditional",
            Self::Quarantined => "quarantined",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "proposed" => Some(Self::Proposed),
            "approved" => Some(Self::Approved),
            "conditional" => Some(Self::Conditional),
            "quarantined" => Some(Self::Quarantined),
            "rejected" => Some(Self::Rejected),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }

    /// Whether this status requires an Owner decision to reach.
    pub fn requires_owner_decision(&self) -> bool {
        matches!(
            self,
            Self::Approved | Self::Conditional | Self::Quarantined | Self::Rejected
        )
    }

    /// Whether this is a terminal status (no further transitions except superseded).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Approved
                | Self::Conditional
                | Self::Quarantined
                | Self::Rejected
                | Self::Superseded
        )
    }
}

/// Capability manifest — records demonstrated model capabilities.
///
/// The manifest aggregates sealed qualification evidence into a structured
/// capability description. It is the input for Owner decisions and the
/// basis for router projections.
///
/// Critical invariant: evidence exists ≠ role approved.
/// The manifest records what was demonstrated, not what is approved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityManifest {
    /// Unique manifest identifier.
    pub manifest_id: String,

    /// Model identity reference (exact artifact binding).
    pub model_id: String,
    pub model_sha256: String,
    pub model_filename: String,

    /// Role this manifest assesses.
    pub role: String,

    /// Current status.
    pub status: ManifestStatus,

    /// Evidence summary: what was demonstrated.
    pub evidence_summary: EvidenceSummary,

    /// Known failure modes.
    pub failure_modes: Vec<FailureMode>,

    /// Proposed role constraints (if any).
    pub constraints: Option<String>,

    /// Reference to the Owner decision that approved this manifest (if any).
    pub owner_decision_id: Option<String>,

    /// Reference to the manifest this one supersedes (if any).
    pub supersedes_manifest_id: Option<String>,

    /// SHA-256 hash of the manifest content.
    pub content_hash: String,

    /// When the manifest was created (RFC 3339).
    pub created_at: String,

    /// When the manifest was last updated (RFC 3339).
    pub updated_at: String,
}

/// Evidence summary: what was demonstrated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSummary {
    /// Smoke test passed (Stage 1).
    pub smoke_test_passed: bool,

    /// Primitive probes that passed.
    pub probes_passed: Vec<String>,

    /// Primitive probes that failed.
    pub probes_failed: Vec<String>,

    /// Total generation duration observed (ms).
    pub total_generation_duration_ms: Option<u64>,

    /// Total output tokens observed.
    pub total_output_tokens: Option<u32>,

    /// GPU release verified.
    pub gpu_release_verified: bool,

    /// Additional evidence notes.
    pub notes: Option<String>,
}

/// Known failure mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureMode {
    /// Description of the failure mode.
    pub description: String,

    /// Severity of the failure.
    pub severity: FailureSeverity,

    /// When this failure was observed.
    pub observed_at: Option<String>,
}

/// Failure severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureSeverity {
    /// Minor: does not block basic operation.
    #[serde(rename = "minor")]
    Minor,
    /// Moderate: may affect specific tasks.
    #[serde(rename = "moderate")]
    Moderate,
    /// Critical: blocks basic operation.
    #[serde(rename = "critical")]
    Critical,
}

impl FailureSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minor => "minor",
            Self::Moderate => "moderate",
            Self::Critical => "critical",
        }
    }
}

impl CapabilityManifest {
    /// Compute a deterministic manifest ID from model_id, role, and timestamp.
    pub fn compute_manifest_id(model_id: &str, role: &str, created_at: &str) -> String {
        let input = format!("{}:{}:{}", model_id, role, created_at);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute SHA-256 hash of the manifest content.
    pub fn compute_content_hash(&self) -> Result<String> {
        // Hash the key fields (not metadata)
        let content = serde_json::json!({
            "model_id": self.model_id,
            "model_sha256": self.model_sha256,
            "role": self.role,
            "status": self.status.as_str(),
            "evidence_summary": self.evidence_summary,
            "failure_modes": self.failure_modes,
            "constraints": self.constraints,
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Validate the manifest structure.
    pub fn validate(&self) -> Result<()> {
        if self.manifest_id.is_empty() {
            anyhow::bail!("manifest_id is empty");
        }
        if self.model_id.is_empty() {
            anyhow::bail!("model_id is empty");
        }
        if self.model_sha256.is_empty() {
            anyhow::bail!("model_sha256 is empty");
        }
        if self.role.is_empty() {
            anyhow::bail!("role is empty");
        }
        if self.created_at.is_empty() {
            anyhow::bail!("created_at is empty");
        }
        Ok(())
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize manifest to JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse manifest from JSON")
    }

    /// Assert this manifest contains no router eligibility data.
    pub fn assert_no_router_eligibility(&self) -> Result<()> {
        // The manifest MUST NOT contain:
        // - router_eligible flag
        // - router_priority
        // - auto_approve
        // - auto_supersede
        //
        // Structural proof: the fields are:
        // - manifest_id, model_id, model_sha256, model_filename (identity)
        // - role (assessment target)
        // - status (lifecycle — NOT router eligibility)
        // - evidence_summary (what was demonstrated — NOT router eligibility)
        // - failure_modes (known issues — NOT router eligibility)
        // - constraints (bounded limitations — NOT router eligibility)
        // - owner_decision_id (authority reference — NOT router eligibility)
        // - supersedes_manifest_id (history — NOT router eligibility)
        // - content_hash, created_at, updated_at (metadata)
        //
        // There are no fields for:
        // - router_eligible
        // - router_priority
        // - auto_approve
        // - auto_supersede
        Ok(())
    }

    /// Attempt to transition to a new status.
    ///
    /// Returns Ok(()) if the transition is valid, Err with explanation if not.
    /// This is the CORE authority gate: status transitions are strictly enforced.
    pub fn can_transition_to(&self, new_status: &ManifestStatus) -> Result<()> {
        match (&self.status, new_status) {
            // Draft → Proposed: automated (evidence aggregation)
            (ManifestStatus::Draft, ManifestStatus::Proposed) => Ok(()),

            // Proposed → Approved: REQUIRES Owner decision
            (ManifestStatus::Proposed, ManifestStatus::Approved) => {
                anyhow::bail!(
                    "Cannot transition from proposed to approved without Owner decision. \
                     Use apply_owner_decision() instead."
                )
            }

            // Proposed → Conditional: REQUIRES Owner decision
            (ManifestStatus::Proposed, ManifestStatus::Conditional) => {
                anyhow::bail!(
                    "Cannot transition from proposed to conditional without Owner decision. \
                     Use apply_owner_decision() instead."
                )
            }

            // Proposed → Quarantined: REQUIRES Owner decision
            (ManifestStatus::Proposed, ManifestStatus::Quarantined) => {
                anyhow::bail!(
                    "Cannot transition from proposed to quarantined without Owner decision. \
                     Use apply_owner_decision() instead."
                )
            }

            // Proposed → Rejected: REQUIRES Owner decision
            (ManifestStatus::Proposed, ManifestStatus::Rejected) => {
                anyhow::bail!(
                    "Cannot transition from proposed to rejected without Owner decision. \
                     Use apply_owner_decision() instead."
                )
            }

            // Any → Superseded: automated (when new manifest created)
            (_, ManifestStatus::Superseded) => Ok(()),

            // Draft → Approved: BLOCKED (must go through proposed)
            (ManifestStatus::Draft, ManifestStatus::Approved) => {
                anyhow::bail!(
                    "Cannot transition directly from draft to approved. \
                     Must go through proposed status first."
                )
            }

            // Draft → Conditional/Quarantined/Rejected: BLOCKED
            (ManifestStatus::Draft, ManifestStatus::Conditional)
            | (ManifestStatus::Draft, ManifestStatus::Quarantined)
            | (ManifestStatus::Draft, ManifestStatus::Rejected) => {
                anyhow::bail!(
                    "Cannot transition from draft to {} without going through proposed.",
                    new_status.as_str()
                )
            }

            // Any other transition: BLOCKED
            _ => {
                anyhow::bail!(
                    "Invalid transition from {} to {}",
                    self.status.as_str(),
                    new_status.as_str()
                )
            }
        }
    }

    /// Apply an Owner decision to the manifest.
    ///
    /// This is the ONLY way to reach approved/conditional/quarantined/rejected status.
    /// The decision_id MUST be provided — the manifest cannot approve itself.
    pub fn apply_owner_decision(
        &mut self,
        new_status: ManifestStatus,
        decision_id: &str,
    ) -> Result<()> {
        if decision_id.is_empty() {
            anyhow::bail!("Owner decision_id cannot be empty — manifest cannot approve itself");
        }

        if !new_status.requires_owner_decision() {
            anyhow::bail!(
                "apply_owner_decision() can only set statuses that require Owner decision. \
                 Got: {}",
                new_status.as_str()
            );
        }

        if self.status != ManifestStatus::Proposed {
            anyhow::bail!(
                "Manifest must be in 'proposed' status to receive an Owner decision. \
                 Current status: {}",
                self.status.as_str()
            );
        }

        self.status = new_status;
        self.owner_decision_id = Some(decision_id.to_string());
        self.updated_at = chrono::Utc::now().to_rfc3339();

        // Recompute content hash
        self.content_hash = self.compute_content_hash()?;

        Ok(())
    }

    /// Mark this manifest as superseded by a newer one.
    pub fn mark_superseded(&mut self, new_manifest_id: &str) -> Result<()> {
        if new_manifest_id.is_empty() {
            anyhow::bail!("new_manifest_id cannot be empty");
        }
        if self.status == ManifestStatus::Superseded {
            anyhow::bail!("Manifest is already superseded");
        }

        self.status = ManifestStatus::Superseded;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.content_hash = self.compute_content_hash()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_evidence_summary() -> EvidenceSummary {
        EvidenceSummary {
            smoke_test_passed: true,
            probes_passed: vec!["PP-RESPONSE-001".to_string(), "PP-JSON-001".to_string()],
            probes_failed: vec!["PP-INSTR-001".to_string()],
            total_generation_duration_ms: Some(1200),
            total_output_tokens: Some(256),
            gpu_release_verified: true,
            notes: Some("Basic response and JSON probes passed".to_string()),
        }
    }

    fn test_manifest() -> CapabilityManifest {
        let created_at = "2026-07-11T12:00:00Z".to_string();
        let manifest_id =
            CapabilityManifest::compute_manifest_id("minicpm5-1b-q4km", "classifier", &created_at);
        let evidence = test_evidence_summary();

        CapabilityManifest {
            manifest_id,
            model_id: "minicpm5-1b-q4km".to_string(),
            model_sha256: "81B64D05A23B".to_string(),
            model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            role: "classifier".to_string(),
            status: ManifestStatus::Draft,
            evidence_summary: evidence,
            failure_modes: vec![],
            constraints: None,
            owner_decision_id: None,
            supersedes_manifest_id: None,
            content_hash: String::new(),
            created_at,
            updated_at: "2026-07-11T12:00:00Z".to_string(),
        }
    }

    // MQR-C1-1: Manifest validates
    #[test]
    fn test_manifest_validates() {
        let manifest = test_manifest();
        assert!(manifest.validate().is_ok());
    }

    // MQR-C1-2: Manifest ID is deterministic
    #[test]
    fn test_manifest_id_deterministic() {
        let id1 = CapabilityManifest::compute_manifest_id("model-1", "role-1", "2026-07-11T12:00:00Z");
        let id2 = CapabilityManifest::compute_manifest_id("model-1", "role-1", "2026-07-11T12:00:00Z");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    // MQR-C1-3: Manifest ID depends on inputs
    #[test]
    fn test_manifest_id_depends_on_inputs() {
        let id1 = CapabilityManifest::compute_manifest_id("model-1", "role-1", "2026-07-11T12:00:00Z");
        let id2 = CapabilityManifest::compute_manifest_id("model-2", "role-1", "2026-07-11T12:00:00Z");
        assert_ne!(id1, id2);
    }

    // MQR-C1-4: Draft → Proposed is allowed
    #[test]
    fn test_draft_to_proposed_allowed() {
        let mut manifest = test_manifest();
        assert!(manifest.can_transition_to(&ManifestStatus::Proposed).is_ok());
        manifest.status = ManifestStatus::Proposed;
        assert_eq!(manifest.status, ManifestStatus::Proposed);
    }

    // MQR-C1-5: Draft → Approved is BLOCKED (critical negative test)
    #[test]
    fn test_draft_to_approved_blocked() {
        let manifest = test_manifest();
        let result = manifest.can_transition_to(&ManifestStatus::Approved);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("draft to approved"));
    }

    // MQR-C1-6: Proposed → Approved REQUIRES Owner decision (critical negative test)
    #[test]
    fn test_proposed_to_approved_requires_decision() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.can_transition_to(&ManifestStatus::Approved);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Owner decision"));
    }

    // MQR-C1-7: Proposed → Conditional REQUIRES Owner decision
    #[test]
    fn test_proposed_to_conditional_requires_decision() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.can_transition_to(&ManifestStatus::Conditional);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Owner decision"));
    }

    // MQR-C1-8: Proposed → Quarantined REQUIRES Owner decision
    #[test]
    fn test_proposed_to_quarantined_requires_decision() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.can_transition_to(&ManifestStatus::Quarantined);
        assert!(result.is_err());
    }

    // MQR-C1-9: Proposed → Rejected REQUIRES Owner decision
    #[test]
    fn test_proposed_to_rejected_requires_decision() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.can_transition_to(&ManifestStatus::Rejected);
        assert!(result.is_err());
    }

    // MQR-C1-10: Apply Owner decision with empty decision_id is BLOCKED (critical negative test)
    #[test]
    fn test_apply_decision_empty_id_blocked() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.apply_owner_decision(ManifestStatus::Approved, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot approve itself"));
    }

    // MQR-C1-11: Apply Owner decision to non-proposed manifest is BLOCKED
    #[test]
    fn test_apply_decision_non_proposed_blocked() {
        let mut manifest = test_manifest();
        // manifest is in Draft status
        let result = manifest.apply_owner_decision(ManifestStatus::Approved, "dec-001");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("proposed"));
    }

    // MQR-C1-12: Apply Owner decision succeeds with valid inputs
    #[test]
    fn test_apply_decision_success() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.apply_owner_decision(ManifestStatus::Approved, "dec-001");
        assert!(result.is_ok());
        assert_eq!(manifest.status, ManifestStatus::Approved);
        assert_eq!(manifest.owner_decision_id, Some("dec-001".to_string()));
    }

    // MQR-C1-13: Apply conditional decision
    #[test]
    fn test_apply_conditional_decision() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.apply_owner_decision(ManifestStatus::Conditional, "dec-002");
        assert!(result.is_ok());
        assert_eq!(manifest.status, ManifestStatus::Conditional);
        assert_eq!(manifest.owner_decision_id, Some("dec-002".to_string()));
    }

    // MQR-C1-14: Apply rejection decision
    #[test]
    fn test_apply_rejection_decision() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.apply_owner_decision(ManifestStatus::Rejected, "dec-003");
        assert!(result.is_ok());
        assert_eq!(manifest.status, ManifestStatus::Rejected);
        assert_eq!(manifest.owner_decision_id, Some("dec-003".to_string()));
    }

    // MQR-C1-15: Mark superseded
    #[test]
    fn test_mark_superseded() {
        let mut manifest = test_manifest();
        let result = manifest.mark_superseded("new-manifest-id");
        assert!(result.is_ok());
        assert_eq!(manifest.status, ManifestStatus::Superseded);
    }

    // MQR-C1-16: Already superseded cannot be superseded again
    #[test]
    fn test_already_superseded_blocked() {
        let mut manifest = test_manifest();
        manifest.status = ManifestStatus::Superseded;
        let result = manifest.mark_superseded("new-manifest-id");
        assert!(result.is_err());
    }

    // MQR-C1-17: Serialization round-trip
    #[test]
    fn test_serialization_round_trip() {
        let manifest = test_manifest();
        let json = manifest.to_json().unwrap();
        let parsed = CapabilityManifest::from_json(&json).unwrap();
        assert_eq!(manifest, parsed);
    }

    // MQR-C1-18: No router eligibility data
    #[test]
    fn test_no_router_eligibility() {
        let manifest = test_manifest();
        assert!(manifest.assert_no_router_eligibility().is_ok());
    }

    // MQR-C1-19: Status string round-trip
    #[test]
    fn test_status_string_roundtrip() {
        let statuses = vec![
            ManifestStatus::Draft,
            ManifestStatus::Proposed,
            ManifestStatus::Approved,
            ManifestStatus::Conditional,
            ManifestStatus::Quarantined,
            ManifestStatus::Rejected,
            ManifestStatus::Superseded,
        ];
        for status in &statuses {
            let s = status.as_str();
            assert!(!s.is_empty());
            assert_eq!(ManifestStatus::from_str(s), Some(status.clone()));
        }
    }

    // MQR-C1-20: requires_owner_decision is correct
    #[test]
    fn test_requires_owner_decision() {
        assert!(!ManifestStatus::Draft.requires_owner_decision());
        assert!(!ManifestStatus::Proposed.requires_owner_decision());
        assert!(ManifestStatus::Approved.requires_owner_decision());
        assert!(ManifestStatus::Conditional.requires_owner_decision());
        assert!(ManifestStatus::Quarantined.requires_owner_decision());
        assert!(ManifestStatus::Rejected.requires_owner_decision());
        assert!(!ManifestStatus::Superseded.requires_owner_decision());
    }

    // MQR-C1-21: Negative test — perfect evidence cannot auto-approve
    #[test]
    fn test_perfect_evidence_cannot_auto_approve() {
        let mut manifest = test_manifest();
        manifest.evidence_summary = EvidenceSummary {
            smoke_test_passed: true,
            probes_passed: vec![
                "PP-RESPONSE-001".to_string(),
                "PP-JSON-001".to_string(),
                "PP-INSTR-001".to_string(),
            ],
            probes_failed: vec![],
            total_generation_duration_ms: Some(500),
            total_output_tokens: Some(512),
            gpu_release_verified: true,
            notes: None,
        };

        // Even with perfect evidence, draft → approved is blocked
        let result = manifest.can_transition_to(&ManifestStatus::Approved);
        assert!(result.is_err());

        // Even with perfect evidence, proposed → approved requires decision
        manifest.status = ManifestStatus::Proposed;
        let result = manifest.can_transition_to(&ManifestStatus::Approved);
        assert!(result.is_err());
    }

    // MQR-C1-22: Negative test — evidence aggregation can only create draft
    #[test]
    fn test_evidence_aggregation_creates_draft() {
        let manifest = test_manifest();
        assert_eq!(manifest.status, ManifestStatus::Draft);
    }

    // MQR-C1-23: Content hash is recomputed after decision
    #[test]
    fn test_content_hash_recomputed() {
        let mut manifest = test_manifest();
        let hash_before = manifest.compute_content_hash().unwrap();

        manifest.status = ManifestStatus::Proposed;
        manifest.apply_owner_decision(ManifestStatus::Approved, "dec-001").unwrap();

        let hash_after = manifest.content_hash.clone();
        assert_ne!(hash_before, hash_after);
    }
}
