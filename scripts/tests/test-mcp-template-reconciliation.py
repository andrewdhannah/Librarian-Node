#!/usr/bin/env python3
"""
WIN-MCP-TEMPLATE-RECONCILE-1 — MCP Template Reconciliation Validation Tests

Verifies that:
1. MCP template/example files exist in expected locations.
2. Windows MCP templates do NOT depend on 'swift run LibrarianServer'.
3. Windows-native MCP example exists with a documented command/path pattern.
4. No real machine-local path is committed in the MCP template.
5. macOS MCP examples remain preserved (if they exist).
6. Platform separation is clearly documented.
7. MCP bridge scripts (PS1) do not depend on bash/curl/python3.
8. Production router/runtime/model files are untouched.

These tests enforce template reconciliation and prevent platform-drift regression.
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
MCP_TEMPLATE_FILE = "mcp/templates/mcp-server.example.json"
MCP_BRIDGE_PS1 = "scripts/mcp-bridge.ps1"
MCP_HEALTH_PS1 = "scripts/check-mcp-health.ps1"

# MCP template paths that must exist
REQUIRED_MCP_FILES = [
    MCP_TEMPLATE_FILE,
    MCP_BRIDGE_PS1,
    MCP_HEALTH_PS1,
]

# Patterns that must NOT appear in any Windows MCP template/script
FORBIDDEN_MACOS_PATTERNS = [
    r"swift run LibrarianServer",
    r"swift\s+run",
    r"mcp-bridge\.sh",
    r"check-mcp-health\.sh",
    r"/bin/bash",
    r"/usr/bin/env bash",
    r"#!/bin/bash",
    r"#!/usr/bin/env bash",
    r"set -euo pipefail",
    r"python3 -c",
    r"curl -s",
]

# Machine-local path patterns (must not appear in MCP templates)
FORBIDDEN_PATH_PATTERNS = [
    r"G:\\llama\.cpp\\",
    r"G:\\llamacpp\\",
    r"G:\\temp\\",
    r"C:\\Users\\.*\\",
    r"D:\\Users\\.*\\",
]

# Allowed placeholder patterns for paths
ALLOWED_PLACEHOLDERS = [
    "<repo-root>",
    "<runtime-node-root>",
]

# macOS examples expected to be preserved
EXPECTED_MACOS_PLATFORM_TAG = "macOS_only"
EXPECTED_WINDOWS_PLATFORM_TAG = "windows_native_command"

# Production directories that must NOT appear in the diff
PRODUCTION_PREFIXES = ["router/", "rust-router/"]

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
# Test 1: Required MCP files exist
# ---------------------------------------------------------------------------
print("\n[1/10] Required MCP Files Exist")

for fname in REQUIRED_MCP_FILES:
    full_path = REPO_ROOT / fname
    test(
        f"[files] {fname} exists",
        full_path.exists(),
        f"Missing: {full_path}",
    )

# ---------------------------------------------------------------------------
# Test 2: MCP template is valid JSON with expected structure
# ---------------------------------------------------------------------------
print("\n[2/10] MCP Template Structure")

template = load_json(MCP_TEMPLATE_FILE)
if template is not None:
    test(
        "[structure] Template has '_meta' section",
        "_meta" in template,
        "Missing _meta section",
    )
    test(
        "[structure] Template has 'examples' array",
        "examples" in template,
        "Missing examples array",
    )
    test(
        "[structure] Template has 'platform_key' section",
        "platform_key" in template,
        "Missing platform_key section",
    )

    # Check examples structure
    examples = template.get("examples", [])
    test(
        "[structure] At least one example exists",
        len(examples) > 0,
        "No examples found",
    )

    for i, ex in enumerate(examples):
        ex_id = f"example[{i}] ({ex.get('platform', 'unknown')})"
        test(
            f"[structure] {ex_id} has 'platform'",
            "platform" in ex,
            f"Missing platform in {ex_id}",
        )
        test(
            f"[structure] {ex_id} has 'label'",
            "label" in ex,
            f"Missing label in {ex_id}",
        )
        test(
            f"[structure] {ex_id} has 'type'",
            "type" in ex,
            f"Missing type in {ex_id}",
        )
        test(
            f"[structure] {ex_id} has 'command'",
            "command" in ex,
            f"Missing command in {ex_id}",
        )
        test(
            f"[structure] {ex_id} has 'macOS_only' flag",
            "macOS_only" in ex,
            f"Missing macOS_only flag in {ex_id}",
        )
        test(
            f"[structure] {ex_id} has 'windows_native_command' flag",
            "windows_native_command" in ex,
            f"Missing windows_native_command flag in {ex_id}",
        )

else:
    test("[structure] Template loads as valid JSON", False, f"Could not load {MCP_TEMPLATE_FILE}")

# ---------------------------------------------------------------------------
# Test 3: No 'swift run LibrarianServer' in Windows templates
# ---------------------------------------------------------------------------
print("\n[3/10] No macOS-Specific Commands in Windows Templates")

# Check the MCP template JSON — only check Windows examples for macOS patterns
# Allow documentation cross-references in 'note' fields
if template is not None:
    examples = template.get("examples", [])
    windows_examples_for_test = [
        ex for ex in examples
        if ex.get("platform") == "windows" or ex.get("macOS_only") is not True
    ]
    if windows_examples_for_test:
        for pattern in FORBIDDEN_MACOS_PATTERNS:
            regex = re.compile(pattern)
            found_in_windows = False
            details = ""
            for ex in windows_examples_for_test:
                # Check operational fields only (label, command, args, type, env)
                # Exclude 'note' field which may contain cross-references to macOS
                ex_operational = {
                    k: v for k, v in ex.items()
                    if k != "note"
                }
                ex_str = json.dumps(ex_operational)
                matches = regex.findall(ex_str)
                if matches:
                    found_in_windows = True
                    details = f"in '{ex.get('label', '?')}': {matches}"
                    break
            test(
                f"[macos-pattern] No Windows example contains '{pattern}'",
                not found_in_windows,
                details,
            )

# Check PowerShell scripts — allow cross-references in comment/help blocks
def get_operational_code(content):
    """Strip PowerShell comment/help blocks to get operational code only."""
    # Remove <# ... #> block comments
    text = re.sub(r'<#.*?#>', '', content, flags=re.DOTALL)
    # Remove single-line comments
    lines = []
    for l in text.split('\n'):
        stripped = l.strip()
        if stripped.startswith('#'):
            continue
        lines.append(l)
    return '\n'.join(lines)

for ps1_file in [MCP_BRIDGE_PS1, MCP_HEALTH_PS1]:
    content = read_file(ps1_file)
    if content is not None:
        operational = get_operational_code(content)
        for pattern in FORBIDDEN_MACOS_PATTERNS:
            regex = re.compile(pattern)
            matches = regex.findall(operational)
            test(
                f"[macos-pattern] {ps1_file} operational code: no '{pattern}'",
                len(matches) == 0,
                f"Found {len(matches)} occurrence(s) in operational code: {matches}",
            )

# ---------------------------------------------------------------------------
# Test 4: Windows-native example exists
# ---------------------------------------------------------------------------
print("\n[4/10] Windows-Native Examples Exist")

if template is not None:
    examples = template.get("examples", [])

    windows_examples = [ex for ex in examples if ex.get("platform") == "windows"]
    test(
        "[windows] At least one Windows-native example exists",
        len(windows_examples) > 0,
        f"Found {len(windows_examples)} Windows example(s)",
    )

    windows_native_examples = [
        ex for ex in examples if ex.get("windows_native_command") is True
    ]
    test(
        "[windows] At least one example with windows_native_command=true",
        len(windows_native_examples) > 0,
        "No example flagged as windows_native_command",
    )

    # Check each Windows example uses placeholders, not machine-local paths
    for ex in windows_examples:
        ex_str = json.dumps(ex)
        for pattern in FORBIDDEN_PATH_PATTERNS:
            regex = re.compile(pattern, re.IGNORECASE)
            matches = regex.findall(ex_str)
            test(
                f"[windows] '{ex.get('label', '?')}': no '{pattern}'",
                len(matches) == 0,
                f"Found machine-local path: {matches}",
            )

    # Check examples use placeholder patterns
    for ex in windows_examples:
        ex_str = json.dumps(ex)
        has_placeholder = any(ph in ex_str for ph in ALLOWED_PLACEHOLDERS)
        test(
            f"[windows] '{ex.get('label', '?')}': uses documented placeholder",
            has_placeholder,
            f"No placeholder found (expected one of: {ALLOWED_PLACEHOLDERS})",
        )

# ---------------------------------------------------------------------------
# Test 5: macOS examples preserved
# ---------------------------------------------------------------------------
print("\n[5/10] macOS Examples Preserved")

if template is not None:
    examples = template.get("examples", [])

    macos_examples = [ex for ex in examples if ex.get("macOS_only") is True]
    test(
        "[macos] At least one macOS-only example preserved",
        len(macos_examples) > 0,
        f"Found {len(macos_examples)} macOS-only example(s) — expected at least 1",
    )

    # Verify macOS examples are clearly labeled
    for ex in macos_examples:
        test(
            f"[macos] '{ex.get('label', '?')}': platform is 'macOS'",
            ex.get("platform") == "macOS",
            f"Platform is '{ex.get('platform')}' instead of 'macOS'",
        )
        test(
            f"[macos] '{ex.get('label', '?')}': macOS_only is true",
            ex.get("macOS_only") is True,
            "macOS_only flag is not True",
        )
        test(
            f"[macos] '{ex.get('label', '?')}': windows_native_command is null or false",
            ex.get("windows_native_command") is None or ex.get("windows_native_command") is False,
            f"windows_native_command is {ex.get('windows_native_command')} on a macOS example",
        )

    # Verify macOS examples DO reference Swift (they should, they're macOS)
    for ex in macos_examples:
        ex_str = json.dumps(ex)
        has_swift = "swift" in ex_str.lower() or "mcp-bridge.sh" in ex_str
        test(
            f"[macos] '{ex.get('label', '?')}': references Swift or mcp-bridge.sh (expected)",
            has_swift,
            "macOS example doesn't reference Swift or mcp-bridge.sh (unexpected for macOS)",
        )

# ---------------------------------------------------------------------------
# Test 6: No real machine-local paths in MCP scripts
# ---------------------------------------------------------------------------
print("\n[6/10] No Machine-Local Paths in MCP Scripts")

for ps1_file in [MCP_BRIDGE_PS1, MCP_HEALTH_PS1]:
    content = read_file(ps1_file)
    if content is not None:
        for pattern in FORBIDDEN_PATH_PATTERNS:
            regex = re.compile(pattern, re.IGNORECASE)
            matches = regex.findall(content)
            test(
                f"[machine-path] {ps1_file}: no '{pattern}'",
                len(matches) == 0,
                f"Found machine-local path: {matches}",
            )

# Also check no raw user home paths in MCP template
if template is not None:
    template_text = json.dumps(template)
    for pattern in FORBIDDEN_PATH_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(template_text)
        test(
            f"[machine-path] Template JSON: no '{pattern}'",
            len(matches) == 0,
            f"Found machine-local path: {matches}",
        )

# ---------------------------------------------------------------------------
# Test 7: Windows MCP scripts do not depend on bash/curl/python3
# ---------------------------------------------------------------------------
print("\n[7/10] No bash/curl/python3 Dependencies in Windows MCP Scripts")

PS_FORBIDDEN_CMDS = [
    "curl ",
    "python3",
    "/bin/bash",
    "bash ",
    ".sh",
]

for ps1_file in [MCP_BRIDGE_PS1, MCP_HEALTH_PS1]:
    content = read_file(ps1_file)
    if content is not None:
        operational_text = get_operational_code(content)
        for cmd in PS_FORBIDDEN_CMDS:
            regex = re.compile(re.escape(cmd), re.IGNORECASE)
            matches = regex.findall(operational_text)
            test(
                f"[deps] {ps1_file}: no '{cmd}' in operational code",
                len(matches) == 0,
                f"Found {len(matches)} occurrence(s): {matches}",
            )

# ---------------------------------------------------------------------------
# Test 8: Platform separation is clearly documented
# ---------------------------------------------------------------------------
print("\n[8/10] Platform Separation Documentation")

if template is not None:
    # Check platform_key explains both platform tags
    platform_key = template.get("platform_key", {})
    test(
        "[platform] platform_key explains 'macOS_only'",
        "macOS_only" in str(platform_key),
        "platform_key missing macOS_only explanation",
    )
    test(
        "[platform] platform_key explains 'windows_native_command'",
        "windows_native_command" in str(platform_key),
        "platform_key missing windows_native_command explanation",
    )

    # Check meta section references platform separation
    meta = template.get("_meta", {})
    meta_str = json.dumps(meta)
    test(
        "[platform] _meta mentions platform separation",
        "platform_separation" in meta_str,
        "_meta missing platform_separation note",
    )

    # Verify each example has a clear platform label
    examples = template.get("examples", [])
    for ex in examples:
        label = ex.get("label", "")
        platform = ex.get("platform", "")
        test(
            f"[platform] Example platform '{platform}' is valid",
            platform in ("macOS", "windows"),
            f"Unexpected platform value: '{platform}'",
        )

# ---------------------------------------------------------------------------
# Test 9: MCP bridge script has correct structure
# ---------------------------------------------------------------------------
print("\n[9/10] MCP Bridge Script Structure")

bridge_content = read_file(MCP_BRIDGE_PS1)
if bridge_content is not None:
    # Check it's a proper PowerShell script with help
    test(
        "[bridge] Has .SYNOPSIS comment",
        ".SYNOPSIS" in bridge_content,
        "Missing .SYNOPSIS help comment",
    )
    test(
        "[bridge] Reads from stdin",
        "Console]::In.ReadLine" in bridge_content or "ReadLine()" in bridge_content,
        "Does not read from stdin via Console.ReadLine",
    )
    test(
        "[bridge] Uses LIBRARIAN_MCP_URL env var",
        "LIBRARIAN_MCP_URL" in bridge_content,
        "Missing LIBRARIAN_MCP_URL environment variable reference",
    )
    test(
        "[bridge] Uses Invoke-RestMethod (not curl)",
        "Invoke-RestMethod" in bridge_content,
        "Uses curl instead of Invoke-RestMethod",
    )
    test(
        "[bridge] Has error handling (try/catch)",
        "try" in bridge_content and "catch" in bridge_content,
        "Missing try/catch error handling",
    )
    test(
        "[bridge] Default MCP URL is http://127.0.0.1:3456/mcp",
        "http://127.0.0.1:3456/mcp" in bridge_content,
        "Default MCP URL is not http://127.0.0.1:3456/mcp",
    )

health_content = read_file(MCP_HEALTH_PS1)
if health_content is not None:
    # Check for similar structure
    test(
        "[health] Has .SYNOPSIS comment",
        ".SYNOPSIS" in health_content,
        "Missing .SYNOPSIS help comment",
    )
    test(
        "[health] Uses Invoke-RestMethod (not curl)",
        "Invoke-RestMethod" in health_content,
        "Uses curl instead of Invoke-RestMethod",
    )
    test(
        "[health] Has error handling (try/catch)",
        "try" in health_content and "catch" in health_content,
        "Missing try/catch error handling",
    )
    test(
        "[health] References expected MCP tools",
        all(tool in health_content for tool in
            ["librarian_checkin", "librarian_checkout", "librarian_search"]),
        "Missing expected MCP tool references",
    )

# ---------------------------------------------------------------------------
# Test 10: Production boundary — no router/runtime/model changes
# ---------------------------------------------------------------------------
print("\n[10/10] Production File Boundary")

import subprocess
diff_result = subprocess.run(
    ["git", "diff", "--name-only"],
    capture_output=True, text=True, cwd=REPO_ROOT,
)
modified_files = [f.strip() for f in diff_result.stdout.split("\n") if f.strip()]

# Check production files not modified
for prefix in PRODUCTION_PREFIXES:
    prod_changes = [f for f in modified_files if f.startswith(prefix)]
    test(
        f"[production] No changes to '{prefix}' files",
        len(prod_changes) == 0,
        f"Production files modified: {prod_changes}",
    )

# Only certain categories should be modified
ALLOWED_MODIFIED_PREFIXES = [
    "mcp/",
    "scripts/mcp-bridge.ps1",
    "scripts/check-mcp-health.ps1",
    "scripts/tests/",
    "docs/sprints/",
    "docs/operations/",
    "reports/",
    "config/",
    ".gitignore",
]
unexpected = [
    f for f in modified_files
    if not any(f.startswith(p) for p in ALLOWED_MODIFIED_PREFIXES)
]
test(
    "[production] All modified files are in allowed categories",
    len(unexpected) == 0,
    f"Unexpected modified files: {unexpected}",
)

# Check new files are in allowed paths
status_result = subprocess.run(
    ["git", "status", "--short"],
    capture_output=True, text=True, cwd=REPO_ROOT,
)
new_files = [line.strip() for line in status_result.stdout.split("\n") if line.strip()]
for nf in new_files:
    if nf.startswith("?? "):
        fname = nf[3:].replace("\\", "/")
        is_allowed = any(fname.startswith(pref) for pref in ALLOWED_MODIFIED_PREFIXES)
        test(
            f"[production] New file in allowed path: {fname}",
            is_allowed,
            f"Unexpected new file: {fname}",
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
