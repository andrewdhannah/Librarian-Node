# SESSION-IDENTITY-CONTRACT.md — Node Identity Contract

**Version:** 1.0.0
**Status:** Canonical
**Last Updated:** 2026-07-24

---

## Purpose

This contract defines the format and requirements for node identity files used by Librarian Node implementations. It is platform-neutral and applies to Windows, Linux, and macOS nodes.

---

## Identity Format

### JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Node Identity",
  "description": "Canonical node identity format for Librarian Nodes",
  "type": "object",
  "required": [
    "node_id",
    "node_type",
    "authority",
    "platform",
    "governance_commit",
    "state",
    "capabilities",
    "created_at"
  ],
  "properties": {
    "node_id": {
      "type": "string",
      "description": "Unique node identifier",
      "minLength": 1
    },
    "node_type": {
      "type": "string",
      "description": "Node type",
      "enum": ["librarian-runtime-node"]
    },
    "authority": {
      "type": "string",
      "description": "Node authority model",
      "enum": ["owner-controlled"]
    },
    "platform": {
      "type": "string",
      "description": "Node platform",
      "enum": ["windows", "linux", "macos"]
    },
    "governance_commit": {
      "type": "string",
      "description": "Canonical governance commit SHA",
      "pattern": "^[a-f0-9]{40}$"
    },
    "state": {
      "type": "string",
      "description": "Current node state",
      "enum": ["UNREGISTERED", "GOVERNED_NODE", "GOVERNED_EXECUTION"]
    },
    "capabilities": {
      "type": "array",
      "description": "List of available capabilities",
      "items": {
        "type": "string"
      }
    },
    "created_at": {
      "type": "string",
      "description": "ISO-8601 timestamp of identity creation",
      "format": "date-time"
    }
  }
}
```

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `node_id` | string | Yes | Unique node identifier (format: `<PLATFORM>-<HOSTNAME>-<ID>`) |
| `node_type` | enum | Yes | Must be `librarian-runtime-node` |
| `authority` | enum | Yes | Must be `owner-controlled` |
| `platform` | enum | Yes | Node platform: `windows`, `linux`, or `macos` |
| `governance_commit` | string | Yes | Canonical governance commit SHA-256 |
| `state` | enum | Yes | Current node state |
| `capabilities` | array | Yes | List of available capabilities |
| `created_at` | string | Yes | ISO-8601 timestamp of identity creation |

---

## Identity Examples

### Windows Node Identity

```json
{
  "node_id": "WINPC-BIG-PICKLE",
  "node_type": "librarian-runtime-node",
  "authority": "owner-controlled",
  "platform": "windows",
  "governance_commit": "6be76216a8048492526c4ca0ae751b6d2d507185",
  "state": "GOVERNED_EXECUTION",
  "capabilities": [
    "governance.read",
    "governance.verify",
    "schema.validate",
    "evidence.generate",
    "custody.track"
  ],
  "created_at": "2026-07-24T14:00:00.0000000-04:00"
}
```

### Linux Node Identity

```json
{
  "node_id": "LINUX-NODE-INITIAL",
  "node_type": "librarian-runtime-node",
  "authority": "owner-controlled",
  "platform": "linux",
  "governance_commit": "6be76216a8048492526c4ca0ae751b6d2d507185",
  "state": "GOVERNED_EXECUTION",
  "capabilities": [
    "governance.read",
    "governance.verify",
    "schema.validate",
    "evidence.generate",
    "custody.track"
  ],
  "created_at": "2026-07-24T14:00:00.0000000-00:00"
}
```

### macOS Node Identity

```json
{
  "node_id": "MACOS-NODE-INITIAL",
  "node_type": "librarian-runtime-node",
  "authority": "owner-controlled",
  "platform": "macos",
  "governance_commit": "6be76216a8048492526c4ca0ae751b6d2d507185",
  "state": "GOVERNED_EXECUTION",
  "capabilities": [
    "governance.read",
    "governance.verify",
    "schema.validate",
    "evidence.generate",
    "custody.track"
  ],
  "created_at": "2026-07-24T14:00:00.0000000-00:00"
}
```

---

## Identity File Locations

| Platform | Location |
|----------|----------|
| Windows | `%APPDATA%\Librarian\node-identity.json` or `C:\Librarian\node-identity.json` |
| Linux | `/etc/librarian/node-identity.json` or `~/librarian/node-identity.json` |
| macOS | `~/Library/Librarian/node-identity.json` |

---

## Validation Rules

### Node ID Format

Node IDs must follow the pattern: `<PLATFORM>-<HOSTNAME>-<ID>`

Examples:
- `WINPC-BIG-PICKLE`
- `LINUX-NODE-INITIAL`
- `MACOS-NODE-INITIAL`

### Node Type

Must be exactly: `librarian-runtime-node`

### Authority

Must be exactly: `owner-controlled`

### Platform

Must be one of: `windows`, `linux`, `macos`

### Governance Commit

Must be a valid SHA-256 hash (40 hexadecimal characters).

### State Transitions

```
UNREGISTERED → GOVERNED_NODE → GOVERNED_EXECUTION
```

### Capabilities

Capabilities must follow the naming convention: `<namespace>.<action>`

Examples:
- `governance.read`
- `governance.verify`
- `schema.validate`
- `evidence.generate`
- `custody.track`

---

## Equivalence Rules

All platform implementations must produce identity files with identical deterministic fields:

**Deterministic Fields (Must Match):**
- `node_type`
- `authority`
- `governance_commit`
- `capabilities`

**Variable Fields (Expected to Differ):**
- `node_id` (different per node)
- `platform` (different per platform)
- `state` (may differ based on startup sequence)
- `created_at` (different per node creation)

---

## References

- Startup Protocol: `STARTUP-PROTOCOL.md`
- Startup Output Contract: `STARTUP-OUTPUT-CONTRACT.md`
- Node Reference Architecture: `../docs/NODE-REFERENCE-ARCHITECTURE.md`
