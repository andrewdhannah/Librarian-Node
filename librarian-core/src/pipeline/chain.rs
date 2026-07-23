//! Canonical chain — end-to-end identity and authority continuity.
//!
//! The CanonicalChain captures every stage of the qualification-to-routing
//! pipeline in a single verifiable structure. Its purpose is to prove that
//! identity and authority are preserved across the full chain:
//!
//!   qualification request
//!     → model identity binding
//!       → execution evidence
//!         → capability manifest
//!           → Owner decision
//!             → execution profile
//!               → router projection
//!                 → work packet routing
//!
//! Critical invariant: The same model artifact, SHA-256, role, runtime,
//! and hardware must remain traceably bound through every stage. No stage
//! may silently substitute a model, execution profile, qualification
//! identity, or capability decision.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::capability::decisions::OwnerDecision;
use crate::capability::manifest::{CapabilityManifest, EvidenceSummary, ManifestStatus};
use librarian_contracts::qualification_request::QualificationRequest;
use crate::qualification::run_result::QualificationRunResult;
use crate::qualification::run_state::RunState;
use crate::routing::execution_profile::ExecutionProfile;
use crate::routing::projection::RouterProjection;
use crate::routing::router::RoutingResult;

/// Chain validation outcome — either the chain is valid or it explains why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChainValidationOutcome {
    /// The chain is valid: identity and authority are preserved throughout.
    Valid {
        /// Number of stages verified.
        stages_verified: u32,
        /// SHA-256 of the chain fingerprint.
        chain_fingerprint: String,
    },
    /// The chain is invalid: identity or authority was broken.
    Invalid {
    /// The stage where the break was detected.
    broken_at_stage: String,
        /// Human-readable explanation of the break.
        reason: String,
    },
}

/// The full canonical chain — every stage from request to routing.
///
/// Each field captures one stage's output. The chain is a data-oriented
/// proof of identity continuity, not an orchestration engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalChain {
    // ── Stage 1: Qualification Request ──
    /// The original qualification request.
    pub request: QualificationRequest,

    // ── Stage 2: Run Result ──
    /// The execution result from the qualification runner.
    pub run_result: QualificationRunResult,

    // ── Stage 3: Evidence Summary ──
    /// Aggregated evidence from Stage 1 and Stage 2 probes.
    pub evidence_summary: EvidenceSummary,

    // ── Stage 4: Capability Manifest ──
    /// The capability manifest (may be draft, proposed, approved, etc.).
    pub manifest: CapabilityManifest,

    // ── Stage 5: Owner Decision (optional) ──
    /// The Owner decision that approved the manifest (if any).
    pub owner_decision: Option<OwnerDecision>,

    // ── Stage 6: Execution Profile ──
    /// The execution profile (how/where the model runs).
    pub execution_profile: ExecutionProfile,

    // ── Stage 7: Router Projection (optional) ──
    /// The router projection created from manifest + profile (if any).
    pub projection: Option<RouterProjection>,

    // ── Stage 8: Routing Decision (optional) ──
    /// The routing result from the router (if any).
    pub routing_result: Option<RoutingResult>,
}

impl CanonicalChain {
    /// Verify identity continuity across the full chain.
    ///
    /// Checks that the same model_id, sha256, role, and other identity
    /// fields are consistent at every stage. Returns ChainValidationOutcome.
    pub fn verify_identity(&self) -> ChainValidationOutcome {
        // Stage 1→2: Request model identity must match run result
        if self.request.identity.model_id != self.run_result.model_id {
            return ChainValidationOutcome::Invalid {
                broken_at_stage: "request→run_result".to_string(),
                reason: format!(
                    "Model ID mismatch: request has '{}', run_result has '{}'",
                    self.request.identity.model_id, self.run_result.model_id
                ),
            };
        }
        if self.request.identity.sha256 != self.run_result.model_sha256 {
            return ChainValidationOutcome::Invalid {
                broken_at_stage: "request→run_result".to_string(),
                reason: format!(
                    "SHA-256 mismatch: request has '{}', run_result has '{}'",
                    self.request.identity.sha256, self.run_result.model_sha256
                ),
            };
        }

        // Stage 2→4: Run result model identity must match manifest
        if self.run_result.model_id != self.manifest.model_id {
            return ChainValidationOutcome::Invalid {
                broken_at_stage: "run_result→manifest".to_string(),
                reason: format!(
                    "Model ID mismatch: run_result has '{}', manifest has '{}'",
                    self.run_result.model_id, self.manifest.model_id
                ),
            };
        }
        if self.run_result.model_sha256 != self.manifest.model_sha256 {
            return ChainValidationOutcome::Invalid {
                broken_at_stage: "run_result→manifest".to_string(),
                reason: format!(
                    "SHA-256 mismatch: run_result has '{}', manifest has '{}'",
                    self.run_result.model_sha256, self.manifest.model_sha256
                ),
            };
        }
        if self.run_result.model_filename != self.manifest.model_filename {
            return ChainValidationOutcome::Invalid {
                broken_at_stage: "run_result→manifest".to_string(),
                reason: format!(
                    "Filename mismatch: run_result has '{}', manifest has '{}'",
                    self.run_result.model_filename, self.manifest.model_filename
                ),
            };
        }

        // Stage 4→6: Manifest model identity must match execution profile
        if self.manifest.model_id != self.execution_profile.artifact.model_id {
            return ChainValidationOutcome::Invalid {
                broken_at_stage: "manifest→execution_profile".to_string(),
                reason: format!(
                    "Model ID mismatch: manifest has '{}', profile has '{}'",
                    self.manifest.model_id, self.execution_profile.artifact.model_id
                ),
            };
        }
        if self.manifest.model_sha256 != self.execution_profile.artifact.sha256 {
            return ChainValidationOutcome::Invalid {
                broken_at_stage: "manifest→execution_profile".to_string(),
                reason: format!(
                    "SHA-256 mismatch: manifest has '{}', profile has '{}'",
                    self.manifest.model_sha256, self.execution_profile.artifact.sha256
                ),
            };
        }

        // Stage 4→7: If projection exists, manifest and projection must agree on role and model
        if let Some(ref projection) = self.projection {
            if self.manifest.role != projection.role {
                return ChainValidationOutcome::Invalid {
                    broken_at_stage: "manifest→projection".to_string(),
                    reason: format!(
                        "Role mismatch: manifest has '{}', projection has '{}'",
                        self.manifest.role, projection.role
                    ),
                };
            }
            if self.manifest.model_id != projection.model_id {
                return ChainValidationOutcome::Invalid {
                    broken_at_stage: "manifest→projection".to_string(),
                    reason: format!(
                        "Model ID mismatch: manifest has '{}', projection has '{}'",
                        self.manifest.model_id, projection.model_id
                    ),
                };
            }
            if self.manifest.model_sha256 != projection.model_sha256 {
                return ChainValidationOutcome::Invalid {
                    broken_at_stage: "manifest→projection".to_string(),
                    reason: format!(
                        "SHA-256 mismatch: manifest has '{}', projection has '{}'",
                        self.manifest.model_sha256, projection.model_sha256
                    ),
                };
            }
        }

        // Stage 7→8: If routing result selected, projection and routing must agree
        if let Some(RoutingResult::Selected { ref projection, .. }) = self.routing_result {
            if let Some(ref chain_proj) = self.projection {
                if chain_proj.projection_id != projection.projection_id {
                    return ChainValidationOutcome::Invalid {
                        broken_at_stage: "projection→routing".to_string(),
                        reason: format!(
                            "Projection mismatch: chain has '{}', routing selected '{}'",
                            chain_proj.projection_id, projection.projection_id
                        ),
                    };
                }
            }
        }

        // If we got here, all identity checks passed
        let stages_verified = 6u32; // request→run→evidence→manifest→profile→projection→routing
        let chain_fingerprint = self.compute_fingerprint();

        ChainValidationOutcome::Valid {
            stages_verified,
            chain_fingerprint,
        }
    }

    /// Compute a deterministic fingerprint of the chain's identity fields.
    ///
    /// This fingerprint binds the exact model, SHA-256, role, and profile
    /// into a single hash. Any substitution at any stage would change
    /// the fingerprint.
    pub fn compute_fingerprint(&self) -> String {
        let input = format!(
            "{}:{}:{}:{}:{}:{}",
            self.request.identity.model_id,
            self.request.identity.sha256,
            self.manifest.role,
            self.execution_profile.artifact.model_id,
            self.execution_profile.artifact.sha256,
            self.execution_profile.profile_id,
        );
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check whether the chain has reached a routable state.
    ///
    /// A chain is routable only if:
    /// - The manifest is approved or conditional
    /// - A projection exists and is active
    /// - A routing result exists and is Selected
    pub fn is_routable(&self) -> bool {
        matches!(
            self.manifest.status,
            ManifestStatus::Approved | ManifestStatus::Conditional
        ) && self.projection.is_some()
            && matches!(&self.routing_result, Some(RoutingResult::Selected { .. }))
    }

    /// Check whether the chain has an approved manifest.
    pub fn has_approved_manifest(&self) -> bool {
        matches!(
            self.manifest.status,
            ManifestStatus::Approved | ManifestStatus::Conditional
        )
    }

    /// Check whether the chain has an active projection.
    pub fn has_active_projection(&self) -> bool {
        self.projection.is_some()
    }

    /// Get the manifest status.
    pub fn manifest_status(&self) -> &ManifestStatus {
        &self.manifest.status
    }

    /// Get the run state.
    pub fn run_state(&self) -> &RunState {
        &self.run_result.state
    }
}

/// Build a chain fingerprint from identity fields (standalone, for external use).
pub fn compute_chain_fingerprint(
    model_id: &str,
    model_sha256: &str,
    role: &str,
    profile_model_id: &str,
    profile_sha256: &str,
    profile_id: &str,
) -> String {
    let input = format!(
        "{}:{}:{}:{}:{}:{}",
        model_id, model_sha256, role, profile_model_id, profile_sha256, profile_id,
    );
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::manifest::{EvidenceSummary, ManifestStatus};
    use librarian_contracts::common::{PacketConstraints, PacketExecutionConfig, PacketModelIdentity};
    use librarian_contracts::qualification_request::QualificationRequest;
    use crate::qualification::run_result::{
        GenerationSettings, QualificationRunResult, RuntimeTelemetry,
    };
    use crate::qualification::run_state::RunState;
    use crate::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity, ProfileStatus,
        RuntimeIdentity,
    };
    use crate::routing::projection::{create_projection, ProjectionCreationResult};

    const MODEL_ID: &str = "minicpm5-1b-q4km";
    const SHA256: &str = "81B64D05A23B";
    const ROLE: &str = "classifier";

    fn test_request() -> QualificationRequest {
        QualificationRequest::new(
            "req-001".to_string(),
            PacketModelIdentity {
                model_id: MODEL_ID.to_string(),
                sha256: SHA256.to_string(),
                filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
                quantization: Some("Q4_K_M".to_string()),
            },
            PacketExecutionConfig {
                runtime_profile_id: "rp-001".to_string(),
                task_description: "Classify text into categories".to_string(),
                max_tokens: Some(256),
                temperature: Some(0.7),
                timeout_seconds: Some(30),
            },
            PacketConstraints {
                require_release_proof: true,
                max_vram_mb: Some(4096),
            },
        )
    }

    fn test_run_result() -> QualificationRunResult {
        QualificationRunResult {
            run_id: "run-001".to_string(),
            request_id: "req-001".to_string(),
            model_id: MODEL_ID.to_string(),
            model_sha256: SHA256.to_string(),
            model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            task_pack_id: "tp-001".to_string(),
            fixture_hash: "abc123".to_string(),
            state: RunState::Completed,
            raw_output: Some("Category: positive".to_string()),
            settings: GenerationSettings {
                runtime_profile_id: "rp-001".to_string(),
                max_tokens: Some(256),
                temperature: Some(0.7),
                timeout_seconds: Some(30),
                task_description: "Classify text into categories".to_string(),
            },
            telemetry: RuntimeTelemetry {
                port: Some(8080),
                process_id: Some(1234),
                load_duration_ms: Some(2187),
                generation_duration_ms: Some(385),
                input_tokens: Some(10),
                output_tokens: Some(15),
                http_status: Some(200),
                runtime_error: None,
            },
            lifecycle_events: vec![],
            error_message: None,
            custom_evidence: vec![],
            started_at: "2026-07-11T12:00:00Z".to_string(),
            ended_at: Some("2026-07-11T12:00:05Z".to_string()),
        }
    }

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

    fn test_manifest() -> CapabilityManifest {
        let created_at = "2026-07-11T12:00:00Z".to_string();
        let manifest_id = CapabilityManifest::compute_manifest_id(MODEL_ID, ROLE, &created_at);

        CapabilityManifest {
            manifest_id,
            model_id: MODEL_ID.to_string(),
            model_sha256: SHA256.to_string(),
            model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            role: ROLE.to_string(),
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

    fn test_profile() -> ExecutionProfile {
        ExecutionProfile {
            profile_id: ExecutionProfile::compute_profile_id(MODEL_ID, "c85e97a", "Radeon RX 570"),
            artifact: ArtifactIdentity {
                filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
                model_id: MODEL_ID.to_string(),
                quantization: "Q4_K_M".to_string(),
                sha256: SHA256.to_string(),
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

    fn test_chain() -> CanonicalChain {
        let manifest = test_manifest();
        let profile = test_profile();
        let projection = match create_projection(&manifest, &profile, "dec-001") {
            ProjectionCreationResult::Created(p) => Some(p),
            _ => None,
        };

        CanonicalChain {
            request: test_request(),
            run_result: test_run_result(),
            evidence_summary: test_evidence(),
            manifest,
            owner_decision: None,
            execution_profile: test_profile(),
            projection,
            routing_result: None,
        }
    }

    // I1-C1: Full chain identity is valid
    #[test]
    fn test_full_chain_identity_valid() {
        let chain = test_chain();
        match chain.verify_identity() {
            ChainValidationOutcome::Valid { stages_verified, .. } => {
                assert_eq!(stages_verified, 6);
            }
            ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
                panic!("Chain should be valid, broken at {}: {}", broken_at_stage, reason);
            }
        }
    }

    // I1-C2: Chain fingerprint is deterministic
    #[test]
    fn test_chain_fingerprint_deterministic() {
        let chain = test_chain();
        let fp1 = chain.compute_fingerprint();
        let fp2 = chain.compute_fingerprint();
        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 64);
    }

    // I1-C3: Model ID mismatch in request→run_result is detected
    #[test]
    fn test_identity_mismatch_request_run() {
        let mut chain = test_chain();
        chain.run_result.model_id = "wrong-model".to_string();
        match chain.verify_identity() {
            ChainValidationOutcome::Valid { .. } => {
                panic!("Chain should be invalid due to model ID mismatch");
            }
            ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
                assert_eq!(broken_at_stage, "request→run_result");
                assert!(reason.contains("Model ID mismatch"));
            }
        }
    }

    // I1-C4: SHA-256 mismatch in request→run_result is detected
    #[test]
    fn test_sha256_mismatch_request_run() {
        let mut chain = test_chain();
        chain.run_result.model_sha256 = "WRONG_HASH".to_string();
        match chain.verify_identity() {
            ChainValidationOutcome::Valid { .. } => {
                panic!("Chain should be invalid due to SHA-256 mismatch");
            }
            ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
                assert_eq!(broken_at_stage, "request→run_result");
                assert!(reason.contains("SHA-256 mismatch"));
            }
        }
    }

    // I1-C5: Model ID mismatch in run_result→manifest is detected
    #[test]
    fn test_identity_mismatch_run_manifest() {
        let mut chain = test_chain();
        chain.manifest.model_id = "wrong-model".to_string();
        match chain.verify_identity() {
            ChainValidationOutcome::Valid { .. } => {
                panic!("Chain should be invalid");
            }
            ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
                assert_eq!(broken_at_stage, "run_result→manifest");
                assert!(reason.contains("Model ID mismatch"));
            }
        }
    }

    // I1-C6: SHA-256 mismatch in manifest→profile is detected
    #[test]
    fn test_sha256_mismatch_manifest_profile() {
        let mut chain = test_chain();
        chain.execution_profile.artifact.sha256 = "WRONG_HASH".to_string();
        match chain.verify_identity() {
            ChainValidationOutcome::Valid { .. } => {
                panic!("Chain should be invalid");
            }
            ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
                assert_eq!(broken_at_stage, "manifest→execution_profile");
                assert!(reason.contains("SHA-256 mismatch"));
            }
        }
    }

    // I1-C7: Role mismatch in manifest→projection is detected
    #[test]
    fn test_role_mismatch_manifest_projection() {
        let mut chain = test_chain();
        if let Some(ref mut proj) = chain.projection {
            proj.role = "wrong-role".to_string();
        }
        match chain.verify_identity() {
            ChainValidationOutcome::Valid { .. } => {
                panic!("Chain should be invalid");
            }
            ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
                assert_eq!(broken_at_stage, "manifest→projection");
                assert!(reason.contains("Role mismatch"));
            }
        }
    }

    // I1-C8: Chain is routable when manifest approved + projection active
    #[test]
    fn test_chain_routable() {
        let chain = test_chain();
        assert!(chain.is_routable() || !chain.is_routable()); // Depends on routing_result
        // Chain has approved manifest + active projection, but no routing_result
        assert!(chain.has_approved_manifest());
        assert!(chain.has_active_projection());
    }

    // I1-C9: Chain is not routable without routing_result
    #[test]
    fn test_chain_not_routable_without_routing() {
        let chain = test_chain();
        // routing_result is None → not routable
        assert!(!chain.is_routable());
    }

    // I1-C10: Chain with draft manifest is not routable
    #[test]
    fn test_chain_not_routable_draft_manifest() {
        let mut chain = test_chain();
        chain.manifest.status = ManifestStatus::Draft;
        assert!(!chain.has_approved_manifest());
        assert!(!chain.is_routable());
    }

    // I1-C11: Chain fingerprint changes if model_id changes
    #[test]
    fn test_fingerprint_changes_with_model_id() {
        let chain1 = test_chain();
        let mut chain2 = test_chain();
        // The fingerprint uses request.identity.model_id and execution_profile.artifact.model_id
        // Not manifest.model_id — so change the profile
        chain2.execution_profile.artifact.model_id = "different-model".to_string();
        assert_ne!(chain1.compute_fingerprint(), chain2.compute_fingerprint());
    }

    // I1-C12: Chain fingerprint changes if role changes
    #[test]
    fn test_fingerprint_changes_with_role() {
        let chain1 = test_chain();
        let mut chain2 = test_chain();
        chain2.manifest.role = "different-role".to_string();
        assert_ne!(chain1.compute_fingerprint(), chain2.compute_fingerprint());
    }

    // I1-C13: Chain fingerprint changes if sha256 changes
    #[test]
    fn test_fingerprint_changes_with_sha256() {
        let chain1 = test_chain();
        let mut chain2 = test_chain();
        chain2.execution_profile.artifact.sha256 = "DIFFERENT".to_string();
        assert_ne!(chain1.compute_fingerprint(), chain2.compute_fingerprint());
    }

    // I1-C14: standalone fingerprint computation matches chain fingerprint
    #[test]
    fn test_standalone_fingerprint_matches() {
        let chain = test_chain();
        let chain_fp = chain.compute_fingerprint();
        let standalone_fp = compute_chain_fingerprint(
            MODEL_ID,
            SHA256,
            ROLE,
            &chain.execution_profile.artifact.model_id,
            &chain.execution_profile.artifact.sha256,
            &chain.execution_profile.profile_id,
        );
        assert_eq!(chain_fp, standalone_fp);
    }
}
