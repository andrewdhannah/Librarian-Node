# WIN-RUNTIME-QUALIFICATION-1: Windows Runtime Node Qualification Proof

## Sprint Summary
- **Layer**: 3 — Runtime Qualification
- **Repository**: librarian-runtime-node
- **Previous Layer Head**: `261c250`
- **Goal**: Produce a reproducible Windows Runtime qualification proof that verifies the running executable artifact, not just the source tree.

## Key Axiom
Source HEAD is not artifact proof. Qualification must verify the executable actually being run.

## Sprint Boundary
This is a runtime qualification sprint. It is not:
- a repo build sprint
- a router implementation sprint
- a model routing policy sprint
- a new feature sprint
- a Mac integration sprint

## Required Proof Dimensions

1. **Executable artifact identity** — binary path, SHA256, build timestamp, source HEAD, provenance match
2. **Router contract behavior** — run frozen contract harness against the running binary
3. **Service/process lifecycle** — start, exercise endpoints, stop, clean shutdown
4. **Network/auth boundary** — default bind safety, LAN exposure, auth-required mode, valid/invalid token
5. **Model/profile fit envelope** — installed profile inventory, qualified envelope from evidence
6. **Request/body limits** — max body bytes, oversized request refusal
7. **Cleanup/orphan proof** — no orphans before/after, ports free, service state preserved
8. **Machine-readable capability receipt** — qualification receipt with `qualified`/`partial`/`blocked`

## Pre-Work
1. Verify current repo HEAD fresh
2. Verify service state (Stopped / Manual)
3. Verify no orphans (llama-server, rust-router, python router)
4. Verify router/backend ports free
5. Preserve stash state

## Acceptance Gates
- Running router binary SHA256 recorded
- Binary build timestamp recorded
- Source HEAD recorded
- Contract harness run against the running binary
- Network/auth boundary verified
- Request size limit verified
- Service/process lifecycle verified
- Model/profile envelope recorded from evidence
- No orphan processes after qualification
- Ports free after qualification
- Service returns to Stopped / Manual
- Machine-readable qualification receipt emitted
- Receipt honestly reports `qualified`, `partial`, or `blocked`
- Working tree clean at closeout
- Stash state preserved

## Files
- `scripts/test-runtime-artifact-identity.ps1` — Dimension 1
- `scripts/test-runtime-contract.ps1` — Dimension 2 (wraps existing contract harness)
- `scripts/test-runtime-lifecycle.ps1` — Dimension 3
- `scripts/test-runtime-network-boundary.ps1` — Dimension 4
- `scripts/test-runtime-profiles.ps1` — Dimension 5
- `scripts/test-runtime-limits.ps1` — Dimension 6
- `scripts/test-runtime-cleanup.ps1` — Dimension 7
- `scripts/run-win-runtime-qualification.ps1` — Orchestrator

## Receipt Schema
See `runtime-node-qualification/v1` schema in the qualification receipt.
