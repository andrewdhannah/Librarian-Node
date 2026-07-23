//! HTTP bridge — canonical-side client for Windows evidence and residency APIs.
//!
//! This module provides a bounded HTTP client that communicates with the
//! Windows runtime node using the sealed F3 bridge packet contracts.
//!
//! The bridge client:
//! - Makes real HTTP requests over the network
//! - Deserializes JSON responses into sealed packet types
//! - Validates packet integrity
//! - Preserves raw transport errors
//!
//! It does NOT:
//! - Interpret capability authority
//! - Make qualification decisions
//! - Execute automatic retries that hide failures
//! - Add capability fields to transported packets

pub mod client;

pub use client::{BridgeClient, BridgeError, EvidenceRunResponse, LifecycleEvent, LifecycleResponse};
