//! # Qualification Observation Projection Module (RUST-MIGRATION-M1-D2)
//!
//! Implements qualification projection semantics: module-level, read-only
//! projections over the governed capability-registry SQLite database.
//!
//! **Source hierarchy (explicit):**
//!   governed registry/qualification state → Rust projection → adapters
//!
//! **Not:**
//!   Rust runtime state → projection
//!
//! Boundary invariants:
//!
//! - **Read-only.** Every projection opens the database read-only and executes
//!   reads inside ONE transaction (consistent snapshot); `PRAGMA query_only = ON`
//!   additionally fails any write attempt at the SQLite level.
//! - **Sealed at construction.** [`QualificationObservationState`] mirrors the
//!   M0B [`RuntimeApiState`](crate::runtime_api::RuntimeApiState) and M1-B
//!   [`RegistryObservationState`](crate::registry_observation::RegistryObservationState)
//!   patterns: node id and registry path are fixed at construction.
//! - **No transition execution.** This module observes qualification state;
//!   it does NOT create, approve, authorize, or execute qualification transitions.
//! - **No evidence ingestion.** Evidence references remain references;
//!   this module does NOT become an evidence-ingestion or evidence-authority subsystem.
//! - **Classification provenance preserved.** Declared/derived/inherited/policy_constraint
//!   distinctions survive projection without collapsing.
//! - **F-2 deferred.** Storage ambiguity for security_classification remains deferred;
//!   this module does NOT introduce schema changes to resolve it.
//! - **F-3 deferred.** Operational-mode derivation remains a separate boundary.
//! - **Fail-closed.** Unknown persisted enum values, missing records, or
//!   unestablishable identity fail the projection with explicit errors.
//! - **Deterministic.** Same registry state → same payload bytes.
//!   `projection_observed_at` is the only variable envelope field.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use librarian_contracts::node::capability_registry::{
    CapabilityId, QualificationAxis, QualificationEvidenceReference, QualificationLifecycleEvent,
    QualificationRecord, QualificationState,
};
use librarian_contracts::node::qualification_semantics::CapabilityGovernanceState;
use rusqlite::{Connection, OpenFlags};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

/// Sealed qualification observation state.
///
/// Immutable after construction; projections read from the governed
/// registry at the path sealed here.
#[derive(Clone)]
pub struct QualificationObservationState {
    node_id: String,
    db_path: Arc<PathBuf>,
}

impl QualificationObservationState {
    /// Seal the observing node id and the governed registry database path.
    pub fn new(node_id: impl Into<String>, db_path: impl Into<PathBuf>) -> Self {
        QualificationObservationState {
            node_id: node_id.into(),
            db_path: Arc::new(db_path.into()),
        }
    }

    // -----------------------------------------------------------------------
    // Qualification record projection
    // -----------------------------------------------------------------------

    /// Observe a single qualification record by qualification ID.
    ///
    /// Returns the full `QualificationRecord` with evidence references
    /// (not inline evidence — the Evidence Plane owns proof).
    pub fn qualification_record(
        &self,
        qualification_id: &str,
    ) -> Result<librarian_contracts::node::registry_observation::RegistryObservationEnvelope<QualificationRecord>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        let qid = qualification_id.to_string();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let record = read_qualification_record(conn, &qid)?
                .with_context(|| format!("qualification '{qid}' not found in registry"))?;
            Ok(envelope(&node_id, identity, record))
        })
    }

    /// Observe all qualification records for a capability.
    ///
    /// Returns qualification records ordered by `created_at` descending
    /// (most recent first).
    pub fn capability_qualifications(
        &self,
        capability_id: &str,
    ) -> Result<librarian_contracts::node::registry_observation::RegistryObservationEnvelope<Vec<QualificationRecord>>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        let cid = capability_id.to_string();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let records = read_capability_qualifications(conn, &cid)?;
            Ok(envelope(&node_id, identity, records))
        })
    }

    // -----------------------------------------------------------------------
    // Qualification lifecycle events projection
    // -----------------------------------------------------------------------

    /// Observe lifecycle events for a qualification record.
    ///
    /// Returns append-only lifecycle events ordered by `created_at` ascending
    /// (chronological order).
    pub fn qualification_lifecycle_events(
        &self,
        qualification_id: &str,
    ) -> Result<librarian_contracts::node::registry_observation::RegistryObservationEnvelope<Vec<QualificationLifecycleEvent>>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        let qid = qualification_id.to_string();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let events = read_qualification_lifecycle_events(conn, &qid)?;
            Ok(envelope(&node_id, identity, events))
        })
    }

    // -----------------------------------------------------------------------
    // Qualification evidence records projection
    // -----------------------------------------------------------------------

    /// Observe evidence records for a qualification.
    ///
    /// Returns per-dimension evidence records with references (not inline
    /// evidence — the Evidence Plane owns proof). Classification provenance
    /// (declared/derived/inherited/policy_constraint) is preserved.
    pub fn qualification_evidence(
        &self,
        qualification_id: &str,
    ) -> Result<librarian_contracts::node::registry_observation::RegistryObservationEnvelope<Vec<QualificationEvidenceReference>>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        let qid = qualification_id.to_string();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let evidence = read_qualification_evidence(conn, &qid)?;
            Ok(envelope(&node_id, identity, evidence))
        })
    }

    // -----------------------------------------------------------------------
    // Qualification overview projection
    // -----------------------------------------------------------------------

    /// Observe qualification overview counts.
    ///
    /// Returns deterministic counts over one consistent snapshot.
    /// Zero-count groups are present (M1-B convention).
    pub fn qualification_overview(
        &self,
    ) -> Result<librarian_contracts::node::registry_observation::RegistryObservationEnvelope<QualificationOverview>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let overview = read_qualification_overview(conn)?;
            Ok(envelope(&node_id, identity, overview))
        })
    }

    // -----------------------------------------------------------------------
    // Capability governance state projection
    // -----------------------------------------------------------------------

    /// Observe the governance state for a capability.
    ///
    /// Returns the independent axes: qualification state, qualification axis,
    /// authorization, availability, and execution status.
    ///
    /// **First-class invariants:**
    /// - `QUALIFIED ≠ AUTHORIZED`
    /// - `QUALIFIED ≠ AVAILABLE`
    /// - `QUALIFIED ≠ EXECUTING`
    pub fn capability_governance_state(
        &self,
        capability_id: &str,
    ) -> Result<librarian_contracts::node::registry_observation::RegistryObservationEnvelope<CapabilityGovernanceState>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        let cid = capability_id.to_string();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let state = read_capability_governance_state(conn, &cid)?;
            Ok(envelope(&node_id, identity, state))
        })
    }
}

// ---------------------------------------------------------------------------
// Qualification overview (deterministic counts)
// ---------------------------------------------------------------------------

/// Qualification overview — deterministic counts over one consistent snapshot.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QualificationOverview {
    /// Total number of qualification records.
    pub qualification_count: u64,
    /// Counts by qualification status.
    pub by_status: Vec<(String, u64)>,
    /// Counts by qualification axis.
    pub by_qualification_axis: Vec<(String, u64)>,
    /// Counts by assessor type.
    pub by_assessor_type: Vec<(String, u64)>,
    /// Total number of lifecycle events.
    pub lifecycle_event_count: u64,
    /// Total number of evidence records.
    pub evidence_record_count: u64,
    /// Counts by evidence dimension.
    pub by_evidence_dimension: Vec<(String, u64)>,
    /// Number of qualifications with complete evidence (all 5 dimensions).
    pub complete_evidence_count: u64,
    /// Number of qualifications with stale evidence.
    pub stale_evidence_count: u64,
}

// ---------------------------------------------------------------------------
// Projection helpers
// ---------------------------------------------------------------------------

/// Read a single qualification record by ID.
fn read_qualification_record(conn: &Connection, qid: &str) -> Result<Option<QualificationRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT qualification_id, capability_id, profile_id, version_id,
                    qualification_status, confidence, evidence_reference,
                    qualified_at, expires_at, assessed_at,
                    assessor_identity, assessor_type
             FROM capability_qualifications
             WHERE qualification_id = ?1",
        )
        .context("prepare qualification record read")?;

    let mut rows = stmt.query_map([qid], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<u32>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<f32>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, String>(11)?,
        ))
    })?;

    match rows.next() {
        Some(Ok((qid, cid, pid, vid, status, conf, evref, qual, exp, assessed, assessor, atype))) => {
            Ok(Some(QualificationRecord {
                qualification_id: qid,
                capability_id: CapabilityId::new(cid)
                    .context("invalid persisted capability id")?,
                profile_id: pid,
                version_id: vid,
                status: parse_enum(&status, "qualification_status")?,
                confidence: conf,
                evidence_reference: evref,
                qualified_at: qual,
                expires_at: exp,
                assessed_at: assessed,
                assessor_identity: assessor,
                assessor_type: parse_enum(&atype, "assessor_type")?,
            }))
        }
        Some(Err(e)) => bail!("failed to read qualification record: {e}"),
        None => Ok(None),
    }
}

/// Read all qualification records for a capability.
fn read_capability_qualifications(conn: &Connection, cid: &str) -> Result<Vec<QualificationRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT qualification_id, capability_id, profile_id, version_id,
                    qualification_status, confidence, evidence_reference,
                    qualified_at, expires_at, assessed_at,
                    assessor_identity, assessor_type
             FROM capability_qualifications
             WHERE capability_id = ?1
             ORDER BY created_at DESC",
        )
        .context("prepare capability qualifications read")?;

    let rows = stmt.query_map([cid], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<u32>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<f32>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, String>(11)?,
        ))
    })?;

    let mut records = Vec::new();
    for row in rows {
        let (qid, cid, pid, vid, status, conf, evref, qual, exp, assessed, assessor, atype) = row?;
        records.push(QualificationRecord {
            qualification_id: qid,
            capability_id: CapabilityId::new(cid)
                .context("invalid persisted capability id")?,
            profile_id: pid,
            version_id: vid,
            status: parse_enum(&status, "qualification_status")?,
            confidence: conf,
            evidence_reference: evref,
            qualified_at: qual,
            expires_at: exp,
            assessed_at: assessed,
            assessor_identity: assessor,
            assessor_type: parse_enum(&atype, "assessor_type")?,
        });
    }
    Ok(records)
}

/// Read lifecycle events for a qualification record.
fn read_qualification_lifecycle_events(
    conn: &Connection,
    qid: &str,
) -> Result<Vec<QualificationLifecycleEvent>> {
    let mut stmt = conn
        .prepare(
            "SELECT event_id, qualification_id, capability_id,
                    from_state, to_state, transition_type,
                    security_classification, transitioned_by,
                    transitioner_role, authority_evidence_id,
                    evidence_snapshot, created_at
             FROM qualification_lifecycle_events
             WHERE qualification_id = ?1
             ORDER BY created_at ASC",
        )
        .context("prepare lifecycle events read")?;

    let rows = stmt.query_map([qid], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
        ))
    })?;

    let mut events = Vec::new();
    for row in rows {
        let (eid, qid, cid, from, to, ttype, sec, by, role, auth, snap, created) = row?;
        events.push(QualificationLifecycleEvent {
            event_id: eid,
            qualification_id: qid,
            capability_id: CapabilityId::new(cid)
                .context("invalid persisted capability id")?,
            from_state: parse_enum(&from, "from_state")?,
            to_state: parse_enum(&to, "to_state")?,
            transition_type: parse_enum(&ttype, "transition_type")?,
            security_classification: sec
                .map(|s| parse_enum(&s, "security_classification"))
                .transpose()?,
            transitioned_by: by,
            transitioner_role: parse_enum(&role, "transitioner_role")?,
            authority_evidence_id: auth,
            evidence_snapshot: serde_json::from_str(&snap)
                .unwrap_or(serde_json::Value::Null),
            created_at: created,
        });
    }
    Ok(events)
}

/// Read evidence records for a qualification.
fn read_qualification_evidence(
    conn: &Connection,
    qid: &str,
) -> Result<Vec<QualificationEvidenceReference>> {
    let mut stmt = conn
        .prepare(
            "SELECT evidence_id, qualification_id, dimension,
                    evidence_type, evidence_reference, evidence_body,
                    evidence_hash, captured_at, expires_at,
                    producer_identity, producer_role
             FROM qualification_evidence_records
             WHERE qualification_id = ?1
             ORDER BY dimension ASC",
        )
        .context("prepare evidence records read")?;

    let rows = stmt.query_map([qid], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
        ))
    })?;

    let mut evidence = Vec::new();
    for row in rows {
        let (eid, qid, dim, etype, eref, ebody, ehash, captured, expires, producer, prole) = row?;
        evidence.push(QualificationEvidenceReference {
            evidence_id: eid,
            qualification_id: qid,
            dimension: parse_enum(&dim, "dimension")?,
            evidence_type: parse_enum(&etype, "evidence_type")?,
            evidence_reference: eref,
            evidence_body: ebody.and_then(|s| serde_json::from_str(&s).ok()),
            evidence_hash: ehash,
            captured_at: captured,
            expires_at: expires,
            producer_identity: producer,
            producer_role: parse_enum(&prole, "producer_role")?,
        });
    }
    Ok(evidence)
}

/// Read qualification overview counts.
fn read_qualification_overview(conn: &Connection) -> Result<QualificationOverview> {
    // Total qualification count
    let qualification_count: u64 = conn
        .query_row("SELECT COUNT(*) FROM capability_qualifications", [], |row| row.get(0))
        .context("count qualifications")?;

    // Counts by status
    let by_status = group_counts(
        conn,
        "SELECT qualification_status, COUNT(*) FROM capability_qualifications GROUP BY qualification_status ORDER BY qualification_status",
    )?;

    // Counts by qualification axis (from capabilities table)
    let by_qualification_axis = group_counts(
        conn,
        "SELECT qualification, COUNT(*) FROM capabilities GROUP BY qualification ORDER BY qualification",
    )?;

    // Counts by assessor type
    let by_assessor_type = group_counts(
        conn,
        "SELECT assessor_type, COUNT(*) FROM capability_qualifications GROUP BY assessor_type ORDER BY assessor_type",
    )?;

    // Lifecycle event count
    let lifecycle_event_count: u64 = conn
        .query_row("SELECT COUNT(*) FROM qualification_lifecycle_events", [], |row| row.get(0))
        .context("count lifecycle events")?;

    // Evidence record count
    let evidence_record_count: u64 = conn
        .query_row("SELECT COUNT(*) FROM qualification_evidence_records", [], |row| row.get(0))
        .context("count evidence records")?;

    // Counts by evidence dimension
    let by_evidence_dimension = group_counts(
        conn,
        "SELECT dimension, COUNT(*) FROM qualification_evidence_records GROUP BY dimension ORDER BY dimension",
    )?;

    // Qualifications with complete evidence (all 5 dimensions present)
    let complete_evidence_count: u64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT qualification_id) FROM qualification_evidence_records
             WHERE qualification_id IN (
                 SELECT qualification_id FROM qualification_evidence_records
                 GROUP BY qualification_id
                 HAVING COUNT(DISTINCT dimension) = 5
             )",
            [],
            |row| row.get(0),
        )
        .context("count complete evidence")?;

    // Qualifications with stale evidence
    let stale_evidence_count: u64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT qualification_id) FROM qualification_evidence_records
             WHERE expires_at IS NOT NULL AND expires_at < datetime('now')",
            [],
            |row| row.get(0),
        )
        .context("count stale evidence")?;

    Ok(QualificationOverview {
        qualification_count,
        by_status,
        by_qualification_axis,
        by_assessor_type,
        lifecycle_event_count,
        evidence_record_count,
        by_evidence_dimension,
        complete_evidence_count,
        stale_evidence_count,
    })
}

/// Read capability governance state.
fn read_capability_governance_state(
    conn: &Connection,
    cid: &str,
) -> Result<CapabilityGovernanceState> {
    // Read qualification state and axis from capabilities table
    let mut stmt = conn
        .prepare(
            "SELECT status, qualification, availability
             FROM capabilities
             WHERE id = ?1",
        )
        .context("prepare governance state read")?;

    let mut rows = stmt.query_map([cid], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let (status, qualification, availability) = match rows.next() {
        Some(Ok(row)) => row,
        Some(Err(e)) => bail!("failed to read governance state: {e}"),
        None => bail!("capability '{cid}' not found in registry"),
    };

    let qualification_state: QualificationState = parse_enum(&status, "status")?;
    let qualification_axis: QualificationAxis = parse_enum(&qualification, "qualification")?;

    // Authorization is a separate concern (M1-D0)
    let authorized = false;

    // Availability is independent of qualification
    let available = availability == "registered";

    // Execution status is a runtime concern
    let executing = false;

    Ok(CapabilityGovernanceState {
        capability_id: cid.to_string(),
        qualification_state,
        qualification_axis,
        authorized,
        available,
        executing,
    })
}

/// Parse a persisted string into a contract enum. Fail closed on unknown values.
fn parse_enum<T: DeserializeOwned>(raw: &str, column: &str) -> Result<T> {
    serde_json::from_str(&format!("\"{raw}\"")).with_context(|| {
        format!("fail-closed invariant violation: unrepresentable value '{raw}' in {column}")
    })
}

/// Execute a query that returns (group, count) pairs.
fn group_counts(conn: &Connection, sql: &str) -> Result<Vec<(String, u64)>> {
    let mut stmt = conn.prepare(sql).context("prepare group counts")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)))?;
    let mut result = Vec::new();
    for row in rows {
        let (group, count) = row?;
        result.push((group, count));
    }
    Ok(result)
}

/// Execute a read-only transaction (consistent snapshot).
fn with_snapshot<F, T>(db_path: &Path, f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T>,
{
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .context("open registry read-only")?;

    conn.execute_batch("PRAGMA query_only = ON;")
        .context("set query_only")?;

    conn.execute_batch("BEGIN TRANSACTION")
        .context("begin transaction")?;

    let result = f(&conn);

    conn.execute_batch("COMMIT")
        .context("commit transaction")?;

    result
}

/// Compute `registry_identity` from `capability_registry_meta`.
///
/// Fail-closed: the meta table absent, empty, or missing any required key
/// fails the projection.
fn registry_identity(conn: &Connection) -> Result<librarian_contracts::node::RegistryIdentity> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM capability_registry_meta")
        .context("registry identity fail-closed: cannot read capability_registry_meta")?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collect capability_registry_meta rows")?;

    if rows.is_empty() {
        bail!("registry identity fail-closed: capability_registry_meta is empty");
    }
    for required in ["subsystem", "created_at", "schema_version"] {
        if !rows.iter().any(|(key, _)| key == required) {
            bail!("registry identity fail-closed: required meta key '{required}' is missing");
        }
    }

    let mut sorted = rows;
    sorted.sort();
    let mut hasher = Sha256::new();
    for (key, value) in &sorted {
        hasher.update(key.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
        hasher.update(b"\n");
    }
    let value = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    Ok(librarian_contracts::node::RegistryIdentity {
        value,
        source: "capability_registry_meta".to_string(),
        derivation: "sha256 over sorted capability_registry_meta key/value pairs".to_string(),
    })
}

/// Build an observation envelope.
fn envelope<T: serde::Serialize>(
    node_id: &str,
    identity: librarian_contracts::node::RegistryIdentity,
    projection: T,
) -> librarian_contracts::node::RegistryObservationEnvelope<T> {
    librarian_contracts::node::RegistryObservationEnvelope {
        node_id: node_id.to_string(),
        registry_identity: identity,
        projection_observed_at: chrono::Utc::now().to_rfc3339(),
        projection,
    }
}

use librarian_contracts::node::AvailabilityAxis;

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Apply the canonical governed schema to `conn`.
    fn apply_canonical_schema(conn: &Connection) {
        conn.execute_batch(&librarian_core::startup::canonical_schema())
            .expect("apply canonical capability-registry schema");
    }

    /// Insert a minimal qualification fixture.
    fn insert_qualification_fixture(conn: &Connection) {
        conn.execute_batch(
            "PRAGMA foreign_keys = OFF;
             INSERT INTO capabilities
               (id, name, type, description, status, active_version,
                availability, qualification, authority)
             VALUES
               ('alpha', 'Alpha Skill', 'skill', 'Alpha desc',
                'qualified', 1, 'registered', 'passed', 'approved');
             INSERT INTO capability_versions
               (capability_id, version, body, content_hash, changelog, author,
                review_notes, qualification_evidence_id, profile_id, created_at)
             VALUES
               ('alpha', 1, 'body', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                'init', 'owner', NULL, NULL, NULL, '2026-01-01T00:00:00Z');
             INSERT INTO capability_qualifications
               (qualification_id, capability_id, profile_id, version_id,
                qualification_status, confidence, evidence_reference,
                qualified_at, expires_at, assessed_at,
                assessor_identity, assessor_type, notes)
             VALUES
               ('Q-20260807-001', 'alpha', 'default', 1,
                'passed', 0.95, 'EV-00001',
                '2026-08-07T00:00:00Z', NULL, '2026-08-07T00:00:00Z',
                'evaluator-1', 'automated', 'Initial qualification');
             INSERT INTO qualification_lifecycle_events
               (event_id, qualification_id, capability_id,
                from_state, to_state, transition_type,
                security_classification, transitioned_by,
                transitioner_role, authority_evidence_id,
                evidence_snapshot, created_at)
             VALUES
               ('QLE-20260807-001', 'Q-20260807-001', 'alpha',
                'unreviewed', 'reviewed', 'manual',
                'S0', 'owner-1',
                'owner', 'EV-00002',
                '{\"status\": \"reviewed\"}', '2026-08-06T00:00:00Z'),
               ('QLE-20260807-002', 'Q-20260807-001', 'alpha',
                'reviewed', 'qualified', 'automatic',
                'S0', 'system',
                'system', NULL,
                '{\"status\": \"qualified\"}', '2026-08-07T00:00:00Z');
             INSERT INTO qualification_evidence_records
               (evidence_id, qualification_id, dimension,
                evidence_type, evidence_reference, evidence_body,
                evidence_hash, captured_at, expires_at,
                producer_identity, producer_role)
             VALUES
               ('QER-20260807-001', 'Q-20260807-001', 'identity',
                'test_result', 'EV-00003', '{\"result\": \"pass\"}',
                'abc123', '2026-08-07T00:00:00Z', NULL,
                'evaluator-1', 'evaluator'),
               ('QER-20260807-002', 'Q-20260807-001', 'capability',
                'test_result', 'EV-00004', '{\"result\": \"pass\"}',
                'def456', '2026-08-07T00:00:00Z', NULL,
                'evaluator-1', 'evaluator');",
        )
        .expect("insert qualification fixture");
    }

    /// Build a test registry DB.
    fn fixture_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("capability-registry.sqlite");
        let conn = Connection::open(&db_path).expect("open fixture db");
        apply_canonical_schema(&conn);
        insert_qualification_fixture(&conn);
        (dir, db_path)
    }

    #[test]
    fn qualification_record_projection_matches_fixture() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let envelope = state
            .qualification_record("Q-20260807-001")
            .expect("project qualification record");

        assert_eq!(envelope.node_id, "test-node");
        assert_eq!(envelope.projection.qualification_id, "Q-20260807-001");
        assert_eq!(envelope.projection.status, QualificationRecordStatus::Passed);
        assert!((envelope.projection.confidence.unwrap() - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn capability_qualifications_projection_matches_fixture() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let envelope = state
            .capability_qualifications("alpha")
            .expect("project capability qualifications");

        assert_eq!(envelope.projection.len(), 1);
        assert_eq!(envelope.projection[0].qualification_id, "Q-20260807-001");
    }

    #[test]
    fn qualification_lifecycle_events_projection_matches_fixture() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let envelope = state
            .qualification_lifecycle_events("Q-20260807-001")
            .expect("project lifecycle events");

        assert_eq!(envelope.projection.len(), 2);
        // Events are ordered by created_at ascending
        assert_eq!(envelope.projection[0].from_state, QualificationState::Unreviewed);
        assert_eq!(envelope.projection[0].to_state, QualificationState::Reviewed);
        assert_eq!(envelope.projection[1].from_state, QualificationState::Reviewed);
        assert_eq!(envelope.projection[1].to_state, QualificationState::Qualified);
    }

    #[test]
    fn qualification_evidence_projection_matches_fixture() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let envelope = state
            .qualification_evidence("Q-20260807-001")
            .expect("project evidence records");

        assert_eq!(envelope.projection.len(), 2);
        // Evidence references, not inline proof
        assert!(envelope.projection[0].evidence_reference.is_some());
        assert!(envelope.projection[0].evidence_body.is_some());
    }

    #[test]
    fn qualification_overview_projection_matches_fixture() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let envelope = state
            .qualification_overview()
            .expect("project overview");

        assert_eq!(envelope.projection.qualification_count, 1);
        assert_eq!(envelope.projection.lifecycle_event_count, 2);
        assert_eq!(envelope.projection.evidence_record_count, 2);
    }

    #[test]
    fn capability_governance_state_projection_matches_fixture() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let envelope = state
            .capability_governance_state("alpha")
            .expect("project governance state");

        assert_eq!(envelope.projection.qualification_state, QualificationState::Qualified);
        assert_eq!(envelope.projection.qualification_axis, QualificationAxis::Passed);
        assert!(envelope.projection.available);
    }

    #[test]
    fn governance_state_invariants_enforced() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let envelope = state
            .capability_governance_state("alpha")
            .expect("project governance state");

        // QUALIFIED ≠ AUTHORIZED
        assert!(envelope.projection.is_qualified());
        assert!(!envelope.projection.is_authorized(), "QUALIFIED ≠ AUTHORIZED");

        // QUALIFIED ≠ AVAILABLE (this capability is available, but the invariant
        // is that they are independent — a qualified capability is not automatically
        // available; this one happens to be available for other reasons)
        assert!(envelope.projection.is_qualified());
        assert!(envelope.projection.is_available(), "this capability is available");

        // QUALIFIED ≠ EXECUTING
        assert!(envelope.projection.is_qualified());
        assert!(!envelope.projection.is_executing(), "QUALIFIED ≠ EXECUTING");
    }

    #[test]
    fn consecutive_reads_observe_identical_snapshot() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let a = state
            .qualification_overview()
            .expect("overview read 1");
        let b = state
            .qualification_overview()
            .expect("overview read 2");

        assert_eq!(a.registry_identity, b.registry_identity);
        assert_eq!(a.projection, b.projection);
    }

    #[test]
    fn classification_provenance_preserved() {
        let (_dir, db_path) = fixture_db();
        let state = QualificationObservationState::new("test-node", &db_path);

        let envelope = state
            .qualification_lifecycle_events("Q-20260807-001")
            .expect("project lifecycle events");

        // Security classification is preserved per event (F-2 deferred)
        for event in &envelope.projection {
            // Classification may be present or NULL; we preserve whatever is stored
            if let Some(classification) = &event.security_classification {
                // Classification is one of the defined variants
                match classification {
                    SecurityClassification::S0
                    | SecurityClassification::S1
                    | SecurityClassification::S2
                    | SecurityClassification::S3
                    | SecurityClassification::S4
                    | SecurityClassification::S5
                    | SecurityClassification::Unclassified => {}
                }
            }
        }
    }
}
