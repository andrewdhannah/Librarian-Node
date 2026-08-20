//! Phase 2 — governance verification.
//!
//! Verifies the governance sync state (`governance-sync.json`) against the
//! canonical governance commit.

use serde::Deserialize;

use librarian_contracts::node::startup::is_sha256_hex;

/// Verification status required for governed startup.
pub const VERIFIED: &str = "verified";
/// Sync/load status required for governed startup.
pub const COMPLETE: &str = "complete";

/// `governance-sync.json` (runtime governance state).
#[derive(Debug, Clone, Deserialize)]
pub struct GovernanceSync {
    /// Governance source repository.
    pub source: String,
    /// Verification status (`verified` | otherwise).
    pub verification_status: String,
    /// Load status (`complete` | otherwise).
    pub load_status: String,
    /// Whether node contracts were loaded.
    pub node_loaded: bool,
    /// Whether contract documents were loaded.
    pub contracts_loaded: bool,
    /// Whether core documents were loaded.
    pub core_loaded: bool,
    /// Sync status (`complete` | otherwise).
    pub sync_status: String,
    /// Last verified governance commit.
    pub last_verified_commit: String,
}

impl GovernanceSync {
    /// Verify governance sync state against the expected canonical commit.
    ///
    /// Returns `(passed, detail)` — a failed check never returns `Err`;
    /// failure is recorded in the startup receipt.
    pub fn verify(&self, expected_commit: &str) -> (bool, String) {
        if self.verification_status != VERIFIED {
            return (
                false,
                format!(
                    "verification_status '{}' != '{VERIFIED}'",
                    self.verification_status
                ),
            );
        }
        if self.load_status != COMPLETE {
            return (
                false,
                format!("load_status '{}' != '{COMPLETE}'", self.load_status),
            );
        }
        if self.sync_status != COMPLETE {
            return (
                false,
                format!("sync_status '{}' != '{COMPLETE}'", self.sync_status),
            );
        }
        if !(self.node_loaded && self.contracts_loaded && self.core_loaded) {
            return (
                false,
                format!(
                    "load flags incomplete (node={} contracts={} core={})",
                    self.node_loaded, self.contracts_loaded, self.core_loaded
                ),
            );
        }
        if self.last_verified_commit != expected_commit {
            return (
                false,
                format!(
                    "last_verified_commit '{}' != expected '{expected_commit}'",
                    self.last_verified_commit
                ),
            );
        }
        if !is_sha256_hex(&self.last_verified_commit) {
            return (
                false,
                format!(
                    "last_verified_commit '{}' is not a 40-char hex SHA",
                    self.last_verified_commit
                ),
            );
        }
        (
            true,
            format!("governance verified at commit {}", self.last_verified_commit),
        )
    }
}
