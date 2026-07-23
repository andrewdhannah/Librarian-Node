# WIN-MCP-TEMPLATE-RECONCILE-1 — Windows MCP Template Example Alignment

**Status**: CLOSED — PROMOTE  
**Date**: 2026-06-28  
**Repo**: `librarian-runtime-node`  
**Branch**: `main`

---

## 1. Objective

Reconcile Windows MCP template examples so tracked MCP startup examples no longer imply macOS-only launch behavior. The known gap was `mcp/templates/mcp-server.example.json`, which lacked a Windows-native example path.

## 2. Starting HEAD

```
0adf02d9aeb162470c07f7b0ad577f917b400d3e
0adf02d docs(runtime): reconcile backend binary authority to llama-server.exe
```

## 3. Final HEAD

```
8d2669cdbc6f65f2f10140fc1d0451e0de65e0a6
8d2669c feat(mcp): reconcile Windows-native MCP template examples
```

## 4. Files Changed

| File | Action | Description |
|------|--------|-------------|
| `mcp/templates/mcp-server.example.json` | Created | Platform-separate MCP server configuration template (5 examples: 2 macOS, 3 Windows) |
| `scripts/mcp-bridge.ps1` | Created | PowerShell stdio bridge for MCP (Windows-native equivalent of `mcp-bridge.sh`) |
| `scripts/check-mcp-health.ps1` | Created | PowerShell MCP health check (Windows-native equivalent of `check-mcp-health.sh`) |
| `scripts/tests/test-mcp-template-reconciliation.py` | Created | 149 validation tests covering template structure, platform separation, path safety, production boundary |

No existing files were modified.

## 5. MCP Template Paths Reconciled

| Template Path | Windows Coverage | macOS Coverage |
|---|---|---|
| `mcp/templates/mcp-server.example.json` | 3 examples | 2 examples (preserved) |
| `scripts/mcp-bridge.ps1` | Native bridge | Referenced as macOS counterpart |
| `scripts/check-mcp-health.ps1` | Native health check | Referenced as macOS counterpart |

## 6. Windows-Native Example Command/Path Pattern

All Windows examples use placeholders — no machine-local paths:

| Example | Command Pattern | Placeholder |
|---------|----------------|-------------|
| Compiled `LibrarianServer.exe` | `<repo-root>\LibrarianServer.exe` | `<repo-root>` |
| PowerShell stdio bridge | `powershell.exe -File <repo-root>\scripts\mcp-bridge.ps1` | `<repo-root>` |
| PowerShell bridge (runtime-node) | `powershell.exe -File <runtime-node-root>\scripts\mcp-bridge.ps1` | `<runtime-node-root>` |

Placeholder documentation in `platform_key.placeholders`:
- `<repo-root>` → e.g., `C:\Users\<user>\Projects\TheLibrarian`
- `<runtime-node-root>` → e.g., `C:\Users\<user>\Projects\librarian-runtime-node`

## 7. Preserved macOS-Only Examples

Two macOS examples remain (both flagged `"macOS_only": true`):

| Label | Command |
|-------|---------|
| macOS — Swift-run Librarian Server | `swift run LibrarianServer` |
| macOS — stdio bridge script (reuse mcp-bridge.sh) | `/bin/bash /path/to/TheLibrarian/scripts/mcp-bridge.sh` |

Both are clearly labeled as macOS-only and reference `swift`, `/bin/bash`, or `mcp-bridge.sh` as expected.

## 8. Platform Separation

- `_meta.platform_separation_note` documents the design principle.
- Each example has an explicit `platform` field (`"macOS"` or `"windows"`).
- Each example has `macOS_only` (bool|null) and `windows_native_command` (bool|null) flags.
- `platform_key` section explains both tags and placeholders.
- No Windows example references `swift`, `/bin/bash`, `mcp-bridge.sh`, or `check-mcp-health.sh` in operational code. Cross-references in documentation `note` fields are labeled as such.

## 9. Validation Results

### MCP Template Reconciliation Tests (new)

```
Test Results: 149 passed, 0 failed, 149 total
All tests passed!
```

Test categories:
1. Required MCP files exist (3 tests)
2. MCP template structure (33 tests)
3. No macOS-specific commands in Windows templates (33 tests)
4. Windows-native examples exist (21 tests)
5. macOS examples preserved (8 tests)
6. No machine-local paths in MCP scripts (15 tests)
7. No bash/curl/python3 dependencies in Windows MCP scripts (10 tests)
8. Platform separation documentation (8 tests)
9. MCP bridge script structure (10 tests)
10. Production file boundary (8 tests)

### Custody Normalization Tests (existing)

```
Test Results: 55 passed, 0 failed, 55 total
All tests passed!
```

## 10. Service State

```
Name                  Status StartType
----                  ------ ---------
LibrarianRunTimeNode Stopped    Manual
```

**Confirmed**: Service remains Stopped / Manual. No service was started or modified.

## 11. Router / Runtime / Model Untouched

- `router/` — No files modified.
- `rust-router/` — No files modified.
- `runtime/` — No files modified.
- `runtime/llama.cpp/` — No files modified.
- `models/` — No files modified.
- **Confirmed**: No production router code, runtime code, or model files were touched.

## 12. Runtime HTTP Untouched

- No HTTP endpoint definitions were changed.
- No port assignments were changed.
- No HTTP semantics were altered.
- **Confirmed**: Runtime HTTP untouched.

## 13. Model Execution Untouched

- No model was loaded, started, or queried.
- No model profile was changed.
- **Confirmed**: Model execution untouched.

## 14. Orphan Process Check

```
(No orphan processes found — the only transient PowerShell from testing has exited)
```

**Confirmed**: 0 orphan processes. No llama-server, rust-router, or other service processes remain.

## 15. Working Tree

```
On branch main
Your branch is ahead of 'origin/main' by 13 commits.
nothing to commit, working tree clean
```

**Confirmed**: Working tree clean after commit.

## 16. Classification

**PROMOTE**

All objectives met:
1. `mcp/templates/mcp-server.example.json` created with platform-separate examples.
2. Windows-native MCP bridge and health check scripts created (pure PowerShell, no bash/curl/python3).
3. macOS examples preserved and clearly labeled as macOS-only.
4. No machine-local paths committed — all values use documented placeholders.
5. 149 validation tests pass; 55 existing custody normalization tests pass.
6. Service remains Stopped/Manual. Router, runtime, HTTP, model execution untouched.
7. 0 orphan processes.
8. Working tree clean.

## 17. Next Sprint Candidate

**WIN-RUNTIME-OPERATOR-RUNBOOK-1** — Make the Windows node usable by an operator without unsafe automation: local config setup, verify ports, verify binary, inspect logs, start/stop checklist, failure triage, evidence capture.
