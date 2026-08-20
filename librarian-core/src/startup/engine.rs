//! Startup engine — executes the 6-phase protocol against a
//! [`StartupContext`] and produces the canonical [`StartupReceipt`].
//!
//! The engine holds no filesystem assumptions: all inputs arrive parsed in
//! the context. Phase failures are recorded in the receipt (`STARTUP_FAILED`,
//! `checks_failed > 0`), never thrown — the receipt is the protocol's
//! terminal artifact in both outcomes.

use anyhow::Result;
use chrono::Utc;
use librarian_contracts::node::{StartupCheck, StartupPhase, StartupReceipt, StartupStatus};

use super::capabilities::CapabilitiesFile;
use super::environment::{self, DatabaseContext};
use super::governance::GovernanceSync;
use super::identity::NodeIdentityFile;
use super::receipt;

/// Everything the startup protocol needs — supplied by the adapter (node
/// entrypoint or fixture harness).
#[derive(Debug, Clone)]
pub struct StartupContext {
    /// Parsed `node-identity.json`.
    pub identity: NodeIdentityFile,
    /// Parsed `governance-sync.json`.
    pub governance: GovernanceSync,
    /// Parsed `capabilities.json`.
    pub capabilities: CapabilitiesFile,
    /// Platform the node must declare (`windows` | `linux` | `macos`).
    pub expected_platform: String,
    /// Canonical governance commit the node must be bound to.
    pub expected_governance_commit: String,
    /// SQLite database context for environment validation.
    pub database: DatabaseContext,
}

/// Result of the startup protocol: the canonical receipt plus the per-phase
/// check audit trail.
#[derive(Debug, Clone)]
pub struct StartupOutcome {
    /// Canonical startup receipt (RECEIPT-SCHEMA-001).
    pub receipt: StartupReceipt,
    /// One check per phase, in protocol order.
    pub checks: Vec<StartupCheck>,
}

/// Deterministic 6-phase startup engine.
pub struct StartupEngine;

impl StartupEngine {
    /// Execute the 6-phase startup protocol and produce the canonical receipt.
    pub fn execute(context: &StartupContext) -> Result<StartupReceipt> {
        Ok(Self::run(context)?.receipt)
    }

    /// Execute the 6-phase startup protocol, returning the receipt and the
    /// per-phase check audit trail.
    pub fn run(context: &StartupContext) -> Result<StartupOutcome> {
        let mut checks: Vec<StartupCheck> = Vec::with_capacity(6);
        let mut passed: u32 = 0;
        let mut failed: u32 = 0;

        // Phase 1 — identity_loading
        let (ok, detail) = context.identity.verify(&context.expected_platform);
        record(&mut checks, &mut passed, &mut failed, StartupPhase::IdentityLoading, ok, detail);

        // Phase 2 — governance_verification
        let (ok, detail) = context.governance.verify(&context.expected_governance_commit);
        record(
            &mut checks,
            &mut passed,
            &mut failed,
            StartupPhase::GovernanceVerification,
            ok,
            detail,
        );

        // Phase 3 — capability_loading
        let (ok, detail) = context.capabilities.verify();
        record(&mut checks, &mut passed, &mut failed, StartupPhase::CapabilityLoading, ok, detail);

        // Phase 4 — environment_validation
        let (ok, detail) = environment::verify(&context.database);
        record(
            &mut checks,
            &mut passed,
            &mut failed,
            StartupPhase::EnvironmentValidation,
            ok,
            detail,
        );

        // Phase 5 — receipt_generation (construct + validate against
        // RECEIPT-SCHEMA-001; structural construction cannot fail, so the
        // check outcome is the schema validation result).
        let now = Utc::now();
        let rid = receipt::receipt_id(&context.expected_platform, now);
        let provisional_status = if failed == 0 {
            StartupStatus::GovernedExecution
        } else {
            StartupStatus::StartupFailed
        };
        let provisional = receipt::build(
            rid.clone(),
            context.identity.node_id.clone(),
            context.expected_platform.clone(),
            context.expected_governance_commit.clone(),
            &checks,
            passed,
            failed,
            provisional_status,
            now,
        );
        let (ok, detail) = match provisional.validate() {
            Ok(()) => (true, "startup receipt conforms to RECEIPT-SCHEMA-001".to_string()),
            Err(e) => (false, format!("startup receipt validation failed: {e}")),
        };
        record(&mut checks, &mut passed, &mut failed, StartupPhase::ReceiptGeneration, ok, detail);

        // Phase 6 — governed_mode
        let success = failed == 0;
        let (ok, detail) = if success {
            (true, "node entered governed execution".to_string())
        } else {
            (
                false,
                format!("governed mode not entered: {failed} startup checks failed"),
            )
        };
        record(&mut checks, &mut passed, &mut failed, StartupPhase::GovernedMode, ok, detail);

        // Final receipt with complete check counts.
        let status = if success {
            StartupStatus::GovernedExecution
        } else {
            StartupStatus::StartupFailed
        };
        let receipt = receipt::build(
            rid,
            context.identity.node_id.clone(),
            context.expected_platform.clone(),
            context.expected_governance_commit.clone(),
            &checks,
            passed,
            failed,
            status,
            now,
        );

        Ok(StartupOutcome { receipt, checks })
    }
}

/// Append a phase check and update the pass/fail counters.
fn record(
    checks: &mut Vec<StartupCheck>,
    passed: &mut u32,
    failed: &mut u32,
    phase: StartupPhase,
    ok: bool,
    detail: String,
) {
    if ok {
        *passed += 1;
    } else {
        *failed += 1;
    }
    checks.push(StartupCheck {
        phase,
        passed: ok,
        detail,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failing_context(db_path: std::path::PathBuf) -> StartupContext {
        // Identity with a wrong platform forces phase 1 failure.
        StartupContext {
            identity: NodeIdentityFile {
                node_type: "librarian-runtime-node".to_string(),
                node_id: "WINPC-BIG-PICKLE".to_string(),
                authority: "owner-controlled".to_string(),
                platform: "linux".to_string(),
                governance_commit: "6be76216a8048492526c4ca0ae751b6d2d507185".to_string(),
                state: "GOVERNED_EXECUTION".to_string(),
                capabilities: vec!["governance_read".to_string()],
                created_at: "2026-07-24T22:18:00.0000000Z".to_string(),
            },
            governance: GovernanceSync {
                source: "github.com/andrewdhannah/Librarian-Node".to_string(),
                verification_status: "verified".to_string(),
                load_status: "complete".to_string(),
                node_loaded: true,
                contracts_loaded: true,
                core_loaded: true,
                sync_status: "complete".to_string(),
                last_verified_commit: "6be76216a8048492526c4ca0ae751b6d2d507185".to_string(),
            },
            capabilities: CapabilitiesFile::default(),
            expected_platform: "windows".to_string(),
            expected_governance_commit: "6be76216a8048492526c4ca0ae751b6d2d507185".to_string(),
            database: DatabaseContext {
                path: db_path,
                schema_sql: super::super::canonical_schema(),
            },
        }
    }

    #[test]
    fn failed_phase_produces_failed_receipt_not_error() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = failing_context(tmp.path().join("runtime-node.db"));
        let outcome = StartupEngine::run(&ctx).unwrap();
        let receipt = &outcome.receipt;
        assert_eq!(receipt.status, StartupStatus::StartupFailed.as_str());
        assert_eq!(receipt.startup_phase, "failed");
        assert!(receipt.checks_failed >= 1);
        assert!(!receipt.identity_loaded);
        assert_eq!(outcome.checks.len(), 6);
        // Every phase still recorded.
        assert_eq!(
            outcome.checks.iter().map(|c| c.phase).collect::<Vec<_>>(),
            StartupPhase::ALL.to_vec()
        );
    }
}
