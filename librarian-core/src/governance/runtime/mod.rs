//! # Runtime Supervision
//!
//! Cross-platform runtime supervision that maps process lifecycle events
//! to the existing governance substrate. No platform-specific types.
//!
//! Maps:
//!   Process start      → ResidencyState::Loading → Active
//!   Process healthy    → ResidencyState::Active
//!   Process degraded   → ResidencyState::Active (with degraded evidence)
//!   Process stop       → ResidencyState::Releasing → Released
//!   Process crash      → ResidencyState::Failed
//!   Process blocked    → ResidencyState::Blocked
//!
//! Each transition produces Evidence and, for lifecycle boundaries, Receipts.
//! The runtime supervisor is platform-agnostic — platform adapters implement
//! the actual process management interface.

pub mod supervisor;
pub mod adapter;
pub mod linux;
