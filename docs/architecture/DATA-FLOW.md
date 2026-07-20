# Data Flow

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. High-Level Data Flow

```
Agent/User
    │
    │ MCP (proposal/evidence/receipt)
    ▼
┌──────────┐
│   Core   │◄──────────────────────────────────┐
└────┬─────┘                                   │
     │                                         │
     │ QualificationRequest (Mac→Windows)      │
     ▼                                         │
┌──────────┐    ┌──────────────────────┐      │
│  Node    │───►│ llama-server.exe      │      │
│          │    │ (model inference)     │      │
└────┬─────┘    └──────────────────────┘      │
     │                                         │
     │ EvidencePacket (Windows→Mac)            │
     └─────────────────────────────────────────┘
```

---

## 2. Request Flow

### 2.1 Model Inference

```
POST /v1/chat/completions
    │
    │ Check health
    ▼
Is model loaded?
    ├── Yes → Forward to llama-server
    └── No  → Reject with 503
             │
             Evidence recorded
```

### 2.2 Model Load

```
POST /backend/select
    │
    │ Verify model exists
    ▼
Is GPU available?
    ├── Yes → Spawn llama-server
    │         │
    │         Wait for health check
    │         │
    │         Evidence recorded
    │
    └── No  → Reject with error
             │
             Evidence recorded
```

### 2.3 Model Unload

```
POST /backend/stop
    │
    │ Send SIGTERM to llama-server
    ▼
    Wait for process exit
    │
    Verify VRAM release
    │
    Evidence recorded
```

---

## 3. Evidence Flow

### 3.1 Lifecycle Evidence

```
Node operation occurs
    │
    EvidenceWriter records event
    │
    ├──→ lifecycle_evidence table (append-only)
    │
    └──→ Evidence export (optional)
```

### 3.2 Evidence Export Flow

```
Local evidence
    │
    Evidence packet (advisory, LOCAL-ONLY)
    │
    Intake boundary record (certifies no authority claim)
    │
    Artifact manifest (hash-indexed)
    │
    Export bundle (portable directory + checksums)
    │
    Transfer receipt (attempt record)
    │
    [Physical handoff to Core]
    │
    Core inspects, ingests, accepts
```

---

## 4. Data Stores

### 4.1 Operational Database (Node)

**Location:** `data/node.db`  
**Engine:** SQLite (WAL mode)

**Tables:**
| Table | Purpose |
|-------|---------|
| local_models | Model inventory |
| runtime_profiles | Runtime profile records |
| hardware_profiles | Hardware measurement records |
| job_leases | Active and historical leases |
| runtime_runs | Execution run records |
| lifecycle_evidence | Append-only lifecycle events |

### 4.2 Canonical Database (Core)

**Location:** Core-side  
**Engine:** SQLite (WAL mode)

**Tables:**
| Table | Purpose |
|-------|---------|
| model_identity | Model identity records |
| task_pack | Task pack records |
| validator_pack | Validator pack records |
| sprint | Sprint governance records |

---

## 5. Data Flow Rules

1. **Evidence flows one direction:** Node → Core
2. **Work packets flow one direction:** Core → Node
3. **Evidence is append-only:** Never modified or deleted
4. **Receipts are generated per mutation:** Every state change produces a receipt
5. **Node does not create canonical truth:** All Node data is advisory

---

## 6. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- ADR-PLATFORM-002 — Platform Lifecycle
- CURRENT-ARCHITECTURE.md — Current architecture
- SERVICE-BOUNDARIES.md — Service boundaries
- DEPENDENCY-MAP.md — Dependency map
