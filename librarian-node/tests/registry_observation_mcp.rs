//! M1-C1 MCP adapter equivalence tests
//!
//! Verifies the adapter contract:
//! - MCP output ≡ Projection module output
//! - Transport identity isolation (MCP session ID must not enter registry semantics)
//! - No mutation capability
//! - Error envelope stability

use std::path::Path;

use librarian_contracts::registry_mcp::McpToolRequest;
use librarian_node::node::registry_observation_mcp::{
    define_observation_tools, execute_observation_tool,
};
use librarian_node::registry_observation::RegistryObservationState;
use rusqlite::Connection;

/// Apply the canonical governed schema to `conn`.
fn apply_canonical_schema(conn: &Connection) {
    conn.execute_batch(&librarian_core::startup::canonical_schema())
        .expect("apply canonical capability-registry schema");
}

/// Insert a minimal fixture for testing.
fn insert_fixture(conn: &Connection) {
    conn.execute_batch(
        "PRAGMA foreign_keys = OFF;
         INSERT INTO capabilities
           (id, name, type, description, status, active_version,
            availability, qualification, authority)
         VALUES
           ('alpha', 'Alpha Skill', 'skill', 'Alpha desc',
            'reviewed', 1, 'registered', 'passed', 'approved');
         INSERT INTO capability_versions
           (capability_id, version, body, content_hash, changelog, author,
            review_notes, qualification_evidence_id, profile_id, created_at)
         VALUES
           ('alpha', 1, 'body', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            'init', 'owner', NULL, NULL, NULL, '2026-01-01T00:00:00Z');
         INSERT INTO capability_types
           (capability_type_id, name, description, category,
            default_profile_id, default_policy_id)
         VALUES
           ('skill', 'Skill', 'Standard skill', 'standard', NULL, NULL);",
    )
    .expect("insert fixture");
}

/// Build a test registry DB.
fn fixture_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("capability-registry.sqlite");
    let conn = Connection::open(&db_path).expect("open fixture db");
    apply_canonical_schema(&conn);
    insert_fixture(&conn);
    (dir, db_path)
}

// ---------------------------------------------------------------------------
// Tool definition tests
// ---------------------------------------------------------------------------

#[test]
fn observation_tool_names_match_locked_contract() {
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

#[test]
fn observation_tools_are_all_read_only() {
    let tools = define_observation_tools();
    for tool in &tools {
        assert!(tool.read_only, "{} must be read_only", tool.tool_name);
        assert_eq!(tool.required_authority, "none", "{} must require no authority", tool.tool_name);
    }
}

#[test]
fn observation_tool_count_matches_contract() {
    let tools = define_observation_tools();
    assert_eq!(tools.len(), 5, "adapter contract locks exactly 5 observation tools");
}

// ---------------------------------------------------------------------------
// Output equivalence tests — MCP output ≡ Projection module output
// ---------------------------------------------------------------------------

#[test]
fn observe_capability_output_matches_projection() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "req-001".to_string(),
        tool_name: "registry.observe_capability".to_string(),
        parameters: serde_json::json!({"capability_id": "alpha"}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Success);
    let result = response.result.expect("must have result");

    // Compare with direct projection output
    let projection = state.capability("alpha").expect("direct projection");
    let projection_json = serde_json::to_value(&projection).expect("serialize projection");

    assert_eq!(result, projection_json, "MCP output must equal projection output");
}

#[test]
fn observe_versions_output_matches_projection() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "req-002".to_string(),
        tool_name: "registry.observe_versions".to_string(),
        parameters: serde_json::json!({"capability_id": "alpha"}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Success);
    let result = response.result.expect("must have result");

    let projection = state.capability_versions("alpha").expect("direct projection");
    let projection_json = serde_json::to_value(&projection).expect("serialize projection");

    assert_eq!(result, projection_json, "MCP output must equal projection output");
}

#[test]
fn observe_dependencies_output_matches_projection() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "req-003".to_string(),
        tool_name: "registry.observe_dependencies".to_string(),
        parameters: serde_json::json!({"capability_id": "alpha"}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Success);
    let result = response.result.expect("must have result");

    let projection = state.capability_dependencies("alpha").expect("direct projection");
    let projection_json = serde_json::to_value(&projection).expect("serialize projection");

    assert_eq!(result, projection_json, "MCP output must equal projection output");
}

#[test]
fn observe_types_output_matches_projection() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "req-004".to_string(),
        tool_name: "registry.observe_types".to_string(),
        parameters: serde_json::json!({}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Success);
    let result = response.result.expect("must have result");

    let projection = state.capability_types().expect("direct projection");
    let projection_json = serde_json::to_value(&projection).expect("serialize projection");

    assert_eq!(result, projection_json, "MCP output must equal projection output");
}

#[test]
fn observe_overview_output_matches_projection() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "req-005".to_string(),
        tool_name: "registry.observe_overview".to_string(),
        parameters: serde_json::json!({}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Success);
    let result = response.result.expect("must have result");

    let projection = state.registry_overview().expect("direct projection");
    let projection_json = serde_json::to_value(&projection).expect("serialize projection");

    assert_eq!(result, projection_json, "MCP output must equal projection output");
}

// ---------------------------------------------------------------------------
// Transport identity isolation
// ---------------------------------------------------------------------------

#[test]
fn mcp_request_id_not_in_projection_envelope() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "transport-session-abc-123".to_string(),
        tool_name: "registry.observe_capability".to_string(),
        parameters: serde_json::json!({"capability_id": "alpha"}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    let result = response.result.expect("must have result");

    // The request_id appears in the response wrapper but NOT in the projection envelope
    assert_eq!(response.request_id, "transport-session-abc-123", "request_id echoed in response");
    let result_str = result.to_string();
    assert!(!result_str.contains("transport-session-abc-123"),
        "transport identity must not enter projection envelope");

    // Verify the projection envelope uses the sealed node_id, not transport identity
    let projection = state.capability("alpha").expect("direct projection");
    assert_eq!(projection.node_id, "test-node");
    assert_ne!(projection.node_id, "transport-session-abc-123");
}

// ---------------------------------------------------------------------------
// Error envelope stability
// ---------------------------------------------------------------------------

#[test]
fn missing_capability_returns_not_found_error() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "req-err-1".to_string(),
        tool_name: "registry.observe_capability".to_string(),
        parameters: serde_json::json!({"capability_id": "nonexistent"}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Error);
    let error = response.error.expect("must have error");
    let error_json: serde_json::Value = serde_json::from_str(&error).expect("error must be JSON");
    assert_eq!(error_json["code"], "CAPABILITY_NOT_FOUND");
}

#[test]
fn missing_parameter_returns_invariant_violation() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "req-err-2".to_string(),
        tool_name: "registry.observe_capability".to_string(),
        parameters: serde_json::json!({}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Error);
    let error = response.error.expect("must have error");
    let error_json: serde_json::Value = serde_json::from_str(&error).expect("error must be JSON");
    assert_eq!(error_json["code"], "INVARIANT_VIOLATION");
}

#[test]
fn unknown_tool_returns_unsupported_observation() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let request = McpToolRequest {
        request_id: "req-err-3".to_string(),
        tool_name: "registry.observe_bogus".to_string(),
        parameters: serde_json::json!({}),
        requester_id: "test-requester".to_string(),
        requested_at: "2026-08-07T00:00:00Z".to_string(),
    };

    let response = execute_observation_tool(request, &state);
    assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Error);
    let error = response.error.expect("must have error");
    let error_json: serde_json::Value = serde_json::from_str(&error).expect("error must be JSON");
    assert_eq!(error_json["code"], "UNSUPPORTED_OBSERVATION");
}

// ---------------------------------------------------------------------------
// No mutation capability
// ---------------------------------------------------------------------------

#[test]
fn observation_adapter_does_not_modify_registry() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    // Read initial state
    let before = state.capability("alpha").expect("initial read");
    let before_identity = before.registry_identity.value.clone();

    // Execute all observation tools
    for (tool_name, params) in [
        ("registry.observe_capability", serde_json::json!({"capability_id": "alpha"})),
        ("registry.observe_versions", serde_json::json!({"capability_id": "alpha"})),
        ("registry.observe_dependencies", serde_json::json!({"capability_id": "alpha"})),
        ("registry.observe_types", serde_json::json!({})),
        ("registry.observe_overview", serde_json::json!({})),
    ] {
        let request = McpToolRequest {
            request_id: format!("mutation-test-{tool_name}"),
            tool_name: tool_name.to_string(),
            parameters: params,
            requester_id: "test-requester".to_string(),
            requested_at: "2026-08-07T00:00:00Z".to_string(),
        };
        let response = execute_observation_tool(request, &state);
        assert_eq!(response.status, librarian_contracts::registry_mcp::McpToolStatus::Success);
    }

    // Registry state is unchanged
    let after = state.capability("alpha").expect("read after observations");
    assert_eq!(before_identity, after.registry_identity.value, "identity unchanged after observations");
    assert_eq!(before.projection, after.projection, "projection unchanged after observations");
}
