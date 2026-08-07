//! # Startup Receipt Contract
//!
//! Canonical startup receipt per `schemas/startup-receipt.schema.json`
//! (sealed suite ID `RECEIPT-SCHEMA-001`). Platform-neutral; produced at the
//! end of the 6-phase startup protocol (`contracts/startup/STARTUP-PROTOCOL.md`).
//!
//! Equivalence surface per `docs/architecture/THREE-WAY-EQUIVALENCE-PROTOCOL.md`:
//! `StartupReceiptFacts` are the deterministic fields that must match exactly
//! across runs and implementations; `receipt_id` and `timestamp` are variable.

use serde::{Deserialize, Serialize};

/// Canonical startup receipt — the output artifact of the 6-phase startup protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartupReceipt {
    /// Unique receipt identifier (pattern `^[A-Z0-9-]+$`).
    pub receipt_id: String,
    /// Unique node identifier.
    pub node_id: String,
    /// Node platform: `windows` | `linux` | `macos`.
    pub platform: String,
    /// Canonical governance commit SHA (40 hex chars).
    pub governance_commit: String,
    /// Startup phase: `complete` | `failed`.
    pub startup_phase: String,
    /// Whether identity was loaded successfully.
    pub identity_loaded: bool,
    /// Whether governance was verified successfully.
    pub governance_verified: bool,
    /// Whether capabilities were loaded successfully.
    pub capabilities_loaded: bool,
    /// Whether environment was validated successfully.
    pub environment_validated: bool,
    /// Number of checks that passed.
    pub checks_passed: u32,
    /// Number of checks that failed.
    pub checks_failed: u32,
    /// Node status after startup: `GOVERNED_EXECUTION` | `STARTUP_FAILED`.
    pub status: String,
    /// ISO-8601 timestamp of receipt generation.
    pub timestamp: String,
}

/// Deterministic facts of a startup receipt — the equivalence surface.
///
/// These fields must match exactly (per `STARTUP-PROTOCOL.md` §Equivalence
/// Requirements); `receipt_id` and `timestamp` are expected to differ.
///
/// `Serialize` is derived for the RUST-M0-0 drift-guard test; the type is NOT
/// part of the receipt wire format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupReceiptFacts {
    /// Unique node identifier.
    pub node_id: String,
    /// Node platform.
    pub platform: String,
    /// Canonical governance commit SHA.
    pub governance_commit: String,
    /// Startup phase.
    pub startup_phase: String,
    /// Whether identity was loaded.
    pub identity_loaded: bool,
    /// Whether governance was verified.
    pub governance_verified: bool,
    /// Whether capabilities were loaded.
    pub capabilities_loaded: bool,
    /// Whether environment was validated.
    pub environment_validated: bool,
    /// Number of checks passed.
    pub checks_passed: u32,
    /// Number of checks failed.
    pub checks_failed: u32,
    /// Node status after startup.
    pub status: String,
}

/// Startup protocol phases (`contracts/startup/STARTUP-PROTOCOL.md`).
///
/// Distinct from the receipt's `startup_phase` string, which is the schema
/// outcome (`complete` | `failed`); this type identifies which protocol step a
/// `StartupCheck` records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupPhase {
    /// Load and validate node identity.
    IdentityLoading,
    /// Verify governance sync state against the canonical commit.
    GovernanceVerification,
    /// Load and validate the node capability set.
    CapabilityLoading,
    /// Validate the runtime environment (SQLite + canonical schema).
    EnvironmentValidation,
    /// Construct and sign the startup receipt.
    ReceiptGeneration,
    /// Enter governed execution mode.
    GovernedMode,
}

impl StartupPhase {
    /// All six phases, in protocol order.
    pub const ALL: [StartupPhase; 6] = [
        StartupPhase::IdentityLoading,
        StartupPhase::GovernanceVerification,
        StartupPhase::CapabilityLoading,
        StartupPhase::EnvironmentValidation,
        StartupPhase::ReceiptGeneration,
        StartupPhase::GovernedMode,
    ];

    /// Serialized form (matches `serde(rename_all = "snake_case")`).
    pub fn as_str(self) -> &'static str {
        match self {
            StartupPhase::IdentityLoading => "identity_loading",
            StartupPhase::GovernanceVerification => "governance_verification",
            StartupPhase::CapabilityLoading => "capability_loading",
            StartupPhase::EnvironmentValidation => "environment_validation",
            StartupPhase::ReceiptGeneration => "receipt_generation",
            StartupPhase::GovernedMode => "governed_mode",
        }
    }
}

/// Terminal node status after startup (`GOVERNED_EXECUTION` | `STARTUP_FAILED`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StartupStatus {
    /// All startup checks passed; node is in governed execution.
    GovernedExecution,
    /// One or more startup checks failed; node did not enter governed mode.
    StartupFailed,
}

impl StartupStatus {
    /// Serialized form (matches `serde(rename_all = "SCREAMING_SNAKE_CASE")`).
    pub fn as_str(self) -> &'static str {
        match self {
            StartupStatus::GovernedExecution => "GOVERNED_EXECUTION",
            StartupStatus::StartupFailed => "STARTUP_FAILED",
        }
    }
}

/// Per-phase startup check record, captured for the receipt audit trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupCheck {
    /// Protocol phase this check belongs to.
    pub phase: StartupPhase,
    /// Whether the phase check passed.
    pub passed: bool,
    /// Human-readable outcome detail.
    pub detail: String,
}

/// Observational runtime lifecycle state (`RUNTIME-API-CONTRACT-001` §2).
///
/// This is an **observational contract representation derived from runtime
/// state. It does not authorize transitions. Lifecycle transitions remain
/// owned by the startup/runtime state machine.** The runtime API may read this
/// state; it MUST NOT write it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeLifecycleState {
    /// Node process started; the 6-phase startup protocol is executing.
    Initializing,
    /// All startup checks passed; receipt sealed (transitional, pre-bind).
    StartupComplete,
    /// Node available to consumers: startup succeeded ∧ receipt exists ∧
    /// governance valid ∧ runtime state observable.
    ServableRuntime,
    /// One or more startup checks failed; process exits pre-bind.
    StartupFailed,
}

impl RuntimeLifecycleState {
    /// All lifecycle states, in transition order.
    pub const ALL: [RuntimeLifecycleState; 4] = [
        RuntimeLifecycleState::Initializing,
        RuntimeLifecycleState::StartupComplete,
        RuntimeLifecycleState::ServableRuntime,
        RuntimeLifecycleState::StartupFailed,
    ];

    /// Serialized form (matches `serde(rename_all = "SCREAMING_SNAKE_CASE")`).
    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeLifecycleState::Initializing => "INITIALIZING",
            RuntimeLifecycleState::StartupComplete => "STARTUP_COMPLETE",
            RuntimeLifecycleState::ServableRuntime => "SERVABLE_RUNTIME",
            RuntimeLifecycleState::StartupFailed => "STARTUP_FAILED",
        }
    }

    /// Whether this state is observable through the runtime API.
    pub fn is_servable(self) -> bool {
        matches!(self, RuntimeLifecycleState::ServableRuntime)
    }
}

impl StartupReceipt {
    /// Extract the deterministic equivalence facts.
    pub fn deterministic_facts(&self) -> StartupReceiptFacts {
        StartupReceiptFacts {
            node_id: self.node_id.clone(),
            platform: self.platform.clone(),
            governance_commit: self.governance_commit.clone(),
            startup_phase: self.startup_phase.clone(),
            identity_loaded: self.identity_loaded,
            governance_verified: self.governance_verified,
            capabilities_loaded: self.capabilities_loaded,
            environment_validated: self.environment_validated,
            checks_passed: self.checks_passed,
            checks_failed: self.checks_failed,
            status: self.status.clone(),
        }
    }

    /// Validate against `schemas/startup-receipt.schema.json` (RECEIPT-SCHEMA-001).
    pub fn validate(&self) -> Result<(), String> {
        if !is_receipt_id(&self.receipt_id) {
            return Err(format!(
                "receipt_id '{}' does not match pattern ^[A-Z0-9-]+$",
                self.receipt_id
            ));
        }
        if self.node_id.is_empty() {
            return Err("node_id must not be empty".to_string());
        }
        if !matches!(self.platform.as_str(), "windows" | "linux" | "macos") {
            return Err(format!(
                "platform '{}' is not one of windows|linux|macos",
                self.platform
            ));
        }
        if !is_sha256_hex(&self.governance_commit) {
            return Err(format!(
                "governance_commit '{}' is not a 40-char hex SHA",
                self.governance_commit
            ));
        }
        if !matches!(self.startup_phase.as_str(), "complete" | "failed") {
            return Err(format!(
                "startup_phase '{}' is not one of complete|failed",
                self.startup_phase
            ));
        }
        if !matches!(self.status.as_str(), "GOVERNED_EXECUTION" | "STARTUP_FAILED") {
            return Err(format!(
                "status '{}' is not one of GOVERNED_EXECUTION|STARTUP_FAILED",
                self.status
            ));
        }
        if self.timestamp.is_empty() {
            return Err("timestamp must not be empty".to_string());
        }
        if chrono::DateTime::parse_from_rfc3339(&self.timestamp).is_err() {
            return Err(format!(
                "timestamp '{}' is not a valid RFC 3339 date-time",
                self.timestamp
            ));
        }
        Ok(())
    }

    /// Serialize to canonical JSON.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("serialization failed: {e}"))
    }

    /// Parse from JSON.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("deserialization failed: {e}"))
    }
}

/// Whether a string matches the receipt ID pattern `^[A-Z0-9-]+$`.
pub fn is_receipt_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
}

/// Whether a string is a 40-character lowercase hex SHA.
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_receipt() -> StartupReceipt {
        StartupReceipt {
            receipt_id: "FIXTURE-RECEIPT-001".to_string(),
            node_id: "WINPC-BIG-PICKLE".to_string(),
            platform: "windows".to_string(),
            governance_commit: "6be76216a8048492526c4ca0ae751b6d2d507185".to_string(),
            startup_phase: "complete".to_string(),
            identity_loaded: true,
            governance_verified: true,
            capabilities_loaded: true,
            environment_validated: true,
            checks_passed: 6,
            checks_failed: 0,
            status: "GOVERNED_EXECUTION".to_string(),
            timestamp: "2026-07-25T02:33:33Z".to_string(),
        }
    }

    #[test]
    fn test_receipt_schema_001_valid() {
        assert!(valid_receipt().validate().is_ok());
    }

    #[test]
    fn test_receipt_schema_001_invalid_receipt_id() {
        let mut r = valid_receipt();
        r.receipt_id = "lower-case".to_string();
        assert!(r.validate().is_err());
    }

    #[test]
    fn test_receipt_schema_001_invalid_platform() {
        let mut r = valid_receipt();
        r.platform = "darwin".to_string();
        assert!(r.validate().is_err());
    }

    #[test]
    fn test_receipt_schema_001_invalid_commit() {
        let mut r = valid_receipt();
        r.governance_commit = "short".to_string();
        assert!(r.validate().is_err());
    }

    #[test]
    fn test_receipt_schema_001_invalid_status() {
        let mut r = valid_receipt();
        r.status = "RUNNING".to_string();
        assert!(r.validate().is_err());
    }

    #[test]
    fn test_receipt_schema_001_invalid_timestamp() {
        let mut r = valid_receipt();
        r.timestamp = "not-a-date".to_string();
        assert!(r.validate().is_err());
    }

    #[test]
    fn test_deterministic_facts_extraction() {
        let r = valid_receipt();
        let facts = r.deterministic_facts();
        assert_eq!(facts.node_id, "WINPC-BIG-PICKLE");
        assert_eq!(facts.checks_passed, 6);
        assert_eq!(facts.status, "GOVERNED_EXECUTION");
    }

    #[test]
    fn test_round_trip() {
        let r = valid_receipt();
        let json = r.to_json().unwrap();
        let parsed = StartupReceipt::from_json(&json).unwrap();
        assert_eq!(r, parsed);
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn test_receipt_id_helpers() {
        assert!(is_receipt_id("WINDOWS-STARTUP-20260724-223333"));
        assert!(is_receipt_id("FIXTURE-RECEIPT-001"));
        assert!(!is_receipt_id("invalid_id"));
        assert!(is_sha256_hex("6be76216a8048492526c4ca0ae751b6d2d507185"));
        assert!(!is_sha256_hex("6BE76216a8048492526c4ca0ae751b6d2d50718"));
    }

    #[test]
    fn test_startup_phase_serialization() {
        assert_eq!(
            StartupPhase::ALL.map(|p| p.as_str()).to_vec(),
            vec![
                "identity_loading",
                "governance_verification",
                "capability_loading",
                "environment_validation",
                "receipt_generation",
                "governed_mode"
            ]
        );
        let json = serde_json::to_string(&StartupPhase::GovernanceVerification).unwrap();
        assert_eq!(json, "\"governance_verification\"");
        let back: StartupPhase = serde_json::from_str(&json).unwrap();
        assert_eq!(back, StartupPhase::GovernanceVerification);
    }

    #[test]
    fn test_startup_status_serialization() {
        assert_eq!(
            StartupStatus::GovernedExecution.as_str(),
            "GOVERNED_EXECUTION"
        );
        assert_eq!(StartupStatus::StartupFailed.as_str(), "STARTUP_FAILED");
        let json = serde_json::to_string(&StartupStatus::GovernedExecution).unwrap();
        assert_eq!(json, "\"GOVERNED_EXECUTION\"");
        let back: StartupStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, StartupStatus::GovernedExecution);
    }

    #[test]
    fn test_startup_check_round_trip() {
        let check = StartupCheck {
            phase: StartupPhase::IdentityLoading,
            passed: true,
            detail: "node identity loaded".to_string(),
        };
        let json = serde_json::to_string(&check).unwrap();
        let back: StartupCheck = serde_json::from_str(&json).unwrap();
        assert_eq!(check, back);
    }

    #[test]
    fn test_runtime_lifecycle_state_serde_matches_as_str() {
        // Observational contract representation (RUNTIME-API-CONTRACT-001 §2.4):
        // wire values are SCREAMING_SNAKE_CASE and must match as_str().
        for state in RuntimeLifecycleState::ALL {
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, format!("\"{}\"", state.as_str()));
            let back: RuntimeLifecycleState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }
    }

    #[test]
    fn test_runtime_lifecycle_state_servable_semantics() {
        assert!(RuntimeLifecycleState::ServableRuntime.is_servable());
        for state in [
            RuntimeLifecycleState::Initializing,
            RuntimeLifecycleState::StartupComplete,
            RuntimeLifecycleState::StartupFailed,
        ] {
            assert!(!state.is_servable(), "{} must not be servable", state.as_str());
        }
    }
}
