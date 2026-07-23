use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolStatus {
    Success,
    Error,
    AuthorizationRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResponse {
    pub request_id: String,
    pub status: McpToolStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub receipt_id: Option<String>,
}
