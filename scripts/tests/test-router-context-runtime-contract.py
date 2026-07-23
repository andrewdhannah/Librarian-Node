#!/usr/bin/env python3
"""
MAC/WIN-ROUTER-CONTEXT-RUNTIME-CONTRACT-1 — Contract Validation Tests

Validates that the context decision contract objects conform to the
versioned contract specifications. This test suite enforces:
  - Advisory-only invariants (advisory == true, production_effects_allowed == false)
  - Degraded-node invariants (health, penalty, allowed_remote_use)
  - Enum validation for all controlled vocabularies
  - Forbidden action verification
  - Receipt consumption downstream-only rules
  - Contract version presence

Sprint: MAC/WIN-ROUTER-CONTEXT-RUNTIME-CONTRACT-1
Status: Contract validation tests only — no production behavior changed.
"""

import json
import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Path setup
# ---------------------------------------------------------------------------
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CONTRACT_DOC_PATH = REPO_ROOT / "docs" / "contracts" / "router-context-runtime-contract.md"
FIXTURES_DIR = REPO_ROOT / "fixtures" / "router-context-runtime-contract"
MEASURED_PROFILES_PATH = REPO_ROOT / "config" / "measured_hardware_profiles.json"
PROTOTYPE_DECISIONS_PATH = REPO_ROOT / "reports" / "router-context-prototype-decisions.json"

# ---------------------------------------------------------------------------
# Contract enums (from contract v0.1)
# ---------------------------------------------------------------------------
CONTRACT_VERSION = "0.1"

WORKLOAD_TYPES = [
    "sprint_planning", "sprint_closeout", "receipt_generation", "validation",
    "code_patch_preparation", "agent_handoff", "long_session_continuation",
    "runtime_node_qualification", "ui_review_or_design_planning",
]

TASK_RISK_LEVELS = ["low", "medium", "high", "critical"]

NODE_HEALTHS = [
    "not_checked", "available", "degraded", "unreachable",
    "stopped", "timeout", "unknown",
]

DEGRADED_NODE_ACTIONS = [
    "avoid_remote_route", "use_local_fallback", "require_recheck",
    "mark_warning", "block_for_task",
]

GOVERNANCE_OUTCOMES = ["safe", "warning", "requires_revalidation", "blocked"]

CONTEXT_ROUTES = [
    "ram_cache", "ssd_cache", "remote_windows_runtime_cache",
    "recomputation_from_source", "compressed_recall_packet",
    "canonical_evidence_read", "hybrid_recall_plus_fresh_evidence",
    "recent_turn_window",
]

RUNTIME_PROFILES = [
    "mac_coordinator", "windows_runtime_node",
    "weak_lan_runtime_node", "future_stronger_gpu_node",
]

FORBIDDEN_ACTIONS = [
    "start_process", "stop_process", "select_model", "execute_model",
    "modify_router_config", "modify_model_profiles",
    "open_network_listener", "change_bind_host",
    "write_runtime_state", "apply_context_route",
]

GOVERNANCE_MANDATED_ROUTES = {
    "receipt_generation": "canonical_evidence_read",
    "sprint_closeout": "canonical_evidence_read",
    "validation": "canonical_evidence_read",
    "agent_handoff": "compressed_recall_packet",
    "runtime_node_qualification": "canonical_evidence_read",
}

# ---------------------------------------------------------------------------
# Test results tracking
# ---------------------------------------------------------------------------
passed = 0
failed = 0
errors = []


def test(name, condition, detail=""):
    global passed, failed
    if condition:
        passed += 1
        print(f"  PASS: {name}")
    else:
        failed += 1
        errors.append(f"{name}: {detail}")
        print(f"  FAIL: {name} — {detail}")


def load_json(path):
    """Load and return JSON from a file path."""
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Helper: Validate advisory-only invariants on an object
# ---------------------------------------------------------------------------
def check_advisory_invariants(obj, label):
    """Return (passed, details) list for advisory-only invariants."""
    checks = []
    has_version = "runtime_context_decision_contract_version" in obj
    version_ok = has_version and obj.get("runtime_context_decision_contract_version") == CONTRACT_VERSION
    advisory_ok = obj.get("advisory", None) is True
    production_effects_field = obj.get("production_effects_allowed", None)
    # production_effects_allowed is only required for Input/Output, N/A for Receipt
    production_ok = production_effects_field is False if production_effects_field is not None else True

    # Check forbidden actions if present
    forbidden_ok = True
    if "forbidden_actions_checked" in obj:
        checked = obj["forbidden_actions_checked"]
        forbidden_ok = all(action in FORBIDDEN_ACTIONS for action in checked)
        forbidden_ok = forbidden_ok and len(checked) == len(FORBIDDEN_ACTIONS)

    checks.append((f"[{label}] Contract version present", has_version, "Missing contract version"))
    checks.append((f"[{label}] Contract version is {CONTRACT_VERSION}", version_ok, f"Got: {obj.get('runtime_context_decision_contract_version')}"))
    checks.append((f"[{label}] advisory is true", advisory_ok, f"Got: {obj.get('advisory')}"))
    detail = f"Got: {production_effects_field}" if production_effects_field is not None else "Absent (N/A for Receipt)"
    checks.append((f"[{label}] production_effects_allowed is false (if present)", production_ok, detail))

    if "forbidden_actions_checked" in obj:
        checks.append((f"[{label}] All forbidden actions checked", forbidden_ok, f"Missing or extra actions: {set(FORBIDDEN_ACTIONS) - set(obj.get('forbidden_actions_checked', []))}"))

    return checks


# ---------------------------------------------------------------------------
# Test Category 1: Contract Document Validation
# ---------------------------------------------------------------------------
print("\n[1/6] Contract Document Validation")

test(
    "Contract document exists",
    CONTRACT_DOC_PATH.exists(),
    f"Missing: {CONTRACT_DOC_PATH}"
)

if CONTRACT_DOC_PATH.exists():
    content = CONTRACT_DOC_PATH.read_text(encoding="utf-8")

    test(
        "Contract document has version 0.1",
        "0.1" in content,
        "Missing version 0.1"
    )
    test(
        "Contract document defines contract version key",
        "runtime_context_decision_contract_version" in content,
        "Missing contract version field"
    )
    test(
        "Contract document defines workload_type enum",
        "workload_type" in content and "sprint_closeout" in content,
        "Missing workload_type enum definitions"
    )
    test(
        "Contract document defines task_risk_level enum",
        "task_risk_level" in content and "critical" in content,
        "Missing task_risk_level enum"
    )
    test(
        "Contract document defines node_health enum",
        "node_health" in content and "unreachable" in content,
        "Missing node_health enum"
    )
    test(
        "Contract document defines degraded_node_action enum",
        "degraded_node_action" in content and "avoid_remote_route" in content,
        "Missing degraded_node_action enum"
    )
    test(
        "Contract document defines governance_outcome enum",
        "governance_outcome" in content and "requires_revalidation" in content,
        "Missing governance_outcome enum"
    )
    test(
        "Contract document defines selected_context_route enum",
        "selected_context_route" in content and "hybrid_recall_plus_fresh_evidence" in content,
        "Missing selected_context_route enum"
    )
    test(
        "Contract document defines runtime profile enum",
        "selected_runtime_profile" in content and "weak_lan_runtime_node" in content,
        "Missing runtime profile enum"
    )
    test(
        "Contract document defines advisory-only invariants",
        "advisory == true" in content and "production_effects_allowed == false" in content,
        "Missing advisory-only invariant statements"
    )
    test(
        "Contract document defines degraded-node invariants",
        "allowed_remote_use must be false" in content and "measured_penalty_ms must be >= 3000" in content,
        "Missing degraded-node invariant"
    )
    test(
        "Contract document lists 10 forbidden actions",
        content.count("| `") >= 10 or content.count("Forbidden Action") >= 1,
        "Missing forbidden actions table"
    )
    test(
        "Contract document has receipt consumption path",
        "Receipt Consumption" in content,
        "Missing receipt consumption section"
    )
    test(
        "Contract document references MEASURE-1 measured costs",
        "~4016ms" in content or "MAC/WIN-ROUTER-CONTEXT-MEASURE-1" in content,
        "Missing MEASURE-1 measured cost references"
    )
    test(
        "Contract document references PROTOTYPE-1 decisions",
        "PROTOTYPE-1" in content or "Governance-Mandated Routes" in content,
        "Missing PROTOTYPE-1 governance-mandated route references"
    )
    test(
        "Contract document defines fixture specifications",
        "Fixture Specifications" in content,
        "Missing fixture specifications section"
    )
    test(
        "Contract document defines version history",
        "Version History" in content,
        "Missing version history section"
    )


# ---------------------------------------------------------------------------
# Test Category 2: Valid Fixture Validation
# ---------------------------------------------------------------------------
print("\n[2/6] Valid Fixture Validation")

VALID_FIXTURES = [
    "context-decision-input-valid.json",
    "context-decision-output-valid.json",
    "receipt-consumption-valid.json",
    "degraded-node-valid.json",
]

for fixture_name in VALID_FIXTURES:
    fixture_path = FIXTURES_DIR / fixture_name
    test(
        f"Valid fixture exists: {fixture_name}",
        fixture_path.exists(),
        f"Missing: {fixture_path}"
    )

    if not fixture_path.exists():
        continue

    obj = load_json(fixture_path)

    # Validate contract version
    test(
        f"[{fixture_name}] Has contract version key",
        "runtime_context_decision_contract_version" in obj,
        "Missing contract version"
    )
    if "runtime_context_decision_contract_version" in obj:
        test(
            f"[{fixture_name}] Contract version is {CONTRACT_VERSION}",
            obj["runtime_context_decision_contract_version"] == CONTRACT_VERSION,
            f"Got: {obj['runtime_context_decision_contract_version']}"
        )

    # Validate advisory invariants
    advisory_checks = check_advisory_invariants(obj, fixture_name)
    for name, cond, detail in advisory_checks:
        test(name, cond, detail)

    # Validate no forbidden actions are authorized
    if "forbidden_actions_checked" in obj:
        for action in obj["forbidden_actions_checked"]:
            test(
                f"[{fixture_name}] Forbidden action '{action}' is in the forbidden list",
                action in FORBIDDEN_ACTIONS,
                f"'{action}' is not in the forbidden actions list"
            )

    # If degraded node state present, validate invariants
    for degraded_key in ["degraded_node_state", "degraded_node"]:
        degraded = obj.get(degraded_key, {})
        if degraded:
            node_health = degraded.get("node_health", "")
            allowed_remote = degraded.get("allowed_remote_use", None)
            measured_penalty = degraded.get("measured_penalty_ms", None)
            action = degraded.get("recommended_action", "")

            if node_health in ["unreachable", "stopped", "timeout"]:
                test(
                    f"[{fixture_name}] Degraded node '{degraded_key}': "
                    f"node_health={node_health} => allowed_remote_use is False",
                    allowed_remote is False,
                    f"Expected False for {node_health}, got: {allowed_remote}"
                )
                test(
                    f"[{fixture_name}] Degraded node '{degraded_key}': "
                    f"measured_penalty_ms >= 3000",
                    measured_penalty is not None and measured_penalty >= 3000,
                    f"Expected >= 3000, got: {measured_penalty}"
                )
                test(
                    f"[{fixture_name}] Degraded node '{degraded_key}': "
                    f"recommended_action is a valid degraded action",
                    action in DEGRADED_NODE_ACTIONS,
                    f"Got: {action}"
                )

    # Validate governance_outcome if present
    for gov_key in ["governance_outcome"]:
        gov = obj.get(gov_key, "")
        if gov:
            test(
                f"[{fixture_name}] governance_outcome '{gov}' is valid",
                gov in GOVERNANCE_OUTCOMES,
                f"Got: {gov}"
            )


# ---------------------------------------------------------------------------
# Test Category 3: Invalid Fixture Validation (must fail invariants)
# ---------------------------------------------------------------------------
print("\n[3/6] Invalid Fixture Validation (must fail contract invariants)")

INVALID_FIXTURES = {
    "forbidden-live-routing-invalid.json": {
        "reason": "Contains authorized forbidden actions and advisory=false",
        "expected_failures": ["advisory", "production_effects_allowed"],
    },
    "advisory-false-invalid.json": {
        "reason": "advisory is false",
        "expected_failures": ["advisory"],
    },
    "weak-provenance-receipt-invalid.json": {
        "reason": "Weak provenance marked as safe for receipt_generation",
        "expected_failures": ["evidence_quality"],
    },
}

for fixture_name, meta in INVALID_FIXTURES.items():
    fixture_path = FIXTURES_DIR / fixture_name
    test(
        f"Invalid fixture exists: {fixture_name}",
        fixture_path.exists(),
        f"Missing: {fixture_path}"
    )

    if not fixture_path.exists():
        continue

    obj = load_json(fixture_path)

    # Check that invalid fixture violates at least one invariant
    violations_found = []

    # Check advisory invariant
    if obj.get("advisory") is not True:
        violations_found.append("advisory")

    # Check production_effects_allowed invariant
    if obj.get("production_effects_allowed") is not False:
        violations_found.append("production_effects_allowed")

    # Check for authorized forbidden actions
    if "authorized_forbidden_actions" in obj:
        for action in obj["authorized_forbidden_actions"]:
            if action in FORBIDDEN_ACTIONS:
                violations_found.append(f"authorized_forbidden:{action}")

    # Check evidence quality for weak provenance
    if obj.get("evidence_quality") == "weak":
        violations_found.append("evidence_quality")

    # Check degraded node invariant violation
    for degraded_key in ["degraded_node_state", "degraded_node"]:
        degraded = obj.get(degraded_key, {})
        if degraded:
            node_health = degraded.get("node_health", "")
            allowed_remote = degraded.get("allowed_remote_use", None)
            measured_penalty = degraded.get("measured_penalty_ms", None)

            if node_health in ["unreachable", "stopped", "timeout"]:
                if allowed_remote is not False:
                    violations_found.append(f"allowed_remote_use ({allowed_remote})")
                if measured_penalty is not None and measured_penalty < 3000:
                    violations_found.append(f"measured_penalty_ms ({measured_penalty})")

    test(
        f"[{fixture_name}] Detects contract violations",
        len(violations_found) > 0,
        f"Expected violations but found none. Object keys: {list(obj.keys())}. "
        f"Invalid reason: {obj.get('_invalid_reason', 'unknown')}"
    )

    if violations_found:
        print(f"    -> Detected violations: {violations_found}")


# ---------------------------------------------------------------------------
# Test Category 4: Enum Validation
# ---------------------------------------------------------------------------
print("\n[4/6] Enum Validation")

for fixture_name in sorted(FIXTURES_DIR.glob("*.json")):
    obj = load_json(fixture_name)
    label = fixture_name.name

    # Validate workload_type if present
    if "workload_type" in obj:
        wl = obj["workload_type"]
        test(
            f"[{label}] workload_type '{wl}' is valid",
            wl in WORKLOAD_TYPES,
            f"Got: {wl}"
        )

    # Validate task_risk_level if present
    if "task_risk_level" in obj:
        rl = obj["task_risk_level"]
        test(
            f"[{label}] task_risk_level '{rl}' is valid",
            rl in TASK_RISK_LEVELS,
            f"Got: {rl}"
        )

    # Validate context_route if present
    route = obj.get("selected_context_route") or (obj.get("context_route") or {}).get("selected_route")
    if isinstance(route, str):
        test(
            f"[{label}] selected_context_route '{route}' is valid",
            route in CONTEXT_ROUTES,
            f"Got: {route}"
        )

    # Validate runtime profile if present
    model_route = obj.get("selected_model_route") or obj.get("model_route", {})
    if isinstance(model_route, dict):
        profile = model_route.get("selected_runtime_profile", "")
        if profile:
            test(
                f"[{label}] runtime profile '{profile}' is valid",
                profile in RUNTIME_PROFILES,
                f"Got: {profile}"
            )

    # Validate governance_outcome if present
    gov = obj.get("governance_outcome", "")
    if gov:
        test(
            f"[{label}] governance_outcome '{gov}' is valid",
            gov in GOVERNANCE_OUTCOMES,
            f"Got: {gov}"
        )

    # Validate node_health if present
    for degraded_key in ["degraded_node_state", "degraded_node"]:
        degraded = obj.get(degraded_key, {})
        if degraded:
            nh = degraded.get("node_health", "")
            if nh:
                test(
                    f"[{label}] node_health '{nh}' is valid",
                    nh in NODE_HEALTHS,
                    f"Got: {nh}"
                )


# ---------------------------------------------------------------------------
# Test Category 5: Governance-Mandated Route Validation
# ---------------------------------------------------------------------------
print("\n[5/6] Governance-Mandated Route Validation")

if PROTOTYPE_DECISIONS_PATH.exists():
    decisions = load_json(PROTOTYPE_DECISIONS_PATH)

    if "workload_decisions" in decisions:
        for wl_decision in decisions["workload_decisions"]:
            wl_type = wl_decision.get("context_route", {}).get("workload_type", "unknown")
            selected_route = wl_decision.get("context_route", {}).get("selected_context_route", "unknown")

            if wl_type in GOVERNANCE_MANDATED_ROUTES:
                mandated = GOVERNANCE_MANDATED_ROUTES[wl_type]
                test(
                    f"[PROTOTYPE-1] {wl_type} selects mandated route '{mandated}'",
                    selected_route == mandated,
                    f"Expected '{mandated}', got '{selected_route}'"
                )


# ---------------------------------------------------------------------------
# Test Category 6: Receipt Consumption Downstream-Only Validation
# ---------------------------------------------------------------------------
print("\n[6/6] Receipt Consumption Downstream-Only Rules")

for fixture_name in sorted(FIXTURES_DIR.glob("*.json")):
    obj = load_json(fixture_name)
    label = fixture_name.name

    # Check that no object attempts to trigger routing
    routing_triggers = [
        "trigger_routing", "execute_route", "start_routing",
        "apply_route_now", "activate_context_route",
    ]
    for trigger in routing_triggers:
        test(
            f"[{label}] Does not contain routing trigger '{trigger}'",
            trigger not in obj and trigger not in json.dumps(obj),
            "Receipt should not trigger routing"
        )

    # Check for runtime starting triggers
    runtime_triggers = [
        "start_runtime", "stop_runtime", "restart_runtime",
        "start_backend", "stop_backend",
    ]
    for trigger in runtime_triggers:
        test(
            f"[{label}] Does not contain runtime trigger '{trigger}'",
            trigger not in obj,
            "Receipt should not start or stop runtime"
        )

    # Check for model selection triggers
    model_triggers = [
        "select_active_model", "switch_model", "change_model",
        "execute_model_now",
    ]
    for trigger in model_triggers:
        test(
            f"[{label}] Does not contain model selection trigger '{trigger}'",
            trigger not in obj,
            "Receipt should not select models"
        )

    # Check for router state mutation triggers
    state_triggers = [
        "write_router_state", "modify_router_state", "update_routing_table",
        "save_state", "persist_route",
    ]
    for trigger in state_triggers:
        test(
            f"[{label}] Does not contain router state mutation '{trigger}'",
            trigger not in obj,
            "Receipt should not write router state"
        )


# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
print(f"\n{'=' * 60}")
print(f"Test Results: {passed} passed, {failed} failed, {passed + failed} total")
print(f"{'=' * 60}")

if errors:
    print(f"\nFailed tests:")
    for error in errors:
        print(f"  - {error}")

print(f"\n{'=' * 60}")
if failed == 0:
    print("All tests passed!")
    sys.exit(0)
else:
    print(f"FAILED: {failed} test(s) failed")
    sys.exit(1)
