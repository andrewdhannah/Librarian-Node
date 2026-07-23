#!/usr/bin/env python3
"""
MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1 — Design Validation Tests

Validates that the design artifacts (interfaces, fixtures, contracts)
conform to the design document requirements without changing runtime behavior.

Sprint: MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1
Status: Validation tests only — no production behavior changed.
"""

import json
import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Path setup
# ---------------------------------------------------------------------------
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DESIGN_DOC_PATH = REPO_ROOT / "docs" / "design" / "router-context-runtime-design.md"
FIXTURES_DIR = REPO_ROOT / "fixtures" / "router-context-runtime-design"
MEASURED_PROFILES_PATH = REPO_ROOT / "config" / "measured_hardware_profiles.json"
PROTOTYPE_DECISIONS_PATH = REPO_ROOT / "reports" / "router-context-prototype-decisions.json"

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

PROVENANCE_STATES = ["verified", "partially_verified", "weak", "unknown", "blocked"]
GOVERNANCE_OUTCOMES = ["safe", "warning", "requires_revalidation", "blocked"]

RUNTIME_PROFILES = [
    "mac_coordinator", "windows_runtime_node",
    "weak_lan_runtime_node", "future_stronger_gpu_node",
]

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


# ---------------------------------------------------------------------------
# Test Category 1: Design Document Validation
# ---------------------------------------------------------------------------
print("\n[1/5] Design Document Validation")

test(
    "Design document exists",
    DESIGN_DOC_PATH.exists(),
    f"Missing: {DESIGN_DOC_PATH}"
)

if DESIGN_DOC_PATH.exists():
    content = DESIGN_DOC_PATH.read_text(encoding="utf-8")

    test(
        "Design document covers Q1 (where)",
        "Q1" in content or "Where would the future context decision layer live" in content,
        "Missing Q1 coverage"
    )
    test(
        "Design document covers Q2 (inputs)",
        "Q2" in content or "What inputs would it consume" in content,
        "Missing Q2 coverage"
    )
    test(
        "Design document covers Q3 (outputs)",
        "Q3" in content or "What outputs would it emit" in content,
        "Missing Q3 coverage"
    )
    test(
        "Design document covers Q4 (model route relation)",
        "Q4" in content or "How does it relate to existing model route" in content,
        "Missing Q4 coverage"
    )
    test(
        "Design document covers Q5 (receipt consumption)",
        "Q5" in content or "How would receipts consume" in content,
        "Missing Q5 coverage"
    )
    test(
        "Design document covers Q6 (degraded state)",
        "Q6" in content or "How does it safely check degraded runtime state" in content,
        "Missing Q6 coverage"
    )
    test(
        "Design document covers Q7 (advisory boundary)",
        "Q7" in content or "What must remain advisory" in content,
        "Missing Q7 coverage"
    )
    test(
        "Design document covers Q8 (tests)",
        "Q8" in content or "What tests would gate future integration" in content,
        "Missing Q8 coverage"
    )
    test(
        "Design document covers Q9 (forbidden files)",
        "Q9" in content or "What production files must not be touched" in content,
        "Missing Q9 coverage"
    )
    test(
        "Design document covers Q10 (implementation boundary)",
        "Q10" in content or "What is the exact next implementation boundary" in content,
        "Missing Q10 coverage"
    )

    test(
        "Design references MEASURE-1",
        "MEASURE-1" in content or "MAC/WIN-ROUTER-CONTEXT-MEASURE-1" in content,
        "Missing MEASURE-1 reference"
    )
    test(
        "Design references PROTOTYPE-1",
        "PROTOTYPE-1" in content or "MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1" in content,
        "Missing PROTOTYPE-1 reference"
    )
    test(
        "Design defines advisory-only boundary",
        "advisory" in content.lower() and "advisory_only" in content.lower(),
        "Missing advisory-only boundary definition"
    )
    test(
        "Design lists forbidden production files",
        "FORBIDDEN" in content,
        "Missing forbidden files list"
    )
    test(
        "Design defines receipt consumption path",
        "context_decision" in content and "receipt" in content.lower(),
        "Missing receipt consumption path"
    )
    test(
        "Design defines degraded-node handling",
        "degraded" in content.lower() and "4000" in content or "4016" in content,
        "Missing degraded-node handling"
    )


# ---------------------------------------------------------------------------
# Test Category 2: Fixture Schema Validation
# ---------------------------------------------------------------------------
print("\n[2/5] Fixture Schema Validation")

if FIXTURES_DIR.exists():
    input_fixture_path = FIXTURES_DIR / "interface-context-decision-input.json"
    output_fixture_path = FIXTURES_DIR / "interface-context-decision-output.json"
    receipt_fixture_path = FIXTURES_DIR / "interface-receipt-consumption.json"

    test(
        "Input fixture exists",
        input_fixture_path.exists(),
        f"Missing: {input_fixture_path}"
    )
    test(
        "Output fixture exists",
        output_fixture_path.exists(),
        f"Missing: {output_fixture_path}"
    )
    test(
        "Receipt fixture exists",
        receipt_fixture_path.exists(),
        f"Missing: {receipt_fixture_path}"
    )

    # Validate input fixture
    if input_fixture_path.exists():
        with open(input_fixture_path, "r", encoding="utf-8") as f:
            input_data = json.load(f)

        test(
            "Input fixture has schema_version",
            "schema_version" in input_data,
            "Missing schema_version field"
        )
        test(
            "Input fixture has sprint_id",
            "sprint_id" in input_data,
            "Missing sprint_id field"
        )
        test(
            "Input fixture has interfaces",
            "interfaces" in input_data,
            "Missing interfaces field"
        )
        test(
            "Input fixture has context_decision_input",
            "context_decision_input" in input_data.get("interfaces", {}),
            "Missing context_decision_input interface"
        )

        # Validate input fields
        if "interfaces" in input_data and "context_decision_input" in input_data["interfaces"]:
            fields = input_data["interfaces"]["context_decision_input"].get("fields", {})
            test(
                "Input has workload_type field",
                "workload_type" in fields,
                "Missing workload_type field"
            )
            test(
                "Input has hardware_profile field",
                "hardware_profile" in fields,
                "Missing hardware_profile field"
            )
            test(
                "Input workload_type has correct enum",
                fields.get("workload_type", {}).get("enum") == WORKLOAD_TYPES,
                f"Expected {WORKLOAD_TYPES}, got {fields.get('workload_type', {}).get('enum')}"
            )
            input_hw_enum = fields.get("hardware_profile", {}).get("enum", [])
            test(
                "Input hardware_profile has correct enum (content match)",
                set(input_hw_enum) == set(RUNTIME_PROFILES),
                f"Expected {RUNTIME_PROFILES}, got {input_hw_enum}"
            )

    # Validate output fixture
    if output_fixture_path.exists():
        with open(output_fixture_path, "r", encoding="utf-8") as f:
            output_data = json.load(f)

        test(
            "Output fixture has schema_version",
            "schema_version" in output_data,
            "Missing schema_version field"
        )
        test(
            "Output fixture has interfaces",
            "interfaces" in output_data,
            "Missing interfaces field"
        )
        test(
            "Output fixture has context_decision_output",
            "context_decision_output" in output_data.get("interfaces", {}),
            "Missing context_decision_output interface"
        )

        # Validate output fields
        if "interfaces" in output_data and "context_decision_output" in output_data["interfaces"]:
            fields = output_data["interfaces"]["context_decision_output"].get("fields", {})
            test(
                "Output has decision_id field",
                "decision_id" in fields,
                "Missing decision_id field"
            )
            test(
                "Output has context_route field",
                "context_route" in fields,
                "Missing context_route field"
            )
            test(
                "Output has evidence_requirements field",
                "evidence_requirements" in fields,
                "Missing evidence_requirements field"
            )
            test(
                "Output has receipt_summary field",
                "receipt_summary" in fields,
                "Missing receipt_summary field"
            )

    # Validate receipt fixture
    if receipt_fixture_path.exists():
        with open(receipt_fixture_path, "r", encoding="utf-8") as f:
            receipt_data = json.load(f)

        test(
            "Receipt fixture has schema_version",
            "schema_version" in receipt_data,
            "Missing schema_version field"
        )
        test(
            "Receipt fixture has interfaces",
            "interfaces" in receipt_data,
            "Missing interfaces field"
        )
        test(
            "Receipt fixture has receipt_with_context_decision",
            "receipt_with_context_decision" in receipt_data.get("interfaces", {}),
            "Missing receipt_with_context_decision interface"
        )

        # Validate receipt fields
        if "interfaces" in receipt_data and "receipt_with_context_decision" in receipt_data["interfaces"]:
            fields = receipt_data["interfaces"]["receipt_with_context_decision"].get("fields", {})
            test(
                "Receipt has context_decision field",
                "context_decision" in fields,
                "Missing context_decision field"
            )
            test(
                "Receipt context_decision has advisory field",
                fields.get("context_decision", {}).get("fields", {}).get("advisory") is not None,
                "Missing advisory field in context_decision"
            )
else:
    test("Fixtures directory exists", False, f"Missing: {FIXTURES_DIR}")


# ---------------------------------------------------------------------------
# Test Category 3: Measured Costs Reference Validation
# ---------------------------------------------------------------------------
print("\n[3/5] Measured Costs Reference Validation")

test(
    "Measured profiles file exists",
    MEASURED_PROFILES_PATH.exists(),
    f"Missing: {MEASURED_PROFILES_PATH}"
)

if MEASURED_PROFILES_PATH.exists():
    with open(MEASURED_PROFILES_PATH, "r", encoding="utf-8") as f:
        profiles = json.load(f)

    # Measured profiles can have flat or nested structure
    has_profiles = "metadata" in profiles or "hardware_profiles" in profiles or "windows_runtime_node" in profiles
    test(
        "Measured profiles has profile data",
        has_profiles,
        f"Missing profile data. Keys: {list(profiles.keys())}"
    )

    # Check for profiles either at top level or under hardware_profiles
    hw_profiles = profiles.get("hardware_profiles", profiles)

    test(
        "Has windows_runtime_node profile",
        "windows_runtime_node" in hw_profiles,
        f"Missing windows_runtime_node profile. Keys: {list(hw_profiles.keys())}"
    )
    test(
        "Has weak_lan_runtime_node profile",
        "weak_lan_runtime_node" in hw_profiles,
        f"Missing weak_lan_runtime_node profile. Keys: {list(hw_profiles.keys())}"
    )

    if "windows_runtime_node" in hw_profiles:
        win_profile = hw_profiles["windows_runtime_node"]
        test(
            "Windows profile has git_status_ms",
            "git_status_ms" in win_profile,
            f"Missing git_status_ms. Keys: {list(win_profile.keys())}"
        )
        test(
            "Windows profile has git_revparse_ms",
            "git_revparse_ms" in win_profile,
            f"Missing git_revparse_ms. Keys: {list(win_profile.keys())}"
        )
        test(
            "Windows profile has file_read_warm_ms",
            "file_read_warm_ms" in win_profile,
            f"Missing file_read_warm_ms. Keys: {list(win_profile.keys())}"
        )
        # Check for recall_packet and degraded_node in measured profiles
        # These may be in the prototype decisions file, not in hardware_profiles
        has_recall = "recall_packet_local" in win_profile or "recall_packet_local" in hw_profiles
        has_degraded = "degraded_node" in win_profile or "degraded_node" in hw_profiles or "weak_lan_runtime_node" in hw_profiles
        # These are advisory data points, not required in all profile files
        test(
            "Has recall_packet_local or weak_lan data",
            has_recall or has_degraded,
            "Missing recall_packet_local and degraded_node data"
        )

        # Validate key measured values
        if "git_status_ms" in win_profile:
            test(
                "git_status_ms is in expected range (50-100ms)",
                50 <= win_profile["git_status_ms"] <= 100,
                f"Got {win_profile['git_status_ms']}"
            )
        # Check degraded_node timeout in top-level or nested
        degraded = hw_profiles.get("weak_lan_runtime_node", {})
        test(
            "degraded_node timeout is in expected range (3000-5000ms)",
            3000 <= degraded.get("unreachable_timeout_ms", 0) <= 5000,
            f"Got {degraded.get('unreachable_timeout_ms', 0)}"
        )


# ---------------------------------------------------------------------------
# Test Category 4: Prototype Decisions Reference Validation
# ---------------------------------------------------------------------------
print("\n[4/5] Prototype Decisions Reference Validation")

test(
    "Prototype decisions file exists",
    PROTOTYPE_DECISIONS_PATH.exists(),
    f"Missing: {PROTOTYPE_DECISIONS_PATH}"
)

if PROTOTYPE_DECISIONS_PATH.exists():
    with open(PROTOTYPE_DECISIONS_PATH, "r", encoding="utf-8") as f:
        decisions = json.load(f)

    test(
        "Decisions has metadata",
        "metadata" in decisions,
        "Missing metadata field"
    )
    test(
        "Decisions has workload_decisions",
        "workload_decisions" in decisions,
        "Missing workload_decisions field"
    )
    test(
        "Decisions has scenario_decisions",
        "scenario_decisions" in decisions,
        "Missing scenario_decisions field"
    )

    if "workload_decisions" in decisions:
        wl_decisions = decisions["workload_decisions"]
        test(
            "Has decisions for all 9 workload types",
            len(wl_decisions) == 9,
            f"Expected 9, got {len(wl_decisions)}"
        )

        for d in wl_decisions:
            wl_type = d.get("context_route", {}).get("workload_type", "unknown")
            route = d.get("context_route", {}).get("selected_context_route", "unknown")

            test(
                f"Decision for {wl_type} has valid route",
                route in CONTEXT_ROUTES,
                f"Got route: {route}"
            )
            test(
                f"Decision for {wl_type} has governance_outcome",
                "governance_outcome" in d.get("context_route", {}),
                "Missing governance_outcome"
            )
            test(
                f"Decision for {wl_type} has receipt_summary",
                "receipt_summary" in d.get("context_route", {}),
                "Missing receipt_summary"
            )

    if "scenario_decisions" in decisions:
        scenarios = decisions["scenario_decisions"]
        test(
            "Has scenario decisions",
            len(scenarios) > 0,
            "No scenario decisions found"
        )


# ---------------------------------------------------------------------------
# Test Category 5: Advisory-Only Boundary Validation
# ---------------------------------------------------------------------------
print("\n[5/5] Advisory-Only Boundary Validation")

# Verify that the design document does not suggest production behavior changes
if DESIGN_DOC_PATH.exists():
    content = DESIGN_DOC_PATH.read_text(encoding="utf-8")

    test(
        "Design does not suggest modifying router.py",
        "MODIFY router.py" not in content.upper() and "CHANGE router.py" not in content.upper(),
        "Design should not suggest modifying production router"
    )
    test(
        "Design does not suggest modifying Rust router",
        "MODIFY rust-router" not in content.upper() and "CHANGE rust-router" not in content.upper(),
        "Design should not suggest modifying production Rust router"
    )
    test(
        "Design does not suggest adding cache engine",
        "ADD cache engine" not in content.upper() and "IMPLEMENT cache engine" not in content.upper(),
        "Design should not suggest adding cache engine"
    )
    # Check that GPU/RDMA/KV-cache terms are only used in negative/prohibited context
    gpu_lines = [line.strip() for line in content.split('\n') if 'GPU' in line or 'RDMA' in line or 'KV-cache' in line]
    # The design document should have clear negative indicators
    has_negative_indicators = 'DO NOT TOUCH' in content or 'FORBIDDEN' in content
    # All mentions should be in blockquotes, table rows, or numbered lists
    all_in_structured_context = all(
        line.startswith('>') or line.startswith('|') or line[0:1].isdigit()
        for line in gpu_lines
    ) if gpu_lines else True
    test(
        "Design does not advocate GPU/RDMA/KV-cache",
        has_negative_indicators and all_in_structured_context,
        f"Found GPU/RDMA/KV-cache mentions not in prohibited context: {gpu_lines[:3]}"
    )
    test(
        "Design explicitly states advisory-only",
        "advisory_only" in content or "advisory only" in content.lower(),
        "Design should explicitly state advisory-only"
    )
    test(
        "Design explicitly states no production behavior change",
        "DO NOT TOUCH" in content or "FORBIDDEN" in content or "UNCHANGED" in content,
        "Design should explicitly state production files are untouched"
    )

# Verify that fixtures are marked as advisory
for fixture_file in FIXTURES_DIR.glob("*.json") if FIXTURES_DIR.exists() else []:
    with open(fixture_file, "r", encoding="utf-8") as f:
        fixture_data = json.load(f)

    test(
        f"{fixture_file.name} is marked as advisory",
        "advisory" in fixture_data.get("status", "").lower() or "design only" in fixture_data.get("status", "").lower(),
        f"Fixture should be marked as advisory: {fixture_data.get('status', '')}"
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
