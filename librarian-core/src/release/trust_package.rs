//! Release trust package — deterministic owner review package.
//!
//! Summarizes validated components, provenance completeness, validation
//! findings, integrity verification, and included artifacts.
//!
//! Does NOT contain: recommendations, release approval, deployment advice.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::provenance::ReleaseProvenance;
use super::validation::{ValidationResult, ValidationSummary};

/// Trust metadata — factual summary of the release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TrustMetadata {
    pub release_id: String,
    pub version: String,
    pub total_components: usize,
    pub total_sprints: usize,
    pub total_evidence_refs: usize,
    pub generated_at: String,
}

/// Deterministic owner-facing trust package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseTrustPackage {
    pub metadata: TrustMetadata,
    pub validation: ValidationResult,
    pub summary: ValidationSummary,
    pub provenance: ReleaseProvenance,
    pub integrity_hash: String,
}

impl ReleaseTrustPackage {
    pub fn compute_integrity_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.metadata.release_id.as_bytes());
        h.update(self.validation.integrity_hash.as_bytes());
        h.update(self.provenance.integrity_hash.as_bytes());
        format!("{:x}", h.finalize())
    }

    /// Build a complete trust package from validation + provenance.
    pub fn build(
        release_id: &str,
        version: &str,
        validation: ValidationResult,
        summary: ValidationSummary,
        provenance: ReleaseProvenance,
    ) -> Self {
        let metadata = TrustMetadata {
            release_id: release_id.into(),
            version: version.into(),
            total_components: 0,
            total_sprints: provenance.chain.iter().filter(|n| n.node_type == "sprint").count(),
            total_evidence_refs: provenance.evidence_refs.len(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        let mut p = ReleaseTrustPackage { metadata, validation, summary, provenance, integrity_hash: String::new() };
        p.integrity_hash = p.compute_integrity_hash();
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::{ReleaseValidation, ReleaseManifest, ReleaseVersion, ReleaseComponent};

    fn manifest() -> ReleaseManifest {
        ReleaseManifest {
            release_id: "R-001".into(), version: ReleaseVersion { major: 1, minor: 0, patch: 0 },
            components: vec![ReleaseComponent { component_id: "C-001".into(), component_type: "model".into(), version: "1.0".into(), sprint_id: "S-001".into(), content_hash: "abc".into() }],
            governance_receipt_refs: vec!["GR-001".into()], included_sprint_ids: vec!["S-001".into()],
            created_at: "2026-01-01".into(), content_hash: String::new(),
        }
    }

    #[test] fn test_build_package() {
        let vr = ReleaseValidation::validate(&manifest(), &["S-001".into()], &["GR-001".into()]);
        let vs = ReleaseValidation::summary(&vr);
        let prov = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["abc".into()]);
        let tp = ReleaseTrustPackage::build("R-001", "1.0.0", vr, vs, prov);
        assert!(tp.validation.valid);
        assert!(!tp.integrity_hash.is_empty());
    }

    #[test] fn test_hash_deterministic() {
        let vr = ReleaseValidation::validate(&manifest(), &["S-001".into()], &["GR-001".into()]);
        let vs = ReleaseValidation::summary(&vr);
        let prov = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["abc".into()]);
        let tp1 = ReleaseTrustPackage::build("R-001", "1.0.0", vr, vs, prov);
        let vr2 = ReleaseValidation::validate(&manifest(), &["S-001".into()], &["GR-001".into()]);
        let vs2 = ReleaseValidation::summary(&vr2);
        let prov2 = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["abc".into()]);
        let tp2 = ReleaseTrustPackage::build("R-001", "1.0.0", vr2, vs2, prov2);
        // Content hashes should match since content is deterministic
        // (timestamps may differ between calls, but integrity_hash excludes timestamps)
        assert_eq!(tp1.integrity_hash, tp2.integrity_hash);
    }

    #[test] fn test_no_authority() {
        let vr = ReleaseValidation::validate(&manifest(), &["S-001".into()], &["GR-001".into()]);
        let vs = ReleaseValidation::summary(&vr);
        let prov = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["abc".into()]);
        let tp = ReleaseTrustPackage::build("R-001", "1.0.0", vr, vs, prov);
        let j = serde_json::to_value(&tp).unwrap();
        assert!(j.get("approve").is_none()); assert!(j.get("recommend").is_none());
        assert!(j.get("deploy").is_none()); assert!(j.get("decision").is_none());
        assert!(j.get("quality").is_none()); assert!(j.get("risk").is_none());
    }
}
