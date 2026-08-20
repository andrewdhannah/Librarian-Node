//! Phase 1 — identity loading.
//!
//! Parses and verifies the runtime node directory `node-identity.json` shape
//! (as anchored by `conformance/fixtures/startup/canonical-startup-input.json`).

use serde::Deserialize;

use librarian_contracts::node::startup::is_sha256_hex;

/// Runtime node type required by the startup contract.
pub const EXPECTED_NODE_TYPE: &str = "librarian-runtime-node";
/// Node state required for governed startup.
pub const EXPECTED_STATE: &str = "GOVERNED_EXECUTION";

/// `node-identity.json` (runtime node directory shape).
#[derive(Debug, Clone, Deserialize)]
pub struct NodeIdentityFile {
    /// Node type discriminator.
    pub node_type: String,
    /// Unique node identifier.
    pub node_id: String,
    /// Governance authority binding.
    pub authority: String,
    /// Node platform.
    pub platform: String,
    /// Governance commit the node is bound to.
    pub governance_commit: String,
    /// Node state.
    pub state: String,
    /// Declared capability names.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Node creation timestamp.
    pub created_at: String,
}

impl NodeIdentityFile {
    /// Verify identity fields against the startup contract.
    ///
    /// Returns `(passed, detail)` — a failed check never returns `Err`;
    /// failure is recorded in the startup receipt.
    pub fn verify(&self, expected_platform: &str) -> (bool, String) {
        if self.node_type != EXPECTED_NODE_TYPE {
            return (
                false,
                format!("node_type '{}' != '{EXPECTED_NODE_TYPE}'", self.node_type),
            );
        }
        if self.node_id.is_empty() {
            return (false, "node_id must not be empty".to_string());
        }
        if self.platform != expected_platform {
            return (
                false,
                format!(
                    "platform '{}' != expected '{expected_platform}'",
                    self.platform
                ),
            );
        }
        if !is_sha256_hex(&self.governance_commit) {
            return (
                false,
                format!(
                    "governance_commit '{}' is not a 40-char hex SHA",
                    self.governance_commit
                ),
            );
        }
        if self.state != EXPECTED_STATE {
            return (false, format!("state '{}' != '{EXPECTED_STATE}'", self.state));
        }
        if self.capabilities.is_empty() {
            return (false, "capabilities list must not be empty".to_string());
        }
        (
            true,
            format!("node identity {} loaded on {}", self.node_id, self.platform),
        )
    }
}
