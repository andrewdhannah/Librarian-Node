//! Comparative audit record — durable audit trail for role-specific comparisons.
//!
//! Each record captures the full context of a comparison: candidate and baseline
//! identity, methodology, thresholds, findings, and evidence references.
//!
//! Critical invariant:
//!   Historical comparison records survive restart as advisory evidence.
//!   Persistence does NOT create routing authority, auto-mutate roster state,
//!   or trigger supersession. Only Owner decisions carry authority.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::analyzer::{ComparisonResult, ComparisonThresholds};
use super::finding::ComparativeFinding;
use super::roster::{RosterPosition, RosterRecommendation};

/// Analyzer version — tracks which comparison logic produced this record.
pub const ANALYZER_VERSION: &str = "1.0.0";

/// Comparison methodology description.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonMethodology {
    /// Role-specific metric comparison against baseline.
    RoleMetricComparison,
    /// Insufficient evidence for metric comparison.
    InsufficientEvidence,
    /// Manual or override comparison (future use).
    ManualOverride,
}

impl ComparisonMethodology {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RoleMetricComparison => "role_metric_comparison",
            Self::InsufficientEvidence => "insufficient_evidence",
            Self::ManualOverride => "manual_override",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "role_metric_comparison" => Some(Self::RoleMetricComparison),
            "insufficient_evidence" => Some(Self::InsufficientEvidence),
            "manual_override" => Some(Self::ManualOverride),
            _ => None,
        }
    }
}

/// Artifact identity — lightweight reference to a model file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactReference {
    /// Model ID.
    pub model_id: String,
    /// SHA-256 hash of the model file.
    pub sha256: String,
    /// Model filename.
    pub filename: String,
    /// Model role.
    pub role: String,
}

/// Comparison thresholds snapshot — captures the thresholds used for this comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThresholdSnapshot {
    /// Quality improvement percentage threshold.
    pub quality_improvement_pct: f64,
    /// Latency improvement percentage threshold.
    pub latency_improvement_pct: f64,
    /// Memory difference MiB threshold.
    pub memory_diff_mb: u64,
    /// File size difference bytes threshold.
    pub file_size_diff_bytes: u64,
}

impl ThresholdSnapshot {
    /// Create from ComparisonThresholds.
    pub fn from_thresholds(t: &ComparisonThresholds) -> Self {
        Self {
            quality_improvement_pct: t.quality_improvement_pct,
            latency_improvement_pct: t.latency_improvement_pct,
            memory_diff_mb: t.memory_diff_mb,
            file_size_diff_bytes: t.file_size_diff_bytes,
        }
    }

    /// Default thresholds.
    pub fn default_thresholds() -> Self {
        Self::from_thresholds(&ComparisonThresholds::default())
    }
}

/// Comparison audit record — durable audit trail for a single comparison.
///
/// This record captures the full comparison context. It is advisory only:
/// persistence does not create routing authority or trigger roster mutations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonAuditRecord {
    /// Unique audit record ID (deterministic from content).
    pub audit_id: String,

    /// Candidate artifact identity.
    pub candidate: ArtifactReference,

    /// Baseline artifact identity.
    pub baseline: ArtifactReference,

    /// Role compared.
    pub role: String,

    /// Comparison methodology used.
    pub methodology: ComparisonMethodology,

    /// Analyzer version that produced this comparison.
    pub analyzer_version: String,

    /// Thresholds snapshot used for this comparison.
    pub thresholds: ThresholdSnapshot,

    /// Findings from the comparison.
    pub findings: Vec<ComparativeFinding>,

    /// Whether the comparison was comparable (sufficient evidence).
    pub is_comparable: bool,

    /// Roster position recommendation (advisory, not authoritative).
    pub recommended_position: RosterPosition,

    /// Reference to the comparison hash from the analyzer.
    pub comparison_hash: String,

    /// Content hash for tamper detection.
    pub content_hash: String,

    /// When this audit record was created.
    pub created_at: String,
}

impl ComparisonAuditRecord {
    /// Compute a deterministic audit ID from the comparison content.
    pub fn compute_audit_id(
        candidate_model_id: &str,
        baseline_model_id: &str,
        role: &str,
        created_at: &str,
    ) -> String {
        let input = format!("{}:{}:{}:{}", candidate_model_id, baseline_model_id, role, created_at);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute a content hash for tamper detection.
    pub fn compute_content_hash(&self) -> Result<String> {
        let content = serde_json::json!({
            "candidate_model_id": self.candidate.model_id,
            "candidate_sha256": self.candidate.sha256,
            "baseline_model_id": self.baseline.model_id,
            "baseline_sha256": self.baseline.sha256,
            "role": self.role,
            "methodology": self.methodology.as_str(),
            "analyzer_version": self.analyzer_version,
            "thresholds": {
                "quality_improvement_pct": self.thresholds.quality_improvement_pct,
                "latency_improvement_pct": self.thresholds.latency_improvement_pct,
                "memory_diff_mb": self.thresholds.memory_diff_mb,
                "file_size_diff_bytes": self.thresholds.file_size_diff_bytes,
            },
            "finding_count": self.findings.len(),
            "is_comparable": self.is_comparable,
            "recommended_position": self.recommended_position.as_str(),
            "comparison_hash": self.comparison_hash,
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Create an audit record from a comparison result and roster recommendation.
    pub fn from_comparison(
        result: &ComparisonResult,
        recommendation: &RosterRecommendation,
        candidate_manifest: &super::super::capability::manifest::CapabilityManifest,
        baseline_manifest: &super::super::capability::manifest::CapabilityManifest,
    ) -> Result<Self> {
        let now = chrono::Utc::now().to_rfc3339();

        let candidate = ArtifactReference {
            model_id: candidate_manifest.model_id.clone(),
            sha256: candidate_manifest.model_sha256.clone(),
            filename: candidate_manifest.model_filename.clone(),
            role: candidate_manifest.role.clone(),
        };

        let baseline = ArtifactReference {
            model_id: baseline_manifest.model_id.clone(),
            sha256: baseline_manifest.model_sha256.clone(),
            filename: baseline_manifest.model_filename.clone(),
            role: baseline_manifest.role.clone(),
        };

        let methodology = if result.is_comparable {
            ComparisonMethodology::RoleMetricComparison
        } else {
            ComparisonMethodology::InsufficientEvidence
        };

        let audit_id = Self::compute_audit_id(
            &candidate.model_id,
            &baseline.model_id,
            &result.role,
            &now,
        );

        let thresholds = ThresholdSnapshot::default_thresholds();

        let mut record = Self {
            audit_id,
            candidate,
            baseline,
            role: result.role.clone(),
            methodology,
            analyzer_version: ANALYZER_VERSION.to_string(),
            thresholds,
            findings: result.findings.clone(),
            is_comparable: result.is_comparable,
            recommended_position: recommendation.position,
            comparison_hash: recommendation.comparison_hash.clone(),
            content_hash: String::new(),
            created_at: now,
        };

        record.content_hash = record.compute_content_hash()?;
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::manifest::{CapabilityManifest, EvidenceSummary, ManifestStatus};
    use crate::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity, ProfileStatus, RuntimeIdentity,
    };

    fn make_manifest(model_id: &str, role: &str) -> CapabilityManifest {
        let created_at = "2026-07-11T12:00:00Z".to_string();
        let manifest_id = CapabilityManifest::compute_manifest_id(model_id, role, &created_at);
        CapabilityManifest {
            manifest_id,
            model_id: model_id.to_string(),
            model_sha256: "abc123def456".to_string(),
            model_filename: format!("{}-model.gguf", model_id),
            role: role.to_string(),
            status: ManifestStatus::Approved,
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

    fn make_profile(model_id: &str) -> ExecutionProfile {
        ExecutionProfile {
            profile_id: ExecutionProfile::compute_profile_id(model_id, "c85e97a", "Radeon RX 570"),
            artifact: ArtifactIdentity {
                filename: format!("{}-model.gguf", model_id),
                model_id: model_id.to_string(),
                quantization: "Q4_K_M".to_string(),
                sha256: "abc123def456".to_string(),
                file_size_bytes: 700_000_000,
            },
            runtime: RuntimeIdentity {
                executable: "llama-server.exe".to_string(),
                version: "c85e97a".to_string(),
                backend: "vulkan".to_string(),
                device_id: Some("Vulkan0".to_string()),
            },
            hardware: HardwareIdentity {
                gpu_description: "Radeon RX 570".to_string(),
                gpu_vram_mb: 4096,
                cpu: "Intel Core i7-7700K".to_string(),
                ram_mb: 16384,
                os: "windows".to_string(),
            },
            metrics: ExecutionMetrics {
                avg_load_duration_ms: Some(2000.0),
                avg_generation_duration_ms: Some(3000.0),
                avg_tokens_per_second: Some(12.5),
                peak_vram_usage_mb: Some(3500),
                observation_count: 5,
            },
            status: ProfileStatus::Active,
            content_hash: String::new(),
            created_at: "2026-07-11T12:00:00Z".to_string(),
            updated_at: "2026-07-11T12:00:00Z".to_string(),
        }
    }

    // H3-U1: Audit ID is deterministic
    #[test]
    fn test_audit_id_deterministic() {
        let id1 = ComparisonAuditRecord::compute_audit_id("model-a", "model-b", "classifier", "2026-01-01");
        let id2 = ComparisonAuditRecord::compute_audit_id("model-a", "model-b", "classifier", "2026-01-01");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    // H3-U2: Audit ID changes with different inputs
    #[test]
    fn test_audit_id_changes_with_model() {
        let id1 = ComparisonAuditRecord::compute_audit_id("model-a", "model-b", "classifier", "2026-01-01");
        let id2 = ComparisonAuditRecord::compute_audit_id("model-x", "model-b", "classifier", "2026-01-01");
        assert_ne!(id1, id2);
    }

    // H3-U3: Content hash is deterministic
    #[test]
    fn test_content_hash_deterministic() {
        let record = make_full_audit_record();
        let hash1 = record.compute_content_hash().unwrap();
        let hash2 = record.compute_content_hash().unwrap();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    // H3-U4: Content hash changes with tampered data
    #[test]
    fn test_content_hash_tamper_detection() {
        let mut record = make_full_audit_record();
        let original_hash = record.compute_content_hash().unwrap();

        // Tamper with finding count
        record.findings.clear();
        let tampered_hash = record.compute_content_hash().unwrap();
        assert_ne!(original_hash, tampered_hash);
    }

    // H3-U5: ComparisonMethodology string round-trip
    #[test]
    fn test_methodology_string_roundtrip() {
        let methods = vec![
            ComparisonMethodology::RoleMetricComparison,
            ComparisonMethodology::InsufficientEvidence,
            ComparisonMethodology::ManualOverride,
        ];
        for m in &methods {
            let s = m.as_str();
            assert!(!s.is_empty());
            assert_eq!(ComparisonMethodology::from_str(s), Some(m.clone()));
        }
        assert_eq!(ComparisonMethodology::from_str("unknown"), None);
    }

    // H3-U6: ThresholdSnapshot preserves values
    #[test]
    fn test_threshold_snapshot_preserves_values() {
        let thresholds = ThresholdSnapshot {
            quality_improvement_pct: 15.0,
            latency_improvement_pct: 20.0,
            memory_diff_mb: 300,
            file_size_diff_bytes: 100 * 1024 * 1024,
        };
        assert_eq!(thresholds.quality_improvement_pct, 15.0);
        assert_eq!(thresholds.latency_improvement_pct, 20.0);
        assert_eq!(thresholds.memory_diff_mb, 300);
        assert_eq!(thresholds.file_size_diff_bytes, 100 * 1024 * 1024);
    }

    // H3-U7: Serialization round-trip
    #[test]
    fn test_serialization_roundtrip() {
        let record = make_full_audit_record();
        let json = serde_json::to_string(&record).unwrap();
        let parsed: ComparisonAuditRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.audit_id, parsed.audit_id);
        assert_eq!(record.candidate, parsed.candidate);
        assert_eq!(record.baseline, parsed.baseline);
        assert_eq!(record.role, parsed.role);
        assert_eq!(record.findings, parsed.findings);
        assert_eq!(record.content_hash, parsed.content_hash);
    }

    // H3-U8: ANALYZER_VERSION is set
    #[test]
    fn test_analyzer_version_set() {
        assert!(!ANALYZER_VERSION.is_empty());
        assert_eq!(ANALYZER_VERSION, "1.0.0");
    }

    // H3-U9: ArtifactReference equality
    #[test]
    fn test_artifact_reference_equality() {
        let a = ArtifactReference {
            model_id: "model-a".to_string(),
            sha256: "abc123".to_string(),
            filename: "model-a.gguf".to_string(),
            role: "classifier".to_string(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    fn make_full_audit_record() -> ComparisonAuditRecord {
        let candidate_manifest = make_manifest("model-fast", "classifier");
        let baseline_manifest = make_manifest("model-slow", "classifier");
        let candidate_profile = make_profile("model-fast");
        let baseline_profile = make_profile("model-slow");

        use crate::comparative::analyzer::{compare_role, ComparisonInput};
        use crate::comparative::roster::evaluate_roster;

        let input = ComparisonInput {
            candidate_manifest: candidate_manifest.clone(),
            candidate_profile: candidate_profile.clone(),
            baseline_manifest: baseline_manifest.clone(),
            baseline_profile: baseline_profile.clone(),
            role: "classifier".to_string(),
            other_role_fillers: vec!["model-slow".to_string()],
        };

        let result = compare_role(&input);
        let recommendation = evaluate_roster(&result).unwrap();

        ComparisonAuditRecord::from_comparison(
            &result,
            &recommendation,
            &candidate_manifest,
            &baseline_manifest,
        ).unwrap()
    }
}
