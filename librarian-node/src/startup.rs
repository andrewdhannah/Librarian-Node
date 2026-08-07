//! Node-level startup module (RUST-MIGRATION-M0B).
//!
//! Adapter layer between the process and the core [`StartupEngine`]: resolves
//! input paths, loads `node-identity.json` / `capabilities.json` /
//! `governance-sync.json`, invokes the deterministic 6-phase protocol, seals
//! the [`StartupOutcome`], and emits the canonical receipt into the evidence
//! directory (append-only, millisecond-stamped filenames).
//!
//! Single code path shared by the router (`src/main.rs`) and the diagnostic
//! probe (`src/bin/startup_probe.rs`) — no parallel startup implementations.
//!
//! Failure semantics: the engine never throws on a failed check; it records the
//! failure in the receipt. Callers (both the router and the probe) MUST treat
//! a non-`GOVERNED_EXECUTION` receipt as fatal and exit before binding.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use librarian_core::startup::{
    canonical_schema, CapabilitiesFile, DatabaseContext, GovernanceSync, NodeIdentityFile,
    StartupContext, StartupEngine, StartupOutcome,
};
use serde::de::DeserializeOwned;

/// Startup inputs resolved at the node process level.
#[derive(Debug, Clone)]
pub struct NodeStartupOptions {
    /// Directory containing `node-identity.json` and `capabilities.json`.
    pub node_dir: PathBuf,
    /// Path to `governance-sync.json`.
    pub governance_sync: PathBuf,
    /// SQLite database path for the capability registry (canonical schema applied).
    pub capability_db: PathBuf,
    /// Evidence output directory (receipt written here, append-only).
    pub evidence_dir: PathBuf,
    /// Expected platform (windows | linux | macos).
    pub platform: String,
    /// Expected canonical governance commit (40-hex SHA).
    pub governance_commit: String,
}

/// Platform for the build target (windows | linux | macos).
pub fn default_platform() -> String {
    if cfg!(target_os = "windows") {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else {
        "linux".to_string()
    }
}

/// Resolve governed startup inputs from CLI overrides and config defaults.
///
/// `base` is the evidence/data directory used to derive defaults when the
/// caller does not pass explicit paths (the router passes `config.evidence_path`).
pub fn resolve_options(
    node_dir: Option<PathBuf>,
    governance_sync: Option<PathBuf>,
    capability_db: Option<PathBuf>,
    evidence_dir: Option<PathBuf>,
    platform: Option<String>,
    governance_commit: Option<String>,
    base: Option<&Path>,
) -> NodeStartupOptions {
    let base = base.unwrap_or_else(|| Path::new("data"));
    NodeStartupOptions {
        node_dir: node_dir.unwrap_or_else(|| base.to_path_buf()),
        governance_sync: governance_sync
            .unwrap_or_else(|| base.join("governance-sync.json")),
        capability_db: capability_db
            .unwrap_or_else(|| base.join("capability-registry.sqlite")),
        evidence_dir: evidence_dir.unwrap_or_else(|| base.join("evidence")),
        platform: platform.unwrap_or_else(default_platform),
        governance_commit: governance_commit
            .unwrap_or_default(),
    }
}

/// Run the governed 6-phase startup protocol and seal the outcome.
///
/// Writes the canonical receipt into `options.evidence_dir` (append-only,
/// millisecond-stamped filename — the receipt_id itself is second-precision,
/// so the stamp keeps evidence unique even for same-second runs).
///
/// Returns the sealed outcome regardless of pass/fail; callers inspect
/// `receipt.status` and exit 1 pre-bind unless it is `GOVERNED_EXECUTION`.
pub fn run_node_startup(options: &NodeStartupOptions) -> Result<StartupOutcome> {
    let identity: NodeIdentityFile = read_json(&options.node_dir.join("node-identity.json")).with_context(
        || format!("load {}", options.node_dir.join("node-identity.json").display()),
    )?;
    let capabilities: CapabilitiesFile = read_json(&options.node_dir.join("capabilities.json")).with_context(
        || format!("load {}", options.node_dir.join("capabilities.json").display()),
    )?;
    let governance: GovernanceSync = read_json(&options.governance_sync)
        .with_context(|| format!("load {}", options.governance_sync.display()))?;

    let context = StartupContext {
        identity,
        governance,
        capabilities,
        expected_platform: options.platform.clone(),
        expected_governance_commit: options.governance_commit.clone(),
        database: DatabaseContext {
            path: options.capability_db.clone(),
            schema_sql: canonical_schema(),
        },
    };

    let outcome = StartupEngine::run(&context)?;

    std::fs::create_dir_all(&options.evidence_dir).context("create evidence dir")?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
    let evidence_path = options
        .evidence_dir
        .join(format!("startup-receipt-{stamp}.json"));
    let json = serde_json::to_string_pretty(&outcome.receipt).context("serialize receipt")?;
    std::fs::write(&evidence_path, json).context("write receipt evidence")?;

    Ok(outcome)
}

/// Load and parse a JSON file into a `DeserializeOwned` target.
fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}
