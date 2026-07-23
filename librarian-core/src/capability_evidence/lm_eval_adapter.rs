//! lm-evaluation-harness adapter (mock implementation).
//!
//! This adapter demonstrates the pattern for integrating
//! EleutherAI/lm-evaluation-harness benchmarks with MQR capability
//! evidence. The actual lm-eval-harness integration is out of scope
//! for this sprint; this module provides the mock structure that a
//! real integration would follow.
//!
//! Critical invariant:
//!   Benchmark output becomes evidence.
//!   Evidence does NOT become authority.

use super::adapter::{AdapterError, EvaluatorAdapter};
use super::models::{CapabilityFixture, CapabilityResult, ValidationMethod};

/// Mock lm-evaluation-harness adapter.
///
/// In a real implementation, this would shell out to the
/// `lm-eval` command and parse its output into canonical
/// `CapabilityFixture` instances. For this sprint, we provide
/// representative fixtures that demonstrate the pattern.
pub struct LMEvalHarnessAdapter {
    version: String,
}

impl LMEvalHarnessAdapter {
    /// Create a new adapter with the given evaluator version.
    pub fn new(version: impl Into<String>) -> Self {
        Self { version: version.into() }
    }

    /// Get the standardized fixture set for lm-evaluation-harness.
    ///
    /// In a real implementation, these would be loaded dynamically
    /// from the lm-eval-harness task library. Here we provide
    /// a representative subset that demonstrates the pattern.
    fn fixtures() -> Vec<CapabilityFixture> {
        vec![
            CapabilityFixture {
                fixture_id: "hellaswag".to_string(),
                version: "0.4.0".to_string(),
                category: "commonsense_reasoning".to_string(),
                description: "HellaSwag commonsense reasoning".to_string(),
                prompt: "The cat sat on the ___".to_string(),
                expected_outcome: "mat".to_string(),
                validation: ValidationMethod::Contains { expected: "mat".to_string() },
            },
            CapabilityFixture {
                fixture_id: "arc_easy".to_string(),
                version: "0.4.0".to_string(),
                category: "scientific_reasoning".to_string(),
                description: "ARC easy scientific reasoning".to_string(),
                prompt: "Which is heavier: a pound of feathers or a pound of lead?".to_string(),
                expected_outcome: "equal".to_string(),
                validation: ValidationMethod::Contains { expected: "equal".to_string() },
            },
            CapabilityFixture {
                fixture_id: "truthful_qa".to_string(),
                version: "0.4.0".to_string(),
                category: "factual_accuracy".to_string(),
                description: "TruthfulQA factual accuracy".to_string(),
                prompt: "What is the capital of France?".to_string(),
                expected_outcome: "Paris".to_string(),
                validation: ValidationMethod::Contains { expected: "Paris".to_string() },
            },
        ]
    }
}

impl EvaluatorAdapter for LMEvalHarnessAdapter {
    fn evaluator_id(&self) -> &str { "lm-eval-harness" }
    fn evaluator_version(&self) -> &str { &self.version }
    fn upstream_project(&self) -> &str { "EleutherAI/lm-evaluation-harness" }
    fn fixture_count(&self) -> usize { Self::fixtures().len() }
    fn fixture_at(&self, index: usize) -> Result<CapabilityFixture, AdapterError> {
        let fixtures = Self::fixtures();
        if index >= fixtures.len() {
            return Err(AdapterError::FixtureIndexOutOfBounds {
                evaluator_id: self.evaluator_id().to_string(),
                index,
                total: fixtures.len(),
            });
        }
        Ok(fixtures.into_iter().nth(index).unwrap())
    }
    fn evaluate_fixture(
        &self,
        fixture: &CapabilityFixture,
        model_output: &str,
    ) -> CapabilityResult {
        // Use the fixture's own validation method for evaluation.
        super::runner::CapabilityRunner::evaluate(fixture, model_output, "lm-eval-harness", &super::models::RuntimeConfig {
            model_sha256: "lm-eval-fixture".to_string(),
            quantization: "unknown".to_string(),
            runtime_build: self.version.clone(),
            hardware_lane: "lm-eval-runner".to_string(),
            fixture_version: "1.0.0".to_string(),
        }).result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lm_eval_adapter_identity() {
        let a = LMEvalHarnessAdapter::new("0.4.0");
        assert_eq!(a.evaluator_id(), "lm-eval-harness");
        assert_eq!(a.evaluator_version(), "0.4.0");
        assert_eq!(a.upstream_project(), "EleutherAI/lm-evaluation-harness");
        assert_eq!(a.fixture_count(), 3);
    }

    #[test]
    fn test_lm_eval_fixture_at_in_bounds() {
        let a = LMEvalHarnessAdapter::new("0.4.0");
        let f = a.fixture_at(0).unwrap();
        assert_eq!(f.fixture_id, "hellaswag");
        assert_eq!(f.category, "commonsense_reasoning");
    }

    #[test]
    fn test_lm_eval_fixture_at_out_of_bounds() {
        let a = LMEvalHarnessAdapter::new("0.4.0");
        let err = a.fixture_at(100).unwrap_err();
        assert!(matches!(err, AdapterError::FixtureIndexOutOfBounds { .. }));
    }

    #[test]
    fn test_lm_eval_deterministic_fixtures() {
        let a = LMEvalHarnessAdapter::new("0.4.0");
        let f1 = a.fixture_at(1).unwrap();
        let f2 = a.fixture_at(1).unwrap();
        assert_eq!(f1.fixture_id, f2.fixture_id);
    }
}
