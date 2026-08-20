-- =============================================================================
-- Capability Registry Schema — Phase 2 (Assurance-Bearing Registry)
-- =============================================================================
-- EPIC-CAPABILITY-REGISTRY-FOUNDATION-1
-- Subsystem: CAPREG
--
-- PURPOSE:
-- Extends Phase 1 from a capability inventory into an assurance-bearing
-- registry. Introduces qualification profiles, qualification records,
-- policy bindings, capability typing, and explicit assurance state columns.
--
-- Phase 1 established: identity, versions, dependencies, content hashes.
-- Phase 2 establishes: qualification, assurance state, policy bindings.
--
-- A capability MUST NOT transition to qualified availability unless a valid
-- qualification profile, qualification record, evidence reference, and
-- governing policy context are present and resolvable.
--
-- DESIGN PRINCIPLES:
--   - capability_versions is APPEND-ONLY — no UPDATE, no DELETE
--   - Every version has a SHA-256 content_hash, computed on insert
--   - The active_version field on capabilities is the only mutable pointer
--   - Status gates execution: unreviewed → reviewed → qualified → deprecated/revoked
--   - Qualification evidence is a REFERENCE, not inline (Evidence Plane owns proof)
--   - Timestamps are ISO 8601 TEXT throughout
--   - All execution policy is stored as JSON, not hardcoded columns
--   - Phase 2 invariant: see CR-I-007
--
-- PHASE 2 TABLES:
--   capability_types           — capability taxonomy (type registry)
--   qualification_profiles     — reusable qualification requirement definitions
--   capability_qualifications  — qualification result records
--   policies                   — executable governance policy definitions
--   policy_bindings            — policy-to-governed-object attachments
--
-- PHASE 2 INVARIANTS:
--   CR-I-007  A capability MUST NOT transition to qualified availability
--             unless a valid qualification profile, qualification record,
--             evidence reference, and governing policy context are present
--             and resolvable.
--   CR-I-008  A qualification profile version increment queues all
--             capabilities bound to that profile for requalification.
--   CR-I-009  A policy binding must reference a valid, active policy.
--             Inactive or revoked policies cannot be bound.
--   CR-I-010  qualification_evidence_id in capability_versions references
--             a resolvable evidence record in the Evidence Plane.
--             (extends CR-I-006 for qualification context.)
-- =============================================================================

-- =============================================================================
-- TABLE: capabilities
-- =============================================================================
-- One row per named capability. This is the registry — the authoritative
-- inventory of all known capabilities in the Librarian ecosystem.
-- =============================================================================

CREATE TABLE IF NOT EXISTS capabilities (
    -- Identity
    id                      TEXT PRIMARY KEY,        -- 'frontend-design'
    name                    TEXT NOT NULL,            -- human-readable display name
    type                    TEXT NOT NULL DEFAULT 'skill',
                                                        -- skill|workflow|policy|validator|template
    description             TEXT NOT NULL,            -- agent-facing description (triggers matching)
    summary                 TEXT,                     -- short label for search result display

    -- Source provenance
    source_type             TEXT,                     -- builtin|imported|anthropic|community|user
    source_reference        TEXT,                     -- URL or origin identifier
    source_author           TEXT,                     -- author or maintainer identity

    -- Lifecycle (Phase 1)
    status                  TEXT NOT NULL DEFAULT 'unreviewed',
                                                        -- unreviewed|reviewed|qualified|deprecated|revoked
    active_version          INTEGER,                  -- which version is currently active (NULL = none)

    -- Assurance State (Phase 2) — Three independent axes
    -- See CAPABILITY-ASSURANCE-CONTRACT-001 §1 for semantics
    availability            TEXT NOT NULL DEFAULT 'registered',
                                                        -- discovered|registered|disabled|removed
    qualification           TEXT NOT NULL DEFAULT 'not_tested',
                                                        -- not_tested|qualifying|passed|failed|stale|suspended
    authority               TEXT NOT NULL DEFAULT 'not_submitted',
                                                        -- not_submitted|pending_review|approved|rejected|revoked

    -- Governance (Phase 1)
                                                        -- green|yellow|red
    execution_policy        TEXT,                     -- JSON: {allowed_agents, network_access, ...}
    trigger_rules           TEXT,                     -- JSON: trigger phrases / match patterns
    constraint_rules        TEXT,                     -- JSON: WCAG, HIG, local-first, etc.
    required_context        TEXT,                     -- JSON: contexts this capability needs loaded

    -- Metadata
    tags                    TEXT,                     -- JSON array of tag strings for discovery
    category                TEXT,                     -- optional grouping category

    -- Timestamps
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for status-based queries (what's available?)
CREATE INDEX IF NOT EXISTS idx_capabilities_status
    ON capabilities(status);

-- Index for type-based queries (find all validators)
CREATE INDEX IF NOT EXISTS idx_capabilities_type
    ON capabilities(type);

-- Index for source-based queries (community vs builtin)
CREATE INDEX IF NOT EXISTS idx_capabilities_source_type
    ON capabilities(source_type);


-- =============================================================================
-- TABLE: capability_versions
-- =============================================================================
-- APPEND-ONLY version store. Every version of a capability's body is recorded
-- here with its content hash. Once written, rows are immutable.
--
-- The Evidence Plane references:
--   capability_id + version + content_hash
--
-- This creates a frozen, verifiable link between execution and instruction.
-- =============================================================================

CREATE TABLE IF NOT EXISTS capability_versions (
    -- Identity
    capability_id           TEXT NOT NULL REFERENCES capabilities(id),
    version                 INTEGER NOT NULL,

    -- Content
    body                    TEXT NOT NULL,            -- full instruction content
    content_hash            TEXT NOT NULL,            -- SHA-256 of body (hex)

    -- Version metadata
    changelog               TEXT,                     -- what changed from previous version
    author                  TEXT,                     -- who imported/reviewed this version
    review_notes            TEXT,                     -- qualification notes (free text)

    -- Evidence reference (NOT inline — Evidence Plane owns the proof)
    qualification_evidence_id TEXT,                   -- EV-XXXXX reference to Evidence Plane record

    -- Qualification Profile (Phase 2)
    -- Binds this version to a qualification profile that defines the
    -- checks, success criteria, and policy context for qualification.
    profile_id              TEXT REFERENCES qualification_profiles(profile_id),

    -- Timestamps
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),

    -- Constraints
    PRIMARY KEY (capability_id, version),

    -- CR-I-002: content_hash must be non-empty
    CHECK (content_hash != ''),
    -- CR-I-001 enforced at application layer: no UPDATE or DELETE on this table
    CHECK (version > 0)
);


-- =============================================================================
-- TABLE: capability_dependencies
-- =============================================================================
-- Declares that a capability depends on other capabilities for correct
-- execution. When a capability is loaded, its dependency chain is resolved
-- into the full capability set before execution.
--
-- CR-I-005: Cycles must be detected and rejected at resolution time.
-- =============================================================================

CREATE TABLE IF NOT EXISTS capability_dependencies (
    -- The capability that has a dependency
    capability_id           TEXT NOT NULL REFERENCES capabilities(id),

    -- The capability it depends on
    dependency_id           TEXT NOT NULL REFERENCES capabilities(id),

    -- Whether the dependency is required (TRUE) or optional (FALSE)
    required                INTEGER NOT NULL DEFAULT 1,

    -- Relationship type
    relationship_type       TEXT NOT NULL DEFAULT 'requires',
                                                        -- requires|extends|refines|conflicts

    -- Timestamps
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),

    PRIMARY KEY (capability_id, dependency_id),

    -- A capability cannot depend on itself
    CHECK (capability_id != dependency_id)
);

-- Index for reverse dependency lookups (what depends on this capability?)
CREATE INDEX IF NOT EXISTS idx_capability_dependencies_dependency
    ON capability_dependencies(dependency_id);


-- =============================================================================
-- PHASE 2: ASSURANCE-BEARING REGISTRY
-- =============================================================================


-- =============================================================================
-- TABLE: capability_types
-- =============================================================================
-- Capability taxonomy. A capability has exactly one type. A type has one
-- qualification profile and one default policy. Types are governed artifacts
-- — they require Owner awareness but schema-level definition is sufficient
-- for Phase 2 registration.
-- =============================================================================

CREATE TABLE IF NOT EXISTS capability_types (
    -- Identity
    capability_type_id      TEXT PRIMARY KEY,         -- 'design-review', 'code-generation', ...
    name                    TEXT NOT NULL,            -- human-readable type name
    description             TEXT NOT NULL,            -- what this type represents

    -- Classification
    category                TEXT NOT NULL DEFAULT 'standard',
                                                        -- standard|system|experimental|external

    -- Profile and policy defaults
    default_profile_id      TEXT REFERENCES qualification_profiles(profile_id),
    default_policy_id       TEXT REFERENCES policies(policy_id),

    -- Timestamps
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_capability_types_category
    ON capability_types(category);


-- =============================================================================
-- TABLE: qualification_profiles
-- =============================================================================
-- Defines reusable qualification requirements. A profile specifies the set of
-- checks, success criteria, and policy context that a capability must satisfy
-- to be considered qualified.
--
-- When a profile version increments, all capabilities qualified under the
-- previous version are queued for requalification (CR-I-008).
-- =============================================================================

CREATE TABLE IF NOT EXISTS qualification_profiles (
    -- Identity
    profile_id              TEXT PRIMARY KEY,         -- 'DESIGN-ADVISORY-001'
    name                    TEXT NOT NULL,            -- human-readable profile name
    description             TEXT NOT NULL,            -- what this profile evaluates
    version                 INTEGER NOT NULL DEFAULT 1,

    -- Profile definition
    checks                  TEXT NOT NULL DEFAULT '[]',
                                                        -- JSON array of check definitions:
                                                        -- [{id, description, required}]
    success_criteria        TEXT,                     -- JSON: {min_confidence, all_checks_required, ...}

    -- Lifecycle
    status                  TEXT NOT NULL DEFAULT 'active',
                                                        -- active|superseded|deprecated
    supersedes              TEXT REFERENCES qualification_profiles(profile_id),

    -- Provenance
    created_by              TEXT,                     -- identity of the profile author
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_qualification_profiles_status
    ON qualification_profiles(status);


-- =============================================================================
-- TABLE: capability_qualifications
-- =============================================================================
-- Records the qualification result for a capability against a profile.
-- Each record captures the outcome, evidence reference, and temporal
-- boundaries of a qualification evaluation.
-- =============================================================================

CREATE TABLE IF NOT EXISTS capability_qualifications (
    -- Identity
    qualification_id        TEXT PRIMARY KEY,         -- 'Q-20260727-001'
    capability_id           TEXT NOT NULL REFERENCES capabilities(id),
    profile_id              TEXT NOT NULL REFERENCES qualification_profiles(profile_id),
    version_id              INTEGER,                  -- capability_versions.version if version-specific

    -- Qualification state
    qualification_status    TEXT NOT NULL DEFAULT 'qualifying',
                                                        -- qualifying|passed|failed|stale|superseded
    confidence              REAL,                     -- 0.0 to 1.0 confidence score

    -- Evidence (NOT inline — Evidence Plane owns proof)
    evidence_reference      TEXT,                     -- EV-XXXXX reference to Evidence Plane record

    -- Temporal boundaries
    qualified_at            TEXT,                     -- ISO 8601
    expires_at              TEXT,                     -- ISO 8601 — NULL = does not expire
    assessed_at             TEXT NOT NULL DEFAULT (datetime('now')),

    -- Provenance
    assessor_identity       TEXT,                     -- who/what performed the qualification
    assessor_type           TEXT NOT NULL DEFAULT 'manual',
                                                        -- manual|automated|external

    -- Notes
    notes                   TEXT,                     -- qualification notes (free text)

    -- Timestamps
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (capability_id, version_id) REFERENCES capability_versions(capability_id, version_id)
);

CREATE INDEX IF NOT EXISTS idx_capability_qualifications_status
    ON capability_qualifications(qualification_status);

CREATE INDEX IF NOT EXISTS idx_capability_qualifications_capability
    ON capability_qualifications(capability_id);

CREATE INDEX IF NOT EXISTS idx_capability_qualifications_profile
    ON capability_qualifications(profile_id);


-- =============================================================================
-- TABLE: policies
-- =============================================================================
-- Executable governance policy definitions. Policies are versioned and have
-- a lifecycle independent of the capabilities they govern. A policy defines
-- permitted actions, restrictions, and evidence requirements.
-- =============================================================================

CREATE TABLE IF NOT EXISTS policies (
    -- Identity
    policy_id               TEXT PRIMARY KEY,         -- 'ADVISORY-ONLY-001', 'EXECUTION-AUTHORIZED-001'
    policy_type             TEXT NOT NULL,             -- advisory|execution|evidence|operations
    name                    TEXT NOT NULL,            -- human-readable policy name
    description             TEXT NOT NULL,            -- what this policy governs
    version                 INTEGER NOT NULL DEFAULT 1,

    -- Policy definition
    policy_definition       TEXT NOT NULL DEFAULT '{}',
                                                        -- JSON: {permissions, restrictions, evidence}

    -- Lifecycle
    status                  TEXT NOT NULL DEFAULT 'active',
                                                        -- active|inactive|superseded|revoked
    supersedes              TEXT REFERENCES policies(policy_id),

    -- Provenance & timestamps
    created_by              TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_policies_type
    ON policies(policy_type);

CREATE INDEX IF NOT EXISTS idx_policies_status
    ON policies(status);


-- =============================================================================
-- TABLE: policy_bindings
-- =============================================================================
-- Attaches policies to governed objects (capabilities or capability types).
-- A binding records which policy applies, to what scope, and whether it is
-- currently active.
-- =============================================================================

CREATE TABLE IF NOT EXISTS policy_bindings (
    -- Identity
    binding_id              TEXT PRIMARY KEY,         -- 'PB-20260727-001'

    -- Target policy
    policy_id               TEXT NOT NULL REFERENCES policies(policy_id),

    -- Bound object (polymorphic: binds to either a specific capability
    -- or a capability type as default for all capabilities of that type)
    capability_id           TEXT REFERENCES capabilities(id),
    capability_type_id      TEXT REFERENCES capability_types(capability_type_id),

    -- Binding scope
    enforcement_scope       TEXT NOT NULL DEFAULT 'full',
                                                        -- full|read_only|advisory
    active_state            TEXT NOT NULL DEFAULT 'active',
                                                        -- active|suspended|expired|removed

    -- Temporal scope
    bound_at                TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at              TEXT,                     -- ISO 8601 — NULL = does not expire
    unbound_at              TEXT,                     -- ISO 8601 — when binding was removed

    -- Provenance
    bound_by                TEXT,                     -- who created the binding
    reason                  TEXT,                     -- why this binding exists

    -- Timestamps
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now')),

    -- Exactly one of capability_id or capability_type_id must be non-NULL
    CHECK (
        (capability_id IS NOT NULL AND capability_type_id IS NULL)
        OR (capability_id IS NULL AND capability_type_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_policy_bindings_policy
    ON policy_bindings(policy_id);

CREATE INDEX IF NOT EXISTS idx_policy_bindings_capability
    ON policy_bindings(capability_id);

CREATE INDEX IF NOT EXISTS idx_policy_bindings_type
    ON policy_bindings(capability_type_id);

CREATE INDEX IF NOT EXISTS idx_policy_bindings_active
    ON policy_bindings(active_state);


-- =============================================================================
-- Invariant Enforcement Views (Phase 1 + Phase 2)
-- =============================================================================
-- These views expose potential invariant violations for monitoring.

-- CR-I-003: Capabilities with active_version set to NULL or a non-existent version
CREATE VIEW IF NOT EXISTS view_capabilities_broken_active_version AS
    SELECT c.id, c.name, c.active_version
    FROM capabilities c
    WHERE c.active_version IS NOT NULL
      AND NOT EXISTS (
          SELECT 1 FROM capability_versions cv
          WHERE cv.capability_id = c.id
            AND cv.version = c.active_version
      );

-- CR-I-004: Status transition violations (expected to be empty at steady state)
-- Detects status values that don't follow the allowed lifecycle.
-- This is enforced at the application layer; this view surfaces anomalies.
CREATE VIEW IF NOT EXISTS view_capabilities_invalid_status AS
    SELECT c.id, c.name, c.status
    FROM capabilities c
    WHERE c.status NOT IN ('unreviewed', 'reviewed', 'qualified', 'deprecated', 'revoked');


-- =============================================================================
-- Phase 2 Invariant Enforcement Views
-- =============================================================================

-- CR-I-007: Capabilities with qualified availability but missing assurance chain
-- A capability must not reach qualified availability without a valid profile,
-- qualification record, evidence, and policy context.
CREATE VIEW IF NOT EXISTS view_capabilities_qualified_without_assurance AS
    SELECT c.id, c.name, c.availability, c.qualification, c.authority
    FROM capabilities c
    WHERE c.availability = 'registered'
      AND c.qualification = 'passed'
      AND (
          -- No active qualification record
          NOT EXISTS (
              SELECT 1 FROM capability_qualifications cq
              WHERE cq.capability_id = c.id
                AND cq.qualification_status = 'passed'
          )
          -- OR no profile assigned
          OR NOT EXISTS (
              SELECT 1 FROM capability_versions cv
              WHERE cv.capability_id = c.id
                AND cv.version = c.active_version
                AND cv.profile_id IS NOT NULL
          )
          -- OR no evidence reference
          OR NOT EXISTS (
              SELECT 1 FROM capability_versions cv
              WHERE cv.capability_id = c.id
                AND cv.version = c.active_version
                AND cv.qualification_evidence_id IS NOT NULL
          )
          -- OR no policy binding
          OR NOT EXISTS (
              SELECT 1 FROM policy_bindings pb
              WHERE pb.capability_id = c.id
                AND pb.active_state = 'active'
          )
      );

-- CR-I-008: Capabilities with stale qualification after profile version change
-- When a qualification profile version increments, capabilities qualified
-- under the previous version are flagged.
CREATE VIEW IF NOT EXISTS view_capabilities_stale_qualification AS
    SELECT c.id AS capability_id,
           c.name AS capability_name,
           cq.profile_id,
           qp.version AS current_profile_version,
           cq.qualification_id,
           cq.qualified_at
    FROM capabilities c
    JOIN capability_qualifications cq ON cq.capability_id = c.id
    JOIN qualification_profiles qp ON qp.profile_id = cq.profile_id
    WHERE cq.qualification_status = 'passed'
      AND EXISTS (
          -- A newer profile version exists than the one used for qualification
          SELECT 1 FROM qualification_profiles qp2
          WHERE qp2.profile_id = cq.profile_id
            AND qp2.version > qp.version
            AND qp2.status = 'active'
      );

-- CR-I-009: Policy bindings referencing inactive/revoked policies
CREATE VIEW IF NOT EXISTS view_policy_bindings_inactive AS
    SELECT pb.binding_id, pb.policy_id, p.status AS policy_status,
           pb.capability_id, pb.active_state
    FROM policy_bindings pb
    JOIN policies p ON p.policy_id = pb.policy_id
    WHERE pb.active_state = 'active'
      AND p.status NOT IN ('active', 'supersedes');


-- =============================================================================
-- Schema Version Tracking
-- =============================================================================

CREATE TABLE IF NOT EXISTS capability_registry_meta (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);

INSERT OR IGNORE INTO capability_registry_meta (key, value)
VALUES ('schema_version', '2');

-- Phase 2 migration tracking
INSERT OR IGNORE INTO capability_registry_meta (key, value)
VALUES ('phase2_migrated_at', datetime('now'));
INSERT OR IGNORE INTO capability_registry_meta (key, value)
VALUES ('phase2_migration', 'CAPABILITY-REGISTRY-PHASE2-MIGRATION-1');

INSERT OR IGNORE INTO capability_registry_meta (key, value)
VALUES ('subsystem', 'CAPREG');

INSERT OR IGNORE INTO capability_registry_meta (key, value)
VALUES ('created_at', datetime('now'));
