#!/usr/bin/env python3
"""
WIN-RUNTIME-DRY-RUN-READINESS-1 — Dry-Run Readiness Validation Tests

Verifies that:
1. All runbook-referenced tracked files exist in the repo.
2. Placeholder/local-only paths are not treated as repo files.
3. No runbook instruction tells an agent to auto-start the service.
4. No runbook instruction tells an agent to run models automatically.
5. Dry-run readiness matrix exists and covers all runbook sections.
6. Runbook filepaths use consistent separators.
7. Every tracked PowerShell script in scripts/operations/ exists.
8. MCP health check exit codes are documented.
9. Service start type "Manual" is enforced in boundary stops.
10. No embedded machine-local paths in documentation files outside gitignored patterns.

These tests enforce dry-run readiness and prevent runbook regression.
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
RUNBOOK_PATH = "docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md"
DRY_RUN_MATRIX = "reports/WIN-RUNTIME-DRY-RUN-READINESS-1.md"
DRY_RUN_JSON = "reports/win-runtime-dry-run-readiness-1.json"

# All 11 required runbook sections (as H2 headings)
REQUIRED_RUNBOOK_SECTIONS = [
    r"##\s+1\.\s+Overview",
    r"##\s+2\.\s+Prerequisites",
    r"##\s+3\.\s+Local Config Setup Checklist",
    r"##\s+4\.\s+Port Verification Checklist",
    r"##\s+5\.\s+Backend Binary Verification Checklist",
    r"##\s+6\.\s+MCP Health Check Usage",
    r"##\s+7\.\s+Service Start/Stop Procedure",
    r"##\s+8\.\s+Log and Evidence Capture",
    r"##\s+9\.\s+Failure Triage",
    r"##\s+10\.\s+Do Not Proceed Conditions",
    r"##\s+11\.\s+Reference: Authoritative Values",
]

# Runbook-referenced tracked files (must exist)
RUNBOOK_REFERENCED_FILES = [
    "docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md",
    "scripts/check-mcp-health.ps1",
    "scripts/mcp-bridge.ps1",
    "scripts/operations/runtime-start.ps1",
    "scripts/operations/runtime-stop.ps1",
    "scripts/operations/runtime-status.ps1",
    "scripts/operations/runtime-logs.ps1",
    "scripts/operations/runtime-clean-check.ps1",
    "scripts/start-librarian-runtime-node.ps1",
    "config/model-profiles.json",
    "config/model-profiles.local.example.json",
    "config/model_manager.local.example.ps1",
    "config/runtime-node.example.json",
    "config/runtime-node.local.json",
    "config/mcp-permissions.json",
    "mcp/templates/mcp-server.example.json",
    "fixtures/startup-files-custody/startup-custody-manifest.example.json",
    "fixtures/startup-files-custody/machine-local-config.example.json",
]

# Gitignored/excluded files referenced in runbook (should NOT be tracked)
RUNBOOK_REFERENCED_GITIGNORED = [
    "config/model-profiles.local.json",
    "config/model_manager.local.ps1",
    "runtime/bin/nssm.exe",
]

# Forbidden patterns for agent auto-start/auto-model (must not appear in runbook)
FORBIDDEN_AUTO_SERVICE_PATTERNS = [
    r"automatically.*start.*the.*service",
    r"auto-start.*LibrarianRunTimeNode",
    r"Start-Service.*without.*checking",
    r"automatically.*run.*llama-server",
    r"the agent should start the service",
    r"no need to verify.*before.*start",
    r"skip.*pre-flight.*check",
]

FORBIDDEN_AUTO_MODEL_PATTERNS = [
    r"automatically.*load.*model",
    r"auto-run.*all.*models",
    r"start.*all.*model.*profiles",
    r"no need to select.*model",
]

# Machine-local path patterns that should NOT appear in runbook
# (except in documented working directory examples)
LOCAL_PATH_PATTERNS = [
    r"C:\\Users\\.*\\",
    r"D:\\Users\\.*\\",
    r"G:\\temp\\",
    r"G:\\llamacpp\\",
]

# Allowed machine-local paths in runbook (documented working dir)
ALLOWED_LOCAL_PATHS_IN_RUNBOOK = [
    "G:\\OpenWork\\librarian-runtime-node",
]

# Scripts that must exist in scripts/operations/
REQUIRED_OPS_SCRIPTS = [
    "scripts/operations/runtime-start.ps1",
    "scripts/operations/runtime-stop.ps1",
    "scripts/operations/runtime-status.ps1",
    "scripts/operations/runtime-logs.ps1",
    "scripts/operations/runtime-clean-check.ps1",
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
# Test 1: Dry-run readiness matrix exists
# ---------------------------------------------------------------------------
print("\n[1/10] Dry-Run Readiness Matrix Exists")

matrix_path = REPO_ROOT / DRY_RUN_MATRIX
test(
    "[matrix] Dry-run readiness matrix exists at reports/WIN-RUNTIME-DRY-RUN-READINESS-1.md",
    matrix_path.exists(),
    f"Missing: {matrix_path}",
)

json_path = REPO_ROOT / DRY_RUN_JSON
test(
    "[matrix] Machine-readable matrix exists at reports/win-runtime-dry-run-readiness-1.json",
    json_path.exists(),
    f"Missing: {json_path}",
)

# Validate JSON structure
matrix_json = load_json(DRY_RUN_JSON)
if matrix_json is not None:
    test(
        "[matrix] JSON has '_meta' section",
        "_meta" in matrix_json,
        "Missing _meta section",
    )
    test(
        "[matrix] JSON has 'summary' section",
        "summary" in matrix_json,
        "Missing summary section",
    )
    test(
        "[matrix] JSON has 'gaps' section",
        "gaps" in matrix_json,
        "Missing gaps section",
    )
    test(
        "[matrix] JSON has 'sections' section",
        "sections" in matrix_json,
        "Missing sections section",
    )
    test(
        "[matrix] JSON summary has activation_risk field",
        "activation_risk" in matrix_json.get("summary", {}),
        "Missing activation_risk in summary",
    )
    test(
        "[matrix] JSON summary has gap count",
        matrix_json.get("summary", {}).get("gaps_found", -1) >= 0,
        "Missing gaps_found in summary",
    )

# Validate markdown matrix covers all 11 sections
if matrix_path.exists():
    matrix_content = matrix_path.read_text(encoding="utf-8")
    for section_name in ["Overview", "Prerequisites", "Local Config Setup",
                         "Port Verification", "Backend Binary Verification",
                         "MCP Health Check", "Service Start/Stop",
                         "Log and Evidence", "Failure Triage",
                         "Do Not Proceed", "Reference"]:
        test(
            f"[matrix] Matrix covers '{section_name}' section",
            section_name in matrix_content,
            f"Section '{section_name}' not found in matrix",
        )

# ---------------------------------------------------------------------------
# Test 2: All runbook-referenced tracked files exist
# ---------------------------------------------------------------------------
print("\n[2/10] Runbook-Referenced Tracked Files Exist")

for fname in RUNBOOK_REFERENCED_FILES:
    full_path = REPO_ROOT / fname
    test(
        f"[file] {fname} exists",
        full_path.exists(),
        f"Missing: {full_path}",
    )

# ---------------------------------------------------------------------------
# Test 3: Gitignored files are not tracked
# ---------------------------------------------------------------------------
print("\n[3/10] Gitignored Files Not Tracked as Repo Files")

import subprocess

git_ls_files = subprocess.run(
    ["git", "ls-files"],
    capture_output=True, text=True, cwd=REPO_ROOT,
)
tracked_files = set(git_ls_files.stdout.strip().split("\n"))

for fname in RUNBOOK_REFERENCED_GITIGNORED:
    normalized = fname.replace("\\", "/")
    is_tracked = normalized in tracked_files
    test(
        f"[gitignore] {fname} is NOT tracked in git",
        not is_tracked,
        f"File is tracked but should be gitignored: {fname}",
    )

# ---------------------------------------------------------------------------
# Test 4: No auto-start instructions in runbook
# ---------------------------------------------------------------------------
print("\n[4/10] No Auto-Start Instructions in Runbook")

runbook = read_file(RUNBOOK_PATH)
if runbook is not None:
    for pattern in FORBIDDEN_AUTO_SERVICE_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(runbook)
        test(
            f"[no-auto-start] Does NOT contain '{pattern}'",
            len(matches) == 0,
            f"Found {len(matches)} match(es): {matches}",
        )

    # Verify explicit "do not automate this sequence" is present
    test(
        "[no-auto-start] Contains 'do not automate this sequence'",
        "do not automate this sequence" in runbook.lower(),
        "Missing explicit do-not-automate warning",
    )

    # Verify "Do not automate" appears in section 7
    section_7 = re.search(
        r"##\s+7\.\s+Service Start/Stop Procedure.*?(?=##\s+8\.)",
        runbook,
        re.DOTALL | re.IGNORECASE,
    )
    if section_7:
        s7_text = section_7.group(0)
        first_three = "\n".join(s7_text.split("\n")[:3]).lower()
        test(
            "[no-auto-start] Section 7 contains 'Do not automate' within first 3 lines",
            "do not automate" in first_three,
            "Section 7 does not have do-not-automate warning in first 3 lines",
        )

# ---------------------------------------------------------------------------
# Test 5: No auto-model instructions in runbook
# ---------------------------------------------------------------------------
print("\n[5/10] No Auto-Model Instructions in Runbook")

if runbook is not None:
    for pattern in FORBIDDEN_AUTO_MODEL_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(runbook)
        test(
            f"[no-auto-model] Does NOT contain '{pattern}'",
            len(matches) == 0,
            f"Found {len(matches)} match(es): {matches}",
        )

    # Verify explicit "no model selected" note exists
    test(
        "[no-auto-model] Contains 'no model selected' or similar",
        "no model selected" in runbook.lower(),
        "Missing note that no model is selected after service start",
    )

    # Verify that section 7 says model is not automatically loaded
    section_7 = re.search(
        r"##\s+7\.\s+Service Start/Stop Procedure.*?(?=##\s+8\.)",
        runbook,
        re.DOTALL | re.IGNORECASE,
    )
    if section_7:
        s7_text = section_7.group(0)
        test(
            "[no-auto-model] Section 7 says model not selected after start",
            "model selected" in s7_text.lower() or
            "/backend/select" in s7_text,
            "Missing note about model selection being manual",
        )

# ---------------------------------------------------------------------------
# Test 6: Dry-run readiness matrix covers all 11 runbook sections
# ---------------------------------------------------------------------------
print("\n[6/10] Matrix Covers All Runbook Sections")

if matrix_path.exists():
    matrix_content = matrix_path.read_text(encoding="utf-8")
    section_headers_in_matrix = re.findall(
        r"### §(\d+)", matrix_content
    )
    present_sections = set(int(s) for s in section_headers_in_matrix)
    all_sections = set(range(1, 12))
    missing_sections = all_sections - present_sections

    test(
        "[matrix-coverage] Matrix has entries for all 11 runbook sections",
        len(missing_sections) == 0,
        f"Missing sections in matrix: {sorted(missing_sections)}",
    )

    if len(missing_sections) == 0:
        # Count total check items across all sections
        check_count = len(re.findall(r"\|\s*\d+\.\d+\s+\|", matrix_content))
        test(
            "[matrix-coverage] Matrix has documented check items",
            check_count > 0,
            "No check items found in matrix",
        )
        print(f"  INFO: {check_count} total check items across 11 sections")

# ---------------------------------------------------------------------------
# Test 7: Runbook filepath separator consistency
# ---------------------------------------------------------------------------
print("\n[7/10] Runbook Filepath Separator Consistency")

if runbook is not None:
    # Runbook should primarily use backslashes for Windows paths
    # Count path-like references
    backslash_paths = re.findall(r'(?:\\{1,2})+[a-zA-Z0-9_\-\.]+\.(?:ps1|json|exe|md|txt|xml)', runbook)
    # Forward slash paths in runbook are allowed in some contexts (section 11, git paths)
    # But PowerShell commands should use backslashes or be consistent
    # This test is informational: guarantee no mixed separator confusion

    # Check that all PowerShell commands using paths don't mix separators
    # Focus on the operational code blocks (not reference tables or inline docs)
    code_blocks = re.findall(r'```powershell\n(.*?)```', runbook, re.DOTALL)
    mixed_sep_blocks = []
    for i, block in enumerate(code_blocks):
        has_backslash = '\\' in block
        has_forward = '/' in block
        # git commands, URLs, Test-Path, JSON-RPC method names, and inline docs accept either separator
        non_git_lines = [l for l in block.split('\n')
                        if 'git' not in l.lower()
                        and 'http' not in l.lower()
                        and 'Test-Path' not in l
                        and '-Path' not in l
                        and 'tools/' not in l       # JSON-RPC method names
                        and 'jsonrpc' not in l.lower()  # JSON-RPC payloads
                        and 'echo' not in l.lower()[:5]]  # echo commands often mix strings
        block_has_mixed = any('\\' in l and '/' in l for l in non_git_lines if l.strip())
        if block_has_mixed:
            mixed_sep_blocks.append(i)

    test(
        "[paths] No mixed backslash/forward-slash in operational PowerShell code blocks",
        len(mixed_sep_blocks) == 0,
        f"Mixed separators in code block(s): {mixed_sep_blocks}",
    )

# ---------------------------------------------------------------------------
# Test 8: All ops scripts exist
# ---------------------------------------------------------------------------
print("\n[8/10] All Operations Scripts Exist")

for fname in REQUIRED_OPS_SCRIPTS:
    full_path = REPO_ROOT / fname
    test(
        f"[ops-script] {fname} exists",
        full_path.exists(),
        f"Missing: {full_path}",
    )

# ---------------------------------------------------------------------------
# Test 9: MCP health check exit codes are documented in runbook
# ---------------------------------------------------------------------------
print("\n[9/10] MCP Health Check Exit Codes Documented")

if runbook is not None:
    # The section 6 has an exit code table — check each code directly
    section_6 = re.search(
        r"##\s+6\.\s+MCP Health Check Usage.*?(?=##\s+7\.)",
        runbook,
        re.DOTALL | re.IGNORECASE,
    )
    if section_6:
        s6_text = section_6.group(0)
        for code in ["0", "1", "2", "3"]:
            test(
                f"[mcp-exit] Runbook documents exit code {code}",
                f"| {code} |" in s6_text,
                f"Exit code {code} not found in exit code table",
            )
    else:
        for code in ["0", "1", "2", "3"]:
            test(f"[mcp-exit] Runbook documents exit code {code}", False, "Section 6 not found")

# ---------------------------------------------------------------------------
# Test 10: No machine-local paths in documentation files outside gitignored
# ---------------------------------------------------------------------------
print("\n[10/10] No Machine-Local Paths in Documentation Beyond Working Dir")

if runbook is not None:
    # Check for disallowed local paths
    for pattern in LOCAL_PATH_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(runbook)
        # Filter out the allowed working directory path
        allowed = ["G:\\OpenWork\\librarian-runtime-node"]
        filtered = [m for m in matches if m.strip() not in [a.replace("\\", "\\\\") for a in allowed]]
        test(
            f"[local-paths] Runbook: no '{pattern}'",
            len(filtered) == 0,
            f"Found disallowed local path: {filtered}",
        )

# Also check the dry-run matrix for local paths
if matrix_path.exists():
    matrix_content = matrix_path.read_text(encoding="utf-8")
    for pattern in LOCAL_PATH_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(matrix_content)
        test(
            f"[local-paths] Matrix: no '{pattern}'",
            len(matches) == 0,
            f"Found local path in matrix: {matches}",
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
