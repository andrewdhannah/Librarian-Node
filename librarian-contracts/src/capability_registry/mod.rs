//! # Capability Registry Contracts
//!
//! Contract types for the Capability Registry MCP tools.
//! Mirrors the canonical contract defined in
//! `docs/contracts/CAPABILITY-REGISTRY-MCP-CONTRACT.md` (Swift Librarian).
//!
//! This module defines the portable contract layer for capability registry
//! operations: search, list, resolve, load, import, and evidence queries.
//!
//! ## Design Principles
//!
//! - Types are serializable to JSON — every runtime (Swift, Rust, etc.)
//!   must produce/consume identical JSON.
//! - No database, no runtime, no authority logic — contracts only.
//! - Required fields are non-Option. Optional fields use `Option<T>`.

pub mod types;
pub mod search;
pub mod resolve;
pub mod load;
pub mod import_skill;
pub mod evidence;

pub use types::*;
pub use search::{SearchRequest, SearchResponse, ListRequest, ListResponse};
pub use resolve::{ResolveRequest, ResolveResponse, CapabilityResolution, ResolveReceipt};
pub use load::{LoadRequest, LoadResponse, CapabilityContext, CapabilityIdentity, CapabilityInstructions, CapabilityGovernance, LoadReceipt};
pub use import_skill::{ImportRequest, ImportResponse};
pub use evidence::*;
