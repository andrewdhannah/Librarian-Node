//! Observability service — read-only aggregation layer.
//!
//! Aggregates qualification state from existing sealed types into
//! structured observability summaries. This service introduces NO
//! mutation, NO decision authority, and NO routing paths.
//!
//! The service consumes existing data:
//!   QualificationRunResult → QualificationRunSummary
//!   CustomRuleEvidence     → EvidenceSummaryView
//!   BatchQualificationResult → BatchExecutionSummary
//!   Vec<QualificationRunResult> → RuntimeHealth
//!   All above              → ObservabilityReport
//!
//! It does NOT produce:
//!   - CapabilityManifest
//!   - OwnerDecision
//!   - RouterProjection
//!   - RoutingLogEntry
//!   - Any state mutation

use super::models::{
    BatchExecutionSummary, EvidenceSummaryView, ObservabilityReport, QualificationRunSummary,
    RuntimeHealth,
};
use crate::qualification::batch::BatchQualificationResult;
use crate::qualification::run_result::QualificationRunResult;

/// Read-only observability service.
///
/// All methods are pure functions — they transform input data into
/// summary views without side effects.
pub struct ObservabilityService;

impl ObservabilityService {
    /// Create a summary of a single qualification run.
    pub fn summarize_run(result: &QualificationRunResult) -> QualificationRunSummary {
        QualificationRunSummary::from_result(result)
    }

    /// Create an evidence summary from a collection of runs.
    pub fn summarize_evidence(results: &[QualificationRunResult]) -> EvidenceSummaryView {
        EvidenceSummaryView::from_results(results)
    }

    /// Create a batch execution summary.
    pub fn summarize_batch(result: &BatchQualificationResult) -> BatchExecutionSummary {
        BatchExecutionSummary::from_batch_result(result)
    }

    /// Create runtime health indicators from run history.
    pub fn health_check(results: &[QualificationRunResult]) -> RuntimeHealth {
        RuntimeHealth::from_results(results)
    }

    /// Create a complete observability report.
    ///
    /// This is the primary entry point for the observability surface.
    /// It produces a single structured view of all qualification activity.
    pub fn report(
        run_results: &[QualificationRunResult],
        batch_result: Option<&BatchQualificationResult>,
    ) -> ObservabilityReport {
        let runs: Vec<QualificationRunSummary> = run_results
            .iter()
            .map(|r| QualificationRunSummary::from_result(r))
            .collect();

        let evidence = EvidenceSummaryView::from_results(run_results);
        let batch = batch_result.map(BatchExecutionSummary::from_batch_result);
        let health = RuntimeHealth::from_results(run_results);
        let generated_at = chrono::Utc::now().to_rfc3339();

        ObservabilityReport {
            runs,
            evidence,
            batch,
            health,
            generated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qualification::run_result::{
        GenerationSettings, RuntimeTelemetry,
    };
    use crate::qualification::run_state::RunState;

    fn make_result(
        run_id: &str,
        model_id: &str,
        state: RunState,
    ) -> QualificationRunResult {
        QualificationRunResult {
            run_id: run_id.to_string(),
            request_id: format!("req-{}", run_id),
            model_id: model_id.to_string(),
            model_sha256: format!("sha256-{}", model_id),
            model_filename: format!("{}.gguf", model_id),
            task_pack_id: "tp-001".to_string(),
            fixture_hash: "abc123".to_string(),
            state,
            raw_output: Some("output".to_string()),
            settings: GenerationSettings {
                runtime_profile_id: "prof-001".to_string(),
                max_tokens: Some(256),
                temperature: Some(0.0),
                timeout_seconds: Some(120),
                task_description: "test".to_string(),
            },
            telemetry: RuntimeTelemetry {
                port: Some(9000),
                process_id: Some(12345),
                load_duration_ms: Some(1000),
                generation_duration_ms: Some(100),
                input_tokens: Some(10),
                output_tokens: Some(50),
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

    // OBS-S1: Summarize single run
    #[test]
    fn test_summarize_single_run() {
        let result = make_result("run-1", "model-a", RunState::Completed);
        let summary = ObservabilityService::summarize_run(&result);
        assert_eq!(summary.run_id, "run-1");
        assert_eq!(summary.model_id, "model-a");
    }

    // OBS-S2: Summarize evidence across runs
    #[test]
    fn test_summarize_evidence() {
        let results = vec![
            make_result("r1", "m1", RunState::Completed),
            make_result("r2", "m2", RunState::ModelFailed),
        ];
        let ev = ObservabilityService::summarize_evidence(&results);
        assert_eq!(ev.total_events, 0); // No lifecycle events in test helpers
        assert_eq!(ev.provenance_refs.len(), 2);
    }

    // OBS-S3: Health check
    #[test]
    fn test_health_check() {
        let results = vec![make_result("r1", "m1", RunState::Completed)];
        let health = ObservabilityService::health_check(&results);
        assert!(health.has_qualification_activity);
        assert_eq!(health.total_runs, 1);
    }

    // OBS-S4: Complete report
    #[test]
    fn test_complete_report() {
        let results = vec![make_result("r1", "m1", RunState::Completed)];
        let report = ObservabilityService::report(&results, None);
        assert_eq!(report.runs.len(), 1);
        assert!(report.batch.is_none());
        assert!(!report.generated_at.is_empty());
    }

    // OBS-S5: Report with empty run list
    #[test]
    fn test_empty_report() {
        let results: Vec<QualificationRunResult> = vec![];
        let report = ObservabilityService::report(&results, None);
        assert!(report.runs.is_empty());
        assert!(report.batch.is_none());
        assert!(report.health.is_healthy);
    }

    // OBS-S6: Report contains no capability authority fields
    #[test]
    fn test_report_no_capability_data() {
        let results = vec![make_result("r1", "m1", RunState::Completed)];
        let report = ObservabilityService::report(&results, None);
        let json = serde_json::to_value(&report).unwrap();
        assert!(json.get("manifest_id").is_none(), "Report must not contain manifest_id");
        assert!(json.get("decision_id").is_none(), "Report must not contain decision_id");
        assert!(json.get("projections").is_none(), "Report must not contain projections");
        assert!(json.get("router_eligible").is_none(), "Report must not contain router_eligible");
        assert!(json.get("approved").is_none(), "Report must not contain approved");
    }
}
