//! Release Trust Foundation — evidence-based release verification.
//!
//! Produces deterministic evidence that a release is composed exclusively
//! of authorized, validated, sealed work with complete provenance.
//!
//! Critical invariant:
//!   The Release Trust layer reports facts only.
//!   It does NOT approve, recommend, or decide releases.
//!   The Owner remains the sole release authority.

pub mod manifest;
pub mod provenance;
pub mod trust_package;
pub mod validation;

pub use manifest::{ReleaseComponent, ReleaseManifest, ReleaseVersion};
pub use provenance::ReleaseProvenance;
pub use trust_package::ReleaseTrustPackage;
pub use validation::{ReleaseValidation, ValidationIssue, ValidationResult, ValidationSummary};
