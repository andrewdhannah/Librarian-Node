# Evidence Manifest

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  
**Purpose:** Record the chain of custody for all evidence produced during the Windows Runtime Node lifecycle.

---

## Evidence Policy

**Evidence is append-only.** State may change; evidence does not. This is a platform-wide invariant (ADR-PLATFORM-002).

---

## Manifest Format

Each evidence collection run records:

```
Timestamp:   YYYY-MM-DD HH:MM:SS UTC
Machine:     DESKTOP-ISNJ51B (Big Pickle)
OS Build:    Windows 10 Pro Workstations 10.0.19041
Git Commit:  abc123def...
Tool Versions:
  rustc: 1.xx.x
  cargo: 1.xx.x
  llama-server: x.x.x
Evidence Produced:
  - evidence/phase0/runtime-inventory.md
  - evidence/phase0/process-list.md
  - ...
Checksums:
  runtime-inventory.md: SHA256:...
  process-list.md: SHA256:...
```

---

## Manifest Entries

### Phase 0

| Run | Date | Machine | Git Commit | Collector |
|-----|------|---------|------------|-----------|
| — | Pending | — | — | — |

### Sprint 1

| Run | Date | Machine | Git Commit | Collector |
|-----|------|---------|------------|-----------|
| — | Pending | — | — | — |

### Sprint 2

| Run | Date | Machine | Git Commit | Collector |
|-----|------|---------|------------|-----------|
| — | Pending | — | — | — |

### Sprint 3

| Run | Date | Machine | Git Commit | Collector |
|-----|------|---------|------------|-----------|
| — | Pending | — | — | — |

### Sprint 4

| Run | Date | Machine | Git Commit | Collector |
|-----|------|---------|------------|-----------|
| — | Pending | — | — | — |

### Sprint 5

| Run | Date | Machine | Git Commit | Collector |
|-----|------|---------|------------|-----------|
| — | Pending | — | — | — |

### Sprint 6

| Run | Date | Machine | Git Commit | Collector |
|-----|------|---------|------------|-----------|
| — | Pending | — | — | — |

### Sprint 7

| Run | Date | Machine | Git Commit | Collector |
|-----|------|---------|------------|-----------|
| — | Pending | — | — | — |

---

## Integrity

Evidence integrity is verified by:
1. Git commit history (immutable record of changes)
2. Optional SHA-256 checksums per artifact
3. Append-only constraint (no modification of existing evidence)
4. Recovery evidence recorded alongside normal evidence
