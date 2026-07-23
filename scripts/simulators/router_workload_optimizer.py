#!/usr/bin/env python3
"""
router_workload_optimizer.py — Librarian Workload-Aware Context Route Optimizer

EXTENDS: context_reuse_simulator.py (MAC/WIN-CONTEXT-REUSE-SIMULATOR-0)

EXPLORATORY RESEARCH ONLY.
No production cache behavior. No GPU/RDMA/KV acceleration claims.
No production router modifications.

Models Librarian workload-aware context route scheduling.
Compares six strategies across eight workload scenarios.
"""

import json
import os
import math
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
DEFAULT_CONFIG_PATH = REPO_ROOT / "config" / "router_workload_optimizer.default.json"
DEFAULT_OUTPUT_PATH = REPO_ROOT / "reports" / "router-workload-optimizer-results.json"

ROUTES_ORDER = [
    "ram_cache",
    "ssd_cache",
    "remote_windows_runtime_cache",
    "recomputation",
    "compressed_recall_packet",
    "recent_turn_window",
    "canonical_evidence_read",
]

STRATEGIES_ORDER = [
    "always_fastest_path",
    "always_safest_path",
    "always_recompute_high_risk",
    "always_recall_packet",
    "prior_generic_scheduler",
    "librarian_workload_aware",
]

SCENARIOS_ORDER = [
    "A_sprint_planning",
    "B_sprint_closeout",
    "C_receipt_generation",
    "D_agent_handoff",
    "E_long_session_continuation",
    "F_runtime_node_qualification",
    "G_ui_review_design",
    "H_parallel_mixed",
]

FRESHNESS_THRESHOLDS = {
    "strict": 5,
    "high": 10,
    "moderate": 20,
    "low": 50,
}

RISK_LEVEL_ORDER = {"low": 0, "medium": 1, "high": 2}


# ---------------------------------------------------------------------------
# Config loading
# ---------------------------------------------------------------------------


def load_config(path: str | None = None) -> dict[str, Any]:
    path = path or str(DEFAULT_CONFIG_PATH)
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Deterministic pseudo-random
# ---------------------------------------------------------------------------


def det_hash(*args) -> int:
    h = 0
    for a in args:
        s = str(a)
        for c in s:
            h = h * 31 + ord(c)
    return h


def det_random(seed: int, mod: int = 10000) -> float:
    """Deterministic float in [0, 1) from seed."""
    return (det_hash(seed) % mod) / mod


# ---------------------------------------------------------------------------
# Governance state determination
# ---------------------------------------------------------------------------


def determine_governance_state(
    route_id: str,
    route_config: dict,
    freshness_ticks: int,
    provenance_verified: bool,
    workload_profile: dict,
    governance_rules: dict,
) -> str:
    """Determine the graduated governance state for a route under a workload."""

    # Routes that force freshness (e.g., recomputation, canonical evidence)
    if route_config.get("force_fresh") or route_config.get("always_fresh"):
        if route_config.get("always_provenance_verified"):
            return "verified_current"
        if provenance_verified:
            return "verified_current"
        return "recent_but_unverified"

    # Disallowed routes
    if route_id in workload_profile.get("disallowed_routes", []):
        return "blocked_for_task"

    # Check provenance
    if not provenance_verified:
        # Even with weak provenance, we don't block — we assign a penalty state
        return "provenance_weak"

    # Check freshness against workload requirement
    freshness_req = workload_profile.get("freshness_requirement", "moderate")
    threshold = FRESHNESS_THRESHOLDS.get(freshness_req, 10)
    tolerance = workload_profile.get("stale_cache_tolerance", "moderate")

    if freshness_ticks <= threshold:
        return "verified_current"
    elif freshness_ticks <= int(threshold * 1.5):
        return "recent_but_unverified"
    elif freshness_ticks <= threshold * 3:
        if tolerance in ("high", "moderate"):
            return "stale_low_risk"
        else:
            return "stale_requires_revalidation"
    else:
        if tolerance == "high":
            return "stale_low_risk"
        else:
            return "stale_requires_revalidation"


# ---------------------------------------------------------------------------
# Hardware-aware latency adjustment
# ---------------------------------------------------------------------------


def hardware_latency_multiplier(hardware_profile: dict, route_config: dict) -> float:
    """Adjust latency based on hardware profile capabilities."""
    mult = hardware_profile.get("reciprocal_transfer_multiplier", 1.0)
    # Network paths get additional adjustment based on LAN quality
    if route_config.get("requires_network"):
        jitter = hardware_profile.get("lan_jitter_ms", 15.0)
        # Higher jitter = worse performance
        mult *= (1.0 + jitter / 100.0)
    return mult


# ---------------------------------------------------------------------------
# Route latency estimation
# ---------------------------------------------------------------------------


def estimate_route_latency(
    route_id: str,
    route_config: dict,
    context_tokens: int,
    append_tokens: int,
    is_cache_hit: bool,
    governance_state: str,
    governance_states: dict,
    hardware_profile: dict,
    scenario_cfg: dict,
    turn_number: int,
    session_id: int = 0,
) -> tuple[float, str]:
    """Estimate latency for a route. Returns (latency_ms, governance_outcome)."""

    cfg = route_config

    # Base latency
    latency = cfg["base_latency_ms"]

    # Context transfer cost
    latency += context_tokens * cfg.get("context_transfer_cost_per_token_ms", 0)

    # Append processing cost
    latency += append_tokens * cfg.get("append_processing_cost_per_token_ms", 0)

    # Cache miss penalty
    if not is_cache_hit:
        latency += cfg.get("cache_miss_penalty_ms", 0)

    # Governance state penalty
    gs_cfg = governance_states.get(governance_state, {})
    latency += gs_cfg.get("latency_penalty_ms", 0)

    # Freshness penalty (graduated)
    freshness_penalty_per_tick = cfg.get("freshness_penalty_per_tick_ms", 0)
    if governance_state in ("stale_low_risk", "stale_requires_revalidation", "recent_but_unverified"):
        # Apply graduated penalty based on how stale
        if governance_state == "stale_requires_revalidation":
            latency += freshness_penalty_per_tick * 2.0  # extra penalty
        elif governance_state == "recent_but_unverified":
            latency += freshness_penalty_per_tick * 0.5

    # Provenance penalty (graduated)
    if governance_state == "provenance_weak":
        latency += cfg.get("provenance_penalty_if_unverified_ms", 0) * 0.5

    # Network cost
    if cfg.get("requires_network", False):
        lan_base = scenario_cfg.get("lan_latency_base_ms",
                                     hardware_profile.get("lan_latency_ms", 35.0))
        lan_jitter = scenario_cfg.get("lan_jitter_ms",
                                       hardware_profile.get("lan_jitter_ms", 15.0))
        is_unstable = scenario_cfg.get("lan_quality",
                                        hardware_profile.get("lan_quality", "stable")) == "unstable"
        if is_unstable:
            # Deterministic jitter
            jitter = math.sin(turn_number * 0.7 + session_id * 3.1) * lan_jitter * 1.2
            latency += lan_base + jitter
        else:
            latency += lan_base

    # Compression/decompression for recall packet
    if route_id == "compressed_recall_packet":
        latency += context_tokens * cfg.get("compression_cost_per_token_ms", 0)
        latency += context_tokens * cfg.get("decompression_cost_per_token_ms", 0)

    # Hardware multiplier
    hw_mult = hardware_latency_multiplier(hardware_profile, cfg)
    latency *= hw_mult

    # Determine governance outcome
    outcome = "safe"
    if governance_state in ("stale_low_risk", "recent_but_unverified"):
        outcome = "warning"
    elif governance_state == "stale_requires_revalidation":
        outcome = "requires_revalidation"
    elif governance_state == "provenance_weak":
        outcome = "warning"
    elif governance_state == "blocked_for_task":
        outcome = "blocked"

    return latency, outcome


# ---------------------------------------------------------------------------
# Route scoring for workload-aware optimizer
# ---------------------------------------------------------------------------


def score_route_for_workload(
    route_id: str,
    route_config: dict,
    workload_profile: dict,
    context_tokens: int,
    append_tokens: int,
    cache_hit: bool,
    governance_state: str,
    governance_states: dict,
    hardware_profile: dict,
    scenario_cfg: dict,
    turn_number: int,
    session_id: int = 0,
) -> tuple[float, str]:
    """Score a route for the workload-aware optimizer. Lower is better.

    Combines latency with governance risk into a single score.
    High-risk governance states get a multiplicative penalty on latency.
    """
    latency, outcome = estimate_route_latency(
        route_id, route_config, context_tokens, append_tokens,
        cache_hit, governance_state, governance_states,
        hardware_profile, scenario_cfg, turn_number, session_id,
    )

    gs_cfg = governance_states.get(governance_state, {})
    risk_score = gs_cfg.get("risk_score", 0.0)

    # Risk-adjusted score: latency * (1 + risk * risk_multiplier)
    risk_mult = 1.0 + risk_score * 3.0  # up to 4x for blocked_for_task
    score = latency * risk_mult

    return score, outcome


# ---------------------------------------------------------------------------
# Route ranking helpers
# ---------------------------------------------------------------------------


def get_route_ranking_for_workload(workload_profile: dict) -> list[str]:
    """Get the ordered route ranking from workload profile."""
    return workload_profile.get("route_ranking", ROUTES_ORDER)


def get_route_preference_set(workload_profile: dict) -> set[str]:
    """Get the preferred routes set from workload profile."""
    return set(workload_profile.get("preferred_routes", []))


# ---------------------------------------------------------------------------
# Strategy decision functions
# ---------------------------------------------------------------------------


def decide_always_fastest_path(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    route_states: dict, session_id: int,
) -> tuple[str, float, dict]:
    """Pick the route with the lowest base latency, ignoring governance."""
    routes = config["context_routes"]
    best_route = None
    best_latency = float("inf")
    alternatives = []

    for rid in routes:
        rcfg = routes[rid]
        if not rcfg.get("always_available", True):
            continue
        if scenario_cfg.get("lan_quality", "stable") == "down" and rcfg.get("requires_network"):
            continue

        hit = route_states[rid]["hit"]
        gs = "verified_current"  # ignore governance
        hw = config["hardware_profiles"].get(scenario_cfg.get("hardware_profile", "current_mac_coordinator"), {})

        latency, outcome = estimate_route_latency(
            rid, rcfg, context_tokens, append_tokens,
            hit, gs, config["governance_states"],
            hw, scenario_cfg, turn_number, session_id,
        )
        alternatives.append({"route": rid, "latency_ms": round(latency, 2)})
        if latency < best_latency:
            best_latency = latency
            best_route = rid

    alternatives.sort(key=lambda x: x["latency_ms"])
    return best_route, best_latency, {
        "alternatives": alternatives[1:],
        "risk_sacrificed": False,
    }


def decide_always_safest_path(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    route_states: dict, session_id: int,
) -> tuple[str, float, dict]:
    """Pick the route with highest provenance/freshness, ignoring latency."""
    routes = config["context_routes"]
    # Canonical evidence read and recomputation are always safe
    safe_order = ["canonical_evidence_read", "recomputation", "ram_cache",
                   "compressed_recall_packet", "ssd_cache", "recent_turn_window",
                   "remote_windows_runtime_cache"]

    best_route = None
    best_latency = float("inf")

    for rid in safe_order:
        if rid not in routes:
            continue
        rcfg = routes[rid]
        if not rcfg.get("always_available", True):
            continue
        if scenario_cfg.get("lan_quality", "stable") == "down" and rcfg.get("requires_network"):
            continue

        hit = route_states[rid]["hit"]
        gs = "verified_current"
        hw = config["hardware_profiles"].get(scenario_cfg.get("hardware_profile", "current_mac_coordinator"), {})

        latency, outcome = estimate_route_latency(
            rid, rcfg, context_tokens, append_tokens,
            hit, gs, config["governance_states"],
            hw, scenario_cfg, turn_number, session_id,
        )
        if best_route is None:
            best_route = rid
            best_latency = latency

    return best_route, best_latency, {"alternatives": [], "risk_sacrificed": False}


def decide_always_recompute_high_risk(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    route_states: dict, session_id: int,
) -> tuple[str, float, dict]:
    """Recompute for high-risk tasks; use fastest cache for others."""
    workload_type = scenario_cfg.get("workload_type", "sprint_planning")
    wp = config["workload_profiles"].get(workload_type, {})
    risk = wp.get("risk_level", "low")

    if risk == "high":
        # Recompute
        rcfg = config["context_routes"]["recomputation"]
        hw = config["hardware_profiles"].get(scenario_cfg.get("hardware_profile", "current_mac_coordinator"), {})
        latency, outcome = estimate_route_latency(
            "recomputation", rcfg, context_tokens, append_tokens,
            True, "verified_current", config["governance_states"],
            hw, scenario_cfg, turn_number, session_id,
        )
        return "recomputation", latency, {"alternatives": [], "risk_sacrificed": False}
    else:
        return decide_always_fastest_path(
            config, scenario_cfg, turn_number,
            context_tokens, append_tokens, route_states, session_id,
        )


def decide_always_recall_packet(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    route_states: dict, session_id: int,
) -> tuple[str, float, dict]:
    """Always use compressed recall packet."""
    rcfg = config["context_routes"]["compressed_recall_packet"]
    hit = route_states["compressed_recall_packet"]["hit"]
    hw = config["hardware_profiles"].get(scenario_cfg.get("hardware_profile", "current_mac_coordinator"), {})

    latency, outcome = estimate_route_latency(
        "compressed_recall_packet", rcfg, context_tokens, append_tokens,
        hit, "verified_current", config["governance_states"],
        hw, scenario_cfg, turn_number, session_id,
    )
    return "compressed_recall_packet", latency, {"alternatives": [], "risk_sacrificed": False}


def decide_prior_generic_scheduler(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    route_states: dict, session_id: int,
) -> tuple[str, float, dict]:
    """Prior generic scheduler: pick lowest latency path, ignoring workload type.

    This is the behavior from MAC/WIN-CONTEXT-REUSE-SIMULATOR-0's local_scheduler.
    It blocks paths that fail governance checks but doesn't understand workload types.
    """
    routes = config["context_routes"]
    gs_cfg = config["governance_states"]
    hw = config["hardware_profiles"].get(scenario_cfg.get("hardware_profile", "current_mac_coordinator"), {})
    fresh_enabled = scenario_cfg.get("freshness_decay_enabled", False)
    prov_enabled = scenario_cfg.get("provenance_checks_enabled", False)
    max_stale = config.get("governance_rules", {}).get("freshness_tick_thresholds", {}).get("high", 10)

    candidates = []
    blocked = {}

    for rid in routes:
        rcfg = routes[rid]
        if not rcfg.get("always_available", True):
            blocked[rid] = "path_unavailable"
            continue
        if scenario_cfg.get("lan_quality", "stable") == "down" and rcfg.get("requires_network"):
            blocked[rid] = "network_unavailable"
            continue

        rs = route_states[rid]
        gs = determine_governance_state(
            rid, rcfg, rs["freshness_ticks"], rs["provenance_verified"],
            {"freshness_requirement": "high", "stale_cache_tolerance": "low",
             "disallowed_routes": []},
            config["governance_rules"],
        )

        # Generic scheduler blocks stale/unverified (hard block like SIM-0)
        if gs in ("stale_requires_revalidation", "blocked_for_task"):
            blocked[rid] = gs
            continue
        if not rs["provenance_verified"] and prov_enabled:
            blocked[rid] = "provenance_unverified"
            continue

        latency, outcome = estimate_route_latency(
            rid, rcfg, context_tokens, append_tokens,
            rs["hit"], gs, gs_cfg, hw, scenario_cfg, turn_number, session_id,
        )
        candidates.append((latency, rid))

    if not candidates:
        # Fallback to recomputation
        rcfg = routes["recomputation"]
        hw2 = config["hardware_profiles"].get(scenario_cfg.get("hardware_profile", "current_mac_coordinator"), {})
        latency, outcome = estimate_route_latency(
            "recomputation", rcfg, context_tokens, append_tokens,
            True, "verified_current", gs_cfg, hw2, scenario_cfg, turn_number, session_id,
        )
        return "recomputation", latency, {"alternatives": [], "blocked": blocked}

    candidates.sort(key=lambda x: x[0])
    chosen = candidates[0][1]
    alts = [{"route": p, "latency_ms": round(l, 2)} for l, p in candidates[1:]]
    return chosen, candidates[0][0], {"alternatives": alts, "blocked": blocked}


def decide_librarian_workload_aware(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    route_states: dict, session_id: int,
) -> tuple[str, float, dict]:
    """Workload-aware optimizer: choose route based on workload type, governance, and risk.

    This is the key new strategy. It:
    1. Looks up the workload profile for the current scenario
    2. Filters out disallowed routes
    3. Ranks remaining routes using the profile's preference/ranking
    4. Applies graduated governance penalties (not hard blocking)
    5. Scores each route by latency * risk multiplier
    6. Picks the route with the lowest risk-adjusted score
    """
    workload_type = scenario_cfg.get("workload_type", "sprint_planning")
    wp = config["workload_profiles"].get(workload_type, {})
    routes = config["context_routes"]
    gs_cfg = config["governance_states"]
    hw_id = scenario_cfg.get("hardware_profile", "current_mac_coordinator")
    hw = config["hardware_profiles"].get(hw_id, {})

    route_ranking = get_route_ranking_for_workload(wp)
    preferred = get_route_preference_set(wp)
    disallowed = set(wp.get("disallowed_routes", []))

    candidates = []
    blocked = {}
    warnings = []
    revalidations = []

    for rid in route_ranking:
        if rid not in routes:
            continue
        rcfg = routes[rid]

        if rid in disallowed:
            blocked[rid] = "disallowed_for_task"
            continue
        if not rcfg.get("always_available", True):
            blocked[rid] = "path_unavailable"
            continue
        if scenario_cfg.get("lan_quality", "stable") == "down" and rcfg.get("requires_network"):
            blocked[rid] = "network_unavailable"
            continue

        rs = route_states[rid]
        gs = determine_governance_state(
            rid, rcfg, rs["freshness_ticks"], rs["provenance_verified"],
            wp, config["governance_rules"],
        )

        # Graduated governance: don't block — apply penalty
        # But DO block for_task and blocked_for_task
        if gs == "blocked_for_task":
            blocked[rid] = gs
            continue

        score, outcome = score_route_for_workload(
            rid, rcfg, wp, context_tokens, append_tokens,
            rs["hit"], gs, gs_cfg, hw, scenario_cfg, turn_number, session_id,
        )

        if outcome == "requires_revalidation":
            revalidations.append(rid)
        elif outcome == "warning":
            warnings.append(rid)

        # Prefer preferred routes with a small bonus
        if rid in preferred:
            score *= 0.9  # 10% bonus for preferred routes

        candidates.append((score, rid, outcome))

    if not candidates:
        rcfg = routes["recomputation"]
        latency, outcome = estimate_route_latency(
            "recomputation", rcfg, context_tokens, append_tokens,
            True, "verified_current", gs_cfg, hw, scenario_cfg, turn_number, session_id,
        )
        return "recomputation", latency, {
            "alternatives": [], "blocked": blocked,
            "warnings": [], "revalidations": [],
            "workload_type": workload_type,
        }

    candidates.sort(key=lambda x: x[0])
    chosen = candidates[0][1]
    chosen_outcome = candidates[0][2]
    alts = [{"route": p, "score": round(s, 2), "outcome": o} for s, p, o in candidates[1:]]

    # Check if performance was sacrificed for governance
    perf_sacrificed = chosen_outcome in ("warning", "requires_revalidation")

    return chosen, candidates[0][0], {
        "alternatives": alts,
        "blocked": blocked,
        "warnings": warnings,
        "revalidations": revalidations,
        "governance_outcome": chosen_outcome,
        "performance_sacrificed_for_evidence": perf_sacrificed,
        "workload_type": workload_type,
    }


STRATEGY_DECISION_MAP = {
    "always_fastest_path": decide_always_fastest_path,
    "always_safest_path": decide_always_safest_path,
    "always_recompute_high_risk": decide_always_recompute_high_risk,
    "always_recall_packet": decide_always_recall_packet,
    "prior_generic_scheduler": decide_prior_generic_scheduler,
    "librarian_workload_aware": decide_librarian_workload_aware,
}


# ---------------------------------------------------------------------------
# Session / scenario simulation
# ---------------------------------------------------------------------------


def simulate_turn(
    config: dict,
    scenario_cfg: dict,
    strategy_id: str,
    turn_number: int,
    route_states: dict,
    context_tokens: int,
    append_tokens: int,
    session_id: int = 0,
) -> dict:
    """Simulate a single turn for a single strategy and return a decision record."""
    decision_fn = STRATEGY_DECISION_MAP[strategy_id]
    chosen_route, score_or_latency, meta = decision_fn(
        config, scenario_cfg, turn_number,
        context_tokens, append_tokens, route_states, session_id,
    )

    rcfg = config["context_routes"].get(chosen_route, {})
    rs = route_states.get(chosen_route, {})

    # Determine governance state for the chosen route
    wp = config["workload_profiles"].get(scenario_cfg.get("workload_type", ""), {})
    gs = determine_governance_state(
        chosen_route, rcfg, rs["freshness_ticks"], rs["provenance_verified"],
        wp, config["governance_rules"],
    )

    # Get actual latency for reporting
    hw = config["hardware_profiles"].get(scenario_cfg.get("hardware_profile", "current_mac_coordinator"), {})
    actual_latency, outcome = estimate_route_latency(
        chosen_route, rcfg, context_tokens, append_tokens,
        rs["hit"], gs, config["governance_states"],
        hw, scenario_cfg, turn_number, session_id,
    )

    # Build alternatives list
    alternatives = []
    for rid in ROUTES_ORDER:
        if rid == chosen_route:
            continue
        if rid not in config["context_routes"]:
            continue
        rrcfg = config["context_routes"][rid]
        rrs = route_states.get(rid, route_states.get(rid, {}))
        rgs = determine_governance_state(
            rid, rrcfg, rrs.get("freshness_ticks", 0), rrs.get("provenance_verified", True),
            wp, config["governance_rules"],
        )
        rlat, rout = estimate_route_latency(
            rid, rrcfg, context_tokens, append_tokens,
            rrs.get("hit", False), rgs, config["governance_states"],
            hw, scenario_cfg, turn_number, session_id,
        )
        alternatives.append({
            "route": rid,
            "latency_ms": round(rlat, 2),
            "governance_state": rgs,
            "outcome": rout,
        })
    alternatives.sort(key=lambda x: x["latency_ms"])

    record = {
        "turn": turn_number,
        "session_id": session_id,
        "context_tokens": context_tokens,
        "append_tokens": append_tokens,
        "selected_route": chosen_route,
        "selected_route_label": rcfg.get("label", chosen_route),
        "estimated_latency_ms": round(actual_latency, 2),
        "estimated_cost": round(actual_latency, 2),
        "cache_hit": rs.get("hit", True),
        "freshness_ticks": rs.get("freshness_ticks", 0),
        "provenance_verified": rs.get("provenance_verified", True),
        "governance_state": gs,
        "governance_outcome": outcome,
        "risk_level": wp.get("risk_level", "low"),
        "performance_sacrificed_for_evidence": meta.get("performance_sacrificed_for_evidence", False),
        "reason_selected": meta.get("reason", f"Workload-aware: {scenario_cfg.get('workload_type', 'unknown')}"),
        "alternatives_rejected": alternatives,
        "blocked_routes": meta.get("blocked", {}),
        "workload_type": scenario_cfg.get("workload_type", "unknown"),
    }
    return record


def simulate_session(
    config: dict,
    scenario_cfg: dict,
    strategy_id: str,
    session_id: int = 0,
    workload_type_override: str | None = None,
) -> dict:
    """Simulate one session across all turns."""
    num_turns = scenario_cfg["num_turns"]
    wt = workload_type_override or scenario_cfg.get("workload_type", "sprint_planning")
    wp = config["workload_profiles"].get(wt, {})

    context_tokens = wp.get("typical_context_tokens", scenario_cfg.get("reused_context_tokens", 32700))
    append_tokens = wp.get("typical_append_tokens", scenario_cfg.get("append_tokens", 429))
    freshness_enabled = scenario_cfg.get("freshness_decay_enabled", False)
    prov_enabled = scenario_cfg.get("provenance_checks_enabled", False)
    stale_start = scenario_cfg.get("freshness_ticks_before_decay_start", 5)

    # Initialize route states
    route_states = {}
    for rid in ROUTES_ORDER:
        route_states[rid] = {
            "hit": True,
            "freshness_ticks": 0,
            "provenance_verified": True,
        }
        if rid == "recomputation" or rid == "canonical_evidence_read":
            route_states[rid]["provenance_verified"] = True
            route_states[rid]["freshness_ticks"] = 0

    # Override freshness ticks for stale scenarios
    if freshness_enabled:
        for rid in ROUTES_ORDER:
            if rid not in ("recomputation", "canonical_evidence_read"):
                # Start with some staleness based on stale_after_ticks in scenario
                stale_after = scenario_cfg.get("stale_after_ticks", stale_start + 10)
                if turn_number := 0:  # will be set per turn
                    pass

    prov_verification_prob = config.get("governance_rules", {}).get("provenance_verification_probability", 0.85)

    strategy_label = config["strategies"][strategy_id]["label"]
    turns = []
    total_latency = 0.0
    cache_hits = 0
    route_usage = {}
    governance_counts = {"safe": 0, "warning": 0, "requires_revalidation": 0, "blocked": 0}
    perf_sacrificed = 0

    for turn in range(1, num_turns + 1):
        # Determine cache hit (deterministic)
        seed = session_id * 10000 + turn + hash(wt) % 1000
        cache_hit_rate = wp.get("acceptable_recall_compression", 0.85)
        hit_val = det_random(seed)
        is_hit = hit_val < cache_hit_rate

        # Update route states
        for rid in ROUTES_ORDER:
            if freshness_enabled:
                if rid in ("recomputation", "canonical_evidence_read"):
                    route_states[rid]["freshness_ticks"] = 0
                    route_states[rid]["provenance_verified"] = True
                else:
                    if turn > stale_start:
                        route_states[rid]["freshness_ticks"] = turn - stale_start
                    else:
                        route_states[rid]["freshness_ticks"] = 0
            else:
                route_states[rid]["freshness_ticks"] = 0

            # Provenance verification (deterministic)
            if prov_enabled:
                prov_val = det_random(seed + hash(rid))
                route_states[rid]["provenance_verified"] = prov_val < prov_verification_prob
            else:
                route_states[rid]["provenance_verified"] = True

            # Cache hit
            route_states[rid]["hit"] = is_hit

            # Recomputation and canonical evidence always hit
            if rid in ("recomputation", "canonical_evidence_read"):
                route_states[rid]["hit"] = True

        record = simulate_turn(
            config, scenario_cfg, strategy_id, turn,
            route_states, context_tokens, append_tokens, session_id,
        )
        turns.append(record)
        total_latency += record["estimated_latency_ms"]
        if record["cache_hit"]:
            cache_hits += 1
        route_usage[record["selected_route"]] = route_usage.get(record["selected_route"], 0) + 1
        governance_counts[record["governance_outcome"]] = governance_counts.get(record["governance_outcome"], 0) + 1
        if record["performance_sacrificed_for_evidence"]:
            perf_sacrificed += 1

    avg_latency = total_latency / num_turns if num_turns > 0 else 0
    throughput = 1000.0 / avg_latency if avg_latency > 0 else 0

    return {
        "session_id": session_id,
        "strategy_id": strategy_id,
        "strategy_label": strategy_label,
        "workload_type": wt,
        "num_turns": num_turns,
        "total_latency_ms": round(total_latency, 2),
        "average_latency_per_turn_ms": round(avg_latency, 2),
        "throughput_turns_per_sec": round(throughput, 4),
        "cache_hits": cache_hits,
        "cache_misses": num_turns - cache_hits,
        "cache_hit_rate_actual": round(cache_hits / num_turns, 4) if num_turns > 0 else 0,
        "route_usage": route_usage,
        "governance_counts": governance_counts,
        "performance_sacrificed_count": perf_sacrificed,
        "turns": turns,
    }


def simulate_scenario(
    config: dict,
    scenario_id: str,
    scenario_cfg: dict,
) -> dict:
    """Run all strategies for a single scenario."""
    num_sessions = scenario_cfg.get("num_sessions", 1)
    scenario_label = scenario_cfg["label"]
    mixed_workloads = scenario_cfg.get("mixed_workloads", None)

    strategy_results = []

    for strategy_id in STRATEGIES_ORDER:
        session_results = []
        for sid in range(num_sessions):
            # For mixed workload scenarios, use different workload types per session
            wt_override = None
            if mixed_workloads and sid < len(mixed_workloads):
                wt_override = mixed_workloads[sid]["workload_type"]

            session_result = simulate_session(
                config, scenario_cfg, strategy_id,
                session_id=sid, workload_type_override=wt_override,
            )
            session_results.append(session_result)

        # Aggregate across sessions
        total_lat = sum(s["total_latency_ms"] for s in session_results)
        total_turns = sum(s["num_turns"] for s in session_results)
        avg_lat = total_lat / total_turns if total_turns > 0 else 0
        total_hits = sum(s["cache_hits"] for s in session_results)
        total_misses = sum(s["cache_misses"] for s in session_results)

        combined_route_usage = {}
        combined_governance = {"safe": 0, "warning": 0, "requires_revalidation": 0, "blocked": 0}
        total_perf_sacrificed = 0
        for s in session_results:
            for pid, cnt in s["route_usage"].items():
                combined_route_usage[pid] = combined_route_usage.get(pid, 0) + cnt
            for k, v in s["governance_counts"].items():
                combined_governance[k] = combined_governance.get(k, 0) + v
            total_perf_sacrificed += s["performance_sacrificed_count"]

        strategy_results.append({
            "strategy_id": strategy_id,
            "strategy_label": config["strategies"][strategy_id]["label"],
            "num_sessions": num_sessions,
            "total_turns": total_turns,
            "total_latency_ms": round(total_lat, 2),
            "average_latency_per_turn_ms": round(avg_lat, 2),
            "throughput_avg_turns_per_sec": round(1000.0 / avg_lat if avg_lat > 0 else 0, 4),
            "cache_hits": total_hits,
            "cache_misses": total_misses,
            "cache_hit_rate_actual": round(total_hits / (total_hits + total_misses), 4) if (total_hits + total_misses) > 0 else 0,
            "route_usage": combined_route_usage,
            "governance_counts": combined_governance,
            "performance_sacrificed_count": total_perf_sacrificed,
            "sessions": session_results,
        })

    # Strategy comparison
    comparison = {}
    for sr in strategy_results:
        comparison[sr["strategy_id"]] = {
            "average_latency_per_turn_ms": sr["average_latency_per_turn_ms"],
            "throughput_avg_turns_per_sec": sr["throughput_avg_turns_per_sec"],
            "cache_hit_rate_actual": sr["cache_hit_rate_actual"],
            "total_latency_ms": sr["total_latency_ms"],
            "governance_counts": sr["governance_counts"],
            "performance_sacrificed_count": sr["performance_sacrificed_count"],
        }

    # Workload-aware vs other strategies
    workload_aware = next((sr for sr in strategy_results if sr["strategy_id"] == "librarian_workload_aware"), None)
    comparisons_vs_others = {}
    if workload_aware:
        wa_lat = workload_aware["average_latency_per_turn_ms"]
        wa_gov = workload_aware["governance_counts"]
        for sr in strategy_results:
            if sr["strategy_id"] == "librarian_workload_aware":
                continue
            bl_lat = sr["average_latency_per_turn_ms"]
            bl_gov = sr["governance_counts"]
            if bl_lat > 0:
                impr = round((bl_lat - wa_lat) / bl_lat * 100, 2)
            else:
                impr = 0.0
            comparisons_vs_others[sr["strategy_id"]] = {
                "baseline_latency_ms": bl_lat,
                "optimizer_latency_ms": wa_lat,
                "latency_improvement_pct": impr,
                "baseline_governance": bl_gov,
                "optimizer_governance": wa_gov,
            }

    # Generic scheduler vs workload-aware (key comparison)
    generic_sched = next((sr for sr in strategy_results if sr["strategy_id"] == "prior_generic_scheduler"), None)
    generic_vs_workload = {}
    if generic_sched and workload_aware:
        gs_lat = generic_sched["average_latency_per_turn_ms"]
        wa_lat2 = workload_aware["average_latency_per_turn_ms"]
        gs_blocks = generic_sched["governance_counts"].get("blocked", 0) + generic_sched["governance_counts"].get("requires_revalidation", 0)
        wa_blocks = workload_aware["governance_counts"].get("blocked", 0) + workload_aware["governance_counts"].get("requires_revalidation", 0)
        generic_vs_workload = {
            "generic_scheduler_latency_ms": gs_lat,
            "workload_aware_latency_ms": wa_lat2,
            "latency_change_pct": round((gs_lat - wa_lat2) / gs_lat * 100, 2) if gs_lat > 0 else 0,
            "generic_governance": generic_sched["governance_counts"],
            "workload_aware_governance": workload_aware["governance_counts"],
            "generic_hard_blocks_or_revalidations": gs_blocks,
            "workload_aware_hard_blocks_or_revalidations": wa_blocks,
            "stale_regression_improved": wa_lat2 < gs_lat,
        }

    return {
        "scenario_id": scenario_id,
        "scenario_label": scenario_label,
        "workload_type": scenario_cfg.get("workload_type", "unknown"),
        "hardware_profile": scenario_cfg.get("hardware_profile", "current_mac_coordinator"),
        "strategy_comparison": comparison,
        "workload_aware_vs_others": comparisons_vs_others,
        "generic_vs_workload_aware": generic_vs_workload,
        "results": strategy_results,
    }


def simulate_all(config: dict) -> dict:
    """Run all scenarios."""
    scenarios = config["scenarios"]
    output = {
        "metadata": {
            "title": "Router Workload Optimizer Results",
            "subtitle": "Librarian Workload-Aware Context Route Scheduling",
            "honesty": "Exploratory research only. No production cache behavior. No GPU/RDMA/KV acceleration claims.",
            "simulator_version": "2.0.0",
            "extends": "context_reuse_simulator.py (MAC/WIN-CONTEXT-REUSE-SIMULATOR-0)",
            "config_file": str(DEFAULT_CONFIG_PATH),
            "workload_profiles_count": len(config.get("workload_profiles", {})),
            "context_routes_count": len(config.get("context_routes", {})),
            "hardware_profiles_count": len(config.get("hardware_profiles", {})),
            "strategies_count": len(config.get("strategies", {})),
        },
        "scenarios": [],
    }
    for sid, scfg in scenarios.items():
        result = simulate_scenario(config, sid, scfg)
        output["scenarios"].append(result)
    return output


# ---------------------------------------------------------------------------
# Human-readable report
# ---------------------------------------------------------------------------


def generate_report(output: dict) -> str:
    """Generate a comprehensive Markdown report."""
    lines = []
    meta = output["metadata"]
    lines.append("# MAC/WIN-ROUTER-WORKLOAD-OPTIMIZER-1 — Sprint Report\n")
    lines.append(f"**{meta['title']}**\n")
    lines.append(f"*{meta['subtitle']}*\n")
    lines.append(f"\n> {meta['honesty']}\n")
    lines.append(f"\nSimulator version: `{meta['simulator_version']}` — extends `{meta['extends']}`\n")
    lines.append(f"\nWorkload profiles: {meta['workload_profiles_count']} | "
                  f"Context routes: {meta['context_routes_count']} | "
                  f"Hardware profiles: {meta['hardware_profiles_count']} | "
                  f"Strategies: {meta['strategies_count']}\n")
    lines.append("\n---\n")

    # Strategy comparison table across scenarios
    lines.append("## Cross-Scenario Strategy Comparison\n")
    lines.append("| Scenario | Strategy | Avg Latency (ms) | Throughput (t/s) | Safe | Warning | Revalidate | Blocked | Perf Sacrificed |")
    lines.append("|----------|----------|------------------:|------------------:|-----:|--------:|-----------:|--------:|----------------:|")

    for scenario in output["scenarios"]:
        for sid, stats in scenario["strategy_comparison"].items():
            label = next(
                (r["strategy_label"] for r in scenario["results"] if r["strategy_id"] == sid),
                sid,
            )
            gov = stats["governance_counts"]
            perf_s = stats["performance_sacrificed_count"]
            # Short strategy name
            short = label.replace("Librarian Workload-Aware Optimizer", "WorkloadAware") \
                         .replace("Prior Generic Scheduler (from SIM-0)", "GenericSched") \
                         .replace("Always Fastest Context Path", "AlwaysFast") \
                         .replace("Always Safest Context Path", "AlwaysSafe") \
                         .replace("Always Recompute for High-Risk Tasks", "RecomputeHiRisk") \
                         .replace("Always Compressed Recall Packet", "AlwaysRecall")
            if len(short) > 20:
                short = sid[:12]
            lines.append(
                f"| {scenario['scenario_label'][:30]} | {short[:15]} | "
                f"{stats['average_latency_per_turn_ms']} | "
                f"{stats['throughput_avg_turns_per_sec']} | "
                f"{gov.get('safe', 0)} | {gov.get('warning', 0)} | "
                f"{gov.get('requires_revalidation', 0)} | {gov.get('blocked', 0)} | "
                f"{perf_s} |"
            )
    lines.append("")

    # Generic scheduler vs workload-aware key comparison
    lines.append("\n## Generic Scheduler vs Workload-Aware Optimizer\n")
    for scenario in output["scenarios"]:
        gv = scenario.get("generic_vs_workload_aware", {})
        if not gv:
            continue
        lines.append(f"\n### {scenario['scenario_label']}\n")
        lines.append(f"- **Generic scheduler latency:** {gv['generic_scheduler_latency_ms']} ms")
        lines.append(f"- **Workload-aware latency:** {gv['workload_aware_latency_ms']} ms")
        lines.append(f"- **Latency change:** {gv['latency_change_pct']}%")
        lines.append(f"- **Generic governance:** safe={gv['generic_governance'].get('safe', 0)}, "
                      f"warning={gv['generic_governance'].get('warning', 0)}, "
                      f"revalidate={gv['generic_governance'].get('requires_revalidation', 0)}, "
                      f"blocked={gv['generic_governance'].get('blocked', 0)}")
        lines.append(f"- **Workload-aware governance:** safe={gv['workload_aware_governance'].get('safe', 0)}, "
                      f"warning={gv['workload_aware_governance'].get('warning', 0)}, "
                      f"revalidate={gv['workload_aware_governance'].get('requires_revalidation', 0)}, "
                      f"blocked={gv['workload_aware_governance'].get('blocked', 0)}")
        lines.append(f"- **Stale-cache regression improved:** {gv['stale_regression_improved']}")
        lines.append(f"- **Generic hard blocks/revalidations:** {gv['generic_hard_blocks_or_revalidations']}")
        lines.append(f"- **Workload-aware hard blocks/revalidations:** {gv['workload_aware_hard_blocks_or_revalidations']}")

    # Per-scenario detailed results
    lines.append("\n---\n")
    lines.append("## Per-Scenario Detailed Results\n")
    for scenario in output["scenarios"]:
        lines.append(f"\n### {scenario['scenario_label']}\n")
        lines.append(f"**Workload type:** `{scenario['workload_type']}` | "
                      f"**Hardware:** `{scenario['hardware_profile']}`\n")

        lines.append("#### Strategy Comparison\n")
        lines.append("| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |")
        lines.append("|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|")
        for sid, stats in scenario["strategy_comparison"].items():
            gov = stats["governance_counts"]
            label = next(
                (r["strategy_label"][:25] for r in scenario["results"] if r["strategy_id"] == sid),
                sid[:15],
            )
            lines.append(
                f"| {label} | {stats['average_latency_per_turn_ms']} | "
                f"{stats['throughput_avg_turns_per_sec']} | "
                f"{gov.get('safe', 0)} | {gov.get('warning', 0)} | "
                f"{gov.get('requires_revalidation', 0)} | {gov.get('blocked', 0)} | "
                f"{stats['performance_sacrificed_count']} |"
            )
        lines.append("")

        # Workload-aware details
        wa = next((r for r in scenario["results"] if r["strategy_id"] == "librarian_workload_aware"), None)
        if wa:
            lines.append("#### Workload-Aware Route Selection\n")
            lines.append(f"**Route usage:** {json.dumps(wa['route_usage'], indent=2)}")
            lines.append(f"**Governance:** {json.dumps(wa['governance_counts'], indent=2)}")
            lines.append(f"**Performance sacrificed for evidence:** {wa['performance_sacrificed_count']} turns\n")

    # Scenario C improvement analysis
    lines.append("\n---\n")
    lines.append("## Scenario C (Stale-Cache Governance) Improvement Analysis\n")
    sc = next((s for s in output["scenarios"] if s["scenario_id"] == "C_receipt_generation"), None)
    if sc:
        gv = sc.get("generic_vs_workload_aware", {})
        if gv:
            lines.append(f"- **Generic scheduler latency:** {gv['generic_scheduler_latency_ms']} ms")
            lines.append(f"- **Workload-aware latency:** {gv['workload_aware_latency_ms']} ms")
            lines.append(f"- **Generic hard blocks/revalidations:** {gv['generic_hard_blocks_or_revalidations']}")
            lines.append(f"- **Workload-aware hard blocks/revalidations:** {gv['workload_aware_hard_blocks_or_revalidations']}")
            if gv['stale_regression_improved']:
                lines.append("- **Result:** Scenario C stale-cache regression is **improved** — the workload-aware optimizer uses graduated penalties instead of hard blocking.")
            else:
                lines.append("- **Result:** Scenario C stale-cache regression is **not improved**.")
        else:
            lines.append("- No generic vs workload-aware comparison available for this scenario.")

    # Summary and recommendation
    lines.append("\n---\n")
    lines.append("## Summary\n")

    # Count improvements vs regressions
    improvements = 0
    regressions = 0
    ties = 0
    for scenario in output["scenarios"]:
        gv = scenario.get("generic_vs_workload_aware", {})
        if gv:
            if gv["stale_regression_improved"]:
                improvements += 1
            elif gv["latency_change_pct"] < -10:
                regressions += 1
            else:
                ties += 1

    lines.append(f"- Scenarios where workload-aware improved over generic: **{improvements}**")
    lines.append(f"- Scenarios where workload-aware regressed: **{regressions}**")
    lines.append(f"- Scenarios where roughly tied: **{ties}**")

    lines.append("\n### Where the workload-aware optimizer helped\n")
    for scenario in output["scenarios"]:
        gv = scenario.get("generic_vs_workload_aware", {})
        if gv and gv.get("stale_regression_improved"):
            lines.append(f"- **{scenario['scenario_label']}**: {gv['latency_change_pct']}% latency improvement")

    lines.append("\n### Where governance requires performance sacrifice\n")
    for scenario in output["scenarios"]:
        wa = next((r for r in scenario["results"] if r["strategy_id"] == "librarian_workload_aware"), None)
        if wa and wa["performance_sacrificed_count"] > 0:
            lines.append(f"- **{scenario['scenario_label']}**: {wa['performance_sacrificed_count']} turns where performance was sacrificed for evidence quality")

    lines.append("\n---\n")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    import argparse
    parser = argparse.ArgumentParser(
        description="Router Workload Optimizer — Librarian Workload-Aware Context Route Scheduling"
    )
    parser.add_argument("--config", default=str(DEFAULT_CONFIG_PATH))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT_PATH))
    parser.add_argument("--report", default=str(REPO_ROOT / "reports" / "MAC-WIN-ROUTER-WORKLOAD-OPTIMIZER-1.md"))
    args = parser.parse_args()

    config = load_config(args.config)
    print(f"Loaded config: {args.config}")
    print(f"  Workload profiles: {len(config.get('workload_profiles', {}))}")
    print(f"  Context routes: {len(config.get('context_routes', {}))}")
    print(f"  Hardware profiles: {len(config.get('hardware_profiles', {}))}")
    print(f"  Strategies: {len(config.get('strategies', {}))}")
    print(f"  Scenarios: {len(config.get('scenarios', {}))}")

    output = simulate_all(config)

    os.makedirs(os.path.dirname(args.output), exist_ok=True)
    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2)
    print(f"\nResults JSON: {args.output}")

    report = generate_report(output)
    os.makedirs(os.path.dirname(args.report), exist_ok=True)
    with open(args.report, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"Report: {args.report}")

    # Quick summary
    print("\n=== QUICK SUMMARY ===")
    for scenario in output["scenarios"]:
        comp = scenario["strategy_comparison"]
        wa = comp.get("librarian_workload_aware", {})
        gs = comp.get("prior_generic_scheduler", {})
        wa_lat = wa.get("average_latency_per_turn_ms", 0)
        gs_lat = gs.get("average_latency_per_turn_ms", 0)
        print(f"\n{scenario['scenario_label']}:")
        print(f"  Generic scheduler: {gs_lat:.2f} ms")
        print(f"  Workload-aware:    {wa_lat:.2f} ms")
        if gs_lat > 0:
            impr = (gs_lat - wa_lat) / gs_lat * 100
            print(f"  Change:            {impr:.1f}%")

    print("\nDone.")


if __name__ == "__main__":
    main()
