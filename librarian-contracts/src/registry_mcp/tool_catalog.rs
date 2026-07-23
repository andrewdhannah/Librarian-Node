use serde::{Deserialize, Serialize};

use super::tool_definition::RegistryMcpTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCatalog {
    pub tools: Vec<RegistryMcpTool>,
    pub version: String,
}
