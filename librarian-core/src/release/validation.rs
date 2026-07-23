//! Release validation — verifies releases are composed of valid, sealed work.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::manifest::ReleaseManifest;

/// Severity of a validation issue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// A single validation finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
}

/// Complete validation result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationResult {
    pub manifest_id: String,
    pub issues: Vec<ValidationIssue>,
    pub valid: bool,
    pub integrity_hash: String,
}

impl ValidationResult {
    pub fn compute_integrity_hash(&self) -> String {
        let mut h = Sha256::new();
        for issue in &self.issues {
            h.update(issue.code.as_bytes());
            h.update(issue.message.as_bytes());
        }
        format!("{:x}", h.finalize())
    }
}

/// Summary of validation findings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationSummary {
    pub total_issues: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

/// Release validation engine — reports findings only, no recommendations.
pub struct ReleaseValidation;

impl ReleaseValidation {
    /// Validate a release manifest against a list of sealed sprint IDs.
    pub fn validate(
        manifest: &ReleaseManifest,
        sealed_sprints: &[String],
        governance_refs: &[String],
    ) -> ValidationResult {
        let mut issues = Vec::new();

        // Check each sprint is sealed
        for sprint_id in &manifest.included_sprint_ids {
            if !sealed_sprints.contains(sprint_id) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    code: "SPRINT_NOT_SEALED".into(),
                    message: format!("Sprint {} is not in sealed list", sprint_id),
                    detail: None,
                });
            }
        }

        // Check governance receipt refs
        for ref_id in &manifest.governance_receipt_refs {
            if !governance_refs.contains(ref_id) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    code: "MISSING_GOVERNANCE_REF".into(),
                    message: format!("Governance receipt {} not found", ref_id),
                    detail: None,
                });
            }
        }

        // Check for missing content hashes
        for c in &manifest.components {
            if c.content_hash.is_empty() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Warning,
                    code: "EMPTY_CONTENT_HASH".into(),
                    message: format!("Component {} has empty content hash", c.component_id),
                    detail: None,
                });
            }
        }

        // Check for empty release
        if manifest.components.is_empty() {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                code: "EMPTY_RELEASE".into(),
                message: "Release has no components".into(),
                detail: None,
            });
        }

        let valid = issues.iter().all(|i| !matches!(i.severity, ValidationSeverity::Error));
        let mut result = ValidationResult {
            manifest_id: manifest.release_id.clone(),
            issues,
            valid,
            integrity_hash: String::new(),
        };
        result.integrity_hash = result.compute_integrity_hash();
        result
    }

    /// Produce summary from a validation result.
    pub fn summary(result: &ValidationResult) -> ValidationSummary {
        let errors = result.issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Error)).count();
        let warnings = result.issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Warning)).count();
        let infos = result.issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Info)).count();
        ValidationSummary { total_issues: result.issues.len(), errors, warnings, infos }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ReleaseManifest {
        ReleaseManifest {
            release_id: "R-001".into(), version: crate::release::ReleaseVersion { major: 1, minor: 0, patch: 0 },
            components: vec![crate::release::ReleaseComponent { component_id: "C-001".into(), component_type: "model".into(), version: "1.0".into(), sprint_id: "S-001".into(), content_hash: "abc".into() }],
            governance_receipt_refs: vec!["GR-001".into()],
            included_sprint_ids: vec!["S-001".into()],
            created_at: "2026-01-01".into(), content_hash: String::new(),
        }
    }

    #[test] fn test_valid_manifest() {
        let r = ReleaseValidation::validate(&manifest(), &["S-001".into()], &["GR-001".into()]);
        assert!(r.valid);
    }

    #[test] fn test_missing_sprint() {
        let r = ReleaseValidation::validate(&manifest(), &[], &["GR-001".into()]);
        assert!(!r.valid);
        assert!(r.issues.iter().any(|i| i.code == "SPRINT_NOT_SEALED"));
    }

    #[test] fn test_missing_governance_ref() {
        let r = ReleaseValidation::validate(&manifest(), &["S-001".into()], &[]);
        assert!(!r.valid);
        assert!(r.issues.iter().any(|i| i.code == "MISSING_GOVERNANCE_REF"));
    }

    #[test] fn test_empty_components() {
        let mut m = manifest(); m.components.clear();
        let r = ReleaseValidation::validate(&m, &["S-001".into()], &["GR-001".into()]);
        assert!(!r.valid);
        assert!(r.issues.iter().any(|i| i.code == "EMPTY_RELEASE"));
    }

    #[test] fn test_summary_counts() {
        let r = ReleaseValidation::validate(&manifest(), &[], &[]);
        let s = ReleaseValidation::summary(&r);
        assert!(s.errors > 0);
    }

    #[test] fn test_integrity_hash_deterministic() {
        let r1 = ReleaseValidation::validate(&manifest(), &["S-001".into()], &["GR-001".into()]);
        let r2 = ReleaseValidation::validate(&manifest(), &["S-001".into()], &["GR-001".into()]);
        assert_eq!(r1.integrity_hash, r2.integrity_hash);
    }

    #[test] fn test_no_authority() {
        let r = ReleaseValidation::validate(&manifest(), &["S-001".into()], &["GR-001".into()]);
        let j = serde_json::to_value(&r).unwrap();
        assert!(j.get("approve").is_none()); assert!(j.get("recommend").is_none());
        assert!(j.get("quality").is_none()); assert!(j.get("decision").is_none());
    }
}
