"""
Workload profiles for the advisory stub.

Contains governance-mandated routes, workload classifications,
evidence requirements, model preferences, and route cost tables.
All data derived from:
  - MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 (governance mandates)
  - MAC/WIN-ROUTER-CONTEXT-MEASURE-1 (measured costs)
  - MAC/WIN-ROUTER-CONTEXT-RUNTIME-CONTRACT-1 (enums, invariants)
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Contract enums (replicated from contract v0.1 for standalone operation)
# ---------------------------------------------------------------------------

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
    "weak_lan_runtime_node", "degraded_node", "future_stronger_gpu_node",
]

FORBIDDEN_ACTIONS = [
    "start_process", "stop_process", "select_model", "execute_model",
    "modify_router_config", "modify_model_profiles",
    "open_network_listener", "change_bind_host",
    "write_runtime_state", "apply_context_route",
]

FRESHNESS_STATES = [
    "verified_current", "recent_but_unverified", "stale_low_risk",
    "stale_requires_revalidation", "provenance_weak", "blocked_for_task",
]

PROVENANCE_STATES = ["verified", "partially_verified", "weak", "unknown", "blocked"]

# This is the contractual list - it is the canonical set used by the stub
ALL_CONTRACT_ENUMS = {
    "workload_type": WORKLOAD_TYPES,
    "task_risk_level": TASK_RISK_LEVELS,
    "node_health": NODE_HEALTHS,
    "degraded_node_action": DEGRADED_NODE_ACTIONS,
    "governance_outcome": GOVERNANCE_OUTCOMES,
    "selected_context_route": CONTEXT_ROUTES,
    "selected_runtime_profile": RUNTIME_PROFILES,
    "freshness_state": FRESHNESS_STATES,
    "provenance_state": PROVENANCE_STATES,
}

# ---------------------------------------------------------------------------
# Governance-mandated routes (from PROTOTYPE-1)
# ---------------------------------------------------------------------------

GOVERNANCE_MANDATED_ROUTES: dict[str, str] = {
    "receipt_generation": "canonical_evidence_read",
    "sprint_closeout": "canonical_evidence_read",
    "validation": "canonical_evidence_read",
    "agent_handoff": "compressed_recall_packet",
    "runtime_node_qualification": "canonical_evidence_read",
}

# ---------------------------------------------------------------------------
# Workload classifications (from PROTOTYPE-1)
# ---------------------------------------------------------------------------

WORKLOAD_CLASSIFICATIONS: dict[str, list[str]] = {
    "evidence_heavy": [
        "receipt_generation", "sprint_closeout",
        "validation", "runtime_node_qualification",
    ],
    "state_transfer": [
        "agent_handoff", "code_patch_preparation",
    ],
    "continuation": [
        "long_session_continuation", "ui_review_or_design_planning",
        "sprint_planning",
    ],
}

# ---------------------------------------------------------------------------
# Default evidence requirements per workload type
# ---------------------------------------------------------------------------

EVIDENCE_REQUIREMENTS: dict[str, dict] = {
    "receipt_generation": {
        "requires_current_git_state": True,
        "requires_current_test_state": True,
        "requires_canonical_source": True,
        "allows_stale_context": False,
        "acceptable_evidence_types": ["git_status", "test_output", "canonical_doc"],
        "risk_level": "high",
        "performance_sacrificed_for_evidence": True,
    },
    "sprint_closeout": {
        "requires_current_git_state": True,
        "requires_current_test_state": True,
        "requires_canonical_source": False,
        "allows_stale_context": False,
        "acceptable_evidence_types": ["git_status", "test_output"],
        "risk_level": "high",
        "performance_sacrificed_for_evidence": True,
    },
    "validation": {
        "requires_current_git_state": True,
        "requires_current_test_state": True,
        "requires_canonical_source": False,
        "allows_stale_context": False,
        "acceptable_evidence_types": ["git_status", "test_output"],
        "risk_level": "medium",
        "performance_sacrificed_for_evidence": True,
    },
    "code_patch_preparation": {
        "requires_current_git_state": False,
        "requires_current_test_state": False,
        "requires_canonical_source": False,
        "allows_stale_context": False,
        "acceptable_evidence_types": ["recall_packet", "session_state"],
        "risk_level": "medium",
        "performance_sacrificed_for_evidence": False,
    },
    "agent_handoff": {
        "requires_current_git_state": False,
        "requires_current_test_state": False,
        "requires_canonical_source": False,
        "allows_stale_context": True,
        "acceptable_evidence_types": ["recall_packet", "session_state"],
        "risk_level": "low",
        "performance_sacrificed_for_evidence": False,
    },
    "sprint_planning": {
        "requires_current_git_state": False,
        "requires_current_test_state": False,
        "requires_canonical_source": False,
        "allows_stale_context": True,
        "acceptable_evidence_types": ["recent_turn", "session_state"],
        "risk_level": "low",
        "performance_sacrificed_for_evidence": False,
    },
    "long_session_continuation": {
        "requires_current_git_state": False,
        "requires_current_test_state": False,
        "requires_canonical_source": False,
        "allows_stale_context": True,
        "acceptable_evidence_types": ["recent_turn", "session_state"],
        "risk_level": "low",
        "performance_sacrificed_for_evidence": False,
    },
    "runtime_node_qualification": {
        "requires_current_git_state": False,
        "requires_current_test_state": False,
        "requires_canonical_source": False,
        "allows_stale_context": False,
        "acceptable_evidence_types": ["git_status", "node_health", "canonical_doc"],
        "risk_level": "high",
        "performance_sacrificed_for_evidence": True,
    },
    "ui_review_or_design_planning": {
        "requires_current_git_state": False,
        "requires_current_test_state": False,
        "requires_canonical_source": False,
        "allows_stale_context": True,
        "acceptable_evidence_types": ["recent_turn", "design_doc"],
        "risk_level": "low",
        "performance_sacrificed_for_evidence": False,
    },
}

# ---------------------------------------------------------------------------
# Model preferences per task class
# ---------------------------------------------------------------------------

MODEL_PREFERENCES: dict[str, dict] = {
    "general_advisory": {
        "model_profile": "phi-4",
        "description": "2.32 GB Q4_K_M. General advisory. Clean output.",
        "limitations": "2.32 GB Q4_K_M. General advisory. Clean output.",
    },
    "code_advisory": {
        "model_profile": "qwen-coder",
        "description": "1.76 GB Q8_0. Best for code tasks.",
        "limitations": "1.76 GB Q8_0. Best for code tasks.",
    },
}

TASK_CLASS_FOR_WORKLOAD: dict[str, str] = {
    "sprint_planning": "general_advisory",
    "sprint_closeout": "general_advisory",
    "receipt_generation": "general_advisory",
    "validation": "general_advisory",
    "code_patch_preparation": "code_advisory",
    "agent_handoff": "general_advisory",
    "long_session_continuation": "general_advisory",
    "runtime_node_qualification": "general_advisory",
    "ui_review_or_design_planning": "general_advisory",
}

# ---------------------------------------------------------------------------
# Route cost estimates (from MEASURE-1 measured data)
# ---------------------------------------------------------------------------

ROUTE_LATENCIES: dict[str, float] = {
    "canonical_evidence_read": 70.9,
    "recomputation_from_source": 500.0,
    "compressed_recall_packet": 4.18,
    "ram_cache": 0.28,
    "ssd_cache": 0.28,
    "recent_turn_window": 0.30,
    "remote_windows_runtime_cache": 4016.0,
}

# ---------------------------------------------------------------------------
# Health/backend defaults
# ---------------------------------------------------------------------------

DEFAULT_NODE_HEALTH = "stopped"  # matches current service state
DEFAULT_BACKEND_STATE = "unavailable"  # matches Stopped service
DEFAULT_RUNTIME_PROFILE = "windows_runtime_node"
