//! # librarian-contracts
//!
//! Neutral packet contracts for Librarian Core/Node communication.
//! No database, no runtime, no authority logic.
//!
//! This crate defines the portable contract layer that every Librarian
//! platform implementation (Swift, Rust, etc.) implements. Contracts are
//! serializable, versioned, and platform-neutral.
//!
//! ## Base Modules
//!
//! - `identity` — Node and platform identity types
//! - `lifecycle` — Lifecycle states, cursors, and transitions (governance plane)
//! - `residency` — Residency states for runtime instance tracking (execution plane)
//! - `residency_status` — Windows→Mac residency query/response packets
//! - `evidence` — Evidence record types
//! - `receipts` — Governance receipt types
//! - `custody` — Custody envelopes, chains, and operations
//! - `capabilities` — Capability declarations
//! - `errors` — Contract-level error types
//! - `serialization` — Deterministic serialization utilities
//! - `common` — Shared packet types (bridge identity, execution, lease)
//!
//! ## Domain Modules
//!
//! - `node` — Node identity, registration, capabilities, hardware, state
//! - `registry`, `registry_apply`, `registry_enforcement`, `registry_mcp`, `registry_owner` — registry lifecycle
//! - `fleet`, `fleet_trust` — fleet inventory, health, and trust
//! - `allocation`, `owner_allocation` — capability allocation and owner review
//! - `workload_session`, `workload_lifecycle`, `session` — workload execution
//! - `evidence_packet`, `evidence_classification`, `evidence_intelligence`, `capability_evidence` — evidence plane
//! - `pattern_escalation`, `anomaly_detection` — operational intelligence
//! - `owner_insight`, `owner_workflows`, `operations` — owner-facing views
//! - `reconciliation`, `recovery_custody` — state reconciliation
//! - `model_runtime`, `core_integration`, `policy`, `qualification_request` — runtime integration
//! - `bootstrap` — bootstrap assessment and plan
//! - `bridge` — HTTP bridge client for canonical-side communication

pub mod identity;
pub mod lifecycle;
pub mod residency;
pub mod residency_status;
pub mod evidence;
pub mod receipts;
pub mod custody;
pub mod capabilities;
pub mod errors;
pub mod serialization;
pub mod common;

pub mod allocation;
pub mod anomaly_detection;
pub mod bootstrap;
pub mod bridge;
pub mod capability_evidence;
pub mod core_integration;
pub mod evidence_classification;
pub mod evidence_intelligence;
pub mod evidence_packet;
pub mod fleet;
pub mod fleet_trust;
pub mod model_runtime;
pub mod node;
pub mod operations;
pub mod owner_allocation;
pub mod owner_insight;
pub mod owner_workflows;
pub mod pattern_escalation;
pub mod policy;
pub mod qualification_request;
pub mod reconciliation;
pub mod recovery_custody;
pub mod registry;
pub mod registry_apply;
pub mod registry_enforcement;
pub mod registry_mcp;
pub mod registry_owner;
pub mod session;
pub mod workload_lifecycle;
pub mod workload_session;

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
