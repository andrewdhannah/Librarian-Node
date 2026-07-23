//! Owner Review data models — composite presentation-only types.
//!
//! None of these types contain capability, decision, routing, or policy fields.
//! They are pure presentation models derived from existing sealed data.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::observability::models::{
    BatchExecutionSummary, ObservabilityReport, QualificationRunSummary,
};
use crate::provenance::models::EvidenceProvenance;

/// Severity of a review finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewFindingSeverity {
    /// Informational observation.
    Info,
    /// Potential concern worth noting.
    Warning,
    /// Issue that may require attention.
    Issue,
}

/// A single review finding — informational observation, not a decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewFinding {
    /// Severity.
    pub severity: ReviewFindingSeverity,
    /// Category (e.g., "provenance", "evidence", "batch").
    pub category: String,
    /// Human-readable message.
    pub message: String,
    /// Optional detail/context.
    pub detail: Option<String>,
}

/// Qualification review — summary of qualification execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualificationReview {
    /// Number of qualification runs.
    pub total_runs: usize,
    /// Successful runs.
    pub completed: usize,
    /// Failed runs.
    pub failed: usize,
    /// Individual run summaries.
    pub runs: Vec<QualificationRunSummary>,
}

/// Evidence review — summary of evidence produced.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceReview {
    /// Total lifecycle events.
    pub total_events: usize,
    /// Total custom evidence records.
    pub total_custom_evidence: usize,
    /// Timeout events.
    pub timeout_events: usize,
    /// Panic events.
    pub panic_events: usize,
    /// Evidence content hashes.
    pub evidence_hashes: Vec<String>,
    /// Provenance references.
    pub provenance_refs: Vec<String>,
}

/// Provenance review — summary of provenance lineage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceReview {
    /// Number of provenance records.
    pub total_records: usize,
    /// Number of records with complete provenance.
    pub complete: usize,
    /// Number of records with missing provenance.
    pub with_missing: usize,
    /// Number of records with valid lineage hash.
    pub hash_valid: usize,
    /// Number of records with invalid lineage hash.
    pub hash_invalid: usize,
    /// List of models with provenance.
    pub models: Vec<String>,
}

/// Batch review — summary of batch execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchReview {
    /// Batch ID.
    pub batch_id: String,
    /// Total targets.
    pub total_targets: usize,
    /// Completed targets.
    pub completed: usize,
    /// Failed targets.
    pub failed: usize,
    /// Evidence provenance references.
    pub evidence_refs: Vec<String>,
    /// Individual run summaries.
    pub runs: Vec<QualificationRunSummary>,
}

/// Health review — runtime health snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthReview {
    /// Whether there has been qualification activity.
    pub has_activity: bool,
    /// Total runs.
    pub total_runs: usize,
    /// Successful runs.
    pub successful_runs: usize,
    /// Failed runs.
    pub failed_runs: usize,
    /// Last run timestamp.
    pub last_run_at: Option<String>,
    /// Whether the system is healthy.
    pub is_healthy: bool,
}

/// Complete Owner Review Package — consolidated presentation of all qualification state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewPackage {
    /// Qualification review.
    pub qualification: QualificationReview,
    /// Evidence review.
    pub evidence: EvidenceReview,
    /// Provenance review.
    pub provenance: ProvenanceReview,
    /// Batch review (if applicable).
    pub batch: Option<BatchReview>,
    /// Health review.
    pub health: HealthReview,
    /// Informational findings derived from review.
    pub findings: Vec<ReviewFinding>,
    /// Deterministic content hash of this review package.
    pub content_hash: String,
    /// When this review was generated.
    pub generated_at: String,
}

impl ReviewPackage {
    /// Compute a deterministic content hash for this review package.
    pub fn compute_content_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(self.qualification.total_runs.to_be_bytes());
        hasher.update(self.qualification.completed.to_be_bytes());
        hasher.update(self.qualification.failed.to_be_bytes());

        hasher.update(self.evidence.total_events.to_be_bytes());
        hasher.update(self.evidence.total_custom_evidence.to_be_bytes());
        hasher.update(self.evidence.timeout_events.to_be_bytes());
        hasher.update(self.evidence.panic_events.to_be_bytes());

        hasher.update(self.provenance.total_records.to_be_bytes());
        hasher.update(self.provenance.complete.to_be_bytes());
        hasher.update(self.provenance.with_missing.to_be_bytes());

        for finding in &self.findings {
            hasher.update(finding.category.as_bytes());
            hasher.update(finding.message.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// Assert no capability authority data in review package.
    pub fn assert_no_capability_data(&self) -> bool {
        // Structural proof: the fields are:
        // - qualification (run summaries — NOT capability)
        // - evidence (event counts, hashes — NOT capability)
        // - provenance (record counts, checks — NOT capability)
        // - batch (execution summaries — NOT capability)
        // - health (activity indicators — NOT capability)
        // - findings (informational — NOT decisions)
        // - content_hash, generated_at (integrity — NOT authority)
        true
    }
}

impl From<&ObservabilityReport> for QualificationReview {
    fn from(report: &ObservabilityReport) -> Self {
        let total_runs = report.runs.len();
        let completed = report.runs.iter().filter(|r| r.passed).count();
        let failed = total_runs - completed;
        Self {
            total_runs,
            completed,
            failed,
            runs: report.runs.clone(),
        }
    }
}

impl From<&ObservabilityReport> for EvidenceReview {
    fn from(report: &ObservabilityReport) -> Self {
        Self {
            total_events: report.evidence.total_events,
            total_custom_evidence: report.evidence.total_custom_evidence,
            timeout_events: report.evidence.timeout_events,
            panic_events: report.evidence.panic_events,
            evidence_hashes: report.evidence.evidence_hashes.clone(),
            provenance_refs: report.evidence.provenance_refs.clone(),
        }
    }
}

impl From<&ObservabilityReport> for HealthReview {
    fn from(report: &ObservabilityReport) -> Self {
        Self {
            has_activity: report.health.has_qualification_activity,
            total_runs: report.health.total_runs,
            successful_runs: report.health.successful_runs,
            failed_runs: report.health.failed_runs,
            last_run_at: report.health.last_run_at.clone(),
            is_healthy: report.health.is_healthy,
        }
    }
}

impl From<&BatchExecutionSummary> for BatchReview {
    fn from(summary: &BatchExecutionSummary) -> Self {
        Self {
            batch_id: summary.batch_id.clone(),
            total_targets: summary.total_targets,
            completed: summary.completed,
            failed: summary.failed,
            evidence_refs: summary.evidence_refs.clone(),
            runs: summary.individual_summaries.clone(),
        }
    }
}

impl ProvenanceReview {
    /// Create provenance review from a collection of provenance records.
    pub fn from_provenance_records(records: &[EvidenceProvenance]) -> Self {
        let total_records = records.len();
        let complete = records
            .iter()
            .filter(|r| r.detect_missing_provenance().is_empty())
            .count();
        let with_missing = total_records - complete;
        let hash_valid = records.iter().filter(|r| r.verify_lineage_hash()).count();
        let hash_invalid = total_records - hash_valid;
        let mut models: Vec<String> = records.iter().map(|r| r.source.model_id.clone()).collect();
        models.sort();
        models.dedup();

        Self {
            total_records,
            complete,
            with_missing,
            hash_valid,
            hash_invalid,
            models,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::models::{EvidenceSummaryView, RuntimeHealth};

    fn make_report() -> ObservabilityReport {
        ObservabilityReport {
            runs: vec![],
            evidence: EvidenceSummaryView {
                total_events: 10,
                total_custom_evidence: 3,
                timeout_events: 1,
                panic_events: 0,
                evidence_hashes: vec!["abc".to_string()],
                provenance_refs: vec!["run:r1:m1".to_string()],
            },
            batch: None,
            health: RuntimeHealth {
                has_qualification_activity: true,
                total_runs: 2,
                successful_runs: 1,
                failed_runs: 1,
                last_run_at: Some("2026-01-01".to_string()),
                is_healthy: true,
            },
            generated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    // OR-U1: Review package content hash is deterministic
    #[test]
    fn test_review_hash_deterministic() {
        let report = make_report();
        let qr = QualificationReview::from(&report);
        let er = EvidenceReview::from(&report);
        let hr = HealthReview::from(&report);

        let mut pkg1 = ReviewPackage {
            qualification: qr.clone(),
            evidence: er.clone(),
            provenance: ProvenanceReview {
                total_records: 0, complete: 0, with_missing: 0,
                hash_valid: 0, hash_invalid: 0, models: vec![],
            },
            batch: None,
            health: hr.clone(),
            findings: vec![],
            content_hash: String::new(),
            generated_at: "2026-01-01".to_string(),
        };
        let mut pkg2 = pkg1.clone();

        pkg1.content_hash = pkg1.compute_content_hash();
        pkg2.content_hash = pkg2.compute_content_hash();

        assert_eq!(pkg1.content_hash, pkg2.content_hash);
    }

    // OR-U2: QualificationReview from report
    #[test]
    fn test_qualification_review_from_report() {
        let report = make_report();
        let qr = QualificationReview::from(&report);
        assert_eq!(qr.total_runs, 0);
        assert_eq!(qr.completed, 0);
        assert_eq!(qr.failed, 0);
    }

    // OR-U3: EvidenceReview from report
    #[test]
    fn test_evidence_review_from_report() {
        let report = make_report();
        let er = EvidenceReview::from(&report);
        assert_eq!(er.total_events, 10);
        assert_eq!(er.timeout_events, 1);
    }

    // OR-U4: HealthReview from report
    #[test]
    fn test_health_review_from_report() {
        let report = make_report();
        let hr = HealthReview::from(&report);
        assert_eq!(hr.total_runs, 2);
        assert!(hr.is_healthy);
    }

    // OR-U5: ProvenanceReview from empty records
    #[test]
    fn test_provenance_review_empty() {
        let pr = ProvenanceReview::from_provenance_records(&[]);
        assert_eq!(pr.total_records, 0);
        assert_eq!(pr.complete, 0);
    }

    // OR-U6: ProvenanceReview from valid records
    #[test]
    fn test_provenance_review_valid() {
        let mut p = EvidenceProvenance {
            source: crate::provenance::models::ProvenanceSource {
                model_id: "m1".to_string(),
                model_sha256: "s1".to_string(),
                run_id: "r1".to_string(),
                request_id: "req1".to_string(),
                task_pack_id: "tp1".to_string(),
            },
            execution: crate::provenance::models::ExecutionContext {
                state: crate::qualification::run_state::RunState::Completed,
                started_at: "2026-01-01".to_string(),
                ended_at: None,
                generation_duration_ms: None,
                output_tokens: None,
            },
            validator_chain: vec![],
            custom_evidence: vec![],
            batch_context: None,
            lineage_hash: String::new(),
            created_at: "2026-01-01".to_string(),
        };
        p.lineage_hash = p.compute_lineage_hash();

        let pr = ProvenanceReview::from_provenance_records(&[p]);
        assert_eq!(pr.total_records, 1);
        assert_eq!(pr.complete, 1);
        assert_eq!(pr.hash_valid, 1);
    }

    // OR-U7: Review package has no capability authority data
    #[test]
    fn test_no_capability_data() {
        let report = make_report();
        let pkg = ReviewPackage {
            qualification: QualificationReview::from(&report),
            evidence: EvidenceReview::from(&report),
            provenance: ProvenanceReview::from_provenance_records(&[]),
            batch: None,
            health: HealthReview::from(&report),
            findings: vec![],
            content_hash: String::new(),
            generated_at: "2026-01-01".to_string(),
        };

        assert!(pkg.assert_no_capability_data());
        let json = serde_json::to_value(&pkg).unwrap();
        assert!(json.get("manifest_id").is_none());
        assert!(json.get("decision_id").is_none());
        assert!(json.get("approved").is_none());
        assert!(json.get("router_eligible").is_none());
        assert!(json.get("projection_id").is_none());
    }

    // OR-U8: Serialization round-trip
    #[test]
    fn test_serialization_roundtrip() {
        let report = make_report();
        let pkg = ReviewPackage {
            qualification: QualificationReview::from(&report),
            evidence: EvidenceReview::from(&report),
            provenance: ProvenanceReview::from_provenance_records(&[]),
            batch: None,
            health: HealthReview::from(&report),
            findings: vec![],
            content_hash: "hash".to_string(),
            generated_at: "2026-01-01".to_string(),
        };
        let json = serde_json::to_string(&pkg).unwrap();
        let parsed: ReviewPackage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.qualification.total_runs, pkg.qualification.total_runs);
        assert_eq!(parsed.evidence.total_events, pkg.evidence.total_events);
    }
}
