//! Observability data models — read-only summary types.
//!
//! Each model is a pure data view. None contain capability authority fields:
//! - No manifest_id, decision_id, or projection references
//! - No approval/rejection status
//! - No router eligibility flags
//! - No Owner decision fields

use serde::{Deserialize, Serialize};

use crate::qualification::batch::BatchQualificationResult;
use crate::qualification::run_result::QualificationRunResult;
use crate::qualification::run_state::RunState;

/// Summary of a single qualification run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualificationRunSummary {
    /// Run ID (deterministic).
    pub run_id: String,

    /// Model ID qualified.
    pub model_id: String,

    /// Qualification request ID.
    pub request_id: String,

    /// Final run state.
    pub state: RunState,

    /// When the run started.
    pub started_at: String,

    /// When the run ended.
    pub ended_at: Option<String>,

    /// Generation duration in milliseconds.
    pub duration_ms: Option<u64>,

    /// Output token count.
    pub output_tokens: Option<u32>,

    /// Number of lifecycle events recorded.
    pub event_count: usize,

    /// Number of custom evidence records.
    pub custom_evidence_count: usize,

    /// Whether the run completed successfully.
    pub passed: bool,
}

/// Summary of evidence produced during qualification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceSummaryView {
    /// Total lifecycle events across observed runs.
    pub total_events: usize,

    /// Total custom evidence records.
    pub total_custom_evidence: usize,

    /// Number of timed-out rule executions.
    pub timeout_events: usize,

    /// Number of panicked rule executions.
    pub panic_events: usize,

    /// Deterministic content hashes for evidence.
    pub evidence_hashes: Vec<String>,

    /// Provenance references (run:run_id:model_id).
    pub provenance_refs: Vec<String>,
}

/// Summary of a batch qualification execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchExecutionSummary {
    /// Batch ID.
    pub batch_id: String,

    /// Total targets in batch.
    pub total_targets: usize,

    /// Number of completed (successful) targets.
    pub completed: usize,

    /// Number of failed targets.
    pub failed: usize,

    /// Evidence provenance references.
    pub evidence_refs: Vec<String>,

    /// Individual run summaries (in execution order).
    pub individual_summaries: Vec<QualificationRunSummary>,
}

/// Runtime health indicators.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeHealth {
    /// Whether qualification runs have been executed (service has been active).
    pub has_qualification_activity: bool,

    /// Total qualification runs observed.
    pub total_runs: usize,

    /// Number of successful runs.
    pub successful_runs: usize,

    /// Number of failed runs.
    pub failed_runs: usize,

    /// Timestamp of the most recent run.
    pub last_run_at: Option<String>,

    /// Whether recent runs show healthy operation.
    pub is_healthy: bool,
}

/// Complete observability report aggregating all qualification views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservabilityReport {
    /// Run summaries.
    pub runs: Vec<QualificationRunSummary>,

    /// Evidence summary.
    pub evidence: EvidenceSummaryView,

    /// Batch summary (if batch was executed).
    pub batch: Option<BatchExecutionSummary>,

    /// Runtime health.
    pub health: RuntimeHealth,

    /// Report generation timestamp.
    pub generated_at: String,
}

impl QualificationRunSummary {
    /// Create a run summary from a qualification run result.
    pub fn from_result(result: &QualificationRunResult) -> Self {
        let duration_ms = result.telemetry.generation_duration_ms;
        let output_tokens = result.telemetry.output_tokens;
        let event_count = result.lifecycle_events.len();
        let custom_evidence_count = result.custom_evidence.len();
        let passed = result.state.is_success();

        Self {
            run_id: result.run_id.clone(),
            model_id: result.model_id.clone(),
            request_id: result.request_id.clone(),
            state: result.state.clone(),
            started_at: result.started_at.clone(),
            ended_at: result.ended_at.clone(),
            duration_ms,
            output_tokens,
            event_count,
            custom_evidence_count,
            passed,
        }
    }
}

impl EvidenceSummaryView {
    /// Create an evidence summary from a collection of run results.
    pub fn from_results(results: &[QualificationRunResult]) -> Self {
        let total_events = results.iter().map(|r| r.lifecycle_events.len()).sum();
        let total_custom_evidence: usize = results.iter().map(|r| r.custom_evidence.len()).sum();
        let timeout_events = results
            .iter()
            .flat_map(|r| &r.custom_evidence)
            .filter(|e| e.outcome.timed_out)
            .count();
        let panic_events = results
            .iter()
            .flat_map(|r| &r.custom_evidence)
            .filter(|e| e.outcome.panicked)
            .count();
        let evidence_hashes: Vec<String> = results
            .iter()
            .flat_map(|r| &r.custom_evidence)
            .map(|e| e.content_hash.clone())
            .collect();
        let provenance_refs: Vec<String> = results
            .iter()
            .map(|r| format!("run:{}:{}", r.run_id, r.model_id))
            .collect();

        Self {
            total_events,
            total_custom_evidence,
            timeout_events,
            panic_events,
            evidence_hashes,
            provenance_refs,
        }
    }
}

impl BatchExecutionSummary {
    /// Create a batch summary from a batch qualification result.
    pub fn from_batch_result(result: &BatchQualificationResult) -> Self {
        let individual_summaries: Vec<QualificationRunSummary> = result
            .individual_results
            .iter()
            .map(|ir| QualificationRunSummary::from_result(&ir.result))
            .collect();

        Self {
            batch_id: result.batch_id.clone(),
            total_targets: result.aggregate.total_targets,
            completed: result.aggregate.completed,
            failed: result.aggregate.failed,
            evidence_refs: result.aggregate.evidence_references.clone(),
            individual_summaries,
        }
    }
}

impl RuntimeHealth {
    /// Create a health summary from a collection of run results.
    pub fn from_results(results: &[QualificationRunResult]) -> Self {
        let total_runs = results.len();
        let successful_runs = results.iter().filter(|r| r.state.is_success()).count();
        let failed_runs = results.iter().filter(|r| r.state.is_failure()).count();
        let last_run_at = results.last().map(|r| r.ended_at.clone()).flatten();
        let has_qualification_activity = total_runs > 0;

        // Healthy if no recent failures or no activity (no news is good news)
        let is_healthy = if total_runs == 0 {
            true
        } else {
            // Healthy if more than 50% of runs succeed
            successful_runs > failed_runs || successful_runs == total_runs
        };

        Self {
            has_qualification_activity,
            total_runs,
            successful_runs,
            failed_runs,
            last_run_at,
            is_healthy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qualification::batch::{
        AggregateBatchSummary, BatchQualificationResult, IndividualBatchResult,
    };
    use crate::qualification::run_result::{
        GenerationSettings, RuntimeTelemetry,
    };
    use crate::qualification::custom_executor::{
        CustomRuleEvidence, CustomRuleOutcome,
    };

    fn make_result(
        run_id: &str,
        model_id: &str,
        state: RunState,
        token_count: Option<u32>,
        duration_ms: Option<u64>,
        custom_evidence: Vec<CustomRuleEvidence>,
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
                generation_duration_ms: duration_ms,
                input_tokens: Some(10),
                output_tokens: token_count,
                http_status: Some(200),
                runtime_error: None,
            },
            lifecycle_events: vec![],
            error_message: None,
            custom_evidence,
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: Some("2026-01-01T00:00:01Z".to_string()),
        }
    }

    fn make_evidence(passed: bool, timed_out: bool, panicked: bool) -> CustomRuleEvidence {
        CustomRuleEvidence {
            rule_id: "CR-TEST".to_string(),
            version: "1.0.0".to_string(),
            outcome: CustomRuleOutcome {
                passed,
                message: None,
                execution_duration_ms: Some(10),
                timed_out,
                panicked,
            },
            task_pack_id: "tp-001".to_string(),
            content_hash: "hash123".to_string(),
            executed_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    // OBS-U1: Run summary from successful result
    #[test]
    fn test_run_summary_success() {
        let result = make_result(
            "run-001", "model-a", RunState::Completed, Some(50), Some(100), vec![],
        );
        let summary = QualificationRunSummary::from_result(&result);
        assert_eq!(summary.run_id, "run-001");
        assert_eq!(summary.model_id, "model-a");
        assert!(summary.passed);
        assert_eq!(summary.duration_ms, Some(100));
        assert_eq!(summary.output_tokens, Some(50));
    }

    // OBS-U2: Run summary from failed result
    #[test]
    fn test_run_summary_failed() {
        let result = make_result(
            "run-002", "model-b", RunState::ModelFailed, None, None, vec![],
        );
        let summary = QualificationRunSummary::from_result(&result);
        assert_eq!(summary.state, RunState::ModelFailed);
        assert!(!summary.passed);
    }

    // OBS-U3: Evidence summary aggregates correctly
    #[test]
    fn test_evidence_summary() {
        let results = vec![
            make_result("r1", "m1", RunState::Completed, Some(10), Some(50), vec![
                make_evidence(true, false, false),
                make_evidence(false, true, false),  // timeout
            ]),
            make_result("r2", "m2", RunState::Completed, Some(20), Some(100), vec![
                make_evidence(false, false, true),  // panic
            ]),
        ];

        let ev = EvidenceSummaryView::from_results(&results);
        assert_eq!(ev.total_custom_evidence, 3);
        assert_eq!(ev.timeout_events, 1);
        assert_eq!(ev.panic_events, 1);
        assert_eq!(ev.provenance_refs.len(), 2);
    }

    // OBS-U4: Evidence summary with no evidence
    #[test]
    fn test_evidence_summary_empty() {
        let results: Vec<QualificationRunResult> = vec![];
        let ev = EvidenceSummaryView::from_results(&results);
        assert_eq!(ev.total_events, 0);
        assert_eq!(ev.total_custom_evidence, 0);
        assert_eq!(ev.timeout_events, 0);
        assert_eq!(ev.panic_events, 0);
        assert!(ev.evidence_hashes.is_empty());
        assert!(ev.provenance_refs.is_empty());
    }

    // OBS-U5: Batch summary from batch result
    #[test]
    fn test_batch_summary() {
        let batch = BatchQualificationResult {
            batch_id: "batch-001".to_string(),
            model_order: vec!["model-a".to_string(), "model-b".to_string()],
            individual_results: vec![
                IndividualBatchResult {
                    position: 0,
                    model_id: "model-a".to_string(),
                    result: make_result("r1", "model-a", RunState::Completed, Some(10), Some(50), vec![]),
                    state: RunState::Completed,
                    error_message: None,
                },
                IndividualBatchResult {
                    position: 1,
                    model_id: "model-b".to_string(),
                    result: make_result("r2", "model-b", RunState::ModelFailed, None, None, vec![]),
                    state: RunState::ModelFailed,
                    error_message: Some("failed".to_string()),
                },
            ],
            aggregate: AggregateBatchSummary {
                total_targets: 2,
                completed: 1,
                failed: 1,
                total_duration_ms: 50,
                evidence_references: vec!["run:r1:model-a".to_string()],
                content_hash: "hash".to_string(),
            },
            started_at: "2026-01-01".to_string(),
            completed_at: "2026-01-01".to_string(),
            content_hash: "hash".to_string(),
        };

        let summary = BatchExecutionSummary::from_batch_result(&batch);
        assert_eq!(summary.total_targets, 2);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.individual_summaries.len(), 2);
    }

    // OBS-U6: Runtime health from empty results
    #[test]
    fn test_runtime_health_empty() {
        let results: Vec<QualificationRunResult> = vec![];
        let health = RuntimeHealth::from_results(&results);
        assert!(!health.has_qualification_activity);
        assert_eq!(health.total_runs, 0);
        assert!(health.is_healthy);
    }

    // OBS-U7: Runtime health from successful runs
    #[test]
    fn test_runtime_health_healthy() {
        let results = vec![
            make_result("r1", "m1", RunState::Completed, Some(10), Some(50), vec![]),
            make_result("r2", "m2", RunState::Completed, Some(20), Some(100), vec![]),
        ];
        let health = RuntimeHealth::from_results(&results);
        assert!(health.has_qualification_activity);
        assert_eq!(health.total_runs, 2);
        assert_eq!(health.successful_runs, 2);
        assert_eq!(health.failed_runs, 0);
        assert!(health.is_healthy);
    }

    // OBS-U8: Runtime health detects failures
    #[test]
    fn test_runtime_health_unhealthy() {
        let results = vec![
            make_result("r1", "m1", RunState::ModelFailed, None, None, vec![]),
            make_result("r2", "m2", RunState::RuntimeFailed, None, None, vec![]),
            make_result("r3", "m3", RunState::ModelFailed, None, None, vec![]),
        ];
        let health = RuntimeHealth::from_results(&results);
        assert_eq!(health.total_runs, 3);
        assert_eq!(health.successful_runs, 0);
        assert_eq!(health.failed_runs, 3);
        assert!(!health.is_healthy);
    }

    // OBS-U9: Observability models have no capability authority fields
    #[test]
    fn test_no_capability_data() {
        let summary = QualificationRunSummary {
            run_id: "r".to_string(),
            model_id: "m".to_string(),
            request_id: "req".to_string(),
            state: RunState::Completed,
            started_at: "".to_string(),
            ended_at: None,
            duration_ms: None,
            output_tokens: None,
            event_count: 0,
            custom_evidence_count: 0,
            passed: true,
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert!(json.get("manifest_id").is_none());
        assert!(json.get("decision_id").is_none());
        assert!(json.get("approved").is_none());
        assert!(json.get("router_eligible").is_none());
        assert!(json.get("projection_id").is_none());
    }
}
