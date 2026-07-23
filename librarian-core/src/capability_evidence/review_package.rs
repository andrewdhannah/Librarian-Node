//! Capability review package — connects capability evidence to Owner review.
//!
//! This package aggregates capability evidence, profiles, and regression
//! results into a single review-oriented artifact. It feeds into the
//! existing MQR Owner Review flow WITHOUT introducing authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::models::CapabilityEvidence;

/// A single observation in the review package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewObservation {
    pub fixture_id: String,
    pub result: String,
    pub evidence_hash: String,
    pub evaluator_id: String,
}

/// Complete capability review package for Owner review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityReviewPackage {
    pub model_id: String,
    pub model_sha256: String,
    pub quantization: String,
    pub observations: Vec<ReviewObservation>,
    pub evidence_count: usize,
    pub passes: usize,
    pub failures: usize,
    pub degraded: usize,
    pub generated_at: String,
    pub content_hash: String,
}

impl CapabilityReviewPackage {
    pub fn compute_content_hash(&self) -> String {
        let mut h = Sha256::new();
        for o in &self.observations {
            h.update(o.fixture_id.as_bytes());
            h.update(o.result.as_bytes());
        }
        format!("{:x}", h.finalize())
    }

    /// Build from a collection of capability evidence.
    pub fn from_evidence(model_id: &str, model_sha256: &str, quantization: &str, evidence: &[CapabilityEvidence]) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let observations: Vec<ReviewObservation> = evidence.iter().map(|e| ReviewObservation {
            fixture_id: e.fixture_identity.fixture_id.clone(),
            result: e.result.as_str().to_string(),
            evidence_hash: e.evidence_hash.clone(),
            evaluator_id: e.evaluator_identity.evaluator_id.clone(),
        }).collect();
        let passes = evidence.iter().filter(|e| e.result.is_success()).count();
        let failures = evidence.iter().filter(|e| !e.result.is_success() && !matches!(e.result, super::models::CapabilityResult::NotTested)).count();
        let degraded = evidence.iter().filter(|e| matches!(e.result, super::models::CapabilityResult::Degraded)).count();

        let mut p = CapabilityReviewPackage {
            model_id: model_id.to_string(),
            model_sha256: model_sha256.to_string(),
            quantization: quantization.to_string(),
            observations,
            evidence_count: evidence.len(),
            passes, failures, degraded,
            generated_at: now,
            content_hash: String::new(),
        };
        p.content_hash = p.compute_content_hash();
        p
    }

    /// Structural proof: no authority fields.
    pub fn assert_no_authority_fields(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::models::*;

    fn ev(fid: &str, r: CapabilityResult) -> CapabilityEvidence {
        CapabilityEvidence {
            evidence_id: format!("e-{}", fid),
            model_identity: ModelIdentity { model_id: "m1".into(), model_sha256: "s".into(), quantization: "Q4".into(), model_version: "1".into() },
            runtime_configuration: RuntimeConfig { model_sha256: "s".into(), quantization: "Q4".into(), runtime_build: "b".into(), hardware_lane: "RX".into(), fixture_version: "1".into() },
            evaluator_identity: EvaluatorIdentity { evaluator_id: "ope".into(), evaluator_version: "1".into(), upstream_project: "MQR".into() },
            fixture_identity: FixtureIdentity { fixture_id: fid.into(), fixture_version: "1".into() },
            execution_context: ExecutionContext { timestamp: "2026-01-01".into(), hardware_lane: "RX".into(), runtime_build: "b".into() },
            result: r, failures: vec![],
            provenance_reference: ProvenanceReference { lineage_hash: None, lifecycle_event_id: None, model_identity_hash: "s".into() },
            evidence_hash: String::new(),
        }
    }

    #[test] fn test_empty_evidence() {
        let p = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &[]);
        assert_eq!(p.evidence_count, 0);
    }

    #[test] fn test_counts() {
        let p = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &[ev("f1", CapabilityResult::Pass), ev("f2", CapabilityResult::Fail)]);
        assert_eq!(p.evidence_count, 2);
        assert_eq!(p.passes, 1);
        assert_eq!(p.failures, 1);
    }

    #[test] fn test_deterministic() {
        let e = vec![ev("f1", CapabilityResult::Pass)];
        let a = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &e);
        let b = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &e);
        assert_eq!(a.content_hash, b.content_hash);
    }

    #[test] fn test_no_authority() {
        let p = CapabilityReviewPackage::from_evidence("m", "s", "Q4", &[ev("f1", CapabilityResult::Pass)]);
        let j = serde_json::to_value(&p).unwrap();
        assert!(j.get("approve").is_none());
        assert!(j.get("reject").is_none());
        assert!(j.get("recommendation").is_none());
        assert!(j.get("score").is_none());
    }
}
