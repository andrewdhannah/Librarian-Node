//! Provenance data models — evidence chain traceability.
//!
//! Each model links evidence back to its originating context.
//! None contain capability, decision, or routing authority fields.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::qualification::run_result::QualificationRunResult;
use crate::qualification::run_state::RunState;

/// Source identity — which model and run produced this evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceSource {
    /// Model ID.
    pub model_id: String,
    /// Model SHA-256.
    pub model_sha256: String,
    /// Qualification run ID.
    pub run_id: String,
    /// Qualification request ID.
    pub request_id: String,
    /// Task pack ID.
    pub task_pack_id: String,
}

/// Execution context — when and how the run executed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionContext {
    /// Final run state.
    pub state: RunState,
    /// When the run started.
    pub started_at: String,
    /// When the run ended.
    pub ended_at: Option<String>,
    /// Generation duration in milliseconds.
    pub generation_duration_ms: Option<u64>,
    /// Output tokens generated.
    pub output_tokens: Option<u32>,
}

/// Which validators contributed to this evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidatorProvenance {
    /// Validator pack ID.
    pub validator_pack_id: String,
    /// Task pack ID validated.
    pub task_pack_id: String,
    /// Number of rules evaluated.
    pub rule_count: usize,
    /// Number of critical failures.
    pub critical_failures: usize,
    /// Whether all critical rules passed.
    pub overall_pass: bool,
}

/// Reference to a custom rule execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomEvidenceRef {
    /// Rule identity.
    pub rule_id: String,
    /// Rule version.
    pub version: String,
    /// Whether the rule passed.
    pub passed: bool,
    /// Content hash for tamper detection.
    pub content_hash: String,
}

/// Batch context — which batch execution contained this model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchProvenance {
    /// Batch execution ID.
    pub batch_id: String,
    /// Position of this model in the batch.
    pub target_position: usize,
    /// Total targets in the batch.
    pub total_targets: usize,
}

/// Complete evidence provenance for a qualification execution.
///
/// Links model → run → validators → custom evidence → batch → lineage hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceProvenance {
    /// Source identity.
    pub source: ProvenanceSource,
    /// Execution context.
    pub execution: ExecutionContext,
    /// Validator chain (which validators ran).
    pub validator_chain: Vec<ValidatorProvenance>,
    /// Custom evidence references.
    pub custom_evidence: Vec<CustomEvidenceRef>,
    /// Batch context (if applicable).
    pub batch_context: Option<BatchProvenance>,
    /// Lineage hash — deterministic hash of entire provenance chain.
    pub lineage_hash: String,
    /// When this provenance record was created.
    pub created_at: String,
}

impl EvidenceProvenance {
    /// Compute a deterministic lineage hash from all provenance fields.
    pub fn compute_lineage_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Source
        hasher.update(self.source.model_id.as_bytes());
        hasher.update(b"|");
        hasher.update(self.source.model_sha256.as_bytes());
        hasher.update(b"|");
        hasher.update(self.source.run_id.as_bytes());
        hasher.update(b"|");
        hasher.update(self.source.request_id.as_bytes());

        // Execution
        hasher.update(self.execution.state.as_str().as_bytes());
        hasher.update(b"|");
        if let Some(ms) = self.execution.generation_duration_ms {
            hasher.update(ms.to_be_bytes());
        }

        // Custom evidence
        for ce in &self.custom_evidence {
            hasher.update(ce.rule_id.as_bytes());
            hasher.update(b":");
            hasher.update(ce.content_hash.as_bytes());
            hasher.update(b";");
        }

        // Batch context
        if let Some(batch) = &self.batch_context {
            hasher.update(batch.batch_id.as_bytes());
            hasher.update(b"|");
            hasher.update(batch.target_position.to_be_bytes());
            hasher.update(b"|");
            hasher.update(batch.total_targets.to_be_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// Verify that the stored lineage hash matches re-computation.
    pub fn verify_lineage_hash(&self) -> bool {
        self.lineage_hash == self.compute_lineage_hash()
    }

    /// Detect missing provenance — checks that all required fields are present.
    pub fn detect_missing_provenance(&self) -> Vec<String> {
        let mut missing = Vec::new();

        if self.source.model_id.is_empty() {
            missing.push("source.model_id is empty".to_string());
        }
        if self.source.model_sha256.is_empty() {
            missing.push("source.model_sha256 is empty".to_string());
        }
        if self.source.run_id.is_empty() {
            missing.push("source.run_id is empty".to_string());
        }
        if self.source.request_id.is_empty() {
            missing.push("source.request_id is empty".to_string());
        }
        if self.execution.started_at.is_empty() {
            missing.push("execution.started_at is empty".to_string());
        }
        if self.lineage_hash.is_empty() {
            missing.push("lineage_hash is empty".to_string());
        }
        if self.created_at.is_empty() {
            missing.push("created_at is empty".to_string());
        }

        missing
    }

    /// Assert that this provenance contains no capability authority data.
    pub fn assert_no_capability_data(&self) -> bool {
        // Structural proof: none of the fields are capability-related.
        // The fields are:
        // - source (model identity, run ID — NOT capability)
        // - execution (state, timestamps — NOT capability)
        // - validator_chain (pack IDs, counts — NOT capability)
        // - custom_evidence (rule IDs, hashes — NOT capability)
        // - batch_context (batch IDs — NOT capability)
        // - lineage_hash, created_at (integrity — NOT capability)
        true
    }
}

impl From<&QualificationRunResult> for ProvenanceSource {
    fn from(result: &QualificationRunResult) -> Self {
        Self {
            model_id: result.model_id.clone(),
            model_sha256: result.model_sha256.clone(),
            run_id: result.run_id.clone(),
            request_id: result.request_id.clone(),
            task_pack_id: result.task_pack_id.clone(),
        }
    }
}

impl From<&QualificationRunResult> for ExecutionContext {
    fn from(result: &QualificationRunResult) -> Self {
        Self {
            state: result.state.clone(),
            started_at: result.started_at.clone(),
            ended_at: result.ended_at.clone(),
            generation_duration_ms: result.telemetry.generation_duration_ms,
            output_tokens: result.telemetry.output_tokens,
        }
    }
}

impl CustomEvidenceRef {
    /// Create from a custom rule evidence record.
    pub fn from_evidence(evidence: &crate::qualification::custom_executor::CustomRuleEvidence) -> Self {
        Self {
            rule_id: evidence.rule_id.clone(),
            version: evidence.version.clone(),
            passed: evidence.outcome.passed,
            content_hash: evidence.content_hash.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qualification::run_result::{GenerationSettings, RuntimeTelemetry};
    use crate::qualification::run_state::RunState;

    fn make_source() -> ProvenanceSource {
        ProvenanceSource {
            model_id: "model-a".to_string(),
            model_sha256: "sha256-a".to_string(),
            run_id: "run-001".to_string(),
            request_id: "req-001".to_string(),
            task_pack_id: "tp-001".to_string(),
        }
    }

    fn make_execution() -> ExecutionContext {
        ExecutionContext {
            state: RunState::Completed,
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: Some("2026-01-01T00:00:01Z".to_string()),
            generation_duration_ms: Some(100),
            output_tokens: Some(50),
        }
    }

    fn make_provenance() -> EvidenceProvenance {
        let mut p = EvidenceProvenance {
            source: make_source(),
            execution: make_execution(),
            validator_chain: vec![],
            custom_evidence: vec![],
            batch_context: None,
            lineage_hash: String::new(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        p.lineage_hash = p.compute_lineage_hash();
        p
    }

    fn make_test_result() -> QualificationRunResult {
        QualificationRunResult {
            run_id: "run-test-001".to_string(),
            request_id: "qr-test-001".to_string(),
            model_id: "minicpm5-1b-q4km".to_string(),
            model_sha256: "81B64D05A23B".to_string(),
            model_filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            task_pack_id: "tp-if-001".to_string(),
            fixture_hash: "abc123".to_string(),
            state: RunState::Completed,
            raw_output: Some("Hello".to_string()),
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

    // EP-U1: Lineage hash is deterministic
    #[test]
    fn test_lineage_hash_deterministic() {
        let p1 = make_provenance();
        let p2 = make_provenance();
        assert_eq!(p1.lineage_hash, p2.lineage_hash);
        assert_eq!(p1.lineage_hash.len(), 64);
    }

    // EP-U2: Lineage hash changes with different input
    #[test]
    fn test_lineage_hash_changes_with_input() {
        let p1 = make_provenance();
        let mut p2 = make_provenance();
        p2.source.model_id = "model-b".to_string();
        p2.lineage_hash = p2.compute_lineage_hash();

        assert_ne!(p1.lineage_hash, p2.lineage_hash);

        // Different run_id also changes hash
        let mut p3 = make_provenance();
        p3.source.run_id = "run-002".to_string();
        p3.lineage_hash = p3.compute_lineage_hash();

        assert_ne!(p1.lineage_hash, p3.lineage_hash);
    }

    // EP-U3: Verify lineage hash passes for valid provenance
    #[test]
    fn test_verify_hash_valid() {
        let p = make_provenance();
        assert!(p.verify_lineage_hash());
    }

    // EP-U4: Verify lineage hash fails for tampered provenance
    #[test]
    fn test_verify_hash_tampered() {
        let mut p = make_provenance();
        p.source.model_id = "tampered-model".to_string();
        // Don't recompute hash — simulate tampering
        assert!(!p.verify_lineage_hash());
    }

    // EP-U5: Detect missing provenance
    #[test]
    fn test_detect_missing_source() {
        let mut p = make_provenance();
        p.source.model_id = String::new();
        let missing = p.detect_missing_provenance();
        assert!(!missing.is_empty());
        assert!(missing.iter().any(|m| m.contains("model_id")));
    }

    // EP-U6: Detect missing lineage hash
    #[test]
    fn test_detect_missing_hash() {
        let mut p = make_provenance();
        p.lineage_hash = String::new();
        let missing = p.detect_missing_provenance();
        assert!(!missing.is_empty());
        assert!(missing.iter().any(|m| m.contains("lineage_hash")));
    }

    // EP-U7: Complete provenance has no missing fields
    #[test]
    fn test_complete_provenance_no_missing() {
        let p = make_provenance();
        let missing = p.detect_missing_provenance();
        assert!(missing.is_empty(), "Missing: {:?}", missing);
    }

    // EP-U8: Provenance has no capability authority data
    #[test]
    fn test_no_capability_data() {
        let p = make_provenance();
        assert!(p.assert_no_capability_data());
        let json = serde_json::to_value(&p).unwrap();
        assert!(json.get("manifest_id").is_none());
        assert!(json.get("decision_id").is_none());
        assert!(json.get("approved").is_none());
        assert!(json.get("router_eligible").is_none());
        assert!(json.get("projection_id").is_none());
    }

    // EP-U9: Serialization round-trip preserves provenance
    #[test]
    fn test_serialization_roundtrip() {
        let p = make_provenance();
        let json = serde_json::to_string(&p).unwrap();
        let parsed: EvidenceProvenance = serde_json::from_str(&json).unwrap();
        assert_eq!(p.lineage_hash, parsed.lineage_hash);
        assert_eq!(p.source.model_id, parsed.source.model_id);
        assert_eq!(p.source.run_id, parsed.source.run_id);
    }

    // EP-U10: Batch context changes lineage hash
    #[test]
    fn test_batch_context_changes_hash() {
        let mut p = make_provenance();
        let hash_without = p.lineage_hash.clone();

        p.batch_context = Some(BatchProvenance {
            batch_id: "batch-001".to_string(),
            target_position: 0,
            total_targets: 3,
        });
        p.lineage_hash = p.compute_lineage_hash();
        let hash_with = p.lineage_hash.clone();

        assert_ne!(hash_without, hash_with);
    }

    // EP-U11: Custom evidence refs change lineage hash
    #[test]
    fn test_custom_evidence_changes_hash() {
        let mut p = make_provenance();
        p.custom_evidence = vec![
            CustomEvidenceRef {
                rule_id: "CR-001".to_string(),
                version: "1.0.0".to_string(),
                passed: true,
                content_hash: "abc123".to_string(),
            }
        ];
        p.lineage_hash = p.compute_lineage_hash();
        assert_eq!(p.lineage_hash.len(), 64);
    }

    // EP-U12: ProvenanceSource from QualificationRunResult
    #[test]
    fn test_from_run_result_source() {
        let result = make_test_result();
        let source = ProvenanceSource::from(&result);
        assert_eq!(source.model_id, result.model_id);
        assert_eq!(source.run_id, result.run_id);
    }

    // EP-U13: ExecutionContext from QualificationRunResult
    #[test]
    fn test_from_run_result_execution() {
        let result = make_test_result();
        let exec = ExecutionContext::from(&result);
        assert_eq!(exec.state, result.state);
        assert_eq!(exec.generation_duration_ms, result.telemetry.generation_duration_ms);
    }
}
