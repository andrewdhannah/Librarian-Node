# MAC/WIN-ROUTER-CONTEXT-RUNTIME-CONTRACT-1

## Router Context Runtime Contract Sprint

**Goal:** Harden runtime-adjacent context decision interface sketches into a versioned, tested contract for future advisory-only decision layer.

**Constraints:** Contract sprint only — no advisory stub, no production router changes.

---

## Sprint Log

| Date | Event |
|------|-------|
| 2026-06-28 | Starting checks: HEAD bf337a8, clean working tree, service Stopped/Manual, orphans 0 |
| 2026-06-28 | Contract document created: `docs/contracts/router-context-runtime-contract.md` (v0.1) |
| 2026-06-28 | 4 valid fixtures + 3 invalid fixtures created |
| 2026-06-28 | Contract test suite written: 229 tests |
| 2026-06-28 | All 229 tests pass |

---

## Key Decisions

1. **Contract before stub:** MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1 → CONTRACT-1 → ADVISORY-STUB-1 pipeline. Sketches hardened before any stub code.
2. **Three top-level objects:** Context Decision Input, Context Decision Output, Receipt Consumption — each with distinct field sets and invariants.
3. **Degraded-node contract:** `if node_health in ["unreachable","stopped","timeout"]: allowed_remote_use=false, measured_penalty_ms>=3000`.
4. **Forbidden live-routing:** 10 actions explicitly prohibited (start_process, stop_process, select_model, execute_model, modify_router_config, modify_model_profiles, open_network_listener, change_bind_host, write_runtime_state, apply_context_route).
5. **Receipt consumption:** Downstream-only — no routing, no runtime lifecycle, no model selection, no router state mutation.
6. **Invalid fixtures:** forbidden-live-routing, advisory-false, weak-provenance-receipt — all detected by tests.

---

## Classification

**PROMOTE** — Contract boundary clean. Ready for advisory stub.
