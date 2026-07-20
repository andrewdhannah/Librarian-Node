# Dependency Map

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Build Dependencies

### Rust (librarian-node)

| Dependency | Version | Purpose |
|------------|---------|---------|
| axum | 0.8 | HTTP server framework |
| tokio | 1.x (full) | Async runtime |
| tower | 0.5 | Middleware |
| tower-http | 0.6 | CORS middleware |
| reqwest | 0.12 (rustls) | HTTP client |
| serde / serde_json | 1.x | Serialization |
| tracing / tracing-subscriber | 0.1 | Structured logging |
| clap | 4.x | CLI argument parsing |
| chrono | 0.4 | Timestamps |
| uuid | 1.x | Unique identifiers |
| rusqlite | 0.31 (bundled) | SQLite database |

### Rust (librarian-contracts)

| Dependency | Version | Purpose |
|------------|---------|---------|
| serde / serde_json | 1.x | Serialization |
| sha2 | 0.10 | Packet hashing |
| chrono | 0.4 | Timestamps |
| uuid | 1.x | Unique identifiers |

### Rust (librarian-core)

| Dependency | Version | Purpose |
|------------|---------|---------|
| rusqlite | 0.31 (bundled) | SQLite database |
| reqwest | 0.12 (rustls) | Bridge client |
| serde / serde_json | 1.x | Serialization |
| chrono | 0.4 | Timestamps |
| uuid | 1.x | Unique identifiers |

---

## 2. Runtime Dependencies

| Dependency | Version | Purpose | Location |
|------------|---------|---------|----------|
| llama-server.exe | Current build | Model inference | `runtime/llama.cpp/` |
| Vulkan loader | System | GPU compute | System library |
| SQLite | Bundled with rusqlite | Database | Bundled |

---

## 3. Operational Dependencies

| Dependency | Version | Purpose | Location |
|------------|---------|---------|----------|
| PowerShell | 5.1+ | Operational scripts | System |
| NSSM | Latest | Windows Service management | `runtime/bin/nssm.exe` |

---

## 4. External Dependencies

| Dependency | Version | Purpose | Notes |
|------------|---------|---------|-------|
| NVIDIA/CUDA | N/A | GPU compute | Not used (Vulkan) |
| AMD ROCm | N/A | GPU compute | Not used (Vulkan) |
| Ollama | Optional | Alternative runtime | `qwen3-vl`, `nomic-embed-text`, `all-minilm` |

---

## 5. Hardware Dependencies

| Component | Requirement | Current | Notes |
|-----------|-------------|---------|-------|
| GPU | Vulkan 1.2+ | RX 570 (Vulkan) | 4GB VRAM |
| RAM | 16GB+ | 16GB | |
| Storage | 10GB+ | 500GB SSD | |
| CPU | x64 | i5-3570K | |

---

## 6. Model Dependencies

| Model | Size | Status | Location |
|-------|------|--------|----------|
| MiniCPM5 1B Q4 | ~600MB | Downloaded | `G:\Models\minicpm5\` |
| MiniCPM5 1B Q8 | ~1GB | Downloaded | `G:\Models\minicpm5\` |
| VibeThinker 3B Q4 | ~2GB | Planned | Owner approved |

---

## 7. Network Dependencies

| Endpoint | Port | Purpose | Required |
|----------|------|---------|----------|
| localhost | 9120-9124 | Node API | Yes (operational) |
| Core (future) | TBD | Packet dispatch | Yes (relationship) |
| MCP (future) | TBD | Agent interaction | Yes (relationship) |

---

## 8. Dependency Graph

```
librarian-node
    ├── axum / tokio
    ├── rusqlite (bundled SQLite)
    ├── serde / serde_json
    ├── chrono / uuid
    ├── tracing / tracing-subscriber
    ├── reqwest
    └── clap
         │
         ├── llama-server.exe
         │       └── Vulkan / GPU drivers
         │
         └── SQLite (local database)

librarian-core (separate crate)
    ├── rusqlite (bundled SQLite)
    ├── reqwest
    ├── serde / serde_json
    └── chrono / uuid

librarian-contracts (shared)
    ├── serde / serde_json
    ├── sha2
    ├── chrono
    └── uuid
```

---

## 9. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- ARCHITECTURAL-BOUNDARY-MAP — Code organization
- CURRENT-ARCHITECTURE.md — Current architecture
- DATA-FLOW.md — Data flow documentation
