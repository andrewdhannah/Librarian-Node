//! Release manifest — canonical release object with integrity.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A component included in a release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ReleaseComponent {
    pub component_id: String,
    pub component_type: String,
    pub version: String,
    pub sprint_id: String,
    pub content_hash: String,
}

/// Semantic version for a release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ReleaseVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for ReleaseVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Canonical release manifest.
///
/// Contains only factual information: identifier, version, components,
/// governance receipt references, and integrity hash.
///
/// Must NOT contain: approval, recommendation, quality scores,
/// deployment state, or release decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseManifest {
    pub release_id: String,
    pub version: ReleaseVersion,
    pub components: Vec<ReleaseComponent>,
    pub governance_receipt_refs: Vec<String>,
    pub included_sprint_ids: Vec<String>,
    pub created_at: String,
    pub content_hash: String,
}

impl ReleaseManifest {
    pub fn compute_content_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.release_id.as_bytes());
        h.update(self.version.to_string().as_bytes());
        for c in &self.components {
            h.update(c.component_id.as_bytes());
            h.update(c.content_hash.as_bytes());
        }
        for s in &self.included_sprint_ids {
            h.update(s.as_bytes());
        }
        format!("{:x}", h.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_version_display() {
        assert_eq!(ReleaseVersion { major: 1, minor: 2, patch: 3 }.to_string(), "1.2.3");
    }

    #[test] fn test_manifest_hash_deterministic() {
        let m1 = manifest(); let m2 = manifest();
        assert_eq!(m1.compute_content_hash(), m2.compute_content_hash());
    }

    #[test] fn test_hash_changes_with_field() {
        let mut m = manifest();
        let h1 = m.compute_content_hash();
        m.release_id = "different".into();
        assert_ne!(h1, m.compute_content_hash());
    }

    #[test] fn test_no_authority_fields() {
        let m = manifest();
        let j = serde_json::to_value(&m).unwrap();
        assert!(j.get("approved").is_none());
        assert!(j.get("recommended").is_none());
        assert!(j.get("quality").is_none());
        assert!(j.get("deploy").is_none());
        assert!(j.get("decision").is_none());
        assert!(j.get("risk").is_none());
    }

    fn manifest() -> ReleaseManifest {
        ReleaseManifest {
            release_id: "R-001".into(),
            version: ReleaseVersion { major: 1, minor: 0, patch: 0 },
            components: vec![ReleaseComponent {
                component_id: "C-001".into(), component_type: "model".into(),
                version: "1.0".into(), sprint_id: "S-001".into(), content_hash: "abc".into(),
            }],
            governance_receipt_refs: vec!["GR-001".into()],
            included_sprint_ids: vec!["S-001".into()],
            created_at: "2026-01-01".into(),
            content_hash: String::new(),
        }
    }
}
