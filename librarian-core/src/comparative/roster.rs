//! Roster recommendation — evaluates comparison results to recommend
//! roster position for a candidate model.
//!
//! The roster is the authoritative list of approved models per role.
//! The analyzer RECOMMENDS a roster position; it does NOT execute
//! supersession or mutate Owner-approved router policy.
//!
//! Critical invariants:
//! - Supersession recommendation ≠ automatic supersession
//! - Comparative classifier finding ≠ Owner decision
//! - Rejected/superseded records preserve model identity, role, comparison basis,
//!   dominant model, evidence references, and retest triggers

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::analyzer::ComparisonResult;
use super::finding::{FindingType, FindingSeverity};

/// Roster position recommendation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RosterPosition {
    /// Candidate should be added to the roster (new role or new model).
    Add,
    /// No roster change recommended (candidate is equivalent or dominated).
    NoChange,
    /// Candidate should replace the baseline (recommendation only, not executed).
    Supersede,
    /// Insufficient evidence to make a recommendation.
    InsufficientEvidence,
}

impl RosterPosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::NoChange => "no_change",
            Self::Supersede => "supersede",
            Self::InsufficientEvidence => "insufficient_evidence",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "add" => Some(Self::Add),
            "no_change" => Some(Self::NoChange),
            "supersede" => Some(Self::Supersede),
            "insufficient_evidence" => Some(Self::InsufficientEvidence),
            _ => None,
        }
    }
}

/// Retest trigger — when should the model be re-evaluated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RetestTrigger {
    /// No retest needed.
    None,
    /// Retest when new qualification evidence is available.
    NewEvidence,
    /// Retest when hardware changes.
    HardwareChange,
    /// Retest when runtime version changes.
    RuntimeChange,
    /// Retest on a fixed schedule (calendar date in `trigger_at`).
    Scheduled(String),
}

/// Supersession record — documents which model should be replaced.
///
/// This is a RECOMMENDATION only. The record must be reviewed by the
/// Owner before any actual supersession occurs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupersessionRecord {
    /// Model being superseded (baseline).
    pub superseded_model_id: String,
    /// Role being compared.
    pub role: String,
    /// Comparison basis (key advantages of the superseding model).
    pub comparison_basis: String,
    /// The model recommended to supersede (candidate).
    pub superseding_model_id: String,
    /// Evidence references supporting supersession.
    pub evidence_refs: Vec<String>,
    /// When this recommendation was created.
    pub created_at: String,
}

/// Rejection record — documents why a candidate was not added to the roster.
///
/// Preserves identity, role, comparison basis, and retest triggers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RejectionRecord {
    /// Model that was rejected.
    pub model_id: String,
    /// Role compared.
    pub role: String,
    /// Why the candidate was rejected.
    pub reason: String,
    /// The dominant model (if the candidate was dominated).
    pub dominant_model_id: Option<String>,
    /// Evidence references.
    pub evidence_refs: Vec<String>,
    /// When to re-evaluate this candidate.
    pub retest_trigger: RetestTrigger,
    /// When this rejection was recorded.
    pub created_at: String,
}

/// Complete roster recommendation output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RosterRecommendation {
    /// Recommended position.
    pub position: RosterPosition,
    /// Candidate model ID.
    pub candidate_model_id: String,
    /// Role evaluated.
    pub role: String,
    /// Human-readable recommendation reason.
    pub reason: String,
    /// Supersession record (if Supersede recommended).
    pub supersession: Option<SupersessionRecord>,
    /// Rejection record (if NoChange or dominated).
    pub rejection: Option<RejectionRecord>,
    /// Retest trigger for this recommendation.
    pub retest_trigger: RetestTrigger,
    /// Reference to the comparison result.
    pub comparison_hash: String,
    /// When this recommendation was created.
    pub created_at: String,
}

/// Evaluate a comparison result and produce a roster recommendation.
///
/// This function is pure — it does NOT create database records,
/// modify the roster, or execute supersession.
pub fn evaluate_roster(result: &ComparisonResult) -> Result<RosterRecommendation> {
    use super::analyzer::compute_comparison_hash;
    let comparison_hash = compute_comparison_hash(result)?;
    let created_at = chrono::Utc::now().to_rfc3339();

    // If not comparable, recommend InsufficientEvidence
    if !result.is_comparable {
        let position = RosterPosition::InsufficientEvidence;
        let reason = format!(
            "Insufficient comparable evidence between '{}' and '{}' for role '{}'",
            result.candidate_model_id, result.baseline_model_id, result.role
        );

        let rejection = Some(RejectionRecord {
            model_id: result.candidate_model_id.clone(),
            role: result.role.clone(),
            reason: reason.clone(),
            dominant_model_id: None,
            evidence_refs: result
                .findings
                .iter()
                .flat_map(|f| f.evidence_refs.clone())
                .collect(),
            retest_trigger: RetestTrigger::NewEvidence,
            created_at: created_at.clone(),
        });

        return Ok(RosterRecommendation {
            position,
            candidate_model_id: result.candidate_model_id.clone(),
            role: result.role.clone(),
            reason,
            supersession: None,
            rejection,
            retest_trigger: RetestTrigger::NewEvidence,
            comparison_hash,
            created_at,
        });
    }

    // Check for unique role advantage (no other model fills this role)
    let has_unique_role = result.findings.iter().any(|f| f.finding_type == FindingType::UniqueRoleAdvantage);

    // Check for supersession recommendation
    let has_supersedes = result.findings.iter().any(|f| f.finding_type == FindingType::SupersedesExistingModel);

    // Check for dominated
    let is_dominated = result.findings.iter().any(|f| f.finding_type == FindingType::DominatedByExistingModel);

    // Check for equivalent
    let is_equivalent = result.findings.iter().any(|f| f.finding_type == FindingType::EquivalentNoMaterialAdvantage);

    // Determine position
    let (position, reason) = if has_supersedes {
        let advantages: Vec<&str> = result
            .findings
            .iter()
            .filter(|f| {
                matches!(
                    f.finding_type,
                    FindingType::QualityAdvantage
                        | FindingType::LatencyAdvantage
                        | FindingType::MemoryAdvantage
                )
            })
            .map(|f| f.finding_type.as_str())
            .collect();

        (
            RosterPosition::Supersede,
            format!(
                "Candidate '{}' exceeds baseline '{}' on [{}] for role '{}'. Recommend supersession.",
                result.candidate_model_id,
                result.baseline_model_id,
                advantages.join(", "),
                result.role,
            ),
        )
    } else if has_unique_role {
        (
            RosterPosition::Add,
            format!(
                "Candidate '{}' fills role '{}' with no other qualified model. Recommend adding to roster.",
                result.candidate_model_id, result.role,
            ),
        )
    } else if is_dominated {
        (
            RosterPosition::NoChange,
            format!(
                "Candidate '{}' is dominated by baseline '{}' for role '{}'. No roster change recommended.",
                result.candidate_model_id, result.baseline_model_id, result.role,
            ),
        )
    } else if is_equivalent {
        (
            RosterPosition::NoChange,
            format!(
                "Candidate '{}' is equivalent to baseline '{}' for role '{}'. No material advantage for roster change.",
                result.candidate_model_id, result.baseline_model_id, result.role,
            ),
        )
    } else {
        // Has some advantages but not enough for supersession
        (
            RosterPosition::NoChange,
            format!(
                "Candidate '{}' has limited advantages over baseline '{}' for role '{}'. No roster change recommended.",
                result.candidate_model_id, result.baseline_model_id, result.role,
            ),
        )
    };

    // Build supersession record if recommended
    let supersession = if position == RosterPosition::Supersede {
        let comparison_basis = result
            .findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Advantage)
            .map(|f| f.basis.as_str())
            .collect::<Vec<_>>()
            .join("; ");

        Some(SupersessionRecord {
            superseded_model_id: result.baseline_model_id.clone(),
            role: result.role.clone(),
            comparison_basis,
            superseding_model_id: result.candidate_model_id.clone(),
            evidence_refs: result
                .findings
                .iter()
                .flat_map(|f| f.evidence_refs.clone())
                .collect(),
            created_at: created_at.clone(),
        })
    } else {
        None
    };

    // Build rejection record if dominated
    let rejection = if position == RosterPosition::NoChange && is_dominated {
        Some(RejectionRecord {
            model_id: result.candidate_model_id.clone(),
            role: result.role.clone(),
            reason: reason.clone(),
            dominant_model_id: Some(result.baseline_model_id.clone()),
            evidence_refs: result
                .findings
                .iter()
                .flat_map(|f| f.evidence_refs.clone())
                .collect(),
            retest_trigger: RetestTrigger::NewEvidence,
            created_at: created_at.clone(),
        })
    } else {
        None
    };

    // Determine retest trigger
    let retest_trigger = if is_dominated || is_equivalent {
        RetestTrigger::NewEvidence
    } else if has_supersedes {
        RetestTrigger::None
    } else {
        RetestTrigger::NewEvidence
    };

    Ok(RosterRecommendation {
        position,
        candidate_model_id: result.candidate_model_id.clone(),
        role: result.role.clone(),
        reason,
        supersession,
        rejection,
        retest_trigger,
        comparison_hash,
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::analyzer::{compare_role, ComparisonInput};
    use crate::capability::manifest::{CapabilityManifest, EvidenceSummary, ManifestStatus};
    use crate::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity, ProfileStatus,
        RuntimeIdentity,
    };

    fn make_manifest(model_id: &str, role: &str, status: ManifestStatus) -> CapabilityManifest {
        let created_at = "2026-07-11T12:00:00Z".to_string();
        let manifest_id = CapabilityManifest::compute_manifest_id(model_id, role, &created_at);
        CapabilityManifest {
            manifest_id,
            model_id: model_id.to_string(),
            model_sha256: "abc123".to_string(),
            model_filename: format!("{}-model.gguf", model_id),
            role: role.to_string(),
            status,
            evidence_summary: EvidenceSummary {
                smoke_test_passed: true,
                probes_passed: vec!["PP-001".to_string()],
                probes_failed: vec![],
                total_generation_duration_ms: Some(5000),
                total_output_tokens: Some(500),
                gpu_release_verified: true,
                notes: None,
            },
            failure_modes: vec![],
            constraints: None,
            owner_decision_id: Some("dec-001".to_string()),
            supersedes_manifest_id: None,
            content_hash: String::new(),
            created_at: created_at.clone(),
            updated_at: created_at,
        }
    }

    fn make_profile(
        model_id: &str,
        avg_tps: Option<f64>,
        avg_gen_ms: Option<f64>,
        vram_mb: u64,
        file_size: u64,
    ) -> ExecutionProfile {
        ExecutionProfile {
            profile_id: ExecutionProfile::compute_profile_id(model_id, "c85e97a", "Radeon RX 570"),
            artifact: ArtifactIdentity {
                filename: format!("{}-model.gguf", model_id),
                model_id: model_id.to_string(),
                quantization: "Q4_K_M".to_string(),
                sha256: "abc123".to_string(),
                file_size_bytes: file_size,
            },
            runtime: RuntimeIdentity {
                executable: "llama-server.exe".to_string(),
                version: "c85e97a".to_string(),
                backend: "vulkan".to_string(),
                device_id: Some("Vulkan0".to_string()),
            },
            hardware: HardwareIdentity {
                gpu_description: "Radeon RX 570".to_string(),
                gpu_vram_mb: vram_mb,
                cpu: "Intel Core i7-7700K".to_string(),
                ram_mb: 16384,
                os: "windows".to_string(),
            },
            metrics: ExecutionMetrics {
                avg_load_duration_ms: Some(2000.0),
                avg_generation_duration_ms: avg_gen_ms,
                avg_tokens_per_second: avg_tps,
                peak_vram_usage_mb: Some(vram_mb.saturating_sub(500)),
                observation_count: 5,
            },
            status: ProfileStatus::Active,
            content_hash: String::new(),
            created_at: "2026-07-11T12:00:00Z".to_string(),
            updated_at: "2026-07-11T12:00:00Z".to_string(),
        }
    }

    fn run_comparison_and_evaluate(
        candidate_id: &str,
        baseline_id: &str,
        role: &str,
        c_tps: Option<f64>,
        c_gen: Option<f64>,
        c_vram: u64,
        c_size: u64,
        b_tps: Option<f64>,
        b_gen: Option<f64>,
        b_vram: u64,
        b_size: u64,
        other_fillers: Vec<&str>,
    ) -> RosterRecommendation {
        let input = ComparisonInput {
            candidate_manifest: make_manifest(candidate_id, role, ManifestStatus::Approved),
            candidate_profile: make_profile(candidate_id, c_tps, c_gen, c_vram, c_size),
            baseline_manifest: make_manifest(baseline_id, role, ManifestStatus::Approved),
            baseline_profile: make_profile(baseline_id, b_tps, b_gen, b_vram, b_size),
            role: role.to_string(),
            other_role_fillers: other_fillers.into_iter().map(String::from).collect(),
        };
        let result = compare_role(&input);
        evaluate_roster(&result).unwrap()
    }

    // C3-R1: Supersede recommendation when candidate exceeds baseline
    #[test]
    fn test_roster_supersede() {
        let rec = run_comparison_and_evaluate(
            "model-new", "model-old", "classifier",
            Some(18.0), Some(2000.0), 2048, 400_000_000,
            Some(10.0), Some(3500.0), 4096, 700_000_000,
            vec!["model-old"],
        );
        assert_eq!(rec.position, RosterPosition::Supersede);
        assert!(rec.supersession.is_some());
        let sup = rec.supersession.unwrap();
        assert_eq!(sup.superseded_model_id, "model-old");
        assert_eq!(sup.superseding_model_id, "model-new");
        assert_eq!(sup.role, "classifier");
    }

    // C3-R2: Add recommendation for unique role
    #[test]
    fn test_roster_add_unique_role() {
        let rec = run_comparison_and_evaluate(
            "model-new", "model-other", "summarizer",
            Some(12.5), Some(3000.0), 4096, 700_000_000,
            Some(12.5), Some(3000.0), 4096, 700_000_000,
            vec![], // No other model fills summarizer
        );
        assert_eq!(rec.position, RosterPosition::Add);
        assert!(rec.supersession.is_none());
        assert!(rec.rejection.is_none());
    }

    // C3-R3: NoChange for dominated candidate
    #[test]
    fn test_roster_no_change_dominated() {
        let rec = run_comparison_and_evaluate(
            "model-weak", "model-strong", "classifier",
            Some(8.0), Some(5000.0), 8192, 1_200_000_000,
            Some(15.0), Some(2500.0), 4096, 700_000_000,
            vec!["model-strong"],
        );
        assert_eq!(rec.position, RosterPosition::NoChange);
        assert!(rec.rejection.is_some());
        let rej = rec.rejection.unwrap();
        assert_eq!(rej.model_id, "model-weak");
        assert_eq!(rej.dominant_model_id, Some("model-strong".to_string()));
    }

    // C3-R4: NoChange for equivalent
    #[test]
    fn test_roster_no_change_equivalent() {
        let rec = run_comparison_and_evaluate(
            "model-a", "model-b", "classifier",
            Some(12.6), Some(3000.0), 4096, 700_000_000,
            Some(12.5), Some(3050.0), 4096, 700_000_000,
            vec!["model-b"],
        );
        assert_eq!(rec.position, RosterPosition::NoChange);
    }

    // C3-R5: InsufficientEvidence
    #[test]
    fn test_roster_insufficient_evidence() {
        let mut candidate = make_manifest("model-new", "classifier", ManifestStatus::Approved);
        candidate.evidence_summary.smoke_test_passed = false;
        let input = ComparisonInput {
            candidate_manifest: candidate,
            candidate_profile: make_profile("model-new", None, None, 4096, 700_000_000),
            baseline_manifest: make_manifest("model-old", "classifier", ManifestStatus::Approved),
            baseline_profile: make_profile("model-old", Some(12.5), Some(3000.0), 4096, 700_000_000),
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-old".to_string()],
        };
        let result = compare_role(&input);
        let rec = evaluate_roster(&result).unwrap();
        assert_eq!(rec.position, RosterPosition::InsufficientEvidence);
        assert!(rec.rejection.is_some());
        assert_eq!(rec.retest_trigger, RetestTrigger::NewEvidence);
    }

    // C3-R6: Supersession record preserves all required fields
    #[test]
    fn test_supersession_record_preserves_fields() {
        let rec = run_comparison_and_evaluate(
            "model-new", "model-old", "classifier",
            Some(18.0), Some(2000.0), 2048, 400_000_000,
            Some(10.0), Some(3500.0), 4096, 700_000_000,
            vec!["model-old"],
        );
        let sup = rec.supersession.unwrap();
        // All required fields present
        assert!(!sup.superseded_model_id.is_empty());
        assert!(!sup.role.is_empty());
        assert!(!sup.comparison_basis.is_empty());
        assert!(!sup.superseding_model_id.is_empty());
        assert!(!sup.evidence_refs.is_empty());
        assert!(!sup.created_at.is_empty());
    }

    // C3-R7: Rejection record preserves all required fields
    #[test]
    fn test_rejection_record_preserves_fields() {
        let rec = run_comparison_and_evaluate(
            "model-weak", "model-strong", "classifier",
            Some(8.0), Some(5000.0), 8192, 1_200_000_000,
            Some(15.0), Some(2500.0), 4096, 700_000_000,
            vec!["model-strong"],
        );
        let rej = rec.rejection.unwrap();
        assert!(!rej.model_id.is_empty());
        assert!(!rej.role.is_empty());
        assert!(!rej.reason.is_empty());
        assert!(!rej.evidence_refs.is_empty());
        assert!(!rej.created_at.is_empty());
    }

    // C3-R8: Recommendation is role-specific
    #[test]
    fn test_recommendation_role_specific() {
        let rec = run_comparison_and_evaluate(
            "model-a", "model-b", "summarizer",
            Some(18.0), Some(2000.0), 2048, 400_000_000,
            Some(10.0), Some(3500.0), 4096, 700_000_000,
            vec![],
        );
        assert_eq!(rec.role, "summarizer");
    }

    // C3-R9: Supersession does NOT auto-execute (recommendation only)
    #[test]
    fn test_supersession_is_recommendation_only() {
        let rec = run_comparison_and_evaluate(
            "model-new", "model-old", "classifier",
            Some(18.0), Some(2000.0), 2048, 400_000_000,
            Some(10.0), Some(3500.0), 4096, 700_000_000,
            vec!["model-old"],
        );
        // The recommendation is Supersede, but there's no mutation
        assert_eq!(rec.position, RosterPosition::Supersede);
        // Supersession record exists but is just a record
        let sup = rec.supersession.unwrap();
        assert!(!sup.created_at.is_empty());
        // No database mutation, no automatic status change — just a recommendation
    }

    // C3-R10: Retest trigger for dominated is NewEvidence
    #[test]
    fn test_retest_trigger_dominated() {
        let rec = run_comparison_and_evaluate(
            "model-weak", "model-strong", "classifier",
            Some(8.0), Some(5000.0), 8192, 1_200_000_000,
            Some(15.0), Some(2500.0), 4096, 700_000_000,
            vec!["model-strong"],
        );
        assert_eq!(rec.retest_trigger, RetestTrigger::NewEvidence);
    }

    // C3-R11: Retest trigger for supersede is None
    #[test]
    fn test_retest_trigger_supersede() {
        let rec = run_comparison_and_evaluate(
            "model-new", "model-old", "classifier",
            Some(18.0), Some(2000.0), 2048, 400_000_000,
            Some(10.0), Some(3500.0), 4096, 700_000_000,
            vec!["model-old"],
        );
        assert_eq!(rec.retest_trigger, RetestTrigger::None);
    }

    // C3-R12: Comparison hash is present and valid
    #[test]
    fn test_comparison_hash_present() {
        let rec = run_comparison_and_evaluate(
            "model-a", "model-b", "classifier",
            Some(12.5), Some(3000.0), 4096, 700_000_000,
            Some(12.5), Some(3000.0), 4096, 700_000_000,
            vec!["model-b"],
        );
        assert_eq!(rec.comparison_hash.len(), 64);
    }

    // C3-R13: NoChange for single advantage (not enough for supersession)
    #[test]
    fn test_roster_no_change_single_advantage() {
        let rec = run_comparison_and_evaluate(
            "model-a", "model-b", "classifier",
            Some(15.0), Some(3000.0), 4096, 700_000_000,
            Some(12.0), Some(3000.0), 4096, 700_000_000,
            vec!["model-b"],
        );
        // Only quality advantage — not enough for supersession (needs 2+ advantages)
        assert_eq!(rec.position, RosterPosition::NoChange);
    }

    // C3-R14: RosterPosition string round-trip
    #[test]
    fn test_roster_position_string_roundtrip() {
        let positions = vec![
            RosterPosition::Add,
            RosterPosition::NoChange,
            RosterPosition::Supersede,
            RosterPosition::InsufficientEvidence,
        ];
        for pos in &positions {
            let s = pos.as_str();
            assert!(!s.is_empty());
            assert_eq!(RosterPosition::from_str(s), Some(pos.clone()));
        }
        assert_eq!(RosterPosition::from_str("unknown"), None);
    }

    // C3-R15: Serialization round-trip
    #[test]
    fn test_recommendation_serialization_roundtrip() {
        let rec = run_comparison_and_evaluate(
            "model-a", "model-b", "classifier",
            Some(12.5), Some(3000.0), 4096, 700_000_000,
            Some(12.5), Some(3000.0), 4096, 700_000_000,
            vec!["model-b"],
        );
        let json = serde_json::to_string(&rec).unwrap();
        let parsed: RosterRecommendation = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.position, parsed.position);
        assert_eq!(rec.candidate_model_id, parsed.candidate_model_id);
        assert_eq!(rec.role, parsed.role);
    }
}
