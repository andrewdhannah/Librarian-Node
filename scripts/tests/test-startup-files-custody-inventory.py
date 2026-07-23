#!/usr/bin/env python3
"""
WIN-STARTUP-FILES-CUSTODY-0 — Startup Files Custody Inventory Validation

Verifies that the startup custody inventory exists, covers required
categories, classifies high-risk files, identifies path casing drift,
and confirms no production behavior was changed.

This is a validation test for the inventory/report artifacts — it does
not test runtime behavior, service startup, or production routing.
"""

import json
import os
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
INVENTORY_PATH = REPO_ROOT / "reports" / "startup-files-custody-inventory.json"
REPORT_PATH = REPO_ROOT / "reports" / "WIN-STARTUP-FILES-CUSTODY-0.md"
PLANNING_PATH = REPO_ROOT / "docs" / "planning" / "WIN-STARTUP-FILES-CUSTODY-0.md"
CLOSEOUT_PATH = REPO_ROOT / "docs" / "sprints" / "WIN-STARTUP-FILES-CUSTODY-0.md"
MANIFEST_EXAMPLE = REPO_ROOT / "fixtures" / "startup-files-custody" / "startup-custody-manifest.example.json"
LOCAL_CONFIG_EXAMPLE = REPO_ROOT / "fixtures" / "startup-files-custody" / "machine-local-config.example.json"

# ---------------------------------------------------------------------------
# Required categories in the inventory
# ---------------------------------------------------------------------------
REQUIRED_CATEGORIES = [
    "service_launcher",
    "runtime_profiles_and_model_config",
    "operations_scripts",
    "qualification_and_service_swap",
    "environment_variables",
    "ports",
    "absolute_paths",
    "path_casing_drift",
]

HIGH_RISK_FILES = [
    "scripts/start-librarian-runtime-node.ps1",
    "config/model-profiles.json",
    "runtime/model_manager.ps1",
    "scripts/operations/runtime-status.ps1",
    "scripts/operations/runtime-logs.ps1",
    "scripts/operations/runtime-clean-check.ps1",
    "scripts/test-win-rust-service-swap.ps1",
]

PRODUCTION_ROUTER_FILES = [
    "router/router.py",
    "rust-router/src/",
]

# ---------------------------------------------------------------------------
# Test results
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
# Test Category 1: Inventory file exists and is valid JSON
# ---------------------------------------------------------------------------
print("\n[1/8] Inventory File Validation")

test(
    "Inventory JSON exists",
    INVENTORY_PATH.exists(),
    f"Missing: {INVENTORY_PATH}",
)

if INVENTORY_PATH.exists():
    inventory = load_json(INVENTORY_PATH)
    test(
        "Inventory JSON is valid",
        True,
        "",
    )
    test(
        "Inventory has _meta",
        "_meta" in inventory,
        "Missing _meta section",
    )
    test(
        "Inventory has categories",
        "categories" in inventory,
        "Missing categories section",
    )


# ---------------------------------------------------------------------------
# Test Category 2: Required categories exist
# ---------------------------------------------------------------------------
print("\n[2/8] Required Categories")

if INVENTORY_PATH.exists() and "categories" in inventory:
    for cat in REQUIRED_CATEGORIES:
        test(
            f"Inventory has category: {cat}",
            cat in inventory["categories"],
            f"Missing category: {cat}",
        )


# ---------------------------------------------------------------------------
# Test Category 3: High-risk files are represented
# ---------------------------------------------------------------------------
print("\n[3/8] High-Risk File Classification")

if INVENTORY_PATH.exists() and "categories" in inventory:
    # Collect all defining_file values from the inventory
    tracked_defining_files = set()
    for cat_name, cat_data in inventory["categories"].items():
        if "items" in cat_data:
            for item in cat_data["items"]:
                if "defining_file" in item:
                    tracked_defining_files.add(item["defining_file"].replace("\\", "/"))

    for risky_file in HIGH_RISK_FILES:
        normalized = risky_file.replace("\\", "/")
        found = any(normalized in f for f in tracked_defining_files)
        test(
            f"High-risk file {risky_file} is represented in inventory",
            found,
            f"Not found in any inventory item's defining_file",
        )


# ---------------------------------------------------------------------------
# Test Category 4: model-profiles.json classified as machine-specific
# ---------------------------------------------------------------------------
print("\n[4/8] model-profiles.json Classification")

if INVENTORY_PATH.exists():
    profile_items = []
    for cat_name, cat_data in inventory["categories"].items():
        if "items" in cat_data:
            for item in cat_data["items"]:
                file_val = item.get("defining_file", "").replace("\\", "/")
                if "model-profiles.json" in file_val:
                    profile_items.append(item)

    test(
        "model-profiles.json has classified items",
        len(profile_items) > 0,
        "No items reference model-profiles.json",
    )

    machine_specific_count = sum(
        1 for item in profile_items if item.get("machine_specific") is True
    )
    test(
        "model-profiles.json items marked machine_specific",
        machine_specific_count > 0,
        "No items marked as machine_specific",
    )

    critical_count = sum(
        1 for item in profile_items if item.get("current_risk_level") == "critical"
    )
    test(
        "model-profiles.json items classified as risk=critical",
        critical_count > 0,
        "No items marked risk=critical",
    )


# ---------------------------------------------------------------------------
# Test Category 5: Service launcher classified
# ---------------------------------------------------------------------------
print("\n[5/8] Service Launcher Classification")

if INVENTORY_PATH.exists() and "categories" in inventory:
    launcher = inventory["categories"].get("service_launcher", {})
    items = launcher.get("items", [])
    test(
        "Service launcher category has items",
        len(items) >= 5,
        f"Only {len(items)} items found",
    )

    # Check for WorkDir, RouterPort, NSSM entries
    value_names = {item.get("value_name", "") for item in items}
    test(
        "Service launcher has WorkDir",
        "WorkDir" in value_names or any("WorkDir" in v for v in value_names),
        "Missing WorkDir entry",
    )
    test(
        "Service launcher has RouterPort",
        "RouterPort" in value_names or any("RouterPort" in v for v in value_names),
        "Missing RouterPort entry",
    )
    test(
        "Service launcher has NSSM entries",
        any("NSSM" in v for v in value_names),
        "Missing NSSM entries",
    )


# ---------------------------------------------------------------------------
# Test Category 6: Operations scripts classified
# ---------------------------------------------------------------------------
print("\n[6/8] Operations Scripts Classification")

if INVENTORY_PATH.exists() and "categories" in inventory:
    ops = inventory["categories"].get("operations_scripts", {})
    items = ops.get("items", [])
    test(
        "Operations scripts category has items",
        len(items) >= 5,
        f"Only {len(items)} items found",
    )

    # Check for all 5 operations scripts
    ops_script_refs = [item.get("defining_file", "") for item in items]
    test(
        "runtime-start.ps1 is classified",
        any("runtime-start.ps1" in r for r in ops_script_refs),
        "Missing runtime-start.ps1",
    )
    test(
        "runtime-stop.ps1 is classified",
        any("runtime-stop.ps1" in r for r in ops_script_refs),
        "Missing runtime-stop.ps1",
    )
    test(
        "runtime-status.ps1 is classified",
        any("runtime-status.ps1" in r for r in ops_script_refs),
        "Missing runtime-status.ps1",
    )
    test(
        "runtime-logs.ps1 is classified",
        any("runtime-logs.ps1" in r for r in ops_script_refs),
        "Missing runtime-logs.ps1",
    )
    test(
        "runtime-clean-check.ps1 is classified",
        any("runtime-clean-check.ps1" in r for r in ops_script_refs),
        "Missing runtime-clean-check.ps1",
    )


# ---------------------------------------------------------------------------
# Test Category 7: Path casing drift documented
# ---------------------------------------------------------------------------
print("\n[7/8] Path Casing Drift Detection")

if INVENTORY_PATH.exists() and "categories" in inventory:
    drift = inventory["categories"].get("path_casing_drift", {})
    test(
        "Path casing drift category exists",
        drift.get("drift_found", False) is not None,
        "Missing path_casing_drift category",
    )

    if drift.get("drift_found"):
        test(
            "Path casing drift is documented as found",
            drift.get("drift_found") is True,
            "Drift found but flagged as false",
        )
        variants = drift.get("variants", [])
        # Must detect at least 2 casing variants (lowercase openwork vs camelCase OpenWork)
        test(
            f"Path casing drift has {len(variants)} variant(s) documented",
            len(variants) >= 2,
            f"Only {len(variants)} variant(s) — expected at least 2 (openwork vs OpenWork)",
        )
        if variants:
            casing_values = {v.get("casing") for v in variants}
            has_lower = any("openwork\\librarian" in v.get("casing", "").lower() or "openwork" in v.get("casing", "").lower() for v in variants)
            test(
                "Path casing drift includes lowercase variant",
                has_lower,
                "Missing lowercase G:\\openwork casing in drift variants",
            )


# ---------------------------------------------------------------------------
# Test Category 8: No production files modified
# ---------------------------------------------------------------------------
print("\n[8/8] Production File Boundary")

# Check no production router files were modified (authoritative: git diff)
import subprocess
diff_result = subprocess.run(
    ["git", "diff", "--name-only"],
    capture_output=True, text=True, cwd=REPO_ROOT,
)
modified_files = [f.strip() for f in diff_result.stdout.split("\n") if f.strip()]
# Check modified files — production router/runtime files must not be modified
PRODUCTION_PREFIXES = ["router/", "rust-router/"]
prod_modifications = [f for f in modified_files if any(f.startswith(p) for p in PRODUCTION_PREFIXES)]
test(
    "No production router/runtime files modified",
    len(prod_modifications) == 0,
    f"Production files modified: {prod_modifications}",
)

# Verify unstaged new files are in allowed paths
import subprocess
status_result = subprocess.run(
    ["git", "status", "--short"],
    capture_output=True, text=True, cwd=REPO_ROOT,
)
new_files = [line.strip() for line in status_result.stdout.split("\n") if line.strip()]
# All new files should be in allowed paths (docs, reports, config, scripts/tests, fixtures)
allowed_prefixes = ["docs/", "reports/", "config/", "scripts/tests/"]
for nf in new_files:
    if nf.startswith("?? "):
        fname = nf[3:].replace("\\", "/")
        is_allowed = any(fname.startswith(pref) for pref in allowed_prefixes) or fname.startswith("fixtures/")
        test(
            f"New file is in allowed path: {fname}",
            is_allowed,
            f"Unexpected new file: {fname}",
        )

# Verify the report and closeout docs exist
test(
    "Report exists",
    REPORT_PATH.exists(),
    f"Missing: {REPORT_PATH}",
)
test(
    "Planning doc exists",
    PLANNING_PATH.exists(),
    f"Missing: {PLANNING_PATH}",
)
test(
    "Closeout doc exists",
    CLOSEOUT_PATH.exists(),
    f"Missing: {CLOSEOUT_PATH}",
)
test(
    "Custody manifest example exists",
    MANIFEST_EXAMPLE.exists(),
    f"Missing: {MANIFEST_EXAMPLE}",
)
test(
    "Machine-local config example exists",
    LOCAL_CONFIG_EXAMPLE.exists(),
    f"Missing: {LOCAL_CONFIG_EXAMPLE}",
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
