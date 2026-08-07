//! # Registry Observation Projection Module (RUST-MIGRATION-M1-B, increment 2)
//!
//! Implements `RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001` (Canonical,
//! `232edfb`; suite `REGISTRY-OBSERVATION-SCHEMA-001`): module-level, read-only
//! projections over the governed capability-registry SQLite database
//! (`capability-registry.sqlite`, canonical Phase 2 + Phase 3 schema embedded in
//! `librarian-core`).
//!
//! Boundary invariants (contract §1.2 / §5.2; work order RUST-MIGRATION-M1-B
//! retained hard boundaries):
//!
//! - **Read-only.** Every projection opens the database read-only and executes
//!   its reads inside ONE transaction (consistent snapshot, contract §4);
//!   `PRAGMA query_only = ON` additionally fails any write attempt at the
//!   SQLite level. There is no write path, no transition method, no authority
//!   method, and no code path that invokes governed registry operations.
//! - **Sealed at construction.** [`RegistryObservationState`] mirrors the M0B
//!   [`RuntimeApiState`](crate::runtime_api::RuntimeApiState) pattern: node id
//!   and registry path are fixed at construction, there are no setters, and no
//!   request path can change them (contract §5.3).
//! - **No transport.** This is the semantic read boundary. HTTP and MCP
//!   adapters consume these projections; they are NOT part of this module
//!   (contract §1.1).
//! - **No competing authority.** SQL provides the frozen storage semantics;
//!   these projections report them. No `RegistryStore`, no DDL changes, no
//!   mutation (work order §0).
//! - **Fail-closed.** Unknown persisted enum values, an unrepresentable
//!   `active_version`, a missing capability, or an identity that cannot be
//!   established from `capability_registry_meta` fail the projection with an
//!   explicit invariant-violation error — never coercion (owner decisions 3
//!   and the §3 fail-closed rule).
//! - **Deterministic.** Same registry state → same payload bytes (fields,
//!   values, ordering). `projection_observed_at` is the only variable envelope
//!   field (contract §3.1).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use librarian_contracts::node::{
    AuthorityAxis, AvailabilityAxis, CapabilityDependency, CapabilityId, CapabilityObservation,
    CapabilityRelationshipType, CapabilityType, CapabilityTypeDefinition, CapabilityVersion,
    CapabilityVersionRecord, QualificationAxis, QualificationState, RegistryIdentity,
    RegistryObservationEnvelope, RegistryOverview, TypeCategory,
};
use rusqlite::{Connection, OpenFlags};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

/// Sealed registry observation state (M0B `RuntimeApiState` pattern, contract
/// §5.3). Immutable after construction; the projections read from the governed
/// registry at the path sealed here.
#[derive(Clone)]
pub struct RegistryObservationState {
    node_id: String,
    db_path: Arc<PathBuf>,
}

impl RegistryObservationState {
    /// Seal the observing node id and the governed registry database path.
    pub fn new(node_id: impl Into<String>, db_path: impl Into<PathBuf>) -> Self {
        RegistryObservationState {
            node_id: node_id.into(),
            db_path: Arc::new(db_path.into()),
        }
    }

    /// Contract §3 — `capability(id)`: identity + lifecycle + assurance axes
    /// for one capability (single `capabilities` row lookup).
    ///
    /// Fails closed if the capability is absent, any persisted axis/type value
    /// is unrepresentable, or `active_version` is not a positive integer.
    pub fn capability(&self, id: &str) -> Result<RegistryObservationEnvelope<CapabilityObservation>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        let id = id.to_string();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let projection = read_capability(conn, &id)?
                .with_context(|| format!("capability '{id}' not found in registry"))?;
            Ok(envelope(&node_id, identity, projection))
        })
    }

    /// Contract §3 — `capability_versions(id)`: append-only version history,
    /// ascending by integer version (CR-I-001). Exposes existence +
    /// `content_hash` evidence anchor, never `body` (owner decision 1).
    ///
    /// Fails closed if the capability is absent.
    pub fn capability_versions(
        &self,
        id: &str,
    ) -> Result<RegistryObservationEnvelope<Vec<CapabilityVersionRecord>>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        let id = id.to_string();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            ensure_capability_exists(conn, &id)?;
            let projection = read_versions(conn, &id)?;
            Ok(envelope(&node_id, identity, projection))
        })
    }

    /// Contract §3 — `capability_dependencies(id)`: dependency references
    /// ordered by `(capability_id, dependency_id)` (CR-I-005). Payload is the
    /// locked M1-A `CapabilityDependency` verbatim — `created_at` is not
    /// carried (owner decision 2).
    ///
    /// Fails closed if the capability is absent.
    pub fn capability_dependencies(
        &self,
        id: &str,
    ) -> Result<RegistryObservationEnvelope<Vec<CapabilityDependency>>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        let id = id.to_string();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            ensure_capability_exists(conn, &id)?;
            let projection = read_dependencies(conn, &id)?;
            Ok(envelope(&node_id, identity, projection))
        })
    }

    /// Contract §3 — `capability_types()`: taxonomy rows ordered by
    /// `capability_type_id`. Rows are NOT the `CapabilityType` enum (naming
    /// guard, owner decision 4).
    pub fn capability_types(&self) -> Result<RegistryObservationEnvelope<Vec<CapabilityTypeDefinition>>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let projection = read_type_definitions(conn)?;
            Ok(envelope(&node_id, identity, projection))
        })
    }

    /// Contract §3 — `registry_overview()`: deterministic counts over the four
    /// tables, all read in the same snapshot. Group sets are fixed (enum `ALL`
    /// order) with zero-count groups present (owner decision 6).
    pub fn registry_overview(&self) -> Result<RegistryObservationEnvelope<RegistryOverview>> {
        let db_path = Arc::clone(&self.db_path);
        let node_id = self.node_id.clone();
        with_snapshot(&db_path, |conn| {
            let identity = registry_identity(conn)?;
            let projection = read_overview(conn)?;
            Ok(envelope(&node_id, identity, projection))
        })
    }
}

/// RFC 3339 UTC observation timestamp, seconds precision with `Z` designator
/// (the only variable envelope field; same discipline as the M0B runtime API).
fn observed_at() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Wrap a deterministic projection payload in the observation envelope.
fn envelope<T>(
    node_id: &str,
    registry_identity: RegistryIdentity,
    projection: T,
) -> RegistryObservationEnvelope<T> {
    RegistryObservationEnvelope {
        node_id: node_id.to_string(),
        registry_identity,
        projection_observed_at: observed_at(),
        projection,
    }
}

/// Execute `f` inside ONE read-only transaction on a single connection.
///
/// Contract §4 (consistent snapshot): the identity read and every payload read
/// of a projection execute within this single transaction. SQLite's default
/// deferred transaction takes a SHARED lock at the first read and holds it
/// until commit, so no intervening write from another connection can appear
/// mid-projection. `PRAGMA query_only = ON` fails any write attempt at the
/// SQLite level — the transaction issues SELECT only (work order §4).
fn with_snapshot<T>(db_path: &Path, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("open registry read-only: {}", db_path.display()))?;
    conn.execute_batch("PRAGMA query_only = ON;")
        .context("set query_only on projection connection")?;
    conn.execute_batch("BEGIN;")
        .context("begin projection snapshot")?;
    let result = f(&conn);
    match result {
        Ok(value) => {
            conn.execute_batch("COMMIT;")
                .context("commit projection snapshot")?;
            Ok(value)
        }
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(err)
        }
    }
}

/// Parse a persisted string into a contract enum. Unknown persisted value →
/// explicit invariant-violation error (owner decision 3: fail closed, no
/// coercion — an unknown value is schema/semantic drift, not a default).
fn parse_enum<T: DeserializeOwned>(raw: &str, column: &str) -> Result<T> {
    serde_json::from_str(&format!("\"{raw}\"")).with_context(|| {
        format!("fail-closed invariant violation: unrepresentable value '{raw}' in {column}")
    })
}

/// Compute `registry_identity` from `capability_registry_meta` (source map §3;
/// contract §3.1 allowed source (a)).
///
/// Fail-closed (source map §3): the meta table absent (SELECT fails), empty,
/// or missing any required key (`subsystem`, `created_at`, `schema_version`)
/// fails the projection — no placeholder or node-derived identity is served.
///
/// `value` = SHA-256 over the sorted `(key, value)` pairs, serialized as
/// `key=value\n` lines (owner decision 5: sorting explicit and deterministic
/// before hashing). The frozen `schema_version = '2'` Phase 3 quirk is hashed
/// as stored — it is never "fixed" (source map §3).
fn registry_identity(conn: &Connection) -> Result<RegistryIdentity> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM capability_registry_meta")
        .context("registry identity fail-closed: cannot read capability_registry_meta (registry identity source)")?;
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
    sorted.sort(); // deterministic: lexicographic (key, value)
    let mut hasher = Sha256::new();
    for (key, value) in &sorted {
        hasher.update(key.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
        hasher.update(b"\n");
    }
    let value = hex(&hasher.finalize());

    Ok(RegistryIdentity {
        value,
        source: "capability_registry_meta".to_string(),
        derivation: "sha256 over sorted capability_registry_meta key/value pairs".to_string(),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether a capability row exists (read within the same snapshot).
fn capability_exists(conn: &Connection, id: &str) -> Result<bool> {
    let exists: i64 = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM capabilities WHERE id = ?1)",
            [id],
            |row| row.get(0),
        )
        .context("check capability existence")?;
    Ok(exists != 0)
}

/// Fail the projection if the capability is absent (id-keyed projections must
/// not serve empty facts for a capability the registry does not know).
fn ensure_capability_exists(conn: &Connection, id: &str) -> Result<()> {
    if !capability_exists(conn, id)? {
        bail!("capability '{id}' not found in registry");
    }
    Ok(())
}

/// Source map §2.1 — `capability(id)` row projection.
fn read_capability(conn: &Connection, id: &str) -> Result<Option<CapabilityObservation>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, type, active_version, status, availability, qualification, authority \
             FROM capabilities WHERE id = ?1",
        )
        .context("prepare capability projection")?;
    let mut rows = stmt
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .context("query capability projection")?;

    let mut found = None;
    if let Some(row) = rows.next() {
        let (id, name, cap_type, active_version, status, availability, qualification, authority) =
            row.context("read capability projection row")?;
        let capability_id =
            CapabilityId::new(id.clone()).context("invalid persisted capability id")?;
        let version = match active_version {
            // NULL active_version = no active version (valid).
            None => None,
            // 0 is unrepresentable: CHECK (version > 0) in capability_versions
            // makes it impossible to reference (CR-I-003) — fail closed rather
            // than manufacture a semantic state.
            Some(v) if v <= 0 => bail!(
                "fail-closed invariant violation: unrepresentable active_version {v} for '{id}' (CHECK version > 0, CR-I-003)"
            ),
            Some(v) => Some(CapabilityVersion::new(v as u32).expect("positive version")),
        };
        found = Some(CapabilityObservation {
            capability_id,
            name,
            capability_type: parse_enum(&cap_type, "capabilities.type")?,
            version,
            lifecycle_state: parse_enum(&status, "capabilities.status")?,
            availability: parse_enum(&availability, "capabilities.availability")?,
            qualification: parse_enum(&qualification, "capabilities.qualification")?,
            authority: parse_enum(&authority, "capabilities.authority")?,
        });
    }
    Ok(found)
}

/// Source map §2.2 — `capability_versions(id)`: append-only rows ascending by
/// integer version; `body` deliberately not exposed (owner decision 1).
fn read_versions(conn: &Connection, id: &str) -> Result<Vec<CapabilityVersionRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT capability_id, version, content_hash, changelog, author, review_notes, \
                    qualification_evidence_id, profile_id, created_at \
             FROM capability_versions WHERE capability_id = ?1 ORDER BY version ASC",
        )
        .context("prepare versions projection")?;
    let rows = stmt
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .context("query versions projection")?;

    let mut records = Vec::new();
    for row in rows {
        let (capability_id, version, content_hash, changelog, author, review_notes, evidence_id, profile_id, created_at) =
            row.context("read version projection row")?;
        let version_u32 = u32::try_from(version)
            .with_context(|| format!("fail-closed: version {version} is not a u32 (CHECK version > 0)"))?;
        records.push(CapabilityVersionRecord {
            capability_id: CapabilityId::new(capability_id).context("invalid persisted capability id")?,
            version: CapabilityVersion::new(version_u32).with_context(|| {
                format!("fail-closed invariant violation: version {version} violates CHECK version > 0 (CR-I-001)")
            })?,
            content_hash,
            changelog,
            author,
            review_notes,
            qualification_evidence_id: evidence_id,
            profile_id,
            created_at,
        });
    }
    Ok(records)
}

/// Source map §2.3 — `capability_dependencies(id)`: outgoing dependency
/// references ordered by `(capability_id, dependency_id)`; payload is the
/// locked `CapabilityDependency` verbatim (owner decision 2).
fn read_dependencies(conn: &Connection, id: &str) -> Result<Vec<CapabilityDependency>> {
    let mut stmt = conn
        .prepare(
            "SELECT capability_id, dependency_id, required, relationship_type \
             FROM capability_dependencies WHERE capability_id = ?1 \
             ORDER BY capability_id ASC, dependency_id ASC",
        )
        .context("prepare dependencies projection")?;
    let rows = stmt
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .context("query dependencies projection")?;

    let mut dependencies = Vec::new();
    for row in rows {
        let (capability_id, dependency_id, required, relationship_type) =
            row.context("read dependency projection row")?;
        let required = match required {
            0 => false,
            1 => true,
            other => bail!(
                "fail-closed invariant violation: unrepresentable required value {other} (schema stores 0/1)"
            ),
        };
        let relationship_type =
            parse_enum(&relationship_type, "capability_dependencies.relationship_type")?;
        let dependency = CapabilityDependency::new(
            CapabilityId::new(capability_id).context("invalid persisted capability id")?,
            CapabilityId::new(dependency_id).context("invalid persisted dependency id")?,
            required,
            relationship_type,
        )
        .context("fail-closed: self-dependency row violates CHECK (capability_id != dependency_id) (CR-I-005)")?;
        dependencies.push(dependency);
    }
    Ok(dependencies)
}

/// Source map §2.4 — `capability_types()`: taxonomy rows ordered by
/// `capability_type_id`. Rows are NOT the `CapabilityType` enum (naming guard).
fn read_type_definitions(conn: &Connection) -> Result<Vec<CapabilityTypeDefinition>> {
    let mut stmt = conn
        .prepare(
            "SELECT capability_type_id, name, description, category, default_profile_id, default_policy_id \
             FROM capability_types ORDER BY capability_type_id ASC",
        )
        .context("prepare type taxonomy projection")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .context("query type taxonomy projection")?;

    let mut definitions = Vec::new();
    for row in rows {
        let (capability_type_id, name, description, category, default_profile_id, default_policy_id) =
            row.context("read type taxonomy row")?;
        definitions.push(CapabilityTypeDefinition {
            capability_type_id,
            name,
            description,
            category: parse_enum(&category, "capability_types.category")?,
            default_profile_id,
            default_policy_id,
        });
    }
    Ok(definitions)
}

/// Source map §2.5 — `registry_overview()`: deterministic counts over the four
/// tables in the same snapshot; group sets fixed in enum `ALL` order with
/// zero-count groups present (owner decision 6). No M1-C/M1-D/F-2 facts.
fn read_overview(conn: &Connection) -> Result<RegistryOverview> {
    Ok(RegistryOverview {
        capability_count: count_rows(conn, "capabilities")?,
        by_status: group_counts(
            conn,
            "SELECT status, COUNT(*) FROM capabilities GROUP BY status",
            &QualificationState::ALL,
            QualificationState::as_str,
            "capabilities.status",
        )?,
        by_availability: group_counts(
            conn,
            "SELECT availability, COUNT(*) FROM capabilities GROUP BY availability",
            &AvailabilityAxis::ALL,
            AvailabilityAxis::as_str,
            "capabilities.availability",
        )?,
        by_qualification: group_counts(
            conn,
            "SELECT qualification, COUNT(*) FROM capabilities GROUP BY qualification",
            &QualificationAxis::ALL,
            QualificationAxis::as_str,
            "capabilities.qualification",
        )?,
        by_authority: group_counts(
            conn,
            "SELECT authority, COUNT(*) FROM capabilities GROUP BY authority",
            &AuthorityAxis::ALL,
            AuthorityAxis::as_str,
            "capabilities.authority",
        )?,
        by_type: group_counts(
            conn,
            "SELECT type, COUNT(*) FROM capabilities GROUP BY type",
            &CapabilityType::ALL,
            CapabilityType::as_str,
            "capabilities.type",
        )?,
        version_count: count_rows(conn, "capability_versions")?,
        dependency_count: count_rows(conn, "capability_dependencies")?,
        dependency_by_relationship: group_counts(
            conn,
            "SELECT relationship_type, COUNT(*) FROM capability_dependencies GROUP BY relationship_type",
            &CapabilityRelationshipType::ALL,
            CapabilityRelationshipType::as_str,
            "capability_dependencies.relationship_type",
        )?,
        type_count: count_rows(conn, "capability_types")?,
        types_by_category: group_counts(
            conn,
            "SELECT category, COUNT(*) FROM capability_types GROUP BY category",
            &TypeCategory::ALL,
            TypeCategory::as_str,
            "capability_types.category",
        )?,
    })
}

fn count_rows(conn: &Connection, table: &str) -> Result<u64> {
    let n: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
        .with_context(|| format!("count rows in {table}"))?;
    Ok(n as u64)
}

/// Count rows per serialized enum value for a GROUP BY query.
///
/// Output order is the enum's `ALL` array order; zero-count groups are present
/// (owner decision 6). Unknown persisted group values fail closed via
/// [`parse_enum`] (owner decision 3).
fn group_counts<T>(
    conn: &Connection,
    sql: &str,
    all: &[T],
    name_of: impl Fn(T) -> &'static str,
    column: &str,
) -> Result<Vec<(String, u64)>>
where
    T: Copy + DeserializeOwned,
{
    let mut stmt = conn.prepare(sql).context("prepare group count")?;
    let mut counts: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut rows = stmt.query([]).context("query group count")?;
    while let Some(row) = rows.next()? {
        let key: String = row.get(0)?;
        let n: i64 = row.get(1)?;
        let _: T = parse_enum(&key, column)?; // fail closed on unknown group value
        *counts.entry(key).or_insert(0) += n as u64;
    }
    Ok(all
        .iter()
        .map(|value| {
            let name = name_of(*value).to_string();
            let count = counts.get(&name).copied().unwrap_or(0);
            (name, count)
        })
        .collect())
}
