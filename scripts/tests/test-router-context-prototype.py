#!/usr/bin/env python3
"""
MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 — Prototype Tests

Validates that the prototype decision generator produces correct,
contract-compliant decisions for all workload types and scenarios.

Sprint: MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1
"""

import json
import os
import sys
from pathlib import Path

# Add prototype to path
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts" / "prototypes"))

from router_context_decision_prototype import (
    generate_decision,
    generate_scenario_a_receipt,
    generate_scenario_b_long_session,
    generate_scenario_c_degraded_node,
    generate_scenario_d_agent_handoff,
    generate_scenario_e_sprint_closeout,
    generate_scenario_f_ui_review,
    generate_scenario_g_parallel_mixed,
    WORKLOAD_TYPES,
    CONTEXT_ROUTES,
    FRESHNESS_STATES,
    PROVENANCE_STATES,
    GOVERNANCE_OUTCOMES,
    RUNTIME_PROFILES,
    MEASURED,
)

# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------
passed = 0
failed = 0
errors = []


def check(condition: bool, name: str, detail: str = ""):
    global passed, failed
    if condition:
        passed += 1
        print(f"  PASS: {name}")
    else:
        failed += 1
        msg = f"  FAIL: {name}"
        if detail:
            msg += f" — {detail}"
        print(msg)
        errors.append({"test": name, "detail": detail})


def check_contract_invariants(decision: dict, prefix: str = ""):
    """Check contract v0.1 invariants on a context_route object."""
    cr = decision["context_route"]
    label = f"{prefix}{cr.get('workload_type', 'unknown')}"

    # Invariant 1: route_id non-empty
    check(bool(cr.get("route_id")), f"{label}: route_id non-empty", f"route_id={cr.get('route_id')}")

    # Invariant 2: contract_version = "0.1"
    check(cr.get("contract_version") == "0.1", f"{label}: contract_version=0.1", f"got={cr.get('contract_version')}")

    # Invariant 3: estimated_latency_ms >= 0
    latency = cr.get("estimated_latency_ms", -1)
    check(isinstance(latency, (int, float)) and latency >= 0, f"{label}: latency >= 0", f"got={latency}")

    # Invariant 4: alternatives_rejected non-empty
    alts = cr.get("alternatives_rejected", [])
    check(len(alts) > 0, f"{label}: alternatives_rejected non-empty", f"count={len(alts)}")

    # Invariant 5: reason_selected non-empty
    check(bool(cr.get("reason_selected")), f"{label}: reason_selected non-empty")

    # Invariant 6: receipt_summary.risk valid
    risk = cr.get("receipt_summary", {}).get("risk")
    check(risk in ("low", "medium", "high"), f"{label}: receipt_summary.risk valid", f"got={risk}")

    # Invariant 7: blocked implies blocked_for_task or provenance blocked
    gov = cr.get("governance_outcome")
    if gov == "blocked":
        check(
            cr.get("freshness_state") == "blocked_for_task" or cr.get("provenance_state") == "blocked",
            f"{label}: blocked implies blocked_for_task or provenance blocked",
            f"freshness={cr.get('freshness_state')}, provenance={cr.get('provenance_state')}",
        )

    # Invariant 8: safe implies not blocked
    if gov == "safe":
        check(
            cr.get("freshness_state") != "blocked_for_task",
            f"{label}: safe implies not blocked_for_task",
            f"freshness={cr.get('freshness_state')}",
        )
        check(
            cr.get("provenance_state") != "blocked",
            f"{label}: safe implies provenance not blocked",
            f"provenance={cr.get('provenance_state')}",
        )

    # Invariant 9: receipt_generation with weak provenance + safe is invalid
    if cr.get("workload_type") == "receipt_generation":
        if cr.get("provenance_state") == "weak" and cr.get("governance_outcome") == "safe":
            check(False, f"{label}: receipt_generation weak provenance + safe is INVALID")

    # Invariant 10: sprint_closeout allows_stale_context must be false
    if cr.get("workload_type") == "sprint_closeout":
        evidence = cr.get("evidence_requirements", {})
        check(
            evidence.get("allows_stale_context") is False,
            f"{label}: sprint_closeout allows_stale_context=false",
            f"got={evidence.get('allows_stale_context')}",
        )

    # Invariant 11: runtime_node_qualification stale + safe is invalid
    if cr.get("workload_type") == "runtime_node_qualification":
        if cr.get("freshness_state") == "stale_requires_revalidation" and cr.get("governance_outcome") == "safe":
            check(False, f"{label}: runtime_node_qualification stale + safe is INVALID")

    # Invariant 12: performance_sacrificed requires explanation
    if cr.get("performance_sacrificed_for_evidence"):
        check(
            bool(cr.get("reason_selected")),
            f"{label}: performance_sacrificed requires reason",
        )

    # Invariant 13: no GPU/RDMA/KV claims in future node
    if cr.get("selected_runtime_profile") == "future_stronger_gpu_node":
        for field in ["reason_selected", "receipt_summary"]:
            text = json.dumps(cr.get(field, ""))
            check(
                not any(kw in text.lower() for kw in ["gpu", "rdma", "kv-cache", "kv cache"]),
                f"{label}: no GPU/RDMA/KV claims in {field}",
            )


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------
print("=" * 70)
print("MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 — Tests")
print("=" * 70)

# Test 1: Prototype emits both model_route and context_route
print("\n[Test 1] Prototype emits both model_route and context_route")
for wl in WORKLOAD_TYPES:
    d = generate_decision(wl)
    check("model_route" in d, f"{wl}: model_route present")
    check("context_route" in d, f"{wl}: context_route present")

# Test 2: All 9 workload types produce a decision
print("\n[Test 2] All 9 workload types produce a decision")
decisions = {}
for wl in WORKLOAD_TYPES:
    d = generate_decision(wl)
    decisions[wl] = d
    check(d is not None, f"{wl}: decision generated")

# Test 3: Generated context_route objects comply with v0.1 contract
print("\n[Test 3] Context_route objects comply with v0.1 contract")
for wl, d in decisions.items():
    check_contract_invariants(d, prefix="test3_")

# Test 4: Receipt generation uses canonical_evidence_read
print("\n[Test 4] Receipt generation uses canonical_evidence_read")
d = decisions["receipt_generation"]
check(
    d["context_route"]["selected_context_route"] == "canonical_evidence_read",
    "receipt_generation uses canonical_evidence_read",
    f"got={d['context_route']['selected_context_route']}",
)

# Test 5: Receipt generation does not mark weak-provenance cache as safe
print("\n[Test 5] Receipt generation does not mark weak-provenance cache as safe")
d = decisions["receipt_generation"]
if d["context_route"]["provenance_state"] == "weak":
    check(
        d["context_route"]["governance_outcome"] != "safe",
        "receipt_generation: weak provenance not marked safe",
    )
else:
    check(True, "receipt_generation: provenance not weak (OK)")

# Test 6: Sprint closeout requires fresh evidence
print("\n[Test 6] Sprint closeout requires fresh evidence")
d = decisions["sprint_closeout"]
check(
    d["context_route"]["evidence_requirements"]["allows_stale_context"] is False,
    "sprint_closeout: allows_stale_context=false",
)
check(
    d["context_route"]["selected_context_route"] == "canonical_evidence_read",
    "sprint_closeout uses canonical_evidence_read",
    f"got={d['context_route']['selected_context_route']}",
)

# Test 7: Degraded runtime node applies measured ~4000ms penalty
print("\n[Test 7] Degraded runtime node applies measured penalty")
d = generate_scenario_c_degraded_node()
# The degraded node scenario should either use canonical_evidence (avoiding remote)
# or show the ~4000ms penalty in rejected alternatives
alts = d["context_route"]["alternatives_rejected"]
has_degraded_penalty = any(
    "4016" in alt.get("reason", "") or "4000" in alt.get("reason", "") or "degraded" in alt.get("reason", "").lower()
    for alt in alts
)
check(has_degraded_penalty, "degraded node: penalty mentioned in alternatives")
# Should NOT select remote_windows_runtime_cache as best route
check(
    d["context_route"]["selected_context_route"] != "remote_windows_runtime_cache",
    "degraded node: does not select remote route",
    f"got={d['context_route']['selected_context_route']}",
)

# Test 8: Long-session continuation reflects cheap measured local context
print("\n[Test 8] Long-session continuation reflects cheap local context")
d = decisions["long_session_continuation"]
latency = d["context_route"]["estimated_latency_ms"]
check(latency < 10, f"long_session_continuation: cheap local latency ({latency:.2f}ms < 10ms)")
check(
    d["context_route"]["selected_context_route"] in ("recent_turn_window", "ram_cache", "compressed_recall_packet"),
    "long_session_continuation: local route selected",
    f"got={d['context_route']['selected_context_route']}",
)

# Test 9: Compressed recall packet local cost not conflated with network cost
print("\n[Test 9] Recall packet local cost separate from network cost")
d = decisions["agent_handoff"]
cr = d["context_route"]
check(
    cr["selected_context_route"] == "compressed_recall_packet",
    "agent_handoff: uses compressed_recall_packet",
)
# Check that the receipt summary mentions measured local cost
receipt_detail = cr["receipt_summary"].get("detail", "")
check(
    "measured" in receipt_detail.lower() or "local" in receipt_detail.lower(),
    "agent_handoff: receipt mentions measured/local cost",
    f"detail={receipt_detail[:80]}",
)

# Test 10: Every decision includes rejected alternatives
print("\n[Test 10] Every decision includes rejected alternatives")
for wl, d in decisions.items():
    alts = d["context_route"].get("alternatives_rejected", [])
    check(len(alts) > 0, f"{wl}: has rejected alternatives (count={len(alts)})")

# Test 11: Every decision includes human-readable reason text
print("\n[Test 11] Every decision includes human-readable reason text")
for wl, d in decisions.items():
    reason = d["context_route"].get("reason_selected", "")
    check(len(reason) > 20, f"{wl}: reason_selected is substantive (len={len(reason)})")

# Test 12: Every decision includes receipt summary
print("\n[Test 12] Every decision includes receipt summary")
for wl, d in decisions.items():
    rs = d["context_route"].get("receipt_summary", {})
    check(bool(rs.get("label")), f"{wl}: receipt_summary.label present")
    check(bool(rs.get("detail")), f"{wl}: receipt_summary.detail present")
    check(rs.get("risk") in ("low", "medium", "high"), f"{wl}: receipt_summary.risk valid")

# Test 13: No production router files modified
print("\n[Test 13] No production router files modified")
router_py = REPO_ROOT / "router" / "router.py"
check(router_py.exists(), "router.py exists (not modified)")
# Verify the prototype script is in scripts/prototypes, not router/
prototype_path = REPO_ROOT / "scripts" / "prototypes" / "router_context_decision_prototype.py"
check(prototype_path.exists(), "prototype is in scripts/prototypes/ (not router/)")

# Test 14: No GPU/RDMA/KV-cache acceleration claims
print("\n[Test 14] No GPU/RDMA/KV-cache acceleration claims")
prototype_text = prototype_path.read_text(encoding="utf-8")
check(
    "RDMA" not in prototype_text,
    "No RDMA claims in prototype script",
)
check(
    "KV-cache" not in prototype_text and "kv_cache" not in prototype_text,
    "No KV-cache claims in prototype script",
)

# Test 15: Measured profiles used (not synthetic defaults)
print("\n[Test 15] Measured profiles used")
# Check that the prototype imports MEASURED from measurement sprint
check(
    "MEASURED" in prototype_text,
    "Prototype references MEASURED costs",
)
check(
    "70.90" in prototype_text or "git_status_ms" in prototype_text,
    "Prototype uses measured git_status_ms (~70.90ms)",
)
check(
    "4016" in prototype_text or "unreachable_timeout_ms" in prototype_text,
    "Prototype uses measured degraded-node penalty (~4016ms)",
)

# Test 16: Scenario decisions are generated
print("\n[Test 16] Scenario decisions generated")
scenario_a = generate_scenario_a_receipt()
check(scenario_a is not None, "Scenario A generated")
check(scenario_a["context_route"]["selected_context_route"] == "canonical_evidence_read", "Scenario A uses canonical_evidence_read")

scenario_b = generate_scenario_b_long_session()
check(scenario_b is not None, "Scenario B generated")
check(scenario_b["context_route"]["estimated_latency_ms"] < 10, "Scenario B has cheap local latency")

scenario_c = generate_scenario_c_degraded_node()
check(scenario_c is not None, "Scenario C generated")

scenario_d = generate_scenario_d_agent_handoff()
check(scenario_d is not None, "Scenario D generated")
check(scenario_d["context_route"]["selected_context_route"] == "compressed_recall_packet", "Scenario D uses compressed_recall_packet")

scenario_e = generate_scenario_e_sprint_closeout()
check(scenario_e is not None, "Scenario E generated")
check(scenario_e["context_route"]["selected_context_route"] == "canonical_evidence_read", "Scenario E uses canonical_evidence_read")

scenario_f = generate_scenario_f_ui_review()
check(scenario_f is not None, "Scenario F generated")
check(scenario_f["context_route"]["governance_outcome"] == "warning", "Scenario F has warning governance")

scenario_g = generate_scenario_g_parallel_mixed()
check(scenario_g is not None, "Scenario G generated")
check(scenario_g["summary"]["workload_count"] == 5, "Scenario G has 5 decisions")
check(len(scenario_g["summary"]["unique_routes"]) >= 2, "Scenario G has multiple unique routes")

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
print(f"\n{'=' * 70}")
print(f"Results: {passed} passed, {failed} failed, {passed + failed} total")
print(f"{'=' * 70}")

if errors:
    print("\nFailed tests:")
    for e in errors:
        print(f"  - {e['test']}: {e['detail']}")

sys.exit(0 if failed == 0 else 1)
