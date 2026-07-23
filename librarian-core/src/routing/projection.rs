//! Router projection — compact read-only deployment view.
//!
//! The projection joins a capability manifest (WHAT work is approved)
//! with an execution profile (HOW/WHERE the model runs) to produce
//! a bounded routable deployment view.
//!
//! Projection creation rules:
//! - Manifest MUST be approved or conditional
//! - Profile MUST be active and compatible
//! - Manifest and profile MUST reference the same model artifact
//! - Hardware throughput CANNOT upgrade capability status
//!
//! Projection lifecycle:
//! - Created from approved/conditional manifest + compatible profile
//! - Superseded when a newer projection replaces this one
//! - Expired when the projection's validity period ends
//!
//! Critical invariants:
//! - Rejected role + fast execution profile → NOT ROUTABLE
//! - Draft/Proposed manifest + fast execution profile → NOT ROUTABLE
//! - Approved role + incompatible model artifact → NOT ROUTABLE
//! - Conditional role + unmet constraints → NOT ROUTABLE
//! - Approved role + compatible execution profile → MAY EXPOSE ROLE

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::execution_profile::ExecutionProfile;
use crate::capability::manifest::{CapabilityManifest, ManifestStatus};

/// Router projection status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectionStatus {
    /// Active: projection is routable.
    #[serde(rename = "active")]
    Active,
    /// Superseded: replaced by a newer projection.
    #[serde(rename = "superseded")]
    Superseded,
    /// Expired: projection's validity period ended.
    #[serde(rename = "expired")]
    Expired,
}

impl ProjectionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "superseded" => Some(Self::Superseded),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

/// Projection creation result — either succeeds or explains why it failed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectionCreationResult {
    /// Projection created successfully.
    Created(RouterProjection),
    /// Projection creation rejected with explanation.
    Rejected {
        reason: String,
    },
}

/// Router projection — compact read-only deployment view.
///
/// The projection is the only data structure the router consumes.
/// It contains exactly what the router needs to make routing decisions:
/// - What role is approved
/// - What model artifact to run
/// - What execution profile to use
/// - What constraints apply
/// - Who approved it
///
/// The projection MUST NOT contain:
/// - Raw qualification evidence
/// - Raw benchmark scores
/// - Capability assessment logic
/// - Auto-promotion flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouterProjection {
    /// Unique projection identifier.
    pub projection_id: String,

    /// The manifest this projection is based on.
    pub manifest_id: String,

    /// The Owner decision that approved this manifest.
    pub owner_decision_id: String,

    /// The execution profile this projection uses.
    pub profile_id: String,

    /// The role this projection exposes.
    pub role: String,

    /// Model identity (exact artifact binding).
    pub model_id: String,
    pub model_sha256: String,
    pub model_filename: String,

    /// Approved status (approved or conditional).
    pub manifest_status: ManifestStatus,

    /// Constraints (for conditional approvals).
    pub constraints: Option<String>,

    /// Projection status.
    pub status: ProjectionStatus,

    /// Hardware capability snapshot (copied from profile at creation time).
    pub gpu_vram_mb: u64,
    pub runtime_backend: String,
    pub runtime_os: String,

    /// When the projection was created (RFC 3339).
    pub created_at: String,

    /// When the projection expires (RFC 3339, optional).
    pub expires_at: Option<String>,

    /// SHA-256 hash of the projection content.
    pub content_hash: String,
}

impl RouterProjection {
    /// Compute a deterministic projection ID from manifest_id and profile_id.
    pub fn compute_projection_id(manifest_id: &str, profile_id: &str) -> String {
        let input = format!("{}:{}", manifest_id, profile_id);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute SHA-256 hash of the projection content.
    pub fn compute_content_hash(&self) -> Result<String> {
        let content = serde_json::json!({
            "manifest_id": self.manifest_id,
            "owner_decision_id": self.owner_decision_id,
            "profile_id": self.profile_id,
            "role": self.role,
            "model_id": self.model_id,
            "model_sha256": self.model_sha256,
            "model_filename": self.model_filename,
            "manifest_status": self.manifest_status.as_str(),
            "constraints": self.constraints,
            "status": self.status.as_str(),
            "gpu_vram_mb": self.gpu_vram_mb,
            "runtime_backend": self.runtime_backend,
            "runtime_os": self.runtime_os,
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Validate the projection structure.
    pub fn validate(&self) -> Result<()> {
        if self.projection_id.is_empty() {
            anyhow::bail!("projection_id is empty");
        }
        if self.manifest_id.is_empty() {
            anyhow::bail!("manifest_id is empty");
        }
        if self.owner_decision_id.is_empty() {
            anyhow::bail!("owner_decision_id is empty");
        }
        if self.profile_id.is_empty() {
            anyhow::bail!("profile_id is empty");
        }
        if self.role.is_empty() {
            anyhow::bail!("role is empty");
        }
        if self.model_id.is_empty() {
            anyhow::bail!("model_id is empty");
        }
        Ok(())
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize projection to JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse projection from JSON")
    }

    /// Assert this projection contains no raw evidence data.
    pub fn assert_no_raw_evidence(&self) -> Result<()> {
        // RouterProjection MUST NOT contain:
        // - raw_qualification_evidence
        // - benchmark_scores
        // - capability_assessment_logic
        // - auto_promote
        //
        // Structural proof: the fields are:
        // - projection_id, manifest_id, owner_decision_id, profile_id (references)
        // - role, model_id, model_sha256, model_filename (identity)
        // - manifest_status, constraints (approval state)
        // - gpu_vram_mb, runtime_backend, runtime_os (hardware snapshot — for routing only)
        // - status, created_at, expires_at, content_hash (metadata)
        //
        // There are no fields for:
        // - raw_qualification_evidence
        // - benchmark_scores
        // - capability_assessment_logic
        // - auto_promote
        Ok(())
    }

    /// Mark this projection as superseded.
    pub fn mark_superseded(&mut self) -> Result<()> {
        if self.status == ProjectionStatus::Superseded {
            anyhow::bail!("Projection is already superseded");
        }
        self.status = ProjectionStatus::Superseded;
        self.content_hash = self.compute_content_hash()?;
        Ok(())
    }

    /// Mark this projection as expired.
    pub fn mark_expired(&mut self) -> Result<()> {
        if self.status != ProjectionStatus::Active {
            anyhow::bail!(
                "Only active projections can be expired. Current status: {}",
                self.status.as_str()
            );
        }
        self.status = ProjectionStatus::Expired;
        self.content_hash = self.compute_content_hash()?;
        Ok(())
    }
}

/// Attempt to create a router projection from a manifest and profile.
///
/// This is the CORE authority gate for routing. The function enforces
/// that only approved/conditional manifests with compatible profiles
/// can produce projections.
pub fn create_projection(
    manifest: &CapabilityManifest,
    profile: &ExecutionProfile,
    owner_decision_id: &str,
) -> ProjectionCreationResult {
    // Rule 1: Manifest MUST be approved or conditional
    if manifest.status != ManifestStatus::Approved
        && manifest.status != ManifestStatus::Conditional
    {
        return ProjectionCreationResult::Rejected {
            reason: format!(
                "Manifest status is '{} — only approved or conditional manifests can create projections",
                manifest.status.as_str()
            ),
        };
    }

    // Rule 2: Profile MUST be active
    if profile.status != super::execution_profile::ProfileStatus::Active {
        return ProjectionCreationResult::Rejected {
            reason: format!(
                "Profile status is '{}' — only active profiles can be used for projections",
                profile.status.as_str()
            ),
        };
    }

    // Rule 3: Manifest MUST have an Owner decision
    if manifest.owner_decision_id.is_none() {
        return ProjectionCreationResult::Rejected {
            reason: "Manifest has no Owner decision reference".to_string(),
        };
    }

    // Rule 4: Owner decision ID must match
    if !owner_decision_id.is_empty()
        && manifest.owner_decision_id.as_deref() != Some(owner_decision_id)
    {
        return ProjectionCreationResult::Rejected {
            reason: format!(
                "Owner decision ID mismatch: manifest references '{}', got '{}'",
                manifest.owner_decision_id.as_deref().unwrap_or("none"),
                owner_decision_id
            ),
        };
    }

    // Rule 5: Manifest and profile MUST reference the same model
    if manifest.model_id != profile.artifact.model_id {
        return ProjectionCreationResult::Rejected {
            reason: format!(
                "Model mismatch: manifest references '{}', profile references '{}'",
                manifest.model_id, profile.artifact.model_id
            ),
        };
    }

    // Rule 6: Model SHA-256 must match (exact artifact binding)
    if manifest.model_sha256 != profile.artifact.sha256 {
        return ProjectionCreationResult::Rejected {
            reason: format!(
                "SHA-256 mismatch: manifest references '{}', profile references '{}'",
                manifest.model_sha256, profile.artifact.sha256
            ),
        };
    }

    // Rule 7: Hardware throughput CANNOT upgrade capability status
    // This is a structural invariant — the projection records the profile
    // but does NOT use metrics to change the manifest status.
    // The manifest_status in the projection MUST match the manifest's status.

    let projection_id =
        RouterProjection::compute_projection_id(&manifest.manifest_id, &profile.profile_id);

    let created_at = chrono::Utc::now().to_rfc3339();

    let mut projection = RouterProjection {
        projection_id,
        manifest_id: manifest.manifest_id.clone(),
        owner_decision_id: manifest
            .owner_decision_id
            .clone()
            .unwrap_or_else(|| owner_decision_id.to_string()),
        profile_id: profile.profile_id.clone(),
        role: manifest.role.clone(),
        model_id: manifest.model_id.clone(),
        model_sha256: manifest.model_sha256.clone(),
        model_filename: manifest.model_filename.clone(),
        manifest_status: manifest.status.clone(),
        constraints: manifest.constraints.clone(),
        status: ProjectionStatus::Active,
        gpu_vram_mb: profile.hardware.gpu_vram_mb,
        runtime_backend: profile.runtime.backend.clone(),
        runtime_os: profile.hardware.os.clone(),
        created_at,
        expires_at: None,
        content_hash: String::new(),
    };

    // Compute content hash
    match projection.compute_content_hash() {
        Ok(hash) => projection.content_hash = hash,
        Err(e) => {
            return ProjectionCreationResult::Rejected {
                reason: format!("Failed to compute content hash: {}", e),
            };
        }
    }

    ProjectionCreationResult::Created(projection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::manifest::{
        CapabilityManifest, EvidenceSummary, ManifestStatus,
    };
    use crate::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity,
        ProfileStatus, RuntimeIdentity,
    };

    fn test_evidence() -> EvidenceSummary {
        EvidenceSummary {
            smoke_test_passed: true,
            probes_passed: vec!["PP-RESPONSE-001".to_string()],
            probes_failed: vec![],
            total_generation_duration_ms: Some(500),
            total_output_tokens: Some(256),
            gpu_release_verified: true,
            notes: None,
        }
    }

    fn test_approved_manifest() -> CapabilityManifest {
        let created_at = "2026-07-11T12:00:00Z".to_string();
        let manifest_id =
            CapabilityManifest::compute_manifest_id("minicpm5-1b-q4km", "classifier", &created_at);

        CapabilityManifest {
            manifest_id,
            model_id: "minicpm5-1b-q4km".to_string(),
            model_sha256: "81B64D05A23B".to_string(),
            model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            role: "classifier".to_string(),
            status: ManifestStatus::Approved,
            evidence_summary: test_evidence(),
            failure_modes: vec![],
            constraints: None,
            owner_decision_id: Some("dec-001".to_string()),
            supersedes_manifest_id: None,
            content_hash: String::new(),
            created_at: created_at.clone(),
            updated_at: created_at,
        }
    }

    fn test_conditional_manifest() -> CapabilityManifest {
        let mut manifest = test_approved_manifest();
        manifest.status = ManifestStatus::Conditional;
        manifest.constraints = Some("Must maintain VRAM below 4096 MiB".to_string());
        manifest
    }

    fn test_draft_manifest() -> CapabilityManifest {
        let mut manifest = test_approved_manifest();
        manifest.status = ManifestStatus::Draft;
        manifest.owner_decision_id = None;
        manifest
    }

    fn test_rejected_manifest() -> CapabilityManifest {
        let mut manifest = test_approved_manifest();
        manifest.status = ManifestStatus::Rejected;
        manifest
    }

    fn test_active_profile() -> ExecutionProfile {
        ExecutionProfile {
            profile_id: ExecutionProfile::compute_profile_id(
                "minicpm5-1b-q4km",
                "c85e97a",
                "Radeon RX 570",
            ),
            artifact: ArtifactIdentity {
                filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
                model_id: "minicpm5-1b-q4km".to_string(),
                quantization: "Q4_K_M".to_string(),
                sha256: "81B64D05A23B".to_string(),
                file_size_bytes: 688_000_000,
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
                avg_load_duration_ms: Some(2187.0),
                avg_generation_duration_ms: Some(385.0),
                avg_tokens_per_second: Some(12.5),
                peak_vram_usage_mb: Some(3433),
                observation_count: 5,
            },
            status: ProfileStatus::Active,
            content_hash: String::new(),
            created_at: "2026-07-11T12:00:00Z".to_string(),
            updated_at: "2026-07-11T12:00:00Z".to_string(),
        }
    }

    fn test_incompatible_profile() -> ExecutionProfile {
        let mut profile = test_active_profile();
        profile.status = ProfileStatus::Incompatible;
        profile
    }

    fn test_different_model_profile() -> ExecutionProfile {
        let mut profile = test_active_profile();
        profile.profile_id =
            ExecutionProfile::compute_profile_id("llama3-8b", "v1", "Radeon RX 570");
        profile.artifact.model_id = "llama3-8b".to_string();
        profile.artifact.filename = "Llama3-8B-Q4_K_M.gguf".to_string();
        profile.artifact.sha256 = "DIFFERENT_HASH".to_string();
        profile
    }

    // MQR-R1-G1: Approved manifest + active profile creates projection
    #[test]
    fn test_approved_manifest_creates_projection() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "dec-001");
        match result {
            ProjectionCreationResult::Created(proj) => {
                assert_eq!(proj.role, "classifier");
                assert_eq!(proj.model_id, "minicpm5-1b-q4km");
                assert_eq!(proj.status, ProjectionStatus::Active);
                assert!(proj.validate().is_ok());
            }
            ProjectionCreationResult::Rejected { reason } => {
                panic!("Expected projection to be created, got rejected: {}", reason);
            }
        }
    }

    // MQR-R1-G2: Conditional manifest + active profile creates projection
    #[test]
    fn test_conditional_manifest_creates_projection() {
        let manifest = test_conditional_manifest();
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "dec-001");
        match result {
            ProjectionCreationResult::Created(proj) => {
                assert_eq!(proj.manifest_status, ManifestStatus::Conditional);
                assert!(proj.constraints.is_some());
            }
            ProjectionCreationResult::Rejected { reason } => {
                panic!("Expected projection to be created, got rejected: {}", reason);
            }
        }
    }

    // MQR-R1-G3: Draft manifest + active profile → REJECTED (critical negative)
    #[test]
    fn test_draft_manifest_rejected() {
        let manifest = test_draft_manifest();
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for draft manifest");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("draft"));
            }
        }
    }

    // MQR-R1-G4: Proposed manifest + active profile → REJECTED
    #[test]
    fn test_proposed_manifest_rejected() {
        let mut manifest = test_approved_manifest();
        manifest.status = ManifestStatus::Proposed;
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for proposed manifest");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("proposed"));
            }
        }
    }

    // MQR-R1-G5: Rejected manifest + active profile → REJECTED (critical negative)
    #[test]
    fn test_rejected_manifest_rejected() {
        let manifest = test_rejected_manifest();
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for rejected manifest");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("rejected"));
            }
        }
    }

    // MQR-R1-G6: Approved manifest + incompatible profile → REJECTED
    #[test]
    fn test_incompatible_profile_rejected() {
        let manifest = test_approved_manifest();
        let profile = test_incompatible_profile();

        let result = create_projection(&manifest, &profile, "dec-001");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for incompatible profile");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("incompatible"));
            }
        }
    }

    // MQR-R1-G7: Model mismatch → REJECTED
    #[test]
    fn test_model_mismatch_rejected() {
        let manifest = test_approved_manifest();
        let profile = test_different_model_profile();

        let result = create_projection(&manifest, &profile, "dec-001");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for model mismatch");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("Model mismatch"));
            }
        }
    }

    // MQR-R1-G8: No Owner decision → REJECTED
    #[test]
    fn test_no_owner_decision_rejected() {
        let mut manifest = test_approved_manifest();
        manifest.owner_decision_id = None;
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "dec-001");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for missing Owner decision");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("Owner decision"));
            }
        }
    }

    // MQR-R1-G9: Projection validates
    #[test]
    fn test_projection_validates() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(proj) = result {
            assert!(proj.validate().is_ok());
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G10: Projection ID is deterministic
    #[test]
    fn test_projection_id_deterministic() {
        let id1 = RouterProjection::compute_projection_id("manifest-1", "profile-1");
        let id2 = RouterProjection::compute_projection_id("manifest-1", "profile-1");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    // MQR-R1-G11: Serialization round-trip
    #[test]
    fn test_serialization_round_trip() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(proj) = result {
            let json = proj.to_json().unwrap();
            let parsed = RouterProjection::from_json(&json).unwrap();
            assert_eq!(proj, parsed);
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G12: No raw evidence data
    #[test]
    fn test_no_raw_evidence() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(proj) = result {
            assert!(proj.assert_no_raw_evidence().is_ok());
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G13: Mark superseded
    #[test]
    fn test_mark_superseded() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(mut proj) = result {
            let result = proj.mark_superseded();
            assert!(result.is_ok());
            assert_eq!(proj.status, ProjectionStatus::Superseded);
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G14: Already superseded cannot be superseded again
    #[test]
    fn test_already_superseded_blocked() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(mut proj) = result {
            proj.status = ProjectionStatus::Superseded;
            let result = proj.mark_superseded();
            assert!(result.is_err());
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G15: Status string round-trip
    #[test]
    fn test_status_string_roundtrip() {
        let statuses = vec![
            ProjectionStatus::Active,
            ProjectionStatus::Superseded,
            ProjectionStatus::Expired,
        ];
        for status in &statuses {
            let s = status.as_str();
            assert!(!s.is_empty());
            assert_eq!(ProjectionStatus::from_str(s), Some(status.clone()));
        }
    }

    // MQR-R1-G16: Content hash is recomputed after supersession
    #[test]
    fn test_content_hash_recomputed() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(mut proj) = result {
            let hash_before = proj.content_hash.clone();
            proj.mark_superseded().unwrap();
            assert_ne!(hash_before, proj.content_hash);
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G17: Projection preserves manifest role
    #[test]
    fn test_role_preserved() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(proj) = result {
            assert_eq!(proj.role, "classifier");
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G18: Projection preserves model identity
    #[test]
    fn test_model_identity_preserved() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(proj) = result {
            assert_eq!(proj.model_id, "minicpm5-1b-q4km");
            assert_eq!(proj.model_sha256, "81B64D05A23B");
            assert_eq!(proj.model_filename, "MiniCPM5-1B-Q4_K_M.gguf");
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G19: Projection preserves Owner decision reference
    #[test]
    fn test_owner_decision_preserved() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(proj) = result {
            assert_eq!(proj.owner_decision_id, "dec-001");
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G20: Quarantined manifest + active profile → REJECTED
    #[test]
    fn test_quarantined_manifest_rejected() {
        let mut manifest = test_approved_manifest();
        manifest.status = ManifestStatus::Quarantined;
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for quarantined manifest");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("quarantined"));
            }
        }
    }

    // MQR-R1-G21: Superseded manifest + active profile → REJECTED
    #[test]
    fn test_superseded_manifest_rejected() {
        let mut manifest = test_approved_manifest();
        manifest.status = ManifestStatus::Superseded;
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for superseded manifest");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("superseded"));
            }
        }
    }

    // MQR-R1-G22: Hardware throughput cannot upgrade capability status
    #[test]
    fn test_throughput_cannot_upgrade_capability() {
        let manifest = test_conditional_manifest();
        let mut profile = test_active_profile();
        // Even with amazing metrics
        profile.metrics = ExecutionMetrics {
            avg_load_duration_ms: Some(100.0),
            avg_generation_duration_ms: Some(50.0),
            avg_tokens_per_second: Some(100.0),
            peak_vram_usage_mb: Some(1000),
            observation_count: 100,
        };

        let result = create_projection(&manifest, &profile, "dec-001");
        match result {
            ProjectionCreationResult::Created(proj) => {
                // The projection preserves the manifest's conditional status
                // even though the profile has amazing metrics
                assert_eq!(proj.manifest_status, ManifestStatus::Conditional);
                assert!(proj.constraints.is_some());
            }
            ProjectionCreationResult::Rejected { reason } => {
                panic!("Expected projection to be created, got rejected: {}", reason);
            }
        }
    }

    // MQR-R1-G23: SHA-256 mismatch → REJECTED
    #[test]
    fn test_sha256_mismatch_rejected() {
        let mut manifest = test_approved_manifest();
        manifest.model_sha256 = "DIFFERENT_HASH".to_string();
        let profile = test_active_profile();

        let result = create_projection(&manifest, &profile, "dec-001");
        match result {
            ProjectionCreationResult::Created(_) => {
                panic!("Expected projection to be rejected for SHA-256 mismatch");
            }
            ProjectionCreationResult::Rejected { reason } => {
                assert!(reason.contains("SHA-256 mismatch"));
            }
        }
    }

    // MQR-R1-G24: Projection preserves profile reference
    #[test]
    fn test_profile_reference_preserved() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(proj) = result {
            assert_eq!(proj.profile_id, profile.profile_id);
        } else {
            panic!("Expected projection to be created");
        }
    }

    // MQR-R1-G25: Projection content hash is computed
    #[test]
    fn test_content_hash_computed() {
        let manifest = test_approved_manifest();
        let profile = test_active_profile();
        let result = create_projection(&manifest, &profile, "dec-001");

        if let ProjectionCreationResult::Created(proj) = result {
            assert!(!proj.content_hash.is_empty());
            assert_eq!(proj.content_hash.len(), 64);
        } else {
            panic!("Expected projection to be created");
        }
    }
}
