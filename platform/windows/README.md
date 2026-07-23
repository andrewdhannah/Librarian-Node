# Windows Platform Adapter

**Status:** Active  

Windows-specific implementation artifacts for the Librarian Node. These are platform adapters — not part of the portable contract layer.

## Contents

| Path | Purpose |
|------|---------|
| `scripts/` *(in repo root)* | PowerShell scripts for runtime management, testing, qualification |
| `config/` | Windows model profiles, runtime config |
| `router/router.py` | Python reference implementation (behavioral reference) |
| NSSM configuration | Windows service wrapper |

## Architecture Note

Windows integration follows the OS adapter pattern defined in CROSSPLATFORM-PORTABILITY-MODEL.md:

| Concern | Windows Adapter |
|---------|----------------|
| Service manager | NSSM / Windows Service API |
| File paths | `%APPDATA%` |
| Credentials | Win Credential Manager |
| Elevation | UAC |
| Process management | `CreateProcess` / `CREATE_NO_WINDOW` |
| CLI tooling | PowerShell |

These adapters are platform-specific. No Windows assumptions leak into `librarian-contracts`, `librarian-core`, or `librarian-node`.
