//! Evaluator adapter framework.
//!
//! This module defines the trait and types that allow external evaluators
//! and internal fixtures to produce canonical capability evidence records.
//!
//! Critical invariant:
//!   Adapters produce capability evidence. They do NOT create authority.
//!   Adapters execute evaluation logic; they do NOT approve models.

use super::models::CapabilityFixture;

/// Errors that can occur during adapter operations.
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterError {
    /// The requested fixture index is out of bounds.
    FixtureIndexOutOfBounds {
        evaluator_id: String,
        index: usize,
        total: usize,
    },
    /// The adapter encountered an error during evaluation.
    EvaluationFailed {
        evaluator_id: String,
        reason: String,
    },
    /// Duplicate registration attempt.
    DuplicateRegistration {
        evaluator_id: String,
    },
    /// Requested evaluator was not found in the registry.
    EvaluatorNotFound {
        evaluator_id: String,
    },
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FixtureIndexOutOfBounds { evaluator_id, index, total } => {
                write!(f, "Fixture index {} out of bounds for evaluator '{}' (total: {})",
                       index, evaluator_id, total)
            }
            Self::EvaluationFailed { evaluator_id, reason } => {
                write!(f, "Evaluation failed for evaluator '{}': {}",
                       evaluator_id, reason)
            }
            Self::DuplicateRegistration { evaluator_id } => {
                write!(f, "Evaluator '{}' is already registered", evaluator_id)
            }
            Self::EvaluatorNotFound { evaluator_id } => {
                write!(f, "Evaluator '{}' not found in registry", evaluator_id)
            }
        }
    }
}

impl std::error::Error for AdapterError {}

/// Trait that all capability evaluators must implement.
///
/// Adapters are the boundary between external evaluation tools and the
/// MQR canonical capability evidence system.
///
/// An adapter is responsible for:
/// 1. Declaring its identity (id, version, upstream project)
/// 2. Producing deterministic capability fixtures
/// 3. Executing the evaluation (via the run method)
/// 4. Producing canonical capability evidence records
///
/// Adapters MUST NOT:
/// - Create authority
/// - Make qualification decisions
/// - Mutate router state
/// - Assign lifecycle transitions
pub trait EvaluatorAdapter: Send + Sync {
    /// Unique identifier of the evaluator (e.g., "lm-eval-harness").
    fn evaluator_id(&self) -> &str;

    /// Version of the evaluator implementation.
    fn evaluator_version(&self) -> &str;

    /// Upstream project name (e.g., "EleutherAI/lm-evaluation-harness").
    fn upstream_project(&self) -> &str;

    /// Number of fixtures this adapter can produce.
    fn fixture_count(&self) -> usize;

    /// Get a fixture by index.
    ///
    /// Returns AdapterError::FixtureIndexOutOfBounds if the index is
    /// out of range. The fixture must be deterministic — calling this
    /// method repeatedly with the same index must return equal fixtures.
    fn fixture_at(&self, index: usize) -> Result<CapabilityFixture, AdapterError>;

    /// Execute the fixture against model output and return the result state.
    ///
    /// Adapters implement this to integrate their specific validation
    /// logic with the MQR canonical evidence model. The result must be
    /// one of the 5 canonical states.
    fn evaluate_fixture(
        &self,
        fixture: &CapabilityFixture,
        model_output: &str,
    ) -> super::models::CapabilityResult;
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::models::{CapabilityResult, ValidationMethod};

    /// Simple test adapter for unit tests.
    struct CountingAdapter {
        count: usize,
    }

    impl EvaluatorAdapter for CountingAdapter {
        fn evaluator_id(&self) -> &str { "counting" }
        fn evaluator_version(&self) -> &str { "1.0.0" }
        fn upstream_project(&self) -> &str { "MQR-test" }
        fn fixture_count(&self) -> usize { self.count }
        fn fixture_at(&self, index: usize) -> Result<CapabilityFixture, AdapterError> {
            if index >= self.count {
                return Err(AdapterError::FixtureIndexOutOfBounds {
                    evaluator_id: self.evaluator_id().to_string(),
                    index,
                    total: self.count,
                });
            }
            Ok(CapabilityFixture {
                fixture_id: format!("fixture-{}", index),
                version: "1.0.0".to_string(),
                category: "test".to_string(),
                description: format!("Test fixture {}", index),
                prompt: "test".to_string(),
                expected_outcome: "ok".to_string(),
                validation: ValidationMethod::Contains { expected: "ok".to_string() },
            })
        }
        fn evaluate_fixture(
            &self,
            _fixture: &CapabilityFixture,
            model_output: &str,
        ) -> CapabilityResult {
            if model_output.contains("ok") {
                CapabilityResult::Pass
            } else {
                CapabilityResult::Fail
            }
        }
    }

    #[test]
    fn test_adapter_identity() {
        let a = CountingAdapter { count: 5 };
        assert_eq!(a.evaluator_id(), "counting");
        assert_eq!(a.evaluator_version(), "1.0.0");
        assert_eq!(a.upstream_project(), "MQR-test");
        assert_eq!(a.fixture_count(), 5);
    }

    #[test]
    fn test_fixture_at_in_bounds() {
        let a = CountingAdapter { count: 3 };
        let f = a.fixture_at(1).unwrap();
        assert_eq!(f.fixture_id, "fixture-1");
    }

    #[test]
    fn test_fixture_at_out_of_bounds() {
        let a = CountingAdapter { count: 3 };
        let err = a.fixture_at(10).unwrap_err();
        match err {
            AdapterError::FixtureIndexOutOfBounds { evaluator_id, index, total } => {
                assert_eq!(evaluator_id, "counting");
                assert_eq!(index, 10);
                assert_eq!(total, 3);
            }
            _ => panic!("Expected FixtureIndexOutOfBounds"),
        }
    }

    #[test]
    fn test_adapter_evaluates_fixture() {
        let a = CountingAdapter { count: 1 };
        let f = a.fixture_at(0).unwrap();
        assert_eq!(a.evaluate_fixture(&f, "this is ok"), CapabilityResult::Pass);
        assert_eq!(a.evaluate_fixture(&f, "this fails"), CapabilityResult::Fail);
    }

    #[test]
    fn test_adapter_error_display() {
        let err = AdapterError::FixtureIndexOutOfBounds {
            evaluator_id: "test".to_string(),
            index: 5,
            total: 2,
        };
        let s = format!("{}", err);
        assert!(s.contains("Fixture index 5"));
        assert!(s.contains("test"));
    }
}
