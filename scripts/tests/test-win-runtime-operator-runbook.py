#!/usr/bin/env python3
"""
WIN-RUNTIME-OPERATOR-RUNBOOK-1 — Operator Runbook Validation Tests

Verifies that:
1. The operator runbook file exists at the expected path.
2. It contains all required sections (config setup, port check, binary check,
   MCP health check, start/stop procedure, log capture, failure triage,
   do-not-proceed conditions).
3. It references the authoritative port map (9130, 9120-9125).
4. It references 'llama-server.exe' as the authorized backend binary.
5. It references 'scripts/check-mcp-health.ps1' for MCP health checks.
6. It contains explicit "do not proceed" conditions.
7. It does NOT instruct agents to auto-start services or auto-run models.
8. It does NOT contain machine-local paths.
9. It references authoritative config file sources.

These tests enforce runbook completeness and prevent operator-guide regression.
"""

import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

# ---------------------------------------------------------------------------
# Files under test
# ---------------------------------------------------------------------------
RUNBOOK_PATH = "docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md"

# Required sections (as section headings)
REQUIRED_SECTIONS = [
    "Overview",
    "Prerequisites",
    "Local Config Setup Checklist",
    "Port Verification Checklist",
    "Backend Binary Verification Checklist",
    "MCP Health Check Usage",
    "Service Start/Stop Procedure",
    "Log and Evidence Capture",
    "Failure Triage",
    "Do Not Proceed Conditions",
]

# Required section-like markers (H2 or H3 headings)
REQUIRED_SECTION_PATTERNS = [
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
]

# Authoritative port numbers that must appear
REQUIRED_PORTS = ["9130", "9120", "9121", "9122", "9123", "9124", "9125"]

# Reference strings that must appear
REQUIRED_REFERENCES = [
    "llama-server.exe",
    "scripts/check-mcp-health.ps1",
    "scripts/mcp-bridge.ps1",
    "scripts/operations/runtime-start.ps1",
    "scripts/operations/runtime-stop.ps1",
    "scripts/operations/runtime-status.ps1",
    "config/model-profiles.json",
    "config/model-profiles.local.example.json",
    "config/model_manager.local.example.ps1",
    "config/runtime-node.example.json",
    "LibrarianRunTimeNode",
    "Manual",
]

# "Do not proceed" conditions that must be present
REQUIRED_DO_NOT_PROCEED_PATTERNS = [
    r"do not proceed",
    r"Hard Stop",
    r"Policy Stop",
    r"Boundary Stop",
    r"unexpected.*LISTENING",
    r"orphan",
    r"not.*running.*as.*Administrator",
]

# Patterns that must NOT appear (no auto-service, no auto-model)
FORBIDDEN_AUTO_PATTERNS = [
    r"Start-Service .*automatically",
    r"auto-start",
    r"automatically.*start.*service",
    r"automatically.*run.*model",
    r"auto-run.*model",
    r"Start-Process.*llama-server",
    r"Start-Service.*without.*human",
]

# Machine-local path patterns (must not appear)
FORBIDDEN_PATH_PATTERNS = [
    r"G:\\llama\.cpp\\",
    r"G:\\llamacpp\\",
    r"G:\\temp\\",
    r"C:\\Users\\.*\\",
    r"D:\\Users\\.*\\",
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


def get_operational_code(content):
    """Strip comment/help blocks to get operational code. (Not needed for markdown,
    but kept for consistency with sister tests.)"""
    return content


# ---------------------------------------------------------------------------
# Test 1: Runbook file exists
# ---------------------------------------------------------------------------
print("\n[1/12] Runbook File Exists")

full_path = REPO_ROOT / RUNBOOK_PATH
test(
    "[file] Operator runbook exists",
    full_path.exists(),
    f"Missing: {full_path}",
)

runbook = read_file(RUNBOOK_PATH)

# ---------------------------------------------------------------------------
# Test 2: All required sections present
# ---------------------------------------------------------------------------
print("\n[2/12] Required Sections Present")

if runbook is not None:
    for pattern in REQUIRED_SECTION_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        match = regex.search(runbook)
        test(
            f"[section] Contains section matching '{pattern}'",
            bool(match),
            f"Pattern not found: {pattern}",
        )
else:
    for pattern in REQUIRED_SECTION_PATTERNS:
        test(f"[section] Contains section matching '{pattern}'", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 3: Authoritative port map referenced
# ---------------------------------------------------------------------------
print("\n[3/12] Authoritative Port Map Referenced")

if runbook is not None:
    for port in REQUIRED_PORTS:
        test(
            f"[ports] References port {port}",
            port in runbook,
            f"Port {port} not found in runbook",
        )
else:
    for port in REQUIRED_PORTS:
        test(f"[ports] References port {port}", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 4: Authorized backend binary referenced
# ---------------------------------------------------------------------------
print("\n[4/12] Authorized Backend Binary Referenced")

if runbook is not None:
    test(
        "[binary] References 'llama-server.exe' as authorized binary",
        "llama-server.exe" in runbook,
        "llama-server.exe not referenced",
    )
    # Check it does NOT reference deprecated binary as authorized (allow in docs context)
    deprecated_refs = re.findall(r"llama-server-mini\.exe", runbook)
    # Allow references that are in deprecation warnings or documented as historical
    # At least one reference must include a deprecation warning
    deprecated_warning_lines = [
        l for l in runbook.split("\n")
        if "llama-server-mini" in l
        and ("deprecated" in l.lower() or "do not use" in l.lower())
    ]
    test(
        "[binary] At least one llama-server-mini.exe reference has deprecation warning",
        len(deprecated_warning_lines) >= 1,
        f"No deprecation warning found in {len(deprecated_refs)} reference(s)",
    )
    # The authoritative binary reference must be llama-server.exe, not mini
    authoritative_line = [
        l for l in runbook.split("\n")
        if "Authoritative backend binary" in l
    ]
    test(
        "[binary] Authoritative binary designation calls out llama-server.exe",
        any("llama-server.exe" in l for l in authoritative_line),
        "Authoritative binary does not reference llama-server.exe",
    )
else:
    test("[binary] References 'llama-server.exe'", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 5: MCP health check script referenced
# ---------------------------------------------------------------------------
print("\n[5/12] MCP Health Check Referenced")

if runbook is not None:
    test(
        "[mcp] References 'scripts/check-mcp-health.ps1'",
        "scripts/check-mcp-health.ps1" in runbook,
        "Missing reference to scripts/check-mcp-health.ps1",
    )
    test(
        "[mcp] References 'scripts/mcp-bridge.ps1'",
        "scripts/mcp-bridge.ps1" in runbook,
        "Missing reference to scripts/mcp-bridge.ps1",
    )
    # Check it mentions MCP exit codes
    test(
        "[mcp] Mentions MCP health check exit codes",
        "exit code" in runbook.lower() or "Exit Code" in runbook,
        "Missing MCP exit code documentation",
    )
else:
    test("[mcp] References health check script", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 6: Required references present
# ---------------------------------------------------------------------------
print("\n[6/12] Required Reference Strings Present")

if runbook is not None:
    for ref in REQUIRED_REFERENCES:
        test(
            f"[ref] References '{ref}'",
            ref in runbook,
            f"Missing reference: {ref}",
        )
else:
    for ref in REQUIRED_REFERENCES:
        test(f"[ref] References '{ref}'", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 7: "Do not proceed" conditions present
# ---------------------------------------------------------------------------
print("\n[7/12] Do Not Proceed Conditions Present")

if runbook is not None:
    # Check for the main section heading
    test(
        "[stop] Has 'Do Not Proceed Conditions' section",
        bool(re.search(r"Do Not Proceed", runbook, re.IGNORECASE)),
        "Missing Do Not Proceed Conditions section",
    )

    for pattern in REQUIRED_DO_NOT_PROCEED_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        test(
            f"[stop] Contains condition pattern '{pattern}'",
            bool(regex.search(runbook)),
            f"Pattern not found: {pattern}",
        )
else:
    for pattern in REQUIRED_DO_NOT_PROCEED_PATTERNS:
        test(f"[stop] Contains condition pattern '{pattern}'", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 8: No auto-service or auto-model instructions
# ---------------------------------------------------------------------------
print("\n[8/12] No Auto-Service or Auto-Model Instructions")

if runbook is not None:
    for pattern in FORBIDDEN_AUTO_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(runbook)
        test(
            f"[no-auto] Does NOT contain '{pattern}'",
            len(matches) == 0,
            f"Found {len(matches)} match(es): {matches}",
        )

    # Check the start/stop section says "do not automate"
    test(
        "[no-auto] Explicitly says 'do not automate this sequence'",
        "do not automate this sequence" in runbook.lower(),
        "Missing explicit do-not-automate warning",
    )
else:
    test("[no-auto] No auto patterns", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 9: No machine-local paths committed
# ---------------------------------------------------------------------------
print("\n[9/12] No Machine-Local Paths in Runbook")

if runbook is not None:
    for pattern in FORBIDDEN_PATH_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(runbook)
        test(
            f"[machine-path] Runbook: no '{pattern}'",
            len(matches) == 0,
            f"Found machine-local path: {matches}",
        )
else:
    for pattern in FORBIDDEN_PATH_PATTERNS:
        test(f"[machine-path] Runbook: no '{pattern}'", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 10: References authoritative config file sources
# ---------------------------------------------------------------------------
print("\n[10/12] References Authoritative Config Sources")

if runbook is not None:
    expected_config_refs = [
        "config/model-profiles.json",
        "config/model-profiles.local.example.json",
        "config/model_manager.local.example.ps1",
        "config/runtime-node.example.json",
        "config/runtime-node.local.json",
        "config/mcp-permissions.json",
        "fixtures/startup-files-custody/startup-custody-manifest.example.json",
    ]
    for ref in expected_config_refs:
        test(
            f"[config] References '{ref}'",
            ref in runbook,
            f"Missing config reference: {ref}",
        )
else:
    for ref in expected_config_refs:
        test(f"[config] References '{ref}'", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 11: Service start/stop are human instructions, not scripts
# ---------------------------------------------------------------------------
print("\n[11/12] Service Start/Stop Are Human Instructions")

if runbook is not None:
    # The start/stop section should have step-by-step instructions,
    # not just reference scripts
    section_7 = re.search(
        r"##\s+7\.\s+Service Start/Stop Procedure.*?(?=##\s+8\.)",
        runbook,
        re.DOTALL | re.IGNORECASE,
    )
    if section_7:
        s7_text = section_7.group(0)

        # Must contain step-by-step PowerShell commands for start
        test(
            "[start/stop] Start section has Step 1 through Step N markers",
            bool(re.search(r"Step \d", s7_text)),
            "Missing numbered steps in start procedure",
        )
        test(
            "[start/stop] Start procedure includes Start-Service command",
            "Start-Service" in s7_text,
            "Missing Start-Service in start procedure",
        )
        test(
            "[start/stop] Start procedure includes port verification",
            "netstat" in s7_text and "9130" in s7_text,
            "Missing port verification in start procedure",
        )
        test(
            "[start/stop] Stop procedure includes Stop-Service command",
            "Stop-Service" in s7_text,
            "Missing Stop-Service in stop procedure",
        )
        test(
            "[start/stop] Stop procedure includes orphan check",
            "Get-Process" in s7_text,
            "Missing orphan process check in stop procedure",
        )
    else:
        test("[start/stop] Section 7 found", False, "Could not extract Section 7")
else:
    test("[start/stop] Section 7 extraction", False, "Runbook not loaded")

# ---------------------------------------------------------------------------
# Test 12: Failure triage covers common failure modes
# ---------------------------------------------------------------------------
print("\n[12/12] Failure Triage Coverage")

if runbook is not None:
    section_9 = re.search(
        r"##\s+9\.\s+Failure Triage.*?(?=##\s+10\.)",
        runbook,
        re.DOTALL | re.IGNORECASE,
    )
    if section_9:
        s9_text = section_9.group(0)

        # Check for common failure modes
        test(
            "[triage] Covers 'service fails to start'",
            "Service Fails to Start" in s9_text or "service.*fail.*start" in s9_text.lower(),
            "Missing service-start failure mode",
        )
        test(
            "[triage] Covers 'service fails to stop'",
            "Service Fails to Stop" in s9_text or "service.*fail.*stop" in s9_text.lower(),
            "Missing service-stop failure mode",
        )
        test(
            "[triage] Covers orphan process cleanup",
            "Orphan Process Cleanup" in s9_text or "orphan.*process" in s9_text.lower(),
            "Missing orphan process cleanup procedure",
        )
        test(
            "[triage] Covers MCP health check failure",
            "MCP Health Check Fails" in s9_text or "MCP.*fail" in s9_text.lower(),
            "Missing MCP health check failure mode",
        )
        test(
            "[triage] Covers config parse error",
            "Config Parse Error" in s9_text or "config.*parse" in s9_text.lower() or "parse error" in s9_text.lower(),
            "Missing config parse error failure mode",
        )
    else:
        test("[triage] Section 9 found", False, "Could not extract Section 9")
else:
    test("[triage] Section 9 extraction", False, "Runbook not loaded")

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
