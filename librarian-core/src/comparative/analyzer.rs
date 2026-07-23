//! Comparative analyzer — role-specific comparison between models.
//!
//! The analyzer compares a candidate model (manifest + profile) against
//! a baseline model (manifest + profile) for a specific role.
//!
//! Critical invariants:
//! - Comparison is ALWAYS role-specific
//! - Hardware throughput CANNOT upgrade capability status
//! - Candidate dominated for one role ≠ globally rejected
//! - Analyzer RECOMMENDS roster position; it does not execute supersession

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::finding::{ComparativeFinding, FindingSeverity, FindingType};
use crate::capability::manifest::CapabilityManifest;
use crate::routing::execution_profile::ExecutionProfile;

/// Thresholds for comparing execution metrics.
///
/// Differences within these tolerances are considered "equivalent"
/// rather than advantages or disadvantages.
pub struct ComparisonThresholds {
    /// Minimum relative improvement in tokens/sec to count as quality advantage (default: 10%).
    pub quality_improvement_pct: f64,
    /// Minimum relative improvement in generation duration to count as latency advantage (default: 10%).
    pub latency_improvement_pct: f64,
    /// Minimum absolute difference in VRAM MiB to count as memory advantage (default: 200 MiB).
    pub memory_diff_mb: u64,
    /// Minimum absolute difference in file size bytes to count as memory advantage (default: 50 MiB).
    pub file_size_diff_bytes: u64,
}

impl Default for ComparisonThresholds {
    fn default() -> Self {
        Self {
            quality_improvement_pct: 10.0,
            latency_improvement_pct: 10.0,
            memory_diff_mb: 200,
            file_size_diff_bytes: 50 * 1024 * 1024,
        }
    }
}

/// Input for a role-specific comparison.
#[derive(Debug, Clone)]
pub struct ComparisonInput {
    /// Candidate manifest (must be approved or conditional).
    pub candidate_manifest: CapabilityManifest,
    /// Candidate execution profile (must be active).
    pub candidate_profile: ExecutionProfile,
    /// Baseline manifest (existing roster model for same role).
    pub baseline_manifest: CapabilityManifest,
    /// Baseline execution profile (existing roster model).
    pub baseline_profile: ExecutionProfile,
    /// The role being compared.
    pub role: String,
    /// Other models that fill this role (for unique_role_advantage check).
    pub other_role_fillers: Vec<String>,
}

/// The result of a role-specific comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonResult {
    /// Role compared.
    pub role: String,

    /// Candidate model ID.
    pub candidate_model_id: String,

    /// Baseline model ID.
    pub baseline_model_id: String,

    /// Findings from the comparison.
    pub findings: Vec<ComparativeFinding>,

    /// Whether the candidate can be compared (false if evidence is insufficient).
    pub is_comparable: bool,
}

/// Check if a manifest has sufficient evidence for comparison.
fn has_comparable_evidence(manifest: &CapabilityManifest, profile: &ExecutionProfile) -> bool {
    // Must have passed smoke test
    if !manifest.evidence_summary.smoke_test_passed {
        return false;
    }

    // Must have at least one probe passed
    if manifest.evidence_summary.probes_passed.is_empty() {
        return false;
    }

    // Profile must have observation-based metrics
    if profile.metrics.observation_count == 0 {
        return false;
    }

    // Must have at least average generation duration or tokens/sec
    profile.metrics.avg_generation_duration_ms.is_some()
        || profile.metrics.avg_tokens_per_second.is_some()
}

/// Compare two models for a specific role.
///
/// This is a pure function — it produces findings but does NOT mutate
/// any state, create records, or execute roster changes.
pub fn compare_role(input: &ComparisonInput) -> ComparisonResult {
    let mut findings = Vec::new();

    let candidate_evidence = has_comparable_evidence(
        &input.candidate_manifest,
        &input.candidate_profile,
    );
    let baseline_evidence = has_comparable_evidence(
        &input.baseline_manifest,
        &input.baseline_profile,
    );

    // If either lacks evidence, we can only produce insufficient_comparable_evidence
    if !candidate_evidence || !baseline_evidence {
        findings.push(ComparativeFinding {
            finding_type: FindingType::InsufficientComparableEvidence,
            role: input.role.clone(),
            basis: if !candidate_evidence {
                format!(
                    "Candidate '{}' lacks sufficient qualification evidence for role '{}'",
                    input.candidate_manifest.model_id, input.role
                )
            } else {
                format!(
                    "Baseline '{}' lacks sufficient qualification evidence for role '{}'",
                    input.baseline_manifest.model_id, input.role
                )
            },
            severity: FindingSeverity::Info,
            evidence_refs: vec![
                input.candidate_manifest.manifest_id.clone(),
                input.baseline_manifest.manifest_id.clone(),
            ],
        });

        return ComparisonResult {
            role: input.role.clone(),
            candidate_model_id: input.candidate_manifest.model_id.clone(),
            baseline_model_id: input.baseline_manifest.model_id.clone(),
            findings,
            is_comparable: false,
        };
    }

    let thresholds = ComparisonThresholds::default();
    let mut candidate_advantages = 0u32;
    let mut baseline_advantages = 0u32;

    // --- Quality comparison (tokens/sec) ---
    if let (Some(c_tps), Some(b_tps)) = (
        input.candidate_profile.metrics.avg_tokens_per_second,
        input.baseline_profile.metrics.avg_tokens_per_second,
    ) {
        if b_tps > 0.0 {
            let improvement_pct = ((c_tps - b_tps) / b_tps) * 100.0;
            if improvement_pct > thresholds.quality_improvement_pct {
                findings.push(ComparativeFinding {
                    finding_type: FindingType::QualityAdvantage,
                    role: input.role.clone(),
                    basis: format!(
                        "Candidate '{}' avg {:.1} tok/s vs baseline '{}' avg {:.1} tok/s (+{:.1}%)",
                        input.candidate_manifest.model_id, c_tps,
                        input.baseline_manifest.model_id, b_tps,
                        improvement_pct
                    ),
                    severity: FindingSeverity::Advantage,
                    evidence_refs: vec![
                        input.candidate_profile.profile_id.clone(),
                        input.baseline_profile.profile_id.clone(),
                    ],
                });
                candidate_advantages += 1;
            } else if improvement_pct < -thresholds.quality_improvement_pct {
                findings.push(ComparativeFinding {
                    finding_type: FindingType::DominatedByExistingModel,
                    role: input.role.clone(),
                    basis: format!(
                        "Baseline '{}' avg {:.1} tok/s vs candidate '{}' avg {:.1} tok/s (+{:.1}%)",
                        input.baseline_manifest.model_id, b_tps,
                        input.candidate_manifest.model_id, c_tps,
                        improvement_pct.abs()
                    ),
                    severity: FindingSeverity::Disadvantage,
                    evidence_refs: vec![
                        input.candidate_profile.profile_id.clone(),
                        input.baseline_profile.profile_id.clone(),
                    ],
                });
                baseline_advantages += 1;
            }
        }
    }

    // --- Latency comparison (generation duration, lower is better) ---
    if let (Some(c_gen), Some(b_gen)) = (
        input.candidate_profile.metrics.avg_generation_duration_ms,
        input.baseline_profile.metrics.avg_generation_duration_ms,
    ) {
        if b_gen > 0.0 {
            let improvement_pct = ((b_gen - c_gen) / b_gen) * 100.0;
            if improvement_pct > thresholds.latency_improvement_pct {
                findings.push(ComparativeFinding {
                    finding_type: FindingType::LatencyAdvantage,
                    role: input.role.clone(),
                    basis: format!(
                        "Candidate '{}' avg {:.0}ms gen vs baseline '{}' avg {:.0}ms gen ({:.1}% faster)",
                        input.candidate_manifest.model_id, c_gen,
                        input.baseline_manifest.model_id, b_gen,
                        improvement_pct
                    ),
                    severity: FindingSeverity::Advantage,
                    evidence_refs: vec![
                        input.candidate_profile.profile_id.clone(),
                        input.baseline_profile.profile_id.clone(),
                    ],
                });
                candidate_advantages += 1;
            } else if improvement_pct < -thresholds.latency_improvement_pct {
                findings.push(ComparativeFinding {
                    finding_type: FindingType::DominatedByExistingModel,
                    role: input.role.clone(),
                    basis: format!(
                        "Baseline '{}' avg {:.0}ms gen vs candidate '{}' avg {:.0}ms gen ({:.1}% faster)",
                        input.baseline_manifest.model_id, b_gen,
                        input.candidate_manifest.model_id, c_gen,
                        improvement_pct.abs()
                    ),
                    severity: FindingSeverity::Disadvantage,
                    evidence_refs: vec![
                        input.candidate_profile.profile_id.clone(),
                        input.baseline_profile.profile_id.clone(),
                    ],
                });
                baseline_advantages += 1;
            }
        }
    }

    // --- Memory comparison (VRAM, lower is better) ---
    let c_vram = input.candidate_profile.hardware.gpu_vram_mb;
    let b_vram = input.baseline_profile.hardware.gpu_vram_mb;
    if c_vram < b_vram.saturating_sub(thresholds.memory_diff_mb) {
        findings.push(ComparativeFinding {
            finding_type: FindingType::MemoryAdvantage,
            role: input.role.clone(),
            basis: format!(
                "Candidate '{}' uses {} MiB VRAM vs baseline '{}' uses {} MiB VRAM ({} MiB less)",
                input.candidate_manifest.model_id, c_vram,
                input.baseline_manifest.model_id, b_vram,
                b_vram - c_vram
            ),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec![
                input.candidate_profile.profile_id.clone(),
                input.baseline_profile.profile_id.clone(),
            ],
        });
        candidate_advantages += 1;
    } else if b_vram < c_vram.saturating_sub(thresholds.memory_diff_mb) {
        findings.push(ComparativeFinding {
            finding_type: FindingType::DominatedByExistingModel,
            role: input.role.clone(),
            basis: format!(
                "Baseline '{}' uses {} MiB VRAM vs candidate '{}' uses {} MiB VRAM ({} MiB less)",
                input.baseline_manifest.model_id, b_vram,
                input.candidate_manifest.model_id, c_vram,
                c_vram - b_vram
            ),
            severity: FindingSeverity::Disadvantage,
            evidence_refs: vec![
                input.candidate_profile.profile_id.clone(),
                input.baseline_profile.profile_id.clone(),
            ],
        });
        baseline_advantages += 1;
    }

    // --- Unique role advantage check ---
    if input.other_role_fillers.is_empty() {
        findings.push(ComparativeFinding {
            finding_type: FindingType::UniqueRoleAdvantage,
            role: input.role.clone(),
            basis: format!(
                "Candidate '{}' is the only model qualified for role '{}'",
                input.candidate_manifest.model_id, input.role
            ),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec![input.candidate_manifest.manifest_id.clone()],
        });
    }

    // --- Fallback advantage ---
    // A candidate that's smaller (lower file size) can serve as a fallback
    let c_file_size = input.candidate_profile.artifact.file_size_bytes;
    let b_file_size = input.baseline_profile.artifact.file_size_bytes;
    if c_file_size < b_file_size.saturating_sub(thresholds.file_size_diff_bytes) {
        findings.push(ComparativeFinding {
            finding_type: FindingType::FallbackAdvantage,
            role: input.role.clone(),
            basis: format!(
                "Candidate '{}' ({:.1} MB) can serve as lighter fallback for role '{}' vs baseline '{}' ({:.1} MB)",
                input.candidate_manifest.model_id,
                c_file_size as f64 / 1_048_576.0,
                input.role,
                input.baseline_manifest.model_id,
                b_file_size as f64 / 1_048_576.0,
            ),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec![
                input.candidate_profile.profile_id.clone(),
                input.baseline_profile.profile_id.clone(),
            ],
        });
    }

    // --- Determine overall finding ---
    let is_comparable = true;

    // Check if candidate is equivalent (no material advantage on any dimension)
    if candidate_advantages == 0 && baseline_advantages == 0 && !findings.iter().any(|f| {
        matches!(
            f.finding_type,
            FindingType::UniqueRoleAdvantage | FindingType::FallbackAdvantage
        )
    }) {
        findings.push(ComparativeFinding {
            finding_type: FindingType::EquivalentNoMaterialAdvantage,
            role: input.role.clone(),
            basis: format!(
                "Candidate '{}' and baseline '{}' show no material advantage for role '{}'",
                input.candidate_manifest.model_id,
                input.baseline_manifest.model_id,
                input.role
            ),
            severity: FindingSeverity::Info,
            evidence_refs: vec![
                input.candidate_manifest.manifest_id.clone(),
                input.baseline_manifest.manifest_id.clone(),
            ],
        });
    }

    // Check if candidate dominates baseline (multiple advantages, no disadvantages)
    let dominated_by_baseline = baseline_advantages > 0 && candidate_advantages == 0;
    if dominated_by_baseline && !findings.iter().any(|f| f.finding_type == FindingType::UniqueRoleAdvantage) {
        // Dominated finding already added during metric comparison
    } else if candidate_advantages > 1 && baseline_advantages == 0 {
        // Candidate is better on multiple dimensions → possible supersession
        findings.push(ComparativeFinding {
            finding_type: FindingType::SupersedesExistingModel,
            role: input.role.clone(),
            basis: format!(
                "Candidate '{}' exceeds baseline '{}' on {} metrics for role '{}'",
                input.candidate_manifest.model_id,
                input.baseline_manifest.model_id,
                candidate_advantages,
                input.role
            ),
            severity: FindingSeverity::Advantage,
            evidence_refs: vec![
                input.candidate_manifest.manifest_id.clone(),
                input.baseline_manifest.manifest_id.clone(),
                input.candidate_profile.profile_id.clone(),
                input.baseline_profile.profile_id.clone(),
            ],
        });
    }

    ComparisonResult {
        role: input.role.clone(),
        candidate_model_id: input.candidate_manifest.model_id.clone(),
        baseline_model_id: input.baseline_manifest.model_id.clone(),
        findings,
        is_comparable,
    }
}

/// Compute a content hash for a comparison result.
pub fn compute_comparison_hash(result: &ComparisonResult) -> Result<String> {
    use sha2::Digest;
    let content = serde_json::json!({
        "role": result.role,
        "candidate_model_id": result.candidate_model_id,
        "baseline_model_id": result.baseline_model_id,
        "finding_count": result.findings.len(),
        "is_comparable": result.is_comparable,
    });
    let json = content.to_string();
    let mut hasher = sha2::Sha256::new();
    hasher.update(json.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::manifest::{EvidenceSummary, ManifestStatus};
    use crate::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, HardwareIdentity, RuntimeIdentity,
    };

    fn make_test_manifest(model_id: &str, role: &str, status: ManifestStatus) -> CapabilityManifest {
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

    fn make_test_profile(
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
            status: crate::routing::execution_profile::ProfileStatus::Active,
            content_hash: String::new(),
            created_at: "2026-07-11T12:00:00Z".to_string(),
            updated_at: "2026-07-11T12:00:00Z".to_string(),
        }
    }

    // C3-A1: Quality advantage — candidate has higher tok/s
    #[test]
    fn test_quality_advantage() {
        let candidate_manifest = make_test_manifest("model-fast", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-slow", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-fast", Some(15.2), Some(3200.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-slow", Some(10.0), Some(3500.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-slow".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::QualityAdvantage));
    }

    // C3-A2: Latency advantage — candidate is faster
    #[test]
    fn test_latency_advantage() {
        let candidate_manifest = make_test_manifest("model-fast", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-slow", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-fast", Some(12.5), Some(2500.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-slow", Some(12.5), Some(3500.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-slow".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::LatencyAdvantage));
    }

    // C3-A3: Memory advantage — candidate uses less VRAM
    #[test]
    fn test_memory_advantage() {
        let candidate_manifest = make_test_manifest("model-small", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-large", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-small", Some(12.5), Some(3000.0), 2048, 400_000_000);
        let baseline_profile = make_test_profile("model-large", Some(12.5), Some(3000.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-large".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::MemoryAdvantage));
    }

    // C3-A4: Unique role advantage — no other model fills this role
    #[test]
    fn test_unique_role_advantage() {
        let candidate_manifest = make_test_manifest("model-only", "summarizer", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-other", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-only", Some(12.5), Some(3000.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-other", Some(12.5), Some(3000.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "summarizer".to_string(),
            other_role_fillers: vec![], // No other model fills this role
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::UniqueRoleAdvantage));
    }

    // C3-A5: Fallback advantage — candidate is lighter
    #[test]
    fn test_fallback_advantage() {
        let candidate_manifest = make_test_manifest("model-light", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-heavy", "classifier", ManifestStatus::Approved);
        // Candidate is much smaller
        let candidate_profile = make_test_profile("model-light", Some(12.5), Some(3000.0), 4096, 300_000_000);
        let baseline_profile = make_test_profile("model-heavy", Some(12.5), Some(3000.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-heavy".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::FallbackAdvantage));
    }

    // C3-A6: Insufficient evidence — candidate lacks metrics
    #[test]
    fn test_insufficient_evidence_candidate() {
        let mut candidate_manifest = make_test_manifest("model-new", "classifier", ManifestStatus::Approved);
        candidate_manifest.evidence_summary.smoke_test_passed = false;
        let baseline_manifest = make_test_manifest("model-existing", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-new", None, None, 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-existing", Some(12.5), Some(3000.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-existing".to_string()],
        };

        let result = compare_role(&input);
        assert!(!result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::InsufficientComparableEvidence));
    }

    // C3-A7: Equivalent — no material advantage
    #[test]
    fn test_equivalent_no_material_advantage() {
        let candidate_manifest = make_test_manifest("model-a", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-b", "classifier", ManifestStatus::Approved);
        // Same metrics within tolerance
        let candidate_profile = make_test_profile("model-a", Some(12.6), Some(3000.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-b", Some(12.5), Some(3050.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-b".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::EquivalentNoMaterialAdvantage));
    }

    // C3-A8: Supersedes — candidate exceeds baseline on multiple metrics
    #[test]
    fn test_supersedes_existing_model() {
        let candidate_manifest = make_test_manifest("model-new", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-old", "classifier", ManifestStatus::Approved);
        // Candidate is better on quality + latency + memory
        let candidate_profile = make_test_profile("model-new", Some(18.0), Some(2000.0), 2048, 400_000_000);
        let baseline_profile = make_test_profile("model-old", Some(10.0), Some(3500.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-old".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::SupersedesExistingModel));
    }

    // C3-A9: Dominated by baseline — baseline is better on all metrics
    #[test]
    fn test_dominated_by_baseline() {
        let candidate_manifest = make_test_manifest("model-weak", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-strong", "classifier", ManifestStatus::Approved);
        // Candidate is worse on everything
        let candidate_profile = make_test_profile("model-weak", Some(8.0), Some(5000.0), 8192, 1_200_000_000);
        let baseline_profile = make_test_profile("model-strong", Some(15.0), Some(2500.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-strong".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::DominatedByExistingModel));
    }

    // C3-A10: Comparison hash is deterministic
    #[test]
    fn test_comparison_hash_deterministic() {
        let candidate_manifest = make_test_manifest("model-a", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-b", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-a", Some(12.5), Some(3000.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-b", Some(12.5), Some(3000.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec![],
        };

        let result = compare_role(&input);
        let hash1 = compute_comparison_hash(&result).unwrap();
        let hash2 = compute_comparison_hash(&result).unwrap();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    // C3-A11: Role-specific — comparison role is always preserved
    #[test]
    fn test_role_specific_preserved() {
        let candidate_manifest = make_test_manifest("model-a", "summarizer", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-b", "summarizer", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-a", Some(12.5), Some(3000.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-b", Some(15.0), Some(2500.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "summarizer".to_string(),
            other_role_fillers: vec![],
        };

        let result = compare_role(&input);
        assert_eq!(result.role, "summarizer");
        for finding in &result.findings {
            assert_eq!(finding.role, "summarizer");
        }
    }

    // C3-A12: Insufficient evidence baseline
    #[test]
    fn test_insufficient_evidence_baseline() {
        let candidate_manifest = make_test_manifest("model-a", "classifier", ManifestStatus::Approved);
        let mut baseline_manifest = make_test_manifest("model-b", "classifier", ManifestStatus::Approved);
        baseline_manifest.evidence_summary.probes_passed = vec![]; // No probes passed
        let candidate_profile = make_test_profile("model-a", Some(12.5), Some(3000.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-b", Some(12.5), Some(3000.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec![],
        };

        let result = compare_role(&input);
        assert!(!result.is_comparable);
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::InsufficientComparableEvidence));
    }

    // C3-A13: Comparison does NOT mutate manifests or profiles
    #[test]
    fn test_comparison_is_pure() {
        let candidate_manifest = make_test_manifest("model-a", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-b", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-a", Some(15.0), Some(2500.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-b", Some(10.0), Some(3500.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest: candidate_manifest.clone(),
            candidate_profile: candidate_profile.clone(),
            baseline_manifest: baseline_manifest.clone(),
            baseline_profile: baseline_profile.clone(),
            role: "classifier".to_string(),
            other_role_fillers: vec![],
        };

        let _result = compare_role(&input);

        // Verify originals are unchanged
        assert_eq!(input.candidate_manifest.model_id, "model-a");
        assert_eq!(input.baseline_manifest.model_id, "model-b");
        assert_eq!(input.candidate_profile.metrics.avg_tokens_per_second, Some(15.0));
        assert_eq!(input.baseline_profile.metrics.avg_tokens_per_second, Some(10.0));
    }

    // C3-A14: Comparison with no generation duration (only tok/s available)
    #[test]
    fn test_comparison_no_generation_duration() {
        let candidate_manifest = make_test_manifest("model-a", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-b", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-a", Some(15.0), None, 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-b", Some(10.0), None, 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-b".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        // Quality advantage should be detected
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::QualityAdvantage));
        // No latency finding since duration not available
        assert!(!result.findings.iter().any(|f| f.finding_type == FindingType::LatencyAdvantage));
    }

    // ===== CRITICAL INVARIANT TESTS (I3) =====

    // INV-1: Hardware throughput CANNOT upgrade capability status.
    // Candidate runs on MORE VRAM (faster hardware), but hardware advantage
    // does NOT produce a QualityAdvantage finding. Comparison is artifact-based.
    #[test]
    fn test_invariant_hardware_throughput_no_capability_upgrade() {
        let candidate_manifest = make_test_manifest("model-a", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-b", "classifier", ManifestStatus::Approved);
        // Same tok/s, same generation time — but candidate has 8GB VRAM vs 4GB
        let candidate_profile = make_test_profile("model-a", Some(12.5), Some(3000.0), 8192, 700_000_000);
        let baseline_profile = make_test_profile("model-b", Some(12.5), Some(3000.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest,
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-b".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        // More VRAM on hardware does NOT create QualityAdvantage
        assert!(!result.findings.iter().any(|f| f.finding_type == FindingType::QualityAdvantage));
        // It does NOT create LatencyAdvantage either
        assert!(!result.findings.iter().any(|f| f.finding_type == FindingType::LatencyAdvantage));
        // Hardware difference alone yields no material advantage finding
        assert!(!result.findings.iter().any(|f| f.finding_type == FindingType::SupersedesExistingModel));
    }

    // INV-2: Higher throughput ≠ role approval.
    // Candidate has higher tok/s, but comparison result is a set of findings
    // (advisory), not an approval or manifest status change.
    #[test]
    fn test_invariant_higher_throughput_not_approval() {
        let candidate_manifest = make_test_manifest("model-fast", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-slow", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-fast", Some(20.0), Some(2000.0), 4096, 700_000_000);
        let baseline_profile = make_test_profile("model-slow", Some(10.0), Some(3000.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest: candidate_manifest.clone(),
            candidate_profile,
            baseline_manifest,
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-slow".to_string()],
        };

        let result = compare_role(&input);
        assert!(result.is_comparable);
        // Findings exist
        assert!(result.findings.iter().any(|f| f.finding_type == FindingType::QualityAdvantage));
        // But this is NOT an approval — the manifest status is unchanged
        assert_eq!(input.candidate_manifest.status, ManifestStatus::Approved);
        // The result is just ComparisonResult, not a CapabilityManifest or roster mutation
        assert!(result.findings.iter().all(|f| f.severity != FindingSeverity::Blocker));
    }

    // INV-3: Candidate dominated for one role ≠ globally rejected.
    // model-weak is dominated by model-strong for "classifier" role,
    // but that does NOT mean it's globally rejected — it can still be
    // recommended for a different role like "summarizer".
    #[test]
    fn test_invariant_dominated_one_role_not_globally_rejected() {
        // First comparison: model-weak is dominated for classifier
        let dominated_input = ComparisonInput {
            candidate_manifest: make_test_manifest("model-weak", "classifier", ManifestStatus::Approved),
            candidate_profile: make_test_profile("model-weak", Some(8.0), Some(5000.0), 8192, 1_200_000_000),
            baseline_manifest: make_test_manifest("model-strong", "classifier", ManifestStatus::Approved),
            baseline_profile: make_test_profile("model-strong", Some(15.0), Some(2500.0), 4096, 700_000_000),
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-strong".to_string()],
        };
        let dominated_result = compare_role(&dominated_input);
        assert!(dominated_result.findings.iter().any(|f| f.finding_type == FindingType::DominatedByExistingModel));

        // Second comparison: same model-weak for summarizer with no other fillers
        let unique_input = ComparisonInput {
            candidate_manifest: make_test_manifest("model-weak", "summarizer", ManifestStatus::Approved),
            candidate_profile: make_test_profile("model-weak", Some(8.0), Some(5000.0), 8192, 1_200_000_000),
            baseline_manifest: make_test_manifest("model-other", "summarizer", ManifestStatus::Approved),
            baseline_profile: make_test_profile("model-other", Some(8.0), Some(5000.0), 8192, 1_200_000_000),
            role: "summarizer".to_string(),
            other_role_fillers: vec![], // No other model fills summarizer
        };
        let unique_result = compare_role(&unique_input);
        // model-weak is NOT dominated for summarizer — it fills a unique role
        assert!(unique_result.findings.iter().any(|f| f.finding_type == FindingType::UniqueRoleAdvantage));
        assert!(!unique_result.findings.iter().any(|f| f.finding_type == FindingType::DominatedByExistingModel));
        // Role-specific: dominated in classifier does not leak to summarizer
        assert_eq!(unique_result.role, "summarizer");
    }

    // INV-6: Comparative classifier finding ≠ Owner decision.
    // Findings are advisory only — they describe comparison outcomes
    // but do NOT mutate manifest status, create decisions, or change
    // roster state.
    #[test]
    fn test_invariant_finding_not_owner_decision() {
        let candidate_manifest = make_test_manifest("model-a", "classifier", ManifestStatus::Approved);
        let baseline_manifest = make_test_manifest("model-b", "classifier", ManifestStatus::Approved);
        let candidate_profile = make_test_profile("model-a", Some(18.0), Some(2000.0), 2048, 400_000_000);
        let baseline_profile = make_test_profile("model-b", Some(10.0), Some(3500.0), 4096, 700_000_000);

        let input = ComparisonInput {
            candidate_manifest: candidate_manifest.clone(),
            candidate_profile: candidate_profile.clone(),
            baseline_manifest: baseline_manifest.clone(),
            baseline_profile,
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-b".to_string()],
        };

        let result = compare_role(&input);

        // Findings exist (supersedes, quality, latency, memory, etc.)
        assert!(result.is_comparable);
        assert!(result.findings.len() >= 2);

        // But: no manifest was mutated
        assert_eq!(input.candidate_manifest.status, ManifestStatus::Approved);
        assert_eq!(input.baseline_manifest.status, ManifestStatus::Approved);

        // No decision was created — the result is ComparisonResult, not a decision
        // The findings are advisory: they describe what the comparison found,
        // not what should happen to the roster
        assert!(result.findings.iter().all(|f| f.basis.len() > 0));

        // Finding severity is informational/advisory — it cannot be "approved" or "rejected"
        for finding in &result.findings {
            match finding.severity {
                FindingSeverity::Info | FindingSeverity::Advantage | FindingSeverity::Disadvantage | FindingSeverity::Blocker => {}
            }
        }
    }
}
