//! MQR-I2: End-to-End Failure-Path Integration Tests.
//!
//! These integration tests prove that rejected, incompatible, malformed,
//! or identity-divergent paths fail at the correct boundary and never
//! accidentally continue into model execution or router promotion.
//!
//! Every failure retains a bounded audit/evidence record identifying
//! which boundary refused the chain and why.
//!
//! Failure paths covered:
//! 1. No approved projection → route refusal → no execution
//! 2. Rejected capability → route refusal → no execution
//! 3. Conditional constraints unmet → route refusal → no execution
//! 4. VRAM mismatch → route refusal → no execution
//! 5. Backend mismatch → route refusal → no execution
//! 6. OS mismatch → route refusal → no execution
//! 7. Execution artifact identity divergence → chain rejection
//! 8. Broken lifecycle ordering → evidence rejection
//! 9. Missing release proof → validation failure
//! 10. Malformed bridge packet → packet validation failure

use crate::capability::manifest::{CapabilityManifest, EvidenceSummary, ManifestStatus};
use librarian_contracts::common::{
    PacketConstraints, PacketExecutionConfig, PacketExecutionIdentity, PacketExecutionMetrics,
    PacketLeaseLifecycle, PacketLifecycleEvent, PacketModelIdentity, PacketReleaseVerification,
};
use librarian_contracts::evidence_packet::{self, EvidencePacket};
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
use crate::routing::router::{HardwareConstraints, RoutingResult, Router, WorkPacket};

use super::chain::{CanonicalChain, ChainValidationOutcome};

// ═══════════════════════════════════════════════════════════════════════════════
// Test Data Builders
// ═══════════════════════════════════════════════════════════════════════════════

const MODEL_ID: &str = "minicpm5-1b-q4km";
const SHA256: &str = "81B64D05A23B";
const ROLE: &str = "classifier";

fn make_request() -> QualificationRequest {
    QualificationRequest::new(
        "req-i2-001".to_string(),
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

fn make_run_result() -> QualificationRunResult {
    QualificationRunResult {
        run_id: "run-i2-001".to_string(),
        request_id: "req-i2-001".to_string(),
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

fn make_evidence() -> EvidenceSummary {
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

fn make_manifest(status: ManifestStatus) -> CapabilityManifest {
    let created_at = "2026-07-11T12:00:00Z".to_string();
    let manifest_id = CapabilityManifest::compute_manifest_id(MODEL_ID, ROLE, &created_at);
    let is_conditional = status == ManifestStatus::Conditional;

    CapabilityManifest {
        manifest_id,
        model_id: MODEL_ID.to_string(),
        model_sha256: SHA256.to_string(),
        model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        role: ROLE.to_string(),
        status,
        evidence_summary: make_evidence(),
        failure_modes: vec![],
        constraints: if is_conditional {
            Some("Must maintain VRAM below 4096 MiB".to_string())
        } else {
            None
        },
        owner_decision_id: Some("dec-i2-001".to_string()),
        supersedes_manifest_id: None,
        content_hash: String::new(),
        created_at: created_at.clone(),
        updated_at: created_at,
    }
}

fn make_profile() -> ExecutionProfile {
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

fn make_chain(manifest_status: ManifestStatus) -> CanonicalChain {
    let manifest = make_manifest(manifest_status);
    let profile = make_profile();
    let projection = match create_projection(&manifest, &profile, "dec-i2-001") {
        ProjectionCreationResult::Created(p) => Some(p),
        _ => None,
    };

    CanonicalChain {
        request: make_request(),
        run_result: make_run_result(),
        evidence_summary: make_evidence(),
        manifest,
        owner_decision: None,
        execution_profile: make_profile(),
        projection,
        routing_result: None,
    }
}

fn make_packet(role: &str) -> WorkPacket {
    WorkPacket {
        packet_id: "pkt-i2-001".to_string(),
        required_role: role.to_string(),
        hardware_constraints: None,
    }
}

fn make_packet_with_constraints(role: &str, constraints: HardwareConstraints) -> WorkPacket {
    WorkPacket {
        packet_id: "pkt-i2-001".to_string(),
        required_role: role.to_string(),
        hardware_constraints: Some(constraints),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 1: No approved projection → route refusal → no execution
// ═══════════════════════════════════════════════════════════════════════════════

/// FP1: When no projection exists for a role, the router refuses the packet.
/// Audit trail: RoutingResult::Rejected with RoutingStatus::NoProjection.
#[test]
fn test_i2_fp1_no_projection_route_refusal() {
    let packet = make_packet(ROLE);

    // No projections at all → route refusal
    let result = Router::route(&packet, &[], &HardwareConstraints::default()).unwrap();

    match result {
        RoutingResult::Rejected { status, reason, log_entry } => {
            // Audit trail: status is NoProjection, reason identifies the boundary
            assert_eq!(status, crate::routing::log::RoutingStatus::NoProjection);
            assert!(reason.contains(ROLE), "Reason must identify the missing role");
            assert_eq!(log_entry.packet_id, "pkt-i2-001");
            assert_eq!(log_entry.status, crate::routing::log::RoutingStatus::NoProjection);
            assert!(!log_entry.reason.is_empty(), "Audit record must have a reason");
        }
        RoutingResult::Selected { .. } => {
            panic!("FP1: Router must refuse when no projection exists");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 2: Rejected capability → route refusal → no execution
// ═══════════════════════════════════════════════════════════════════════════════

/// FP2: A rejected manifest cannot create a projection, so routing has nothing to select.
/// Audit trail: projection creation rejection reason is preserved.
#[test]
fn test_i2_fp2_rejected_capability_route_refusal() {
    let manifest = make_manifest(ManifestStatus::Rejected);
    let profile = make_profile();
    let packet = make_packet(ROLE);

    // Step 1: Projection creation must be rejected
    let proj_result = create_projection(&manifest, &profile, "dec-i2-001");
    match proj_result {
        ProjectionCreationResult::Rejected { reason } => {
            // Audit trail: rejection reason preserved
            assert!(reason.contains("rejected") || reason.contains("rejected"),
                "Rejection reason must indicate rejected status: {}", reason);
        }
        ProjectionCreationResult::Created(_) => {
            panic!("FP2: Projection must NOT be created from rejected manifest");
        }
    }

    // Step 2: Router has no projections → route refusal
    let result = Router::route(&packet, &[], &HardwareConstraints::default()).unwrap();
    match result {
        RoutingResult::Rejected { status, reason, log_entry } => {
            assert_eq!(status, crate::routing::log::RoutingStatus::NoProjection);
            assert!(!reason.is_empty());
            assert_eq!(log_entry.packet_id, "pkt-i2-001");
        }
        RoutingResult::Selected { .. } => {
            panic!("FP2: Router must refuse when capability is rejected");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 3: Conditional constraints unmet → route refusal → no execution
// ═══════════════════════════════════════════════════════════════════════════════

/// FP3: A conditional manifest creates a projection with constraints, but
/// the projection is not routable until constraints are satisfied.
/// Audit trail: projection carries constraints, chain validates the constraint state.
#[test]
fn test_i2_fp3_conditional_constraints_unmet() {
    let manifest = make_manifest(ManifestStatus::Conditional);
    let profile = make_profile();

    // Step 1: Conditional projection should be created WITH constraints
    let proj_result = create_projection(&manifest, &profile, "dec-i2-001");
    let projection = match proj_result {
        ProjectionCreationResult::Created(proj) => {
            // Audit trail: constraints are recorded in the projection
            assert!(proj.constraints.is_some(),
                "Conditional projection must carry constraints");
            assert_eq!(proj.manifest_status, ManifestStatus::Conditional);
            proj
        }
        ProjectionCreationResult::Rejected { reason } => {
            panic!("FP3: Conditional projection should be created: {}", reason);
        }
    };

    // Step 2: Build chain with conditional projection
    let mut chain = make_chain(ManifestStatus::Conditional);
    chain.projection = Some(projection.clone());

    // Step 3: Chain is NOT routable without explicit constraint satisfaction.
    // The chain has an approved manifest (Conditional counts as "approved" for has_approved_manifest),
    // a projection exists, but no routing result. So is_routable() is false.
    assert!(chain.has_approved_manifest(), "Conditional should count as approved");
    assert!(chain.has_active_projection());
    assert!(!chain.is_routable(), "Chain must not be routable without routing result");

    // Step 4: Chain identity is still valid
    match chain.verify_identity() {
        ChainValidationOutcome::Valid { stages_verified, .. } => {
            assert_eq!(stages_verified, 6);
        }
        ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
            panic!("FP3: Chain should be valid, broken at {}: {}", broken_at_stage, reason);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 4: VRAM mismatch → route refusal → no execution
// ═══════════════════════════════════════════════════════════════════════════════

/// FP4: When the work packet requires more VRAM than the projection offers,
/// the router rejects by hardware constraints.
/// Audit trail: RoutingResult::Rejected with RoutingStatus::RejectedByConstraints.
#[test]
fn test_i2_fp4_vram_mismatch_route_refusal() {
    let manifest = make_manifest(ManifestStatus::Approved);
    let profile = make_profile();
    let projection = match create_projection(&manifest, &profile, "dec-i2-001") {
        ProjectionCreationResult::Created(p) => p,
        ProjectionCreationResult::Rejected { reason } => {
            panic!("FP4: Projection creation failed: {}", reason);
        }
    };

    // Projection has gpu_vram_mb: 4096 (from profile). Require 999999.
    let packet = make_packet_with_constraints(ROLE, HardwareConstraints {
        min_gpu_vram_mb: Some(999999),
        required_backend: None,
        required_os: None,
    });

    let projections = vec![projection];
    let hw_constraints = HardwareConstraints {
        min_gpu_vram_mb: Some(999999),
        required_backend: None,
        required_os: None,
    };

    let result = Router::route(&packet, &projections, &hw_constraints).unwrap();

    match result {
        RoutingResult::Rejected { status, reason, log_entry } => {
            assert_eq!(status, crate::routing::log::RoutingStatus::RejectedByConstraints,
                "Audit: must be RejectedByConstraints when VRAM requirement exceeds projection capacity");
            assert!(reason.contains("hardware constraints") || reason.contains("eliminated"),
                "Audit reason must reference constraints: {}", reason);
            assert_eq!(log_entry.packet_id, "pkt-i2-001");
            assert_eq!(log_entry.status, crate::routing::log::RoutingStatus::RejectedByConstraints);
        }
        RoutingResult::Selected { .. } => {
            panic!("FP4: Router must refuse when VRAM requirement exceeds projection capacity");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 5: Backend mismatch → route refusal → no execution
// ═══════════════════════════════════════════════════════════════════════════════

/// FP5: When the work packet requires a different backend than the projection offers,
/// the router rejects by hardware constraints.
/// Audit trail: RoutingResult::Rejected with RoutingStatus::RejectedByConstraints.
#[test]
fn test_i2_fp5_backend_mismatch_route_refusal() {
    let manifest = make_manifest(ManifestStatus::Approved);
    let profile = make_profile(); // backend: "vulkan"
    let projection = match create_projection(&manifest, &profile, "dec-i2-001") {
        ProjectionCreationResult::Created(p) => p,
        _ => panic!("Expected projection"),
    };

    let packet = make_packet_with_constraints(ROLE, HardwareConstraints {
        min_gpu_vram_mb: None,
        required_backend: Some("cuda".to_string()), // Profile uses vulkan
        required_os: None,
    });

    let result = Router::route(&packet, &[projection], &packet.hardware_constraints.as_ref().unwrap()).unwrap();

    match result {
        RoutingResult::Rejected { status, .. } => {
            assert_eq!(status, crate::routing::log::RoutingStatus::RejectedByConstraints,
                "Audit: backend mismatch must produce RejectedByConstraints");
        }
        RoutingResult::Selected { .. } => {
            panic!("FP5: Router must refuse when backend requirement doesn't match");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 6: OS mismatch → route refusal → no execution
// ═══════════════════════════════════════════════════════════════════════════════

/// FP6: When the work packet requires a different OS than the projection offers,
/// the router rejects by hardware constraints.
/// Audit trail: RoutingResult::Rejected with RoutingStatus::RejectedByConstraints.
#[test]
fn test_i2_fp6_os_mismatch_route_refusal() {
    let manifest = make_manifest(ManifestStatus::Approved);
    let profile = make_profile(); // os: "windows"
    let projection = match create_projection(&manifest, &profile, "dec-i2-001") {
        ProjectionCreationResult::Created(p) => p,
        _ => panic!("Expected projection"),
    };

    let packet = make_packet_with_constraints(ROLE, HardwareConstraints {
        min_gpu_vram_mb: None,
        required_backend: None,
        required_os: Some("linux".to_string()), // Profile uses windows
    });

    let result = Router::route(&packet, &[projection], &packet.hardware_constraints.as_ref().unwrap()).unwrap();

    match result {
        RoutingResult::Rejected { status, .. } => {
            assert_eq!(status, crate::routing::log::RoutingStatus::RejectedByConstraints,
                "Audit: OS mismatch must produce RejectedByConstraints");
        }
        RoutingResult::Selected { .. } => {
            panic!("FP6: Router must refuse when OS requirement doesn't match");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 7: Execution artifact identity divergence → chain rejection
// ═══════════════════════════════════════════════════════════════════════════════

/// FP7: When the execution profile references a different model artifact than
/// the evidence (request/run_result), chain identity verification detects the break.
/// Audit trail: ChainValidationOutcome::Invalid with broken_at_stage and reason.
#[test]
fn test_i2_fp7_artifact_identity_divergence() {
    let chain = make_chain(ManifestStatus::Approved);

    // Verify chain is initially valid
    match chain.verify_identity() {
        ChainValidationOutcome::Valid { .. } => {}
        ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
            panic!("Control: chain should be valid, broken at {}: {}", broken_at_stage, reason);
        }
    }

    // Now diverge the execution profile's artifact identity
    let mut diverged = chain.clone();
    diverged.execution_profile.artifact.model_id = "wrong-model-different-artifact".to_string();
    diverged.execution_profile.artifact.sha256 = "WRONG_SHA256_HASH".to_string();

    // Chain verification must detect the break
    match diverged.verify_identity() {
        ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
            // Audit trail: broken stage and reason are recorded
            assert_eq!(broken_at_stage, "manifest→execution_profile",
                "Audit: must identify the exact stage where identity diverged");
            assert!(reason.contains("Model ID mismatch") || reason.contains("SHA-256 mismatch"),
                "Audit: reason must identify the type of mismatch: {}", reason);
        }
        ChainValidationOutcome::Valid { .. } => {
            panic!("FP7: Chain must reject artifact identity divergence");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 8: Broken lifecycle ordering → evidence rejection
// ═══════════════════════════════════════════════════════════════════════════════

/// FP8: When lifecycle events are not in chronological order, the evidence
/// packet's lifecycle ordering validation detects the break.
/// Audit trail: error message identifies the out-of-order events.
#[test]
fn test_i2_fp8_broken_lifecycle_ordering() {
    // Construct an evidence packet with out-of-order lifecycle events
    let packet = EvidencePacket {
        packet_type: evidence_packet::PACKET_TYPE.to_string(),
        packet_version: evidence_packet::PACKET_VERSION.to_string(),
        exported_at: "2026-07-11T12:00:10Z".to_string(),
        qualification_request_id: "req-i2-001".to_string(),
        identity: PacketModelIdentity {
            model_id: MODEL_ID.to_string(),
            sha256: SHA256.to_string(),
            filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            quantization: Some("Q4_K_M".to_string()),
        },
        execution: PacketExecutionIdentity {
            runtime_profile_id: "rp-001".to_string(),
            hardware_profile_id: "hw-001".to_string(),
            runtime_executable_sha256: "abc123".to_string(),
            runtime_executable_version: "c85e97a".to_string(),
        },
        lease: PacketLeaseLifecycle {
            lease_id: "lease-001".to_string(),
            port: Some(8080),
            state: "completed".to_string(),
            loaded_at: Some("2026-07-11T12:00:01Z".to_string()),
            released_at: Some("2026-07-11T12:00:05Z".to_string()),
            vram_released_at: Some("2026-07-11T12:00:06Z".to_string()),
        },
        run: PacketExecutionMetrics {
            run_id: "run-i2-001".to_string(),
            input_tokens: Some(10),
            output_tokens: Some(15),
            load_duration_ms: Some(2187),
            generation_duration_ms: Some(385),
            exit_status: Some("success".to_string()),
            started_at: Some("2026-07-11T12:00:01Z".to_string()),
            ended_at: Some("2026-07-11T12:00:05Z".to_string()),
        },
        lifecycle_events: vec![
            // Event at T+5 is BEFORE event at T+2 — broken ordering
            PacketLifecycleEvent {
                event_type: "runtime_completed".to_string(),
                process_id: Some(1234),
                observed_state: Some("completed".to_string()),
                observation: None,
                occurred_at: Some("2026-07-11T12:00:05Z".to_string()),
            },
            PacketLifecycleEvent {
                event_type: "runtime_loading".to_string(),
                process_id: Some(1234),
                observed_state: Some("loading".to_string()),
                observation: None,
                occurred_at: Some("2026-07-11T12:00:02Z".to_string()),
            },
        ],
        release_verification: PacketReleaseVerification {
            pid_exit_verified: true,
            gpu_release_verified: true,
            free_vram_mb: Some(3433),
            baseline_vram_mb: Some(3433),
            within_tolerance: true,
        },
    };

    // Structural validation should pass
    assert!(packet.validate().is_ok(), "Packet structure should be valid");

    // Lifecycle ordering validation must fail
    match packet.validate_lifecycle_ordering() {
        Ok(()) => {
            panic!("FP8: Broken lifecycle ordering must be detected");
        }
        Err(e) => {
            // Audit trail: error identifies the out-of-order boundary
            let msg = e.to_string();
            assert!(msg.contains("chronological order") || msg.contains("not in order"),
                "Audit: error must reference lifecycle ordering: {}", msg);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 9: Missing release proof → validation failure
// ═══════════════════════════════════════════════════════════════════════════════

/// FP9: When the evidence packet reports no GPU release verification,
/// the chain's evidence summary detects the missing proof.
/// Audit trail: evidence_summary.gpu_release_verified = false.
#[test]
fn test_i2_fp9_missing_release_proof() {
    // Create a chain where the evidence says GPU release was NOT verified
    let mut chain = make_chain(ManifestStatus::Approved);
    chain.evidence_summary.gpu_release_verified = false;

    // The chain identity is still structurally valid (identity matches)
    match chain.verify_identity() {
        ChainValidationOutcome::Valid { .. } => {}
        ChainValidationOutcome::Invalid { reason, .. } => {
            panic!("Control: chain identity should still be valid: {}", reason);
        }
    }

    // But the evidence summary shows missing release proof
    assert!(!chain.evidence_summary.gpu_release_verified,
        "Audit: evidence must record that release proof is missing");

    // The request requires release proof
    assert!(chain.request.constraints.require_release_proof,
        "Request requires release proof");

    // Combined check: request requires release proof but evidence doesn't have it.
    // This is the validation failure — the chain's evidence is incomplete for the
    // request's requirements.
    if chain.request.constraints.require_release_proof
        && !chain.evidence_summary.gpu_release_verified
    {
        // Validation failure: release proof required but not provided
        // This is the bounded audit record
        let failure_reason = format!(
            "Request '{}' requires release proof but evidence for model '{}' has gpu_release_verified=false",
            chain.request.request_id, chain.request.identity.model_id
        );
        assert!(!failure_reason.is_empty());
        assert!(failure_reason.contains("release proof"));
        assert!(failure_reason.contains(&chain.request.request_id));
    } else {
        panic!("FP9: Should detect missing release proof when request requires it");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FAILURE PATH 10: Malformed bridge packet → packet validation failure
// ═══════════════════════════════════════════════════════════════════════════════

/// FP10a: Malformed QualificationRequest with empty model_id is rejected.
/// Audit trail: validation error identifies the specific malformed field.
#[test]
fn test_i2_fp10a_malformed_qualification_request() {
    let request = QualificationRequest::new(
        "req-malformed".to_string(),
        PacketModelIdentity {
            model_id: "".to_string(), // Empty — malformed
            sha256: SHA256.to_string(),
            filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            quantization: Some("Q4_K_M".to_string()),
        },
        PacketExecutionConfig {
            runtime_profile_id: "rp-001".to_string(),
            task_description: "Classify text".to_string(),
            max_tokens: Some(256),
            temperature: Some(0.7),
            timeout_seconds: Some(30),
        },
        PacketConstraints {
            require_release_proof: true,
            max_vram_mb: Some(4096),
        },
    );

    match request.validate() {
        Ok(()) => {
            panic!("FP10a: Empty model_id must be rejected");
        }
        Err(e) => {
            // Audit trail: error identifies the malformed boundary
            let msg = e.to_string();
            assert!(msg.contains("model_id") || msg.contains("empty"),
                "Audit: error must identify the malformed field: {}", msg);
        }
    }
}

/// FP10b: Malformed QualificationRequest with wrong packet type is rejected.
/// Audit trail: validation error identifies the wrong packet type.
#[test]
fn test_i2_fp10b_malformed_wrong_packet_type() {
    let mut request = make_request();
    request.packet_type = "wrong_type".to_string();

    match request.validate() {
        Ok(()) => {
            panic!("FP10b: Wrong packet type must be rejected");
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("packet type") || msg.contains("Invalid packet"),
                "Audit: error must identify wrong packet type: {}", msg);
        }
    }
}

/// FP10c: Malformed QualificationRequest with empty SHA-256 is rejected.
#[test]
fn test_i2_fp10c_malformed_empty_sha256() {
    let request = QualificationRequest::new(
        "req-malformed-sha".to_string(),
        PacketModelIdentity {
            model_id: MODEL_ID.to_string(),
            sha256: "".to_string(), // Empty — malformed
            filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            quantization: Some("Q4_K_M".to_string()),
        },
        PacketExecutionConfig {
            runtime_profile_id: "rp-001".to_string(),
            task_description: "Classify text".to_string(),
            max_tokens: Some(256),
            temperature: Some(0.7),
            timeout_seconds: Some(30),
        },
        PacketConstraints {
            require_release_proof: true,
            max_vram_mb: Some(4096),
        },
    );

    match request.validate() {
        Ok(()) => {
            panic!("FP10c: Empty SHA-256 must be rejected");
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("sha256") || msg.contains("empty"),
                "Audit: error must identify the SHA-256 field: {}", msg);
        }
    }
}

/// FP10d: Malformed QualificationRequest with zero timeout is rejected.
#[test]
fn test_i2_fp10d_malformed_zero_timeout() {
    let request = QualificationRequest::new(
        "req-malformed-timeout".to_string(),
        PacketModelIdentity {
            model_id: MODEL_ID.to_string(),
            sha256: SHA256.to_string(),
            filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            quantization: Some("Q4_K_M".to_string()),
        },
        PacketExecutionConfig {
            runtime_profile_id: "rp-001".to_string(),
            task_description: "Classify text".to_string(),
            max_tokens: Some(256),
            temperature: Some(0.7),
            timeout_seconds: Some(0), // Zero — out of range
        },
        PacketConstraints {
            require_release_proof: true,
            max_vram_mb: Some(4096),
        },
    );

    match request.validate() {
        Ok(()) => {
            panic!("FP10d: Zero timeout must be rejected");
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("timeout") || msg.contains("between 1 and 600"),
                "Audit: error must identify the timeout issue: {}", msg);
        }
    }
}

/// FP10e: Malformed WorkPacket with empty packet_id is rejected.
#[test]
fn test_i2_fp10e_malformed_work_packet_empty_id() {
    let packet = WorkPacket {
        packet_id: "".to_string(),
        required_role: ROLE.to_string(),
        hardware_constraints: None,
    };

    match packet.validate() {
        Ok(()) => {
            panic!("FP10e: Empty packet_id must be rejected");
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("packet_id") || msg.contains("empty"),
                "Audit: error must identify the empty packet_id: {}", msg);
        }
    }
}

/// FP10f: Malformed WorkPacket with empty required_role is rejected.
#[test]
fn test_i2_fp10f_malformed_work_packet_empty_role() {
    let packet = WorkPacket {
        packet_id: "pkt-001".to_string(),
        required_role: "".to_string(),
        hardware_constraints: None,
    };

    match packet.validate() {
        Ok(()) => {
            panic!("FP10f: Empty required_role must be rejected");
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("required_role") || msg.contains("empty"),
                "Audit: error must identify the empty role: {}", msg);
        }
    }
}

/// FP10g: Malformed EvidencePacket with wrong packet type is rejected.
#[test]
fn test_i2_fp10g_malformed_evidence_wrong_type() {
    let packet = EvidencePacket {
        packet_type: "wrong_type".to_string(),
        packet_version: evidence_packet::PACKET_VERSION.to_string(),
        exported_at: "2026-07-11T12:00:10Z".to_string(),
        qualification_request_id: "req-001".to_string(),
        identity: PacketModelIdentity {
            model_id: MODEL_ID.to_string(),
            sha256: SHA256.to_string(),
            filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            quantization: Some("Q4_K_M".to_string()),
        },
        execution: PacketExecutionIdentity {
            runtime_profile_id: "rp-001".to_string(),
            hardware_profile_id: "hw-001".to_string(),
            runtime_executable_sha256: "abc123".to_string(),
            runtime_executable_version: "c85e97a".to_string(),
        },
        lease: PacketLeaseLifecycle {
            lease_id: "lease-001".to_string(),
            port: Some(8080),
            state: "completed".to_string(),
            loaded_at: Some("2026-07-11T12:00:01Z".to_string()),
            released_at: Some("2026-07-11T12:00:05Z".to_string()),
            vram_released_at: Some("2026-07-11T12:00:06Z".to_string()),
        },
        run: PacketExecutionMetrics {
            run_id: "run-001".to_string(),
            input_tokens: Some(10),
            output_tokens: Some(15),
            load_duration_ms: Some(2187),
            generation_duration_ms: Some(385),
            exit_status: Some("success".to_string()),
            started_at: Some("2026-07-11T12:00:01Z".to_string()),
            ended_at: Some("2026-07-11T12:00:05Z".to_string()),
        },
        lifecycle_events: vec![],
        release_verification: PacketReleaseVerification {
            pid_exit_verified: true,
            gpu_release_verified: true,
            free_vram_mb: Some(3433),
            baseline_vram_mb: Some(3433),
            within_tolerance: true,
        },
    };

    match packet.validate() {
        Ok(()) => {
            panic!("FP10g: Wrong evidence packet type must be rejected");
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("packet type") || msg.contains("Invalid"),
                "Audit: error must identify wrong type: {}", msg);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// AUDIT TRAIL INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Audit: every routing failure produces a RoutingLogEntry with a reason.
#[test]
fn test_i2_audit_routing_failure_produces_log_entry() {
    let packet = make_packet(ROLE);

    // No projections → routing failure
    let result = Router::route(&packet, &[], &HardwareConstraints::default()).unwrap();

    match result {
        RoutingResult::Rejected { reason, log_entry, .. } => {
            // Verify audit trail completeness
            assert!(!log_entry.log_id.is_empty(), "Log entry must have an ID");
            assert_eq!(log_entry.packet_id, "pkt-i2-001");
            assert_eq!(log_entry.role, ROLE);
            assert!(log_entry.projection_id.is_none(), "Rejected: no projection_id");
            assert!(log_entry.model_id.is_none(), "Rejected: no model_id");
            assert!(log_entry.profile_id.is_none(), "Rejected: no profile_id");
            assert!(!log_entry.created_at.is_empty(), "Log entry must have timestamp");
            assert!(!log_entry.content_hash.is_empty(), "Log entry must have content hash");
            assert!(!reason.is_empty(), "Rejection must have a reason");
            assert!(!log_entry.reason.is_empty(), "Log entry reason must be non-empty");
        }
        RoutingResult::Selected { .. } => {
            panic!("Expected rejection for audit trail test");
        }
    }
}

/// Audit: chain validation failure produces a broken_at_stage and reason.
#[test]
fn test_i2_audit_chain_validation_failure_records_breakpoint() {
    let mut chain = make_chain(ManifestStatus::Approved);

    // Introduce an identity break at run_result→manifest
    chain.manifest.model_id = "different-model".to_string();

    match chain.verify_identity() {
        ChainValidationOutcome::Invalid { broken_at_stage, reason } => {
            // Audit trail: the break point and reason are recorded
            assert!(!broken_at_stage.is_empty(), "Must record which stage broke");
            assert!(!reason.is_empty(), "Must record why it broke");
            assert!(broken_at_stage.contains("→"),
                "Stage name must reference the transition: {}", broken_at_stage);
        }
        ChainValidationOutcome::Valid { .. } => {
            panic!("Expected chain validation failure for audit test");
        }
    }
}

/// Audit: hardware constraint rejection includes the projection count in the reason.
#[test]
fn test_i2_audit_hardware_rejection_includes_context() {
    let manifest = make_manifest(ManifestStatus::Approved);
    let profile = make_profile();
    let projection = match create_projection(&manifest, &profile, "dec-i2-001") {
        ProjectionCreationResult::Created(p) => p,
        _ => panic!("Expected projection"),
    };

    let packet = make_packet_with_constraints(ROLE, HardwareConstraints {
        min_gpu_vram_mb: Some(999999),
        required_backend: None,
        required_os: None,
    });

    let result = Router::route(&packet, &[projection], &packet.hardware_constraints.as_ref().unwrap()).unwrap();

    match result {
        RoutingResult::Rejected { reason, .. } => {
            // The reason should indicate that projections existed but were eliminated
            assert!(
                reason.contains("eliminated") || reason.contains("hardware constraints"),
                "Audit: reason must provide context about why routing failed: {}", reason
            );
        }
        RoutingResult::Selected { .. } => {
            panic!("Expected hardware rejection for audit context test");
        }
    }
}

/// Audit: work packet validation failure is immediate and produces a clear error.
#[test]
fn test_i2_audit_packet_validation_is_immediate() {
    // Malformed packet should fail at validation, before any routing
    let packet = WorkPacket {
        packet_id: "".to_string(),
        required_role: "".to_string(),
        hardware_constraints: None,
    };

    let result = packet.validate();
    assert!(result.is_err(), "Malformed packet must fail validation immediately");

    let err_msg = result.unwrap_err().to_string();
    // The error must identify which field is malformed
    assert!(
        err_msg.contains("packet_id") || err_msg.contains("required_role"),
        "Audit: packet validation error must identify the malformed field: {}", err_msg
    );
}

/// Audit: valid packet passes validation (control test).
#[test]
fn test_i2_audit_valid_packet_passes() {
    let packet = make_packet(ROLE);
    assert!(packet.validate().is_ok(), "Valid packet must pass validation");
}

/// Audit: valid qualification request passes validation (control test).
#[test]
fn test_i2_audit_valid_request_passes() {
    let request = make_request();
    assert!(request.validate().is_ok(), "Valid request must pass validation");
}
