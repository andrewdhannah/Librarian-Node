//! Evidence provenance — traceability across the qualification lifecycle.
//!
//! Every evidence record carries provenance metadata linking it back to
//! its originating qualification context: model, run, validators, custom
//! rules, and batch execution.
//!
//! Critical invariant:
//!   Evidence provenance enables traceability.
//!   It does NOT enable authority.
//!   Provenance metadata has no capability, decision, or routing fields.

pub mod builder;
pub mod models;

pub use builder::ProvenanceBuilder;
pub use models::{
    BatchProvenance, CustomEvidenceRef, EvidenceProvenance, ExecutionContext, ProvenanceSource,
    ValidatorProvenance,
};
