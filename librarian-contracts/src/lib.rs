//! # librarian-contracts
//!
//! Neutral packet contracts for Librarian Core/Node communication.
//! No database, no runtime, no authority logic.
//!
//! This crate defines the portable contract layer that every Librarian
//! platform implementation (Swift, Rust, etc.) implements. Contracts are
//! serializable, versioned, and platform-neutral.
//!
//! ## Modules
//!
//! - `identity` — Node and platform identity types
//! - `lifecycle` — Lifecycle states, cursors, and transitions (governance plane)
//! - `residency` — Residency states for runtime instance tracking (execution plane)
//! - `evidence` — Evidence record types
//! - `receipts` — Governance receipt types
//! - `custody` — Custody envelopes and operations
//! - `capabilities` — Capability declarations
//! - `errors` — Contract-level error types
//! - `serialization` — Deterministic serialization utilities

pub mod identity;
pub mod lifecycle;
pub mod residency;
pub mod evidence;
pub mod receipts;
pub mod custody;
pub mod capabilities;
pub mod errors;
pub mod serialization;

pub mod prelude {
    pub use crate::identity::*;
    pub use crate::lifecycle::*;
    pub use crate::residency::*;
    pub use crate::evidence::*;
    pub use crate::receipts::*;
    pub use crate::custody::*;
    pub use crate::capabilities::*;
    pub use crate::errors::*;
    pub use crate::serialization::*;
}
