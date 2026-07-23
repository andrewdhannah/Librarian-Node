//! Router — selects projections for work packets.
//!
//! The router is the operational decision point. It takes a work packet
//! (with a required role) and selects the best approved projection that
//! satisfies the role and any hardware constraints.
//!
//! Critical invariants:
//! - Router only consumes RouterProjection — never raw benchmarks or manifests
//! - Router only selects Active projections
//! - Router records every decision in the routing log
//! - Router fails closed: no projection → NoProjection, never a default

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::log::{log_rejected, log_selected, RoutingLogEntry, RoutingStatus};
use super::projection::{ProjectionStatus, RouterProjection};

/// Hardware constraints for routing — filters projections by hardware requirements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardwareConstraints {
    /// Minimum GPU VRAM in MiB required by the work packet.
    pub min_gpu_vram_mb: Option<u64>,
    /// Required backend (e.g., "vulkan", "cuda", "cpu"). None = any.
    pub required_backend: Option<String>,
    /// Required OS platform. None = any.
    pub required_os: Option<String>,
}

impl Default for HardwareConstraints {
    fn default() -> Self {
        Self {
            min_gpu_vram_mb: None,
            required_backend: None,
            required_os: None,
        }
    }
}

/// Work packet — the input to the router.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkPacket {
    /// Unique packet identifier.
    pub packet_id: String,

    /// The role required by this work packet.
    pub required_role: String,

    /// Optional hardware constraints.
    pub hardware_constraints: Option<HardwareConstraints>,
}

impl WorkPacket {
    /// Validate the work packet structure.
    pub fn validate(&self) -> Result<()> {
        if self.packet_id.is_empty() {
            anyhow::bail!("packet_id is empty");
        }
        if self.required_role.is_empty() {
            anyhow::bail!("required_role is empty");
        }
        Ok(())
    }
}

/// Routing result — either a selected projection or a rejection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoutingResult {
    /// A projection was selected.
    Selected {
        projection: RouterProjection,
        log_entry: RoutingLogEntry,
    },
    /// No projection was found or hardware constraints eliminated all candidates.
    Rejected {
        status: RoutingStatus,
        reason: String,
        log_entry: RoutingLogEntry,
    },
}

/// Check if a projection satisfies hardware constraints.
fn satisfies_constraints(projection: &RouterProjection, constraints: &HardwareConstraints) -> bool {
    // Check GPU VRAM: projection must have at least the minimum required.
    if let Some(min_vram) = constraints.min_gpu_vram_mb {
        if projection.gpu_vram_mb < min_vram {
            return false;
        }
    }

    // Check backend: projection must match the required backend.
    if let Some(ref required_backend) = constraints.required_backend {
        if projection.runtime_backend != *required_backend {
            return false;
        }
    }

    // Check OS: projection must match the required OS.
    if let Some(ref required_os) = constraints.required_os {
        if projection.runtime_os != *required_os {
            return false;
        }
    }

    true
}

/// The Router — selects projections for work packets.
pub struct Router;

impl Router {
    /// Route a work packet to the best matching projection.
    ///
    /// Takes a work packet and a list of active projections. Returns a
    /// RoutingResult with the selected projection and a log entry.
    ///
    /// Selection algorithm:
    /// 1. Filter projections by role match
    /// 2. Filter by active status (should already be filtered)
    /// 3. If no candidates → NoProjection
    /// 4. If exactly one → Selected
    /// 5. If multiple → select by most recent created_at (newest wins)
    /// 6. Record decision in log
    pub fn route(
        packet: &WorkPacket,
        projections: &[RouterProjection],
        hardware_constraints: &HardwareConstraints,
    ) -> Result<RoutingResult> {
        // Validate the packet
        packet.validate().context("Invalid work packet")?;

        // Step 1: Filter by role
        let role_matches: Vec<&RouterProjection> = projections
            .iter()
            .filter(|p| p.role == packet.required_role && p.status == ProjectionStatus::Active)
            .collect();

        // Step 2: Apply hardware constraints at projection level
        let candidates: Vec<&RouterProjection> = role_matches
            .iter()
            .filter(|p| satisfies_constraints(p, hardware_constraints))
            .copied()
            .collect();

        // Step 3: No candidates → rejection
        if candidates.is_empty() {
            let status = if role_matches.is_empty() {
                RoutingStatus::NoProjection
            } else {
                RoutingStatus::RejectedByConstraints
            };

            let reason = if role_matches.is_empty() {
                format!(
                    "No active projection found for role '{}'",
                    packet.required_role
                )
            } else {
                format!(
                    "Found {} projection(s) for role '{}' but all eliminated by hardware constraints",
                    role_matches.len(),
                    packet.required_role
                )
            };

            let log_entry = log_rejected(
                &packet.packet_id,
                &packet.required_role,
                &reason,
                status,
            )?;

            return Ok(RoutingResult::Rejected {
                status,
                reason,
                log_entry,
            });
        }

        // Step 4: If exactly one → Select it
        // Step 5: If multiple → select by most recent created_at
        let selected = candidates
            .iter()
            .max_by(|a, b| a.created_at.cmp(&b.created_at))
            .expect("candidates is non-empty");

        let reason = if candidates.len() == 1 {
            format!(
                "Single active projection for role '{}': {}",
                packet.required_role, selected.projection_id
            )
        } else {
            format!(
                "Multiple projections for role '{}'; selected most recent: {} (created_at: {})",
                packet.required_role, selected.projection_id, selected.created_at
            )
        };

        let log_entry = log_selected(
            &packet.packet_id,
            &packet.required_role,
            &selected.projection_id,
            &selected.model_id,
            &selected.profile_id,
            &reason,
        )?;

        Ok(RoutingResult::Selected {
            projection: (*selected).clone(),
            log_entry,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::manifest::{
        CapabilityManifest, EvidenceSummary, ManifestStatus,
    };
    use crate::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity, ProfileStatus,
        RuntimeIdentity,
    };
    use crate::routing::projection::create_projection;

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

    fn test_manifest(role: &str) -> CapabilityManifest {
        let created_at = "2026-07-11T12:00:00Z".to_string();
        let manifest_id =
            CapabilityManifest::compute_manifest_id("minicpm5-1b-q4km", role, &created_at);

        CapabilityManifest {
            manifest_id,
            model_id: "minicpm5-1b-q4km".to_string(),
            model_sha256: "81B64D05A23B".to_string(),
            model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            role: role.to_string(),
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

    fn test_projection(role: &str) -> RouterProjection {
        let manifest = test_manifest(role);
        let profile = test_profile();
        match create_projection(&manifest, &profile, "dec-001") {
            crate::routing::projection::ProjectionCreationResult::Created(p) => p,
            _ => panic!("Failed to create test projection"),
        }
    }

    fn test_packet(role: &str) -> WorkPacket {
        WorkPacket {
            packet_id: "pkt-001".to_string(),
            required_role: role.to_string(),
            hardware_constraints: None,
        }
    }

    // R2-R1: Single projection selected for role
    #[test]
    fn test_single_projection_selected() {
        let proj = test_projection("classifier");
        let packet = test_packet("classifier");
        let result = Router::route(&packet, &[proj.clone()], &HardwareConstraints::default()).unwrap();

        match result {
            RoutingResult::Selected { projection, log_entry } => {
                assert_eq!(projection.projection_id, proj.projection_id);
                assert_eq!(log_entry.status, RoutingStatus::Selected);
                assert!(log_entry.validate().is_ok());
            }
            RoutingResult::Rejected { reason, .. } => {
                panic!("Expected selection, got rejected: {}", reason);
            }
        }
    }

    // R2-R2: No projection for role → NoProjection
    #[test]
    fn test_no_projection_for_role() {
        let proj = test_projection("classifier");
        let packet = test_packet("summarizer");
        let result = Router::route(&packet, &[proj], &HardwareConstraints::default()).unwrap();

        match result {
            RoutingResult::Rejected { status, reason, log_entry } => {
                assert_eq!(status, RoutingStatus::NoProjection);
                assert!(reason.contains("summarizer"));
                assert_eq!(log_entry.status, RoutingStatus::NoProjection);
            }
            RoutingResult::Selected { .. } => {
                panic!("Expected rejection for non-matching role");
            }
        }
    }

    // R2-R3: Empty projections → NoProjection
    #[test]
    fn test_empty_projections() {
        let packet = test_packet("classifier");
        let result = Router::route(&packet, &[], &HardwareConstraints::default()).unwrap();

        match result {
            RoutingResult::Rejected { status, .. } => {
                assert_eq!(status, RoutingStatus::NoProjection);
            }
            RoutingResult::Selected { .. } => {
                panic!("Expected rejection for empty projections");
            }
        }
    }

    // R2-R4: Multiple projections → select most recent
    #[test]
    fn test_multiple_projections_selects_most_recent() {
        let mut proj1 = test_projection("classifier");
        proj1.created_at = "2026-07-11T12:00:00Z".to_string();

        let mut proj2 = test_projection("classifier");
        proj2.created_at = "2026-07-11T13:00:00Z".to_string();

        // Create a second projection with a different manifest to get a unique projection_id
        // The projection_id is computed from manifest_id + profile_id, so I need
        // to use a different manifest or profile for the second one.
        // Let me use a different manifest for the second projection.

        let manifest2 = {
            let mut m = test_manifest("classifier");
            m.manifest_id = CapabilityManifest::compute_manifest_id(
                "minicpm5-1b-q4km",
                "classifier",
                "2026-07-11T13:00:00Z",
            );
            m.created_at = "2026-07-11T13:00:00Z".to_string();
            m.updated_at = "2026-07-11T13:00:00Z".to_string();
            m
        };

        let mut profile2 = test_profile();
        profile2.created_at = "2026-07-11T13:00:00Z".to_string();

        let proj2 = match create_projection(&manifest2, &profile2, "dec-001") {
            crate::routing::projection::ProjectionCreationResult::Created(p) => p,
            _ => panic!("Failed to create second projection"),
        };

        let packet = test_packet("classifier");
        let result = Router::route(&packet, &[proj1, proj2.clone()], &HardwareConstraints::default()).unwrap();

        match result {
            RoutingResult::Selected { projection, .. } => {
                assert_eq!(projection.projection_id, proj2.projection_id);
            }
            RoutingResult::Rejected { reason, .. } => {
                panic!("Expected selection, got rejected: {}", reason);
            }
        }
    }

    // R2-R5: Superseded projection is not selected
    #[test]
    fn test_superseded_projection_not_selected() {
        let mut proj = test_projection("classifier");
        proj.status = ProjectionStatus::Superseded;
        let packet = test_packet("classifier");
        let result = Router::route(&packet, &[proj], &HardwareConstraints::default()).unwrap();

        match result {
            RoutingResult::Rejected { status, .. } => {
                assert_eq!(status, RoutingStatus::NoProjection);
            }
            RoutingResult::Selected { .. } => {
                panic!("Expected rejection for superseded projection");
            }
        }
    }

    // R2-R6: Expired projection is not selected
    #[test]
    fn test_expired_projection_not_selected() {
        let mut proj = test_projection("classifier");
        proj.status = ProjectionStatus::Expired;
        let packet = test_packet("classifier");
        let result = Router::route(&packet, &[proj], &HardwareConstraints::default()).unwrap();

        match result {
            RoutingResult::Rejected { status, .. } => {
                assert_eq!(status, RoutingStatus::NoProjection);
            }
            RoutingResult::Selected { .. } => {
                panic!("Expected rejection for expired projection");
            }
        }
    }

    // R2-R7: Work packet validates
    #[test]
    fn test_work_packet_validates() {
        let packet = test_packet("classifier");
        assert!(packet.validate().is_ok());
    }

    // R2-R8: Work packet validates fails on empty packet_id
    #[test]
    fn test_work_packet_empty_id() {
        let packet = WorkPacket {
            packet_id: "".to_string(),
            required_role: "classifier".to_string(),
            hardware_constraints: None,
        };
        assert!(packet.validate().is_err());
    }

    // R2-R9: Work packet validates fails on empty required_role
    #[test]
    fn test_work_packet_empty_role() {
        let packet = WorkPacket {
            packet_id: "pkt-001".to_string(),
            required_role: "".to_string(),
            hardware_constraints: None,
        };
        assert!(packet.validate().is_err());
    }

    // R2-R10: Log entry records selected projection
    #[test]
    fn test_log_records_selected() {
        let proj = test_projection("classifier");
        let packet = test_packet("classifier");
        let result = Router::route(&packet, &[proj.clone()], &HardwareConstraints::default()).unwrap();

        if let RoutingResult::Selected { log_entry, .. } = result {
            assert_eq!(log_entry.packet_id, "pkt-001");
            assert_eq!(log_entry.role, "classifier");
            assert_eq!(log_entry.projection_id, Some(proj.projection_id));
            assert_eq!(log_entry.status, RoutingStatus::Selected);
        } else {
            panic!("Expected selection");
        }
    }

    // R2-R11: Log entry records rejection
    #[test]
    fn test_log_records_rejection() {
        let packet = test_packet("summarizer");
        let result = Router::route(&packet, &[], &HardwareConstraints::default()).unwrap();

        if let RoutingResult::Rejected { log_entry, .. } = result {
            assert_eq!(log_entry.packet_id, "pkt-001");
            assert_eq!(log_entry.role, "summarizer");
            assert_eq!(log_entry.projection_id, None);
            assert_eq!(log_entry.status, RoutingStatus::NoProjection);
        } else {
            panic!("Expected rejection");
        }
    }

    // R2-R12: Mixed projections — only matching role selected
    #[test]
    fn test_mixed_roles_only_matching_selected() {
        let proj_classifier = test_projection("classifier");
        let proj_summarizer = test_projection("summarizer");
        let packet = test_packet("classifier");

        let result = Router::route(
            &packet,
            &[proj_classifier.clone(), proj_summarizer],
            &HardwareConstraints::default(),
        )
        .unwrap();

        match result {
            RoutingResult::Selected { projection, .. } => {
                assert_eq!(projection.role, "classifier");
                assert_eq!(projection.projection_id, proj_classifier.projection_id);
            }
            RoutingResult::Rejected { reason, .. } => {
                panic!("Expected selection, got rejected: {}", reason);
            }
        }
    }

    // R2-R13: Routing status round-trip
    #[test]
    fn test_routing_status_from_str() {
        assert_eq!(RoutingStatus::from_str("selected"), Some(RoutingStatus::Selected));
        assert_eq!(RoutingStatus::from_str("no_projection"), Some(RoutingStatus::NoProjection));
        assert_eq!(RoutingStatus::from_str("rejected_by_constraints"), Some(RoutingStatus::RejectedByConstraints));
        assert_eq!(RoutingStatus::from_str("ambiguous_role"), Some(RoutingStatus::AmbiguousRole));
        assert_eq!(RoutingStatus::from_str("packet_rejected"), Some(RoutingStatus::PacketRejected));
        assert_eq!(RoutingStatus::from_str("invalid"), None);
    }

    // R2-R14: Hardware constraints default is permissive
    #[test]
    fn test_hardware_constraints_default() {
        let constraints = HardwareConstraints::default();
        assert_eq!(constraints.min_gpu_vram_mb, None);
        assert_eq!(constraints.required_backend, None);
        assert_eq!(constraints.required_os, None);
    }

    // R2-R15: Rejected result contains correct status
    #[test]
    fn test_rejected_result_status() {
        let packet = test_packet("summarizer");
        let result = Router::route(&packet, &[], &HardwareConstraints::default()).unwrap();

        match result {
            RoutingResult::Rejected { status, reason, .. } => {
                assert_eq!(status, RoutingStatus::NoProjection);
                assert!(reason.contains("summarizer"));
            }
            RoutingResult::Selected { .. } => {
                panic!("Expected rejection");
            }
        }
    }

    // R2-R16: Projection with non-matching role is ignored
    #[test]
    fn test_non_matching_role_ignored() {
        let proj = test_projection("classifier");
        let packet = test_packet("translator");
        let result = Router::route(&packet, &[proj], &HardwareConstraints::default()).unwrap();

        match result {
            RoutingResult::Rejected { status, .. } => {
                assert_eq!(status, RoutingStatus::NoProjection);
            }
            RoutingResult::Selected { .. } => {
                panic!("Expected rejection for non-matching role");
            }
        }
    }
}
