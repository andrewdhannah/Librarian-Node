//! HTTP bridge client — bounded communication with Windows runtime node.
//!
//! The client makes real HTTP requests to the Windows evidence and residency
//! endpoints. It deserializes responses into sealed packet types and validates
//! their integrity.
//!
//! Error categories are preserved — transport failures, HTTP status codes,
//! deserialization errors, and validation failures are all distinguishable.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::evidence_packet::EvidencePacket;

/// Classified bridge error — preserves the failure category.
#[derive(Debug, Clone)]
pub enum BridgeError {
    /// Connection failure (DNS, TCP, refused, etc.)
    Transport(String),

    /// Request timeout.
    Timeout(String),

    /// HTTP status error (404, 500, etc.) with response body.
    HttpStatus { status: u16, body: String },

    /// JSON deserialization failure.
    Deserialization { detail: String, raw_body: String },

    /// Packet structural validation failure.
    Validation(String),

    /// Identity mismatch between request and response.
    IdentityMismatch(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::Transport(msg) => write!(f, "Transport error: {}", msg),
            BridgeError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            BridgeError::HttpStatus { status, body } => {
                write!(f, "HTTP {}: {}", status, body)
            }
            BridgeError::Deserialization { detail, .. } => {
                write!(f, "Deserialization error: {}", detail)
            }
            BridgeError::Validation(msg) => write!(f, "Validation error: {}", msg),
            BridgeError::IdentityMismatch(msg) => {
                write!(f, "Identity mismatch: {}", msg)
            }
        }
    }
}

impl std::error::Error for BridgeError {}

/// Raw lifecycle event from the /evidence/lifecycle endpoint.
///
/// This is NOT a sealed packet type — it's the raw JSON shape returned
/// by the Windows lifecycle endpoint. The bridge client receives this
/// and the caller is responsible for converting to sealed types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifecycleEvent {
    pub evidence_id: String,
    pub event_type: String,
    pub model_id: String,
    pub profile_id: Option<String>,
    pub lease_id: String,
    pub run_id: String,
    pub process_id: Option<i32>,
    pub observed_state: Option<String>,
    pub observation_json: Option<String>,
    pub occurred_at: Option<String>,
    pub recorded_at: Option<String>,
}

/// Raw lifecycle response from the /evidence/lifecycle endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifecycleResponse {
    pub events: Vec<LifecycleEvent>,
    pub count: usize,
}

/// Evidence run response — EvidencePacket retrieved over HTTP.
///
/// This is the same EvidencePacket type but retrieved through the bridge.
/// The caller must still call `.validate()` and `.assert_no_capability_data()`.
pub type EvidenceRunResponse = EvidencePacket;

/// Residency status response — retrieved over HTTP.
///
/// This is the same ResidencyStatusResponse type but retrieved through the bridge.
/// The caller must still call `.validate()` and `.assert_no_capability_data()`.
pub use crate::residency_status::ResidencyStatusResponse;

/// HTTP bridge client for Windows runtime node communication.
pub struct BridgeClient {
    /// Base URL of the Windows runtime node (e.g., "http://127.0.0.1:9120").
    base_url: String,
    /// reqwest HTTP client with configured timeout.
    http: reqwest::Client,
}

impl BridgeClient {
    /// Create a new bridge client targeting the given Windows node.
    pub fn new(base_url: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        })
    }

    /// Create a bridge client with a custom timeout.
    pub fn with_timeout(base_url: &str, timeout: std::time::Duration) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        })
    }

    /// Create a bridge client with no timeout (for testing timeout scenarios).
    pub fn with_no_timeout(base_url: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .build()
            .context("Failed to create HTTP client")?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        })
    }

    /// Retrieve an EvidencePacket for a specific run.
    ///
    /// GET /evidence/runs/{run_id}?request_id=...&sha256=...&version=...
    ///
    /// Returns the EvidencePacket on success, or a classified BridgeError.
    pub async fn get_evidence_run(
        &self,
        run_id: &str,
        request_id: &str,
        sha256: &str,
        version: &str,
    ) -> std::result::Result<EvidenceRunResponse, BridgeError> {
        let url = format!(
            "{}/evidence/runs/{}",
            self.base_url, run_id
        );

        let response = self.http
            .get(&url)
            .query(&[
                ("request_id", request_id),
                ("sha256", sha256),
                ("version", version),
            ])
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    BridgeError::Timeout(format!("Evidence run request timed out: {}", e))
                } else if e.is_connect() {
                    BridgeError::Transport(format!("Failed to connect to Windows node: {}", e))
                } else {
                    BridgeError::Transport(format!("HTTP request failed: {}", e))
                }
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            BridgeError::Transport(format!("Failed to read response body: {}", e))
        })?;

        if !status.is_success() {
            return Err(BridgeError::HttpStatus {
                status: status.as_u16(),
                body: body.clone(),
            });
        }

        // Deserialize JSON into EvidencePacket
        let packet: EvidencePacket = serde_json::from_str(&body).map_err(|e| {
            BridgeError::Deserialization {
                detail: format!("Failed to parse EvidencePacket: {}", e),
                raw_body: body.clone(),
            }
        })?;

        // Validate the packet structure
        packet.validate().map_err(|e| {
            BridgeError::Validation(format!("EvidencePacket validation failed: {}", e))
        })?;

        Ok(packet)
    }

    /// Retrieve lifecycle events for a lease.
    ///
    /// GET /evidence/lifecycle?lease_id=...&limit=...
    ///
    /// Returns the raw lifecycle events on success.
    pub async fn get_evidence_lifecycle(
        &self,
        lease_id: &str,
        limit: Option<i64>,
    ) -> std::result::Result<LifecycleResponse, BridgeError> {
        let url = format!("{}/evidence/lifecycle", self.base_url);

        let mut request = self.http.get(&url).query(&[("lease_id", lease_id)]);
        if let Some(limit) = limit {
            request = request.query(&[("limit", &limit.to_string())]);
        }

        let response = request
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    BridgeError::Timeout(format!("Lifecycle request timed out: {}", e))
                } else if e.is_connect() {
                    BridgeError::Transport(format!("Failed to connect to Windows node: {}", e))
                } else {
                    BridgeError::Transport(format!("HTTP request failed: {}", e))
                }
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            BridgeError::Transport(format!("Failed to read response body: {}", e))
        })?;

        if !status.is_success() {
            return Err(BridgeError::HttpStatus {
                status: status.as_u16(),
                body: body.clone(),
            });
        }

        // Deserialize JSON into LifecycleResponse
        let lifecycle: LifecycleResponse = serde_json::from_str(&body).map_err(|e| {
            BridgeError::Deserialization {
                detail: format!("Failed to parse LifecycleResponse: {}", e),
                raw_body: body.clone(),
            }
        })?;

        Ok(lifecycle)
    }

    /// Retrieve residency status from the Windows node.
    ///
    /// GET /residency/status?model_id=...
    ///
    /// Returns the ResidencyStatusResponse on success.
    pub async fn get_residency_status(
        &self,
        model_id: Option<&str>,
    ) -> std::result::Result<ResidencyStatusResponse, BridgeError> {
        let url = format!("{}/residency/status", self.base_url);

        let mut request = self.http.get(&url);
        if let Some(model_id) = model_id {
            request = request.query(&[("model_id", model_id)]);
        }

        let response = request
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    BridgeError::Timeout(format!("Residency status request timed out: {}", e))
                } else if e.is_connect() {
                    BridgeError::Transport(format!("Failed to connect to Windows node: {}", e))
                } else {
                    BridgeError::Transport(format!("HTTP request failed: {}", e))
                }
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            BridgeError::Transport(format!("Failed to read response body: {}", e))
        })?;

        if !status.is_success() {
            return Err(BridgeError::HttpStatus {
                status: status.as_u16(),
                body: body.clone(),
            });
        }

        // Deserialize JSON into ResidencyStatusResponse
        let residency: crate::residency_status::ResidencyStatusResponse = serde_json::from_str(&body).map_err(|e| {
            BridgeError::Deserialization {
                detail: format!("Failed to parse ResidencyStatusResponse: {}", e),
                raw_body: body.clone(),
            }
        })?;

        // Validate the response structure
        residency.validate().map_err(|e| {
            BridgeError::Validation(format!("ResidencyStatusResponse validation failed: {}", e))
        })?;

        Ok(residency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // H1-B1: BridgeError display for each category
    #[test]
    fn test_bridge_error_transport_display() {
        let err = BridgeError::Transport("connection refused".to_string());
        assert!(err.to_string().contains("Transport"));
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_bridge_error_timeout_display() {
        let err = BridgeError::Timeout("request timed out".to_string());
        assert!(err.to_string().contains("Timeout"));
    }

    #[test]
    fn test_bridge_error_http_status_display() {
        let err = BridgeError::HttpStatus {
            status: 404,
            body: "not found".to_string(),
        };
        assert!(err.to_string().contains("404"));
    }

    #[test]
    fn test_bridge_error_deserialization_display() {
        let err = BridgeError::Deserialization {
            detail: "unexpected token".to_string(),
            raw_body: "{}".to_string(),
        };
        assert!(err.to_string().contains("Deserialization"));
    }

    #[test]
    fn test_bridge_error_validation_display() {
        let err = BridgeError::Validation("empty model_id".to_string());
        assert!(err.to_string().contains("Validation"));
    }

    #[test]
    fn test_bridge_error_identity_mismatch_display() {
        let err = BridgeError::IdentityMismatch("sha256 mismatch".to_string());
        assert!(err.to_string().contains("Identity mismatch"));
    }

    // H1-B2: LifecycleEvent serialization roundtrip
    #[test]
    fn test_lifecycle_event_roundtrip() {
        let event = LifecycleEvent {
            evidence_id: "ev-001".to_string(),
            event_type: "process_started".to_string(),
            model_id: "minicpm5-1b-q4km".to_string(),
            profile_id: Some("prof-001".to_string()),
            lease_id: "lease-001".to_string(),
            run_id: "run-001".to_string(),
            process_id: Some(12345),
            observed_state: Some("loading".to_string()),
            observation_json: Some("{}".to_string()),
            occurred_at: Some("2026-07-12T00:00:00Z".to_string()),
            recorded_at: Some("2026-07-12T00:00:01Z".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: LifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    // H1-B3: LifecycleResponse serialization roundtrip
    #[test]
    fn test_lifecycle_response_roundtrip() {
        let resp = LifecycleResponse {
            events: vec![],
            count: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: LifecycleResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, parsed);
    }

    // H1-B4: BridgeClient creation succeeds
    #[test]
    fn test_bridge_client_creation() {
        let client = BridgeClient::new("http://127.0.0.1:9120");
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url, "http://127.0.0.1:9120");
    }

    // H1-B5: BridgeClient strips trailing slash
    #[test]
    fn test_bridge_client_strips_trailing_slash() {
        let client = BridgeClient::new("http://127.0.0.1:9120/").unwrap();
        assert_eq!(client.base_url, "http://127.0.0.1:9120");
    }
}
