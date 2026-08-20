//! Phase 3 — capability loading.
//!
//! Verifies the node capability set (`capabilities.json`) contains all
//! required capabilities for governed execution.

use std::collections::HashMap;

use serde::Deserialize;

/// Capabilities required for governed execution (per the canonical
/// `capabilities.json` anchored by the M0A fixture).
pub const REQUIRED_CAPABILITIES: [&str; 6] = [
    "governance_read",
    "governance_verify",
    "execution_allowed",
    "evidence_generation",
    "custody_tracking",
    "receipt_validation",
];

/// `capabilities.json` (flat name → enabled map).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CapabilitiesFile {
    /// Capability name → enabled flag.
    #[serde(flatten)]
    pub flags: HashMap<String, bool>,
}

impl CapabilitiesFile {
    /// Verify all required capabilities are present and enabled.
    ///
    /// Returns `(passed, detail)` — a failed check never returns `Err`;
    /// failure is recorded in the startup receipt.
    pub fn verify(&self) -> (bool, String) {
        let missing: Vec<&str> = REQUIRED_CAPABILITIES
            .iter()
            .copied()
            .filter(|name| !self.flags.get(*name).copied().unwrap_or(false))
            .collect();
        if missing.is_empty() {
            (
                true,
                format!(
                    "all {} required capabilities present and enabled",
                    REQUIRED_CAPABILITIES.len()
                ),
            )
        } else {
            (
                false,
                format!(
                    "missing or disabled required capabilities: {}",
                    missing.join(", ")
                ),
            )
        }
    }
}
