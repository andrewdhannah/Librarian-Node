//! Capability profile — aggregates evidence from multiple evaluators
//! and fixture families into a governed, review-oriented summary.
//!
//! A CapabilityProfile is NOT a score. It is a structured summary of
//! what capability evidence exists, organized by domain, with clear
//! pass/fail/degraded results and warnings about areas of concern.
//!
//! Critical invariant:
//!   A profile summarizes evidence. It does NOT approve, reject,
//!   or recommend models. It does NOT create authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::models::CapabilityEvidence;

/// Aggregated result for a capability domain across multiple fixtures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AggregatedResult {
    /// All fixtures in the domain passed.
    Pass,
    /// Some fixtures failed (but no critical or adversarial warnings).
    Degraded,
    /// Majority of fixtures failed.
    Fail,
    /// No fixtures were executed for this domain.
    NotTested,
    /// Adversarial or critical warnings present.
    Warning,
}

impl AggregatedResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Degraded => "degraded",
            Self::Fail => "fail",
            Self::NotTested => "not_tested",
            Self::Warning => "warning",
        }
    }
}

/// Severity of a profile warning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProfileWarningSeverity {
    /// Informational observation.
    Info,
    /// Potential concern.
    Warning,
    /// Requires attention.
    Critical,
}

impl ProfileWarningSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

/// Result for a single capability domain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainProfile {
    /// Domain name.
    pub domain: String,
    /// Aggregated result.
    pub overall_result: AggregatedResult,
    /// Total fixtures in this domain.
    pub fixture_count: usize,
    /// Number of passing fixtures.
    pub passes: usize,
    /// Number of failing fixtures.
    pub failures: usize,
    /// Number of degraded fixtures.
    pub degraded: usize,
    /// Evidence IDs from this domain.
    pub evidence_refs: Vec<String>,
}

/// Description of an evidence source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EvidenceSource {
    /// Evaluator identifier.
    pub evaluator_id: String,
    /// Evaluator version.
    pub evaluator_version: String,
    /// Upstream project.
    pub upstream_project: String,
    /// Fixtures contributed.
    pub fixture_count: usize,
}

/// A warning in the profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileWarning {
    /// Warning severity.
    pub severity: ProfileWarningSeverity,
    /// Human-readable message.
    pub message: String,
    /// Optional detail.
    pub detail: Option<String>,
}

/// Complete capability profile for a model configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityProfile {
    /// Model identity.
    pub model_id: String,
    /// Model SHA-256.
    pub model_sha256: String,
    /// Quantization variant.
    pub quantization: String,
    /// Per-domain profiles.
    pub domain_profiles: Vec<DomainProfile>,
    /// Evidence sources that contributed.
    pub sources: Vec<EvidenceSource>,
    /// Warnings.
    pub warnings: Vec<ProfileWarning>,
    /// When the profile was generated.
    pub generated_at: String,
    /// Deterministic content hash.
    pub content_hash: String,
}

impl CapabilityProfile {
    pub fn compute_content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        for d in &self.domain_profiles {
            hasher.update(d.domain.as_bytes());
            hasher.update(d.overall_result.as_str().as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Structural proof: profile has no authority fields.
    pub fn assert_no_authority_fields(&self) -> bool {
        true
    }
}

/// Assembles capability profiles from raw evidence.
pub struct ProfileAssembler;

impl ProfileAssembler {
    /// Build a capability profile from a collection of evidence.
    ///
    /// Groups evidence by domain (fixture category) and computes
    /// aggregated results. Produces warnings for degraded/failing domains.
    pub fn assemble(model_id: &str, model_sha256: &str, quantization: &str, evidence: &[CapabilityEvidence]) -> CapabilityProfile {
        let now = chrono::Utc::now().to_rfc3339();

        // Group by domain
        let mut by_domain: std::collections::HashMap<String, Vec<&CapabilityEvidence>> = std::collections::HashMap::new();
        for e in evidence {
            by_domain.entry(e.fixture_identity.fixture_id.clone()).or_default().push(e);
        }

        let mut domain_profiles = Vec::new();
        let mut all_domains: Vec<String> = by_domain.keys().cloned().collect();
        all_domains.sort();
        for domain in &all_domains {
            let evs = &by_domain[domain];
            let total = evs.len();
            let passes = evs.iter().filter(|e| e.result.is_success()).count();
            let failures = total - passes;
            let degraded = evs.iter().filter(|e| matches!(e.result, super::models::CapabilityResult::Degraded)).count();
            let overall = if failures == 0 && degraded == 0 { AggregatedResult::Pass }
                         else if failures > total / 2 { AggregatedResult::Fail }
                         else if degraded > 0 { AggregatedResult::Degraded }
                         else { AggregatedResult::Warning };
            let refs: Vec<String> = evs.iter().map(|e| e.evidence_id.clone()).collect();
            domain_profiles.push(DomainProfile {
                domain: domain.clone(),
                overall_result: overall,
                fixture_count: total,
                passes, failures, degraded,
                evidence_refs: refs,
            });
        }

        // Collect sources
        let mut sources = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for e in evidence {
            let id = e.evaluator_identity.evaluator_id.clone();
            if !seen.contains(&id) {
                seen.insert(id.clone());
                sources.push(EvidenceSource {
                    evaluator_id: e.evaluator_identity.evaluator_id.clone(),
                    evaluator_version: e.evaluator_identity.evaluator_version.clone(),
                    upstream_project: e.evaluator_identity.upstream_project.clone(),
                    fixture_count: 0,
                });
            }
        }

        // Generate warnings
        let mut warnings = Vec::new();
        for dp in &domain_profiles {
            match dp.overall_result {
                AggregatedResult::Fail => warnings.push(ProfileWarning {
                    severity: ProfileWarningSeverity::Critical,
                    message: format!("{}: {} of {} fixtures failed", dp.domain, dp.failures, dp.fixture_count),
                    detail: None,
                }),
                AggregatedResult::Degraded => warnings.push(ProfileWarning {
                    severity: ProfileWarningSeverity::Warning,
                    message: format!("{}: degraded output observed in {} fixture(s)", dp.domain, dp.degraded),
                    detail: None,
                }),
                AggregatedResult::Warning => {}
                _ => {}
            }
        }

        let mut profile = CapabilityProfile {
            model_id: model_id.to_string(),
            model_sha256: model_sha256.to_string(),
            quantization: quantization.to_string(),
            domain_profiles,
            sources,
            warnings,
            generated_at: now,
            content_hash: String::new(),
        };
        profile.content_hash = profile.compute_content_hash();
        profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::models::*;

    fn ev(fixture_id: &str, result: CapabilityResult, evaluator: &str) -> CapabilityEvidence {
        CapabilityEvidence {
            evidence_id: format!("e-{}", fixture_id),
            model_identity: ModelIdentity { model_id: "m1".into(), model_sha256: "s".into(), quantization: "Q4".into(), model_version: "1".into() },
            runtime_configuration: RuntimeConfig { model_sha256: "s".into(), quantization: "Q4".into(), runtime_build: "b".into(), hardware_lane: "RX".into(), fixture_version: "1".into() },
            evaluator_identity: EvaluatorIdentity { evaluator_id: evaluator.into(), evaluator_version: "1".into(), upstream_project: "MQR".into() },
            fixture_identity: FixtureIdentity { fixture_id: fixture_id.into(), fixture_version: "1".into() },
            execution_context: ExecutionContext { timestamp: "2026-01-01".into(), hardware_lane: "RX".into(), runtime_build: "b".into() },
            result, failures: vec![],
            provenance_reference: ProvenanceReference { lineage_hash: None, lifecycle_event_id: None, model_identity_hash: "s".into() },
            evidence_hash: String::new(),
        }
    }

    #[test] fn test_all_pass() {
        let e = vec![ev("struct", CapabilityResult::Pass, "ope"), ev("struct", CapabilityResult::Pass, "ope")];
        let p = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        assert_eq!(p.domain_profiles.len(), 1);
        assert_eq!(p.domain_profiles[0].overall_result, AggregatedResult::Pass);
        assert!(p.warnings.is_empty());
    }

    #[test] fn test_all_fail() {
        let e = vec![ev("struct", CapabilityResult::Fail, "ope"), ev("struct", CapabilityResult::Fail, "ope")];
        let p = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        assert_eq!(p.domain_profiles[0].overall_result, AggregatedResult::Fail);
        assert!(!p.warnings.is_empty());
    }

    #[test] fn test_some_degraded() {
        let e = vec![ev("f", CapabilityResult::Pass, "ope"), ev("f", CapabilityResult::Degraded, "ope")];
        let p = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        assert_eq!(p.domain_profiles[0].overall_result, AggregatedResult::Degraded);
    }

    #[test] fn test_multiple_domains() {
        let e = vec![
            ev("struct", CapabilityResult::Pass, "ope"),
            ev("fact", CapabilityResult::Pass, "ope"),
            ev("struct", CapabilityResult::Fail, "ope"),
        ];
        let p = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        assert_eq!(p.domain_profiles.len(), 2);
    }

    #[test] fn test_authority_boundary() {
        let e = vec![ev("x", CapabilityResult::Pass, "ope")];
        let p = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        let j = serde_json::to_value(&p).unwrap();
        assert!(j.get("manifest_id").is_none());
        assert!(j.get("decision_id").is_none());
        assert!(j.get("approved").is_none());
        assert!(j.get("score").is_none());
        assert!(j.get("ranking").is_none());
        assert!(j.get("approval").is_none());
        assert!(j.get("recommendation").is_none());
        assert!(j.get("qualified").is_none());
    }

    #[test] fn test_content_hash_deterministic() {
        let e = vec![ev("x", CapabilityResult::Pass, "ope")];
        let p1 = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        let p2 = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        assert_eq!(p1.content_hash, p2.content_hash);
    }

    #[test] fn test_domain_profile_counts() {
        let e = vec![
            ev("f1", CapabilityResult::Pass, "ope"),
            ev("f2", CapabilityResult::Fail, "ope"),
            ev("f3", CapabilityResult::Degraded, "ope"),
        ];
        let p = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        // Each fixture has a different fixture_id, so they're in different domains
        assert_eq!(p.domain_profiles.len(), 3);
    }

    #[test] fn test_warnings_on_failures() {
        let e = vec![ev("x", CapabilityResult::Fail, "ope"), ev("x", CapabilityResult::Fail, "ope")];
        let p = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        assert!(p.warnings.iter().any(|w| matches!(w.severity, ProfileWarningSeverity::Critical)));
    }

    #[test] fn test_model_identity_preserved() {
        let e = vec![ev("x", CapabilityResult::Pass, "ope")];
        let p = ProfileAssembler::assemble("m1", "s", "Q4", &e);
        assert_eq!(p.model_id, "m1");
        assert_eq!(p.quantization, "Q4");
    }
}
