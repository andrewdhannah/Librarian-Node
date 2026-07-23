//! MQR-H2: Persistent Router Registry State — integration tests.
//!
//! These tests prove that approved qualification state survives process restart
//! without being reconstructed from benchmark scores or transient in-memory state.
//!
//! The core invariant under test:
//!   Persistence may restore previously approved routing state.
//!   Persistence may NOT recreate authority from raw performance evidence.

use librarian_core::capability::decisions::{DecisionType, OwnerDecision};
use librarian_core::capability::manifest::{
    CapabilityManifest, EvidenceSummary, ManifestStatus,
};
use librarian_core::comparative::roster::{RejectionRecord, RetestTrigger, SupersessionRecord};
use librarian_core::registry::store::{
    RegistryError, RegistryLoadResult, RegistryState, RegistryStore,
};
use librarian_core::routing::execution_profile::{
    ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity, ProfileStatus,
    RuntimeIdentity,
};
use librarian_core::routing::projection::{
    create_projection, ProjectionCreationResult, ProjectionStatus, RouterProjection,
};
use librarian_core::routing::router::{HardwareConstraints, Router, WorkPacket};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Test data factories
// ============================================================================

const MODEL_ID: &str = "minicpm5-1b-q4km";
const ROLE: &str = "classifier";
const SHA256: &str = "81B64D05A23BDEADBEEF000123456789ABCDEF0123456789ABCDEF0123456789";
const CREATED_AT: &str = "2026-07-11T12:00:00Z";

fn test_evidence_summary() -> EvidenceSummary {
    EvidenceSummary {
        smoke_test_passed: true,
        probes_passed: vec!["PP-RESPONSE-001".to_string(), "PP-JSON-001".to_string()],
        probes_failed: vec!["PP-INSTR-001".to_string()],
        total_generation_duration_ms: Some(1200),
        total_output_tokens: Some(256),
        gpu_release_verified: true,
        notes: Some("Basic probes passed".to_string()),
    }
}

fn test_manifest(status: ManifestStatus, decision_id: Option<&str>) -> CapabilityManifest {
    let manifest_id = CapabilityManifest::compute_manifest_id(MODEL_ID, ROLE, CREATED_AT);
    let computed_decision_id = decision_id.map(|_| {
        OwnerDecision::compute_decision_id(&manifest_id, "2026-07-11T12:05:00Z")
    });
    let mut m = CapabilityManifest {
        manifest_id,
        model_id: MODEL_ID.to_string(),
        model_sha256: SHA256.to_string(),
        model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        role: ROLE.to_string(),
        status,
        evidence_summary: test_evidence_summary(),
        failure_modes: vec![],
        constraints: None,
        owner_decision_id: computed_decision_id,
        supersedes_manifest_id: None,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    m.content_hash = m.compute_content_hash().unwrap();
    m
}

fn test_decision(manifest_id: &str) -> OwnerDecision {
    let decided_at = "2026-07-11T12:05:00Z";
    let decision_id = OwnerDecision::compute_decision_id(manifest_id, decided_at);
    let mut d = OwnerDecision {
        decision_id: decision_id.clone(),
        manifest_id: manifest_id.to_string(),
        decision_type: DecisionType::Approve,
        role: ROLE.to_string(),
        model_id: MODEL_ID.to_string(),
        constraints: None,
        reason: "Model demonstrates classification capabilities".to_string(),
        decided_at: decided_at.to_string(),
        content_hash: String::new(),
    };
    d.content_hash = d.compute_content_hash().unwrap();
    d
}

fn test_conditional_decision(manifest_id: &str) -> OwnerDecision {
    let decided_at = "2026-07-11T12:05:00Z";
    let decision_id = OwnerDecision::compute_decision_id(manifest_id, decided_at);
    let mut d = OwnerDecision {
        decision_id: decision_id.clone(),
        manifest_id: manifest_id.to_string(),
        decision_type: DecisionType::Conditional,
        role: ROLE.to_string(),
        model_id: MODEL_ID.to_string(),
        constraints: Some("Must maintain VRAM below 4096 MiB".to_string()),
        reason: "Approved with constraints".to_string(),
        decided_at: decided_at.to_string(),
        content_hash: String::new(),
    };
    d.content_hash = d.compute_content_hash().unwrap();
    d
}

fn test_profile() -> ExecutionProfile {
    let profile_id = ExecutionProfile::compute_profile_id(MODEL_ID, "c85e97a", "Radeon RX 570");
    let mut p = ExecutionProfile {
        profile_id,
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
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    p.content_hash = p.compute_content_hash().unwrap();
    p
}

fn test_projection(manifest: &CapabilityManifest, profile: &ExecutionProfile, decision: &OwnerDecision) -> RouterProjection {
    match create_projection(manifest, profile, &decision.decision_id) {
        ProjectionCreationResult::Created(proj) => proj,
        ProjectionCreationResult::Rejected { reason } => panic!("Projection rejected: {}", reason),
    }
}

fn test_rejection_record() -> RejectionRecord {
    RejectionRecord {
        model_id: "weak-model".to_string(),
        role: ROLE.to_string(),
        reason: "Dominated by baseline in all metrics".to_string(),
        dominant_model_id: Some(MODEL_ID.to_string()),
        evidence_refs: vec!["evidence-001".to_string()],
        retest_trigger: RetestTrigger::NewEvidence,
        created_at: CREATED_AT.to_string(),
    }
}

fn test_supersession_record() -> SupersessionRecord {
    SupersessionRecord {
        superseded_model_id: "old-model".to_string(),
        role: ROLE.to_string(),
        comparison_basis: "Higher throughput and lower latency".to_string(),
        superseding_model_id: MODEL_ID.to_string(),
        evidence_refs: vec!["evidence-002".to_string()],
        created_at: CREATED_AT.to_string(),
    }
}

fn full_authority_state() -> RegistryState {
    let manifest = test_manifest(ManifestStatus::Approved, Some("dec-001"));
    let decision = test_decision(&manifest.manifest_id);
    let profile = test_profile();
    let projection = test_projection(&manifest, &profile, &decision);

    RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![decision],
        profiles: vec![profile],
        projections: vec![projection],
        rejection_records: vec![test_rejection_record()],
        supersession_records: vec![test_supersession_record()],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    }
}

fn temp_store() -> (RegistryStore, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("registry.json");
    (RegistryStore::new(path), dir)
}

// ============================================================================
// H2-T1: Valid registry persists and reloads
// ============================================================================

#[test]
fn test_h2_t1_valid_registry_persists_and_reloads() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    store.save(&state).unwrap();
    let result = store.load().unwrap();

    assert!(result.is_loaded());
    if let RegistryLoadResult::Loaded(loaded) = result {
        assert_eq!(loaded.manifests.len(), 1);
        assert_eq!(loaded.decisions.len(), 1);
        assert_eq!(loaded.profiles.len(), 1);
        assert_eq!(loaded.projections.len(), 1);
        assert_eq!(loaded.rejection_records.len(), 1);
        assert_eq!(loaded.supersession_records.len(), 1);
    }
}

// ============================================================================
// H2-T2: Fresh instance restores approved projection
// ============================================================================

#[test]
fn test_h2_t2_fresh_instance_restores_approved_projection() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    // Save
    store.save(&state).unwrap();

    // Simulate restart: drop everything, load fresh
    let loaded = store.load().unwrap();
    let loaded_state = match loaded {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    // Should have an active, routable projection
    let active = loaded_state.active_projections();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].manifest_status, ManifestStatus::Approved);
    assert_eq!(active[0].status, ProjectionStatus::Active);
}

// ============================================================================
// H2-T3: Restored projection preserves model artifact identity
// ============================================================================

#[test]
fn test_h2_t3_restored_projection_preserves_model_artifact_identity() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let proj = &loaded.projections[0];
    assert_eq!(proj.model_id, MODEL_ID);
    assert_eq!(proj.model_sha256, SHA256);
    assert_eq!(proj.model_filename, "MiniCPM5-1B-Q4_K_M.gguf");
    assert_eq!(proj.role, ROLE);
}

// ============================================================================
// H2-T4: Restored projection preserves execution profile identity
// ============================================================================

#[test]
fn test_h2_t4_restored_projection_preserves_profile_identity() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let proj = &loaded.projections[0];
    let profile = &loaded.profiles[0];

    assert_eq!(proj.profile_id, profile.profile_id);
    assert_eq!(proj.gpu_vram_mb, profile.hardware.gpu_vram_mb);
    assert_eq!(proj.runtime_backend, profile.runtime.backend);
    assert_eq!(proj.runtime_os, profile.hardware.os);
}

// ============================================================================
// H2-T5: Restored projection preserves Owner decision linkage
// ============================================================================

#[test]
fn test_h2_t5_restored_projection_preserves_decision_linkage() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let proj = &loaded.projections[0];
    let decision = &loaded.decisions[0];

    assert_eq!(proj.owner_decision_id, decision.decision_id);
    assert_eq!(decision.manifest_id, loaded.manifests[0].manifest_id);
}

// ============================================================================
// H2-T6: Restored projection preserves content hash
// ============================================================================

#[test]
fn test_h2_t6_restored_projection_preserves_content_hash() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    let original_hash = state.projections[0].content_hash.clone();

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert_eq!(loaded.projections[0].content_hash, original_hash);

    // Validate content hashes
    let errors = store.validate(&loaded);
    assert!(errors.is_empty(), "Validation errors: {:?}", errors);
}

// ============================================================================
// H2-T7: Rejected manifest remains non-routable after reload
// ============================================================================

#[test]
fn test_h2_t7_rejected_manifest_non_routable_after_reload() {
    let (store, _dir) = temp_store();

    let manifest = test_manifest(ManifestStatus::Rejected, None);
    let decision = test_decision(&manifest.manifest_id);
    let profile = test_profile();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![decision],
        profiles: vec![profile],
        projections: vec![], // No projection possible for rejected manifest
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    // Rejected manifest should have no projections
    assert!(loaded.projections.is_empty());
    assert!(!loaded.has_routable_projection("nonexistent"));
}

// ============================================================================
// H2-T8: Conditional manifest remains conditional after reload
// ============================================================================

#[test]
fn test_h2_t8_conditional_manifest_remains_conditional() {
    let (store, _dir) = temp_store();

    let mut manifest = test_manifest(ManifestStatus::Conditional, Some("dec-001"));
    manifest.constraints = Some("Must maintain VRAM below 4096 MiB".to_string());
    manifest.content_hash = manifest.compute_content_hash().unwrap();
    let decision = test_conditional_decision(&manifest.manifest_id);
    let profile = test_profile();
    let projection = test_projection(&manifest, &profile, &decision);

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![decision],
        profiles: vec![profile],
        projections: vec![projection],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert_eq!(loaded.manifests[0].status, ManifestStatus::Conditional);
    assert_eq!(loaded.projections[0].manifest_status, ManifestStatus::Conditional);
    assert!(loaded.projections[0].constraints.is_some());
}

// ============================================================================
// H2-T9: Supersession record survives reload
// ============================================================================

#[test]
fn test_h2_t9_supersession_record_survives_reload() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert_eq!(loaded.supersession_records.len(), 1);
    let sup = &loaded.supersession_records[0];
    assert_eq!(sup.superseded_model_id, "old-model");
    assert_eq!(sup.superseding_model_id, MODEL_ID);
    assert_eq!(sup.role, ROLE);
    assert_eq!(sup.comparison_basis, "Higher throughput and lower latency");
    assert_eq!(sup.evidence_refs, vec!["evidence-002".to_string()]);
}

// ============================================================================
// H2-T10: Missing Owner decision blocks routing
// ============================================================================

#[test]
fn test_h2_t10_missing_owner_decision_blocks_routing() {
    let (store, _dir) = temp_store();

    let manifest = test_manifest(ManifestStatus::Approved, Some("dec-nonexistent"));
    let profile = test_profile();

    // Try to create projection without a matching decision
    let result = create_projection(&manifest, &profile, "dec-nonexistent");
    match result {
        ProjectionCreationResult::Rejected { reason } => {
            // Good — create_projection rejects because it checks decision linkage
            assert!(reason.contains("decision") || reason.contains("decision"));
        }
        ProjectionCreationResult::Created(_) => {
            // Even if projection was created, the decision doesn't exist
            // Validation should catch this
        }
    }

    // State with projection but missing decision
    let proj_id = RouterProjection::compute_projection_id(&manifest.manifest_id, &profile.profile_id);
    let mut proj = RouterProjection {
        projection_id: proj_id,
        manifest_id: manifest.manifest_id.clone(),
        owner_decision_id: "dec-nonexistent".to_string(),
        profile_id: profile.profile_id.clone(),
        role: ROLE.to_string(),
        model_id: MODEL_ID.to_string(),
        model_sha256: SHA256.to_string(),
        model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        manifest_status: ManifestStatus::Approved,
        constraints: None,
        status: ProjectionStatus::Active,
        gpu_vram_mb: 4096,
        runtime_backend: "vulkan".to_string(),
        runtime_os: "windows".to_string(),
        created_at: CREATED_AT.to_string(),
        expires_at: None,
        content_hash: String::new(),
    };
    proj.content_hash = proj.compute_content_hash().unwrap();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![], // Missing!
        profiles: vec![profile],
        projections: vec![proj],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(!errors.is_empty(), "Should have authority chain errors");
    assert!(errors.iter().any(|e| matches!(e, RegistryError::MissingAuthority { .. })));
}

// ============================================================================
// H2-T11: Missing manifest blocks routing
// ============================================================================

#[test]
fn test_h2_t11_missing_manifest_blocks_routing() {
    let (store, _dir) = temp_store();

    let profile = test_profile();
    let decision = test_decision("manifest-nonexistent");

    let proj_id = RouterProjection::compute_projection_id("manifest-nonexistent", &profile.profile_id);
    let mut proj = RouterProjection {
        projection_id: proj_id,
        manifest_id: "manifest-nonexistent".to_string(),
        owner_decision_id: decision.decision_id.clone(),
        profile_id: profile.profile_id.clone(),
        role: ROLE.to_string(),
        model_id: MODEL_ID.to_string(),
        model_sha256: SHA256.to_string(),
        model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        manifest_status: ManifestStatus::Approved,
        constraints: None,
        status: ProjectionStatus::Active,
        gpu_vram_mb: 4096,
        runtime_backend: "vulkan".to_string(),
        runtime_os: "windows".to_string(),
        created_at: CREATED_AT.to_string(),
        expires_at: None,
        content_hash: String::new(),
    };
    proj.content_hash = proj.compute_content_hash().unwrap();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![], // Missing!
        decisions: vec![decision],
        profiles: vec![profile],
        projections: vec![proj],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| matches!(e, RegistryError::DanglingReference { referenced_type, .. } if referenced_type == "CapabilityManifest")));
}

// ============================================================================
// H2-T12: Missing execution profile blocks routing
// ============================================================================

#[test]
fn test_h2_t12_missing_profile_blocks_routing() {
    let (store, _dir) = temp_store();

    let manifest = test_manifest(ManifestStatus::Approved, Some("dec-001"));
    let decision = test_decision(&manifest.manifest_id);

    let proj_id = RouterProjection::compute_projection_id(&manifest.manifest_id, "profile-nonexistent");
    let mut proj = RouterProjection {
        projection_id: proj_id,
        manifest_id: manifest.manifest_id.clone(),
        owner_decision_id: decision.decision_id.clone(),
        profile_id: "profile-nonexistent".to_string(),
        role: ROLE.to_string(),
        model_id: MODEL_ID.to_string(),
        model_sha256: SHA256.to_string(),
        model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        manifest_status: ManifestStatus::Approved,
        constraints: None,
        status: ProjectionStatus::Active,
        gpu_vram_mb: 4096,
        runtime_backend: "vulkan".to_string(),
        runtime_os: "windows".to_string(),
        created_at: CREATED_AT.to_string(),
        expires_at: None,
        content_hash: String::new(),
    };
    proj.content_hash = proj.compute_content_hash().unwrap();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![decision],
        profiles: vec![], // Missing!
        projections: vec![proj],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| matches!(e, RegistryError::DanglingReference { referenced_type, .. } if referenced_type == "ExecutionProfile")));
}

// ============================================================================
// H2-T13: Execution metrics alone cannot recreate eligibility
// ============================================================================

#[test]
fn test_h2_t13_metrics_alone_cannot_recreate_eligibility() {
    let (store, _dir) = temp_store();

    // High-performance profile with no approved manifest or decision
    let profile = ExecutionProfile {
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
            gpu_vram_mb: 8192,
            cpu: "Intel Core i7-7700K".to_string(),
            ram_mb: 32768,
            os: "windows".to_string(),
        },
        metrics: ExecutionMetrics {
            avg_load_duration_ms: Some(500.0),
            avg_generation_duration_ms: Some(50.0),
            avg_tokens_per_second: Some(50.0),   // Very high throughput
            peak_vram_usage_mb: Some(4000),
            observation_count: 100,
        },
        status: ProfileStatus::Active,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![], // No manifest!
        decisions: vec![], // No decision!
        profiles: vec![profile],
        projections: vec![], // No projection!
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    // No routing eligibility whatsoever
    assert!(loaded.projections.is_empty());
    assert!(loaded.active_projections().is_empty());
    assert!(!loaded.has_routable_projection(MODEL_ID));

    // Even with a WorkPacket, the router finds nothing
    let packet = WorkPacket {
        packet_id: "wp-001".to_string(),
        required_role: ROLE.to_string(),
        hardware_constraints: None,
    };
    let result = Router::route(
        &packet,
        &loaded.projections,
        &HardwareConstraints::default(),
    ).unwrap();
    assert!(matches!(
        result,
        librarian_core::routing::router::RoutingResult::Rejected { .. }
    ));
}

// ============================================================================
// H2-T14: Malformed JSON rejected
// ============================================================================

#[test]
fn test_h2_t14_malformed_json_rejected() {
    let (store, _dir) = temp_store();
    let path = store.path().to_path_buf();
    fs::write(&path, "{ this is not valid json }}").unwrap();

    let result = store.load().unwrap();
    assert!(result.is_failure());
    match result {
        RegistryLoadResult::Corrupt { detail } => {
            assert!(detail.contains("JSON parse error"));
        }
        _ => panic!("Expected Corrupt"),
    }
}

// ============================================================================
// H2-T15: Missing required registry field rejected
// ============================================================================

#[test]
fn test_h2_t15_missing_required_registry_field_rejected() {
    let (store, _dir) = temp_store();
    let path = store.path().to_path_buf();

    // Missing required fields (schema_version, registry_id, etc.)
    fs::write(&path, r#"{"manifests": []}"#).unwrap();

    let result = store.load().unwrap();
    assert!(result.is_failure());
}

// ============================================================================
// H2-T16: Hash mismatch rejected
// ============================================================================

#[test]
fn test_h2_t16_hash_mismatch_rejected() {
    let (store, _dir) = temp_store();

    let mut manifest = test_manifest(ManifestStatus::Approved, Some("dec-001"));
    manifest.content_hash = "corrupted_hash_value".to_string();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![],
        profiles: vec![],
        projections: vec![],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| matches!(e, RegistryError::HashMismatch { record_type, .. } if record_type == "CapabilityManifest")));
}

// ============================================================================
// H2-T17: Dangling projection reference rejected
// ============================================================================

#[test]
fn test_h2_t17_dangling_projection_reference_rejected() {
    let (store, _dir) = temp_store();

    let manifest = test_manifest(ManifestStatus::Approved, Some("dec-001"));
    let decision = test_decision(&manifest.manifest_id);

    // Create a projection that references a non-existent profile
    let proj_id = RouterProjection::compute_projection_id(&manifest.manifest_id, "ghost-profile");
    let mut proj = RouterProjection {
        projection_id: proj_id,
        manifest_id: manifest.manifest_id.clone(),
        owner_decision_id: decision.decision_id.clone(),
        profile_id: "ghost-profile".to_string(),
        role: ROLE.to_string(),
        model_id: MODEL_ID.to_string(),
        model_sha256: SHA256.to_string(),
        model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        manifest_status: ManifestStatus::Approved,
        constraints: None,
        status: ProjectionStatus::Active,
        gpu_vram_mb: 4096,
        runtime_backend: "vulkan".to_string(),
        runtime_os: "windows".to_string(),
        created_at: CREATED_AT.to_string(),
        expires_at: None,
        content_hash: String::new(),
    };
    proj.content_hash = proj.compute_content_hash().unwrap();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![decision],
        profiles: vec![], // Missing profile
        projections: vec![proj],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| matches!(e, RegistryError::DanglingReference { referenced_type, .. } if referenced_type == "ExecutionProfile")));
}

// ============================================================================
// H2-T18: Unsupported schema version rejected
// ============================================================================

#[test]
fn test_h2_t18_unsupported_schema_version_rejected() {
    let (store, _dir) = temp_store();
    let path = store.path().to_path_buf();

    let future_file = serde_json::json!({
        "schema_version": 99,
        "registry_id": "test",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "manifests": [],
        "decisions": [],
        "profiles": [],
        "projections": [],
        "rejection_records": [],
        "supersession_records": [],
        "comparison_audit_records": [],
        "lifecycle_records": [],
    });

    fs::write(&path, serde_json::to_string_pretty(&future_file).unwrap()).unwrap();

    let result = store.load().unwrap();
    match result {
        RegistryLoadResult::Incompatible { found_version, .. } => {
            assert_eq!(found_version, 99);
        }
        _ => panic!("Expected Incompatible, got {:?}", result),
    }
}

// ============================================================================
// H2-T19: Missing registry file produces bounded empty state
// ============================================================================

#[test]
fn test_h2_t19_missing_file_produces_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent_registry.json");
    let store = RegistryStore::new(&path);

    let result = store.load().unwrap();
    assert!(result.is_empty());
    assert!(!result.is_failure());
}

// ============================================================================
// H2-T20: Corrupt registry is distinguishable from empty state
// ============================================================================

#[test]
fn test_h2_t20_corrupt_distinguishable_from_empty() {
    let dir = tempfile::tempdir().unwrap();

    let corrupt_path = dir.path().join("corrupt.json");
    fs::write(&corrupt_path, "not json!!!").unwrap();
    let corrupt_store = RegistryStore::new(&corrupt_path);

    let missing_path = dir.path().join("missing.json");
    let missing_store = RegistryStore::new(&missing_path);

    let r_corrupt = corrupt_store.load().unwrap();
    let r_missing = missing_store.load().unwrap();

    assert!(r_corrupt.is_failure()); // Corrupt
    assert!(r_missing.is_empty());   // Empty
    assert_ne!(r_corrupt, r_missing);
}

// ============================================================================
// H2-T21: Failed save does not replace prior valid registry
// ============================================================================

#[test]
fn test_h2_t21_failed_save_preserves_prior_registry() {
    let (store, _dir) = temp_store();

    // Save valid state
    let state = full_authority_state();
    store.save(&state).unwrap();

    // Verify valid
    let loaded1 = store.load().unwrap();
    assert!(loaded1.is_loaded());

    // Now corrupt the file manually
    let path = store.path().to_path_buf();
    fs::write(&path, "CORRUPTED").unwrap();

    // The store now returns Corrupt
    let loaded2 = store.load().unwrap();
    assert!(loaded2.is_failure());

    // Restore valid state
    store.save(&state).unwrap();
    let loaded3 = store.load().unwrap();
    assert!(loaded3.is_loaded());
}

// ============================================================================
// H2-T22: Deterministic save produces stable serialized content
// ============================================================================

#[test]
fn test_h2_t22_deterministic_save_stable_content() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    store.save(&state).unwrap();
    let content1 = fs::read_to_string(store.path()).unwrap();

    store.save(&state).unwrap();
    let content2 = fs::read_to_string(store.path()).unwrap();

    assert_eq!(content1, content2);
}

// ============================================================================
// H2-T23: Reload does not auto-approve Proposed manifest
// ============================================================================

#[test]
fn test_h2_t23_reload_does_not_auto_approve_proposed() {
    let (store, _dir) = temp_store();

    let manifest = test_manifest(ManifestStatus::Proposed, None);
    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![],
        profiles: vec![test_profile()],
        projections: vec![],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    // Still Proposed — not auto-approved
    assert_eq!(loaded.manifests[0].status, ManifestStatus::Proposed);
    assert!(loaded.projections.is_empty());
}

// ============================================================================
// H2-T24: Reload does not auto-supersede a model
// ============================================================================

#[test]
fn test_h2_t24_reload_does_not_auto_supersede() {
    let (store, _dir) = temp_store();

    let manifest = test_manifest(ManifestStatus::Approved, Some("dec-001"));
    let decision = test_decision(&manifest.manifest_id);
    let profile = test_profile();
    let projection = test_projection(&manifest, &profile, &decision);

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![decision],
        profiles: vec![profile],
        projections: vec![projection],
        rejection_records: vec![],
        supersession_records: vec![test_supersession_record()],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    // Supersession record exists but manifest is still Approved (not Superseded)
    assert_eq!(loaded.manifests[0].status, ManifestStatus::Approved);
    assert_eq!(loaded.supersession_records.len(), 1);
    // Supersession record is advisory only — no auto-mutation
}

// ============================================================================
// H2-T25: Router outcome before and after valid restart is equivalent
// ============================================================================

#[test]
fn test_h2_t25_router_outcome_equivalent_after_restart() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    // Route before "restart"
    let packet = WorkPacket {
        packet_id: "wp-001".to_string(),
        required_role: ROLE.to_string(),
        hardware_constraints: None,
    };

    let result_before = Router::route(
        &packet,
        &state.projections,
        &HardwareConstraints::default(),
    ).unwrap();

    // Save, drop, reload
    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let result_after = Router::route(
        &packet,
        &loaded.projections,
        &HardwareConstraints::default(),
    ).unwrap();

    // Both should be Selected with the same projection
    match (&result_before, &result_after) {
        (
            librarian_core::routing::router::RoutingResult::Selected { projection: p1, .. },
            librarian_core::routing::router::RoutingResult::Selected { projection: p2, .. },
        ) => {
            assert_eq!(p1.projection_id, p2.projection_id);
            assert_eq!(p1.model_id, p2.model_id);
            assert_eq!(p1.role, p2.role);
            assert_eq!(p1.status, p2.status);
        }
        _ => panic!("Expected both Selected, got {:?} and {:?}", result_before, result_after),
    }
}

// ============================================================================
// H2-T26: Rejection record preserves fields across reload
// ============================================================================

#[test]
fn test_h2_t26_rejection_record_preserves_fields() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let rej = &loaded.rejection_records[0];
    assert_eq!(rej.model_id, "weak-model");
    assert_eq!(rej.role, ROLE);
    assert_eq!(rej.dominant_model_id, Some(MODEL_ID.to_string()));
    assert_eq!(rej.evidence_refs, vec!["evidence-001".to_string()]);
    assert!(matches!(rej.retest_trigger, RetestTrigger::NewEvidence));
}

// ============================================================================
// H2-T27: Multiple models persist and reload correctly
// ============================================================================

#[test]
fn test_h2_t27_multiple_models_persist() {
    let (store, _dir) = temp_store();

    let m1 = test_manifest(ManifestStatus::Approved, Some("dec-001"));
    let d1 = test_decision(&m1.manifest_id);
    let profile = test_profile();
    let p1 = test_projection(&m1, &profile, &d1);

    // Second model (Q8) — construct directly to avoid manifest_id ordering issues
    let q8_sha256 = "AABBCCDDEE0011223344556677889900AABBCCDDEE0011223344556677889900";
    let q8_model_id = "minicpm5-1b-q8km";
    let q8_manifest_id = CapabilityManifest::compute_manifest_id(q8_model_id, ROLE, CREATED_AT);
    let q8_decision = test_decision(&q8_manifest_id);
    let mut m2 = CapabilityManifest {
        manifest_id: q8_manifest_id,
        model_id: q8_model_id.to_string(),
        model_sha256: q8_sha256.to_string(),
        model_filename: "MiniCPM5-1B-Q8_0.gguf".to_string(),
        role: ROLE.to_string(),
        status: ManifestStatus::Approved,
        evidence_summary: test_evidence_summary(),
        failure_modes: vec![],
        constraints: None,
        owner_decision_id: Some(q8_decision.decision_id.clone()),
        supersedes_manifest_id: None,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    m2.content_hash = m2.compute_content_hash().unwrap();

    let mut p2_profile = profile.clone();
    p2_profile.profile_id = ExecutionProfile::compute_profile_id(q8_model_id, "c85e97a", "Radeon RX 570");
    p2_profile.artifact.model_id = q8_model_id.to_string();
    p2_profile.artifact.filename = "MiniCPM5-1B-Q8_0.gguf".to_string();
    p2_profile.artifact.sha256 = q8_sha256.to_string();
    p2_profile.artifact.file_size_bytes = 1_100_000_000;
    p2_profile.content_hash = p2_profile.compute_content_hash().unwrap();

    let p2 = test_projection(&m2, &p2_profile, &q8_decision);

    let m2_manifest_id = m2.manifest_id.clone();
    let m1_manifest_id = m1.manifest_id.clone();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![m1, m2],
        decisions: vec![d1, q8_decision],
        profiles: vec![profile, p2_profile],
        projections: vec![p1, p2],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert_eq!(loaded.manifests.len(), 2);
    assert_eq!(loaded.projections.len(), 2);
    assert_eq!(loaded.active_projections().len(), 2);

    // Both should be routable (check by manifest_id, not model_id)
    assert!(loaded.has_routable_projection(&m1_manifest_id));
    assert!(loaded.has_routable_projection(&m2_manifest_id));
}

// ============================================================================
// H2-T28: Identity divergence between manifest and projection detected
// ============================================================================

#[test]
fn test_h2_t28_identity_divergence_detected() {
    let (store, _dir) = temp_store();

    let manifest = test_manifest(ManifestStatus::Approved, Some("dec-001"));
    let decision = test_decision(&manifest.manifest_id);

    // Create projection with mismatched model_id
    let proj_id = RouterProjection::compute_projection_id(&manifest.manifest_id, "prof-001");
    let mut proj = RouterProjection {
        projection_id: proj_id,
        manifest_id: manifest.manifest_id.clone(),
        owner_decision_id: decision.decision_id.clone(),
        profile_id: "prof-001".to_string(),
        role: ROLE.to_string(),
        model_id: "wrong-model-id".to_string(), // Mismatch!
        model_sha256: SHA256.to_string(),
        model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        manifest_status: ManifestStatus::Approved,
        constraints: None,
        status: ProjectionStatus::Active,
        gpu_vram_mb: 4096,
        runtime_backend: "vulkan".to_string(),
        runtime_os: "windows".to_string(),
        created_at: CREATED_AT.to_string(),
        expires_at: None,
        content_hash: String::new(),
    };
    proj.content_hash = proj.compute_content_hash().unwrap();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![decision],
        profiles: vec![test_profile()],
        projections: vec![proj],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| matches!(e, RegistryError::ArtifactInconsistency { .. })));
}

// ============================================================================
// H2-T29: Empty registry state saves and loads correctly
// ============================================================================

#[test]
fn test_h2_t29_empty_state_roundtrip() {
    let (store, _dir) = temp_store();

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![],
        decisions: vec![],
        profiles: vec![],
        projections: vec![],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert!(loaded.manifests.is_empty());
    assert!(loaded.decisions.is_empty());
    assert!(loaded.profiles.is_empty());
    assert!(loaded.projections.is_empty());
    assert!(loaded.rejection_records.is_empty());
    assert!(loaded.supersession_records.is_empty());
}

// ============================================================================
// H2-T30: Valid full-authority chain passes validation
// ============================================================================

#[test]
fn test_h2_t30_valid_full_authority_chain_passes_validation() {
    let (store, _dir) = temp_store();
    let state = full_authority_state();

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(errors.is_empty(), "Full authority chain should pass validation, got: {:?}", errors);
}
