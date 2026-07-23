//! Qualification lifecycle — durable history and state tracking.
//!
//! Tracks models from discovery through qualification, approval, active
//! use, deprecation, and retirement.
//!
//! Critical invariants:
//!   Lifecycle state changes require explicit authority events.
//!   Evidence, observability, provenance, and review packages CANNOT
//!   change lifecycle state. Only Owner decisions can promote to
//!   Approved, Deprecated, or Retired states.
//!   Historical transitions are immutable.

pub mod history;
pub mod models;
pub mod transitions;

pub use history::LifecycleHistory;
pub use models::{LifecycleAuthority, LifecycleEvent, LifecycleRecord, LifecycleState};
pub use transitions::{LifecycleTransition, TransitionError};
