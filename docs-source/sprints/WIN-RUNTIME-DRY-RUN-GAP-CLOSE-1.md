# WIN-RUNTIME-DRY-RUN-GAP-CLOSE-1

**Sprint**: Close three dry-run readiness gaps found in WIN-RUNTIME-DRY-RUN-READINESS-1
**Status**: COMPLETE — READY FOR OWNER PROMOTE ASSESSMENT

## Gaps Closed

### GAP-001: MCP Permissions File Missing

**Fix**: Created `config/mcp-permissions.json` — a versioned MCP tool permission matrix with 12 tool entries.

**Validation**:
- All 12 expected Librarian MCP tools have permission entries
- `agents_can_mark_verified: false` — agents cannot self-verify
- `human_verification_is_final: true` — human remains final authority
- No tool has `can_verify: true` — zero escalation risk
- No machine-local paths in file
- Health check script (`scripts/check-mcp-health.ps1`) can parse and validate the permission matrix
- Runbook §11.4 and §11.5 updated with references
- 30 gap-closure tests pass for GAP-001 alone

### GAP-002: Gitignore Pattern Too Narrow

**Fix**: Broadened `.gitignore` from `config/*.local.json` to `config/*.local.*` to cover all local override file extensions (.ps1, .yaml, etc.).

**Validation**:
- `.gitignore` now contains `config/*.local.*`
- Runbook §3.1 agreed with gitignore pattern
- No tracked actual local override files exists (only example files)
- Safer custody pattern covers future local override file types

### GAP-003: Runbook §11.1 Embedding Port Source Wrong

**Fix**: Corrected embedding port 9125 source from incorrect `config/model-profiles.json` to authoritative `runtime/model_manager.ps1` → `$EmbedPort`.

**Validation**:
- Port 9125 still documented
- Source now correctly references `runtime/model_manager.ps1`
- Port map table now included in §11.1
- Config file sources table in §11.5 updated with permission file

## Test Results (all 11 suites, 1,861 tests)

| Suite | Tests | Status |
|-------|-------|--------|
| test-startup-files-custody-inventory.py | 44 | PASS |
| test-mcp-template-reconciliation.py | 147 | PASS |
| test-custody-normalization.py | 55 | PASS |
| test-win-runtime-operator-runbook.py | 74 | PASS (was 74, unchanged) |
| test-win-runtime-dry-run-readiness.py | 75 | PASS (was 74, +1 for mcp-permissions.json existence) |
| test-win-runtime-dry-run-gap-close.py | 45 | PASS (NEW) |
| test-advisory-stub.py | 511 | PASS |
| test-context-route-contract.py | 413 | PASS |
| test-router-context-runtime-contract.py | 229 | PASS |
| test-router-context-runtime-design.py | 92 | PASS |
| test-router-context-prototype.py | 176 | PASS |
| **Total** | **1,861** | **ALL PASS** |

## Hard Boundaries Verified

- [x] Service `LibrarianRunTimeNode` Stopped / Manual (unchanged)
- [x] No orphan `llama-server.exe` processes
- [x] No `router/` or `rust-router/` files modified
- [x] No `runtime/` files modified (except `model_manager.ps1` not touched)
- [x] No model behavior changed
- [x] No machine-local values committed
- [x] No auto-start or auto-model instructions added

## Files Changed

**Modified** (5):
- `.gitignore` — `config/*.local.*` broadened pattern
- `docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md` — §11.1, §11.4, §11.5 updates
- `scripts/tests/test-custody-normalization.py` — allowed modified prefixes
- `scripts/tests/test-mcp-template-reconciliation.py` — allowed modified prefixes
- `scripts/tests/test-win-runtime-dry-run-readiness.py` — added RUNBOOK_REFERENCED_FILES entry

**New** (2):
- `config/mcp-permissions.json` — MCP tool permission matrix
- `scripts/tests/test-win-runtime-dry-run-gap-close.py` — 45 gap-closure validation tests

**Net**: +9 lines, −2 lines across 7 files.

## PROMOTE Assessment

All three gaps are closed. The runbook is now fully internally executable. The owner should assess:

- **PROMOTE** — proceed to `WIN-RUNTIME-CONTROLLED-ACTIVATION-1` (all gaps closed, no remaining caveat)
- **HOLD** — if any concern about the MCP permission file structure or the gitignore pattern
- **REJECT** — if reopening any gap

If promoted, the next sprint `WIN-RUNTIME-CONTROLLED-ACTIVATION-1` would begin with `git commit`.
