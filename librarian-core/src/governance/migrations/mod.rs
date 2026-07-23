//! # Governance Database Migrations
//!
//! Numbered migration framework for the governance database.
//! Every schema change is a numbered migration with up/down SQL,
//! recorded as evidence and receipt.
//!
//! The migration framework itself creates two meta-tables:
//! - `schema_version` — Tracks current schema version
//! - `migration_log` — Append-only log of every migration applied
//!
//! Migrations are deterministic and ordered. The runner applies
//! pending migrations on database open.

pub mod framework;
pub mod migrations_list;
