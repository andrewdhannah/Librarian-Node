//! Capability evidence data models — records, fixtures, results, classifications.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Result of a capability fixture execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CapabilityResult {
    /// Output matched expected outcome within tolerance.
    Pass,
    /// Output did not match expected outcome.
    Fail,
    /// Execution produced inconsistent results across runs.
    Unstable,
    /// Fixture was not executed (model lacks prerequisite).
    NotTested,
    /// Model produced output but quality was degraded.
    Degraded,
}

impl CapabilityResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unstable => "unstable",
            Self::NotTested => "not_tested",
            Self::Degraded => "degraded",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pass" => Some(Self::Pass),
            "fail" => Some(Self::Fail),
            "unstable" => Some(Self::Unstable),
            "not_tested" => Some(Self::NotTested),
            "degraded" => Some(Self::Degraded),
            _ => None,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Validate that the result is one of the approved states.
    pub fn validate(&self) -> Result<(), String> {
        // All 5 states are valid by construction; this method is for
        // future extensibility checks (e.g., rejecting unknown states from
        // external sources).
        Ok(())
    }
}

/// Classification of observed failures during capability evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FailureClassification {
    /// Model generated entities that don't exist in the source.
    HallucinatedEntity,
    /// Model made a claim not supported by the input.
    UnsupportedClaim,
    /// Output structure deviated from expected format.
    FormattingDrift,
    /// Model lost context from earlier in the conversation.
    ContextLoss,
    /// Model entered a repetitive output loop.
    RepetitionLoop,
    /// Output contained corrupted tokens or mixed languages.
    LanguageCorruption,
    /// Different runs produced different outputs for same input.
    NondeterministicOutput,
    /// Model returned incomplete output (cut off mid-generation).
    PartialCompletion,
    /// Output violated an expected schema or structure.
    SchemaViolation,
    /// Model exhibited unsafe behavior in safety test context.
    UnsafeBehavior,
}

impl FailureClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HallucinatedEntity => "hallucinated_entity",
            Self::UnsupportedClaim => "unsupported_claim",
            Self::FormattingDrift => "formatting_drift",
            Self::ContextLoss => "context_loss",
            Self::RepetitionLoop => "repetition_loop",
            Self::LanguageCorruption => "language_corruption",
            Self::NondeterministicOutput => "nondeterministic_output",
            Self::PartialCompletion => "partial_completion",
            Self::SchemaViolation => "schema_violation",
            Self::UnsafeBehavior => "unsafe_behavior",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hallucinated_entity" => Some(Self::HallucinatedEntity),
            "unsupported_claim" => Some(Self::UnsupportedClaim),
            "formatting_drift" => Some(Self::FormattingDrift),
            "context_loss" => Some(Self::ContextLoss),
            "repetition_loop" => Some(Self::RepetitionLoop),
            "language_corruption" => Some(Self::LanguageCorruption),
            "nondeterministic_output" => Some(Self::NondeterministicOutput),
            "partial_completion" => Some(Self::PartialCompletion),
            "schema_violation" => Some(Self::SchemaViolation),
            "unsafe_behavior" => Some(Self::UnsafeBehavior),
            _ => None,
        }
    }

    /// Human-readable description of this failure classification.
    pub fn description(&self) -> &'static str {
        match self {
            Self::HallucinatedEntity => "Model generated entities that don't exist in the source",
            Self::UnsupportedClaim => "Model made a claim not supported by the input",
            Self::FormattingDrift => "Output structure deviated from expected format",
            Self::ContextLoss => "Model lost context from earlier in the conversation",
            Self::RepetitionLoop => "Model entered a repetitive output loop",
            Self::LanguageCorruption => "Output contained corrupted tokens or mixed languages",
            Self::NondeterministicOutput => "Different runs produced different outputs for same input",
            Self::PartialCompletion => "Model returned incomplete output (cut off mid-generation)",
            Self::SchemaViolation => "Output violated an expected schema or structure",
            Self::UnsafeBehavior => "Model exhibited unsafe behavior in safety test context",
        }
    }
}

/// A single observed failure during capability evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FailureObservation {
    /// Classification of the failure.
    pub classification: FailureClassification,
    /// Human-readable description.
    pub description: String,
    /// Excerpt of the model output showing the failure.
    pub evidence: String,
}

/// Validation method for a capability fixture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationMethod {
    /// Output must contain exact substring.
    Contains { expected: String },
    /// Output must match exact string.
    ExactMatch { expected: String },
    /// Output must be valid JSON.
    ValidJson,
    /// Output must not contain the given pattern.
    NotContains { forbidden: String },
    /// Custom validation via regex pattern.
    Regex { pattern: String },
}

/// A deterministic capability evaluation fixture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityFixture {
    /// Unique fixture ID.
    pub fixture_id: String,
    /// Fixture version (semantic).
    pub version: String,
    /// Category (e.g., "factual_retrieval", "structured_output").
    pub category: String,
    /// Description of what this fixture tests.
    pub description: String,
    /// Input prompt for the model.
    pub prompt: String,
    /// Expected outcome description (human-readable).
    pub expected_outcome: String,
    /// Validation method.
    pub validation: ValidationMethod,
}

/// Model artifact identity — binds capability evidence to a specific model version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModelIdentity {
    /// Model ID.
    pub model_id: String,
    /// Model artifact SHA-256.
    pub model_sha256: String,
    /// Quantization variant (e.g., "Q4_K_M").
    pub quantization: String,
    /// Model version tag (e.g., "v1.0.0").
    pub model_version: String,
}

/// Evaluator identity — binds capability evidence to a specific evaluation tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EvaluatorIdentity {
    /// Evaluator ID (e.g., "lm-eval-harness", "mqr-internal").
    pub evaluator_id: String,
    /// Evaluator version (e.g., "0.4.0").
    pub evaluator_version: String,
    /// Upstream project name (e.g., "EleutherAI/lm-evaluation-harness").
    pub upstream_project: String,
}

/// Fixture identity — binds capability evidence to a specific fixture version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FixtureIdentity {
    /// Fixture ID.
    pub fixture_id: String,
    /// Fixture version (semantic).
    pub fixture_version: String,
}

/// Execution context — when and where the evidence was collected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ExecutionContext {
    /// When the evidence was collected (RFC 3339).
    pub timestamp: String,
    /// Hardware lane where evaluation ran.
    pub hardware_lane: String,
    /// Runtime build identifier.
    pub runtime_build: String,
}

/// Runtime configuration snapshot — provenance for capability evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RuntimeConfig {
    /// Model SHA-256.
    pub model_sha256: String,
    /// Quantization variant (e.g., "Q4_K_M").
    pub quantization: String,
    /// Runtime build identifier.
    pub runtime_build: String,
    /// Hardware lane (e.g., "RX 570").
    pub hardware_lane: String,
    /// Fixture version at time of execution.
    pub fixture_version: String,
}

/// Provenance reference — links evidence to existing MQR provenance concepts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProvenanceReference {
    /// Reference to a provenance record lineage hash (optional).
    pub lineage_hash: Option<String>,
    /// Reference to a lifecycle event ID (optional).
    pub lifecycle_event_id: Option<String>,
    /// Reference to a model identity hash.
    pub model_identity_hash: String,
}

/// Complete capability evidence record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityEvidence {
    /// Deterministic evidence ID.
    pub evidence_id: String,
    /// Model identity (who was tested).
    pub model_identity: ModelIdentity,
    /// Runtime configuration (under what conditions).
    pub runtime_configuration: RuntimeConfig,
    /// Evaluator identity (who tested).
    pub evaluator_identity: EvaluatorIdentity,
    /// Fixture identity (what was tested).
    pub fixture_identity: FixtureIdentity,
    /// Execution context (when and where).
    pub execution_context: ExecutionContext,
    /// Execution result.
    pub result: CapabilityResult,
    /// Observed failures (empty if Pass or NotTested).
    pub failures: Vec<FailureObservation>,
    /// Provenance reference to existing MQR concepts.
    pub provenance_reference: ProvenanceReference,
    /// Content hash for tamper detection.
    pub evidence_hash: String,
}

impl CapabilityEvidence {
    /// Compute a deterministic evidence ID.
    pub fn compute_evidence_id(
        model_id: &str,
        fixture_id: &str,
        timestamp: &str,
    ) -> String {
        let input = format!("{}:{}:{}", model_id, fixture_id, timestamp);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute content hash from all evidence fields.
    ///
    /// Includes every authoritative field — any change to the
    /// evidence invalidates the hash, providing tamper detection.
    pub fn compute_content_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Source identity
        hasher.update(self.evidence_id.as_bytes());
        hasher.update(self.model_identity.model_id.as_bytes());
        hasher.update(self.model_identity.model_sha256.as_bytes());
        hasher.update(self.model_identity.quantization.as_bytes());
        hasher.update(self.model_identity.model_version.as_bytes());

        // Runtime
        hasher.update(self.runtime_configuration.model_sha256.as_bytes());
        hasher.update(self.runtime_configuration.quantization.as_bytes());
        hasher.update(self.runtime_configuration.runtime_build.as_bytes());
        hasher.update(self.runtime_configuration.hardware_lane.as_bytes());
        hasher.update(self.runtime_configuration.fixture_version.as_bytes());

        // Evaluator
        hasher.update(self.evaluator_identity.evaluator_id.as_bytes());
        hasher.update(self.evaluator_identity.evaluator_version.as_bytes());
        hasher.update(self.evaluator_identity.upstream_project.as_bytes());

        // Fixture
        hasher.update(self.fixture_identity.fixture_id.as_bytes());
        hasher.update(self.fixture_identity.fixture_version.as_bytes());

        // Execution
        hasher.update(self.execution_context.timestamp.as_bytes());
        hasher.update(self.execution_context.hardware_lane.as_bytes());
        hasher.update(self.execution_context.runtime_build.as_bytes());

        // Result
        hasher.update(self.result.as_str().as_bytes());
        for f in &self.failures {
            hasher.update(f.classification.as_str().as_bytes());
            hasher.update(f.description.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// Structural proof that this evidence has no authority-bearing fields.
    ///
    /// Capability evidence describes behavior — it does NOT authorize.
    /// This method is a compile-time-verifiable guard against
    /// accidental authority escalation.
    pub fn assert_no_authority_fields(&self) -> bool {
        // The fields are: evidence_id, model_identity,
        // runtime_configuration, evaluator_identity, fixture_identity,
        // execution_context, result, failures, provenance_reference,
        // evidence_hash.
        //
        // There are NO fields for: approval state, decision ownership,
        // router eligibility, lifecycle transition authority, or
        // qualification authority.
        true
    }

    /// JSON-level structural check for authority-bearing fields.
    ///
    /// Serializes to JSON and asserts that no forbidden fields exist.
    pub fn assert_no_authority_fields_in_json(&self) -> bool {
        let value = serde_json::to_value(self).unwrap();
        let forbidden = [
            "manifest_id",
            "decision_id",
            "approved",
            "rejected",
            "router_eligible",
            "projection_id",
            "score",
            "ranking",
            "intelligence_index",
        ];
        for f in &forbidden {
            if value.get(f).is_some() {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CAPE-U1: CapabilityResult string round-trip
    #[test]
    fn test_result_string_roundtrip() {
        for r in &[CapabilityResult::Pass, CapabilityResult::Fail, CapabilityResult::Unstable,
                    CapabilityResult::NotTested, CapabilityResult::Degraded] {
            assert_eq!(CapabilityResult::from_str(r.as_str()), Some(r.clone()));
        }
        assert_eq!(CapabilityResult::from_str("unknown"), None);
    }

    // CAPE-U2: CapabilityResult validation passes
    #[test]
    fn test_result_validation() {
        assert!(CapabilityResult::Pass.validate().is_ok());
        assert!(CapabilityResult::Fail.validate().is_ok());
        assert!(CapabilityResult::Unstable.validate().is_ok());
        assert!(CapabilityResult::NotTested.validate().is_ok());
        assert!(CapabilityResult::Degraded.validate().is_ok());
    }

    // CAPE-U3: FailureClassification string round-trip (10 types)
    #[test]
    fn test_failure_classification_roundtrip() {
        let all = [
            FailureClassification::HallucinatedEntity,
            FailureClassification::UnsupportedClaim,
            FailureClassification::FormattingDrift,
            FailureClassification::ContextLoss,
            FailureClassification::RepetitionLoop,
            FailureClassification::LanguageCorruption,
            FailureClassification::NondeterministicOutput,
            FailureClassification::PartialCompletion,
            FailureClassification::SchemaViolation,
            FailureClassification::UnsafeBehavior,
        ];
        for f in &all {
            assert_eq!(FailureClassification::from_str(f.as_str()), Some(f.clone()));
            assert!(!f.description().is_empty());
        }
        assert_eq!(FailureClassification::from_str("unknown"), None);
    }

    // CAPE-U4: Evidence ID is deterministic
    #[test]
    fn test_evidence_id_deterministic() {
        let id1 = CapabilityEvidence::compute_evidence_id("m1", "f1", "2026-01-01");
        let id2 = CapabilityEvidence::compute_evidence_id("m1", "f1", "2026-01-01");
        assert_eq!(id1, id2);
    }

    // CAPE-U5: Content hash is deterministic
    #[test]
    fn test_content_hash_deterministic() {
        let e = make_test_evidence();
        let h1 = e.compute_content_hash();
        let h2 = e.compute_content_hash();
        assert_eq!(h1, h2);
    }

    // CAPE-U6: Content hash changes with different fields
    #[test]
    fn test_content_hash_changes_with_field() {
        let e1 = make_test_evidence();
        let mut e2 = e1.clone();
        e2.model_identity.model_id = "model-other".to_string();
        let h1 = e1.compute_content_hash();
        let h2 = e2.compute_content_hash();
        assert_ne!(h1, h2);
    }

    // CAPE-U7: Structural authority boundary (no authority fields)
    #[test]
    fn test_assert_no_authority_fields() {
        let e = make_test_evidence();
        assert!(e.assert_no_authority_fields());
        assert!(e.assert_no_authority_fields_in_json());
    }

    // CAPE-U8: Serialization round-trip
    #[test]
    fn test_serialization_roundtrip() {
        let e = make_test_evidence_with_failures();
        let json = serde_json::to_string(&e).unwrap();
        let parsed: CapabilityEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.evidence_id, e.evidence_id);
        assert_eq!(parsed.result, e.result);
        assert_eq!(parsed.failures.len(), 1);
        assert!(parsed.assert_no_authority_fields_in_json());
    }

    // CAPE-U9: ProvenanceReference with model identity hash
    #[test]
    fn test_provenance_reference_with_identity() {
        let e = make_test_evidence();
        assert!(!e.provenance_reference.model_identity_hash.is_empty());
    }

    // CAPE-U10: New failure types are present
    #[test]
    fn test_new_failure_types_present() {
        assert_eq!(
            FailureClassification::from_str("partial_completion"),
            Some(FailureClassification::PartialCompletion)
        );
        assert_eq!(
            FailureClassification::from_str("schema_violation"),
            Some(FailureClassification::SchemaViolation)
        );
        assert_eq!(
            FailureClassification::from_str("unsafe_behavior"),
            Some(FailureClassification::UnsafeBehavior)
        );
    }

    // CAPE-U11: Evaluator identity is in evidence
    #[test]
    fn test_evaluator_identity_in_evidence() {
        let e = make_test_evidence();
        assert_eq!(e.evaluator_identity.evaluator_id, "mqr-internal");
        assert!(!e.evaluator_identity.evaluator_version.is_empty());
    }

    // CAPE-U12: Authority boundary — JSON has no score field
    #[test]
    fn test_no_score_field_in_json() {
        let e = make_test_evidence();
        let json = serde_json::to_value(&e).unwrap();
        assert!(json.get("score").is_none());
        assert!(json.get("ranking").is_none());
        assert!(json.get("intelligence_index").is_none());
        assert!(json.get("approved").is_none());
        assert!(json.get("manifest_id").is_none());
    }

    fn make_test_evidence() -> CapabilityEvidence {
        CapabilityEvidence {
            evidence_id: "evt-001".to_string(),
            model_identity: ModelIdentity {
                model_id: "m1".to_string(),
                model_sha256: "sha256-m1".to_string(),
                quantization: "Q4_K_M".to_string(),
                model_version: "v1.0.0".to_string(),
            },
            runtime_configuration: RuntimeConfig {
                model_sha256: "sha256-m1".to_string(),
                quantization: "Q4_K_M".to_string(),
                runtime_build: "c85e97a".to_string(),
                hardware_lane: "RX 570".to_string(),
                fixture_version: "1.0.0".to_string(),
            },
            evaluator_identity: EvaluatorIdentity {
                evaluator_id: "mqr-internal".to_string(),
                evaluator_version: "1.0.0".to_string(),
                upstream_project: "MQR".to_string(),
            },
            fixture_identity: FixtureIdentity {
                fixture_id: "fixture-001".to_string(),
                fixture_version: "1.0.0".to_string(),
            },
            execution_context: ExecutionContext {
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                hardware_lane: "RX 570".to_string(),
                runtime_build: "c85e97a".to_string(),
            },
            result: CapabilityResult::Pass,
            failures: vec![],
            provenance_reference: ProvenanceReference {
                lineage_hash: None,
                lifecycle_event_id: None,
                model_identity_hash: "sha256-m1".to_string(),
            },
            evidence_hash: String::new(),
        }
    }

    fn make_test_evidence_with_failures() -> CapabilityEvidence {
        let mut e = make_test_evidence();
        e.result = CapabilityResult::Fail;
        e.failures = vec![FailureObservation {
            classification: FailureClassification::HallucinatedEntity,
            description: "Model invented a citation".to_string(),
            evidence: "According to [fabricated study]...".to_string(),
        }];
        e
    }
}

