#!/usr/bin/env python3
"""
MAC/WIN-ROUTER-CONTEXT-RUNTIME-ADVISORY-STUB-1 — Advisory Stub Tests

Validates that the offline advisory stub engine:
  - Generates contract-valid decision outputs for all 9 workload types
  - Enforces advisory-only invariants (advisory==true, production_effects_allowed==false)
  - Includes all 10 forbidden actions checked
  - Produces downstream-only receipt consumption (no routing/runtime triggers)
  - Uses governance-mandated routes correctly
  - Validates its own output successfully
  - Rejects invalid inputs

No production router behavior changed. No runtime HTTP. No model execution.
"""

import json
import os
import sys
from pathlib import Path

# Add repo root and scripts path to import the advisory stub
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from advisory_stub.stub_engine import (
    AdvisoryStubError,
    build_degraded_node_state,
    generate_all_decisions,
    generate_decision,
    validate_output,
)
from advisory_stub.workload_profiles import (
    FORBIDDEN_ACTIONS,
    GOVERNANCE_MANDATED_ROUTES,
    WORKLOAD_TYPES,
)

# ---------------------------------------------------------------------------
# Path setup
# ---------------------------------------------------------------------------
CONTRACT_DOC_PATH = REPO_ROOT / "docs" / "contracts" / "router-context-runtime-contract.md"
CONTRACT_FIXTURES_DIR = REPO_ROOT / "fixtures" / "router-context-runtime-contract"

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
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Test Category 1: Stub Engine — All Workload Types
# ---------------------------------------------------------------------------
print("\n[1/7] Stub Engine — All Workload Types Generate Successfully")

all_decisions = generate_all_decisions()

for wl in WORKLOAD_TYPES:
    decision = all_decisions.get(wl, {})
    is_valid = "error" not in decision
    test(
        f"[stub] generate_decision('{wl}') succeeds",
        is_valid,
        decision.get("error", "unknown error"),
    )

    if not is_valid:
        continue

    # Check advisory invariants
    test(
        f"[stub] {wl}: advisory is True",
        decision.get("advisory") is True,
        f"Got: {decision.get('advisory')}",
    )
    test(
        f"[stub] {wl}: production_effects_allowed is False",
        decision.get("production_effects_allowed") is False,
        f"Got: {decision.get('production_effects_allowed')}",
    )
    test(
        f"[stub] {wl}: contract version is 0.1",
        decision.get("runtime_context_decision_contract_version") == "0.1",
        f"Got: {decision.get('runtime_context_decision_contract_version')}",
    )

    # Check forbidden actions
    checked = decision.get("forbidden_actions_checked", [])
    test(
        f"[stub] {wl}: all {len(FORBIDDEN_ACTIONS)} forbidden actions checked",
        set(checked) == set(FORBIDDEN_ACTIONS),
        f"Missing: {set(FORBIDDEN_ACTIONS) - set(checked)}",
    )

    # Check required top-level fields
    for field in ["request_id", "model_route", "context_route",
                  "degraded_node_state", "decision_summary", "receipt_consumption"]:
        test(
            f"[stub] {wl}: has '{field}' field",
            field in decision,
            f"Missing required field",
        )


# ---------------------------------------------------------------------------
# Test Category 2: Model Route Validation
# ---------------------------------------------------------------------------
print("\n[2/7] Model Route Validation")

for wl in WORKLOAD_TYPES:
    decision = all_decisions.get(wl, {})
    if "error" in decision:
        continue

    mr = decision.get("model_route", {})
    test(
        f"[model_route] {wl}: has selected_runtime_profile",
        "selected_runtime_profile" in mr,
        "Missing field",
    )
    test(
        f"[model_route] {wl}: has selected_model_profile",
        "selected_model_profile" in mr,
        "Missing field",
    )
    test(
        f"[model_route] {wl}: has backend_state",
        "backend_state" in mr and mr["backend_state"] == "unavailable",
        f"Got: {mr.get('backend_state')}",
    )
    test(
        f"[model_route] {wl}: has estimated_runtime_cost_ms",
        "estimated_runtime_cost_ms" in mr,
        "Missing field",
    )
    test(
        f"[model_route] {wl}: has reason_selected",
        "reason_selected" in mr and len(mr["reason_selected"]) > 20,
        "Missing or too short",
    )
    test(
        f"[model_route] {wl}: has limitations",
        "limitations" in mr,
        "Missing field",
    )


# ---------------------------------------------------------------------------
# Test Category 3: Context Route Validation
# ---------------------------------------------------------------------------
print("\n[3/7] Context Route Validation")

for wl in WORKLOAD_TYPES:
    decision = all_decisions.get(wl, {})
    if "error" in decision:
        continue

    cr = decision.get("context_route", {})
    test(
        f"[context_route] {wl}: has selected_route",
        "selected_route" in cr,
        "Missing field",
    )
    test(
        f"[context_route] {wl}: selected_route is valid",
        cr.get("selected_route") in ["ram_cache", "ssd_cache",
            "remote_windows_runtime_cache", "recomputation_from_source",
            "compressed_recall_packet", "canonical_evidence_read",
            "hybrid_recall_plus_fresh_evidence", "recent_turn_window"],
        f"Got: {cr.get('selected_route')}",
    )
    test(
        f"[context_route] {wl}: estimated_latency_ms > 0",
        cr.get("estimated_latency_ms", 0) > 0,
        f"Got: {cr.get('estimated_latency_ms')}",
    )
    test(
        f"[context_route] {wl}: all_route_latencies has entries",
        len(cr.get("all_route_latencies", {})) >= 5,
        f"Got {len(cr.get('all_route_latencies', {}))} entries",
    )
    test(
        f"[context_route] {wl}: has freshness_state",
        "freshness_state" in cr,
        "Missing field",
    )
    test(
        f"[context_route] {wl}: has provenance_state",
        "provenance_state" in cr,
        "Missing field",
    )
    test(
        f"[context_route] {wl}: has governance_outcome",
        "governance_outcome" in cr,
        "Missing field",
    )

    # Governance-mandated route check
    mandated = GOVERNANCE_MANDATED_ROUTES.get(wl)
    if mandated:
        test(
            f"[context_route] {wl}: follows governance mandate to use '{mandated}'",
            cr.get("selected_route") == mandated,
            f"Expected '{mandated}', got '{cr.get('selected_route')}'",
        )


# ---------------------------------------------------------------------------
# Test Category 4: Degraded Node State Validation
# ---------------------------------------------------------------------------
print("\n[4/7] Degraded Node State Validation")

for wl in WORKLOAD_TYPES:
    decision = all_decisions.get(wl, {})
    if "error" in decision:
        continue

    dns = decision.get("degraded_node_state", {})
    test(
        f"[degraded] {wl}: has node_health",
        "node_health" in dns,
        "Missing field",
    )
    test(
        f"[degraded] {wl}: node_health is 'stopped' (current service state)",
        dns.get("node_health") == "stopped",
        f"Got: {dns.get('node_health')}",
    )
    test(
        f"[degraded] {wl}: allowed_remote_use is False (stopped node)",
        dns.get("allowed_remote_use") is False,
        f"Got: {dns.get('allowed_remote_use')}",
    )
    test(
        f"[degraded] {wl}: measured_penalty_ms >= 3000",
        dns.get("measured_penalty_ms", 0) >= 3000,
        f"Got: {dns.get('measured_penalty_ms')}",
    )
    test(
        f"[degraded] {wl}: has recommended_action",
        "recommended_action" in dns,
        "Missing field",
    )
    test(
        f"[degraded] {wl}: has last_check_latency_ms",
        "last_check_latency_ms" in dns,
        "Missing field",
    )
    test(
        f"[degraded] {wl}: has reason",
        "reason" in dns and len(dns["reason"]) > 10,
        "Missing or too short",
    )

# Test the degraded node builder directly for edge cases
for health in ["unreachable", "stopped", "timeout"]:
    state = build_degraded_node_state(node_health=health)
    test(
        f"[degraded] build_degraded({health}): allowed_remote_use is False",
        state.get("allowed_remote_use") is False,
        f"Got: {state.get('allowed_remote_use')}",
    )
    test(
        f"[degraded] build_degraded({health}): measured_penalty_ms >= 3000",
        state.get("measured_penalty_ms", 0) >= 3000,
        f"Got: {state.get('measured_penalty_ms')}",
    )

for health in ["available", "not_checked"]:
    state = build_degraded_node_state(node_health=health)
    test(
        f"[degraded] build_degraded({health}): allowed_remote_use is True",
        state.get("allowed_remote_use") is True,
        f"Got: {state.get('allowed_remote_use')}",
    )


# ---------------------------------------------------------------------------
# Test Category 5: Receipt Consumption Validation
# ---------------------------------------------------------------------------
print("\n[5/7] Receipt Consumption — Downstream-Only Rules")

for wl in WORKLOAD_TYPES:
    decision = all_decisions.get(wl, {})
    if "error" in decision:
        continue

    rc = decision.get("receipt_consumption", {})
    test(
        f"[receipt] {wl}: has route_summary",
        "route_summary" in rc,
        "Missing field",
    )
    test(
        f"[receipt] {wl}: has selected_model_route",
        "selected_model_route" in rc,
        "Missing field",
    )
    test(
        f"[receipt] {wl}: has selected_context_route",
        "selected_context_route" in rc,
        "Missing field",
    )
    test(
        f"[receipt] {wl}: has governance_outcome",
        "governance_outcome" in rc,
        "Missing field",
    )
    test(
        f"[receipt] {wl}: has evidence_quality",
        "evidence_quality" in rc,
        "Missing field",
    )
    test(
        f"[receipt] {wl}: has performance_sacrificed_for_evidence",
        "performance_sacrificed_for_evidence" in rc,
        "Missing field",
    )
    test(
        f"[receipt] {wl}: has degraded_node_summary",
        "degraded_node_summary" in rc,
        "Missing field",
    )
    test(
        f"[receipt] {wl}: has rejected_alternatives",
        "rejected_alternatives" in rc,
        "Missing field",
    )
    test(
        f"[receipt] {wl}: has receipt_text",
        "receipt_text" in rc and len(rc["receipt_text"]) > 20,
        "Missing or too short",
    )

    # No routing triggers in receipt
    rc_str = json.dumps(rc)
    routing_triggers = [
        "trigger_routing", "start_runtime", "stop_runtime",
        "select_active_model", "write_router_state",
        "apply_context_route", "execute_model",
    ]
    for trigger in routing_triggers:
        test(
            f"[receipt] {wl}: no downstream violation '{trigger}'",
            trigger.lower() not in rc_str.lower(),
            f"Found unsafe term: {trigger}",
        )


# ---------------------------------------------------------------------------
# Test Category 6: Contract Validation Roundtrip
# ---------------------------------------------------------------------------
print("\n[6/7] Contract Validation Roundtrip")

# All generated decisions should pass validate_output()
for wl in WORKLOAD_TYPES:
    decision = all_decisions.get(wl, {})
    if "error" in decision:
        continue

    violations = validate_output(decision)
    test(
        f"[validation] {wl}: validate_output() returns no violations",
        len(violations) == 0,
        f"Violations: {violations}",
    )

# generate_decision with validate=True should not raise
for wl in WORKLOAD_TYPES:
    try:
        decision = generate_decision(wl, validate=True)
        test(
            f"[validation] generate_decision('{wl}', validate=True) succeeds",
            True,
            "",
        )
    except AdvisoryStubError as e:
        test(
            f"[validation] generate_decision('{wl}', validate=True) succeeds",
            False,
            str(e),
        )

# Invalid workload_type should raise
try:
    generate_decision("invalid_workload_type", validate=False)
    test(
        "[validation] Invalid workload_type is rejected",
        False,
        "Should have raised AdvisoryStubError",
    )
except AdvisoryStubError:
    test(
        "[validation] Invalid workload_type is rejected",
        True,
        "",
    )

# Manually corrupt a decision and verify validation catches it
base = generate_decision("sprint_closeout", validate=True)
corrupted = dict(base)
corrupted["advisory"] = False
violations = validate_output(corrupted)
test(
    "[validation] advisory=False is detected as violation",
    len(violations) > 0 and "advisory" in violations[0].lower(),
    f"Expected advisory violation, got: {violations}",
)

corrupted2 = dict(base)
corrupted2["forbidden_actions_checked"] = ["start_process"]
violations2 = validate_output(corrupted2)
test(
    "[validation] Missing forbidden actions is detected",
    len(violations2) > 0 and "forbidden" in violations2[0].lower(),
    f"Expected forbidden action violation, got: {violations2}",
)


# ---------------------------------------------------------------------------
# Test Category 7: Decision Summary Validation
# ---------------------------------------------------------------------------
print("\n[7/7] Decision Summary Validation")

for wl in WORKLOAD_TYPES:
    decision = all_decisions.get(wl, {})
    if "error" in decision:
        continue

    ds = decision.get("decision_summary", {})
    test(
        f"[summary] {wl}: has workload_type field",
        ds.get("workload_type") == wl,
        f"Expected '{wl}', got '{ds.get('workload_type')}'",
    )
    test(
        f"[summary] {wl}: has selected_model field",
        "selected_model" in ds,
        "Missing field",
    )
    test(
        f"[summary] {wl}: has selected_context_route field",
        "selected_context_route" in ds,
        "Missing field",
    )
    test(
        f"[summary] {wl}: has estimated_total_latency_ms field",
        "estimated_total_latency_ms" in ds,
        "Missing field",
    )
    test(
        f"[summary] {wl}: has governance_outcome field",
        "governance_outcome" in ds,
        "Missing field",
    )
    test(
        f"[summary] {wl}: summary matches context_route",
        ds.get("selected_context_route") == decision.get("context_route", {}).get("selected_route"),
        f"Summary says '{ds.get('selected_context_route')}', "
        f"context_route says '{decision.get('context_route', {}).get('selected_route')}'",
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
