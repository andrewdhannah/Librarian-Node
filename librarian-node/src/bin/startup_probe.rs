//! M0A startup probe — deterministic 6-phase startup protocol entrypoint.
//!
//! Deliberately SEPARATE from the hardened router (`src/main.rs`,
//! ROUTER-RUST-HARDEN-1): the first production runtime artifact must not be
//! coupled to HTTP/service concerns (RUST-MIGRATION-M0A work order, §Scope).
//!
//! Behavior: load runtime node directory + governance sync → invoke the
//! core `StartupEngine` → emit the canonical startup receipt into the
//! evidence directory → exit 0 on GOVERNED_EXECUTION, 1 otherwise.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;
use librarian_core::startup::{
    canonical_schema, CapabilitiesFile, DatabaseContext, GovernanceSync, NodeIdentityFile,
    StartupContext, StartupEngine,
};
use serde::de::DeserializeOwned;

#[derive(Parser)]
#[command(
    name = "startup_probe",
    about = "Execute the 6-phase startup protocol and emit the canonical startup receipt",
    version
)]
struct Args {
    /// Directory containing node-identity.json and capabilities.json.
    #[arg(long)]
    node_dir: PathBuf,

    /// Path to governance-sync.json.
    #[arg(long)]
    governance_sync: PathBuf,

    /// SQLite database path (created if missing; canonical schema applied).
    #[arg(long)]
    db_path: PathBuf,

    /// Evidence output directory (receipt written here, append-only).
    #[arg(long)]
    evidence_dir: PathBuf,

    /// Expected platform.
    #[arg(long)]
    platform: String,

    /// Expected canonical governance commit (40-hex SHA).
    #[arg(long)]
    governance_commit: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let identity: NodeIdentityFile = read_json(&args.node_dir.join("node-identity.json"))
        .with_context(|| format!("load {}", args.node_dir.join("node-identity.json").display()))?;
    let capabilities: CapabilitiesFile = read_json(&args.node_dir.join("capabilities.json"))
        .with_context(|| format!("load {}", args.node_dir.join("capabilities.json").display()))?;
    let governance: GovernanceSync = read_json(&args.governance_sync)
        .with_context(|| format!("load {}", args.governance_sync.display()))?;

    let context = StartupContext {
        identity,
        governance,
        capabilities,
        expected_platform: args.platform,
        expected_governance_commit: args.governance_commit,
        database: DatabaseContext {
            path: args.db_path,
            schema_sql: canonical_schema(),
        },
    };

    let outcome = StartupEngine::run(&context)?;
    let receipt = &outcome.receipt;

    // Evidence emission (append-only; millisecond component keeps filenames
    // unique even for same-second runs, since receipt_id is second-precision).
    std::fs::create_dir_all(&args.evidence_dir).context("create evidence dir")?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
    let evidence_path = args
        .evidence_dir
        .join(format!("startup-receipt-{stamp}.json"));
    let json = serde_json::to_string_pretty(receipt).context("serialize receipt")?;
    std::fs::write(&evidence_path, json).context("write receipt evidence")?;

    println!("=== M0A startup protocol (RUST-MIGRATION-M0A) ===");
    for check in &outcome.checks {
        let mark = if check.passed { "PASS" } else { "FAIL" };
        println!("[{mark}] {}: {}", check.phase.as_str(), check.detail);
    }
    println!(
        "checks: {}/{} passed",
        receipt.checks_passed, receipt.checks_passed + receipt.checks_failed
    );
    println!("status: {}", receipt.status);
    println!("receipt_id: {}", receipt.receipt_id);
    println!("receipt: {}", evidence_path.display());

    if receipt.status == "GOVERNED_EXECUTION" {
        Ok(())
    } else {
        bail!(
            "startup failed: {} checks failed (see {}", 
            receipt.checks_failed,
            evidence_path.display()
        )
    }
}

/// Load and parse a JSON file into a DeserializeOwned target.
fn read_json<T: DeserializeOwned>(path: &std::path::Path) -> Result<T> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}
