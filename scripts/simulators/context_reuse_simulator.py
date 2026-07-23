#!/usr/bin/env python3
"""
context_reuse_simulator.py — DualPath-Inspired Context Reuse Scheduling Simulator

EXPLORATORY RESEARCH ONLY.
No production cache behavior. No GPU/RDMA/KV acceleration claims.

Models agentic context reuse across five cache/reuse paths for The Librarian.
Compares six scheduling strategies across five workload scenarios.
"""

import json
import os
import sys
import math
from copy import deepcopy
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
DEFAULT_CONFIG_PATH = REPO_ROOT / "config" / "context_reuse_simulator.default.json"
DEFAULT_OUTPUT_PATH = REPO_ROOT / "reports" / "context-reuse-simulator-results.json"

PATHS_ORDER = [
    "ram_cache",
    "ssd_cache",
    "remote_windows_runtime_cache",
    "recomputation",
    "compressed_recall_packet",
]

STRATEGIES_ORDER = [
    "always_ram",
    "always_ssd",
    "always_remote",
    "always_recompute",
    "always_recall_packet",
    "local_scheduler",
]

# ---------------------------------------------------------------------------
# Config loading
# ---------------------------------------------------------------------------


def load_config(path: str | None = None) -> dict[str, Any]:
    path = path or str(DEFAULT_CONFIG_PATH)
    with open(path, "r", encoding="utf-8") as f:
        cfg = json.load(f)
    return cfg


# ---------------------------------------------------------------------------
# Latency estimation
# ---------------------------------------------------------------------------


def estimate_path_latency(
    path_id: str,
    path_cfg: dict,
    context_tokens: int,
    append_tokens: int,
    is_cache_hit: bool,
    freshness_ticks: int,
    provenance_verified: bool,
    freshness_checks_enabled: bool,
    provenance_checks_enabled: bool,
    scheduler_weights: dict,
    scenario_cfg: dict,
    turn_number: int,
) -> float:
    """Compute estimated latency for a single path on a single turn."""
    cfg = path_cfg

    # Base latency
    latency = cfg["base_latency_ms"]

    # Context transfer cost
    latency += context_tokens * cfg.get("context_transfer_cost_per_token_ms", 0)

    # Append processing cost
    latency += append_tokens * cfg.get("append_processing_cost_per_token_ms", 0)

    # Cache miss penalty
    if not is_cache_hit:
        latency += cfg.get("cache_miss_penalty_ms", 0)

    # Freshness penalty
    if freshness_checks_enabled and cfg.get("supports_freshness_check", False):
        decay = scheduler_weights.get("freshness_tick_penalty", 1.0)
        penalty = freshness_ticks * decay * cfg.get("freshness_penalty_per_tick_ms", 0)
        latency += penalty

    # Provenance penalty
    if provenance_checks_enabled and cfg.get("supports_provenance_check", False) and not provenance_verified:
        w = scheduler_weights.get("provenance_penalty", 1.5)
        latency += w * cfg.get("provenance_penalty_if_unverified_ms", 0)

    # Network cost
    if cfg.get("requires_network", False):
        lan_base = scenario_cfg.get("lan_latency_base_ms", cfg.get("lan_latency_ms", 35))
        lan_jitter = scenario_cfg.get("lan_jitter_ms", cfg.get("lan_jitter_ms", 15))
        is_unstable = scenario_cfg.get("lan_quality", "stable") == "unstable"
        if is_unstable:
            jitter_weight = scheduler_weights.get("network_jitter_weight", 1.2)
            # Deterministic jitter: use turn_number to create reproducible variation
            jitter = math.sin(turn_number * 0.7) * lan_jitter * jitter_weight
            latency += lan_base + jitter
        else:
            latency += lan_base

    # Compression / decompression for recall packet
    if path_id == "compressed_recall_packet":
        latency += context_tokens * cfg.get("compression_cost_per_token_ms", 0)
        latency += context_tokens * cfg.get("decompression_cost_per_token_ms", 0)

    return latency


# ---------------------------------------------------------------------------
# Path availability
# ---------------------------------------------------------------------------


def path_is_available(path_id: str, path_cfg: dict, config: dict, scenario_cfg: dict) -> bool:
    """Check if a path is available given the scenario context."""
    if not path_cfg.get("always_available", True):
        # e.g. remote path may be unavailable
        if path_id == "remote_windows_runtime_cache":
            lan_quality = scenario_cfg.get("lan_quality", "stable")
            if lan_quality == "down":
                return False
    return True


# ---------------------------------------------------------------------------
# Strategy decision functions
# ---------------------------------------------------------------------------


def decide_always_ram(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    path_states: dict,
    freshness_checks_enabled: bool, provenance_checks_enabled: bool,
) -> tuple[str, float, dict]:
    """Always use RAM cache if available; fall back to recomputation."""
    pcfg = config["paths"]["ram_cache"]
    hit = path_states["ram_cache"]["hit"]
    lat = estimate_path_latency(
        "ram_cache", pcfg, context_tokens, append_tokens, hit,
        path_states["ram_cache"]["freshness_ticks"],
        path_states["ram_cache"]["provenance_verified"],
        freshness_checks_enabled, provenance_checks_enabled,
        config["scheduler"]["weights"], scenario_cfg, turn_number,
    )
    if hit:
        return "ram_cache", lat, {}
    # fall back to recompute
    rcfg = config["paths"]["recomputation"]
    rlat = estimate_path_latency(
        "recomputation", rcfg, context_tokens, append_tokens, True,
        0, True, False, False,
        config["scheduler"]["weights"], scenario_cfg, turn_number,
    )
    return "recomputation", rlat, {"fallback": "ram_cache_miss"}


def decide_always_ssd(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    path_states: dict,
    freshness_checks_enabled: bool, provenance_checks_enabled: bool,
) -> tuple[str, float, dict]:
    """Use SSD cache from turn 2 onward; fall back to recomputation."""
    if turn_number == 1:
        # First turn: use recomputation
        rcfg = config["paths"]["recomputation"]
        rlat = estimate_path_latency(
            "recomputation", rcfg, context_tokens, append_tokens, True,
            0, True, False, False,
            config["scheduler"]["weights"], scenario_cfg, turn_number,
        )
        return "recomputation", rlat, {"reason": "first_turn_no_cache"}
    pcfg = config["paths"]["ssd_cache"]
    hit = path_states["ssd_cache"]["hit"]
    lat = estimate_path_latency(
        "ssd_cache", pcfg, context_tokens, append_tokens, hit,
        path_states["ssd_cache"]["freshness_ticks"],
        path_states["ssd_cache"]["provenance_verified"],
        freshness_checks_enabled, provenance_checks_enabled,
        config["scheduler"]["weights"], scenario_cfg, turn_number,
    )
    if hit:
        return "ssd_cache", lat, {}
    rcfg = config["paths"]["recomputation"]
    rlat = estimate_path_latency(
        "recomputation", rcfg, context_tokens, append_tokens, True,
        0, True, False, False,
        config["scheduler"]["weights"], scenario_cfg, turn_number,
    )
    return "recomputation", rlat, {"fallback": "ssd_cache_miss"}


def decide_always_remote(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    path_states: dict,
    freshness_checks_enabled: bool, provenance_checks_enabled: bool,
) -> tuple[str, float, dict]:
    """Always route through remote Windows runtime cache."""
    pcfg = config["paths"]["remote_windows_runtime_cache"]
    if not path_is_available("remote_windows_runtime_cache", pcfg, config, scenario_cfg):
        rcfg = config["paths"]["recomputation"]
        rlat = estimate_path_latency(
            "recomputation", rcfg, context_tokens, append_tokens, True,
            0, True, False, False,
            config["scheduler"]["weights"], scenario_cfg, turn_number,
        )
        return "recomputation", rlat, {"fallback": "remote_unavailable"}
    hit = path_states["remote_windows_runtime_cache"]["hit"]
    lat = estimate_path_latency(
        "remote_windows_runtime_cache", pcfg, context_tokens, append_tokens, hit,
        path_states["remote_windows_runtime_cache"]["freshness_ticks"],
        path_states["remote_windows_runtime_cache"]["provenance_verified"],
        freshness_checks_enabled, provenance_checks_enabled,
        config["scheduler"]["weights"], scenario_cfg, turn_number,
    )
    return "remote_windows_runtime_cache", lat, {}


def decide_always_recompute(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    path_states: dict,
    freshness_checks_enabled: bool, provenance_checks_enabled: bool,
) -> tuple[str, float, dict]:
    """Never use any cache."""
    rcfg = config["paths"]["recomputation"]
    lat = estimate_path_latency(
        "recomputation", rcfg, context_tokens, append_tokens, True,
        0, True, False, False,
        config["scheduler"]["weights"], scenario_cfg, turn_number,
    )
    return "recomputation", lat, {}


def decide_always_recall_packet(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    path_states: dict,
    freshness_checks_enabled: bool, provenance_checks_enabled: bool,
) -> tuple[str, float, dict]:
    """Always use compressed recall packet."""
    pcfg = config["paths"]["compressed_recall_packet"]
    hit = path_states["compressed_recall_packet"]["hit"]
    lat = estimate_path_latency(
        "compressed_recall_packet", pcfg, context_tokens, append_tokens, hit,
        path_states["compressed_recall_packet"]["freshness_ticks"],
        path_states["compressed_recall_packet"]["provenance_verified"],
        freshness_checks_enabled, provenance_checks_enabled,
        config["scheduler"]["weights"], scenario_cfg, turn_number,
    )
    return "compressed_recall_packet", lat, {}


def decide_local_scheduler(
    config: dict, scenario_cfg: dict, turn_number: int,
    context_tokens: int, append_tokens: int,
    path_states: dict,
    freshness_checks_enabled: bool, provenance_checks_enabled: bool,
) -> tuple[str, float, dict]:
    """Choose best available path based on estimated latency, freshness, and provenance.

    Simple, explainable scheduler:
    1. Enumerate all available paths
    2. Estimate latency for each
    3. Block paths that fail freshness or provenance governance checks
    4. Pick the path with the lowest adjusted estimated latency
    """
    candidates = []
    blocked_reasons = {}

    for pid in PATHS_ORDER:
        pcfg = config["paths"][pid]
        if not path_is_available(pid, pcfg, config, scenario_cfg):
            blocked_reasons[pid] = "path_unavailable"
            continue

        freshness_ok = True
        provenance_ok = True

        # Freshness check
        if freshness_checks_enabled and pcfg.get("supports_freshness_check", False):
            max_stale = config["scheduler"].get("max_freshness_ticks_before_stale", 10)
            if path_states[pid]["freshness_ticks"] > max_stale:
                freshness_ok = False

        # Provenance check
        if provenance_checks_enabled and pcfg.get("supports_provenance_check", False):
            if not path_states[pid]["provenance_verified"]:
                provenance_ok = False

        if not freshness_ok:
            blocked_reasons[pid] = "freshness_stale"
            continue
        if not provenance_ok:
            blocked_reasons[pid] = "provenance_unverified"
            continue

        hit = path_states[pid]["hit"]
        lat = estimate_path_latency(
            pid, pcfg, context_tokens, append_tokens, hit,
            path_states[pid]["freshness_ticks"],
            path_states[pid]["provenance_verified"],
            freshness_checks_enabled, provenance_checks_enabled,
            config["scheduler"]["weights"], scenario_cfg, turn_number,
        )
        candidates.append((lat, pid))

    if not candidates:
        # Fallback to recomputation (always available, always fresh)
        rcfg = config["paths"]["recomputation"]
        lat = estimate_path_latency(
            "recomputation", rcfg, context_tokens, append_tokens, True,
            0, True, False, False,
            config["scheduler"]["weights"], scenario_cfg, turn_number,
        )
        return "recomputation", lat, {"fallback": "all_paths_blocked_or_unavailable",
                                       "blocked": blocked_reasons}

    candidates.sort(key=lambda x: x[0])
    chosen = candidates[0][1]
    alt = [p for _, p in candidates[1:]]
    return chosen, candidates[0][0], {"alternatives_considered": alt, "blocked": blocked_reasons}


# ---------------------------------------------------------------------------
# Decision dispatch
# ---------------------------------------------------------------------------

STRATEGY_DECISION_MAP = {
    "always_ram": decide_always_ram,
    "always_ssd": decide_always_ssd,
    "always_remote": decide_always_remote,
    "always_recompute": decide_always_recompute,
    "always_recall_packet": decide_always_recall_packet,
    "local_scheduler": decide_local_scheduler,
}


# ---------------------------------------------------------------------------
# Session / scenario simulation
# ---------------------------------------------------------------------------


def simulate_turn(
    config: dict,
    scenario_cfg: dict,
    strategy_id: str,
    turn_number: int,
    path_states: dict,
    freshness_checks_enabled: bool,
    provenance_checks_enabled: bool,
    context_tokens: int,
    append_tokens: int,
) -> dict:
    """Simulate a single turn and return a decision record."""
    decision_fn = STRATEGY_DECISION_MAP[strategy_id]
    chosen_path, latency, meta = decision_fn(
        config, scenario_cfg, turn_number, context_tokens, append_tokens,
        path_states, freshness_checks_enabled, provenance_checks_enabled,
    )

    # Build explanation record
    path_cfg = config["paths"].get(chosen_path, {})
    ps = path_states.get(chosen_path, {})
    alternatives = []
    for pid in PATHS_ORDER:
        if pid != chosen_path:
            pcfg = config["paths"][pid]
            hit = path_states.get(pid, {}).get("hit", False)
            alt_lat = estimate_path_latency(
                pid, pcfg, context_tokens, append_tokens, hit,
                path_states.get(pid, {}).get("freshness_ticks", 0),
                path_states.get(pid, {}).get("provenance_verified", False),
                freshness_checks_enabled, provenance_checks_enabled,
                config["scheduler"]["weights"], scenario_cfg, turn_number,
            )
            alternatives.append({
                "path": pid,
                "estimated_latency_ms": round(alt_lat, 2),
            })
    alternatives.sort(key=lambda x: x["estimated_latency_ms"])

    record = {
        "turn": turn_number,
        "context_tokens": context_tokens,
        "append_tokens": append_tokens,
        "selected_path": chosen_path,
        "selected_path_label": path_cfg.get("label", chosen_path),
        "estimated_latency_ms": round(latency, 2),
        "estimated_cost": round(latency, 2),  # simplified: cost ≈ latency
        "cache_hit": ps.get("hit", True),
        "freshness_ticks": ps.get("freshness_ticks", 0),
        "provenance_verified": ps.get("provenance_verified", True),
        "reason_selected": meta.get("reason", ""),
        "alternatives_rejected": alternatives,
        "fallback": meta.get("fallback", ""),
        "blocked": meta.get("blocked", {}),
    }
    return record


def simulate_session(
    config: dict,
    scenario_cfg: dict,
    strategy_id: str,
    session_id: int = 0,
) -> dict:
    """Simulate one session across all turns."""
    num_turns = scenario_cfg["num_turns"]
    context_tokens = scenario_cfg["reused_context_tokens"]
    append_tokens = scenario_cfg["append_tokens"]
    cache_hit_rate = scenario_cfg["cache_hit_rate"]
    freshness_enabled = scenario_cfg.get("freshness_decay_enabled", False)
    provenance_enabled = scenario_cfg.get("provenance_checks_enabled", False)
    strategy_label = config["strategies"][strategy_id]["label"]

    # Initialize per-path state (deterministic seeded by session_id)
    path_states: dict[str, dict] = {}
    for pid in PATHS_ORDER:
        freshness_key = freshness_enabled
        path_states[pid] = {
            "hit": True,  # will be updated per turn
            "freshness_ticks": 0,
            "provenance_verified": config["scheduler"].get("provenance_verified_default", False),
        }
        if pid == "recomputation":
            path_states[pid]["freshness_ticks"] = 0
            path_states[pid]["provenance_verified"] = True

    freshness_decay = config["scheduler"].get("freshness_decay_per_tick", {})
    stale_after = scenario_cfg.get("stale_after_ticks", 999)
    shared_overlap = scenario_cfg.get("shared_context_overlap", 0.0)

    turns = []
    total_latency = 0.0
    cache_hits = 0
    path_usage: dict[str, int] = {}

    for turn in range(1, num_turns + 1):
        # Determine cache hit deterministically based on turn number and rate
        # Use a deterministic pseudo-random function
        seed = session_id * 10000 + turn
        # Simple deterministic hash
        hit_threshold = int(cache_hit_rate * 10000)
        hit_val = (seed * 1103515245 + 12345) % 10000
        is_hit = hit_val < hit_threshold

        # Update freshness ticks for all paths
        for pid in PATHS_ORDER:
            if freshness_enabled:
                if path_states[pid]["hit"]:
                    decay = freshness_decay.get(pid, 1.0)
                    path_states[pid]["freshness_ticks"] = int(
                        path_states[pid]["freshness_ticks"] * decay + 1
                    )
                else:
                    path_states[pid]["freshness_ticks"] = 0
            else:
                path_states[pid]["freshness_ticks"] = 0

            # Update hit flag
            path_states[pid]["hit"] = is_hit

            # Provenance verification (randomized but deterministic)
            if provenance_enabled:
                prov_val = (seed * 98765 + pid_hash(pid)) % 10000
                path_states[pid]["provenance_verified"] = prov_val < 9000
            else:
                path_states[pid]["provenance_verified"] = True

            # Freshness reset on recomputation
            if pid == "recomputation":
                path_states[pid]["freshness_ticks"] = 0
                path_states[pid]["provenance_verified"] = True
                path_states[pid]["hit"] = True

        # Adjust context size for shared overlap in parallel sessions
        actual_context = context_tokens
        if turn > 1 and shared_overlap > 0:
            overlap_tokens = int(context_tokens * shared_overlap)
            actual_context = context_tokens - overlap_tokens

        record = simulate_turn(
            config, scenario_cfg, strategy_id, turn,
            path_states, freshness_enabled, provenance_enabled,
            actual_context, append_tokens,
        )
        turns.append(record)
        total_latency += record["estimated_latency_ms"]
        if record["cache_hit"]:
            cache_hits += 1
        path_usage[record["selected_path"]] = path_usage.get(record["selected_path"], 0) + 1

    avg_latency = total_latency / num_turns if num_turns > 0 else 0
    throughput = 1000.0 / avg_latency if avg_latency > 0 else 0  # turns per second

    return {
        "session_id": session_id,
        "strategy_id": strategy_id,
        "strategy_label": strategy_label,
        "num_turns": num_turns,
        "total_latency_ms": round(total_latency, 2),
        "average_latency_per_turn_ms": round(avg_latency, 2),
        "throughput_turns_per_sec": round(throughput, 4),
        "cache_hits": cache_hits,
        "cache_misses": num_turns - cache_hits,
        "cache_hit_rate_actual": round(cache_hits / num_turns, 4) if num_turns > 0 else 0,
        "path_usage": path_usage,
        "turns": turns,
    }


def pid_hash(pid: str) -> int:
    """Simple deterministic hash for path id."""
    h = 0
    for c in pid:
        h = h * 31 + ord(c)
    return h


def simulate_scenario(
    config: dict,
    scenario_id: str,
    scenario_cfg: dict,
) -> dict:
    """Run all strategies for a single scenario."""
    num_sessions = scenario_cfg.get("num_sessions", 1)
    scenario_label = scenario_cfg["label"]
    scenario_results = []

    for strategy_id in STRATEGIES_ORDER:
        session_results = []
        for sid in range(num_sessions):
            session_result = simulate_session(config, scenario_cfg, strategy_id, session_id=sid)
            session_results.append(session_result)

        # Aggregate across sessions
        total_lat = sum(s["total_latency_ms"] for s in session_results)
        total_turns = sum(s["num_turns"] for s in session_results)
        avg_lat = total_lat / total_turns if total_turns > 0 else 0
        total_hits = sum(s["cache_hits"] for s in session_results)
        total_misses = sum(s["cache_misses"] for s in session_results)
        throughputs = [s["throughput_turns_per_sec"] for s in session_results]
        avg_throughput = sum(throughputs) / len(throughputs) if throughputs else 0

        combined_path_usage: dict[str, int] = {}
        for s in session_results:
            for pid, cnt in s["path_usage"].items():
                combined_path_usage[pid] = combined_path_usage.get(pid, 0) + cnt

        scenario_results.append({
            "strategy_id": strategy_id,
            "strategy_label": config["strategies"][strategy_id]["label"],
            "num_sessions": num_sessions,
            "total_turns": total_turns,
            "total_latency_ms": round(total_lat, 2),
            "average_latency_per_turn_ms": round(avg_lat, 2),
            "throughput_avg_turns_per_sec": round(avg_throughput, 4),
            "cache_hits": total_hits,
            "cache_misses": total_misses,
            "cache_hit_rate_actual": round(total_hits / (total_hits + total_misses), 4) if (total_hits + total_misses) > 0 else 0,
            "path_usage": combined_path_usage,
            "sessions": session_results,
        })

    # Strategy comparison table
    comparison = {}
    for sr in scenario_results:
        comparison[sr["strategy_id"]] = {
            "average_latency_per_turn_ms": sr["average_latency_per_turn_ms"],
            "throughput_avg_turns_per_sec": sr["throughput_avg_turns_per_sec"],
            "cache_hit_rate_actual": sr["cache_hit_rate_actual"],
            "total_latency_ms": sr["total_latency_ms"],
        }

    # Scheduler vs best baseline
    scheduler_result = next((sr for sr in scenario_results if sr["strategy_id"] == "local_scheduler"), None)
    baselines = [sr for sr in scenario_results if sr["strategy_id"] != "local_scheduler"]
    improvements = {}
    if scheduler_result and baselines:
        sched_lat = scheduler_result["average_latency_per_turn_ms"]
        for br in baselines:
            bl_lat = br["average_latency_per_turn_ms"]
            if bl_lat > 0:
                impr = round((bl_lat - sched_lat) / bl_lat * 100, 2)
            else:
                impr = 0.0
            improvements[br["strategy_id"]] = {
                "baseline_latency_ms": bl_lat,
                "scheduler_latency_ms": sched_lat,
                "improvement_pct": impr,
            }

    return {
        "scenario_id": scenario_id,
        "scenario_label": scenario_label,
        "assumptions": {
            "num_sessions": num_sessions,
            "num_turns": scenario_cfg["num_turns"],
            "reused_context_tokens": scenario_cfg["reused_context_tokens"],
            "append_tokens": scenario_cfg["append_tokens"],
            "cache_hit_rate": scenario_cfg["cache_hit_rate"],
            "freshness_decay_enabled": scenario_cfg.get("freshness_decay_enabled", False),
            "provenance_checks_enabled": scenario_cfg.get("provenance_checks_enabled", False),
            "lan_quality": scenario_cfg.get("lan_quality", "stable"),
        },
        "strategy_comparison": comparison,
        "scheduler_vs_baseline": improvements,
        "results": scenario_results,
    }


def simulate_all(config: dict) -> dict:
    """Run all scenarios."""
    scenarios = config["scenarios"]
    output = {
        "metadata": {
            "title": "Context Reuse Simulator Results",
            "subtitle": "DualPath-Inspired Context Reuse Scheduling for The Librarian",
            "honesty": "Exploratory research only. No production cache behavior. No GPU/RDMA/KV acceleration claims.",
            "simulator_version": "1.0.0",
            "config_file": str(DEFAULT_CONFIG_PATH),
            "workload_assumptions": {
                "avg_reused_context_tokens": config["workload"]["avg_reused_context_tokens"],
                "avg_append_tokens": config["workload"]["avg_append_tokens"],
                "cache_hit_rate": config["workload"]["cache_hit_rate"],
                "shape": config["workload"]["shape_description"],
                "bottleneck": config["workload"]["expected_bottleneck"],
            },
        },
        "scenarios": [],
    }
    for sid, scfg in scenarios.items():
        result = simulate_scenario(config, sid, scfg)
        output["scenarios"].append(result)
    return output


# ---------------------------------------------------------------------------
# Reporting helpers
# ---------------------------------------------------------------------------


def generate_human_report(output: dict) -> str:
    """Generate a human-readable Markdown report from the simulation output."""
    lines = []
    lines.append("# Context Reuse Simulator — Research Report\n")
    lines.append(f"**{output['metadata']['title']}**\n")
    lines.append(f"*{output['metadata']['subtitle']}*\n")
    lines.append(f"\n> {output['metadata']['honesty']}\n")
    lines.append("\n---\n")

    # Workload assumptions
    wl = output["metadata"]["workload_assumptions"]
    lines.append("## Workload Assumptions\n")
    lines.append(f"- Average reused context: **{wl['avg_reused_context_tokens']} tokens**")
    lines.append(f"- Average append length: **{wl['avg_append_tokens']} tokens**")
    lines.append(f"- Approximate cache-hit rate: **{wl['cache_hit_rate']}**")
    lines.append(f"- Workload shape: {wl['shape']}")
    lines.append(f"- Expected bottleneck: {wl['bottleneck']}")
    lines.append("")

    for scenario in output["scenarios"]:
        lines.append(f"\n## {scenario['scenario_label']}\n")
        lines.append(f"**Scenario ID:** `{scenario['scenario_id']}`\n")

        sa = scenario["assumptions"]
        lines.append("### Assumptions\n")
        lines.append(f"- Sessions: {sa['num_sessions']}, Turns per session: {sa['num_turns']}")
        lines.append(f"- Context tokens: {sa['reused_context_tokens']}, Append tokens: {sa['append_tokens']}")
        lines.append(f"- Cache hit rate: {sa['cache_hit_rate']}")
        lines.append(f"- Freshness decay: {sa['freshness_decay_enabled']}, Provenance checks: {sa['provenance_checks_enabled']}")
        lines.append(f"- LAN quality: {sa['lan_quality']}")
        lines.append("")

        # Strategy comparison table
        lines.append("### Strategy Comparison\n")
        lines.append("| Strategy | Avg Latency (ms) | Throughput (turns/s) | Cache Hit Rate |")
        lines.append("|----------|-----------------:|---------------------:|---------------:|")
        comp = scenario["strategy_comparison"]
        for sid, stats in comp.items():
            label = next(
                (r["strategy_label"] for r in scenario["results"] if r["strategy_id"] == sid),
                sid,
            )
            lines.append(
                f"| {label} | {stats['average_latency_per_turn_ms']} | "
                f"{stats['throughput_avg_turns_per_sec']} | {stats['cache_hit_rate_actual']} |"
            )
        lines.append("")

        # Scheduler vs baseline
        imp = scenario.get("scheduler_vs_baseline", {})
        if imp:
            lines.append("### Scheduler vs Baseline\n")
            lines.append("| Baseline | Baseline Lat (ms) | Scheduler Lat (ms) | Improvement |")
            lines.append("|----------|------------------:|-------------------:|------------:|")
            sched_lat = comp.get("local_scheduler", {}).get("average_latency_per_turn_ms", 0)
            for bid, stats in imp.items():
                bl_label = next(
                    (r["strategy_label"] for r in scenario["results"] if r["strategy_id"] == bid),
                    bid,
                )
                impr = stats["improvement_pct"]
                marker = "✅" if impr > 5 else ("⚠️" if impr > 0 else "❌")
                lines.append(
                    f"| {bl_label} | {stats['baseline_latency_ms']} | "
                    f"{stats['scheduler_latency_ms']} | {impr}% {marker} |"
                )
            lines.append("")

    lines.append("\n## Summary\n")
    lines.append("### Where the scheduler helped\n")
    for scenario in output["scenarios"]:
        imp = scenario.get("scheduler_vs_baseline", {})
        sid = scenario["scenario_id"]
        positive = {k: v for k, v in imp.items() if v["improvement_pct"] > 5}
        negative = {k: v for k, v in imp.items() if v["improvement_pct"] <= 0}
        neutral = {k: v for k, v in imp.items() if 0 < v["improvement_pct"] <= 5}
        lines.append(f"\n**{scenario['scenario_label']}**")
        if positive:
            lines.append(f"- Positive vs: {', '.join(f'{k} ({v["improvement_pct"]}%)' for k, v in positive.items())}")
        if neutral:
            lines.append(f"- Neutral vs: {', '.join(k for k in neutral)}")
        if negative:
            lines.append(f"- Negative vs: {', '.join(f'{k} ({v["improvement_pct"]}%)' for k, v in negative.items())}")

    lines.append("\n---\n")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    import argparse
    parser = argparse.ArgumentParser(
        description="Context Reuse Simulator — DualPath-Inspired Context Reuse Scheduling"
    )
    parser.add_argument(
        "--config", default=str(DEFAULT_CONFIG_PATH),
        help=f"Config file path (default: {DEFAULT_CONFIG_PATH})",
    )
    parser.add_argument(
        "--output", default=str(DEFAULT_OUTPUT_PATH),
        help=f"Output JSON path (default: {DEFAULT_OUTPUT_PATH})",
    )
    parser.add_argument(
        "--human-report", default="",
        help="Optional path for human-readable Markdown report",
    )
    args = parser.parse_args()

    config = load_config(args.config)
    print(f"Loaded config from: {args.config}")
    print(f"  Paths: {len(config['paths'])}")
    print(f"  Strategies: {len(config['strategies'])}")
    print(f"  Scenarios: {len(config['scenarios'])}")

    output = simulate_all(config)

    # Save JSON output
    os.makedirs(os.path.dirname(args.output), exist_ok=True)
    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2)
    print(f"\nResults written to: {args.output}")

    # Generate human report if requested
    if args.human_report:
        report = generate_human_report(output)
        os.makedirs(os.path.dirname(args.human_report) or ".", exist_ok=True)
        with open(args.human_report, "w", encoding="utf-8") as f:
            f.write(report)
        print(f"Human report written to: {args.human_report}")

    # Print quick summary
    print("\n=== QUICK SUMMARY ===")
    for scenario in output["scenarios"]:
        comp = scenario["strategy_comparison"]
        sched = comp.get("local_scheduler", {})
        best_baseline = min(
            (v for k, v in comp.items() if k != "local_scheduler"),
            key=lambda x: x["average_latency_per_turn_ms"],
        )
        print(f"\n{scenario['scenario_label']}:")
        print(f"  Scheduler avg latency: {sched.get('average_latency_per_turn_ms', 'N/A'):.2f} ms")
        print(f"  Best baseline avg latency: {best_baseline['average_latency_per_turn_ms']:.2f} ms")

    print("\nDone.")


if __name__ == "__main__":
    main()
