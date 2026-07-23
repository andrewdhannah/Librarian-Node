//! Comparative finding — role-specific comparison result between two models.
//!
//! Each finding identifies a specific advantage, disadvantage, or equivalence
//! between a candidate model and a baseline model for a given role.
//!
//! Critical invariant: findings are always role-specific. A model must not
//! be globally rejected merely because another model is stronger overall.

use serde::{Deserialize, Serialize};

/// Type of comparative finding.
///
/// Each variant represents a specific comparison outcome between
/// a candidate and baseline model for a given role.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingType {
    /// Candidate fills a role no other model fills.
    UniqueRoleAdvantage,
    /// Candidate has better quality metrics (tokens/sec) for this role.
    QualityAdvantage,
    /// Candidate has lower latency (generation duration) for this role.
    LatencyAdvantage,
    /// Candidate uses less memory (VRAM or file size) for this role.
    MemoryAdvantage,
    /// Candidate can serve as fallback when the primary model is unavailable.
    FallbackAdvantage,
    /// Baseline is better on all comparable metrics for this role.
    DominatedByExistingModel,
    /// No meaningful difference between candidate and baseline for this role.
    EquivalentNoMaterialAdvantage,
    /// Candidate should replace baseline for this role (recommendation only).
    SupersedesExistingModel,
    /// Insufficient evidence to make a comparison for this role.
    InsufficientComparableEvidence,
}

impl FindingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UniqueRoleAdvantage => "unique_role_advantage",
            Self::QualityAdvantage => "quality_advantage",
            Self::LatencyAdvantage => "latency_advantage",
            Self::MemoryAdvantage => "memory_advantage",
            Self::FallbackAdvantage => "fallback_advantage",
            Self::DominatedByExistingModel => "dominated_by_existing_model",
            Self::EquivalentNoMaterialAdvantage => "equivalent_no_material_advantage",
            Self::SupersedesExistingModel => "supersedes_existing_model",
            Self::InsufficientComparableEvidence => "insufficient_comparable_evidence",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "unique_role_advantage" => Some(Self::UniqueRoleAdvantage),
            "quality_advantage" => Some(Self::QualityAdvantage),
            "latency_advantage" => Some(Self::LatencyAdvantage),
            "memory_advantage" => Some(Self::MemoryAdvantage),
            "fallback_advantage" => Some(Self::FallbackAdvantage),
            "dominated_by_existing_model" => Some(Self::DominatedByExistingModel),
            "equivalent_no_material_advantage" => Some(Self::EquivalentNoMaterialAdvantage),
            "supersedes_existing_model" => Some(Self::SupersedesExistingModel),
            "insufficient_comparable_evidence" => Some(Self::InsufficientComparableEvidence),
            _ => None,
        }
    }
}

/// Severity of a finding — how significant is the comparison result.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingSeverity {
    /// Informational — no action required.
    Info,
    /// Advantage — candidate is better in some dimension.
    Advantage,
    /// Disadvantage — candidate is worse in some dimension.
    Disadvantage,
    /// Blocker — candidate should not be added for this role.
    Blocker,
}

/// A single comparative finding between candidate and baseline for a role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparativeFinding {
    /// Type of finding.
    pub finding_type: FindingType,

    /// Role this finding applies to.
    pub role: String,

    /// Human-readable explanation of the finding.
    pub basis: String,

    /// Severity of the finding.
    pub severity: FindingSeverity,

    /// References to evidence (manifest IDs, profile IDs).
    pub evidence_refs: Vec<String>,
}

impl ComparativeFinding {
    /// Compute a deterministic finding ID from role + finding_type + basis.
    pub fn compute_finding_id(&self) -> String {
        use sha2::{Digest, Sha256};
        let input = format!("{}:{}:{}", self.role, self.finding_type.as_str(), self.basis);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // C3-FT1: FindingType string round-trip
    #[test]
    fn test_finding_type_string_roundtrip() {
        let types = vec![
            FindingType::UniqueRoleAdvantage,
            FindingType::QualityAdvantage,
            FindingType::LatencyAdvantage,
            FindingType::MemoryAdvantage,
            FindingType::FallbackAdvantage,
            FindingType::DominatedByExistingModel,
            FindingType::EquivalentNoMaterialAdvantage,
            FindingType::SupersedesExistingModel,
            FindingType::InsufficientComparableEvidence,
        ];
        for ft in &types {
            let s = ft.as_str();
            assert!(!s.is_empty());
            assert_eq!(FindingType::from_str(s), Some(ft.clone()));
        }
    }

    // C3-FT2: Unknown finding type returns None
    #[test]
    fn test_finding_type_unknown() {
        assert_eq!(FindingType::from_str("unknown"), None);
    }

    // C3-FT3: Finding severity variants exist
    #[test]
    fn test_finding_severity_variants() {
        let severities = vec![
            FindingSeverity::Info,
            FindingSeverity::Advantage,
            FindingSeverity::Disadvantage,
            FindingSeverity::Blocker,
        ];
        assert_eq!(severities.len(), 4);
    }

    // C3-FT4: Finding ID is deterministic
    #[test]
    fn test_finding_id_deterministic() {
        let finding = ComparativeFinding {
            finding_type: FindingType::QualityAdvantage,
            role: "classifier".to_string(),
            basis: "Candidate avg 15.2 tok/s vs baseline 12.5 tok/s".to_string(),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec!["manifest-001".to_string()],
        };
        let id1 = finding.compute_finding_id();
        let id2 = finding.compute_finding_id();
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    // C3-FT5: Finding ID changes with different inputs
    #[test]
    fn test_finding_id_changes_with_role() {
        let f1 = ComparativeFinding {
            finding_type: FindingType::QualityAdvantage,
            role: "classifier".to_string(),
            basis: "same basis".to_string(),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec![],
        };
        let f2 = ComparativeFinding {
            finding_type: FindingType::QualityAdvantage,
            role: "summarizer".to_string(),
            basis: "same basis".to_string(),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec![],
        };
        assert_ne!(f1.compute_finding_id(), f2.compute_finding_id());
    }

    // C3-FT6: Finding ID changes with different finding type
    #[test]
    fn test_finding_id_changes_with_type() {
        let f1 = ComparativeFinding {
            finding_type: FindingType::QualityAdvantage,
            role: "classifier".to_string(),
            basis: "same basis".to_string(),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec![],
        };
        let f2 = ComparativeFinding {
            finding_type: FindingType::LatencyAdvantage,
            role: "classifier".to_string(),
            basis: "same basis".to_string(),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec![],
        };
        assert_ne!(f1.compute_finding_id(), f2.compute_finding_id());
    }

    // C3-FT7: Finding with all 9 types can be constructed
    #[test]
    fn test_all_nine_finding_types_constructible() {
        let types = [
            FindingType::UniqueRoleAdvantage,
            FindingType::QualityAdvantage,
            FindingType::LatencyAdvantage,
            FindingType::MemoryAdvantage,
            FindingType::FallbackAdvantage,
            FindingType::DominatedByExistingModel,
            FindingType::EquivalentNoMaterialAdvantage,
            FindingType::SupersedesExistingModel,
            FindingType::InsufficientComparableEvidence,
        ];
        for ft in &types {
            let finding = ComparativeFinding {
                finding_type: ft.clone(),
                role: "test-role".to_string(),
                basis: "test basis".to_string(),
                severity: FindingSeverity::Info,
                evidence_refs: vec![],
            };
            assert_eq!(finding.finding_type, *ft);
        }
        assert_eq!(types.len(), 9);
    }

    // C3-FT8: Serialization round-trip
    #[test]
    fn test_finding_serialization_roundtrip() {
        let finding = ComparativeFinding {
            finding_type: FindingType::SupersedesExistingModel,
            role: "classifier".to_string(),
            basis: "Candidate: 15.2 tok/s, 3.2s load. Baseline: 12.5 tok/s, 4.1s load.".to_string(),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec!["manifest-001".to_string(), "profile-001".to_string()],
        };
        let json = serde_json::to_string(&finding).unwrap();
        let parsed: ComparativeFinding = serde_json::from_str(&json).unwrap();
        assert_eq!(finding, parsed);
    }
}
