//! librarian-contracts — neutral packet contracts for LibrarianOS Core/Node communication.
//!
//! This crate contains the sealed packet types that cross the authority boundary
//! between Core (canonical authority) and Node (execution runtime).
//!
//! It contains NO:
//! - Database logic
//! - Runtime or process management
//! - Authority decision making
//! - HTTP server or endpoint definitions
//!
//! Both `librarian-core` and `librarian-node` depend on this crate.
//! Neither owns it. It is the neutral contract layer.

pub mod allocation;
pub mod bootstrap;
pub mod policy;
pub mod common;
pub mod core_integration;
pub mod evidence_packet;
pub mod fleet;
pub mod node;
pub mod operations;
pub mod registry;
pub mod owner_allocation;
pub mod owner_workflows;
pub mod qualification_request;
pub mod residency_status;
pub mod bridge;
pub mod capability_evidence;
pub mod custody;
pub mod evidence_classification;
pub mod evidence_intelligence;
pub mod session;
pub mod workload_lifecycle;
pub mod workload_session;
pub mod anomaly_detection;
pub mod owner_insight;
pub mod pattern_escalation;
pub mod recovery_custody;
pub mod reconciliation;
pub mod model_runtime;
pub mod fleet_trust;
pub mod registry_apply;
pub mod registry_enforcement;
pub mod registry_mcp;
pub mod registry_owner;

pub use common::*;
pub use evidence_packet::EvidencePacket;
pub use qualification_request::QualificationRequest;
pub use residency_status::{ActiveLease, ActiveRun, ResidencyStatusQuery, ResidencyStatusResponse};
