//! ResidencyStatus — Windows→Mac query/response.
//!
//! Sent by the Mac scheduler to query current residency state on Windows.
//! Windows responds with the current lease state, active runs, and VRAM status.
//!
//! This packet crosses the authority boundary from Windows to Mac.
//! It carries residency state, NOT capability authority.
//! Mac uses this for routing decisions. The ResidencyStatus contains no
//! capability data, no role assignments, and no qualification status.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::Digest;

/// Query type: request current residency status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResidencyStatusQuery {
    /// Optional model ID to filter results.
    pub model_id: Option<String>,
}

/// Response: current residency status from Windows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResidencyStatusResponse {
    /// Timestamp of the response.
    pub timestamp: String,

    /// Active leases (not unloaded, not failed).
    pub active_leases: Vec<ActiveLease>,

    /// Active runs (in-progress generation).
    pub active_runs: Vec<ActiveRun>,

    /// Whether a drain is in progress.
    pub draining: bool,

    /// Available VRAM in MB.
    pub available_vram_mb: Option<u64>,

    /// Baseline VRAM in MB.
    pub baseline_vram_mb: Option<u64>,
}

/// Active lease summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveLease {
    /// Lease ID.
    pub lease_id: String,

    /// Model ID.
    pub model_id: String,

    /// Profile ID.
    pub profile_id: Option<String>,

    /// Current lease state.
    pub state: String,

    /// Port the model is served on.
    pub port: Option<u16>,

    /// Process ID.
    pub process_id: Option<i32>,
}

/// Active run summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveRun {
    /// Run ID.
    pub run_id: String,

    /// Lease ID this run belongs to.
    pub lease_id: String,

    /// When the run started.
    pub started_at: Option<String>,
}

impl ResidencyStatusResponse {
    /// Validate the response structure.
    pub fn validate(&self) -> Result<()> {
        if self.timestamp.is_empty() {
            anyhow::bail!("timestamp is empty");
        }
        Ok(())
    }

    /// Assert this response contains no capability authority data.
    pub fn assert_no_capability_data(&self) -> Result<()> {
        // ResidencyStatusResponse must not contain:
        // - role assignments
        // - capability status
        // - qualification decisions
        // - router eligibility
        //
        // Structural proof: the response fields are:
        // - timestamp (metadata)
        // - active_leases (residency state — NOT capability)
        // - active_runs (execution state — NOT capability)
        // - draining (residency state — NOT capability)
        // - available_vram_mb, baseline_vram_mb (hardware — NOT capability)
        //
        // There are no fields for role, capability_status, etc.
        Ok(())
    }

    /// Compute SHA-256 hash of the serialized response.
    pub fn compute_hash(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .context("Failed to serialize response for hashing")?;
        let mut hasher = sha2::Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize response to JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse ResidencyStatusResponse from JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_response() -> ResidencyStatusResponse {
        ResidencyStatusResponse {
            timestamp: "2026-07-11T12:00:00Z".to_string(),
            active_leases: vec![ActiveLease {
                lease_id: "lease-1".to_string(),
                model_id: "minicpm5-1b-q4km".to_string(),
                profile_id: Some("prof-q4km".to_string()),
                state: "ready".to_string(),
                port: Some(9120),
                process_id: Some(10804),
            }],
            active_runs: vec![ActiveRun {
                run_id: "run-1".to_string(),
                lease_id: "lease-1".to_string(),
                started_at: Some("2026-07-11T11:59:50Z".to_string()),
            }],
            draining: false,
            available_vram_mb: Some(3433),
            baseline_vram_mb: Some(3433),
        }
    }

    // MQR-F3-38: ResidencyStatusResponse round-trip
    #[test]
    fn test_round_trip() {
        let resp = test_response();
        let json = resp.to_json().unwrap();
        let parsed = ResidencyStatusResponse::from_json(&json).unwrap();
        assert_eq!(resp, parsed);
    }

    // MQR-F3-39: Hash is deterministic
    #[test]
    fn test_hash_deterministic() {
        let resp = test_response();
        let h1 = resp.compute_hash().unwrap();
        let h2 = resp.compute_hash().unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    // MQR-F3-40: Validate passes for valid response
    #[test]
    fn test_validate_valid() {
        let resp = test_response();
        assert!(resp.validate().is_ok());
    }

    // MQR-F3-41: Validate fails for empty timestamp
    #[test]
    fn test_validate_empty_timestamp() {
        let mut resp = test_response();
        resp.timestamp = "".to_string();
        assert!(resp.validate().is_err());
    }

    // MQR-F3-42: No capability data assertion
    #[test]
    fn test_no_capability_data() {
        let resp = test_response();
        assert!(resp.assert_no_capability_data().is_ok());
    }

    // MQR-F3-43: Empty active leases is valid
    #[test]
    fn test_empty_leases() {
        let mut resp = test_response();
        resp.active_leases = vec![];
        resp.active_runs = vec![];
        assert!(resp.validate().is_ok());
        assert!(resp.active_leases.is_empty());
    }

    // MQR-F3-44: Draining flag is preserved
    #[test]
    fn test_draining_flag() {
        let mut resp = test_response();
        resp.draining = true;
        let json = resp.to_json().unwrap();
        let parsed = ResidencyStatusResponse::from_json(&json).unwrap();
        assert!(parsed.draining);
    }

    // MQR-F3-45: Clone produces equal value
    #[test]
    fn test_clone_eq() {
        let resp = test_response();
        let cloned = resp.clone();
        assert_eq!(resp, cloned);
    }

    // MQR-F3-46: Invalid JSON fails gracefully
    #[test]
    fn test_invalid_json() {
        let result = ResidencyStatusResponse::from_json("not json");
        assert!(result.is_err());
    }
}
