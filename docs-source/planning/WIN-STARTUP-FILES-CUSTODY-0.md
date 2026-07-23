# WIN-STARTUP-FILES-CUSTODY-0

## Sprint Planning Document

### Goal

Inventory and classify every startup/runtime configuration surface used by `librarian-runtime-node`, then define the custody boundary needed before any startup files, model profiles, service launch scripts, or runtime operation scripts are modified.

### Sprint Type

Planning / inventory / custody-boundary sprint.

**This is NOT:**
- A startup-behavior change sprint
- A service launcher refactor sprint
- A model profile migration sprint

### Key Question

> What files and values control Windows runtime-node startup, where are they defined, where are they consumed, and which ones are machine-specific, mission-critical, mutable, or unsafe to leave scattered?

### Starting State

- HEAD: `c38fe8b`
- Working tree: clean
- Service: `LibrarianRunTimeNode` — Stopped / Manual
- Orphans: 0
- NSSM: not in PATH (non-admin shell)

### Scope

Inventory these categories:
1. Service launcher (`scripts/start-librarian-runtime-node.ps1`, NSSM config)
2. Runtime profiles and model config (`config/model-profiles.json`, `runtime/model_manager.ps1`)
3. Operations scripts (`scripts/operations/runtime-*.ps1`)
4. Qualification and service-swap scripts
5. Environment variables
6. Ports
7. Absolute paths

Classify every value. Create risk register. Define custody plan.

### Boundaries (Do Not)

- Do not change service startup behavior
- Do not change production router behavior
- Do not change runtime-node HTTP behavior
- Do not change model execution
- Do not edit `config/model-profiles.json` values except to inspect
- Do not template, relocate, or normalize paths yet
- Do not modify NSSM configuration
- Do not start backend processes
- Do not fix hardcoded paths

### Deliverables

| Path | Description |
|------|-------------|
| `docs/planning/WIN-STARTUP-FILES-CUSTODY-0.md` | This planning document |
| `reports/startup-files-custody-inventory.json` | Machine-readable inventory |
| `reports/WIN-STARTUP-FILES-CUSTODY-0.md` | Human-readable report |
| `docs/sprints/WIN-STARTUP-FILES-CUSTODY-0.md` | Sprint closeout doc |
| `fixtures/startup-files-custody/startup-custody-manifest.example.json` | Example custody manifest |
| `fixtures/startup-files-custody/machine-local-config.example.json` | Example local config |
| `scripts/tests/test-startup-files-custody-inventory.py` | Inventory validation test |

### Classification

**PROMOTE** — if inventory is complete enough to support WIN-STARTUP-FILES-CUSTODY-1.
