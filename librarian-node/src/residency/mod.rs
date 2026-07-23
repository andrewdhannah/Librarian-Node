//! Residency supervisor — enforces single-model GPU residency on the Windows runtime node.
//!
//! The supervisor guarantees that the qualified llama.cpp runtime never intentionally
//! operates more than one GPU-resident model process at a time, serializes residency
//! transitions, persists leases and runs through the operational DB, and reconciles
//! runtime truth after failures or restarts.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │          ModelResidencySupervisor                │
//! │  (Arc<tokio::sync::Mutex<SupervisorState>>)     │
//! ├─────────────────────────────────────────────────┤
//! │  acquire_model()  → Loading → Ready             │
//! │  start_run()      → Running                     │
//! │  drain()          → Draining → Unloading        │
//! │  release_model()  → VerifyingRelease → Unloaded │
//! │  reconcile()      → startup recovery            │
//! ├─────────────────────────────────────────────────┤
//! │  Composes BackendProcess for process lifecycle   │
//! │  Uses RuntimeDatabase for persistence            │
//! │  Produces LifecycleEvidence for audit trail      │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! # Invariants
//!
//! - Maximum active supervised model processes: 1
//! - Maximum active residency leases: 1
//! - A run cannot begin without an appropriate active lease
//! - Release must be verified (PID exit + GPU memory) before new model can load

pub mod reconciliation;
pub mod state;
pub mod supervisor;

pub use state::{ResidencyState, RuntimeStopStrategy, StateTransitionError, validate_transition};
pub use supervisor::{ModelResidencySupervisor, SupervisorConfig, ResidencySnapshot};
