# MCP Connection

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. MCP Architecture

MCP (Model Context Protocol) is used as transport for Agent ↔ Core interaction. It is NOT used as the Core/Node boundary.

```
Agent/Human
     |
     v
    MCP                          ← transport layer (Agent ↔ Core)
     |
     v
Librarian Core                   ← authority layer
     |
     | QualificationRequest / EvidencePacket / ResidencyStatus
     v
Librarian Node                   ← execution layer (HTTP/REST bridge)
```

---

## 2. MCP Roles

| Role | Description | Protocol |
|------|-------------|----------|
| Agent ↔ Core | Propose changes, submit evidence, return receipts | MCP (stdio/HTTP) |
| Core ↔ Node | Dispatch work packets, return evidence, query status | HTTP/REST (packet contracts) |

---

## 3. MCP Tools

MCP tools follow a proposal-and-apply model:

| Tool | Purpose | Pattern |
|------|---------|---------|
| `project_proposal_submit` | Propose a change | Proposal → Review → Apply |
| `project_evidence_submit` | Return evidence | Evidence → Validate → Store |
| `project_receipt_submit` | Return action receipts | Receipt → Verify → Archive |

**Generic file-write MCP tools must not be exposed on canonical paths.**

---

## 4. Current MCP Status

| Component | MCP Status | Notes |
|-----------|------------|-------|
| Node | No MCP server | Node communicates via HTTP/REST |
| Core | MCP contract defined | Not yet implemented |
| Bridge | Draft only | `mcp-bridge.ps1` — stdio→HTTP bridge |
| Templates | Draft | `mcp/templates/mcp-server.example.json` |

---

## 5. MCP Constraints

1. **MCP is not an authority boundary** — MCP exposes capabilities, not authority
2. **MCP is not Core/Node transport** — Core/Node boundary is packet contracts
3. **MCP tools must not bypass authority** — All mutations require Core validation
4. **MCP discovery does not equal trust** — Discovering a tool does not authorize its use

---

## 6. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- CURRENT-ARCHITECTURE.md — Current architecture
- SERVICE-BOUNDARIES.md — Service boundaries
- DATA-FLOW.md — Data flow documentation
