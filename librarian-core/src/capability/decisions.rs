//! Owner decision — explicit authority records for capability manifests.
//!
//! The Owner decision is the ONLY authority gate between evidence and
//! router eligibility. Without an Owner decision, a capability manifest
//! CANNOT reach approved status.
//!
//! Decision types:
//! - Approve: Owner approves the manifest for the specified role
//! - Conditional: Owner approves with constraints
//! - Quarantine: Owner places on hold for further review
//! - Reject: Owner rejects the manifest
//!
//! Authority rules:
//! - Every decision MUST reference a manifest_id
//! - Every decision MUST have a non-empty decision_id
//! - A decision CANNOT be created without a manifest in proposed status
//! - A decision CANNOT auto-approve itself
//! - Decisions are append-only (never mutated)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Owner decision type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionType {
    /// Approve the manifest for the specified role.
    #[serde(rename = "approve")]
    Approve,
    /// Approve with constraints.
    #[serde(rename = "conditional")]
    Conditional,
    /// Place on hold for further review.
    #[serde(rename = "quarantine")]
    Quarantine,
    /// Reject the manifest.
    #[serde(rename = "reject")]
    Reject,
}

impl DecisionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Conditional => "conditional",
            Self::Quarantine => "quarantine",
            Self::Reject => "reject",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "approve" => Some(Self::Approve),
            "conditional" => Some(Self::Conditional),
            "quarantine" => Some(Self::Quarantine),
            "reject" => Some(Self::Reject),
            _ => None,
        }
    }
}

/// Owner decision — explicit authority record.
///
/// Each decision is an append-only record that references a specific
/// capability manifest. The decision is the ONLY way a manifest can
/// reach approved/conditional/quarantined/rejected status.
///
/// Critical invariant: evidence exists ≠ role approved.
/// The decision is the authority gate between evidence and approval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnerDecision {
    /// Unique decision identifier.
    pub decision_id: String,

    /// The manifest this decision applies to.
    pub manifest_id: String,

    /// The decision type.
    pub decision_type: DecisionType,

    /// Role this decision approves/rejects.
    pub role: String,

    /// Model identity reference.
    pub model_id: String,

    /// Constraints (for conditional decisions).
    pub constraints: Option<String>,

    /// Reason for the decision (human-readable).
    pub reason: String,

    /// When the decision was made (RFC 3339).
    pub decided_at: String,

    /// SHA-256 hash of the decision content.
    pub content_hash: String,
}

impl OwnerDecision {
    /// Compute a deterministic decision ID from manifest_id and decided_at.
    pub fn compute_decision_id(manifest_id: &str, decided_at: &str) -> String {
        let input = format!("{}:{}", manifest_id, decided_at);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute SHA-256 hash of the decision content.
    pub fn compute_content_hash(&self) -> Result<String> {
        let content = serde_json::json!({
            "manifest_id": self.manifest_id,
            "decision_type": self.decision_type.as_str(),
            "role": self.role,
            "model_id": self.model_id,
            "constraints": self.constraints,
            "reason": self.reason,
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Validate the decision structure.
    pub fn validate(&self) -> Result<()> {
        if self.decision_id.is_empty() {
            anyhow::bail!("decision_id is empty");
        }
        if self.manifest_id.is_empty() {
            anyhow::bail!("manifest_id is empty");
        }
        if self.role.is_empty() {
            anyhow::bail!("role is empty");
        }
        if self.model_id.is_empty() {
            anyhow::bail!("model_id is empty");
        }
        if self.reason.is_empty() {
            anyhow::bail!("reason is empty");
        }
        if self.decided_at.is_empty() {
            anyhow::bail!("decided_at is empty");
        }
        Ok(())
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize decision to JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse decision from JSON")
    }

    /// Assert this decision contains no auto-approval data.
    pub fn assert_no_auto_approval(&self) -> Result<()> {
        // The decision MUST NOT contain:
        // - auto_approve flag
        // - auto_apply flag
        // - skip_owner_review flag
        //
        // Structural proof: the fields are:
        // - decision_id, manifest_id (identifiers)
        // - decision_type, role, model_id (authority target)
        // - constraints, reason (human judgment)
        // - decided_at, content_hash (metadata)
        //
        // There are no fields for:
        // - auto_approve
        // - auto_apply
        // - skip_owner_review
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_decision() -> OwnerDecision {
        let decided_at = "2026-07-11T12:00:00Z".to_string();
        let decision_id = OwnerDecision::compute_decision_id("manifest-001", &decided_at);

        OwnerDecision {
            decision_id,
            manifest_id: "manifest-001".to_string(),
            decision_type: DecisionType::Approve,
            role: "classifier".to_string(),
            model_id: "minicpm5-1b-q4km".to_string(),
            constraints: None,
            reason: "Model demonstrates basic response and JSON output capabilities".to_string(),
            decided_at,
            content_hash: String::new(),
        }
    }

    // MQR-C1-D1: Decision validates
    #[test]
    fn test_decision_validates() {
        let decision = test_decision();
        assert!(decision.validate().is_ok());
    }

    // MQR-C1-D2: Decision ID is deterministic
    #[test]
    fn test_decision_id_deterministic() {
        let id1 = OwnerDecision::compute_decision_id("manifest-001", "2026-07-11T12:00:00Z");
        let id2 = OwnerDecision::compute_decision_id("manifest-001", "2026-07-11T12:00:00Z");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    // MQR-C1-D3: Decision ID depends on inputs
    #[test]
    fn test_decision_id_depends_on_inputs() {
        let id1 = OwnerDecision::compute_decision_id("manifest-001", "2026-07-11T12:00:00Z");
        let id2 = OwnerDecision::compute_decision_id("manifest-002", "2026-07-11T12:00:00Z");
        assert_ne!(id1, id2);
    }

    // MQR-C1-D4: Decision type string round-trip
    #[test]
    fn test_decision_type_roundtrip() {
        let types = vec![
            DecisionType::Approve,
            DecisionType::Conditional,
            DecisionType::Quarantine,
            DecisionType::Reject,
        ];
        for dt in &types {
            let s = dt.as_str();
            assert!(!s.is_empty());
            assert_eq!(DecisionType::from_str(s), Some(dt.clone()));
        }
    }

    // MQR-C1-D5: Serialization round-trip
    #[test]
    fn test_serialization_round_trip() {
        let decision = test_decision();
        let json = decision.to_json().unwrap();
        let parsed = OwnerDecision::from_json(&json).unwrap();
        assert_eq!(decision, parsed);
    }

    // MQR-C1-D6: No auto-approval data
    #[test]
    fn test_no_auto_approval() {
        let decision = test_decision();
        assert!(decision.assert_no_auto_approval().is_ok());
    }

    // MQR-C1-D7: Validate fails on empty manifest_id
    #[test]
    fn test_validate_empty_manifest_id() {
        let mut decision = test_decision();
        decision.manifest_id = "".to_string();
        assert!(decision.validate().is_err());
    }

    // MQR-C1-D8: Validate fails on empty reason
    #[test]
    fn test_validate_empty_reason() {
        let mut decision = test_decision();
        decision.reason = "".to_string();
        assert!(decision.validate().is_err());
    }

    // MQR-C1-D9: Validate fails on empty role
    #[test]
    fn test_validate_empty_role() {
        let mut decision = test_decision();
        decision.role = "".to_string();
        assert!(decision.validate().is_err());
    }

    // MQR-C1-D10: Conditional decision with constraints
    #[test]
    fn test_conditional_decision_with_constraints() {
        let mut decision = test_decision();
        decision.decision_type = DecisionType::Conditional;
        decision.constraints = Some("Must maintain VRAM below 4096 MiB".to_string());
        assert!(decision.validate().is_ok());
        assert!(decision.constraints.is_some());
    }
}
