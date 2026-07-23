# WO-006: UBUNTU-TIER-1-PORT-1

**Status:** Authorized
**Repository:** Librarian-Node
**Dependencies:** WO-001 through WO-005 (complete)

---

## Objective

Validate that a Linux runtime can consume the existing governance substrate through the `RuntimeAdapter` boundary — without requiring the substrate to know Linux exists.

## Allowed Scope

- Linux `RuntimeAdapter` implementation
- systemd event translation → `ProcessEvent`
- Linux process discovery
- XDG filesystem integration (`~/.local/share/Librarian/`)
- Ubuntu build workflow
- Debian package structure
- Linux platform evidence artifacts

## Non-Scope

```
New governance primitives:        0
New contract modules:             0
New receipt types:                0
New evidence categories:          0
New lifecycle states:             0
New residency states:             0
Changes to MQR:                   0
Changes to WO-005 behavior:       0
Linux-specific governance types:  0
```

## Acceptance Gates

| Gate | Requirement |
|------|-------------|
| LC-1 | Linux adapter attaches without contract changes |
| LC-2 | Process events map to existing ResidencyState |
| LC-3 | Evidence uses existing evidence categories |
| LC-4 | Receipts use existing receipt envelope |
| LC-5 | Custody semantics remain unchanged |
| LC-6 | Receipt comparison matches MQR + WO-005 shape |
| LC-7 | No Linux-specific governance concepts introduced |
