//! Persistent registry — bounded persistence for qualification routing state.
//!
//! The registry persists approved capability manifests, Owner decisions,
//! execution profiles, router projections, rejection records, and
//! supersession records to a JSON file using atomic writes.
//!
//! Critical invariant:
//!   Persistence may restore previously approved routing state.
//!   Persistence may NOT recreate authority from raw performance evidence.
//!
//! Architecture:
//!   - RegistryStore: handles file I/O with atomic writes
//!   - RegistryState: validated in-memory state after load
//!   - RegistryFile: serialized format (JSON with schema version)
//!   - RegistryLoadResult: distinguishes Loaded/Empty/Incompatible/Corrupt
//!
//! Schema versioning prevents silent acceptance of incompatible formats.
//! Content hash validation ensures records are not corrupted during persistence.
//! Authority chain validation ensures projections remain traceable to their
//! required provenance (manifest → Owner decision → execution profile).

pub mod store;

pub use store::*;
