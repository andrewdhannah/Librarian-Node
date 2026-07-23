use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryOwnerAction {
    pub action_id: String,
    pub tool_name: String,
    pub request_id: String,
    pub parameters: serde_json::Value,
    pub requester_id: String,
    pub status: String,
    pub created_at: String,
    pub receipt_id: Option<String>,
}
