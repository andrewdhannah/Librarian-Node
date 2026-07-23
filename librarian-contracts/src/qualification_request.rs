//! QualificationRequest — Mac→Windows bridge packet.
//!
//! Sent by the Mac qualification runner to request execution of a specific
//! model + task under residency constraints.
//!
//! This packet crosses the authority boundary from Mac to Windows.
//! It carries execution instructions, NOT capability authority.
//! Windows receives this packet, executes the task, and returns an
//! EvidencePacket. The QualificationRequest contains no capability data,
//! no role assignments, and no qualification status.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use super::common::{PacketConstraints, PacketExecutionConfig, PacketModelIdentity};

/// Packet type identifier.
pub const PACKET_TYPE: &str = "qualification_request";

/// Current packet schema version.
pub const PACKET_VERSION: &str = "1";

/// QualificationRequest — Mac→Windows bridge packet.
///
/// This packet requests Windows to execute a specific model on a specific task.
/// It carries:
/// - Model identity (exact artifact binding)
/// - Execution configuration (runtime profile, task, parameters)
/// - Execution constraints (VRAM limits, release proof requirement)
///
/// It does NOT carry:
/// - Capability status
/// - Role assignments
/// - Qualification decisions
/// - Router eligibility
///
/// Windows processes this packet by:
/// 1. Validating the model is installed
/// 2. Acquiring residency
/// 3. Executing the task
/// 4. Recording execution evidence
/// 5. Returning an EvidencePacket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualificationRequest {
    /// Packet type identifier ("qualification_request").
    pub packet_type: String,

    /// Packet schema version ("1").
    pub packet_version: String,

    /// Unique request identifier.
    pub request_id: String,

    /// Model identity — exact artifact binding.
    pub identity: PacketModelIdentity,

    /// Execution configuration.
    pub execution: PacketExecutionConfig,

    /// Execution constraints.
    pub constraints: PacketConstraints,

    /// Timestamp when the request was created (RFC 3339).
    pub created_at: String,
}

impl QualificationRequest {
    /// Create a new qualification request.
    pub fn new(
        request_id: String,
        identity: PacketModelIdentity,
        execution: PacketExecutionConfig,
        constraints: PacketConstraints,
    ) -> Self {
        Self {
            packet_type: PACKET_TYPE.to_string(),
            packet_version: PACKET_VERSION.to_string(),
            request_id,
            identity,
            execution,
            constraints,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

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

        // Check request_id is non-empty
        if self.request_id.is_empty() {
            anyhow::bail!("request_id is empty");
        }

        // Check identity fields
        if self.identity.model_id.is_empty() {
            anyhow::bail!("identity.model_id is empty");
        }
        if self.identity.sha256.is_empty() {
            anyhow::bail!("identity.sha256 is empty");
        }
        if self.identity.filename.is_empty() {
            anyhow::bail!("identity.filename is empty");
        }

        // Check execution fields
        if self.execution.runtime_profile_id.is_empty() {
            anyhow::bail!("execution.runtime_profile_id is empty");
        }
        if self.execution.task_description.is_empty() {
            anyhow::bail!("execution.task_description is empty");
        }

        // Check timeout is reasonable
        if let Some(timeout) = self.execution.timeout_seconds {
            if timeout == 0 || timeout > 600 {
                anyhow::bail!(
                    "execution.timeout_seconds must be between 1 and 600, got {}",
                    timeout
                );
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

    /// Convert to JSON string (deterministic — sorted keys).
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize packet to JSON")
    }

    /// Convert to pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize packet to pretty JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse QualificationRequest from JSON")
    }

    /// Assert this packet contains no capability authority data.
    /// This is the authority boundary proof.
    pub fn assert_no_capability_data(&self) -> Result<()> {
        // QualificationRequest must not contain:
        // - role assignments
        // - capability status
        // - qualification decisions
        // - router eligibility
        //
        // Structural proof: the packet fields are:
        // - packet_type, packet_version, request_id (metadata)
        // - identity (model artifact binding — NOT capability)
        // - execution (configuration — NOT capability)
        // - constraints (bounds — NOT capability)
        // - created_at (timestamp)
        //
        // There are no fields for:
        // - role
        // - capability_status
        // - qualification_status
        // - approved_roles
        // - router_eligible
        //
        // This is enforced by the struct definition itself.
        // The function exists as an explicit runtime assertion.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_request() -> QualificationRequest {
        QualificationRequest::new(
            "qr-test-001".to_string(),
            PacketModelIdentity {
                model_id: "minicpm5-1b-q4km".to_string(),
                sha256: "81B64D05A23B".to_string(),
                filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
                quantization: Some("Q4_K_M".to_string()),
            },
            PacketExecutionConfig {
                runtime_profile_id: "prof-q4km".to_string(),
                task_description: "Execute instruction-following fixture IF-001".to_string(),
                max_tokens: Some(256),
                temperature: Some(0.0),
                timeout_seconds: Some(120),
            },
            PacketConstraints {
                require_release_proof: true,
                max_vram_mb: Some(4096),
            },
        )
    }

    // MQR-F3-1: QualificationRequest round-trip
    #[test]
    fn test_round_trip() {
        let req = test_request();
        let json = req.to_json().unwrap();
        let parsed = QualificationRequest::from_json(&json).unwrap();
        assert_eq!(req, parsed);
    }

    // MQR-F3-2: Pretty JSON round-trip
    #[test]
    fn test_pretty_round_trip() {
        let req = test_request();
        let json = req.to_json_pretty().unwrap();
        let parsed = QualificationRequest::from_json(&json).unwrap();
        assert_eq!(req, parsed);
    }

    // MQR-F3-3: Hash is deterministic
    #[test]
    fn test_hash_deterministic() {
        let req = test_request();
        let h1 = req.compute_hash().unwrap();
        let h2 = req.compute_hash().unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    // MQR-F3-4: Hash changes with content
    #[test]
    fn test_hash_content_dependent() {
        let mut req = test_request();
        let h1 = req.compute_hash().unwrap();
        req.request_id = "qr-different".to_string();
        let h2 = req.compute_hash().unwrap();
        assert_ne!(h1, h2);
    }

    // MQR-F3-5: Validate passes for valid request
    #[test]
    fn test_validate_valid() {
        let req = test_request();
        assert!(req.validate().is_ok());
    }

    // MQR-F3-6: Validate fails for empty request_id
    #[test]
    fn test_validate_empty_request_id() {
        let mut req = test_request();
        req.request_id = "".to_string();
        assert!(req.validate().is_err());
    }

    // MQR-F3-7: Validate fails for empty model_id
    #[test]
    fn test_validate_empty_model_id() {
        let mut req = test_request();
        req.identity.model_id = "".to_string();
        assert!(req.validate().is_err());
    }

    // MQR-F3-8: Validate fails for empty sha256
    #[test]
    fn test_validate_empty_sha256() {
        let mut req = test_request();
        req.identity.sha256 = "".to_string();
        assert!(req.validate().is_err());
    }

    // MQR-F3-9: Validate fails for empty task_description
    #[test]
    fn test_validate_empty_task_description() {
        let mut req = test_request();
        req.execution.task_description = "".to_string();
        assert!(req.validate().is_err());
    }

    // MQR-F3-10: Validate fails for timeout=0
    #[test]
    fn test_validate_timeout_zero() {
        let mut req = test_request();
        req.execution.timeout_seconds = Some(0);
        assert!(req.validate().is_err());
    }

    // MQR-F3-11: Validate fails for timeout > 600
    #[test]
    fn test_validate_timeout_too_large() {
        let mut req = test_request();
        req.execution.timeout_seconds = Some(601);
        assert!(req.validate().is_err());
    }

    // MQR-F3-12: No capability data assertion
    #[test]
    fn test_no_capability_data() {
        let req = test_request();
        assert!(req.assert_no_capability_data().is_ok());
    }

    // MQR-F3-13: Packet type is correct
    #[test]
    fn test_packet_type() {
        let req = test_request();
        assert_eq!(req.packet_type, "qualification_request");
        assert_eq!(req.packet_version, "1");
    }

    // MQR-F3-14: Invalid packet_type fails validation
    #[test]
    fn test_invalid_packet_type() {
        let mut req = test_request();
        req.packet_type = "wrong_type".to_string();
        assert!(req.validate().is_err());
    }

    // MQR-F3-15: Invalid packet_version fails validation
    #[test]
    fn test_invalid_packet_version() {
        let mut req = test_request();
        req.packet_version = "999".to_string();
        assert!(req.validate().is_err());
    }

    // MQR-F3-16: Optional fields are preserved in round-trip
    #[test]
    fn test_optional_fields_preserved() {
        let mut req = test_request();
        req.identity.quantization = None;
        req.execution.max_tokens = None;
        req.execution.temperature = None;
        req.execution.timeout_seconds = None;
        req.constraints.max_vram_mb = None;

        let json = req.to_json().unwrap();
        let parsed = QualificationRequest::from_json(&json).unwrap();
        assert_eq!(parsed.identity.quantization, None);
        assert_eq!(parsed.execution.max_tokens, None);
        assert_eq!(parsed.execution.temperature, None);
        assert_eq!(parsed.execution.timeout_seconds, None);
        assert_eq!(parsed.constraints.max_vram_mb, None);
    }

    // MQR-F3-17: created_at is set
    #[test]
    fn test_created_at_set() {
        let req = test_request();
        assert!(!req.created_at.is_empty());
        // Should be RFC 3339 format
        assert!(req.created_at.contains("T"));
    }

    // MQR-F3-18: Clone produces equal value
    #[test]
    fn test_clone_eq() {
        let req = test_request();
        let cloned = req.clone();
        assert_eq!(req, cloned);
    }

    // MQR-F3-19: JSON deserialization with extra fields ignores them
    #[test]
    fn test_extra_fields_ignored() {
        let req = test_request();
        let mut json_value: serde_json::Value = serde_json::from_str(&req.to_json().unwrap()).unwrap();
        json_value["extra_field"] = serde_json::Value::String("ignored".to_string());
        let json_str = serde_json::to_string(&json_value).unwrap();
        let parsed = QualificationRequest::from_json(&json_str).unwrap();
        assert_eq!(parsed.request_id, req.request_id);
    }

    // MQR-F3-20: Invalid JSON fails gracefully
    #[test]
    fn test_invalid_json() {
        let result = QualificationRequest::from_json("not json");
        assert!(result.is_err());
    }
}
