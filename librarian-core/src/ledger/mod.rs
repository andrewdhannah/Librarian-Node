//! Sprint ledger governance — tamper-resistant, auditable execution history.
//!
//! Provides programmatic records for sprint authorization, completion,
//! state transitions, and governance validation. The ledger is the
//! source of truth for what work was authorized, what was delivered,
//! and what state each sprint achieved.

pub mod models;
pub mod store;
pub mod validation;

pub use models::{
    GovernanceReceipt, SprintAuthorization, SprintLedger, SprintReceipt, SprintState,
};
pub use store::LedgerStore;
pub use validation::{LedgerValidation, TransitionError};
