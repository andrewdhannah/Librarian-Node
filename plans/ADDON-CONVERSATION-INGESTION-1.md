# ADDON-CONVERSATION-INGESTION-1

**Status:** Planning — not yet authorized
**Prerequisites:** SDK epic ⏳

---

## Objective

Build the first provenance-heavy SDK provider: a conversation ingestion tool that imports Claude and ChatGPT export data into a governed, searchable, provenance-tracked local knowledge store.

This provider serves dual purposes:
1. **SDK validation** — exercises artifact ownership, provenance chains, entity extraction, and derived artifact tracking
2. **Personal knowledge portability** — demonstrates that user data can be owned locally and shared with AI providers through controlled context packages rather than bulk exports

## Strategic Context

Without a local knowledge layer, switching AI providers requires either:
- Surrendering complete conversation history to the new provider ("import all my data")
- Starting from zero context

With this provider, the model becomes:

```
Claude/ChatGPT Export
        ↓
Local governed knowledge store (user-owned)
        ↓
Context package (selective projection, not bulk export)
        ↓
Any AI provider (temporary processor, not owner)
```

The provider separates **knowledge ownership** (local store, user-controlled)
from **reasoning capability** (AI provider, interchangeable).

Information layers:

| Layer | Content | Sharing Model |
|-------|---------|---------------|
| Identity/Profile | Preferences, working style, stable facts | Controlled projection |
| Project Knowledge | Decisions, architecture, requirements | Per-task permission |
| Historical Context | Conversations, experiments, drafts | Local only — not shared |
| Raw Source | Original exports, attachments | Local only — not shared |

When switching providers, the system generates a context artifact containing
only what the new provider needs — not the entire history.

Same governance pattern: Entity → Decision → Permission → Capability → Context

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

## Provider Sandbox Model

Each provider owns an isolated directory that it cannot escape without authorization.

```
data/providers/claude-ingestion/
├── data.db                    ← structured data (conversations, entities)
├── conversations/             ← ingested source artifacts
├── artifacts/                 ← derived artifacts (summaries, extractions)
├── indexes/                   ← search indexes
└── migrations/                ← schema versions
```

The provider receives a `StorageClient` handle, not a raw filesystem path.
The runtime enforces the sandbox boundary — no provider can access another
provider's directory or the substrate's `governance.db` without explicit
permission routing through the capability layer.

### Storage Ownership Model

| Layer | Owner | Purpose |
|-------|-------|---------|
| Raw sources | External | Original Claude/ChatGPT exports, files |
| Provider sandbox | Capability provider | Parsing, normalization, extraction, provenance |
| Governance DB | Librarian substrate | Entities, decisions, permissions, receipts, evidence |
| Obsidian vault | Human projection | Review, editing, navigation, graph exploration |
| RAG index | Retrieval projection | Semantic search and context selection |
| Context packages | Controlled output | Minimum information for AI providers |

### Projections Are Disposable

The governed knowledge store is authoritative. Derived views can be regenerated:

```
Obsidian vault    → rebuild from provider data
RAG index         → re-embed from provider data
Context packages  → regenerate on request
```

But provenance and governance records remain stable.

### Provider Removal

```
1. Disable provider capability
2. Record removal decision (governance receipt)
3. Delete provider sandbox
4. Preserve governance history
```

After removal, the substrate can answer "this provider existed and imported
these artifacts" but cannot recreate deleted content. Audit history and
content retention are separate concerns.

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
Parse + Store (provider sandbox)
    ├── conversation.db
    ├── source artifacts (hashed, timestamped)
    └── extracted entities
    ↓
Generate Obsidian projection
    ↓
Record provenance
    ↓
Update RAG index (optional)
    ↓
Emit evidence + receipt
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
