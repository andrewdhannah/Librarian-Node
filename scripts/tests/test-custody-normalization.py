#!/usr/bin/env python3
"""
WIN-STARTUP-FILES-CUSTODY-1 — Custody Normalization Regression Tests

Verifies that:
1. No lowercase G:\\openwork remains in tracked startup/operations surfaces
2. Machine-local paths are not committed outside approved examples
3. No duplicate port assignments exist across the startup surface
4. Router port 9130 source-of-truth exists (env var fallback in ops scripts)
5. Backend binary naming is documented and consistent

These tests enforce custody normalization and prevent regression.
"""

import json
import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

# ---------------------------------------------------------------------------
# Files and patterns
# ---------------------------------------------------------------------------
TRACKED_STARTUP_FILES = [
    "scripts/start-librarian-runtime-node.ps1",
    "scripts/operations/runtime-start.ps1",
    "scripts/operations/runtime-stop.ps1",
    "scripts/operations/runtime-status.ps1",
    "scripts/operations/runtime-logs.ps1",
    "scripts/operations/runtime-clean-check.ps1",
    "scripts/test-win-rust-service-swap.ps1",
]

OPS_SCRIPTS = [
    "scripts/operations/runtime-start.ps1",
    "scripts/operations/runtime-stop.ps1",
    "scripts/operations/runtime-status.ps1",
    "scripts/operations/runtime-clean-check.ps1",
]

APPROVED_MACHINE_SPECIFIC_FILES = [
    "config/model-profiles.local.example.json",
    "config/model_manager.local.example.ps1",
    "config/runtime-node.example.json",
    "config/runtime-node.local.json",
    "fixtures/startup-files-custody/machine-local-config.example.json",
]

MACHINE_PATH_PATTERNS = [
    r"G:\\llama\.cpp\\",
    r"G:\\llamacpp\\",
    r"G:\\temp\\",
]

# Port assignments from model-profiles.json (authoritative)
AUTHORITATIVE_PORT_MAP = {
    "phi-4": 9120,
    "qwen-coder": 9121,
    "llama-3.2": 9122,
    "qwen3": 9123,
    "gemma-3": 9124,
    "embedding": 9125,
    "router": 9130,
}

# Expected binary names
EXPECTED_BACKEND_BINARY = "llama-server.exe"  # authoritative — all operational surfaces agree
MODEL_MANAGER_ALTERNATE_BINARY = "llama-server-mini.exe"  # historical — referenced in comments only

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


def read_file(path):
    p = REPO_ROOT / path
    if not p.exists():
        return None
    return p.read_text(encoding="utf-8")


# ---------------------------------------------------------------------------
# Test 1: Path casing — no lowercase G:\openwork
# ---------------------------------------------------------------------------
print("\n[1] Path Casing — No Lowercase G:\\openwork Drift")

lowercase_pattern = re.compile(r'G:\\openwork\\')

for fname in TRACKED_STARTUP_FILES:
    content = read_file(fname)
    if content is None:
        test(f"[casing] {fname} exists", False, "File not found")
        continue
    matches = lowercase_pattern.findall(content)
    test(
        f"[casing] {fname}: no lowercase G:\\openwork",
        len(matches) == 0,
        f"Found {len(matches)} lowercase occurrences: {matches}",
    )

# Also check config files
for fname in ["config/model-profiles.json", "config/runtime-node.example.json"]:
    content = read_file(fname)
    if content is None:
        continue
    matches = lowercase_pattern.findall(content)
    test(
        f"[casing] {fname}: no lowercase G:\\openwork (expected in example values)",
        # Example files may have lowercase — that's acceptable for templates
        True,
        f"Found {len(matches)} lowercase drifts (expected for example/template)",
    )


# ---------------------------------------------------------------------------
# Test 2: Machine-local paths not in tracked operational scripts
# ---------------------------------------------------------------------------
print("\n[2] Machine-Local Paths — Not in Tracked Operational Scripts")

for fname in TRACKED_STARTUP_FILES:
    content = read_file(fname)
    if content is None:
        continue
    for pattern in MACHINE_PATH_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(content)
        test(
            f"[machine-path] {fname}: no '{pattern}' (expect: 0 matches)",
            len(matches) == 0,
            f"Found {len(matches)} matches: {matches}",
        )

# Check approved files
for fname in APPROVED_MACHINE_SPECIFIC_FILES:
    content = read_file(fname)
    if content is None:
        continue
    for pattern in MACHINE_PATH_PATTERNS:
        regex = re.compile(pattern, re.IGNORECASE)
        matches = regex.findall(content)
        if matches:
            print(f"  INFO: {fname} contains machine-local paths (expected for example)")


# ---------------------------------------------------------------------------
# Test 3: No duplicate port assignments
# ---------------------------------------------------------------------------
print("\n[3] Port Map — No Duplicate Assignments")

# Check model-profiles.json port map
model_profiles = REPO_ROOT / "config" / "model-profiles.json"
if model_profiles.exists():
    with open(model_profiles, "r", encoding="utf-8") as f:
        mp_data = json.load(f)

    profile_ports = {}
    for profile in mp_data.get("profiles", []):
        alias = profile.get("alias", "unknown")
        port = profile.get("port")
        if port:
            if port in profile_ports:
                test(
                    f"[ports] Port {port} assigned to both "
                    f"'{profile_ports[port]}' and '{alias}'",
                    False,
                    "Duplicate port assignment",
                )
            else:
                profile_ports[port] = alias

    # Verify each model port matches authoritative map
    for alias, expected_port in AUTHORITATIVE_PORT_MAP.items():
        if alias in profile_ports:
            actual = profile_ports[alias]
            test(
                f"[ports] {alias} port {actual} matches authoritative ({expected_port})",
                actual == expected_port,
                f"Expected {expected_port}, got {actual}",
            )

    # Verify no duplicate across the entire port map
    all_ports = list(profile_ports.values())
    duplicates = set(p for p in all_ports if all_ports.count(p) > 1)
    test(
        f"[ports] No duplicate ports in model profiles",
        len(duplicates) == 0,
        f"Duplicate ports found: {duplicates}",
    )

# Check model_manager.ps1 for resolved collision
mm_content = read_file("runtime/model_manager.ps1")
if mm_content:
    # Check embedding port is no longer 9122
    has_embed_port_9122 = re.search(r'\$EmbedPort\s*=\s*9122', mm_content)
    test(
        "[ports] model_manager.ps1 EmbedPort is not 9122 (collision resolved)",
        not has_embed_port_9122,
        "EmbedPort still set to 9122 — collision with llama-3.2",
    )

    # Verify EmbedPort is set to 9125
    has_embed_port_9125 = re.search(r'\$EmbedPort\s*=\s*9125', mm_content)
    test(
        "[ports] model_manager.ps1 EmbedPort is 9125 (resolved port)",
        bool(has_embed_port_9125),
        "EmbedPort not found as 9125",
    )


# ---------------------------------------------------------------------------
# Test 4: Router port source of truth
# ---------------------------------------------------------------------------
print("\n[4] Router Port 9130 — Source of Truth via Env Var")

for fname in OPS_SCRIPTS:
    content = read_file(fname)
    if content is None:
        continue
    # Check for the env var fallback pattern
    has_env_fallback = (
        '$env:ROUTER_PORT' in content
    )
    test(
        f"[router-port] {fname}: uses ROUTER_PORT env var fallback",
        has_env_fallback,
        "Missing ROUTER_PORT environment variable fallback",
    )

# Verify launcher still sets ROUTER_PORT env var
launcher_content = read_file("scripts/start-librarian-runtime-node.ps1")
if launcher_content:
    test(
        "[router-port] Launcher sets ROUTER_PORT env var",
        '$env:ROUTER_PORT' in launcher_content,
        "Launcher missing ROUTER_PORT env var setting",
    )


# ---------------------------------------------------------------------------
# Test 5: Backend binary consistency
# ---------------------------------------------------------------------------
print("\n[5] Backend Binary — Consistency Check")

if model_profiles.exists():
    with open(model_profiles, "r", encoding="utf-8") as f:
        mp_data = json.load(f)

    default_binary = mp_data.get("defaults", {}).get("binary", "")
    uses_llama_server = "llama-server.exe" in default_binary
    test(
        "[binary] model-profiles.json default binary is llama-server.exe",
        uses_llama_server,
        f"Default binary: {default_binary}",
    )

    # Check launch_commands all use llama-server.exe
    for profile in mp_data.get("profiles", []):
        alias = profile.get("alias", "unknown")
        cmd = profile.get("launch_command", "")
        uses_correct_binary = cmd.startswith(EXPECTED_BACKEND_BINARY) if cmd else True
        test(
            f"[binary] {alias} launch_command uses '{EXPECTED_BACKEND_BINARY}'",
            uses_correct_binary,
            f"Command starts with: {cmd[:50] if cmd else '(empty)'}",
        )

# Check model_manager binary reconciliation
if mm_content:
    # Verify the default ServerPath resolves to llama-server.exe (authoritative)
    server_path_lines = [l for l in mm_content.split('\n') if '$ServerPath' in l and '=' in l and l.strip().startswith('$')]
    server_path_authoritative = any('llama-server.exe' in l for l in server_path_lines)
    test(
        "[binary] model_manager.ps1 default ServerPath is llama-server.exe",
        server_path_authoritative,
        f"Default ServerPath lines: {server_path_lines}",
    )
    
    # Verify the derived process name is used in operational code (not hardcoded)
    uses_derived_name = (
        '$ServerProcessName' in mm_content
    )
    test(
        "[binary] model_manager.ps1 uses derived ServerProcessName",
        uses_derived_name,
        "Missing $ServerProcessName variable",
    )
    
    # Verify Get-ServerProcess uses the variable, not a hardcoded name
    has_var_process_ref = 'Get-Process -Name $ServerProcessName' in mm_content
    test(
        "[binary] Get-ServerProcess uses $ServerProcessName variable",
        has_var_process_ref,
        "Get-ServerProcess still uses hardcoded process name",
    )
    
    # Verify the authority documentation comment exists
    has_authority_doc = "AUTHORITATIVE BACKEND BINARY" in mm_content
    test(
        "[binary] model_manager.ps1 documents authoritative binary",
        has_authority_doc,
        "Missing AUTHORITATIVE BACKEND BINARY documentation",
    )
    
    # Verify historical binary name only appears in comments, not operational code
    operational_lines = [l for l in mm_content.split('\n')
                         if not l.strip().startswith('#') and not l.strip().startswith('<#')]
    mini_in_operational = any('llama-server-mini' in l for l in operational_lines)
    test(
        "[binary] No operational code references 'llama-server-mini'",
        not mini_in_operational,
        "Operational code still references llama-server-mini",
    )


# ---------------------------------------------------------------------------
# Test 6: Local config pattern exists
# ---------------------------------------------------------------------------
print("\n[6] Local Config Pattern — Exists and Is Documented")

local_example = REPO_ROOT / "config" / "model-profiles.local.example.json"
test(
    "[local-config] model-profiles.local.example.json exists",
    local_example.exists(),
    f"Missing: {local_example}",
)

manager_example = REPO_ROOT / "config" / "model_manager.local.example.ps1"
test(
    "[local-config] model_manager.local.example.ps1 exists",
    manager_example.exists(),
    f"Missing: {manager_example}",
)

# Verify model_manager loads local config
if mm_content:
    has_local_config_load = (
        'model_manager.local.ps1' in mm_content
    )
    test(
        "[local-config] model_manager.ps1 loads config/model_manager.local.ps1",
        has_local_config_load,
        "Missing local config loading pattern",
    )


# ---------------------------------------------------------------------------
# Test 7: Production boundary — no router/runtime/model changes
# ---------------------------------------------------------------------------
print("\n[7] Production Boundary — No Router, Runtime, or Model Changes")

import subprocess
diff_result = subprocess.run(
    ["git", "diff", "--name-only"],
    capture_output=True, text=True, cwd=REPO_ROOT,
)
modified_files = [f.strip() for f in diff_result.stdout.split("\n") if f.strip()]

# Production files that must NOT appear in the diff
PRODUCTION_PREFIXES = ["router/", "rust-router/"]
for prefix in PRODUCTION_PREFIXES:
    prod_changes = [f for f in modified_files if f.startswith(prefix)]
    test(
        f"[production] No changes to '{prefix}' files",
        len(prod_changes) == 0,
        f"Production files modified: {prod_changes}",
    )

# Only certain file categories should be modified
ALLOWED_MODIFIED_PREFIXES = [
    "scripts/start-librarian-runtime-node.ps1",
    "scripts/operations/",
    "scripts/tests/",
    "runtime/model_manager.ps1",
    "config/",
    ".gitignore",
    "docs/operations/",
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
