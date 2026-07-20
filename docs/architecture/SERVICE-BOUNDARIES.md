# Service Boundaries

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Service Map

```
┌─────────────────────────────────────────────────────────────┐
│                      Windows Machine                          │
│                                                               │
│  ┌─────────────────────┐    ┌─────────────────────┐         │
│  │   librarian-node     │    │   llama-server.exe  │         │
│  │   (axum HTTP server) │    │   (model inference)  │         │
│  │                      │    │                     │         │
│  │   Port: 9120-9124    │    │   Port: assigned     │         │
│  │                      │    │                     │         │
│  │   Health: /health    │    │   Health: /health    │         │
│  └──────────┬───────────┘    └──────────┬──────────┘         │
│             │                          │                      │
│             └──────────┬───────────────┘                     │
│                        │                                      │
│  ┌─────────────────────▼─────────────────────┐               │
│  │              Windows Service                │               │
│  │          (NSSM or native)                   │               │
│  │                                             │               │
│  │   Start → Run → Healthy → Degraded → Stop   │               │
│  └─────────────────────────────────────────────┘               │
│                                                               │
│  ┌─────────────────────┐    ┌─────────────────────┐         │
│  │   PowerShell Ops    │    │   Custody Ledger    │         │
│  │   (operational      │    │   (chain of custody) │         │
│  │    scripts)         │    │                      │         │
│  └─────────────────────┘    └──────────────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Service Ports

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| librarian-node | 9120 | HTTP | Main API |
| librarian-node | 9121 | HTTP | Health endpoint |
| librarian-node | 9122 | HTTP | Backend status |
| librarian-node | 9123 | HTTP | Model inference (OpenAI-compatible) |
| librarian-node | 9124 | HTTP | Evidence export |

---

## 3. Service Lifecycle

```
Installed
    │
    │ start
    ▼
Starting
    │
    │ health check
    ▼
Healthy
    │
    ├──→ Degraded (partial failure)
    │       │
    │       └──→ Failed → Restart
    │
    │ stop
    ▼
Stopped
    │
    │ uninstall
    ▼
Uninstalled
```

---

## 4. Boundary Rules

### Authority Boundary

| Direction | Allowed | Forbidden |
|-----------|---------|-----------|
| Core → Node | Work packets, queries | None |
| Node → Core | Evidence, receipts, status | Authority claims |
| Agent → Node | None (through Core only) | Direct commands |
| Node → Agent | None (through Core only) | Direct results |

### Network Boundary

| Access | Source | Allowed |
|--------|--------|---------|
| HTTP API | localhost only | Yes |
| Health endpoint | localhost only | Yes |
| Model inference | localhost only | Yes |
| MCP | Agent ↔ Core | Yes |

### Data Boundary

| Data | Owner | Can Read | Can Write |
|------|-------|----------|-----------|
| Canonical DB | Core | Core only | Core only |
| Operational DB | Node | Node | Node |
| Evidence | Node → Core | Both | Node only |
| Receipts | Both | Both | Respective owner |
| Configuration | Respective component | Respective component | Respective component |

---

## 5. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- CURRENT-ARCHITECTURE.md — Current architecture
- MCP-CONNECTION.md — MCP documentation
- DATA-FLOW.md — Data flow documentation
