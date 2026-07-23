use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolRequest {
    pub request_id: String,
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub requester_id: String,
    pub requested_at: String,
}
