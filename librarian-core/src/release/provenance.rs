//! Release provenance — complete release lineage from evidence to manifest.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A node in the provenance chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProvenanceNode {
    pub node_type: String,
    pub node_id: String,
    pub reference: String,
    pub content_hash: String,
}

/// A reference to evidence in the chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EvidenceReference {
    pub evidence_type: String,
    pub evidence_id: String,
    pub content_hash: String,
}

/// Complete release provenance — links a release to its full evidence chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseProvenance {
    pub release_id: String,
    pub chain: Vec<ProvenanceNode>,
    pub evidence_refs: Vec<EvidenceReference>,
    pub integrity_hash: String,
}

impl ReleaseProvenance {
    pub fn compute_integrity_hash(&self) -> String {
        let mut h = Sha256::new();
        for n in &self.chain { h.update(n.node_id.as_bytes()); h.update(n.content_hash.as_bytes()); }
        for e in &self.evidence_refs { h.update(e.evidence_id.as_bytes()); h.update(e.content_hash.as_bytes()); }
        format!("{:x}", h.finalize())
    }

    /// Build provenance from manifest + sprint + governance refs.
    pub fn build(
        release_id: &str,
        sprint_ids: &[String],
        governance_refs: &[String],
        evidence_hashes: &[String],
    ) -> Self {
        let mut chain = Vec::new();
        chain.push(ProvenanceNode { node_type: "release".into(), node_id: release_id.into(), reference: "manifest".into(), content_hash: String::new() });
        for s in sprint_ids {
            chain.push(ProvenanceNode { node_type: "sprint".into(), node_id: s.clone(), reference: "ledger".into(), content_hash: String::new() });
        }
        for g in governance_refs {
            chain.push(ProvenanceNode { node_type: "governance".into(), node_id: g.clone(), reference: "receipt".into(), content_hash: String::new() });
        }
        let evidence_refs: Vec<EvidenceReference> = evidence_hashes.iter().enumerate().map(|(i, h)| {
            EvidenceReference { evidence_type: "capability".into(), evidence_id: format!("ev-{}", i), content_hash: h.clone() }
        }).collect();
        let mut p = ReleaseProvenance { release_id: release_id.into(), chain, evidence_refs, integrity_hash: String::new() };
        p.integrity_hash = p.compute_integrity_hash();
        p
    }

    /// Verify integrity hash matches recomputation.
    pub fn verify(&self) -> bool { self.integrity_hash == self.compute_integrity_hash() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_build_provenance() {
        let p = ReleaseProvenance::build("R-001", &["S-001".into()], &["GR-001".into()], &["abc".into()]);
        assert_eq!(p.chain.len(), 3); // release + sprint + governance
        assert_eq!(p.evidence_refs.len(), 1);
    }

    #[test] fn test_hash_deterministic() {
        let p1 = ReleaseProvenance::build("R-1", &["S-1".into()], &["GR-1".into()], &["h1".into()]);
        let p2 = ReleaseProvenance::build("R-1", &["S-1".into()], &["GR-1".into()], &["h1".into()]);
        assert_eq!(p1.integrity_hash, p2.integrity_hash);
    }

    #[test] fn test_verify_passes() {
        let p = ReleaseProvenance::build("R-1", &["S-1".into()], &["GR-1".into()], &["h1".into()]);
        assert!(p.verify());
    }

    #[test] fn test_no_authority() {
        let p = ReleaseProvenance::build("R-1", &["S-1".into()], &["GR-1".into()], &["h1".into()]);
        let j = serde_json::to_value(&p).unwrap();
        assert!(j.get("approve").is_none()); assert!(j.get("recommend").is_none());
    }
}
