//! Owner Review Surface — consolidated qualification review package.
//!
//! Composes existing sealed data into a single deterministic review package
//! for Owner review. The review is strictly presentation-only:
//! - No mutation of source data
//! - No persistence side effects
//! - No decision, routing, or policy fields
//! - Reproducible output

pub mod builder;
pub mod models;

pub use builder::ReviewBuilder;
pub use models::{
    BatchReview, EvidenceReview, HealthReview, ProvenanceReview, QualificationReview,
    ReviewFinding, ReviewFindingSeverity, ReviewPackage,
};
