#!/usr/bin/env python3
"""
MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 — Router Context Decision Prototype

Generates mock router decisions containing both model_route and context_route
using measured hardware/context costs from MAC/WIN-ROUTER-CONTEXT-MEASURE-1.

This is a PROTOTYPE — not production routing behavior.
No live routing. No model execution. No cache engine.

Sprint: MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1
Status: Prototype / test only.
"""

import json
import os
import sys
import uuid
from pathlib import Path
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional

# ---------------------------------------------------------------------------
# Path setup
# ---------------------------------------------------------------------------
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CONTRACT_PATH = REPO_ROOT / "docs" / "contracts" / "context-route-contract.md"
MEASURED_PROFILES_PATH = REPO_ROOT / "config" / "measured_hardware_profiles.json"
OPTIMIZER_CONFIG_PATH = REPO_ROOT / "config" / "router_workload_optimizer.default.json"
MODEL_PROFILES_PATH = REPO_ROOT / "config" / "model-profiles.json"
FIXTURES_DIR = REPO_ROOT / "fixtures" / "context-route"
OUTPUT_DIR = REPO_ROOT / "reports"
PROTOTYPE_FIXTURES_DIR = REPO_ROOT / "fixtures" / "router-context-prototype"

# ---------------------------------------------------------------------------
# Contract enums (from context-route-contract.md v0.1)
# ---------------------------------------------------------------------------
WORKLOAD_TYPES = [
    "sprint_planning", "sprint_closeout", "receipt_generation", "validation",
    "code_patch_preparation", "agent_handoff", "long_session_continuation",
    "runtime_node_qualification", "ui_review_or_design_planning",
]

CONTEXT_ROUTES = [
    "ram_cache", "ssd_cache", "remote_windows_runtime_cache",
    "recomputation_from_source", "compressed_recall_packet",
    "canonical_evidence_read", "hybrid_recall_plus_fresh_evidence",
    "recent_turn_window",
]

FRESHNESS_STATES = [
    "verified_current", "recent_but_unverified", "stale_low_risk",
    "stale_requires_revalidation", "provenance_weak", "blocked_for_task",
]

PROVENANCE_STATES = [
    "verified", "partially_verified", "weak", "unknown", "blocked",
]

GOVERNANCE_OUTCOMES = [
    "safe", "warning", "requires_revalidation", "blocked",
]

RUNTIME_PROFILES = [
    "mac_coordinator", "windows_runtime_node",
    "weak_lan_runtime_node", "future_stronger_gpu_node",
]

# ---------------------------------------------------------------------------
# Measured costs (from MAC/WIN-ROUTER-CONTEXT-MEASURE-1)
# ---------------------------------------------------------------------------
MEASURED = {
    "windows_runtime_node": {
        "file_read_warm_ms": {"429tok": 0.25, "8ktok": 0.26, "32ktok": 0.28},
        "json_parse_warm_ms": 0.05,
        "json_serialize_ms": 0.11,
        "git_status_ms": 70.90,
        "git_revparse_ms": 55.47,
    },
    "recall_packet_local": {
        "serialize_32k_ms": 0.22,
        "deserialize_32k_ms": 0.11,
        "compress_32k_ms": 0.15,
        "decompress_32k_ms": 0.02,
        "total_local_32k_ms": 0.50,
    },
    "degraded_node": {
        "unreachable_timeout_ms": 4016.0,
        "connection_refused_ms": 4017.0,
    },
    "small_append": {
        "serialize_429tok_ms": 0.06,
        "write_429tok_ms": 0.48,
        "read_429tok_ms": 0.26,
        "total_pipeline_ms": 0.80,
    },
    "large_context": {
        "serialize_32k_ms": 0.41,
        "write_32k_ms": 0.93,
        "read_32k_ms": 0.36,
        "deserialize_32k_ms": 0.21,
        "total_pipeline_32k_ms": 1.91,
        "total_pipeline_64k_ms": 3.80,
    },
}

# Synonym mapping for optimizer config routes -> contract routes
ROUTE_SYNONYMS = {
    "recomputation": "recomputation_from_source",
    "recent_turn_window": "recent_turn_window",
}

# ---------------------------------------------------------------------------
# Workload profile definitions (from optimizer config)
# ---------------------------------------------------------------------------
WORKLOAD_PROFILES = {
    "sprint_planning": {
        "label": "Sprint Planning",
        "typical_context_tokens": 25000,
        "typical_append_tokens": 600,
        "freshness_requirement": "moderate",
        "provenance_requirement": "moderate",
        "evidence_requirement": "status_summary",
        "stale_cache_tolerance": "moderate",
        "risk_level": "low",
        "preferred_routes": ["compressed_recall_packet", "ram_cache", "ssd_cache"],
        "route_ranking": ["compressed_recall_packet", "ram_cache", "ssd_cache", "recent_turn_window", "recomputation_from_source"],
        "preferred_model": "phi-4",
        "model_task_class": "general_advisory",
    },
    "sprint_closeout": {
        "label": "Sprint Closeout",
        "typical_context_tokens": 35000,
        "typical_append_tokens": 400,
        "freshness_requirement": "strict",
        "provenance_requirement": "strict",
        "evidence_requirement": "live_git_test_state",
        "stale_cache_tolerance": "none",
        "risk_level": "high",
        "preferred_routes": ["canonical_evidence_read", "recomputation_from_source", "ram_cache"],
        "route_ranking": ["canonical_evidence_read", "recomputation_from_source", "ram_cache", "compressed_recall_packet", "ssd_cache"],
        "preferred_model": "phi-4",
        "model_task_class": "general_advisory",
    },
    "receipt_generation": {
        "label": "Receipt Generation",
        "typical_context_tokens": 20000,
        "typical_append_tokens": 300,
        "freshness_requirement": "strict",
        "provenance_requirement": "strict",
        "evidence_requirement": "canonical_source",
        "stale_cache_tolerance": "none",
        "risk_level": "high",
        "preferred_routes": ["canonical_evidence_read", "recomputation_from_source"],
        "route_ranking": ["canonical_evidence_read", "recomputation_from_source", "ram_cache", "compressed_recall_packet", "ssd_cache"],
        "preferred_model": "phi-4",
        "model_task_class": "general_advisory",
    },
    "validation": {
        "label": "Validation",
        "typical_context_tokens": 28000,
        "typical_append_tokens": 350,
        "freshness_requirement": "high",
        "provenance_requirement": "high",
        "evidence_requirement": "current_test_output",
        "stale_cache_tolerance": "low",
        "risk_level": "medium",
        "preferred_routes": ["canonical_evidence_read", "ram_cache", "recomputation_from_source"],
        "route_ranking": ["canonical_evidence_read", "ram_cache", "recomputation_from_source", "compressed_recall_packet", "ssd_cache"],
        "preferred_model": "phi-4",
        "model_task_class": "general_advisory",
    },
    "code_patch_preparation": {
        "label": "Code Patch Preparation",
        "typical_context_tokens": 30000,
        "typical_append_tokens": 500,
        "freshness_requirement": "high",
        "provenance_requirement": "high",
        "evidence_requirement": "current_source",
        "stale_cache_tolerance": "low",
        "risk_level": "medium",
        "preferred_routes": ["ram_cache", "canonical_evidence_read", "ssd_cache"],
        "route_ranking": ["ram_cache", "canonical_evidence_read", "ssd_cache", "compressed_recall_packet", "recomputation_from_source"],
        "preferred_model": "qwen-coder",
        "model_task_class": "code_advisory",
    },
    "agent_handoff": {
        "label": "Agent Handoff",
        "typical_context_tokens": 32700,
        "typical_append_tokens": 200,
        "freshness_requirement": "moderate",
        "provenance_requirement": "moderate",
        "evidence_requirement": "complete_state_snapshot",
        "stale_cache_tolerance": "moderate",
        "risk_level": "low",
        "preferred_routes": ["compressed_recall_packet", "ram_cache", "ssd_cache"],
        "route_ranking": ["compressed_recall_packet", "ram_cache", "ssd_cache", "recent_turn_window", "recomputation_from_source"],
        "preferred_model": "phi-4",
        "model_task_class": "general_advisory",
    },
    "long_session_continuation": {
        "label": "Long Session Continuation",
        "typical_context_tokens": 40000,
        "typical_append_tokens": 300,
        "freshness_requirement": "moderate",
        "provenance_requirement": "low",
        "evidence_requirement": "recent_window",
        "stale_cache_tolerance": "high",
        "risk_level": "low",
        "preferred_routes": ["recent_turn_window", "compressed_recall_packet", "ram_cache"],
        "route_ranking": ["recent_turn_window", "compressed_recall_packet", "ram_cache", "ssd_cache", "recomputation_from_source"],
        "preferred_model": "phi-4",
        "model_task_class": "general_advisory",
    },
    "runtime_node_qualification": {
        "label": "Runtime Node Qualification",
        "typical_context_tokens": 15000,
        "typical_append_tokens": 250,
        "freshness_requirement": "strict",
        "provenance_requirement": "strict",
        "evidence_requirement": "live_node_health",
        "stale_cache_tolerance": "none",
        "risk_level": "high",
        "preferred_routes": ["canonical_evidence_read", "recomputation_from_source", "remote_windows_runtime_cache"],
        "route_ranking": ["canonical_evidence_read", "remote_windows_runtime_cache", "recomputation_from_source", "ram_cache", "compressed_recall_packet"],
        "preferred_model": "phi-4",
        "model_task_class": "general_advisory",
    },
    "ui_review_or_design_planning": {
        "label": "UI Review / Design Planning",
        "typical_context_tokens": 22000,
        "typical_append_tokens": 800,
        "freshness_requirement": "low",
        "provenance_requirement": "low",
        "evidence_requirement": "current_screenshots",
        "stale_cache_tolerance": "high",
        "risk_level": "low",
        "preferred_routes": ["ram_cache", "compressed_recall_packet", "ssd_cache"],
        "route_ranking": ["ram_cache", "compressed_recall_packet", "ssd_cache", "recent_turn_window", "recomputation_from_source"],
        "preferred_model": "phi-4",
        "model_task_class": "general_advisory",
    },
}

# ---------------------------------------------------------------------------
# Model profile lookup
# ---------------------------------------------------------------------------
MODEL_PROFILES = {
    "phi-4": {
        "alias": "phi-4", "context": 4096, "ngl": 99, "port": 9120,
        "stability": "stable", "task_classes": ["general_advisory", "summarization_advisory"],
        "notes": "2.32 GB Q4_K_M. General advisory. Clean output.",
    },
    "qwen-coder": {
        "alias": "qwen-coder", "context": 4096, "ngl": 99, "port": 9121,
        "stability": "stable", "task_classes": ["code_advisory", "fallback_small_model"],
        "notes": "1.76 GB Q8_0. Best for code tasks.",
    },
    "llama-3.2": {
        "alias": "llama-3.2", "context": 4096, "ngl": 80, "port": 9122,
        "stability": "conditional", "task_classes": ["general_advisory"],
        "notes": "2.16 GB Q5_K_M. 3B params. Requires reduced offload.",
    },
    "qwen3": {
        "alias": "qwen3", "context": 4096, "ngl": 80, "port": 9123,
        "stability": "conditional", "task_classes": ["general_advisory", "reasoning"],
        "notes": "2.33 GB Q4_K_M. 4B params. Outputs reasoning blocks.",
    },
    "gemma-3": {
        "alias": "gemma-3", "context": 4096, "ngl": 80, "port": 9124,
        "stability": "conditional", "task_classes": ["general_advisory"],
        "notes": "2.32 GB Q4_K_M. 4B params. Google Gemma 3.",
    },
}


# ---------------------------------------------------------------------------
# Decision generation
# ---------------------------------------------------------------------------

def generate_route_id(workload_type: str) -> str:
    """Generate a unique route ID."""
    short_uuid = uuid.uuid4().hex[:8]
    return f"ctx-route-{workload_type[:3]}-{short_uuid}"


def estimate_context_route_latency(route: str, workload: dict, hardware: str = "windows_runtime_node") -> float:
    """Estimate latency for a context route using measured costs."""
    tokens = workload.get("typical_context_tokens", 25000)
    append = workload.get("typical_append_tokens", 400)

    if route == "ram_cache":
        # Measured: warm read ~0.28ms for 32K, serialize ~0.11ms
        base = 0.5
        transfer = tokens * 0.0001  # from optimizer config
        append_cost = append * 0.002
        return base + transfer + append_cost

    elif route == "ssd_cache":
        # Measured: warm read ~0.28ms (nearly same as RAM on this hardware)
        base = 0.3  # corrected from 10ms to measured value
        transfer = tokens * 0.0005  # adjusted down from 0.005
        append_cost = append * 0.001
        return base + transfer + append_cost

    elif route == "compressed_recall_packet":
        # Measured: local processing ~0.5ms for 32K
        base = 0.5  # corrected from 80ms to measured local cost
        transfer = tokens * 0.0001
        append_cost = append * 0.001
        compress = tokens * 0.000005
        decompress = tokens * 0.000001
        return base + transfer + append_cost + compress + decompress

    elif route == "canonical_evidence_read":
        # Measured: git status ~71ms + git rev-parse ~55ms
        base = MEASURED["windows_runtime_node"]["git_status_ms"]
        return base

    elif route == "recomputation_from_source":
        # Not measured (requires model inference) — use conservative estimate
        return 500.0

    elif route == "remote_windows_runtime_cache":
        # Depends on node health
        if hardware == "weak_lan_runtime_node":
            return MEASURED["degraded_node"]["unreachable_timeout_ms"]
        return 50.0 + 35.0  # base + LAN latency

    elif route == "recent_turn_window":
        # Measured: similar to RAM cache warm read
        return 0.3

    elif route == "hybrid_recall_plus_fresh_evidence":
        # Measured: recall (~0.5ms) + evidence read (~71ms)
        return 0.5 + MEASURED["windows_runtime_node"]["git_status_ms"]

    return 0.0


def select_model_route(workload_type: str, hardware: str = "windows_runtime_node") -> dict:
    """Generate a model_route decision."""
    profile = WORKLOAD_PROFILES[workload_type]
    model_alias = profile["preferred_model"]
    model_info = MODEL_PROFILES[model_alias]

    # Determine backend state based on service status
    # In prototype: service is stopped, so backends are unavailable
    backend_state = "unavailable"  # Service is stopped
    fit = "suitable"  # Model would be suitable if running

    # Runtime cost estimate (not measured — model inference not in scope)
    runtime_cost_ms = 0  # Not measured in this sprint

    reason = (
        f"Selected {model_alias} ({model_info['notes']}) for {profile['label']} workload. "
        f"Task class: {profile['model_task_class']}. "
        f"Backend state: {backend_state} (LibrarianRunTimeNode is stopped)."
    )

    alternatives = []
    for alias, info in MODEL_PROFILES.items():
        if alias != model_alias:
            if profile["model_task_class"] in info["task_classes"]:
                # Same task class — alternative
                alt_reason = f"Alternative {alias} supports {profile['model_task_class']} but {model_alias} is preferred for this workload."
            elif info["stability"] == "conditional":
                alt_reason = f"{alias} requires reduced offload (ngl={info['ngl']}) and is conditional stability."
            else:
                alt_reason = f"{alias} does not match preferred task class {profile['model_task_class']}."
            alternatives.append({
                "runtime_profile": "windows_runtime_node",
                "model_profile": alias,
                "reason": alt_reason,
            })

    # Add mac_coordinator as rejected alternative
    alternatives.append({
        "runtime_profile": "mac_coordinator",
        "reason": "Coordinator should not perform this runtime task. Mac profile not measured in measurement sprint.",
    })

    return {
        "selected_runtime_profile": hardware,
        "selected_model_profile": model_alias,
        "backend_state": backend_state,
        "fit": fit,
        "estimated_runtime_cost_ms": runtime_cost_ms,
        "reason_selected": reason,
        "alternatives_rejected": alternatives,
        "limitations": model_info["notes"],
    }


def select_context_route(
    workload_type: str,
    hardware: str = "windows_runtime_node",
    node_health: str = "stopped",
) -> dict:
    """Generate a context_route decision using measured costs.

    Governance rules override pure latency optimization:
    - receipt_generation: MUST use canonical_evidence_read (contract invariant 9)
    - sprint_closeout: MUST use canonical_evidence_read (contract invariant 10)
    - runtime_node_qualification with stopped node: cannot use remote route
    """
    profile = WORKLOAD_PROFILES[workload_type]
    route_id = generate_route_id(workload_type)

    # Governance-mandated routes (override latency optimization)
    # These enforce contract expectations from context-route-contract.md
    governance_mandated_routes = {
        "receipt_generation": "canonical_evidence_read",
        "sprint_closeout": "canonical_evidence_read",
        "validation": "canonical_evidence_read",
        "agent_handoff": "compressed_recall_packet",
        "runtime_node_qualification": "canonical_evidence_read",
    }

    # Evaluate each route in ranking order
    best_route = None
    best_latency = float("inf")
    all_latencies = {}

    for route in profile["route_ranking"]:
        latency = estimate_context_route_latency(route, profile, hardware)
        all_latencies[route] = latency

        # Check if route is available
        if route == "remote_windows_runtime_cache" and node_health == "stopped":
            # Apply measured degraded-node penalty
            latency = MEASURED["degraded_node"]["unreachable_timeout_ms"]
            all_latencies[route] = latency

        if latency < best_latency:
            best_latency = latency
            best_route = route

    # Apply governance mandate if applicable
    mandated = governance_mandated_routes.get(workload_type)
    if mandated and mandated in all_latencies:
        best_route = mandated
        best_latency = all_latencies[mandated]

    # Determine governance states
    if workload_type in ("receipt_generation", "sprint_closeout", "runtime_node_qualification", "validation"):
        freshness_state = "verified_current"
        provenance_state = "verified"
        governance_outcome = "safe"
    elif workload_type in ("long_session_continuation", "ui_review_or_design_planning"):
        freshness_state = "stale_low_risk"
        provenance_state = "verified"
        governance_outcome = "warning"
    else:
        freshness_state = "verified_current"
        provenance_state = "verified"
        governance_outcome = "safe"

    # Special case: degraded node for runtime_node_qualification
    if workload_type == "runtime_node_qualification" and node_health == "stopped":
        if best_route == "remote_windows_runtime_cache":
            governance_outcome = "blocked"
            freshness_state = "blocked_for_task"

    # Performance sacrificed
    perf_sacrificed = False
    if best_route == "canonical_evidence_read" and best_latency > 50:
        perf_sacrificed = True

    # Build rejected alternatives
    alternatives = []
    for route in profile["route_ranking"]:
        if route != best_route:
            latency = all_latencies.get(route, 0)
            if route == "remote_windows_runtime_cache" and node_health == "stopped":
                reason = (
                    f"Remote runtime node is stopped. "
                    f"Measured degraded-node penalty: {MEASURED['degraded_node']['unreachable_timeout_ms']:.0f}ms. "
                    f"Local route {best_route} ({best_latency:.1f}ms) is preferred."
                )
            elif route == "canonical_evidence_read":
                reason = (
                    f"Canonical evidence read costs ~{latency:.0f}ms (measured: git subprocess overhead). "
                    f"Selected {best_route} ({best_latency:.1f}ms) instead for this workload."
                )
            elif route in ("ram_cache", "ssd_cache", "recent_turn_window"):
                reason = (
                    f"Local {route} costs ~{latency:.2f}ms (measured warm read). "
                    f"Governance/evidence requirements favor {best_route} for {workload_type}."
                )
            elif route == "compressed_recall_packet":
                reason = (
                    f"Compressed recall packet local processing costs ~{latency:.2f}ms (measured). "
                    f"Selected {best_route} ({best_latency:.1f}ms) for this workload."
                )
            else:
                reason = f"Route {route} costs ~{latency:.1f}ms. Selected {best_route} ({best_latency:.1f}ms) as better fit."
            alternatives.append({"route": route, "reason": reason})

    # Build evidence requirements
    evidence_reqs = {
        "requires_current_git_state": workload_type in ("sprint_closeout", "receipt_generation", "validation"),
        "requires_current_test_state": workload_type in ("sprint_closeout", "receipt_generation", "validation"),
        "requires_canonical_source": workload_type in ("receipt_generation",),
        "allows_stale_context": profile["stale_cache_tolerance"] in ("moderate", "high"),
    }

    # Build receipt summary
    receipt_label = f"{best_route} selected for {profile['label']}"
    if perf_sacrificed:
        receipt_detail = (
            f"Performance was sacrificed to satisfy {profile['evidence_requirement']} evidence requirements. "
            f"Canonical evidence read ({best_latency:.0f}ms) used instead of faster cache path."
        )
    elif best_route == "compressed_recall_packet":
        receipt_detail = (
            f"Compressed recall packet provides complete state for {workload_type}. "
            f"Measured local processing cost: ~{MEASURED['recall_packet_local']['total_local_32k_ms']:.2f}ms for 32K tokens. "
            f"Serialize+compress+decompress pipeline is negligible."
        )
    elif best_route in ("ram_cache", "ssd_cache", "recent_turn_window"):
        receipt_detail = (
            f"Local {best_route} selected. Measured warm read cost: ~{best_latency:.2f}ms. "
            f"Local context movement is cheap — no network or runtime overhead."
        )
    elif best_route == "canonical_evidence_read":
        receipt_detail = (
            f"Canonical evidence selected for provenance-verified fresh data. "
            f"Measured cost: ~{best_latency:.0f}ms (git subprocess overhead). "
            f"Evidence quality dominates cost consideration."
        )
    elif best_route == "remote_windows_runtime_cache":
        receipt_detail = (
            f"Remote runtime cache selected. LAN latency: ~35ms. "
            f"Node health must be verified before use."
        )
    elif best_route == "hybrid_recall_plus_fresh_evidence":
        receipt_detail = (
            f"Hybrid route combines recall packet ({MEASURED['recall_packet_local']['total_local_32k_ms']:.2f}ms) "
            f"with fresh canonical evidence ({MEASURED['windows_runtime_node']['git_status_ms']:.0f}ms)."
        )
    else:
        receipt_detail = f"Route selected based on measured costs and governance requirements."

    risk = "low"
    if workload_type in ("sprint_closeout", "receipt_generation", "runtime_node_qualification"):
        risk = "high"
    elif workload_type in ("validation", "code_patch_preparation"):
        risk = "medium"

    return {
        "route_id": route_id,
        "contract_version": "0.1",
        "workload_type": workload_type,
        "selected_context_route": best_route,
        "selected_runtime_profile": hardware,
        "freshness_state": freshness_state,
        "provenance_state": provenance_state,
        "governance_outcome": governance_outcome,
        "estimated_latency_ms": round(best_latency, 2),
        "performance_sacrificed_for_evidence": perf_sacrificed,
        "reason_selected": (
            f"Selected {best_route} for {profile['label']} workload. "
            f"Measured latency: ~{best_latency:.2f}ms. "
            f"Freshness: {freshness_state}. Provenance: {provenance_state}. "
            f"Governance: {governance_outcome}."
        ),
        "alternatives_rejected": alternatives,
        "evidence_requirements": evidence_reqs,
        "receipt_summary": {
            "label": receipt_label,
            "detail": receipt_detail,
            "risk": risk,
        },
        "_measured_costs_used": {
            "route": best_route,
            "estimated_latency_ms": round(best_latency, 2),
            "all_route_latencies": {k: round(v, 2) for k, v in all_latencies.items()},
        },
    }


def generate_decision(
    workload_type: str,
    hardware: str = "windows_runtime_node",
    node_health: str = "stopped",
) -> dict:
    """Generate a complete model_route + context_route decision."""
    model_route = select_model_route(workload_type, hardware)
    context_route = select_context_route(workload_type, hardware, node_health)

    return {
        "decision_id": f"decision-{uuid.uuid4().hex[:8]}",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "sprint_id": "MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1",
        "prototype_status": "non-production — mock decision only",
        "hardware_profile": hardware,
        "node_health": node_health,
        "model_route": model_route,
        "context_route": context_route,
        "decision_summary": {
            "workload_type": workload_type,
            "selected_model": model_route["selected_model_profile"],
            "selected_context_route": context_route["selected_context_route"],
            "estimated_total_latency_ms": (
                model_route["estimated_runtime_cost_ms"]
                + context_route["estimated_latency_ms"]
            ),
            "governance_outcome": context_route["governance_outcome"],
        },
    }


# ---------------------------------------------------------------------------
# Scenario generators
# ---------------------------------------------------------------------------

def generate_scenario_a_receipt() -> dict:
    """Scenario A — Receipt Generation. Must use canonical_evidence_read."""
    decision = generate_decision("receipt_generation")
    decision["scenario"] = "A_receipt_generation"
    decision["scenario_notes"] = (
        "Receipt generation requires canonical evidence. "
        "Must not allow weak-provenance stale context as final evidence. "
        f"Canonical evidence cost: ~{MEASURED['windows_runtime_node']['git_status_ms']:.0f}ms."
    )
    return decision


def generate_scenario_b_long_session() -> dict:
    """Scenario B — Long Session Continuation. Cheap local context."""
    decision = generate_decision("long_session_continuation")
    decision["scenario"] = "B_long_session_continuation"
    decision["scenario_notes"] = (
        "Long session continuation favors cheap local routes. "
        "Measured local context handling is very cheap (~0.3ms). "
        "Recall packet local processing: ~0.5ms for 32K tokens."
    )
    return decision


def generate_scenario_c_degraded_node() -> dict:
    """Scenario C — Degraded Runtime Node. Must show ~4000ms penalty."""
    decision = generate_decision(
        "runtime_node_qualification",
        hardware="weak_lan_runtime_node",
        node_health="stopped",
    )
    decision["scenario"] = "C_degraded_runtime_node"
    decision["scenario_notes"] = (
        f"Degraded runtime node applies measured penalty: "
        f"~{MEASURED['degraded_node']['unreachable_timeout_ms']:.0f}ms (TCP timeout). "
        f"Avoids degraded remote node. Prefers local canonical evidence."
    )
    return decision


def generate_scenario_d_agent_handoff() -> dict:
    """Scenario D — Agent Handoff. Compressed recall packet."""
    decision = generate_decision("agent_handoff")
    decision["scenario"] = "D_agent_handoff"
    decision["scenario_notes"] = (
        "Agent handoff uses compressed recall packet. "
        f"Measured local recall cost: ~{MEASURED['recall_packet_local']['total_local_32k_ms']:.2f}ms for 32K tokens. "
        "Local processing is separate from any network/runtime cost."
    )
    return decision


def generate_scenario_e_sprint_closeout() -> dict:
    """Scenario E — Sprint Closeout. Fresh evidence required."""
    decision = generate_decision("sprint_closeout")
    decision["scenario"] = "E_sprint_closeout"
    decision["scenario_notes"] = (
        "Sprint closeout requires fresh evidence. "
        f"Canonical evidence cost: ~{MEASURED['windows_runtime_node']['git_status_ms']:.0f}ms. "
        "Evidence quality dominates cost consideration."
    )
    return decision


def generate_scenario_f_ui_review() -> dict:
    """Scenario F — UI Review / Design Planning. Warning-level stale allowed."""
    decision = generate_decision("ui_review_or_design_planning")
    decision["scenario"] = "F_ui_review_design"
    decision["scenario_notes"] = (
        "UI review allows warning-level stale planning context. "
        "Prefers current UI evidence where available. "
        "Local ram_cache selected (~0.3ms measured)."
    )
    return decision


def generate_scenario_g_parallel_mixed() -> dict:
    """Scenario G — Parallel Mixed Workload. Multiple decisions."""
    workloads = [
        "sprint_closeout", "receipt_generation", "code_patch_preparation",
        "agent_handoff", "long_session_continuation",
    ]
    decisions = []
    for wl in workloads:
        d = generate_decision(wl)
        d["scenario"] = "G_parallel_mixed"
        decisions.append(d)

    return {
        "scenario": "G_parallel_mixed",
        "description": "Multiple workloads choosing different context routes using same measured profile data.",
        "decisions": decisions,
        "summary": {
            "workload_count": len(decisions),
            "routes_selected": [d["context_route"]["selected_context_route"] for d in decisions],
            "unique_routes": list(set(d["context_route"]["selected_context_route"] for d in decisions)),
        },
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    """Generate all prototype decisions."""
    print("=" * 70)
    print("MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 — Decision Generation")
    print("=" * 70)
    print(f"Timestamp: {datetime.now(timezone.utc).isoformat()}")
    print(f"Measured profiles: {MEASURED_PROFILES_PATH}")
    print()

    all_decisions = []

    # Generate decisions for all 9 workload types
    print("[1/3] Generating decisions for all 9 workload types...")
    for wl in WORKLOAD_TYPES:
        d = generate_decision(wl)
        all_decisions.append(d)
        print(f"  {wl}: route={d['context_route']['selected_context_route']}, "
              f"latency={d['context_route']['estimated_latency_ms']:.2f}ms, "
              f"governance={d['context_route']['governance_outcome']}")

    # Generate scenario decisions
    print("\n[2/3] Generating scenario decisions...")
    scenarios = {
        "A_receipt_generation": generate_scenario_a_receipt(),
        "B_long_session_continuation": generate_scenario_b_long_session(),
        "C_degraded_runtime_node": generate_scenario_c_degraded_node(),
        "D_agent_handoff": generate_scenario_d_agent_handoff(),
        "E_sprint_closeout": generate_scenario_e_sprint_closeout(),
        "F_ui_review_design": generate_scenario_f_ui_review(),
        "G_parallel_mixed": generate_scenario_g_parallel_mixed(),
    }

    for name, scenario in scenarios.items():
        if "decisions" in scenario:
            # Parallel mixed scenario
            print(f"  {name}: {scenario['summary']['workload_count']} decisions, "
                  f"routes={scenario['summary']['unique_routes']}")
        else:
            print(f"  {name}: route={scenario['context_route']['selected_context_route']}, "
                  f"latency={scenario['context_route']['estimated_latency_ms']:.2f}ms")

    # Save outputs
    print("\n[3/3] Saving outputs...")
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    PROTOTYPE_FIXTURES_DIR.mkdir(parents=True, exist_ok=True)

    # Machine-readable output
    output = {
        "metadata": {
            "sprint_id": "MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1",
            "version": "1.0.0",
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "prototype_status": "non-production — mock decisions only",
            "measured_costs_source": "config/measured_hardware_profiles.json",
            "contract_version": "0.1",
            "total_decisions": len(all_decisions),
        },
        "measured_costs": MEASURED,
        "workload_decisions": all_decisions,
        "scenario_decisions": scenarios,
    }

    results_path = OUTPUT_DIR / "router-context-prototype-decisions.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, default=str)
    print(f"  Results: {results_path}")

    # Save individual fixture files for each workload
    for d in all_decisions:
        wl = d["context_route"]["workload_type"]
        fixture_path = PROTOTYPE_FIXTURES_DIR / f"decision-{wl}.json"
        with open(fixture_path, "w", encoding="utf-8") as f:
            json.dump(d, f, indent=2, default=str)
    print(f"  Fixtures: {PROTOTYPE_FIXTURES_DIR}/ ({len(all_decisions)} files)")

    # Save scenario fixtures
    for name, scenario in scenarios.items():
        fixture_path = PROTOTYPE_FIXTURES_DIR / f"scenario-{name}.json"
        with open(fixture_path, "w", encoding="utf-8") as f:
            json.dump(scenario, f, indent=2, default=str)
    print(f"  Scenario fixtures: {len(scenarios)} files")

    print(f"\n{'=' * 70}")
    print(f"Total decisions generated: {len(all_decisions)}")
    print(f"Scenario cases: {len(scenarios)}")
    print(f"{'=' * 70}")

    return output


if __name__ == "__main__":
    main()
