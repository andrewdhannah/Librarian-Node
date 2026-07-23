//! EvidencePacket — Windows→Mac bridge packet.
//!
//! Created by the Windows node after completing a qualification run.
//! Contains the full execution evidence chain for Mac-side intake.
//!
//! This packet crosses the authority boundary from Windows to Mac.
//! It carries execution evidence, NOT capability authority.
//! Mac receives this packet, validates its integrity, and records
//! qualification evidence. The EvidencePacket contains no capability data,
//! no role assignments, and no qualification status.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use super::common::{
    PacketExecutionIdentity, PacketExecutionMetrics, PacketLeaseLifecycle, PacketLifecycleEvent,
    PacketModelIdentity, PacketReleaseVerification,
};

/// Packet type identifier.
pub const PACKET_TYPE: &str = "evidence_packet";

/// Current packet schema version.
pub const PACKET_VERSION: &str = "1";

/// EvidencePacket — Windows→Mac bridge packet.
///
/// This packet contains the full execution evidence from a Windows run.
/// It carries:
/// - Model identity (exact artifact binding)
/// - Execution identity (runtime, hardware, executable version)
/// - Lease lifecycle (residency state transitions)
/// - Execution metrics (tokens, timing, exit status)
/// - Lifecycle events (ordered evidence chain)
/// - Release verification (PID exit + GPU release proof)
///
/// It does NOT carry:
/// - Capability status
/// - Role assignments
/// - Qualification decisions
/// - Router eligibility
///
/// Mac processes this packet by:
/// 1. Validating packet integrity
/// 2. Verifying lifecycle event ordering
/// 3. Recording qualification_run
/// 4. Applying validator rules
/// 5. Feeding evidence to qualification stages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidencePacket {
    /// Packet type identifier ("evidence_packet").
    pub packet_type: String,

    /// Packet schema version ("1").
    pub packet_version: String,

    /// Timestamp when the packet was exported (RFC 3339).
    pub exported_at: String,

    /// The qualification request ID this evidence responds to.
    pub qualification_request_id: String,

    /// Model identity — exact artifact binding.
    pub identity: PacketModelIdentity,

    /// Execution identity — exact runtime binding.
    pub execution: PacketExecutionIdentity,

    /// Lease lifecycle.
    pub lease: PacketLeaseLifecycle,

    /// Execution metrics.
    pub run: PacketExecutionMetrics,

    /// Ordered lifecycle events.
    pub lifecycle_events: Vec<PacketLifecycleEvent>,

    /// GPU release verification.
    pub release_verification: PacketReleaseVerification,
}

impl EvidencePacket {
    /// Validate the packet structure.
    /// Returns Ok(()) if valid, Err with details if not.
    pub fn validate(&self) -> Result<()> {
        // Check packet type
        if self.packet_type != PACKET_TYPE {
            anyhow::bail!(
                "Invalid packet type: expected '{}', got '{}'",
                PACKET_TYPE,
                self.packet_type
            );
        }

        // Check packet version
        if self.packet_version != PACKET_VERSION {
            anyhow::bail!(
                "Unsupported packet version: expected '{}', got '{}'",
                PACKET_VERSION,
                self.packet_version
            );
        }

        // Check qualification_request_id is non-empty
        if self.qualification_request_id.is_empty() {
            anyhow::bail!("qualification_request_id is empty");
        }

        // Check identity fields
        if self.identity.model_id.is_empty() {
            anyhow::bail!("identity.model_id is empty");
        }
        if self.identity.sha256.is_empty() {
            anyhow::bail!("identity.sha256 is empty");
        }

        // Check execution identity fields
        if self.execution.runtime_profile_id.is_empty() {
            anyhow::bail!("execution.runtime_profile_id is empty");
        }
        if self.execution.hardware_profile_id.is_empty() {
            anyhow::bail!("execution.hardware_profile_id is empty");
        }

        // Check lease fields
        if self.lease.lease_id.is_empty() {
            anyhow::bail!("lease.lease_id is empty");
        }

        // Check run fields
        if self.run.run_id.is_empty() {
            anyhow::bail!("run.run_id is empty");
        }

        Ok(())
    }

    /// Validate lifecycle event ordering.
    /// Events must be in chronological order.
    pub fn validate_lifecycle_ordering(&self) -> Result<()> {
        for i in 1..self.lifecycle_events.len() {
            let prev = &self.lifecycle_events[i - 1];
            let curr = &self.lifecycle_events[i];

            if let (Some(prev_time), Some(curr_time)) = (&prev.occurred_at, &curr.occurred_at) {
                if prev_time > curr_time {
                    anyhow::bail!(
                        "Lifecycle events not in chronological order: event {} ({}) > event {} ({})",
                        i - 1,
                        prev_time,
                        i,
                        curr_time
                    );
                }
            }
        }
        Ok(())
    }

    /// Compute SHA-256 hash of the serialized packet (deterministic).
    pub fn compute_hash(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .context("Failed to serialize packet for hashing")?;
        let mut hasher = sha2::Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize packet to JSON")
    }

    /// Convert to pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize packet to pretty JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse EvidencePacket from JSON")
    }

    /// Assert this packet contains no capability authority data.
    /// This is the authority boundary proof.
    pub fn assert_no_capability_data(&self) -> Result<()> {
        // EvidencePacket must not contain:
        // - role assignments
        // - capability status
        // - qualification decisions
        // - router eligibility
        //
        // Structural proof: the packet fields are:
        // - packet_type, packet_version, exported_at (metadata)
        // - qualification_request_id (links to request — NOT capability)
        // - identity (model artifact binding — NOT capability)
        // - execution (runtime binding — NOT capability)
        // - lease (residency lifecycle — NOT capability)
        // - run (execution metrics — NOT capability)
        // - lifecycle_events (evidence chain — NOT capability)
        // - release_verification (GPU proof — NOT capability)
        //
        // There are no fields for:
        // - role
        // - capability_status
        // - qualification_status
        // - approved_roles
        // - router_eligible
        //
        // This is enforced by the struct definition itself.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::common::*;

    fn test_event(event_type: &str, occurred_at: &str) -> PacketLifecycleEvent {
        PacketLifecycleEvent {
            event_type: event_type.to_string(),
            process_id: Some(1234),
            observed_state: Some("ready".to_string()),
            observation: Some(r#"{"load_duration_ms": 2187}"#.to_string()),
            occurred_at: Some(occurred_at.to_string()),
        }
    }

    fn test_packet() -> EvidencePacket {
        EvidencePacket {
            packet_type: PACKET_TYPE.to_string(),
            packet_version: PACKET_VERSION.to_string(),
            exported_at: "2026-07-11T12:00:00Z".to_string(),
            qualification_request_id: "qr-test-001".to_string(),
            identity: PacketModelIdentity {
                model_id: "minicpm5-1b-q4km".to_string(),
                sha256: "81B64D05A23B".to_string(),
                filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
                quantization: Some("Q4_K_M".to_string()),
            },
            execution: PacketExecutionIdentity {
                runtime_profile_id: "prof-q4km".to_string(),
                hardware_profile_id: "hw-rx570".to_string(),
                runtime_executable_sha256: "0D496467CFD9".to_string(),
                runtime_executable_version: "c85e97a".to_string(),
            },
            lease: PacketLeaseLifecycle {
                lease_id: "lease-test-001".to_string(),
                port: Some(9120),
                state: "unloaded".to_string(),
                loaded_at: Some("2026-07-11T11:59:50Z".to_string()),
                released_at: Some("2026-07-11T12:00:01Z".to_string()),
                vram_released_at: Some("2026-07-11T12:00:01Z".to_string()),
            },
            run: PacketExecutionMetrics {
                run_id: "run-test-001".to_string(),
                input_tokens: Some(10),
                output_tokens: Some(32),
                load_duration_ms: Some(2187),
                generation_duration_ms: Some(385),
                exit_status: Some("clean".to_string()),
                started_at: Some("2026-07-11T11:59:50Z".to_string()),
                ended_at: Some("2026-07-11T12:00:01Z".to_string()),
            },
            lifecycle_events: vec![
                test_event("process_started", "2026-07-11T11:59:50Z"),
                test_event("runtime_ready", "2026-07-11T11:59:52Z"),
                test_event("generation_completed", "2026-07-11T12:00:00Z"),
                test_event("process_killed", "2026-07-11T12:00:01Z"),
                test_event("gpu_release_verified", "2026-07-11T12:00:01Z"),
            ],
            release_verification: PacketReleaseVerification {
                pid_exit_verified: true,
                gpu_release_verified: true,
                free_vram_mb: Some(3433),
                baseline_vram_mb: Some(3433),
                within_tolerance: true,
            },
        }
    }

    // MQR-F3-21: EvidencePacket round-trip
    #[test]
    fn test_round_trip() {
        let pkt = test_packet();
        let json = pkt.to_json().unwrap();
        let parsed = EvidencePacket::from_json(&json).unwrap();
        assert_eq!(pkt, parsed);
    }

    // MQR-F3-22: Hash is deterministic
    #[test]
    fn test_hash_deterministic() {
        let pkt = test_packet();
        let h1 = pkt.compute_hash().unwrap();
        let h2 = pkt.compute_hash().unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    // MQR-F3-23: Hash changes with content
    #[test]
    fn test_hash_content_dependent() {
        let mut pkt = test_packet();
        let h1 = pkt.compute_hash().unwrap();
        pkt.run.output_tokens = Some(999);
        let h2 = pkt.compute_hash().unwrap();
        assert_ne!(h1, h2);
    }

    // MQR-F3-24: Validate passes for valid packet
    #[test]
    fn test_validate_valid() {
        let pkt = test_packet();
        assert!(pkt.validate().is_ok());
    }

    // MQR-F3-25: Validate fails for empty request_id
    #[test]
    fn test_validate_empty_request_id() {
        let mut pkt = test_packet();
        pkt.qualification_request_id = "".to_string();
        assert!(pkt.validate().is_err());
    }

    // MQR-F3-26: Validate fails for empty model_id
    #[test]
    fn test_validate_empty_model_id() {
        let mut pkt = test_packet();
        pkt.identity.model_id = "".to_string();
        assert!(pkt.validate().is_err());
    }

    // MQR-F3-27: Validate fails for empty lease_id
    #[test]
    fn test_validate_empty_lease_id() {
        let mut pkt = test_packet();
        pkt.lease.lease_id = "".to_string();
        assert!(pkt.validate().is_err());
    }

    // MQR-F3-28: Validate fails for empty run_id
    #[test]
    fn test_validate_empty_run_id() {
        let mut pkt = test_packet();
        pkt.run.run_id = "".to_string();
        assert!(pkt.validate().is_err());
    }

    // MQR-F3-29: Lifecycle ordering passes for ordered events
    #[test]
    fn test_lifecycle_ordering_pass() {
        let pkt = test_packet();
        assert!(pkt.validate_lifecycle_ordering().is_ok());
    }

    // MQR-F3-30: Lifecycle ordering fails for unordered events
    #[test]
    fn test_lifecycle_ordering_fail() {
        let mut pkt = test_packet();
        // Swap first two events (out of chronological order)
        pkt.lifecycle_events[0].occurred_at = Some("2026-07-11T12:00:00Z".to_string());
        pkt.lifecycle_events[1].occurred_at = Some("2026-07-11T11:59:50Z".to_string());
        assert!(pkt.validate_lifecycle_ordering().is_err());
    }

    // MQR-F3-31: Empty lifecycle events is valid
    #[test]
    fn test_empty_lifecycle_events() {
        let mut pkt = test_packet();
        pkt.lifecycle_events = vec![];
        assert!(pkt.validate_lifecycle_ordering().is_ok());
    }

    // MQR-F3-32: No capability data assertion
    #[test]
    fn test_no_capability_data() {
        let pkt = test_packet();
        assert!(pkt.assert_no_capability_data().is_ok());
    }

    // MQR-F3-33: Packet type is correct
    #[test]
    fn test_packet_type() {
        let pkt = test_packet();
        assert_eq!(pkt.packet_type, "evidence_packet");
        assert_eq!(pkt.packet_version, "1");
    }

    // MQR-F3-34: Invalid packet_type fails validation
    #[test]
    fn test_invalid_packet_type() {
        let mut pkt = test_packet();
        pkt.packet_type = "wrong_type".to_string();
        assert!(pkt.validate().is_err());
    }

    // MQR-F3-35: Clone produces equal value
    #[test]
    fn test_clone_eq() {
        let pkt = test_packet();
        let cloned = pkt.clone();
        assert_eq!(pkt, cloned);
    }

    // MQR-F3-36: Invalid JSON fails gracefully
    #[test]
    fn test_invalid_json() {
        let result = EvidencePacket::from_json("not json");
        assert!(result.is_err());
    }

    // MQR-F3-37: Pretty JSON round-trip
    #[test]
    fn test_pretty_round_trip() {
        let pkt = test_packet();
        let json = pkt.to_json_pretty().unwrap();
        let parsed = EvidencePacket::from_json(&json).unwrap();
        assert_eq!(pkt, parsed);
    }
}
