//! Provenance builder — constructs EvidenceProvenance from qualification outputs.
//!
//! Collects provenance metadata from:
//! - QualificationRunResult (source identity, execution context)
//! - ValidatorEngine results (validator chain)
//! - CustomRuleEvidence (custom evidence refs)
//! - IndividualBatchResult (batch context)
//!
//! All methods are pure functions — they produce provenance without
//! mutating or authorizing anything.

use super::models::{
    BatchProvenance, CustomEvidenceRef, EvidenceProvenance, ExecutionContext, ProvenanceSource,
    ValidatorProvenance,
};
use crate::qualification::batch::IndividualBatchResult;
use crate::qualification::run_result::QualificationRunResult;

/// Evidence provenance builder.
///
/// Constructs complete provenance records from qualification outputs.
/// The builder has no side effects — it creates provenance data only.
pub struct ProvenanceBuilder;

impl ProvenanceBuilder {
    /// Build provenance from a single qualification run result.
    ///
    /// Includes:
    /// - Source identity (model, run, request, task pack)
    /// - Execution context (state, timestamps, telemetry)
    /// - Custom evidence references (from result.custom_evidence)
    /// - Deterministic lineage hash
    pub fn from_run_result(result: &QualificationRunResult) -> EvidenceProvenance {
        let source = ProvenanceSource::from(result);
        let execution = ExecutionContext::from(result);

        let custom_evidence: Vec<CustomEvidenceRef> = result
            .custom_evidence
            .iter()
            .map(CustomEvidenceRef::from_evidence)
            .collect();

        let mut provenance = EvidenceProvenance {
            source,
            execution,
            validator_chain: vec![],
            custom_evidence,
            batch_context: None,
            lineage_hash: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        provenance.lineage_hash = provenance.compute_lineage_hash();
        provenance
    }

    /// Build provenance from a batch individual result.
    ///
    /// Adds batch context linking this model's position in the batch.
    pub fn from_batch_result(
        individual: &IndividualBatchResult,
        batch_id: &str,
        total_targets: usize,
    ) -> EvidenceProvenance {
        let mut provenance = Self::from_run_result(&individual.result);

        provenance.batch_context = Some(BatchProvenance {
            batch_id: batch_id.to_string(),
            target_position: individual.position,
            total_targets,
        });

        // Recompute hash with batch context
        provenance.lineage_hash = provenance.compute_lineage_hash();
        provenance
    }

    /// Add a validator chain entry to existing provenance.
    pub fn add_validator(
        provenance: &mut EvidenceProvenance,
        validator: ValidatorProvenance,
    ) {
        provenance.validator_chain.push(validator);
        provenance.lineage_hash = provenance.compute_lineage_hash();
    }

    /// Verify provenance integrity — checks lineage hash and field completeness.
    pub fn verify(provenance: &EvidenceProvenance) -> Result<(), Vec<String>> {
        // Check hash integrity
        if !provenance.verify_lineage_hash() {
            return Err(vec![
                "Lineage hash mismatch — provenance may have been tampered".to_string(),
            ]);
        }

        // Check field completeness
        let missing = provenance.detect_missing_provenance();
        if !missing.is_empty() {
            return Err(missing);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qualification::custom_executor::{CustomRuleEvidence, CustomRuleOutcome};
    use crate::qualification::run_result::{GenerationSettings, RuntimeTelemetry};
    use crate::qualification::run_state::RunState;

    fn test_result() -> QualificationRunResult {
        QualificationRunResult {
            run_id: "run-test-001".to_string(),
            request_id: "qr-test-001".to_string(),
            model_id: "minicpm5-1b-q4km".to_string(),
            model_sha256: "81B64D05A23B".to_string(),
            model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            task_pack_id: "tp-if-001".to_string(),
            fixture_hash: "abc123".to_string(),
            state: RunState::Completed,
            raw_output: Some("Hello, how can I help you?".to_string()),
            settings: GenerationSettings {
                runtime_profile_id: "prof-q4km".to_string(),
                max_tokens: Some(256),
                temperature: Some(0.0),
                timeout_seconds: Some(120),
                task_description: "Test fixture".to_string(),
            },
            telemetry: RuntimeTelemetry {
                port: Some(9120),
                process_id: Some(10804),
                load_duration_ms: Some(2187),
                generation_duration_ms: Some(385),
                input_tokens: Some(10),
                output_tokens: Some(32),
                http_status: Some(200),
                runtime_error: None,
            },
            lifecycle_events: vec![],
            error_message: None,
            custom_evidence: vec![],
            started_at: "2026-07-11T12:00:00Z".to_string(),
            ended_at: Some("2026-07-11T12:00:01Z".to_string()),
        }
    }

    // EP-B1: Build from run result
    #[test]
    fn test_build_from_run_result() {
        let result = test_result();
        let provenance = ProvenanceBuilder::from_run_result(&result);

        assert_eq!(provenance.source.model_id, result.model_id);
        assert_eq!(provenance.source.run_id, result.run_id);
        assert_eq!(provenance.execution.state, result.state);
        assert!(!provenance.lineage_hash.is_empty());
        assert!(provenance.verify_lineage_hash());
    }

    // EP-B2: Build from run result with custom evidence
    #[test]
    fn test_build_with_custom_evidence() {
        let mut result = test_result();
        result.custom_evidence = vec![
            CustomRuleEvidence {
                rule_id: "CR-001".to_string(),
                version: "1.0.0".to_string(),
                outcome: CustomRuleOutcome {
                    passed: true,
                    message: None,
                    execution_duration_ms: Some(10),
                    timed_out: false,
                    panicked: false,
                },
                task_pack_id: "tp-001".to_string(),
                content_hash: "abc123".to_string(),
                executed_at: "2026-01-01T00:00:00Z".to_string(),
            },
        ];

        let provenance = ProvenanceBuilder::from_run_result(&result);
        assert_eq!(provenance.custom_evidence.len(), 1);
        assert_eq!(provenance.custom_evidence[0].rule_id, "CR-001");
    }

    // EP-B3: Add validator to provenance
    #[test]
    fn test_add_validator() {
        let result = test_result();
        let mut provenance = ProvenanceBuilder::from_run_result(&result);

        let validator = ValidatorProvenance {
            validator_pack_id: "vp-001".to_string(),
            task_pack_id: "tp-001".to_string(),
            rule_count: 5,
            critical_failures: 0,
            overall_pass: true,
        };

        ProvenanceBuilder::add_validator(&mut provenance, validator);
        assert_eq!(provenance.validator_chain.len(), 1);
        assert!(provenance.verify_lineage_hash());
    }

    // EP-B4: Verify passes for valid provenance
    #[test]
    fn test_verify_valid() {
        let result = test_result();
        let provenance = ProvenanceBuilder::from_run_result(&result);
        assert!(ProvenanceBuilder::verify(&provenance).is_ok());
    }

    // EP-B5: Verify fails for tampered provenance
    #[test]
    fn test_verify_tampered() {
        let result = test_result();
        let mut provenance = ProvenanceBuilder::from_run_result(&result);
        provenance.source.model_id = "tampered".to_string();
        let verification = ProvenanceBuilder::verify(&provenance);
        assert!(verification.is_err());
    }

    // EP-B6: Module registration proof
    #[test]
    fn test_provenance_module_exists() {
        let _ = crate::provenance::ProvenanceBuilder;
    }
}
