//! Capability evidence — deterministic capability evaluation framework.
//!
//! Proves what a model configuration can do, what it cannot reliably do,
//! under what runtime conditions the evidence was collected, and which
//! failure modes were observed.
//!
//! Critical invariant:
//!   Capability evidence describes behavior. It does NOT authorize,
//!   approve, reject, or route. No capability result can create authority.

pub mod adapter;
pub mod adversarial_fixtures;
pub mod code_needle_adapter;
pub mod lm_eval_adapter;
pub mod models;
pub mod operational_fixtures;
pub mod operational_runner;
pub mod profile;
pub mod quantization_differential;
pub mod replay;
pub mod review_package;
pub mod registry;
pub mod runner;

pub use adapter::{AdapterError, EvaluatorAdapter};
pub use adversarial_fixtures::{AdversarialDomain, AdversarialFixtures, AdversarialRunner};
pub use code_needle_adapter::CodeNeedleAdapter;
pub use lm_eval_adapter::LMEvalHarnessAdapter;
pub use models::{
    CapabilityEvidence, CapabilityFixture, CapabilityResult, EvaluatorIdentity,
    ExecutionContext, FailureClassification, FailureObservation, FixtureIdentity,
    ModelIdentity, ProvenanceReference, RuntimeConfig, ValidationMethod,
};
pub use operational_fixtures::{OperationalDomain, OperationalFixtures};
pub use operational_runner::OperationalRunner;
pub use profile::{AggregatedResult, CapabilityProfile, DomainProfile, EvidenceSource, ProfileAssembler, ProfileWarning, ProfileWarningSeverity};
pub use quantization_differential::{
    EvidenceDifference, FixtureDifferential, QuantizationDifferential,
    QuantizationDifferentialTool, RunConfig,
};
pub use replay::{CapabilityRegressionDetector, CapabilityReplay, FixtureComparison, RegressionResult};
pub use review_package::CapabilityReviewPackage;
pub use registry::AdapterRegistry;
pub use runner::{CapabilityRunner, DEFAULT_EVALUATOR_ID, DEFAULT_EVALUATOR_VERSION, DEFAULT_UPSTREAM_PROJECT};
