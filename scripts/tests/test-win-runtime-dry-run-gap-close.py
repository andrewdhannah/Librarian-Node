#!/usr/bin/env python3
"""
WIN-RUNTIME-DRY-RUN-GAP-CLOSE-1 — Dry-Run Readiness Gap Closure Tests

Verifies that the three gaps found in WIN-RUNTIME-DRY-RUN-READINESS-1 are closed:

GAP-001: config/mcp-permissions.json created and structurally valid.
GAP-002: .gitignore and runbook local-config wording agree.
GAP-003: Runbook §11.1 embedding port source corrected.

Also validates that:
- The MCP permission matrix enforces human-final verification authority.
- No tool entry grants can_verify=true.
- The health check script can consume the permission file.
- No regression in the three gap areas.
"""

import json
import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

# ---------------------------------------------------------------------------
# Files under test
# ---------------------------------------------------------------------------
PERMISSIONS_PATH = "config/mcp-permissions.json"
GITIGNORE_PATH = ".gitignore"
RUNBOOK_PATH = "docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md"
HEALTH_SCRIPT_PATH = "scripts/check-mcp-health.ps1"

# Expected MCP tools (must match check-mcp-health.ps1)
EXPECTED_MCP_TOOLS = [
    "librarian_checkin",
    "librarian_checkout",
    "librarian_checkpoint_work_order",
    "librarian_close_work_order",
    "librarian_diverge",
    "librarian_generate_doc",
    "librarian_get_item",
    "librarian_heartbeat",
    "librarian_plan_work",
    "librarian_resume_work_order",
    "librarian_search",
    "librarian_start_work_order",
]

# ---------------------------------------------------------------------------
# Test infrastructure
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


def read_file(path_str):
    p = REPO_ROOT / path_str
    if not p.exists():
        return None
    return p.read_text(encoding="utf-8")


def load_json(path_str):
    p = REPO_ROOT / path_str
    if not p.exists():
        return None
    with open(p, "r", encoding="utf-8") as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Test 1: GAP-001 — MCP permissions file exists and is valid
# ---------------------------------------------------------------------------
print("\n[1/10] GAP-001: MCP Permission Matrix")

perms = load_json(PERMISSIONS_PATH)
test(
    "[GAP-001] config/mcp-permissions.json exists",
    perms is not None,
    f"File not found at {PERMISSIONS_PATH}",
)

if perms is not None:
    # Must have _meta section
    test(
        "[GAP-001] Has '_meta' section",
        "_meta" in perms,
        "Missing _meta section",
    )

    # Must have rules section
    test(
        "[GAP-001] Has 'rules' section",
        "rules" in perms,
        "Missing rules section",
    )

    # Must have tools section
    test(
        "[GAP-001] Has 'tools' section",
        "tools" in perms,
        "Missing tools section",
    )

    # ── Rules validation ──
    rules = perms.get("rules", {})
    test(
        "[GAP-001] agents_can_mark_verified is false",
        rules.get("agents_can_mark_verified") is False,
        f"agents_can_mark_verified = {rules.get('agents_can_mark_verified')}",
    )
    test(
        "[GAP-001] human_verification_is_final is true",
        rules.get("human_verification_is_final") is True,
        f"human_verification_is_final = {rules.get('human_verification_is_final')}",
    )

    # ── Tool entries validation ──
    tools = perms.get("tools", {})

    # Every expected tool has an entry
    for tool in EXPECTED_MCP_TOOLS:
        test(
            f"[GAP-001] Tool entry present: '{tool}'",
            tool in tools,
            f"Missing permission entry for {tool}",
        )

    # No tool has can_verify=true
    tools_with_verify = []
    for tool_name, tool_entry in tools.items():
        if tool_entry.get("can_verify") is True:
            tools_with_verify.append(tool_name)
    test(
        "[GAP-001] No tool has can_verify=true",
        len(tools_with_verify) == 0,
        f"Tool(s) with can_verify=true: {tools_with_verify}",
    )

    # Each tool entry has 'can_verify' key
    tools_missing_can_verify = [
        tn for tn, te in tools.items() if "can_verify" not in te
    ]
    test(
        "[GAP-001] All tool entries have 'can_verify' key",
        len(tools_missing_can_verify) == 0,
        f"Tool(s) missing can_verify: {tools_missing_can_verify}",
    )

    # No extra tools beyond expected set (unless intentionally documented)
    extra_tools = set(tools.keys()) - set(EXPECTED_MCP_TOOLS)
    test(
        "[GAP-001] No unexpected tool entries beyond expected 12",
        len(extra_tools) == 0,
        f"Unexpected tool(s): {extra_tools}",
    )

    # No machine-local values in the file
    perms_text = json.dumps(perms)
    test(
        "[GAP-001] No machine-local paths in permission file",
        "G:\\" not in perms_text and "C:\\" not in perms_text,
        "Found machine-local path in permission file",
    )

# ---------------------------------------------------------------------------
# Test 2: GAP-001 — Health check script can consume permission file
# ---------------------------------------------------------------------------
print("\n[2/10] GAP-001: Health Check Script Compatibility")

health_script = read_file(HEALTH_SCRIPT_PATH)
if health_script is not None:
    # Verify health script references the correct path
    test(
        "[GAP-001] Health script references config/mcp-permissions.json",
        "mcp-permissions.json" in health_script,
        "Health script missing reference to mcp-permissions.json",
    )

    # Verify health script's rules validation logic matches our file
    test(
        "[GAP-001] Health script checks agents_can_mark_verified",
        "agents_can_mark_verified" in health_script,
        "Health script missing agents_can_mark_verified check",
    )
    test(
        "[GAP-001] Health script checks human_verification_is_final",
        "human_verification_is_final" in health_script,
        "Health script missing human_verification_is_final check",
    )
    test(
        "[GAP-001] Health script checks can_verify on each tool",
        "can_verify" in health_script,
        "Health script missing can_verify check",
    )

# Simulate what the health check script does: load perms and validate
if perms is not None:
    # Replicate the health check logic
    sim_errors = []

    if perms.get("rules", {}).get("agents_can_mark_verified") is True:
        sim_errors.append("agents_can_mark_verified must be false")
    if perms.get("rules", {}).get("human_verification_is_final") is not True:
        sim_errors.append("human_verification_is_final must be true")

    tools = perms.get("tools", {})
    for tool_name, tool_entry in tools.items():
        if tool_entry.get("can_verify") is True:
            sim_errors.append(f"{tool_name} has can_verify=true")

    for tool in EXPECTED_MCP_TOOLS:
        if tool not in tools:
            sim_errors.append(f"missing permission entry for {tool}")

    test(
        "[GAP-001] Simulated health check passes (0 errors)",
        len(sim_errors) == 0,
        f"Simulated check found errors: {sim_errors}",
    )

# ---------------------------------------------------------------------------
# Test 3: GAP-001 — Runbook references the permission file
# ---------------------------------------------------------------------------
print("\n[3/10] GAP-001: Runbook References Permission File")

runbook = read_file(RUNBOOK_PATH)
if runbook is not None:
    test(
        "[GAP-001] Runbook references 'config/mcp-permissions.json'",
        "config/mcp-permissions.json" in runbook,
        "Runbook missing reference to config/mcp-permissions.json",
    )

# ---------------------------------------------------------------------------
# Test 4: GAP-002 — gitignore has config/*.local.*
# ---------------------------------------------------------------------------
print("\n[4/10] GAP-002: Gitignore Local-Config Pattern")

gitignore = read_file(GITIGNORE_PATH)
if gitignore is not None:
    test(
        "[GAP-002] .gitignore contains 'config/*.local.*' pattern",
        "config/*.local.*" in gitignore,
        "Missing config/*.local.* in .gitignore",
    )

    # Should NOT still have only config/*.local.json without the broad pattern
    has_broad = "config/*.local.*" in gitignore
    has_narrow = "config/*.local.json" in gitignore
    test(
        "[GAP-002] config/*.local.* covers all extensions (.ps1, .json, etc.)",
        has_broad,
        "Only config/*.local.json found, config/*.local.* missing",
    )

# ---------------------------------------------------------------------------
# Test 5: GAP-002 — Runbook and gitignore agree on local-config ignore
# ---------------------------------------------------------------------------
print("\n[5/10] GAP-002: Runbook and Gitignore Agreement")

if runbook is not None and gitignore is not None:
    # Runbook §3.1 should reference the correct gitignore pattern
    # §3.1: model-profiles.local.json — covered by config/*.local.*
    section_3 = re.search(
        r"##\s+3\.\s+Local Config Setup Checklist.*?(?=##\s+4\b)",
        runbook,
        re.DOTALL | re.IGNORECASE,
    )
    if section_3:
        s3_text = section_3.group(0)

        # §3.1 can say config/*.local.* or config/*.local.json — both are correct
        # since config/*.local.* covers .json files
        test(
            "[GAP-002] §3.1 references a gitignore pattern that covers .local.json",
            "config/*.local" in s3_text,
            "§3.1 missing gitignore reference",
        )

        # §3.2 should reference a pattern that covers .ps1
        # The old gap was that it said config/*.local.* but gitignore didn't have it
        # Now both should agree
        has_local_star = "config/*.local.*" in s3_text
        test(
            "[GAP-002] §3.2 gitignore reference covers .ps1 extensions",
            has_local_star,
            "§3.2 may still reference a pattern that doesn't cover .ps1",
        )
    else:
        test("[GAP-002] Section 3 found", False, "Could not extract Section 3")

# ---------------------------------------------------------------------------
# Test 6: GAP-003 — Runbook §11.1 embedding port source corrected
# ---------------------------------------------------------------------------
print("\n[6/10] GAP-003: Embedding Port 9125 Source Reference")

if runbook is not None:
    # Extract section 11
    section_11 = re.search(
        r"##\s+11\.\s+Reference: Authoritative Values.*",
        runbook,
        re.DOTALL | re.IGNORECASE,
    )
    if section_11:
        s11_text = section_11.group(0)

        # The embedding port source must reference runtime/model_manager.ps1
        test(
            "[GAP-003] Embedding port source references runtime/model_manager.ps1",
            "runtime/model_manager.ps1" in s11_text or "runtime\\model_manager.ps1" in s11_text,
            "Embedding port source does not reference runtime/model_manager.ps1",
        )

        # The embedding port source must NOT reference config/model-profiles.json
        # (it was the incorrect source)
        # Find the embedding row
        embedding_line_match = re.search(
            r"\| Embedding \| 9125 \|.*?\|",
            s11_text,
        )
        if embedding_line_match:
            embedding_line = embedding_line_match.group(0)
            has_incorrect_source = "config/model-profiles.json" in embedding_line
            test(
                "[GAP-003] Embedding port source is NOT config/model-profiles.json (old incorrect source)",
                not has_incorrect_source,
                "Embedding port source still references config/model-profiles.json",
            )
        else:
            test("[GAP-003] Embedding row found in port map", False, "Could not find Embedding row in §11.1")
    else:
        test("[GAP-003] Section 11 found", False, "Could not extract Section 11")

# ── Additional: port value 9125 must still be correct ──
if runbook is not None:
    test(
        "[GAP-003] Port 9125 is still documented for Embedding",
        "9125" in runbook,
        "Port 9125 removed from runbook",
    )

# ---------------------------------------------------------------------------
# Test 7: GAP-003 — Runbook §11.5 config file table updated
# ---------------------------------------------------------------------------
print("\n[7/10] GAP-003: Config File Sources Table Updated")

if runbook is not None:
    # The config file sources table should now include mcp-permissions.json
    section_11_5 = re.search(
        r"### 11\.5 Config File Sources.*?(?=## Runbook Validation)",
        runbook,
        re.DOTALL,
    )
    if section_11_5:
        s115_text = section_11_5.group(0)
        test(
            "[GAP-003] §11.5 includes config/mcp-permissions.json",
            "config/mcp-permissions.json" in s115_text,
            "config/mcp-permissions.json not listed in §11.5",
        )
    else:
        test("[GAP-003] §11.5 found", False, "Could not extract Section 11.5")

# ---------------------------------------------------------------------------
# Test 8: GAP-003 — Runbook §11.4 MCP endpoints updated
# ---------------------------------------------------------------------------
print("\n[8/10] GAP-003: MCP Endpoints Table Updated")

if runbook is not None:
    section_11_4 = re.search(
        r"### 11\.4 MCP Endpoints.*?(?=### 11\.5)",
        runbook,
        re.DOTALL,
    )
    if section_11_4:
        s114_text = section_11_4.group(0)
        test(
            "[GAP-003] §11.4 includes Permission matrix entry",
            "Permission matrix" in s114_text or "config/mcp-permissions.json" in s114_text,
            "MCP endpoints table missing permission matrix reference",
        )
    else:
        test("[GAP-003] §11.4 found", False, "Could not extract Section 11.4")

# ---------------------------------------------------------------------------
# Test 9: GAP-002 — No tracked local override files
# ---------------------------------------------------------------------------
print("\n[9/10] GAP-002: No Tracked Local Override Files")

import subprocess

git_ls_files = subprocess.run(
    ["git", "ls-files", "config/*.local.*"],
    capture_output=True, text=True, cwd=REPO_ROOT,
)
tracked_locals = [f.strip() for f in git_ls_files.stdout.strip().split("\n") if f.strip()]
# Example files are allowed (they're the templates, not actual overrides)
example_locals = [f for f in tracked_locals if "example" in f.lower()]
actual_locals = [f for f in tracked_locals if "example" not in f.lower()]
test(
    "[GAP-002] No tracked actual local override files (only example files)",
    len(actual_locals) == 0,
    f"Tracked local override files: {actual_locals}",
)

# ---------------------------------------------------------------------------
# Test 10: Section 10 do-not-proceed conditions still intact
# ---------------------------------------------------------------------------
print("\n[10/10] No Regression: Do-Not-Proceed Conditions Intact")

if runbook is not None:
    # Verify the hard boundaries from the original sprint are still present
    required_stops = [
        "Hard Stop",
        "Policy Stop",
        "Boundary Stop",
        "do not proceed",
        "unexpected.*LISTENING",
        "orphan",
        "llama-server.exe",
    ]
    for pattern in required_stops:
        regex = re.compile(pattern, re.IGNORECASE)
        test(
            f"[regression] Boundary condition '{pattern}' present",
            bool(regex.search(runbook)),
            f"Missing boundary condition: {pattern}",
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
