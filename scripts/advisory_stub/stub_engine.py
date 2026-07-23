"""
Advisory stub engine — generates contract-valid context decision outputs.

Reads workload profiles and measured costs, produces model_route +
context_route + receipt_consumption objects. All outputs enforce:

    advisory == true
    production_effects_allowed == false
    forbidden_actions_checked includes all 10 forbidden actions

Offline/sidecar only. No router HTTP, no process control, no model execution.
"""

from __future__ import annotations

import json
import time
import uuid
from pathlib import Path
from typing import Any

from . import __contract_version__
from .workload_profiles import (
    ALL_CONTRACT_ENUMS,
    CONTEXT_ROUTES,
    DEFAULT_BACKEND_STATE,
    DEFAULT_NODE_HEALTH,
    DEFAULT_RUNTIME_PROFILE,
    DEGRADED_NODE_ACTIONS,
    EVIDENCE_REQUIREMENTS,
    FORBIDDEN_ACTIONS,
    GOVERNANCE_MANDATED_ROUTES,
    GOVERNANCE_OUTCOMES,
    MODEL_PREFERENCES,
    PROVENANCE_STATES,
    ROUTE_LATENCIES,
    TASK_CLASS_FOR_WORKLOAD,
    TASK_RISK_LEVELS,
    WORKLOAD_TYPES,
)

# Default risk level mapping (not all workloads need "high")
DEFAULT_RISK: dict[str, str] = {
    "receipt_generation": "high",
    "sprint_closeout": "high",
    "validation": "medium",
    "code_patch_preparation": "medium",
    "agent_handoff": "low",
    "sprint_planning": "low",
    "long_session_continuation": "low",
    "runtime_node_qualification": "high",
    "ui_review_or_design_planning": "low",
}


class AdvisoryStubError(Exception):
    """Base error for advisory stub violations."""


# ---------------------------------------------------------------------------
# Contract validation helpers
# ---------------------------------------------------------------------------

CONTRACT_VERSION = __contract_version__


def _check_advisory(obj: dict, label: str) -> list[str]:
    """Validate advisory-only invariants. Returns list of violations."""
    violations = []
    if obj.get("advisory") is not True:
        violations.append(f"{label}: advisory is not True (got {obj.get('advisory')})")
    if "production_effects_allowed" in obj:
        if obj["production_effects_allowed"] is not False:
            violations.append(
                f"{label}: production_effects_allowed is not False "
                f"(got {obj['production_effects_allowed']})"
            )
    if obj.get("runtime_context_decision_contract_version") != CONTRACT_VERSION:
        violations.append(
            f"{label}: contract version mismatch "
            f"(got {obj.get('runtime_context_decision_contract_version')}, "
            f"expected {CONTRACT_VERSION})"
        )
    return violations


def _check_forbidden_actions(obj: dict) -> list[str]:
    """Validate all forbidden actions are checked. Returns list of violations."""
    violations = []
    checked = obj.get("forbidden_actions_checked", [])
    missing = [a for a in FORBIDDEN_ACTIONS if a not in checked]
    extra = [a for a in checked if a not in FORBIDDEN_ACTIONS]
    if missing:
        violations.append(f"Missing forbidden actions checked: {missing}")
    if extra:
        violations.append(f"Extra unauthorized actions: {extra}")
    return violations


def _check_enum(value: str, enum_name: str) -> list[str]:
    """Validate a value is in the named enum. Returns list of violations."""
    if value not in ALL_CONTRACT_ENUMS.get(enum_name, []):
        return [f"'{value}' is not a valid {enum_name}"]
    return []


def _check_required_field(obj: dict, field: str, obj_label: str) -> list[str]:
    """Validate a required field is present and non-None."""
    if field not in obj or obj[field] is None:
        return [f"{obj_label}: missing required field '{field}'"]
    return []


# ---------------------------------------------------------------------------
# Degraded node state builder
# ---------------------------------------------------------------------------

def build_degraded_node_state(
    node_health: str = DEFAULT_NODE_HEALTH,
    measured_penalty_ms: float = 4016.0,
) -> dict:
    """
    Build a contract-valid degraded_node_state object.

    For Stopped/Manual service state, enforces:
      - allowed_remote_use == false
      - measured_penalty_ms >= 3000
      - recommended_action is a valid degraded_node_action
    """
    action_map: dict[str, str] = {
        "unreachable": "avoid_remote_route",
        "stopped": "avoid_remote_route",
        "timeout": "block_for_task",
        "degraded": "use_local_fallback",
        "not_checked": "require_recheck",
        "available": "mark_warning",
        "unknown": "require_recheck",
    }

    recommended = action_map.get(node_health, "require_recheck")
    allowed_remote = node_health in ["available", "not_checked"]

    state = {
        "node_health": node_health,
        "last_check_latency_ms": 4015.3,
        "timeout_ms": 5000,
        "measured_penalty_ms": measured_penalty_ms,
        "recommended_action": recommended,
        "allowed_remote_use": allowed_remote,
        "reason": (
            f"LibrarianRunTimeNode service is {node_health.capitalize()}."
            f" Measured TCP timeout: ~{measured_penalty_ms:.0f}ms."
        ),
    }

    # Apply degraded-node invariants
    if node_health in ["unreachable", "stopped", "timeout"]:
        state["allowed_remote_use"] = False
        state["measured_penalty_ms"] = max(measured_penalty_ms, 3000.0)
        if node_health == "timeout":
            state["recommended_action"] = "block_for_task"

    return state


# ---------------------------------------------------------------------------
# Model route advisor (advisory-only)
# ---------------------------------------------------------------------------

def advise_model_route(workload_type: str) -> dict:
    """
    Generate an advisory model route for the given workload type.

    Does not select, start, or execute any model — only advises.
    """
    task_class = TASK_CLASS_FOR_WORKLOAD.get(workload_type, "general_advisory")
    model_pref = MODEL_PREFERENCES.get(task_class, MODEL_PREFERENCES["general_advisory"])

    model_route = {
        "selected_runtime_profile": DEFAULT_RUNTIME_PROFILE,
        "selected_model_profile": model_pref["model_profile"],
        "backend_state": DEFAULT_BACKEND_STATE,
        "fit": "suitable",
        "estimated_runtime_cost_ms": 0,
        "reason_selected": (
            f"Selected {model_pref['model_profile']} "
            f"({model_pref['description']}) for "
            f"{workload_type.replace('_', ' ').title()} workload. "
            f"Task class: {task_class}. "
            f"Backend state: {DEFAULT_BACKEND_STATE} "
            f"(LibrarianRunTimeNode is {DEFAULT_NODE_HEALTH})."
        ),
        "limitations": model_pref["limitations"],
    }

    return model_route


# ---------------------------------------------------------------------------
# Context route advisor (advisory-only)
# ---------------------------------------------------------------------------

def advise_context_route(workload_type: str) -> dict:
    """
    Generate an advisory context route for the given workload type.

    Follows governance mandates from PROTOTYPE-1.
    Uses measured cost data from MEASURE-1.
    Does not apply or execute any route — only advises.
    """
    mandated = GOVERNANCE_MANDATED_ROUTES.get(workload_type)
    evidence = EVIDENCE_REQUIREMENTS.get(workload_type, {})
    performance_sacrificed = evidence.get("performance_sacrificed_for_evidence", False)

    if mandated:
        selected_route = mandated
        estimated_latency = ROUTE_LATENCIES.get(selected_route, 70.9)
        freshness_state = "verified_current"
        provenance_state = "verified"
        governance = "safe"

        reason = (
            f"Selected {selected_route} for "
            f"{workload_type.replace('_', ' ').title()} workload. "
            f"(Governance-mandated per PROTOTYPE-1.) "
            f"Measured latency: ~{estimated_latency:.2f}ms. "
            f"Freshness: {freshness_state}. "
            f"Provenance: {provenance_state}. Governance: {governance}."
        )
    else:
        selected_route = "recent_turn_window"
        estimated_latency = ROUTE_LATENCIES.get(selected_route, 0.30)
        freshness_state = "stale_low_risk"
        provenance_state = "verified"
        governance = "warning"

        reason = (
            f"Selected {selected_route} for "
            f"{workload_type.replace('_', ' ').title()} workload. "
            f"(No governance mandate — using continuation default.) "
            f"Measured latency: ~{estimated_latency:.2f}ms. "
            f"Freshness: {freshness_state}. "
            f"Provenance: {provenance_state}. Governance: {governance}."
        )

    context_route = {
        "selected_route": selected_route,
        "estimated_latency_ms": estimated_latency,
        "all_route_latencies": {**ROUTE_LATENCIES},
        "freshness_state": freshness_state,
        "provenance_state": provenance_state,
        "governance_outcome": governance,
        "performance_sacrificed_for_evidence": performance_sacrificed,
    }

    return context_route


# ---------------------------------------------------------------------------
# Decision summary builder
# ---------------------------------------------------------------------------

def build_decision_summary(
    workload_type: str,
    model_profile: str,
    context_route: str,
    estimated_latency: float,
    governance_outcome: str,
) -> dict:
    """Build a compact decision summary."""
    return {
        "workload_type": workload_type,
        "selected_model": model_profile,
        "selected_context_route": context_route,
        "estimated_total_latency_ms": estimated_latency,
        "governance_outcome": governance_outcome,
    }


# ---------------------------------------------------------------------------
# Receipt consumption builder (downstream-only)
# ---------------------------------------------------------------------------

def build_receipt_consumption(
    workload_type: str,
    model_route: dict,
    context_route: dict,
    degraded_state: dict,
    evidence: dict,
) -> dict:
    """
    Build a receipt_consumption object.

    Downstream-only — must not trigger routing, start runtime,
    modify runtime, select model, or write router state.
    """
    selected_context_route_name = context_route["selected_route"]
    estimated_latency = context_route["estimated_latency_ms"]
    governance_outcome = context_route["governance_outcome"]
    performance_flag = context_route["performance_sacrificed_for_evidence"]

    # Generate rejected alternatives from all non-selected routes
    rejected = []
    for route_name, lat in ROUTE_LATENCIES.items():
        if route_name == selected_context_route_name:
            continue
        # Build a meaningful rejection reason
        if route_name == "remote_windows_runtime_cache" and degraded_state.get("node_health") in ["stopped", "unreachable", "timeout"]:
            rejected.append({
                "route": route_name,
                "reason": (
                    f"Remote runtime node is {degraded_state.get('node_health')}. "
                    f"Measured degraded-node penalty: "
                    f"{ROUTE_LATENCIES.get('remote_windows_runtime_cache')}ms. "
                    f"Local route {selected_context_route_name} "
                    f"({estimated_latency}ms) is preferred."
                ),
            })
        elif performance_flag and route_name in ["ram_cache", "ssd_cache", "recent_turn_window"]:
            rejected.append({
                "route": route_name,
                "reason": (
                    f"Governance/evidence requirements favor "
                    f"{selected_context_route_name} "
                    f"for {workload_type}."
                ),
            })
        else:
            rejected.append({
                "route": route_name,
                "reason": (
                    f"Route {route_name} costs ~{lat}ms. "
                    f"Selected {selected_context_route_name} "
                    f"({estimated_latency}ms) as better fit."
                ),
            })

    # Determine evidence quality from context route freshness/provenance
    freshness = context_route.get("freshness_state", "recent_but_unverified")
    provenance = context_route.get("provenance_state", "partially_verified")

    if freshness == "verified_current" and provenance == "verified":
        evidence_quality = "verified_current"
    elif freshness in ["stale_low_risk", "recent_but_unverified"]:
        evidence_quality = "recent_but_unverified"
    else:
        evidence_quality = "provenance_weak"

    # Build the receipt text
    if performance_flag:
        perf_note = (
            f"Performance was sacrificed to satisfy "
            f"{workload_type} evidence requirements."
        )
    else:
        perf_note = (
            f"No performance sacrificed — preferred route "
            f"{selected_context_route_name} is also the fastest path."
        )

    receipt_text = (
        f"{workload_type.replace('_', ' ').title()} routed via "
        f"{selected_context_route_name} ({estimated_latency:.0f}ms). "
        f"{perf_note} "
        f"Node is {degraded_state.get('node_health', 'unknown')} "
        f"— remote routing avoided."
    )

    receipt = {
        "route_summary": (
            f"{selected_context_route_name} selected for "
            f"{workload_type.replace('_', ' ').title()}"
        ),
        "selected_model_route": {
            "profile": model_route["selected_model_profile"],
            "backend_state": model_route["backend_state"],
            "fit": model_route["fit"],
        },
        "selected_context_route": {
            "route": selected_context_route_name,
            "estimated_latency_ms": estimated_latency,
            "governance_outcome": governance_outcome,
        },
        "governance_outcome": governance_outcome,
        "evidence_quality": evidence_quality,
        "performance_sacrificed_for_evidence": performance_flag,
        "degraded_node_summary": {
            "node_health": degraded_state.get("node_health", DEFAULT_NODE_HEALTH),
            "recommended_action": degraded_state.get("recommended_action", "require_recheck"),
        },
        "rejected_alternatives": rejected,
        "receipt_text": receipt_text,
    }

    return receipt


# ---------------------------------------------------------------------------
# Full decision engine
# ---------------------------------------------------------------------------

def generate_decision(
    workload_type: str = "sprint_closeout",
    task_risk_level: str | None = None,
    request_id: str | None = None,
    validate: bool = True,
) -> dict:
    """
    Generate a complete, contract-valid ContextDecisionOutput.

    This is the main entry point for the advisory stub engine. It:
      1. Determines the model route (advisory-only)
      2. Determines the context route (governance-mandated)
      3. Builds the degraded node state (from measured data)
      4. Builds the decision summary
      5. Builds the receipt consumption (downstream-only)
      6. Validates the full output against the contract

    Args:
        workload_type: One of the WORKLOAD_TYPES enum values.
        task_risk_level: Optional override for risk level.
        request_id: Optional request ID (auto-generated if not provided).
        validate: If True, validates output against contract (default True).

    Returns:
        Contract-valid ContextDecisionOutput as a dict.

    Raises:
        AdvisoryStubError: If workload_type is invalid or validation fails.
    """
    if workload_type not in WORKLOAD_TYPES:
        raise AdvisoryStubError(
            f"Invalid workload_type '{workload_type}'. "
            f"Valid: {WORKLOAD_TYPES}"
        )

    # Auto-generate request_id if not provided
    if request_id is None:
        request_id = f"stub-{workload_type}-{uuid.uuid4().hex[:8]}"

    # Determine risk level
    risk = task_risk_level or DEFAULT_RISK.get(workload_type, "medium")

    # Build model route (advisory-only)
    model_route = advise_model_route(workload_type)

    # Build context route (governance-mandated)
    context_route = advise_context_route(workload_type)

    # Build degraded node state
    degraded_state = build_degraded_node_state(DEFAULT_NODE_HEALTH)

    # Build decision summary
    decision_summary = build_decision_summary(
        workload_type=workload_type,
        model_profile=model_route["selected_model_profile"],
        context_route=context_route["selected_route"],
        estimated_latency=context_route["estimated_latency_ms"],
        governance_outcome=context_route["governance_outcome"],
    )

    # Build receipt consumption
    evidence = EVIDENCE_REQUIREMENTS.get(workload_type, {})
    receipt_consumption = build_receipt_consumption(
        workload_type=workload_type,
        model_route=model_route,
        context_route=context_route,
        degraded_state=degraded_state,
        evidence=evidence,
    )

    # Assemble full output
    output: dict[str, Any] = {
        "runtime_context_decision_contract_version": CONTRACT_VERSION,
        "request_id": request_id,
        "advisory": True,
        "production_effects_allowed": False,
        "model_route": model_route,
        "context_route": context_route,
        "degraded_node_state": degraded_state,
        "decision_summary": decision_summary,
        "receipt_consumption": receipt_consumption,
        "forbidden_actions_checked": list(FORBIDDEN_ACTIONS),
    }

    if validate:
        violations = validate_output(output)
        if violations:
            raise AdvisoryStubError(
                f"Generated output failed contract validation: {violations}"
            )

    return output


# ---------------------------------------------------------------------------
# Output validation
# ---------------------------------------------------------------------------

def validate_output(output: dict) -> list[str]:
    """
    Validate a ContextDecisionOutput dict against the contract v0.1.

    Returns a list of violation strings (empty if valid).
    """
    violations = []

    # Advisory invariants
    violations.extend(_check_advisory(output, "output"))
    violations.extend(_check_forbidden_actions(output))

    # Required top-level fields
    required_fields = [
        "runtime_context_decision_contract_version",
        "request_id", "advisory", "production_effects_allowed",
        "model_route", "context_route", "degraded_node_state",
        "decision_summary", "receipt_consumption",
        "forbidden_actions_checked",
    ]
    for field in required_fields:
        violations.extend(_check_required_field(output, field, "output"))

    # Validate model_route fields
    mr = output.get("model_route", {})
    for field in ["selected_runtime_profile", "selected_model_profile",
                  "backend_state", "fit", "estimated_runtime_cost_ms",
                  "reason_selected", "limitations"]:
        violations.extend(_check_required_field(mr, field, "model_route"))
    if mr.get("selected_runtime_profile") and mr["selected_runtime_profile"] in ["future_stronger_gpu_node"]:
        violations.append(
            "model_route: future_stronger_gpu_node must not include "
            "GPU/RDMA/KV-cache acceleration claims"
        )

    # Validate context_route fields
    cr = output.get("context_route", {})
    for field in ["selected_route", "estimated_latency_ms",
                  "all_route_latencies", "freshness_state",
                  "provenance_state", "governance_outcome",
                  "performance_sacrificed_for_evidence"]:
        violations.extend(_check_required_field(cr, field, "context_route"))
    violations.extend(_check_enum(cr.get("selected_route", ""), "selected_context_route"))
    violations.extend(_check_enum(cr.get("governance_outcome", ""), "governance_outcome"))
    violations.extend(_check_enum(cr.get("freshness_state", ""), "freshness_state"))
    violations.extend(_check_enum(cr.get("provenance_state", ""), "provenance_state"))

    # Validate degraded_node_state fields
    dns = output.get("degraded_node_state", {})
    for field in ["node_health", "last_check_latency_ms", "timeout_ms",
                  "measured_penalty_ms", "recommended_action",
                  "allowed_remote_use", "reason"]:
        violations.extend(_check_required_field(dns, field, "degraded_node_state"))
    violations.extend(_check_enum(dns.get("node_health", ""), "node_health"))
    violations.extend(_check_enum(dns.get("recommended_action", ""), "degraded_node_action"))

    # Apply degraded-node invariants
    nh = dns.get("node_health", "")
    if nh in ["unreachable", "stopped", "timeout"]:
        if dns.get("allowed_remote_use") is not False:
            violations.append(
                f"degraded_node_state: node_health={nh} requires "
                f"allowed_remote_use=false"
            )
        if dns.get("measured_penalty_ms", 0) < 3000:
            violations.append(
                f"degraded_node_state: node_health={nh} requires "
                f"measured_penalty_ms>=3000 "
                f"(got {dns.get('measured_penalty_ms')})"
            )
        if dns.get("recommended_action") not in DEGRADED_NODE_ACTIONS:
            violations.append(
                f"degraded_node_state: invalid recommended_action "
                f"for {nh}"
            )

    # Validate decision_summary fields
    ds = output.get("decision_summary", {})
    for field in ["workload_type", "selected_model",
                  "selected_context_route", "estimated_total_latency_ms",
                  "governance_outcome"]:
        violations.extend(_check_required_field(ds, field, "decision_summary"))

    # Validate receipt_consumption fields
    rc = output.get("receipt_consumption", {})
    for field in ["route_summary", "selected_model_route",
                  "selected_context_route", "governance_outcome",
                  "evidence_quality", "performance_sacrificed_for_evidence",
                  "rejected_alternatives", "receipt_text"]:
        violations.extend(_check_required_field(rc, field, "receipt_consumption"))

    # Check downstream-only rules (no routing triggers)
    downstream_safe_terms = [
        "route_summary", "selected_model_route", "selected_context_route",
        "governance_outcome", "evidence_quality",
        "performance_sacrificed_for_evidence", "degraded_node_summary",
        "rejected_alternatives", "receipt_text",
    ]
    unsafe_terms = [
        "trigger_routing", "start_runtime", "stop_runtime",
        "select_active_model", "write_router_state",
        "apply_context_route", "execute_model",
    ]
    rc_str = json.dumps(rc)
    for term in unsafe_terms:
        if term.lower() in rc_str.lower():
            violations.append(
                f"receipt_consumption: contains unsafe term '{term}'"
            )

    return violations


# ---------------------------------------------------------------------------
# Batch generation (all workload types)
# ---------------------------------------------------------------------------

def generate_all_decisions() -> dict[str, dict]:
    """Generate contract-valid decisions for all 9 workload types."""
    results = {}
    for wl in WORKLOAD_TYPES:
        try:
            results[wl] = generate_decision(wl, validate=True)
        except AdvisoryStubError as e:
            results[wl] = {"error": str(e)}
    return results


# ---------------------------------------------------------------------------
# Save decision to file
# ---------------------------------------------------------------------------

def save_decision(output: dict, path: str | Path) -> Path:
    """Save a decision output to a JSON file."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)
    return path
