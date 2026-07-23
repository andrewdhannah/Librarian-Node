# Linux Platform Adapter

**Status:** Tier 1 (Ubuntu LTS)
**Repository:** Librarian-Node

---

## Purpose

Linux-specific implementation artifacts for the Librarian Node. These are platform adapters — not part of the portable governance layer.

## Architecture

```
Linux Runtime
     |
     | systemctl / /proc / journald
     v
LinuxAdapter (RuntimeAdapter impl)
     |
     | ProcessEvent
     v
RuntimeSupervisor (governance core)
     |
     | ResidencyState + Evidence + Receipt
     v
GovernanceDb
```

## Adapter Mapping

| Linux Concern | Adapter |
|---------------|---------|
| Service manager | systemd (`systemctl start/stop/status`) |
| Process discovery | `/proc` filesystem |
| File paths | XDG Base Directory (`~/.local/share/Librarian/`) |
| Logging | journald |
| Packaging | `.deb` (Ubuntu LTS) |
| Service user | `librarian` (dedicated system user) |

## Non-Goals

- No Linux-specific governance concepts
- No Linux-specific receipt types
- No Linux-specific evidence categories
- No Linux-specific lifecycle states
- No Linux-specific residency states

## Build Target

```
x86_64-unknown-linux-gnu
Ubuntu 24.04 LTS (Noble Numbat)
```

## Evidence

Linux platform evidence is collected under `evidence/phase0/linux/` when run on an Ubuntu target.
