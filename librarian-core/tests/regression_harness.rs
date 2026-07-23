//! MQR-REGRESSION-HARNESS-1 — Long-term regression harness for sealed MQR baseline.
//!
//! Provides:
//! 1. Reusable fixture framework covering the complete qualification lifecycle
//! 2. End-to-end deterministic replay verification
//! 3. Permanent authority boundary regression suite
//! 4. Failure scenario coverage
//! 5. Baseline compatibility validation
//!
//! These tests protect the sealed MQR baseline against future regressions.
//! They do NOT introduce new qualification behavior.

use librarian_core::qualification::run_state::RunState;
use librarian_core::routing::log::RoutingStatus;

// ============================================================================
// Fixture Framework — reusable test data across the qualification lifecycle
// ============================================================================

mod fixtures {
    use librarian_core::observability::models::ObservabilityReport;
    use librarian_core::observability::service::ObservabilityService;
    use librarian_core::provenance::builder::ProvenanceBuilder;
    use librarian_core::provenance::models::EvidenceProvenance;
    use librarian_core::qualification::batch::{
        BatchQualificationResult, IndividualBatchResult,
    };
    use librarian_core::qualification::custom_executor::{
        CustomRuleEvidence, CustomRuleOutcome,
    };
    use librarian_core::qualification::run_result::{
        GenerationSettings, QualificationRunResult, RuntimeTelemetry,
    };
    use librarian_core::qualification::run_state::RunState;
    use librarian_core::review::builder::ReviewBuilder;
    use librarian_core::review::models::ReviewPackage;

    /// Create a qualification run result with the given state.
    pub fn run_result(model_id: &str, state: RunState, tokens: u32) -> QualificationRunResult {
        QualificationRunResult {
            run_id: format!("run-{}", model_id),
            request_id: format!("req-{}", model_id),
            model_id: model_id.to_string(),
            model_sha256: format!("sha256-{}", model_id),
            model_filename: format!("{}.gguf", model_id),
            task_pack_id: format!("tp-{}", model_id),
            fixture_hash: "abc123".to_string(),
            state,
            raw_output: Some(format!("output from {}", model_id)),
            settings: GenerationSettings {
                runtime_profile_id: format!("prof-{}", model_id),
                max_tokens: Some(256),
                temperature: Some(0.0),
                timeout_seconds: Some(120),
                task_description: format!("task for {}", model_id),
            },
            telemetry: RuntimeTelemetry {
                port: Some(9000),
                process_id: Some(12345),
                load_duration_ms: Some(1000),
                generation_duration_ms: Some(100),
                input_tokens: Some(10),
                output_tokens: Some(tokens),
                http_status: Some(200),
                runtime_error: None,
            },
            lifecycle_events: vec![],
            error_message: None,
            custom_evidence: vec![],
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: Some("2026-01-01T00:00:01Z".to_string()),
        }
    }

    /// Create a custom rule evidence record.
    pub fn custom_evidence(rule_id: &str, passed: bool, timed_out: bool, panicked: bool) -> CustomRuleEvidence {
        CustomRuleEvidence {
            rule_id: rule_id.to_string(),
            version: "1.0.0".to_string(),
            outcome: CustomRuleOutcome {
                passed,
                message: None,
                execution_duration_ms: if timed_out || panicked { None } else { Some(10) },
                timed_out,
                panicked,
            },
            task_pack_id: "tp-001".to_string(),
            content_hash: format!("hash-{}", rule_id),
            executed_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    /// Create an observability report from run results.
    pub fn observability_report(results: &[QualificationRunResult]) -> ObservabilityReport {
        ObservabilityService::report(results, None)
    }

    /// Create provenance from a run result.
    pub fn provenance(result: &QualificationRunResult) -> EvidenceProvenance {
        ProvenanceBuilder::from_run_result(result)
    }

    /// Create provenance records from multiple run results.
    pub fn provenance_records(results: &[QualificationRunResult]) -> Vec<EvidenceProvenance> {
        results.iter().map(|r| ProvenanceBuilder::from_run_result(r)).collect()
    }

    /// Create a batch result with per-target states.
    pub fn batch_result(results: Vec<QualificationRunResult>) -> BatchQualificationResult {
        let completed = results.iter().filter(|r| r.state.is_success()).count();
        let failed = results.iter().filter(|r| r.state.is_failure()).count();
        let total_targets = results.len();
        let evidence_refs: Vec<String> = results.iter()
            .filter(|r| r.state.is_success())
            .map(|r| format!("run:{}:{}", r.run_id, r.model_id)).collect();
        let total_duration: u64 = results.iter().filter_map(|r| r.telemetry.generation_duration_ms).sum();
        let model_order: Vec<String> = results.iter().map(|r| r.model_id.clone()).collect();

        BatchQualificationResult {
            batch_id: "batch-regression".to_string(),
            model_order,
            individual_results: results.into_iter().enumerate().map(|(i, r)| {
                IndividualBatchResult {
                    position: i,
                    model_id: r.model_id.clone(),
                    result: r.clone(),
                    state: r.state.clone(),
                    error_message: r.error_message.clone(),
                }
            }).collect(),
            aggregate: librarian_core::qualification::batch::AggregateBatchSummary {
                total_targets,
                completed,
                failed,
                total_duration_ms: total_duration,
                evidence_references: evidence_refs,
                content_hash: "batch-hash".to_string(),
            },
            started_at: "2026-01-01T00:00:00Z".to_string(),
            completed_at: "2026-01-01T00:00:01Z".to_string(),
            content_hash: "batch-result-hash".to_string(),
        }
    }

    /// Create a review package from run results + provenance.
    pub fn review_package(results: &[QualificationRunResult], prov: &[EvidenceProvenance]) -> ReviewPackage {
        ReviewBuilder::build(results, prov, None)
    }

    /// Create a review package with batch context.
    pub fn review_package_with_batch(
        results: &[QualificationRunResult],
        prov: &[EvidenceProvenance],
        batch: &BatchQualificationResult,
    ) -> ReviewPackage {
        ReviewBuilder::build(results, prov, Some(batch))
    }
}

// ============================================================================
// End-to-End Replay Tests
// ============================================================================

/// RH-T1: Same input produces same qualification run result
#[test]
fn test_rh_t1_replay_qualification_run() {
    let r1 = fixtures::run_result("model-a", RunState::Completed, 50);
    let r2 = fixtures::run_result("model-a", RunState::Completed, 50);
    assert_eq!(r1.run_id, r2.run_id);
    assert_eq!(r1.model_id, r2.model_id);
    assert_eq!(r1.state, r2.state);
    assert_eq!(r1.telemetry.output_tokens, r2.telemetry.output_tokens);
}

/// RH-T2: Same evidence produces same observability summaries
#[test]
fn test_rh_t2_replay_observability() {
    let r1 = fixtures::run_result("model-a", RunState::Completed, 50);
    let r2 = fixtures::run_result("model-a", RunState::Completed, 50);

    let report1 = fixtures::observability_report(&[r1]);
    let report2 = fixtures::observability_report(&[r2]);

    assert_eq!(report1.runs.len(), report2.runs.len());
    assert_eq!(report1.health.total_runs, report2.health.total_runs);
}

/// RH-T3: Same provenance produces same review package content hash
#[test]
fn test_rh_t3_replay_review_package() {
    let results = vec![
        fixtures::run_result("model-a", RunState::Completed, 50),
    ];
    let prov = fixtures::provenance_records(&results);

    let pkg1 = fixtures::review_package(&results, &prov);
    let pkg2 = fixtures::review_package(&results, &prov);

    assert_eq!(pkg1.content_hash, pkg2.content_hash);
}

// ============================================================================
// Determinism Verification
// ============================================================================

/// RH-T4: Custom evidence content hash is deterministic
#[test]
fn test_rh_t4_evidence_hash_deterministic() {
    let e1 = fixtures::custom_evidence("CR-001", true, false, false);
    let e2 = fixtures::custom_evidence("CR-001", true, false, false);
    assert_eq!(e1.compute_content_hash().unwrap(), e2.compute_content_hash().unwrap());
}

/// RH-T5: Observability report is deterministic for same inputs
#[test]
fn test_rh_t5_observability_deterministic() {
    let results = vec![
        fixtures::run_result("model-a", RunState::Completed, 50),
        fixtures::run_result("model-b", RunState::Completed, 100),
    ];
    let report1 = fixtures::observability_report(&results);
    let report2 = fixtures::observability_report(&results);

    // Content hashes from evidence summary view
    assert_eq!(report1.evidence.provenance_refs, report2.evidence.provenance_refs);
    assert_eq!(report1.health.total_runs, report2.health.total_runs);
}

/// RH-T6: Provenance lineage hash is deterministic
#[test]
fn test_rh_t6_provenance_deterministic() {
    let result = fixtures::run_result("model-a", RunState::Completed, 50);
    let p1 = fixtures::provenance(&result);
    let p2 = fixtures::provenance(&result);
    assert_eq!(p1.lineage_hash, p2.lineage_hash);
}

// ============================================================================
// Authority Boundary Regression Suite
// ============================================================================

/// RH-T7: Evidence cannot approve capabilities (structural proof)
#[test]
fn test_rh_t7_evidence_no_approval() {
    let evidence = fixtures::custom_evidence("CR-001", true, false, false);
    let json = serde_json::to_value(&evidence).unwrap();
    assert!(json.get("manifest_id").is_none(), "Evidence has no manifest_id");
    assert!(json.get("decision_id").is_none(), "Evidence has no decision_id");
    assert!(json.get("approved").is_none(), "Evidence has no approved field");
    assert!(json.get("rejected").is_none(), "Evidence has no rejected field");
    assert!(json.get("router_eligible").is_none(), "Evidence has no router_eligible");
}

/// RH-T8: Review packages cannot create decisions (structural proof)
#[test]
fn test_rh_t8_review_no_decisions() {
    let results = vec![fixtures::run_result("model-a", RunState::Completed, 50)];
    let prov = fixtures::provenance_records(&results);
    let pkg = fixtures::review_package(&results, &prov);

    assert!(pkg.assert_no_capability_data());
    let json = serde_json::to_value(&pkg).unwrap();
    assert!(json.get("manifest_id").is_none(), "Review has no manifest_id");
    assert!(json.get("decision_id").is_none(), "Review has no decision_id");
    assert!(json.get("approved").is_none(), "Review has no approved");
    assert!(json.get("router_eligible").is_none(), "Review has no router_eligible");
    assert!(json.get("projection_id").is_none(), "Review has no projection_id");
}

/// RH-T9: Provenance cannot mutate routing (structural proof)
#[test]
fn test_rh_t9_provenance_no_routing() {
    let result = fixtures::run_result("model-a", RunState::Completed, 50);
    let prov = fixtures::provenance(&result);
    let json = serde_json::to_value(&prov).unwrap();
    assert!(json.get("projection_id").is_none(), "Provenance has no projection_id");
    assert!(json.get("router_eligible").is_none(), "Provenance has no router_eligible");
    assert!(json.get("routing_status").is_none(), "Provenance has no routing_status");
}

/// RH-T10: Observability cannot modify qualification state (structural proof)
#[test]
fn test_rh_t10_observability_no_mutation() {
    let results = vec![fixtures::run_result("model-a", RunState::Completed, 50)];
    let report = fixtures::observability_report(&results);

    // Observability report is a new struct, not a mutation of source data
    // Source data is unchanged
    assert_eq!(results[0].state, RunState::Completed);
    assert_eq!(results[0].telemetry.generation_duration_ms, Some(100));

    // Report has no capability fields
    let json = serde_json::to_value(&report).unwrap();
    assert!(json.get("manifest_id").is_none(), "Observability has no manifest_id");
    assert!(json.get("decision_id").is_none(), "Observability has no decision_id");
}

/// RH-T11: Batch aggregation cannot bypass gates (structural proof)
#[test]
fn test_rh_t11_batch_no_bypass() {
    let results = vec![
        fixtures::run_result("model-a", RunState::Completed, 50),
        fixtures::run_result("model-b", RunState::ModelFailed, 0),
    ];
    let batch = fixtures::batch_result(results);

    let json = serde_json::to_value(&batch).unwrap();
    assert!(json.get("manifest_id").is_none(), "Batch has no manifest_id");
    assert!(json.get("decision_id").is_none(), "Batch has no decision_id");
    assert!(json.get("approved").is_none(), "Batch has no approved");
    assert!(json.get("router_eligible").is_none(), "Batch has no router_eligible");
    assert!(json.get("projection_id").is_none(), "Batch has no projection_id");

    // Batch preserves the failure
    assert_eq!(batch.aggregate.failed, 1);
    assert_eq!(batch.aggregate.completed, 1);
}

// ============================================================================
// Failure Scenario Coverage
// ============================================================================

/// RH-T12: Timeout evidence preserves timeout flag
#[test]
fn test_rh_t12_timeout_evidence() {
    let evidence = fixtures::custom_evidence("CR-TIMEOUT", false, true, false);
    assert!(!evidence.outcome.passed);
    assert!(evidence.outcome.timed_out);
    assert!(!evidence.outcome.panicked);
}

/// RH-T13: Panic evidence preserves panic flag
#[test]
fn test_rh_t13_panic_evidence() {
    let evidence = fixtures::custom_evidence("CR-PANIC", false, false, true);
    assert!(!evidence.outcome.passed);
    assert!(!evidence.outcome.timed_out);
    assert!(evidence.outcome.panicked);
}

/// RH-T14: Missing provenance is detectable in fixture data
#[test]
fn test_rh_t14_missing_provenance_detectable() {
    let result = fixtures::run_result("model-a", RunState::Completed, 50);
    let mut prov = fixtures::provenance(&result);
    let missing = prov.detect_missing_provenance();
    assert!(missing.is_empty(), "Complete provenance should have no missing fields: {:?}", missing);

    // Tamper with source
    prov.source.model_id = String::new();
    let missing2 = prov.detect_missing_provenance();
    assert!(!missing2.is_empty(), "Should detect missing model_id");
}

/// RH-T15: Batch partial failure is preserved in observability
#[test]
fn test_rh_t15_batch_partial_failure() {
    let results = vec![
        fixtures::run_result("model-good", RunState::Completed, 50),
        fixtures::run_result("model-bad", RunState::ModelFailed, 0),
        fixtures::run_result("model-good-2", RunState::Completed, 100),
    ];
    let batch = fixtures::batch_result(results);
    assert_eq!(batch.aggregate.completed, 2);
    assert_eq!(batch.aggregate.failed, 1);
    assert_eq!(batch.aggregate.total_targets, 3);

    // Evidence refs only from completed
    assert_eq!(batch.aggregate.evidence_references.len(), 2);
}

/// RH-T16: Run result with failure state preserves error
#[test]
fn test_rh_t16_failure_state_preserved() {
    let r = fixtures::run_result("model-fail", RunState::RuntimeFailed, 0);
    assert!(r.state.is_failure());
    assert!(!r.state.is_success());
}

// ============================================================================
// Baseline Compatibility Validation
// ============================================================================

/// RH-T17: Complete lifecycle round-trip — model → run → evidence → observability → provenance → review
#[test]
fn test_rh_t17_complete_lifecycle_roundtrip() {
    // 1. Model runs
    let results = vec![
        fixtures::run_result("model-a", RunState::Completed, 50),
        fixtures::run_result("model-b", RunState::Completed, 100),
    ];

    // 2. Evidence collection (add custom evidence to one run)
    let mut results_with_ev = results.clone();
    results_with_ev[0].custom_evidence = vec![
        fixtures::custom_evidence("CR-SMOKE", true, false, false),
    ];

    // 3. Observability
    let report = fixtures::observability_report(&results_with_ev);
    assert_eq!(report.runs.len(), 2);
    assert_eq!(report.evidence.total_custom_evidence, 1);

    // 4. Provenance
    let prov = fixtures::provenance_records(&results_with_ev);
    assert_eq!(prov.len(), 2);
    assert!(prov[0].verify_lineage_hash());
    assert!(prov[1].verify_lineage_hash());

    // 5. Review package
    let pkg = fixtures::review_package(&results_with_ev, &prov);
    assert_eq!(pkg.qualification.total_runs, 2);
    assert!(pkg.assert_no_capability_data());

    // 6. Batch context
    let batch_results = vec![
        fixtures::run_result("model-x", RunState::Completed, 50),
        fixtures::run_result("model-y", RunState::Completed, 75),
    ];
    let batch = fixtures::batch_result(batch_results);
    assert_eq!(batch.aggregate.completed, 2);

    // 7. Review with batch
    let batch_prov = fixtures::provenance_records(&results_with_ev);
    let pkg_with_batch = fixtures::review_package_with_batch(&results_with_ev, &batch_prov, &batch);
    assert!(pkg_with_batch.batch.is_some());
    assert_eq!(pkg_with_batch.batch.as_ref().unwrap().total_targets, 2);
}

/// RH-T18: Deterministic hash chain across the full pipeline
#[test]
fn test_rh_t18_deterministic_hash_chain() {
    // Model → Run → Evidence → Observer → Provenance → Review
    let r1 = fixtures::run_result("model-a", RunState::Completed, 50);
    let r2 = fixtures::run_result("model-a", RunState::Completed, 50);

    let mut ev1 = r1.clone();
    ev1.custom_evidence = vec![fixtures::custom_evidence("CR-001", true, false, false)];
    let mut ev2 = r2.clone();
    ev2.custom_evidence = vec![fixtures::custom_evidence("CR-001", true, false, false)];

    // Each layer produces deterministic output
    let report1 = fixtures::observability_report(&[ev1.clone()]);
    let report2 = fixtures::observability_report(&[ev2.clone()]);
    assert_eq!(report1.evidence.evidence_hashes, report2.evidence.evidence_hashes);

    let prov1 = fixtures::provenance_records(&[ev1.clone()]);
    let prov2 = fixtures::provenance_records(&[ev2.clone()]);
    assert_eq!(prov1[0].lineage_hash, prov2[0].lineage_hash);

    let pkg1 = fixtures::review_package(&[ev1], &prov1);
    let pkg2 = fixtures::review_package(&[ev2], &prov2);
    assert_eq!(pkg1.content_hash, pkg2.content_hash);
}

/// RH-T19: All status enums have valid round-trip
#[test]
fn test_rh_t19_status_enum_roundtrips() {
    use librarian_core::capability::manifest::ManifestStatus;
    use librarian_core::routing::projection::ProjectionStatus;
    use librarian_core::qualification::validator_engine::RuleSeverity;

    // RunState
    let states = [
        RunState::Received, RunState::FixtureResolved, RunState::LoadingRuntime,
        RunState::Executing, RunState::Completed, RunState::RunnerFailed,
        RunState::ModelFailed, RunState::RuntimeFailed, RunState::Timeout,
    ];
    for s in &states {
        assert_eq!(RunState::from_str(s.as_str()), Some(s.clone()));
    }

    // ManifestStatus
    let m_statuses = [
        ManifestStatus::Draft, ManifestStatus::Proposed, ManifestStatus::Approved,
        ManifestStatus::Conditional, ManifestStatus::Quarantined,
        ManifestStatus::Rejected, ManifestStatus::Superseded,
    ];
    for s in &m_statuses {
        assert_eq!(ManifestStatus::from_str(s.as_str()), Some(s.clone()));
    }

    // ProjectionStatus
    assert_eq!(ProjectionStatus::from_str("active"), Some(ProjectionStatus::Active));
    assert_eq!(ProjectionStatus::from_str("superseded"), Some(ProjectionStatus::Superseded));

    // RoutingStatus
    for s in &[RoutingStatus::Selected, RoutingStatus::NoProjection,
               RoutingStatus::RejectedByConstraints, RoutingStatus::AmbiguousRole,
               RoutingStatus::PacketRejected] {
        assert_eq!(RoutingStatus::from_str(s.as_str()), Some(s.clone()));
    }

    // RuleSeverity
    assert_eq!(RuleSeverity::from_str("critical"), RuleSeverity::Critical);
    assert_eq!(RuleSeverity::from_str("warning"), RuleSeverity::Warning);
    assert_eq!(RuleSeverity::from_str("info"), RuleSeverity::Info);
}

/// RH-T20: All assert_no_capability_data checks pass on core types
#[test]
fn test_rh_t20_all_no_capability_checks() {
    // These types have assert_no_capability_data methods (compile-time proof they exist)
    let run_result = fixtures::run_result("test", RunState::Completed, 50);
    assert!(run_result.assert_no_capability_data().is_ok());

    let batch_result = fixtures::batch_result(vec![run_result]);
    assert!(batch_result.assert_no_capability_data().is_ok());
}
