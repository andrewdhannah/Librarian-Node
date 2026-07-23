//! Review Builder — deterministic assembly of Owner Review Package.
//!
//! Composes existing sealed data into a single review package.
//! The builder has no side effects and no authority — it is
//! a pure transformation from existing data to presentation.

use super::models::{
    BatchReview, EvidenceReview, HealthReview, ProvenanceReview, QualificationReview,
    ReviewFinding, ReviewFindingSeverity, ReviewPackage,
};
use crate::observability::models::ObservabilityReport;
use crate::observability::service::ObservabilityService;
use crate::provenance::models::EvidenceProvenance;
use crate::qualification::batch::BatchQualificationResult;
use crate::qualification::run_result::QualificationRunResult;

/// Deterministic review builder — no side effects, no authority.
pub struct ReviewBuilder;

impl ReviewBuilder {
    /// Build a complete review package from existing sealed data.
    ///
    /// Inputs:
    /// - `run_results`: Slice of qualification run results
    /// - `provenance_records`: Slice of provenance records
    /// - `batch_result`: Optional batch qualification result
    ///
    /// Output:
    /// - `ReviewPackage`: Deterministic, presentation-only review
    ///
    /// Properties:
    /// - Same inputs → identical review package
    /// - No mutation of source data
    /// - No persistence side effects
    /// - Reproducible
    pub fn build(
        run_results: &[QualificationRunResult],
        provenance_records: &[EvidenceProvenance],
        batch_result: Option<&BatchQualificationResult>,
    ) -> ReviewPackage {
        let now = chrono::Utc::now().to_rfc3339();

        // Generate observability report
        let report = ObservabilityService::report(run_results, batch_result);

        // Derive review sections
        let qualification = QualificationReview::from(&report);
        let evidence = EvidenceReview::from(&report);
        let health = HealthReview::from(&report);
        let provenance = ProvenanceReview::from_provenance_records(provenance_records);
        let batch = batch_result.map(|br| {
            let summary = ObservabilityService::summarize_batch(br);
            BatchReview::from(&summary)
        });

        // Generate review findings
        let findings = Self::generate_findings(run_results, provenance_records, &report);

        // Assemble package
        let mut pkg = ReviewPackage {
            qualification,
            evidence,
            provenance,
            batch,
            health,
            findings,
            content_hash: String::new(),
            generated_at: now,
        };

        pkg.content_hash = pkg.compute_content_hash();
        pkg
    }

    /// Generate informational review findings from existing data.
    ///
    /// These are presentation-only observations. They are NOT decisions.
    fn generate_findings(
        run_results: &[QualificationRunResult],
        provenance_records: &[EvidenceProvenance],
        report: &ObservabilityReport,
    ) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        // Check for unfinished runs
        let unfinished = run_results
            .iter()
            .filter(|r| !r.state.is_terminal())
            .count();
        if unfinished > 0 {
            findings.push(ReviewFinding {
                severity: ReviewFindingSeverity::Warning,
                category: "execution".to_string(),
                message: format!("{} qualification run(s) did not complete", unfinished),
                detail: None,
            });
        }

        // Check for evidence without provenance
        if report.evidence.total_custom_evidence > 0 && provenance_records.is_empty() {
            findings.push(ReviewFinding {
                severity: ReviewFindingSeverity::Warning,
                category: "provenance".to_string(),
                message: "Custom evidence exists but no provenance records found".to_string(),
                detail: Some(format!(
                    "{} custom evidence records, 0 provenance records",
                    report.evidence.total_custom_evidence
                )),
            });
        }

        // Check for timeout events
        if report.evidence.timeout_events > 0 {
            findings.push(ReviewFinding {
                severity: ReviewFindingSeverity::Warning,
                category: "evidence".to_string(),
                message: format!(
                    "{} custom rule execution(s) timed out",
                    report.evidence.timeout_events
                ),
                detail: None,
            });
        }

        // Check for panic events
        if report.evidence.panic_events > 0 {
            findings.push(ReviewFinding {
                severity: ReviewFindingSeverity::Issue,
                category: "evidence".to_string(),
                message: format!(
                    "{} custom rule execution(s) panicked",
                    report.evidence.panic_events
                ),
                detail: None,
            });
        }

        // Check for missing provenance fields
        for record in provenance_records {
            let missing = record.detect_missing_provenance();
            if !missing.is_empty() {
                findings.push(ReviewFinding {
                    severity: ReviewFindingSeverity::Warning,
                    category: "provenance".to_string(),
                    message: format!(
                        "Provenance record '{}' has missing fields",
                        record.source.run_id
                    ),
                    detail: Some(format!("Missing: {:?}", missing)),
                });
            }
        }

        // Check for invalid lineage hashes
        for record in provenance_records {
            if !record.verify_lineage_hash() {
                findings.push(ReviewFinding {
                    severity: ReviewFindingSeverity::Issue,
                    category: "provenance".to_string(),
                    message: format!(
                        "Provenance record '{}' has invalid lineage hash",
                        record.source.run_id
                    ),
                    detail: Some(format!(
                        "Stored: {}, computed: {}",
                        record.lineage_hash,
                        record.compute_lineage_hash()
                    )),
                });
            }
        }

        // Check for batch execution failures
        if let Some(ref batch) = report.batch {
            if batch.failed > 0 {
                findings.push(ReviewFinding {
                    severity: ReviewFindingSeverity::Warning,
                    category: "batch".to_string(),
                    message: format!(
                        "Batch qualification: {} of {} target(s) failed",
                        batch.failed, batch.total_targets
                    ),
                    detail: None,
                });
            }
        }

        // Runtime health check
        if report.health.has_qualification_activity && !report.health.is_healthy {
            findings.push(ReviewFinding {
                severity: ReviewFindingSeverity::Issue,
                category: "health".to_string(),
                message: "Runtime health check indicates issues".to_string(),
                detail: Some(format!(
                    "{} successful, {} failed out of {} runs",
                    report.health.successful_runs,
                    report.health.failed_runs,
                    report.health.total_runs
                )),
            });
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qualification::run_result::{GenerationSettings, RuntimeTelemetry};
    use crate::qualification::run_state::RunState;

    fn make_result(state: RunState) -> QualificationRunResult {
        QualificationRunResult {
            run_id: "run-test".to_string(),
            request_id: "req-test".to_string(),
            model_id: "model-a".to_string(),
            model_sha256: "sha256-a".to_string(),
            model_filename: "model-a.gguf".to_string(),
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
            started_at: "2026-01-01".to_string(),
            ended_at: Some("2026-01-01T00:00:01Z".to_string()),
        }
    }

    // OR-B1: Build review package from empty data
    #[test]
    fn test_build_empty() {
        let pkg = ReviewBuilder::build(&[], &[], None);
        assert_eq!(pkg.qualification.total_runs, 0);
        assert!(pkg.findings.is_empty());
        assert!(pkg.batch.is_none());
        assert!(!pkg.content_hash.is_empty());
    }

    // OR-B2: Build review package with runs
    #[test]
    fn test_build_with_runs() {
        let results = vec![
            make_result(RunState::Completed),
            make_result(RunState::Completed),
        ];
        let pkg = ReviewBuilder::build(&results, &[], None);
        assert_eq!(pkg.qualification.total_runs, 2);
        assert_eq!(pkg.qualification.completed, 2);
        assert!(!pkg.content_hash.is_empty());
    }

    // OR-B3: Build review package is deterministic
    #[test]
    fn test_build_deterministic() {
        let results = vec![make_result(RunState::Completed)];
        let prov_records: Vec<EvidenceProvenance> = vec![];

        let pkg1 = ReviewBuilder::build(&results, &prov_records, None);
        let pkg2 = ReviewBuilder::build(&results, &prov_records, None);

        // Content hash should be the same since it excludes timestamps
        assert_eq!(pkg1.content_hash, pkg2.content_hash);
    }

    // OR-B4: Findings generated for panic events
    #[test]
    fn test_findings_panic_events() {
        let mut result = make_result(RunState::Completed);
        result.custom_evidence = vec![
            crate::qualification::custom_executor::CustomRuleEvidence {
                rule_id: "CR-PANIC".to_string(),
                version: "1.0.0".to_string(),
                outcome: crate::qualification::custom_executor::CustomRuleOutcome {
                    passed: false, message: None, execution_duration_ms: None,
                    timed_out: false, panicked: true,
                },
                task_pack_id: "tp-001".to_string(),
                content_hash: "hash1".to_string(),
                executed_at: "2026-01-01".to_string(),
            },
        ];

        let pkg = ReviewBuilder::build(&[result], &[], None);
        let panic_findings: Vec<&ReviewFinding> = pkg
            .findings
            .iter()
            .filter(|f| f.category == "evidence" && f.message.contains("panicked"))
            .collect();
        assert_eq!(panic_findings.len(), 1);
    }

    // OR-B5: Findings for timeout events
    #[test]
    fn test_findings_timeout_events() {
        let mut result = make_result(RunState::Completed);
        result.custom_evidence = vec![
            crate::qualification::custom_executor::CustomRuleEvidence {
                rule_id: "CR-TIMEOUT".to_string(),
                version: "1.0.0".to_string(),
                outcome: crate::qualification::custom_executor::CustomRuleOutcome {
                    passed: false, message: None, execution_duration_ms: None,
                    timed_out: true, panicked: false,
                },
                task_pack_id: "tp-001".to_string(),
                content_hash: "hash2".to_string(),
                executed_at: "2026-01-01".to_string(),
            },
        ];

        let pkg = ReviewBuilder::build(&[result], &[], None);
        let timeout_findings: Vec<&ReviewFinding> = pkg
            .findings
            .iter()
            .filter(|f| f.category == "evidence" && f.message.contains("timed out"))
            .collect();
        assert_eq!(timeout_findings.len(), 1);
    }

    // OR-B6: Review package has no capability authority data
    #[test]
    fn test_no_capability_data() {
        let results = vec![make_result(RunState::Completed)];
        let pkg = ReviewBuilder::build(&results, &[], None);
        assert!(pkg.assert_no_capability_data());

        let json = serde_json::to_value(&pkg).unwrap();
        assert!(json.get("manifest_id").is_none());
        assert!(json.get("decision_id").is_none());
        assert!(json.get("approved").is_none());
        assert!(json.get("router_eligible").is_none());
    }
}
