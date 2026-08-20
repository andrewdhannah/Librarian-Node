//! M0A startup probe — deterministic 6-phase startup protocol entrypoint.
//!
//! Deliberately SEPARATE from the hardened router (`src/main.rs`,
//! ROUTER-RUST-HARDEN-1): the first production runtime artifact must not be
//! coupled to HTTP/service concerns (RUST-MIGRATION-M0A work order, §Scope).
//!
//! Behavior: resolve the runtime node directory + governance sync → invoke the
//! core `StartupEngine` (via the shared `librarian_node::startup` adapter) →
//! emit the canonical startup receipt into the evidence directory → exit 0 on
//! GOVERNED_EXECUTION, 1 otherwise.
//!
//! Since M0B the probe shares its startup path with the router
//! (`librarian_node::startup::run_node_startup`) — no parallel implementation.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;
use librarian_node::startup::{NodeStartupOptions, run_node_startup};

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

    let outcome = run_node_startup(&NodeStartupOptions {
        node_dir: args.node_dir,
        governance_sync: args.governance_sync,
        capability_db: args.db_path,
        evidence_dir: args.evidence_dir,
        platform: args.platform,
        governance_commit: args.governance_commit,
    })
    .context("run node startup protocol")?;
    let receipt = &outcome.receipt;

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

    if receipt.status == "GOVERNED_EXECUTION" {
        Ok(())
    } else {
        bail!(
            "startup failed: {} checks failed",
            receipt.checks_failed
        )
    }
}
