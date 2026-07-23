//! Observability surface — read-only aggregation of qualification state.
//!
//! This module provides structured summaries of qualification activity
//! without introducing any mutation, decision, or routing authority.
//!
//! Critical invariant:
//!   Observability displays evidence. It does NOT approve, reject, route,
//!   decide, or mutate any state.

pub mod models;
pub mod service;

pub use models::{
    BatchExecutionSummary, EvidenceSummaryView, ObservabilityReport, QualificationRunSummary,
    RuntimeHealth,
};
pub use service::ObservabilityService;
