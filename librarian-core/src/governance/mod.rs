//! # Governance Core
//!
//! Portable governance algorithm implementations for The Librarian platform.
//! This module implements the governance primitives that the Swift Core
//! provides on macOS, now available as a cross-platform Rust crate.
//!
//! All types use the portable contract definitions from `librarian-contracts`.
//! No platform-specific assumptions. No runtime dependencies beyond SQLite.
//!
//! ## Modules
//!
//! - `db` — SQLite-backed canonical state persistence
//! - `cursor` — Lifecycle cursor engine (transitions, not just states)
//! - `custody` — Custody protocol (check-out, check-in, integrity verification)
//! - `evidence` — Evidence generation using contract types
//! - `receipts` — Receipt generation using contract types
//! - `equivalence` — Equivalence check harness for Swift↔Rust comparison

pub mod db;
pub mod cursor;
pub mod custody;
pub mod evidence;
pub mod receipts;
pub mod equivalence;
pub mod qualification;
pub mod runtime;
