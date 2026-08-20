//! M1-C1 — Registry Observation MCP Adapter
//!
//! Connects MCP `registry.observe_*` tools to the governed projection module
//! (`RegistryObservationState`). This is a passthrough adapter: it consumes
//! projection envelopes and serializes them as MCP tool responses.
//!
//! **Invariant:** MCP output ≡ Projection module output.
//! The adapter does not transform, filter, or reorder projection fields.
//!
//! **Transport identity separation:** MCP session identity and network
//! connection identity must not enter registry semantics. The adapter
//! does not inject transport metadata into projection envelopes.

use librarian_contracts::registry_mcp::{McpToolRequest, McpToolResponse, McpToolStatus, RegistryMcpTool};
use librarian_contracts::node::registry_observation::RegistryObservationEnvelope;

use crate::registry_observation::RegistryObservationState;

/// Define the 5 `registry.observe_*` MCP tools.
///
/// These tools map 1:1 to projection methods. The tool names follow the
/// locked adapter contract (`REGISTRY-OBSERVATION-ADAPTER-CONTRACT-001`).
pub fn define_observation_tools() -> Vec<RegistryMcpTool> {
    vec![
        RegistryMcpTool {
            tool_name: "registry.observe_capability".to_string(),
            description: "Observe a single capability's identity and assurance axes through the governed projection boundary".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "capability_id": {
                        "type": "string",
                        "description": "The capability identifier to observe"
                    }
                },
                "required": ["capability_id"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": {"type": "string"},
                    "registry_identity": {"type": "object"},
                    "projection_observed_at": {"type": "string"},
                    "projection": {"type": "object"}
                }
            }),
            required_authority: "none".to_string(),
            read_only: true,
        },
        RegistryMcpTool {
            tool_name: "registry.observe_versions".to_string(),
            description: "Observe a capability's version history through the governed projection boundary".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "capability_id": {
                        "type": "string",
                        "description": "The capability identifier to observe versions for"
                    }
                },
                "required": ["capability_id"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": {"type": "string"},
                    "registry_identity": {"type": "object"},
                    "projection_observed_at": {"type": "string"},
                    "projection": {"type": "array"}
                }
            }),
            required_authority: "none".to_string(),
            read_only: true,
        },
        RegistryMcpTool {
            tool_name: "registry.observe_dependencies".to_string(),
            description: "Observe a capability's dependency graph through the governed projection boundary".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "capability_id": {
                        "type": "string",
                        "description": "The capability identifier to observe dependencies for"
                    }
                },
                "required": ["capability_id"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": {"type": "string"},
                    "registry_identity": {"type": "object"},
                    "projection_observed_at": {"type": "string"},
                    "projection": {"type": "array"}
                }
            }),
            required_authority: "none".to_string(),
            read_only: true,
        },
        RegistryMcpTool {
            tool_name: "registry.observe_types".to_string(),
            description: "Observe the capability type taxonomy through the governed projection boundary".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": {"type": "string"},
                    "registry_identity": {"type": "object"},
                    "projection_observed_at": {"type": "string"},
                    "projection": {"type": "array"}
                }
            }),
            required_authority: "none".to_string(),
            read_only: true,
        },
        RegistryMcpTool {
            tool_name: "registry.observe_overview".to_string(),
            description: "Observe registry overview counts through the governed projection boundary".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": {"type": "string"},
                    "registry_identity": {"type": "object"},
                    "projection_observed_at": {"type": "string"},
                    "projection": {"type": "object"}
                }
            }),
            required_authority: "none".to_string(),
            read_only: true,
        },
    ]
}

/// Execute an observation MCP tool request.
///
/// Returns `McpToolResponse` with the projection envelope serialized as JSON.
/// On projection failure, returns an error response with a stable error code.
///
/// **Transport identity separation:** The `request_id` from the MCP transport
/// is echoed back in the response, but it is NOT included in the projection
/// envelope or any registry semantics.
pub fn execute_observation_tool(
    request: McpToolRequest,
    state: &RegistryObservationState,
) -> McpToolResponse {
    let result = match request.tool_name.as_str() {
        "registry.observe_capability" => {
            let capability_id = match request.parameters.get("capability_id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => {
                    return McpToolResponse {
                        request_id: request.request_id,
                        status: McpToolStatus::Error,
                        result: None,
                        error: Some(serde_json::to_string(&serde_json::json!({
                            "code": "INVARIANT_VIOLATION",
                            "message": "Missing required parameter: capability_id"
                        })).unwrap_or_else(|_| r#"{"code":"INVARIANT_VIOLATION","message":"Missing required parameter: capability_id"}"#.to_string())),
                        receipt_id: None,
                    };
                }
            };
            match state.capability(capability_id) {
                Ok(envelope) => serialize_envelope(&envelope),
                Err(e) => return error_response(request.request_id, &e),
            }
        }
        "registry.observe_versions" => {
            let capability_id = match request.parameters.get("capability_id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => {
                    return McpToolResponse {
                        request_id: request.request_id,
                        status: McpToolStatus::Error,
                        result: None,
                        error: Some(serde_json::to_string(&serde_json::json!({
                            "code": "INVARIANT_VIOLATION",
                            "message": "Missing required parameter: capability_id"
                        })).unwrap_or_else(|_| r#"{"code":"INVARIANT_VIOLATION","message":"Missing required parameter: capability_id"}"#.to_string())),
                        receipt_id: None,
                    };
                }
            };
            match state.capability_versions(capability_id) {
                Ok(envelope) => serialize_envelope(&envelope),
                Err(e) => return error_response(request.request_id, &e),
            }
        }
        "registry.observe_dependencies" => {
            let capability_id = match request.parameters.get("capability_id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => {
                    return McpToolResponse {
                        request_id: request.request_id,
                        status: McpToolStatus::Error,
                        result: None,
                        error: Some(serde_json::to_string(&serde_json::json!({
                            "code": "INVARIANT_VIOLATION",
                            "message": "Missing required parameter: capability_id"
                        })).unwrap_or_else(|_| r#"{"code":"INVARIANT_VIOLATION","message":"Missing required parameter: capability_id"}"#.to_string())),
                        receipt_id: None,
                    };
                }
            };
            match state.capability_dependencies(capability_id) {
                Ok(envelope) => serialize_envelope(&envelope),
                Err(e) => return error_response(request.request_id, &e),
            }
        }
        "registry.observe_types" => {
            match state.capability_types() {
                Ok(envelope) => serialize_envelope(&envelope),
                Err(e) => return error_response(request.request_id, &e),
            }
        }
        "registry.observe_overview" => {
            match state.registry_overview() {
                Ok(envelope) => serialize_envelope(&envelope),
                Err(e) => return error_response(request.request_id, &e),
            }
        }
        _ => {
            return McpToolResponse {
                request_id: request.request_id,
                status: McpToolStatus::Error,
                result: None,
                error: Some(serde_json::to_string(&serde_json::json!({
                    "code": "UNSUPPORTED_OBSERVATION",
                    "message": format!("Unknown observation tool: {}", request.tool_name)
                })).unwrap_or_else(|_| format!(r#"{{"code":"UNSUPPORTED_OBSERVATION","message":"Unknown observation tool: {}"}}"#, request.tool_name))),
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

/// Serialize a projection envelope to JSON.
///
/// This is the equivalence boundary: the serialized JSON MUST be
/// byte-identical to `serde_json::to_value(&envelope)`.
fn serialize_envelope<T: serde::Serialize>(envelope: &RegistryObservationEnvelope<T>) -> serde_json::Value {
    serde_json::to_value(envelope).unwrap_or_else(|e| {
        serde_json::json!({
            "code": "PROJECTION_SNAPSHOT_FAILED",
            "message": format!("Failed to serialize projection envelope: {e}")
        })
    })
}

/// Build an error response from a projection failure.
///
/// Maps projection errors to stable MCP error codes. The error message
/// does not contain transport identity, internal storage paths, or
/// SQL query text.
fn error_response(request_id: String, error: &anyhow::Error) -> McpToolResponse {
    let error_msg = error.to_string();
    let (code, message) = if error_msg.contains("registry identity fail-closed") {
        ("REGISTRY_IDENTITY_UNAVAILABLE", error_msg)
    } else if error_msg.contains("fail-closed invariant violation") {
        ("INVARIANT_VIOLATION", error_msg)
    } else if error_msg.contains("not found in registry") {
        ("CAPABILITY_NOT_FOUND", error_msg)
    } else if error_msg.contains("no such table") || error_msg.contains("cannot read") {
        ("REGISTRY_NOT_INITIALIZED", error_msg)
    } else {
        ("PROJECTION_SNAPSHOT_FAILED", error_msg)
    };

    McpToolResponse {
        request_id,
        status: McpToolStatus::Error,
        result: None,
        error: Some(serde_json::to_string(&serde_json::json!({
            "code": code,
            "message": message
        })).unwrap_or_else(|_| format!(r#"{{"code":"{code}","message":"{message}"}}"#))),
        receipt_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Observation tool names match the locked adapter contract.
    #[test]
    fn observation_tool_names_match_contract() {
        let tools = define_observation_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.tool_name.as_str()).collect();
        assert_eq!(names, vec![
            "registry.observe_capability",
            "registry.observe_versions",
            "registry.observe_dependencies",
            "registry.observe_types",
            "registry.observe_overview",
        ]);
    }

    /// All observation tools are read-only.
    #[test]
    fn observation_tools_are_read_only() {
        let tools = define_observation_tools();
        for tool in &tools {
            assert!(tool.read_only, "{} must be read_only", tool.tool_name);
            assert_eq!(tool.required_authority, "none", "{} must require no authority", tool.tool_name);
        }
    }

    /// Observation tool count matches the locked adapter contract.
    #[test]
    fn observation_tool_count_matches_contract() {
        let tools = define_observation_tools();
        assert_eq!(tools.len(), 5, "adapter contract locks exactly 5 observation tools");
    }
}
