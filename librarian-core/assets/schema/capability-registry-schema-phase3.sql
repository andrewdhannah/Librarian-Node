-- =============================================================================
-- Capability Registry Schema — Phase 3 (Qualification Execution Governance)
-- =============================================================================
-- EPIC-CAPABILITY-REGISTRY-FOUNDATION-1
-- Sprint: CAPABILITY-REGISTRY-PHASE3-QUALIFICATION-EXECUTION-1
-- Subsystem: CAPREG
--
-- PURPOSE:
-- Extends Phase 2 (assurance-bearing registry) with qualification lifecycle
-- state tracking, S0-S5 security classification enforcement, identity
-- separation, authority transition records, evidence dimension binding,
-- and operational mode derivation support.
--
-- Phase 2 established: qualification profiles, qualification records,
--   policies, policy bindings, capability types.
-- Phase 3 establishes: qualification lifecycle state machine, S0-S5
--   security classification, derived operational mode, explainability.
--
-- POLICY INVARIANTS (frozen):
--   PI-001  S0-S5 meanings are frozen. No semantic changes without
--           policy amendment and re-authorization.
--   PI-002  qualification_state and security_classification are
--           independent axes — QUALIFIED + S2 and QUALIFIED + S5 are
--           both valid states with different evidence burdens.
--   PI-003  operational_mode is DERIVED, never stored directly.
--   PI-004  Self-attestation of security classification or operational
--           mode is prohibited.
--   PI-005  S-level elevation (e.g., S2 → S4) requires a new
--           qualification event, not an attribute update.
-- =============================================================================

-- =============================================================================
-- PHASE 3: QUALIFICATION LIFECYCLE
-- =============================================================================


-- =============================================================================
-- TABLE: qualification_lifecycle_events
-- =============================================================================
-- Append-only event log for all qualification state transitions.
-- Every transition recorded here is immutable. This is the audit trail
-- for qualification decisions.
-- =============================================================================

CREATE TABLE IF NOT EXISTS qualification_lifecycle_events (
    -- Identity
    event_id                TEXT PRIMARY KEY,          -- 'QLE-20260727-001'
    qualification_id        TEXT NOT NULL REFERENCES capability_qualifications(qualification_id),
    capability_id           TEXT NOT NULL REFERENCES capabilities(id),

    -- Transition
    from_state              TEXT NOT NULL,              -- previous qualification_state
    to_state                TEXT NOT NULL,              -- new qualification_state
    transition_type         TEXT NOT NULL,              -- automatic|manual

    -- Security classification at time of transition
    security_classification TEXT,                       -- S0–S5 (may be NULL for pre-classification states)

    -- Authority
    transitioned_by         TEXT NOT NULL,              -- identity who performed transition
    transitioner_role       TEXT NOT NULL,              -- system|evaluator|approver|owner
    authority_evidence_id   TEXT,                       -- EV-XXXXX if manual transition

    -- Evidence snapshot
    evidence_snapshot       TEXT NOT NULL DEFAULT '{}', -- JSON: evidence state at transition time

    -- Timestamps
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),

    -- Phase 2 -> Phase 3 boundary: S-level stability
    -- The security_classification stored here is the FROZEN S-level at time
    -- of transition. Audit replay reads this — not current registry state.
    CHECK (security_classification IS NULL OR
           security_classification IN ('S0','S1','S2','S3','S4','S5')),
    CHECK (transition_type IN ('automatic', 'manual')),
    CHECK (transitioner_role IN ('system', 'evaluator', 'approver', 'owner'))
);

CREATE INDEX IF NOT EXISTS idx_qlifecycle_qualification
    ON qualification_lifecycle_events(qualification_id);

CREATE INDEX IF NOT EXISTS idx_qlifecycle_capability
    ON qualification_lifecycle_events(capability_id);

CREATE INDEX IF NOT EXISTS idx_qlifecycle_transition
    ON qualification_lifecycle_events(transition_type);

CREATE INDEX IF NOT EXISTS idx_qlifecycle_created
    ON qualification_lifecycle_events(created_at);


-- =============================================================================
-- TABLE: qualification_evidence_records
-- =============================================================================
-- Per-dimension evidence records for qualification. Each qualification
-- record may have 0..N evidence records across the five dimensions:
-- identity, capability, security level, qualification, constraints.
-- =============================================================================

CREATE TABLE IF NOT EXISTS qualification_evidence_records (
    -- Identity
    evidence_id             TEXT PRIMARY KEY,          -- 'QER-20260727-001'
    qualification_id        TEXT NOT NULL REFERENCES capability_qualifications(qualification_id),

    -- Dimension (one of the five evidence dimensions)
    dimension               TEXT NOT NULL
        CHECK (dimension IN ('identity', 'capability', 'security_level', 'qualification', 'constraints')),

    -- Evidence content
    evidence_type           TEXT NOT NULL,              -- test_result|review_approval|benchmark|audit_log|receipt
    evidence_reference      TEXT,                       -- EV-XXXXX or external reference
    evidence_body           TEXT,                       -- JSON: evidence payload

    -- Freshness
    evidence_hash           TEXT NOT NULL,              -- SHA-256 of evidence_body
    captured_at             TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at              TEXT,                       -- ISO 8601 — NULL = does not expire

    -- Provenance
    producer_identity       TEXT NOT NULL,              -- who produced this evidence
    producer_role           TEXT NOT NULL DEFAULT 'evaluator',
                                                         -- evaluator|system|automated_harness|external

    -- Timestamps
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),

    CHECK (evidence_hash != '')
);

CREATE INDEX IF NOT EXISTS idx_qevidence_qualification
    ON qualification_evidence_records(qualification_id);

CREATE INDEX IF NOT EXISTS idx_qevidence_dimension
    ON qualification_evidence_records(dimension);

CREATE INDEX IF NOT EXISTS idx_qevidence_freshness
    ON qualification_evidence_records(expires_at);


-- =============================================================================
-- VIEW: qualification_resolvability
-- =============================================================================
-- For each qualification, shows whether the evidence chain is complete
-- across all five dimensions. Used by derive-operational-mode.py.
-- =============================================================================

CREATE VIEW IF NOT EXISTS view_qualification_resolvability AS
    SELECT
        cq.qualification_id,
        cq.capability_id,
        cq.qualification_status,
        cq.profile_id,
        -- Count distinct evidence dimensions present
        (SELECT COUNT(DISTINCT qer.dimension)
         FROM qualification_evidence_records qer
         WHERE qer.qualification_id = cq.qualification_id
        ) AS evidence_dimensions_present,
        -- Evidence freshness: are all present dimensions non-expired?
        CASE
            WHEN (SELECT COUNT(*) FROM qualification_evidence_records qer
                  WHERE qer.qualification_id = cq.qualification_id
                    AND qer.expires_at IS NOT NULL
                    AND qer.expires_at < datetime('now')) > 0
            THEN 'stale'
            WHEN (SELECT COUNT(*) FROM qualification_evidence_records qer
                  WHERE qer.qualification_id = cq.qualification_id) = 0
            THEN 'no_evidence'
            ELSE 'fresh'
        END AS evidence_freshness,
        -- Security classification at time of qualification
        cq.qualification_status || '+' || COALESCE(cq.notes, 'S0') AS classification_context
    FROM capability_qualifications cq;


-- =============================================================================
-- VIEW: operational_mode_derivation
-- =============================================================================
-- Derives operational mode from stored facts. This is the policy evaluation
-- function: f(security_classification, qualification_state, evidence, constraints).
--
-- The derivation is deterministic — same inputs always produce same mode.
-- =============================================================================

CREATE VIEW IF NOT EXISTS view_operational_mode_derivation AS
    SELECT
        c.id AS capability_id,
        c.name AS capability_name,
        c.availability,
        c.status AS lifecycle_status,
        c.qualification,
        c.authority,
        c.security_classification AS raw_security_classification,
        COALESCE(
            (SELECT c2.security_classification FROM capabilities c2 WHERE c2.id = c.id
             AND c2.security_classification IN ('S0','S1','S2','S3','S4','S5')),
            'S0'
        ) AS security_classification,
        qr.evidence_freshness,
        qr.evidence_dimensions_present,
        -- Derived operational mode
        CASE
            WHEN c.qualification = 'revoked' THEN 'explain_only'
            WHEN c.status = 'revoked' THEN 'explain_only'
            WHEN qr.evidence_freshness = 'stale' THEN 'review_assist'
            WHEN c.qualification = 'passed' THEN
                CASE
                    WHEN COALESCE(c.security_classification, 'S0') IN ('S4', 'S5')
                         AND qr.evidence_freshness != 'fresh'
                    THEN 'review_assist'
                    ELSE 'autonomous_assist'
                END
            WHEN c.qualification IN ('qualifying', 'not_tested') THEN 'recommend_only'
            WHEN c.qualification = 'failed' THEN 'recommend_only'
            WHEN c.qualification = 'stale' THEN 'review_assist'
            WHEN c.qualification = 'suspended' THEN 'explain_only'
            ELSE 'recommend_only'
        END AS derived_operational_mode,
        -- Explainability: why this mode?
        CASE
            WHEN c.qualification = 'revoked' OR c.status = 'revoked'
            THEN 'Capability qualification revoked.'
            WHEN qr.evidence_freshness = 'stale'
            THEN 'Evidence has expired or is stale. Operational mode degraded to review_assist.'
            WHEN c.qualification = 'passed' AND COALESCE(c.security_classification, 'S0') IN ('S4', 'S5') AND qr.evidence_freshness != 'fresh'
            THEN 'High security level (S4/S5) requires fresh evidence for autonomous operation.'
            WHEN c.qualification = 'passed'
            THEN 'Fully qualified. Autonomous operation permitted within security boundary.'
            WHEN c.qualification IN ('qualifying', 'not_tested')
            THEN 'Qualification not yet complete. Provide recommendations only.'
            WHEN c.qualification = 'failed'
            THEN 'Qualification failed. Capability cannot be used autonomously.'
            WHEN c.qualification = 'stale'
            THEN 'Qualification evidence is stale. Requalification required.'
            WHEN c.qualification = 'suspended'
            THEN 'Capability suspended due to boundary violation.'
            ELSE 'Qualification state indeterminate. Defaulting to recommend_only.'
        END AS mode_explanation,
        -- Constraints
        c.execution_policy AS constraints
    FROM capabilities c
    LEFT JOIN view_qualification_resolvability qr ON qr.capability_id = c.id;


-- =============================================================================
-- Schema Version
-- =============================================================================

INSERT OR IGNORE INTO capability_registry_meta (key, value)
VALUES ('schema_version', '3');

INSERT OR IGNORE INTO capability_registry_meta (key, value)
VALUES ('phase3_migrated_at', datetime('now'));
INSERT OR IGNORE INTO capability_registry_meta (key, value)
VALUES ('phase3_migration', 'CAPABILITY-REGISTRY-PHASE3-QUALIFICATION-EXECUTION-1');
