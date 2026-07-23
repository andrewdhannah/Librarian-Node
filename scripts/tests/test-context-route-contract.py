#!/usr/bin/env python3
"""
test-context-route-contract.py — Contract tests for context-route object (v0.1)

Validates fixture files against the context-route contract defined in:
  docs/contracts/context-route-contract.md

Non-production. Simulator and test only.

Sprint: MAC/WIN-ROUTER-CONTEXT-CONTRACT-0
"""

import json
import os
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
FIXTURES_DIR = REPO_ROOT / "fixtures" / "context-route"

# ---------------------------------------------------------------------------
# Enumerations from the contract
# ---------------------------------------------------------------------------

VALID_WORKLOAD_TYPES = {
    "sprint_planning",
    "sprint_closeout",
    "receipt_generation",
    "validation",
    "code_patch_preparation",
    "agent_handoff",
    "long_session_continuation",
    "runtime_node_qualification",
    "ui_review_or_design_planning",
}

VALID_CONTEXT_ROUTES = {
    "ram_cache",
    "ssd_cache",
    "remote_windows_runtime_cache",
    "recomputation_from_source",
    "compressed_recall_packet",
    "canonical_evidence_read",
    "hybrid_recall_plus_fresh_evidence",
}

VALID_FRESHNESS_STATES = {
    "verified_current",
    "recent_but_unverified",
    "stale_low_risk",
    "stale_requires_revalidation",
    "provenance_weak",
    "blocked_for_task",
}

VALID_PROVENANCE_STATES = {
    "verified",
    "partially_verified",
    "weak",
    "unknown",
    "blocked",
}

VALID_GOVERNANCE_OUTCOMES = {
    "safe",
    "warning",
    "requires_revalidation",
    "blocked",
}

VALID_RUNTIME_PROFILES = {
    "mac_coordinator",
    "windows_runtime_node",
    "weak_lan_runtime_node",
    "future_stronger_gpu_node",
}

VALID_RECEIPT_RISKS = {"low", "medium", "high"}

# GPU/RDMA/KV claim patterns (case-insensitive)
GPU_CLAIM_PATTERNS = [
    "gpu", "rdma", "kv-cache", "kv cache", "kvcache",
    "nvlink", "pcie rdma", "cuda kernel",
]


# ---------------------------------------------------------------------------
# Test harness
# ---------------------------------------------------------------------------


class TestResult:
    def __init__(self, name: str, passed: bool, detail: str = ""):
        self.name = name
        self.passed = passed
        self.detail = detail

    def __repr__(self):
        status = "PASS" if self.passed else "FAIL"
        msg = f"  [{status}] {self.name}"
        if self.detail:
            msg += f" — {self.detail}"
        return msg


results: list[TestResult] = []


def test(name: str, passed: bool, detail: str = ""):
    results.append(TestResult(name, passed, detail))


def load_fixture(path: Path) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Required fields
# ---------------------------------------------------------------------------

REQUIRED_FIELDS = [
    "route_id",
    "contract_version",
    "workload_type",
    "selected_context_route",
    "selected_runtime_profile",
    "freshness_state",
    "provenance_state",
    "governance_outcome",
    "estimated_latency_ms",
    "performance_sacrificed_for_evidence",
    "reason_selected",
    "alternatives_rejected",
    "evidence_requirements",
    "receipt_summary",
]


def check_required_fields(fixture: dict, fname: str):
    for field in REQUIRED_FIELDS:
        test(
            f"{fname}: required field '{field}' exists",
            field in fixture,
            f"missing: {field}" if field not in fixture else "ok",
        )


# ---------------------------------------------------------------------------
# Enum validations
# ---------------------------------------------------------------------------

ENUM_CHECKS = [
    ("workload_type", VALID_WORKLOAD_TYPES),
    ("selected_context_route", VALID_CONTEXT_ROUTES),
    ("freshness_state", VALID_FRESHNESS_STATES),
    ("provenance_state", VALID_PROVENANCE_STATES),
    ("governance_outcome", VALID_GOVERNANCE_OUTCOMES),
    ("selected_runtime_profile", VALID_RUNTIME_PROFILES),
]


def check_enums(fixture: dict, fname: str):
    for field, valid_set in ENUM_CHECKS:
        val = fixture.get(field, "")
        test(
            f"{fname}: '{field}' is valid enum",
            val in valid_set,
            f"value='{val}', valid={sorted(valid_set)}" if val not in valid_set else f"value='{val}'",
        )


# ---------------------------------------------------------------------------
# Field-specific validations
# ---------------------------------------------------------------------------


def check_field_validations(fixture: dict, fname: str):
    # route_id
    test(
        f"{fname}: route_id is non-empty string",
        isinstance(fixture.get("route_id"), str) and len(fixture.get("route_id", "")) > 0,
    )

    # contract_version
    test(
        f"{fname}: contract_version is '0.1'",
        fixture.get("contract_version") == "0.1",
        f"got: {fixture.get('contract_version')}",
    )

    # estimated_latency_ms
    lat = fixture.get("estimated_latency_ms")
    test(
        f"{fname}: estimated_latency_ms is numeric and >= 0",
        isinstance(lat, (int, float)) and lat >= 0,
        f"got: {lat}",
    )

    # alternatives_rejected
    alts = fixture.get("alternatives_rejected")
    test(
        f"{fname}: alternatives_rejected is a list",
        isinstance(alts, list),
        f"got: {type(alts).__name__}",
    )
    if isinstance(alts, list):
        test(
            f"{fname}: alternatives_rejected has at least one entry",
            len(alts) > 0,
        )
        for i, alt in enumerate(alts):
            test(
                f"{fname}: alternatives_rejected[{i}] has 'route'",
                "route" in alt,
            )
            test(
                f"{fname}: alternatives_rejected[{i}] has 'reason'",
                "reason" in alt,
            )

    # reason_selected
    test(
        f"{fname}: reason_selected is non-empty string",
        isinstance(fixture.get("reason_selected"), str) and len(fixture.get("reason_selected", "")) > 0,
    )

    # receipt_summary
    rs = fixture.get("receipt_summary", {})
    test(f"{fname}: receipt_summary is present", isinstance(rs, dict) and len(rs) > 0)
    test(
        f"{fname}: receipt_summary.label is non-empty",
        isinstance(rs.get("label"), str) and len(rs.get("label", "")) > 0,
    )
    test(
        f"{fname}: receipt_summary.detail is non-empty",
        isinstance(rs.get("detail"), str) and len(rs.get("detail", "")) > 0,
    )
    test(
        f"{fname}: receipt_summary.risk is valid enum",
        rs.get("risk") in VALID_RECEIPT_RISKS,
        f"got: {rs.get('risk')}",
    )

    # evidence_requirements
    er = fixture.get("evidence_requirements", {})
    test(f"{fname}: evidence_requirements is present", isinstance(er, dict) and len(er) > 0)
    for req_field in ["requires_current_git_state", "requires_current_test_state",
                       "requires_canonical_source", "allows_stale_context"]:
        test(
            f"{fname}: evidence_requirements.{req_field} is boolean",
            isinstance(er.get(req_field), bool),
            f"got: {er.get(req_field)}",
        )


# ---------------------------------------------------------------------------
# Governance invariants
# ---------------------------------------------------------------------------


def check_governance_invariants(fixture: dict, fname: str):
    gov = fixture.get("governance_outcome", "")
    fresh = fixture.get("freshness_state", "")
    prov = fixture.get("provenance_state", "")
    workload = fixture.get("workload_type", "")

    # Invariant 7: blocked outcome consistency
    if gov == "blocked":
        test(
            f"{fname}: blocked outcome implies blocked_for_task or blocked provenance",
            fresh == "blocked_for_task" or prov == "blocked",
            f"freshness={fresh}, provenance={prov}",
        )

    # Invariant 8: safe outcome consistency
    if gov == "safe":
        test(
            f"{fname}: safe outcome implies not blocked_for_task",
            fresh != "blocked_for_task",
            f"freshness={fresh}",
        )
        test(
            f"{fname}: safe outcome implies not blocked provenance",
            prov != "blocked",
            f"provenance={prov}",
        )

    # Invariant 9: receipt_generation provenance rules
    if workload == "receipt_generation":
        if prov == "weak" and gov == "safe":
            test(
                f"{fname}: receipt_generation cannot use weak provenance as safe",
                False,
                f"provenance={prov}, governance={gov}",
            )
        else:
            test(
                f"{fname}: receipt_generation provenance/governance consistent",
                True,
            )

    # Invariant 10: sprint_closeout freshness rules
    if workload == "sprint_closeout":
        er = fixture.get("evidence_requirements", {})
        if er.get("allows_stale_context") is True:
            test(
                f"{fname}: sprint_closeout cannot allow stale context",
                False,
            )
        else:
            test(
                f"{fname}: sprint_closeout evidence requirements consistent",
                True,
            )

    # Invariant 11: runtime_node_qualification staleness rules
    if workload == "runtime_node_qualification":
        if fresh == "stale_requires_revalidation" and gov == "safe":
            test(
                f"{fname}: runtime_node_qualification cannot treat stale as safe",
                False,
                f"freshness={fresh}, governance={gov}",
            )
        else:
            test(
                f"{fname}: runtime_node_qualification staleness consistent",
                True,
            )

    # Invariant 12: performance sacrifice explanation
    if fixture.get("performance_sacrificed_for_evidence") is True:
        test(
            f"{fname}: performance sacrifice has non-empty reason",
            isinstance(fixture.get("reason_selected"), str) and len(fixture.get("reason_selected", "")) > 0,
        )


# ---------------------------------------------------------------------------
# GPU/RDMA/KV claim check for future node
# ---------------------------------------------------------------------------


def check_gpu_claims(fixture: dict, fname: str):
    profile = fixture.get("selected_runtime_profile", "")
    if profile == "future_stronger_gpu_node":
        # Check all string fields for GPU/RDMA/KV claims
        text_fields = [
            "reason_selected",
            "receipt_summary.label",
            "receipt_summary.detail",
        ]
        for tf in text_fields:
            parts = tf.split(".")
            val = fixture
            for p in parts:
                val = val.get(p, {}) if isinstance(val, dict) else ""
            if not isinstance(val, str):
                val = str(val)
            val_lower = val.lower()
            for pattern in GPU_CLAIM_PATTERNS:
                if pattern in val_lower:
                    test(
                        f"{fname}: future node must not contain '{pattern}' claim in {tf}",
                        False,
                        f"found '{pattern}' in '{val}'",
                    )
                    return
        test(
            f"{fname}: future node has no GPU/RDMA/KV claims",
            True,
        )


# ---------------------------------------------------------------------------
# Main test runner
# ---------------------------------------------------------------------------


def run_all_tests():
    if not FIXTURES_DIR.exists():
        print(f"ERROR: Fixtures directory not found: {FIXTURES_DIR}")
        sys.exit(1)

    fixture_files = sorted(FIXTURES_DIR.glob("*.json"))
    if not fixture_files:
        print(f"ERROR: No fixture files found in {FIXTURES_DIR}")
        sys.exit(1)

    print(f"Running contract tests against {len(fixture_files)} fixtures...\n")

    for fpath in fixture_files:
        fname = fpath.stem
        try:
            fixture = load_fixture(fpath)
        except json.JSONDecodeError as e:
            test(f"{fname}: valid JSON", False, str(e))
            continue

        print(f"  Testing: {fname}")
        check_required_fields(fixture, fname)
        check_enums(fixture, fname)
        check_field_validations(fixture, fname)
        check_governance_invariants(fixture, fname)
        check_gpu_claims(fixture, fname)

    # Summary
    passed = sum(1 for r in results if r.passed)
    failed = sum(1 for r in results if not r.passed)
    total = len(results)

    print(f"\n{'='*60}")
    print(f"Results: {passed}/{total} passed, {failed} failed")
    print(f"{'='*60}")

    if failed > 0:
        print(f"\nFailed tests:")
        for r in results:
            if not r.passed:
                print(f"  {r}")

    print()
    return failed == 0


if __name__ == "__main__":
    success = run_all_tests()
    sys.exit(0 if success else 1)
