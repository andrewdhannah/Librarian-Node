use std::path::PathBuf;

use librarian_contracts::registry_mcp::{
    McpToolCatalog, McpToolRequest, McpToolResponse, McpToolStatus, RegistryMcpTool,
    RegistryOwnerAction,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::identity_service::NodeIdentityService;
use super::owner_workflow_service::OwnerWorkflowService;
use super::registry_candidate_service::RegistryCandidateService;
use super::state::NodeStateMachine;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    pending_actions: Vec<RegistryOwnerAction>,
}

pub struct RegistryMcpService {
    pending_actions: Vec<RegistryOwnerAction>,
    persistence_path: PathBuf,
}

impl RegistryMcpService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let pending_actions = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => state.pending_actions,
                    Err(_) => Vec::new(),
                },
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        RegistryMcpService {
            pending_actions,
            persistence_path,
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            pending_actions: self.pending_actions.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    fn next_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn get_tool_catalog(&self) -> McpToolCatalog {
        McpToolCatalog {
            tools: define_tools(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn execute_tool(
        &mut self,
        request: McpToolRequest,
        registry_candidate_service: &RegistryCandidateService,
        identity_service: &NodeIdentityService,
        node_state: &NodeStateMachine,
        owner_workflow_service: &mut OwnerWorkflowService,
    ) -> McpToolResponse {
        let tools = define_tools();
        let tool = match tools.into_iter().find(|t| t.tool_name == request.tool_name) {
            Some(t) => t,
            None => {
                return McpToolResponse {
                    request_id: request.request_id,
                    status: McpToolStatus::Error,
                    result: None,
                    error: Some(format!("Unknown tool: {}", request.tool_name)),
                    receipt_id: None,
                };
            }
        };

        if tool.read_only {
            self.execute_read_tool(request, tool, registry_candidate_service, identity_service, node_state)
        } else {
            self.execute_write_tool(request, tool, owner_workflow_service)
        }
    }

    fn execute_read_tool(
        &self,
        request: McpToolRequest,
        _tool: RegistryMcpTool,
        registry_candidate_service: &RegistryCandidateService,
        identity_service: &NodeIdentityService,
        node_state: &NodeStateMachine,
    ) -> McpToolResponse {
        let result = match request.tool_name.as_str() {
            "registry.inspect_node" => {
                let identity = identity_service.get_identity();
                serde_json::json!({
                    "node_id": identity.node_id,
                    "display_name": identity.display_name,
                    "platform": identity.platform,
                    "runtime_version": identity.runtime_version,
                    "state": node_state.current().as_str(),
                    "last_state_change": node_state.last_change(),
                })
            }
            "registry.query_candidates" => {
                let status_filter = request.parameters.get("status").and_then(|v| v.as_str());
                let candidates = registry_candidate_service.get_candidates(status_filter);
                serde_json::json!({
                    "candidates": candidates,
                    "count": candidates.len(),
                })
            }
            "registry.retrieve_evidence" => {
                let candidate_id = match request.parameters.get("candidate_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => {
                        return McpToolResponse {
                            request_id: request.request_id,
                            status: McpToolStatus::Error,
                            result: None,
                            error: Some("Missing required parameter: candidate_id".to_string()),
                            receipt_id: None,
                        };
                    }
                };
                let evidence = registry_candidate_service.get_evidence(candidate_id);
                serde_json::json!({
                    "evidence": evidence,
                    "count": evidence.len(),
                })
            }
            _ => {
                return McpToolResponse {
                    request_id: request.request_id,
                    status: McpToolStatus::Error,
                    result: None,
                    error: Some(format!("Unhandled read tool: {}", request.tool_name)),
                    receipt_id: None,
                };
            }
        };

        McpToolResponse {
            request_id: request.request_id,
            status: McpToolStatus::Success,
            result: Some(result),
            error: None,
            receipt_id: None,
        }
    }

    fn execute_write_tool(
        &mut self,
        request: McpToolRequest,
        _tool: RegistryMcpTool,
        owner_workflow_service: &mut OwnerWorkflowService,
    ) -> McpToolResponse {
        let action_id = self.next_id();
        let now = chrono::Utc::now().to_rfc3339();

        let action = RegistryOwnerAction {
            action_id: action_id.clone(),
            tool_name: request.tool_name.clone(),
            request_id: request.request_id.clone(),
            parameters: request.parameters.clone(),
            requester_id: request.requester_id.clone(),
            status: "pending_authorization".to_string(),
            created_at: now.clone(),
            receipt_id: None,
        };

        let summary = format!(
            "Registry MCP write tool '{}' requested by '{}'",
            request.tool_name, request.requester_id
        );
        owner_workflow_service.log_action(
            "mcp_write_request",
            "registry_mcp_action",
            &summary,
            None,
            Some(&action_id),
        );

        self.pending_actions.push(action);
        self.persist();

        McpToolResponse {
            request_id: request.request_id,
            status: McpToolStatus::AuthorizationRequired,
            result: Some(serde_json::json!({
                "action_id": action_id,
                "tool_name": request.tool_name,
                "message": "Owner authorization required before this write tool can execute",
            })),
            error: None,
            receipt_id: None,
        }
    }

    pub fn get_pending_actions(&self) -> &[RegistryOwnerAction] {
        &self.pending_actions
    }

    pub fn resolve_action(&mut self, action_id: &str, receipt_id: &str) -> Option<RegistryOwnerAction> {
        let idx = self
            .pending_actions
            .iter()
            .position(|a| a.action_id == action_id)?;
        let action = &mut self.pending_actions[idx];
        action.status = "executed".to_string();
        action.receipt_id = Some(receipt_id.to_string());
        let result = action.clone();
        self.persist();
        Some(result)
    }
}

fn define_tools() -> Vec<RegistryMcpTool> {
    vec![
        RegistryMcpTool {
            tool_name: "registry.inspect_node".to_string(),
            description: "Query node identity and state".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": {"type": "string"},
                    "display_name": {"type": "string"},
                    "platform": {"type": "string"},
                    "runtime_version": {"type": "string"},
                    "state": {"type": "string"},
                    "last_state_change": {"type": "string"}
                }
            }),
            required_authority: "owner".to_string(),
            read_only: true,
        },
        RegistryMcpTool {
            tool_name: "registry.query_candidates".to_string(),
            description: "List pending candidates in the registry".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string", "description": "Optional status filter"}
                },
                "required": []
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "candidates": {"type": "array"},
                    "count": {"type": "integer"}
                }
            }),
            required_authority: "owner".to_string(),
            read_only: true,
        },
        RegistryMcpTool {
            tool_name: "registry.retrieve_evidence".to_string(),
            description: "Get candidate evidence packages by candidate ID".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "candidate_id": {"type": "string", "description": "The candidate ID to retrieve evidence for"}
                },
                "required": ["candidate_id"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "evidence": {"type": "array"},
                    "count": {"type": "integer"}
                }
            }),
            required_authority: "owner".to_string(),
            read_only: true,
        },
        RegistryMcpTool {
            tool_name: "registry.submit_review".to_string(),
            description: "Submit owner decision on a registry candidate. Requires owner authorization.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "candidate_id": {"type": "string"},
                    "decision": {"type": "string", "enum": ["approve", "reject", "request_info"]},
                    "reviewer": {"type": "string"},
                    "reason": {"type": "string"}
                },
                "required": ["candidate_id", "decision", "reviewer", "reason"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action_id": {"type": "string"},
                    "tool_name": {"type": "string"},
                    "message": {"type": "string"}
                }
            }),
            required_authority: "owner".to_string(),
            read_only: false,
        },
        RegistryMcpTool {
            tool_name: "registry.request_action".to_string(),
            description: "Request a governed action on the registry. Requires owner authorization.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action_type": {"type": "string", "description": "Type of action to perform"},
                    "params": {"type": "object", "description": "Action-specific parameters"}
                },
                "required": ["action_type", "params"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action_id": {"type": "string"},
                    "tool_name": {"type": "string"},
                    "message": {"type": "string"}
                }
            }),
            required_authority: "owner".to_string(),
            read_only: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{
        NodeIdentityService, NodeStateMachine, OwnerWorkflowService, RegistryCandidateService,
    };
    use tempfile::tempdir;

    fn test_setup() -> (
        RegistryMcpService,
        RegistryCandidateService,
        NodeIdentityService,
        NodeStateMachine,
        OwnerWorkflowService,
        tempfile::TempDir,
    ) {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join("registry_mcp.json");
        let candidates_path = dir.path().join("candidates.json");
        let identity_path = dir.path().join("identity.json");
        let ow_path = dir.path().join("owner_workflows.json");

        let mcp = RegistryMcpService::new(mcp_path);
        let candidates = RegistryCandidateService::new(candidates_path);
        let identity = NodeIdentityService::new(identity_path);
        let state = NodeStateMachine::new();
        let ow = OwnerWorkflowService::new(ow_path);

        (mcp, candidates, identity, state, ow, dir)
    }

    #[test]
    fn test_catalog_returns_all_five_tools() {
        let (mcp, _, _, _, _, _) = test_setup();
        let catalog = mcp.get_tool_catalog();
        assert_eq!(catalog.tools.len(), 5);

        let names: Vec<&str> = catalog.tools.iter().map(|t| t.tool_name.as_str()).collect();
        assert!(names.contains(&"registry.inspect_node"));
        assert!(names.contains(&"registry.query_candidates"));
        assert!(names.contains(&"registry.retrieve_evidence"));
        assert!(names.contains(&"registry.submit_review"));
        assert!(names.contains(&"registry.request_action"));
    }

    #[test]
    fn test_catalog_tools_have_schemas() {
        let (mcp, _, _, _, _, _) = test_setup();
        let catalog = mcp.get_tool_catalog();
        for tool in &catalog.tools {
            assert!(!tool.description.is_empty(), "Tool {} missing description", tool.tool_name);
            assert!(!tool.required_authority.is_empty(), "Tool {} missing authority", tool.tool_name);
            assert!(tool.input_schema.is_object(), "Tool {} missing input_schema", tool.tool_name);
            assert!(tool.output_schema.is_object(), "Tool {} missing output_schema", tool.tool_name);
        }
    }

    #[test]
    fn test_catalog_tools_have_correct_read_only_flags() {
        let (mcp, _, _, _, _, _) = test_setup();
        let catalog = mcp.get_tool_catalog();

        for tool in &catalog.tools {
            match tool.tool_name.as_str() {
                "registry.inspect_node"
                | "registry.query_candidates"
                | "registry.retrieve_evidence" => {
                    assert!(tool.read_only, "Tool {} should be read-only", tool.tool_name);
                }
                "registry.submit_review" | "registry.request_action" => {
                    assert!(!tool.read_only, "Tool {} should be write", tool.tool_name);
                }
                _ => panic!("Unexpected tool: {}", tool.tool_name),
            }
        }
    }

    #[test]
    fn test_execute_read_tool_inspect_node() {
        let (mut mcp, candidates, identity, state, mut ow, _dir) = test_setup();

        let request = McpToolRequest {
            request_id: "req-1".to_string(),
            tool_name: "registry.inspect_node".to_string(),
            parameters: serde_json::json!({}),
            requester_id: "bridge-1".to_string(),
            requested_at: chrono::Utc::now().to_rfc3339(),
        };

        let response = mcp.execute_tool(request, &candidates, &identity, &state, &mut ow);
        assert_eq!(response.status, McpToolStatus::Success);
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert!(result.get("node_id").is_some());
        assert!(result.get("state").is_some());
    }

    #[test]
    fn test_execute_read_tool_query_candidates() {
        let (mut mcp, candidates, identity, state, mut ow, _dir) = test_setup();

        let request = McpToolRequest {
            request_id: "req-2".to_string(),
            tool_name: "registry.query_candidates".to_string(),
            parameters: serde_json::json!({}),
            requester_id: "bridge-1".to_string(),
            requested_at: chrono::Utc::now().to_rfc3339(),
        };

        let response = mcp.execute_tool(request, &candidates, &identity, &state, &mut ow);
        assert_eq!(response.status, McpToolStatus::Success);
        let result = response.result.unwrap();
        assert_eq!(result["count"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_execute_read_tool_retrieve_evidence() {
        let (mut mcp, candidates, identity, state, mut ow, _dir) = test_setup();

        let request = McpToolRequest {
            request_id: "req-3".to_string(),
            tool_name: "registry.retrieve_evidence".to_string(),
            parameters: serde_json::json!({"candidate_id": "nonexistent"}),
            requester_id: "bridge-1".to_string(),
            requested_at: chrono::Utc::now().to_rfc3339(),
        };

        let response = mcp.execute_tool(request, &candidates, &identity, &state, &mut ow);
        assert_eq!(response.status, McpToolStatus::Success);
        let result = response.result.unwrap();
        assert_eq!(result["count"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_execute_write_tool_returns_authorization_required() {
        let (mut mcp, candidates, identity, state, mut ow, _dir) = test_setup();

        let request = McpToolRequest {
            request_id: "req-4".to_string(),
            tool_name: "registry.submit_review".to_string(),
            parameters: serde_json::json!({
                "candidate_id": "cand-1",
                "decision": "approve",
                "reviewer": "bridge",
                "reason": "Approved via MCP"
            }),
            requester_id: "bridge-1".to_string(),
            requested_at: chrono::Utc::now().to_rfc3339(),
        };

        let response = mcp.execute_tool(request, &candidates, &identity, &state, &mut ow);
        assert_eq!(response.status, McpToolStatus::AuthorizationRequired);
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert!(result.get("action_id").is_some());
        assert_eq!(result["tool_name"].as_str().unwrap(), "registry.submit_review");
    }

    #[test]
    fn test_execute_invalid_tool_returns_error() {
        let (mut mcp, candidates, identity, state, mut ow, _dir) = test_setup();

        let request = McpToolRequest {
            request_id: "req-5".to_string(),
            tool_name: "registry.nonexistent".to_string(),
            parameters: serde_json::json!({}),
            requester_id: "bridge-1".to_string(),
            requested_at: chrono::Utc::now().to_rfc3339(),
        };

        let response = mcp.execute_tool(request, &candidates, &identity, &state, &mut ow);
        assert_eq!(response.status, McpToolStatus::Error);
        assert!(response.error.is_some());
        assert!(response.error.unwrap().contains("Unknown tool"));
    }

    #[test]
    fn test_write_tool_creates_pending_action() {
        let (mut mcp, candidates, identity, state, mut ow, _dir) = test_setup();

        let request = McpToolRequest {
            request_id: "req-6".to_string(),
            tool_name: "registry.request_action".to_string(),
            parameters: serde_json::json!({
                "action_type": "expire_candidates",
                "params": {"days_old": 30}
            }),
            requester_id: "bridge-1".to_string(),
            requested_at: chrono::Utc::now().to_rfc3339(),
        };

        let response = mcp.execute_tool(request, &candidates, &identity, &state, &mut ow);
        assert_eq!(response.status, McpToolStatus::AuthorizationRequired);

        assert_eq!(mcp.pending_actions.len(), 1);
        assert_eq!(mcp.pending_actions[0].tool_name, "registry.request_action");
        assert_eq!(mcp.pending_actions[0].status, "pending_authorization");
    }

    #[test]
    fn test_resolve_action_updates_status() {
        let (mut mcp, _candidates, _identity, _state, _ow, _dir) = test_setup();
        let action_id = mcp.next_id();

        mcp.pending_actions.push(RegistryOwnerAction {
            action_id: action_id.clone(),
            tool_name: "registry.submit_review".to_string(),
            request_id: "req-7".to_string(),
            parameters: serde_json::json!({}),
            requester_id: "bridge-1".to_string(),
            status: "pending_authorization".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            receipt_id: None,
        });

        let resolved = mcp.resolve_action(&action_id, "receipt-1").unwrap();
        assert_eq!(resolved.status, "executed");
        assert_eq!(resolved.receipt_id, Some("receipt-1".to_string()));

        assert_eq!(mcp.pending_actions[0].status, "executed");
    }

    #[test]
    fn test_retrieve_evidence_missing_param() {
        let (mut mcp, candidates, identity, state, mut ow, _dir) = test_setup();

        let request = McpToolRequest {
            request_id: "req-8".to_string(),
            tool_name: "registry.retrieve_evidence".to_string(),
            parameters: serde_json::json!({}),
            requester_id: "bridge-1".to_string(),
            requested_at: chrono::Utc::now().to_rfc3339(),
        };

        let response = mcp.execute_tool(request, &candidates, &identity, &state, &mut ow);
        assert_eq!(response.status, McpToolStatus::Error);
        assert!(response.error.is_some());
    }

    #[test]
    fn test_catalog_version_is_set() {
        let (mcp, _, _, _, _, _) = test_setup();
        let catalog = mcp.get_tool_catalog();
        assert!(!catalog.version.is_empty());
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join("registry_mcp.json");
        let cand_path = dir.path().join("candidates.json");
        let id_path = dir.path().join("identity.json");
        let ow_path = dir.path().join("owner_workflows.json");

        let action_id;
        {
            let mut mcp = RegistryMcpService::new(&mcp_path);
            let mut ow = OwnerWorkflowService::new(&ow_path);
            let candidates = RegistryCandidateService::new(&cand_path);
            let identity = NodeIdentityService::new(&id_path);
            let state = NodeStateMachine::new();

            let request = McpToolRequest {
                request_id: "req-persist".to_string(),
                tool_name: "registry.request_action".to_string(),
                parameters: serde_json::json!({"action_type": "test", "params": {}}),
                requester_id: "bridge-1".to_string(),
                requested_at: chrono::Utc::now().to_rfc3339(),
            };
            let response = mcp.execute_tool(request, &candidates, &identity, &state, &mut ow);
            action_id = response.result.unwrap()["action_id"].as_str().unwrap().to_string();
        }

        {
            let mcp = RegistryMcpService::new(&mcp_path);
            assert_eq!(mcp.pending_actions.len(), 1);
            assert_eq!(mcp.pending_actions[0].action_id, action_id);
        }
    }
}
