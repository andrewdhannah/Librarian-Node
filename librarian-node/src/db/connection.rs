//! SQLite connection configuration.
//!
//! Every opened connection consistently establishes the required SQLite posture:
//! - WAL journal mode (set once at DB init)
//! - Foreign keys enforced per connection
//! - Busy timeout for concurrent access
//! - Synchronous = NORMAL for local operational DB

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Configure a connection with the standard PRAGMAs.
/// Called on every new connection from RuntimeDatabase::open_connection().
pub fn configure_connection(conn: &Connection) -> Result<()> {
    // synchronous is per-connection; must be set on every connection, not just at init.
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;",
    )
    .context("Failed to configure connection PRAGMAs")?;

    Ok(())
}

/// Configure the database-level settings (run once at DB creation).
/// Sets WAL journal mode (this is database-level, persists across connections).
pub fn configure_database_init(conn: &Connection) -> Result<()> {
    // journal_mode = WAL is database-level (stored in file header), only needs setting once.
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .context("Failed to set journal_mode=WAL")?;

    Ok(())
}

/// Verify that the database has the expected PRAGMA settings.
/// Returns Ok(()) if all checks pass, Err with details if not.
pub fn verify_pragmas(conn: &Connection) -> Result<()> {
    // Check WAL mode
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .context("Failed to query journal_mode")?;
    if journal_mode != "wal" {
        anyhow::bail!(
            "Expected journal_mode=wal, got '{}'",
            journal_mode
        );
    }

    // Check foreign keys
    let fk: i32 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .context("Failed to query foreign_keys")?;
    if fk != 1 {
        anyhow::bail!("Expected foreign_keys=ON, got {}", fk);
    }

    // Check busy timeout
    let busy: i32 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .context("Failed to query busy_timeout")?;
    if busy != 5000 {
        anyhow::bail!("Expected busy_timeout=5000, got {}", busy);
    }

    // Check synchronous (returns integer: 0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA)
    let sync: i32 = conn
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .context("Failed to query synchronous")?;
    if sync != 1 {
        anyhow::bail!("Expected synchronous=NORMAL (1), got {}", sync);
    }

    Ok(())
}
