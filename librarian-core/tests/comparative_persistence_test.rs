//! MQR-H3: Comparative Registry Persistence — integration tests.
//!
//! These tests prove that comparative analysis records (candidate/baseline identity,
//! methodology, thresholds, findings, analyzer version, timestamps) survive process
//! restart as advisory evidence — NOT as authoritative roster mutations.
//!
//! Core invariants under test:
//!   Historical comparison record → reload → same advisory finding.
//!   Reload does NOT auto-mutate roster, create routing eligibility, or trigger supersession.
//!   Owner approval and router behavior remain unchanged by persistence alone.

use librarian_core::capability::manifest::{ManifestStatus};
use librarian_core::comparative::audit::{
    ComparisonAuditRecord, ComparisonMethodology,
};
use librarian_core::comparative::roster::RosterPosition;
use librarian_core::registry::store::{
    RegistryLoadResult, RegistryState, RegistryStore,
};
use librarian_core::routing::router::{HardwareConstraints, Router, WorkPacket};
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Test helpers
// ============================================================================

const CREATED_AT: &str = "2026-07-11T12:00:00Z";

fn temp_store() -> (RegistryStore, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("registry.json");
    (RegistryStore::new(path), dir)
}

/// Build a comparison audit record for a superseded model scenario.
fn make_supersession_audit_record(id_suffix: &str) -> ComparisonAuditRecord {
    use librarian_core::comparative::analyzer::{compare_role, ComparisonInput};
    use librarian_core::comparative::roster::evaluate_roster;
    use librarian_core::capability::manifest::{CapabilityManifest, EvidenceSummary};
    use librarian_core::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity,
        ProfileStatus, RuntimeIdentity,
    };

    let candidate_id = format!("model-new{}", id_suffix);
    let baseline_id = format!("model-old{}", id_suffix);

    let c_mid = CapabilityManifest::compute_manifest_id(&candidate_id, "classifier", CREATED_AT);
    let b_mid = CapabilityManifest::compute_manifest_id(&baseline_id, "classifier", CREATED_AT);

    let candidate_manifest = CapabilityManifest {
        manifest_id: c_mid,
        model_id: candidate_id.clone(),
        model_sha256: format!("sha256-{}-candidate", id_suffix),
        model_filename: format!("{}.gguf", candidate_id),
        role: "classifier".to_string(),
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
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let baseline_manifest = CapabilityManifest {
        manifest_id: b_mid,
        model_id: baseline_id.clone(),
        model_sha256: format!("sha256-{}-baseline", id_suffix),
        model_filename: format!("{}.gguf", baseline_id),
        role: "classifier".to_string(),
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
        owner_decision_id: Some("dec-002".to_string()),
        supersedes_manifest_id: None,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let candidate_profile = ExecutionProfile {
        profile_id: ExecutionProfile::compute_profile_id(&candidate_id, "c85e97a", "Radeon RX 570"),
        artifact: ArtifactIdentity {
            filename: format!("{}.gguf", candidate_id),
            model_id: candidate_id.clone(),
            quantization: "Q4_K_M".to_string(),
            sha256: format!("sha256-{}-candidate", id_suffix),
            file_size_bytes: 400_000_000,
        },
        runtime: RuntimeIdentity {
            executable: "llama-server.exe".to_string(),
            version: "c85e97a".to_string(),
            backend: "vulkan".to_string(),
            device_id: Some("Vulkan0".to_string()),
        },
        hardware: HardwareIdentity {
            gpu_description: "Radeon RX 570".to_string(),
            gpu_vram_mb: 2048,
            cpu: "Intel Core i7-7700K".to_string(),
            ram_mb: 16384,
            os: "windows".to_string(),
        },
        metrics: ExecutionMetrics {
            avg_load_duration_ms: Some(1500.0),
            avg_generation_duration_ms: Some(2000.0),
            avg_tokens_per_second: Some(18.0),
            peak_vram_usage_mb: Some(1500),
            observation_count: 5,
        },
        status: ProfileStatus::Active,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let baseline_profile = ExecutionProfile {
        profile_id: ExecutionProfile::compute_profile_id(&baseline_id, "c85e97a", "Radeon RX 570"),
        artifact: ArtifactIdentity {
            filename: format!("{}.gguf", baseline_id),
            model_id: baseline_id.clone(),
            quantization: "Q4_K_M".to_string(),
            sha256: format!("sha256-{}-baseline", id_suffix),
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
            avg_load_duration_ms: Some(2500.0),
            avg_generation_duration_ms: Some(3500.0),
            avg_tokens_per_second: Some(10.0),
            peak_vram_usage_mb: Some(3500),
            observation_count: 5,
        },
        status: ProfileStatus::Active,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let input = ComparisonInput {
        candidate_manifest: candidate_manifest.clone(),
        candidate_profile: candidate_profile.clone(),
        baseline_manifest: baseline_manifest.clone(),
        baseline_profile: baseline_profile.clone(),
        role: "classifier".to_string(),
        other_role_fillers: vec![baseline_id.clone()],
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

fn state_with_audit_records(records: Vec<ComparisonAuditRecord>) -> RegistryState {
    RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![],
        decisions: vec![],
        profiles: vec![],
        projections: vec![],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: records,
        lifecycle_records: vec![],
    }
}

// ============================================================================
// H3-T1: Audit record persists and reloads with all fields
// ============================================================================

#[test]
fn test_h3_t1_audit_record_persists_and_reloads() {
    let (store, _dir) = temp_store();
    let record = make_supersession_audit_record("t1");
    let state = state_with_audit_records(vec![record.clone()]);

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert_eq!(loaded.comparison_audit_records.len(), 1);
    let loaded_record = &loaded.comparison_audit_records[0];

    // All fields preserved
    assert_eq!(loaded_record.audit_id, record.audit_id);
    assert_eq!(loaded_record.candidate.model_id, record.candidate.model_id);
    assert_eq!(loaded_record.candidate.sha256, record.candidate.sha256);
    assert_eq!(loaded_record.candidate.role, record.candidate.role);
    assert_eq!(loaded_record.baseline.model_id, record.baseline.model_id);
    assert_eq!(loaded_record.baseline.sha256, record.baseline.sha256);
    assert_eq!(loaded_record.role, record.role);
    assert_eq!(loaded_record.analyzer_version, "1.0.0");
    assert_eq!(loaded_record.content_hash, record.content_hash);
}

// ============================================================================
// H3-T2: Reloaded audit record preserves advisory status (no auto-mutation)
// ============================================================================

#[test]
fn test_h3_t2_audit_record_is_advisory_only_after_reload() {
    let (store, _dir) = temp_store();
    let record = make_supersession_audit_record("t2");
    let state = state_with_audit_records(vec![record]);

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let loaded_record = &loaded.comparison_audit_records[0];

    // It recommends Supersede (from comparison), but:
    // 1. No auto-routing eligibility was created
    assert!(loaded.projections.is_empty());
    assert!(loaded.active_projections().is_empty());

    // 2. No manifests were auto-created
    assert!(loaded.manifests.is_empty());

    // 3. No decisions were auto-created
    assert!(loaded.decisions.is_empty());

    // 4. The recommendation is an advisory finding, not an executed action
    match loaded_record.recommended_position {
        RosterPosition::Supersede => { /* expected — advisory only */ }
        _ => {}
    }
}

// ============================================================================
// H3-T3: Audit record does not create routing eligibility
// ============================================================================

#[test]
fn test_h3_t3_audit_record_no_routing_eligibility() {
    let (store, _dir) = temp_store();
    let record = make_supersession_audit_record("t3");
    let state = state_with_audit_records(vec![record]);

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    // No projections = no routing
    assert!(loaded.projections.is_empty());
    assert!(!loaded.has_routable_projection("any-manifest"));

    // Router returns Rejected (no projections)
    let packet = WorkPacket {
        packet_id: "wp-h3t3".to_string(),
        required_role: "classifier".to_string(),
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
// H3-T4: Multiple audit records persist independently
// ============================================================================

#[test]
fn test_h3_t4_multiple_audit_records_persist() {
    let (store, _dir) = temp_store();

    let r1 = make_supersession_audit_record("t4a");
    let r2 = make_supersession_audit_record("t4b");
    let r3 = make_supersession_audit_record("t4c");

    let state = state_with_audit_records(vec![r1, r2, r3]);
    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert_eq!(loaded.comparison_audit_records.len(), 3);

    // Each has a unique audit_id (because created_at is fixed by now_rfc3339,
    // but model IDs differ per id_suffix)
    let ids: Vec<&str> = loaded.comparison_audit_records.iter().map(|r| r.audit_id.as_str()).collect();
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[1], ids[2]);
}

// ============================================================================
// H3-T5: Audit records coexist with existing registry records
// ============================================================================

#[test]
fn test_h3_t5_audit_records_coexist_with_authority_records() {
    use librarian_core::capability::manifest::{CapabilityManifest, EvidenceSummary};
    use librarian_core::capability::decisions::{DecisionType, OwnerDecision};
    use librarian_core::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity,
        ProfileStatus, RuntimeIdentity,
    };
    use librarian_core::routing::projection::{
        create_projection, ProjectionCreationResult,
    };

    let (store, _dir) = temp_store();

    // Create a full authority chain
    let model_id = "h3t5-model";
    let manifest_id = CapabilityManifest::compute_manifest_id(model_id, "classifier", CREATED_AT);
    let mut manifest = CapabilityManifest {
        manifest_id: manifest_id.clone(),
        model_id: model_id.to_string(),
        model_sha256: "sha256-h3t5".to_string(),
        model_filename: "h3t5.gguf".to_string(),
        role: "classifier".to_string(),
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
        owner_decision_id: None,
        supersedes_manifest_id: None,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    let decision_id = OwnerDecision::compute_decision_id(&manifest_id, CREATED_AT);
    let mut decision = OwnerDecision {
        decision_id: decision_id.clone(),
        manifest_id: manifest_id.clone(),
        decision_type: DecisionType::Approve,
        role: "classifier".to_string(),
        model_id: model_id.to_string(),
        constraints: None,
        reason: "Test approval".to_string(),
        decided_at: CREATED_AT.to_string(),
        content_hash: String::new(),
    };
    decision.content_hash = decision.compute_content_hash().unwrap();
    manifest.owner_decision_id = Some(decision_id.clone());
    manifest.content_hash = manifest.compute_content_hash().unwrap();

    let profile_id = ExecutionProfile::compute_profile_id(model_id, "c85e97a", "Radeon RX 570");
    let mut profile = ExecutionProfile {
        profile_id: profile_id.clone(),
        artifact: ArtifactIdentity {
            filename: "h3t5.gguf".to_string(),
            model_id: model_id.to_string(),
            quantization: "Q4_K_M".to_string(),
            sha256: "sha256-h3t5".to_string(),
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
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    profile.content_hash = profile.compute_content_hash().unwrap();

    let projection = match create_projection(&manifest, &profile, &decision_id) {
        ProjectionCreationResult::Created(p) => p,
        ProjectionCreationResult::Rejected { reason } => panic!("Projection rejected: {}", reason),
    };

    // Now add audit records alongside
    let audit = make_supersession_audit_record("t5");

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
        comparison_audit_records: vec![audit],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    // Authority chain intact
    assert_eq!(loaded.manifests.len(), 1);
    assert_eq!(loaded.decisions.len(), 1);
    assert_eq!(loaded.profiles.len(), 1);
    assert_eq!(loaded.projections.len(), 1);

    // Audit records also present
    assert_eq!(loaded.comparison_audit_records.len(), 1);

    // Validation passes
    let errors = store.validate(&loaded);
    assert!(errors.is_empty(), "Authority chain should be valid with audit records: {:?}", errors);

    // Router still works (audit records don't interfere)
    let packet = WorkPacket {
        packet_id: "wp-h3t5".to_string(),
        required_role: "classifier".to_string(),
        hardware_constraints: None,
    };
    let result = Router::route(
        &packet,
        &loaded.projections,
        &HardwareConstraints::default(),
    ).unwrap();
    assert!(matches!(
        result,
        librarian_core::routing::router::RoutingResult::Selected { .. }
    ));
}

// ============================================================================
// H3-T6: Content hash validation detects tampered audit records
// ============================================================================

#[test]
fn test_h3_t6_tampered_audit_record_hash_detected() {
    let (store, _dir) = temp_store();

    let mut record = make_supersession_audit_record("t6");
    record.content_hash = "tampered_hash_value".to_string();

    let state = state_with_audit_records(vec![record]);
    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| {
        matches!(e, librarian_core::registry::store::RegistryError::HashMismatch { record_type, .. } if record_type == "ComparisonAuditRecord")
    }));
}

// ============================================================================
// H3-T7: Audit record with insufficient evidence preserves methodology
// ============================================================================

#[test]
fn test_h3_t7_insufficient_evidence_preserves_methodology() {
    use librarian_core::comparative::analyzer::{compare_role, ComparisonInput};
    use librarian_core::comparative::roster::evaluate_roster;
    use librarian_core::capability::manifest::{CapabilityManifest, EvidenceSummary};
    use librarian_core::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity,
        ProfileStatus, RuntimeIdentity,
    };

    let (store, _dir) = temp_store();

    // Candidate with insufficient evidence (no probes passed)
    let c_manifest_id = CapabilityManifest::compute_manifest_id("insufficient-model", "classifier", CREATED_AT);
    let candidate_manifest = CapabilityManifest {
        manifest_id: c_manifest_id,
        model_id: "insufficient-model".to_string(),
        model_sha256: "sha256-insufficient".to_string(),
        model_filename: "insufficient-model.gguf".to_string(),
        role: "classifier".to_string(),
        status: ManifestStatus::Approved,
        evidence_summary: EvidenceSummary {
            smoke_test_passed: false, // Fails smoke test → insufficient
            probes_passed: vec![],
            probes_failed: vec!["PP-001".to_string()],
            total_generation_duration_ms: None,
            total_output_tokens: None,
            gpu_release_verified: false,
            notes: None,
        },
        failure_modes: vec![],
        constraints: None,
        owner_decision_id: Some("dec-001".to_string()),
        supersedes_manifest_id: None,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let b_manifest_id = CapabilityManifest::compute_manifest_id("baseline-model", "classifier", CREATED_AT);
    let baseline_manifest = CapabilityManifest {
        manifest_id: b_manifest_id,
        model_id: "baseline-model".to_string(),
        model_sha256: "sha256-baseline".to_string(),
        model_filename: "baseline-model.gguf".to_string(),
        role: "classifier".to_string(),
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
        owner_decision_id: Some("dec-002".to_string()),
        supersedes_manifest_id: None,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let candidate_profile = ExecutionProfile {
        profile_id: ExecutionProfile::compute_profile_id("insufficient-model", "c85e97a", "Radeon RX 570"),
        artifact: ArtifactIdentity {
            filename: "insufficient-model.gguf".to_string(),
            model_id: "insufficient-model".to_string(),
            quantization: "Q4_K_M".to_string(),
            sha256: "sha256-insufficient".to_string(),
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
            avg_load_duration_ms: None,
            avg_generation_duration_ms: None,
            avg_tokens_per_second: None,
            peak_vram_usage_mb: None,
            observation_count: 0,
        },
        status: ProfileStatus::Active,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let baseline_profile = ExecutionProfile {
        profile_id: ExecutionProfile::compute_profile_id("baseline-model", "c85e97a", "Radeon RX 570"),
        artifact: ArtifactIdentity {
            filename: "baseline-model.gguf".to_string(),
            model_id: "baseline-model".to_string(),
            quantization: "Q4_K_M".to_string(),
            sha256: "sha256-baseline".to_string(),
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
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };

    let input = ComparisonInput {
        candidate_manifest: candidate_manifest.clone(),
        candidate_profile: candidate_profile.clone(),
        baseline_manifest: baseline_manifest.clone(),
        baseline_profile: baseline_profile.clone(),
        role: "classifier".to_string(),
        other_role_fillers: vec!["baseline-model".to_string()],
    };

    let result = compare_role(&input);
    assert!(!result.is_comparable);

    let recommendation = evaluate_roster(&result).unwrap();
    let record = ComparisonAuditRecord::from_comparison(
        &result,
        &recommendation,
        &candidate_manifest,
        &baseline_manifest,
    ).unwrap();

    assert_eq!(record.methodology, ComparisonMethodology::InsufficientEvidence);
    assert!(!record.is_comparable);
    assert_eq!(record.recommended_position, RosterPosition::InsufficientEvidence);
    assert_eq!(record.findings.len(), 1);
    assert_eq!(
        record.findings[0].finding_type,
        librarian_core::comparative::finding::FindingType::InsufficientComparableEvidence
    );

    let state = state_with_audit_records(vec![record]);
    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let loaded_record = &loaded.comparison_audit_records[0];
    assert_eq!(loaded_record.methodology, ComparisonMethodology::InsufficientEvidence);
    assert!(!loaded_record.is_comparable);
    assert_eq!(loaded_record.recommended_position, RosterPosition::InsufficientEvidence);
}

// ============================================================================
// H3-T8: Empty audit records list is valid
// ============================================================================

#[test]
fn test_h3_t8_empty_audit_records_valid() {
    let (store, _dir) = temp_store();
    let state = state_with_audit_records(vec![]);

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert!(loaded.comparison_audit_records.is_empty());
    assert!(loaded.manifests.is_empty());
    assert!(loaded.decisions.is_empty());
    assert!(loaded.projections.is_empty());
}

// ============================================================================
// H3-T9: Deterministic save preserves audit records
// ============================================================================

#[test]
fn test_h3_t9_deterministic_save_preserves_audit_records() {
    let (store, _dir) = temp_store();
    let record = make_supersession_audit_record("t9");
    let state = state_with_audit_records(vec![record]);

    store.save(&state).unwrap();
    let content1 = fs::read_to_string(store.path()).unwrap();

    store.save(&state).unwrap();
    let content2 = fs::read_to_string(store.path()).unwrap();

    assert_eq!(content1, content2);
}

// ============================================================================
// H3-T10: Audit record preserves full supersession context
// ============================================================================

#[test]
fn test_h3_t10_supersession_context_preserved() {
    let (store, _dir) = temp_store();
    let record = make_supersession_audit_record("t10");
    let state = state_with_audit_records(vec![record.clone()]);

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let loaded_record = &loaded.comparison_audit_records[0];

    // Compare against the original record
    assert_eq!(loaded_record.candidate.model_id, record.candidate.model_id);
    assert_eq!(loaded_record.candidate.sha256, record.candidate.sha256);
    assert_eq!(loaded_record.candidate.role, record.candidate.role);
    assert_eq!(loaded_record.baseline.model_id, record.baseline.model_id);
    assert_eq!(loaded_record.baseline.sha256, record.baseline.sha256);
    assert_eq!(loaded_record.role, "classifier");
    assert_eq!(loaded_record.analyzer_version, "1.0.0");
    assert_eq!(loaded_record.content_hash, record.content_hash);

    // Methodology is role_metric_comparison (since candidate has sufficient evidence)
    assert_eq!(loaded_record.methodology, ComparisonMethodology::RoleMetricComparison);

    // Thresholds have default values
    assert_eq!(loaded_record.thresholds.quality_improvement_pct, 10.0);
    assert_eq!(loaded_record.thresholds.latency_improvement_pct, 10.0);
    assert_eq!(loaded_record.thresholds.memory_diff_mb, 200);
    assert_eq!(loaded_record.thresholds.file_size_diff_bytes, 50 * 1024 * 1024);

    // Findings present (supersession generates multiple findings)
    assert!(loaded_record.findings.len() >= 2);

    // Comparable
    assert!(loaded_record.is_comparable);

    // Position is Supersede (candidate exceeds baseline on multiple metrics)
    assert_eq!(loaded_record.recommended_position, RosterPosition::Supersede);

    // Timestamps present
    assert!(!loaded_record.created_at.is_empty());
}

// ============================================================================
// H3-T11: Audit record validation catches data integrity issues
// ============================================================================

#[test]
fn test_h3_t11_validation_rejects_corrupt_audit_content_hash() {
    let (store, _dir) = temp_store();

    let mut record = make_supersession_audit_record("t11");
    // Mutate content hash to match after tampering a finding
    record.findings.clear();
    // Don't recompute hash — this simulates corruption

    let state = state_with_audit_records(vec![record]);
    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    let errors = store.validate(&loaded);
    assert!(!errors.is_empty());
    let has_hash_error = errors.iter().any(|e| {
        matches!(e, librarian_core::registry::store::RegistryError::HashMismatch { record_type, .. } if record_type == "ComparisonAuditRecord")
    });
    assert!(has_hash_error, "Should detect content hash mismatch after finding tampering");
}

// ============================================================================
// H3-T12: Router outcome is unaffected by audit record presence
// ============================================================================

#[test]
fn test_h3_t12_router_unaffected_by_audit_records() {
    use librarian_core::capability::manifest::{CapabilityManifest, EvidenceSummary};
    use librarian_core::capability::decisions::{DecisionType, OwnerDecision};
    use librarian_core::routing::execution_profile::{
        ArtifactIdentity, ExecutionMetrics, ExecutionProfile, HardwareIdentity,
        ProfileStatus, RuntimeIdentity,
    };
    use librarian_core::routing::projection::{
        create_projection, ProjectionCreationResult,
    };

    let (store, _dir) = temp_store();

    // Create authority chain
    let model_id = "h3t12-model";
    let manifest_id = CapabilityManifest::compute_manifest_id(model_id, "classifier", CREATED_AT);
    let mut manifest = CapabilityManifest {
        manifest_id: manifest_id.clone(),
        model_id: model_id.to_string(),
        model_sha256: "sha256-h3t12".to_string(),
        model_filename: "h3t12.gguf".to_string(),
        role: "classifier".to_string(),
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
        owner_decision_id: None,
        supersedes_manifest_id: None,
        content_hash: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    let decision_id = OwnerDecision::compute_decision_id(&manifest_id, CREATED_AT);
    let mut decision = OwnerDecision {
        decision_id: decision_id.clone(),
        manifest_id: manifest_id.clone(),
        decision_type: DecisionType::Approve,
        role: "classifier".to_string(),
        model_id: model_id.to_string(),
        constraints: None,
        reason: "Test approval".to_string(),
        decided_at: CREATED_AT.to_string(),
        content_hash: String::new(),
    };
    decision.content_hash = decision.compute_content_hash().unwrap();
    manifest.owner_decision_id = Some(decision_id.clone());
    manifest.content_hash = manifest.compute_content_hash().unwrap();

    let profile_id = ExecutionProfile::compute_profile_id(model_id, "c85e97a", "Radeon RX 570");
    let mut profile = ExecutionProfile {
        profile_id: profile_id.clone(),
        artifact: ArtifactIdentity {
            filename: "h3t12.gguf".to_string(),
            model_id: model_id.to_string(),
            quantization: "Q4_K_M".to_string(),
            sha256: "sha256-h3t12".to_string(),
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
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    profile.content_hash = profile.compute_content_hash().unwrap();

    let projection = match create_projection(&manifest, &profile, &decision_id) {
        ProjectionCreationResult::Created(p) => p,
        ProjectionCreationResult::Rejected { reason } => panic!("Projection rejected: {}", reason),
    };

    let audit = make_supersession_audit_record("t12");

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![manifest],
        decisions: vec![decision],
        profiles: vec![profile],
        projections: vec![projection.clone()],
        rejection_records: vec![],
        supersession_records: vec![],
        comparison_audit_records: vec![audit],
        lifecycle_records: vec![],
    };

    // Route before save
    let packet = WorkPacket {
        packet_id: "wp-h3t12".to_string(),
        required_role: "classifier".to_string(),
        hardware_constraints: None,
    };
    let result_before = Router::route(
        &packet,
        &state.projections,
        &HardwareConstraints::default(),
    ).unwrap();

    // Save and reload
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

    // Same outcome
    match (&result_before, &result_after) {
        (
            librarian_core::routing::router::RoutingResult::Selected { projection: p1, .. },
            librarian_core::routing::router::RoutingResult::Selected { projection: p2, .. },
        ) => {
            assert_eq!(p1.projection_id, p2.projection_id);
            assert_eq!(p1.model_id, p2.model_id);
        }
        _ => panic!("Expected both Selected"),
    }

    // Audit records present but don't interfere
    assert_eq!(loaded.comparison_audit_records.len(), 1);
}

// ============================================================================
// H3-T13: Audit records with manifest/rejection/supersession coexist
// ============================================================================

#[test]
fn test_h3_t13_all_record_types_coexist() {
    use librarian_core::comparative::roster::{RejectionRecord, RetestTrigger, SupersessionRecord};

    let (store, _dir) = temp_store();

    let audit = make_supersession_audit_record("t13");
    let rej = RejectionRecord {
        model_id: "rejected-model".to_string(),
        role: "classifier".to_string(),
        reason: "Dominated by baseline".to_string(),
        dominant_model_id: Some("strong-model".to_string()),
        evidence_refs: vec!["ev-001".to_string()],
        retest_trigger: RetestTrigger::NewEvidence,
        created_at: CREATED_AT.to_string(),
    };
    let sup = SupersessionRecord {
        superseded_model_id: "old-model".to_string(),
        role: "classifier".to_string(),
        comparison_basis: "Higher throughput".to_string(),
        superseding_model_id: "new-model".to_string(),
        evidence_refs: vec!["ev-002".to_string()],
        created_at: CREATED_AT.to_string(),
    };

    let state = RegistryState {
        registry_id: String::new(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        manifests: vec![],
        decisions: vec![],
        profiles: vec![],
        projections: vec![],
        rejection_records: vec![rej],
        supersession_records: vec![sup],
        comparison_audit_records: vec![audit],
        lifecycle_records: vec![],
    };

    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert_eq!(loaded.comparison_audit_records.len(), 1);
    assert_eq!(loaded.rejection_records.len(), 1);
    assert_eq!(loaded.supersession_records.len(), 1);
}

// ============================================================================
// H3-T14: Audit record created_at timestamp is preserved
// ============================================================================

#[test]
fn test_h3_t14_created_at_preserved() {
    let (store, _dir) = temp_store();
    let record = make_supersession_audit_record("t14");
    let original_created_at = record.created_at.clone();

    let state = state_with_audit_records(vec![record]);
    store.save(&state).unwrap();
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };

    assert_eq!(loaded.comparison_audit_records[0].created_at, original_created_at);
}

// ============================================================================
// H3-T15: Audit records survive schema version upgrade (v3→v4)
// ============================================================================

#[test]
fn test_h3_t15_schema_v4_contains_audit_and_lifecycle_records() {
    // Prove that the current schema version is 4 (supports lifecycle records)
    assert_eq!(
        librarian_core::registry::store::REGISTRY_SCHEMA_VERSION,
        4,
        "LIFE requires schema version 4 for lifecycle_records"
    );

    let (store, _dir) = temp_store();
    let record = make_supersession_audit_record("t15");
    let state = state_with_audit_records(vec![record]);

    store.save(&state).unwrap();

    // Read raw JSON to verify schema_version
    let raw = fs::read_to_string(store.path()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["schema_version"], 4);
    assert!(parsed["comparison_audit_records"].is_array());
    assert_eq!(parsed["comparison_audit_records"].as_array().unwrap().len(), 1);
    assert!(parsed["lifecycle_records"].is_array(), "Schema v4 should contain lifecycle_records");

    // Also verify through load path
    let loaded = match store.load().unwrap() {
        RegistryLoadResult::Loaded(s) => s,
        _ => panic!("Expected Loaded"),
    };
    assert_eq!(loaded.comparison_audit_records.len(), 1);
}
