# ADDON-CONVERSATION-INGESTION-1

**Status:** Planning — not yet authorized
**Prerequisites:** SDK epic ⏳

---

## Objective

Build the first SDK reference add-on: a conversation ingestion tool that imports Claude and ChatGPT export data into a governed, searchable, provenance-tracked format.

This add-on exercises every SDK boundary and serves as the reference implementation for future add-on development.

## Architecture

```
Claude Export JSON / ChatGPT Export
         ↓
Ingestion Add-on
         ├── Parse conversations
         ├── Detect projects (keyword matching)
         ├── Store in private conversation.db
         ├── Generate Obsidian Markdown
         ├── Record provenance
         ├── Emit evidence + receipt
         └── Register search capability

Capabilities:
  conversation.import       — ingest export files
  conversation.search      — full-text search across conversations
  conversation.export      — export to Obsidian vault (ZIP)
```

## What Already Exists

The HTML tool (`claude_archive_drop_site_v2.html`) already handles:

- JSON parsing (single convs, lists, wrapped exports, JSONL)
- Message text extraction (user + assistant)
- Project detection via keyword matching (15+ projects)
- Markdown generation with YAML frontmatter
- Obsidian vault ZIP packaging
- Status signal detection (blocked, next_action, shipped, in_progress)

The add-on wraps this in governance and adds:

- Private conversation.db storage
- Schema migrations
- Provenance records
- Evidence + receipt generation
- Search capability
- Health reporting

## Capabilities

| Capability | Description | Input | Output |
|-----------|-------------|-------|--------|
| `conversation.import` | Import Claude/ChatGPT export | File path | Record count, provenance |
| `conversation.search` | Full-text search | Query string | Matched conversation list |
| `conversation.export` | Export as Obsidian vault | Format option | ZIP download |

## Output

```
/vault
  Conversations/
    2026-07-23-capability-router.md
  Projects/
    TheLibrarian.md
    FlightPlan.md
    ...
  Entities/
    CapabilityRegistry.md
  MOCs/
    TheLibrarian-MOC.md
```

## Governance Integration

```
Import Request
    ↓
Entity check
    ↓
Permission check
    ↓
Custody claim
    ↓
Parse + Store (private DB)
    ↓
Generate Markdown
    ↓
Record provenance
    ↓
Emit evidence receipt
    ↓
Release custody
```

## Acceptance Gates

| Gate | Description |
|------|-------------|
| CI-1 | Conversation import parses Claude JSON export |
| CI-2 | Project detection maps conversations to known projects |
| CI-3 | Conversations stored in private add-on database |
| CI-4 | Markdown output with YAML frontmatter generated |
| CI-5 | Provenance recorded (source hash, timestamp, entity count) |
| CI-6 | Evidence + receipt emitted on import |
| CI-7 | Search capability queries across imported conversations |
| CI-8 | Obsidian vault export generates downloadable ZIP |
| CI-9 | Health reports index status and DB integrity |
| CI-10 | No new governance primitives introduced |
