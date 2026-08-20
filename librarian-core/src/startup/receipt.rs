//! Phase 5 — receipt generation.
//!
//! Constructs the canonical startup receipt (RECEIPT-SCHEMA-001).
//! `receipt_id` and `timestamp` are variable fields; everything else is
//! deterministic and derived from the executed checks.

use chrono::{DateTime, SecondsFormat, Utc};
use librarian_contracts::node::{StartupCheck, StartupPhase, StartupReceipt, StartupStatus};

/// Build the reference-format receipt id: `WINDOWS-STARTUP-20260724-223333`.
pub fn receipt_id(platform: &str, now: DateTime<Utc>) -> String {
    format!(
        "{}-STARTUP-{}",
        platform.to_uppercase(),
        now.format("%Y%m%d-%H%M%S")
    )
}

/// RFC 3339 timestamp with seconds precision: `2026-07-25T02:33:33Z`.
pub fn timestamp(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Construct the canonical startup receipt from executed check outcomes.
pub fn build(
    receipt_id: String,
    node_id: String,
    platform: String,
    governance_commit: String,
    checks: &[StartupCheck],
    checks_passed: u32,
    checks_failed: u32,
    status: StartupStatus,
    now: DateTime<Utc>,
) -> StartupReceipt {
    let identity_loaded = phase_passed(checks, StartupPhase::IdentityLoading);
    let governance_verified = phase_passed(checks, StartupPhase::GovernanceVerification);
    let capabilities_loaded = phase_passed(checks, StartupPhase::CapabilityLoading);
    let environment_validated = phase_passed(checks, StartupPhase::EnvironmentValidation);

    StartupReceipt {
        receipt_id,
        node_id,
        platform,
        governance_commit,
        startup_phase: if status == StartupStatus::GovernedExecution {
            "complete".to_string()
        } else {
            "failed".to_string()
        },
        identity_loaded,
        governance_verified,
        capabilities_loaded,
        environment_validated,
        checks_passed,
        checks_failed,
        status: status.as_str().to_string(),
        timestamp: timestamp(now),
    }
}

/// Whether the check for the given phase passed.
fn phase_passed(checks: &[StartupCheck], phase: StartupPhase) -> bool {
    checks
        .iter()
        .find(|check| check.phase == phase)
        .map(|check| check.passed)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use librarian_contracts::node::startup::{is_receipt_id, is_sha256_hex};

    fn fixed_clock() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 25, 2, 33, 33).unwrap()
    }

    #[test]
    fn test_receipt_id_reference_format() {
        assert_eq!(
            receipt_id("windows", fixed_clock()),
            "WINDOWS-STARTUP-20260725-023333"
        );
        assert!(is_receipt_id(&receipt_id("windows", fixed_clock())));
    }

    #[test]
    fn test_timestamp_reference_format() {
        assert_eq!(timestamp(fixed_clock()), "2026-07-25T02:33:33Z");
    }

    #[test]
    fn test_receipt_id_uses_uppercase_platform() {
        let rid = receipt_id("windows", fixed_clock());
        assert!(rid.starts_with("WINDOWS-"));
        assert!(is_receipt_id(&rid));
    }

    #[test]
    fn test_governance_commit_hex() {
        assert!(is_sha256_hex("6be76216a8048492526c4ca0ae751b6d2d507185"));
    }
}
