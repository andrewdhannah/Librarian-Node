//! # Startup Protocol Engine (M0A)
//!
//! Deterministic 6-phase startup: identity loading → governance verification →
//! capability loading → environment validation → receipt generation →
//! governed mode. Conformance target: C2 (Evidence Compatible).
//!
//! The engine holds no filesystem assumptions: all inputs arrive as parsed
//! values in [`StartupContext`]; file loading is the caller's (adapter) job.
//! SQLite handling is confined to [`environment`] and the canonical schema is
//! embedded (see `assets/schema/README.md`).

mod capabilities;
mod engine;
mod environment;
mod governance;
mod identity;
mod receipt;

pub use capabilities::{CapabilitiesFile, REQUIRED_CAPABILITIES};
pub use engine::{StartupContext, StartupEngine, StartupOutcome};
pub use environment::DatabaseContext;
pub use governance::GovernanceSync;
pub use identity::NodeIdentityFile;

/// Canonical capability-registry SQL schema (Phase 2: 9 tables + Phase 3: 2
/// tables), embedded byte-identical from TheLibrarian `@15c5ef2` (sprint-3).
/// All DDL is `CREATE TABLE IF NOT EXISTS` with no transaction wrappers, so
/// application is idempotent.
pub fn canonical_schema() -> String {
    let phase2 = include_str!("../../assets/schema/capability-registry-schema.sql");
    let phase3 = include_str!("../../assets/schema/capability-registry-schema-phase3.sql");
    format!("{phase2}\n{phase3}\n")
}
