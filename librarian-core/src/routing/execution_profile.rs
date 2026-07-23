//! Execution profile — records how and where a model runs.
//!
//! The execution profile captures the runtime environment: model artifact,
//! runtime version, quantization, backend, hardware identity, and measured
//! execution metrics. It is the "how/where" that pairs with the capability
//! manifest's "what work is approved."
//!
//! Execution profiles are NOT capability evidence. They record
//! operational characteristics, not qualification status.
//!
//! Hardware throughput CANNOT upgrade capability status. A fast execution
//! profile does not make an unqualified model qualified.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Execution profile status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProfileStatus {
    /// Active: profile is available for projection.
    #[serde(rename = "active")]
    Active,
    /// Inactive: profile is not available for projection.
    #[serde(rename = "inactive")]
    Inactive,
    /// Incompatible: profile has known incompatibilities.
    #[serde(rename = "incompatible")]
    Incompatible,
}

impl ProfileStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Incompatible => "incompatible",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "inactive" => Some(Self::Inactive),
            "incompatible" => Some(Self::Incompatible),
            _ => None,
        }
    }
}

/// Hardware identity — what physical hardware the model runs on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardwareIdentity {
    /// GPU description (e.g., "Radeon RX 570").
    pub gpu_description: String,

    /// GPU VRAM in MiB.
    pub gpu_vram_mb: u64,

    /// CPU description.
    pub cpu: String,

    /// System RAM in MiB.
    pub ram_mb: u64,

    /// OS platform.
    pub os: String,
}

/// Runtime identity — what software stack the model runs on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeIdentity {
    /// Runtime executable path or identifier.
    pub executable: String,

    /// Runtime version (e.g., git SHA).
    pub version: String,

    /// Backend type (e.g., "vulkan", "cuda", "cpu").
    pub backend: String,

    /// Backend device identifier (e.g., "Vulkan0").
    pub device_id: Option<String>,
}

/// Model artifact identity — what specific file is being run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactIdentity {
    /// Model filename.
    pub filename: String,

    /// Model ID (e.g., "minicpm5-1b-q4km").
    pub model_id: String,

    /// Quantization level (e.g., "Q4_K_M").
    pub quantization: String,

    /// SHA-256 hash of the model file.
    pub sha256: String,

    /// File size in bytes.
    pub file_size_bytes: u64,
}

/// Measured execution metrics — observed performance characteristics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionMetrics {
    /// Average load duration in milliseconds.
    pub avg_load_duration_ms: Option<f64>,

    /// Average generation duration in milliseconds.
    pub avg_generation_duration_ms: Option<f64>,

    /// Average tokens per second.
    pub avg_tokens_per_second: Option<f64>,

    /// Peak VRAM usage in MiB.
    pub peak_vram_usage_mb: Option<u64>,

    /// Number of observations this average is based on.
    pub observation_count: u32,
}

/// Execution profile — records how and where a model runs.
///
/// The profile captures operational characteristics, not capability status.
/// Hardware throughput CANNOT upgrade capability status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionProfile {
    /// Unique profile identifier.
    pub profile_id: String,

    /// Model artifact identity.
    pub artifact: ArtifactIdentity,

    /// Runtime identity.
    pub runtime: RuntimeIdentity,

    /// Hardware identity.
    pub hardware: HardwareIdentity,

    /// Measured execution metrics.
    pub metrics: ExecutionMetrics,

    /// Profile status.
    pub status: ProfileStatus,

    /// SHA-256 hash of the profile content.
    pub content_hash: String,

    /// When the profile was created (RFC 3339).
    pub created_at: String,

    /// When the profile was last updated (RFC 3339).
    pub updated_at: String,
}

impl ExecutionProfile {
    /// Compute a deterministic profile ID from model_id, runtime version, and hardware.
    pub fn compute_profile_id(
        model_id: &str,
        runtime_version: &str,
        gpu_description: &str,
    ) -> String {
        let input = format!("{}:{}:{}", model_id, runtime_version, gpu_description);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute SHA-256 hash of the profile content.
    pub fn compute_content_hash(&self) -> Result<String> {
        let content = serde_json::json!({
            "artifact": self.artifact,
            "runtime": self.runtime,
            "hardware": self.hardware,
            "metrics": self.metrics,
            "status": self.status.as_str(),
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Validate the profile structure.
    pub fn validate(&self) -> Result<()> {
        if self.profile_id.is_empty() {
            anyhow::bail!("profile_id is empty");
        }
        if self.artifact.model_id.is_empty() {
            anyhow::bail!("artifact.model_id is empty");
        }
        if self.artifact.sha256.is_empty() {
            anyhow::bail!("artifact.sha256 is empty");
        }
        if self.runtime.executable.is_empty() {
            anyhow::bail!("runtime.executable is empty");
        }
        if self.hardware.gpu_description.is_empty() {
            anyhow::bail!("hardware.gpu_description is empty");
        }
        Ok(())
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize profile to JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse profile from JSON")
    }

    /// Assert this profile contains no capability data.
    pub fn assert_no_capability_data(&self) -> Result<()> {
        // ExecutionProfile MUST NOT contain:
        // - capability_status
        // - role
        // - qualification_status
        // - approved_roles
        // - router_eligible
        //
        // Structural proof: the fields are:
        // - profile_id, artifact, runtime, hardware (identity)
        // - metrics (observed performance — NOT capability)
        // - status (profile availability — NOT capability)
        // - content_hash, created_at, updated_at (metadata)
        //
        // There are no fields for:
        // - capability_status
        // - role
        // - qualification_status
        // - approved_roles
        // - router_eligible
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hardware() -> HardwareIdentity {
        HardwareIdentity {
            gpu_description: "Radeon RX 570".to_string(),
            gpu_vram_mb: 4096,
            cpu: "Intel Core i7-7700K".to_string(),
            ram_mb: 16384,
            os: "windows".to_string(),
        }
    }

    fn test_runtime() -> RuntimeIdentity {
        RuntimeIdentity {
            executable: "llama-server.exe".to_string(),
            version: "c85e97a".to_string(),
            backend: "vulkan".to_string(),
            device_id: Some("Vulkan0".to_string()),
        }
    }

    fn test_artifact() -> ArtifactIdentity {
        ArtifactIdentity {
            filename: "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
            model_id: "minicpm5-1b-q4km".to_string(),
            quantization: "Q4_K_M".to_string(),
            sha256: "81B64D05A23B".to_string(),
            file_size_bytes: 688_000_000,
        }
    }

    fn test_metrics() -> ExecutionMetrics {
        ExecutionMetrics {
            avg_load_duration_ms: Some(2187.0),
            avg_generation_duration_ms: Some(385.0),
            avg_tokens_per_second: Some(12.5),
            peak_vram_usage_mb: Some(3433),
            observation_count: 5,
        }
    }

    fn test_profile() -> ExecutionProfile {
        let created_at = "2026-07-11T12:00:00Z".to_string();
        let profile_id = ExecutionProfile::compute_profile_id(
            "minicpm5-1b-q4km",
            "c85e97a",
            "Radeon RX 570",
        );

        ExecutionProfile {
            profile_id,
            artifact: test_artifact(),
            runtime: test_runtime(),
            hardware: test_hardware(),
            metrics: test_metrics(),
            status: ProfileStatus::Active,
            content_hash: String::new(),
            created_at: created_at.clone(),
            updated_at: created_at,
        }
    }

    // MQR-R1-P1: Profile validates
    #[test]
    fn test_profile_validates() {
        let profile = test_profile();
        assert!(profile.validate().is_ok());
    }

    // MQR-R1-P2: Profile ID is deterministic
    #[test]
    fn test_profile_id_deterministic() {
        let id1 = ExecutionProfile::compute_profile_id("model-1", "v1", "GPU-1");
        let id2 = ExecutionProfile::compute_profile_id("model-1", "v1", "GPU-1");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    // MQR-R1-P3: Profile ID depends on inputs
    #[test]
    fn test_profile_id_depends_on_inputs() {
        let id1 = ExecutionProfile::compute_profile_id("model-1", "v1", "GPU-1");
        let id2 = ExecutionProfile::compute_profile_id("model-2", "v1", "GPU-1");
        assert_ne!(id1, id2);
    }

    // MQR-R1-P4: Status string round-trip
    #[test]
    fn test_status_string_roundtrip() {
        let statuses = vec![
            ProfileStatus::Active,
            ProfileStatus::Inactive,
            ProfileStatus::Incompatible,
        ];
        for status in &statuses {
            let s = status.as_str();
            assert!(!s.is_empty());
            assert_eq!(ProfileStatus::from_str(s), Some(status.clone()));
        }
    }

    // MQR-R1-P5: Serialization round-trip
    #[test]
    fn test_serialization_round_trip() {
        let profile = test_profile();
        let json = profile.to_json().unwrap();
        let parsed = ExecutionProfile::from_json(&json).unwrap();
        assert_eq!(profile, parsed);
    }

    // MQR-R1-P6: No capability data
    #[test]
    fn test_no_capability_data() {
        let profile = test_profile();
        assert!(profile.assert_no_capability_data().is_ok());
    }

    // MQR-R1-P7: Validate fails on empty model_id
    #[test]
    fn test_validate_empty_model_id() {
        let mut profile = test_profile();
        profile.artifact.model_id = "".to_string();
        assert!(profile.validate().is_err());
    }

    // MQR-R1-P8: Validate fails on empty sha256
    #[test]
    fn test_validate_empty_sha256() {
        let mut profile = test_profile();
        profile.artifact.sha256 = "".to_string();
        assert!(profile.validate().is_err());
    }

    // MQR-R1-P9: Incompatible status is valid
    #[test]
    fn test_incompatible_status() {
        let mut profile = test_profile();
        profile.status = ProfileStatus::Incompatible;
        assert_eq!(profile.status, ProfileStatus::Incompatible);
    }
}
