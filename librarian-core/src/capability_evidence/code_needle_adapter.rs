//! Code Needle adapter (mock implementation).
//!
//! This adapter demonstrates the pattern for integrating
//! code-reconstruction benchmarks (like Code Needle) with MQR
//! capability evidence. It tests whether a model can reproduce
//! code snippets from a given prompt.
//!
//! Critical invariant:
//!   Code reconstruction output becomes evidence.
//!   Evidence does NOT become authority.

use super::adapter::{AdapterError, EvaluatorAdapter};
use super::models::{CapabilityFixture, CapabilityResult, ValidationMethod};

/// Mock Code Needle adapter for code reconstruction testing.
pub struct CodeNeedleAdapter {
    version: String,
}

impl CodeNeedleAdapter {
    /// Create a new adapter with the given evaluator version.
    pub fn new(version: impl Into<String>) -> Self {
        Self { version: version.into() }
    }

    /// Get the code-reconstruction fixture set.
    fn fixtures() -> Vec<CapabilityFixture> {
        vec![
            CapabilityFixture {
                fixture_id: "code_needle_short".to_string(),
                version: "1.0.0".to_string(),
                category: "code_reconstruction".to_string(),
                description: "Short code snippet reconstruction".to_string(),
                prompt: "Write a Python function that adds two numbers:\n\ndef add(a, b):\n    ".to_string(),
                expected_outcome: "return a + b".to_string(),
                validation: ValidationMethod::Contains { expected: "return a + b".to_string() },
            },
            CapabilityFixture {
                fixture_id: "code_needle_medium".to_string(),
                version: "1.0.0".to_string(),
                category: "code_reconstruction".to_string(),
                description: "Medium code block reconstruction".to_string(),
                prompt: "Write a Python function that returns the factorial:\n\ndef factorial(n):\n    if n == 0:\n        return 1\n    ".to_string(),
                expected_outcome: "return n * factorial(n-1)".to_string(),
                validation: ValidationMethod::Contains { expected: "return n * factorial(n-1)".to_string() },
            },
        ]
    }
}

impl EvaluatorAdapter for CodeNeedleAdapter {
    fn evaluator_id(&self) -> &str { "code-needle" }
    fn evaluator_version(&self) -> &str { &self.version }
    fn upstream_project(&self) -> &str { "MQR-code-needle" }
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
        // Delegate to the canonical runner
        super::runner::CapabilityRunner::evaluate(fixture, model_output, "code-needle", &super::models::RuntimeConfig {
            model_sha256: "code-needle-fixture".to_string(),
            quantization: "unknown".to_string(),
            runtime_build: self.version.clone(),
            hardware_lane: "code-needle-runner".to_string(),
            fixture_version: "1.0.0".to_string(),
        }).result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_needle_adapter_identity() {
        let a = CodeNeedleAdapter::new("1.0.0");
        assert_eq!(a.evaluator_id(), "code-needle");
        assert_eq!(a.evaluator_version(), "1.0.0");
        assert_eq!(a.upstream_project(), "MQR-code-needle");
        assert_eq!(a.fixture_count(), 2);
    }

    #[test]
    fn test_code_needle_fixture_at_in_bounds() {
        let a = CodeNeedleAdapter::new("1.0.0");
        let f = a.fixture_at(0).unwrap();
        assert_eq!(f.fixture_id, "code_needle_short");
        assert_eq!(f.category, "code_reconstruction");
    }

    #[test]
    fn test_code_needle_deterministic() {
        let a = CodeNeedleAdapter::new("1.0.0");
        let f1 = a.fixture_at(0).unwrap();
        let f2 = a.fixture_at(0).unwrap();
        assert_eq!(f1.fixture_id, f2.fixture_id);
    }
}
