//! # Custody Protocol
//!
//! Portable custody protocol implementation. Manages document check-out,
//! check-in, integrity verification, and lease management.
//!
//! The custody protocol ensures:
//! - Documents are checked out before editing
//! - Content integrity is verified on check-in (SHA-256)
//! - Staleness is detected (document changed after check-out)
//! - Custody events form an append-only evidence chain

use anyhow::Result;
use librarian_contracts::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use super::db::GovernanceDb;

/// A checked-out document with its content snapshot.
#[derive(Debug, Clone)]
pub struct CheckedOutDocument {
    /// The document reference.
    pub document_reference: String,
    /// The project this document belongs to.
    pub project_id: String,
    /// The node that checked it out.
    pub checked_out_by: String,
    /// SHA-256 hash of the content at check-out time.
    pub original_hash: String,
    /// ISO 8601 timestamp of check-out.
    pub checked_out_at: String,
    /// When this lease expires.
    pub expires_at: Option<String>,
}

/// Errors that can occur during custody operations.
#[derive(Debug, thiserror::Error)]
pub enum CustodyError {
    #[error("Document is already checked out: {0}")]
    AlreadyCheckedOut(String),
    #[error("Document is not checked out: {0}")]
    NotCheckedOut(String),
    #[error("Content has changed since check-out (stale): {0}")]
    StaleContent(String),
    #[error("Lease has expired for: {0}")]
    LeaseExpired(String),
    #[error("Custody mode mismatch: expected {expected:?}, got {actual:?}")]
    ModeMismatch {
        expected: CustodyMode,
        actual: CustodyMode,
    },
    #[error("Database error: {0}")]
    Database(String),
}

impl From<anyhow::Error> for CustodyError {
    fn from(e: anyhow::Error) -> Self {
        CustodyError::Database(e.to_string())
    }
}

/// The custody protocol engine.
pub struct CustodyEngine {
    db: GovernanceDb,
    /// In-memory check-out registry (could be DB-backed in production).
    checkouts: std::sync::Mutex<HashMap<String, CheckedOutDocument>>,
}

impl CustodyEngine {
    /// Create a new custody engine.
    pub fn new(db: GovernanceDb) -> Self {
        Self {
            db,
            checkouts: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Compute the SHA-256 hash of content bytes.
    pub fn hash_content(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Check out a document for editing.
    pub fn check_out(
        &self,
        document_reference: &str,
        project_id: &str,
        node_id: &str,
        content_hash: &str,
        lease_minutes: Option<u64>,
    ) -> Result<CheckedOutDocument, CustodyError> {
        let mut checkouts = self.checkouts.lock().unwrap();

        // Check if already checked out
        if checkouts.contains_key(document_reference) {
            return Err(CustodyError::AlreadyCheckedOut(document_reference.to_string()));
        }

        let now = chrono::Utc::now();
        let checked_out_at = now.to_rfc3339();
        let expires_at = lease_minutes.map(|m| {
            (now + chrono::Duration::minutes(m as i64)).to_rfc3339()
        });

        let doc = CheckedOutDocument {
            document_reference: document_reference.to_string(),
            project_id: project_id.to_string(),
            checked_out_by: node_id.to_string(),
            original_hash: content_hash.to_string(),
            checked_out_at: checked_out_at.clone(),
            expires_at: expires_at.clone(),
        };

        checkouts.insert(document_reference.to_string(), doc.clone());

        // Record custody event
        let event = CustodyEvent {
            event_id: format!("co-{}-{}", node_id, now.timestamp()),
            project_id: project_id.to_string(),
            mcp_session_id: String::new(),
            node_id: node_id.to_string(),
            window_id: None,
            work_packet_id: None,
            tool_name: "custody_engine".into(),
            authority_role: CustodyAuthorityRole::Agent,
            document_reference: document_reference.to_string(),
            custody_action: CustodyAction::Claim,
            previous_custody_mode: None,
            resulting_custody_mode: Some(CustodyMode::LocalWorkingCopy),
            mutation_allowance: Some(MutationAllowance::WorkingCopyOnly),
            decision_reference: None,
            provenance_receipt: None,
            refusal_reason: None,
            target_project_id: None,
            target_session_id: None,
            target_node_id: None,
            timestamp: checked_out_at,
        };
        self.db.record_custody_event(&event)?;

        Ok(doc)
    }

    /// Check in a document. Verifies content integrity.
    pub fn check_in(
        &self,
        document_reference: &str,
        new_content_hash: &str,
        node_id: &str,
    ) -> Result<CheckedOutDocument, CustodyError> {
        let mut checkouts = self.checkouts.lock().unwrap();

        let doc = checkouts
            .get(document_reference)
            .ok_or_else(|| CustodyError::NotCheckedOut(document_reference.to_string()))?;

        // Check lease expiry
        if let Some(ref expires) = doc.expires_at {
            let expires_dt = chrono::DateTime::parse_from_rfc3339(expires)
                .map_err(|e| CustodyError::Database(e.to_string()))?;
            if chrono::Utc::now() > expires_dt {
                return Err(CustodyError::LeaseExpired(document_reference.to_string()));
            }
        }

        // Verify content integrity
        if new_content_hash != doc.original_hash {
            return Err(CustodyError::StaleContent(document_reference.to_string()));
        }

        let now = chrono::Utc::now().to_rfc3339();

        // Record release custody event
        let event = CustodyEvent {
            event_id: format!("ci-{}-{}", node_id, chrono::Utc::now().timestamp()),
            project_id: doc.project_id.clone(),
            mcp_session_id: String::new(),
            node_id: node_id.to_string(),
            window_id: None,
            work_packet_id: None,
            tool_name: "custody_engine".into(),
            authority_role: CustodyAuthorityRole::Agent,
            document_reference: document_reference.to_string(),
            custody_action: CustodyAction::Release,
            previous_custody_mode: Some(CustodyMode::LocalWorkingCopy),
            resulting_custody_mode: Some(CustodyMode::LocalCanonical),
            mutation_allowance: Some(MutationAllowance::ReadOnly),
            decision_reference: None,
            provenance_receipt: None,
            refusal_reason: None,
            target_project_id: None,
            target_session_id: None,
            target_node_id: None,
            timestamp: now,
        };
        self.db.record_custody_event(&event)?;

        let doc = checkouts.remove(document_reference).unwrap();
        Ok(doc)
    }

    /// Check if a document is currently checked out.
    pub fn is_checked_out(&self, document_reference: &str) -> bool {
        let checkouts = self.checkouts.lock().unwrap();
        checkouts.contains_key(document_reference)
    }

    /// Get check-out info for a document.
    pub fn get_checkout(&self, document_reference: &str) -> Option<CheckedOutDocument> {
        let checkouts = self.checkouts.lock().unwrap();
        checkouts.get(document_reference).cloned()
    }

    /// Get the number of active check-outs.
    pub fn active_checkout_count(&self) -> usize {
        let checkouts = self.checkouts.lock().unwrap();
        checkouts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_engine() -> CustodyEngine {
        let db = GovernanceDb::open_in_memory().unwrap();
        CustodyEngine::new(db)
    }

    #[test]
    fn test_check_out_and_check_in() {
        let engine = setup_engine();
        let content = b"hello world";
        let hash = CustodyEngine::hash_content(content);

        let doc = engine.check_out("doc://test", "test-project", "node-1", &hash, Some(60)).unwrap();
        assert_eq!(doc.document_reference, "doc://test");
        assert!(engine.is_checked_out("doc://test"));

        let released = engine.check_in("doc://test", &hash, "node-1").unwrap();
        assert_eq!(released.document_reference, "doc://test");
        assert!(!engine.is_checked_out("doc://test"));
    }

    #[test]
    fn test_double_check_out_rejected() {
        let engine = setup_engine();
        let hash = CustodyEngine::hash_content(b"test");
        engine.check_out("doc://test", "test-project", "node-1", &hash, Some(60)).unwrap();
        let result = engine.check_out("doc://test", "test-project", "node-2", &hash, Some(60));
        assert!(result.is_err());
    }

    #[test]
    fn test_stale_content_detected() {
        let engine = setup_engine();
        let original_hash = CustodyEngine::hash_content(b"original");
        let modified_hash = CustodyEngine::hash_content(b"modified");

        engine.check_out("doc://test", "test-project", "node-1", &original_hash, Some(60)).unwrap();
        let result = engine.check_in("doc://test", &modified_hash, "node-1");
        assert!(result.is_err());
        match result {
            Err(CustodyError::StaleContent(_)) => {}
            _ => panic!("Expected StaleContent error"),
        }
    }

    #[test]
    fn test_check_in_without_checkout() {
        let engine = setup_engine();
        let hash = CustodyEngine::hash_content(b"test");
        let result = engine.check_in("doc://nonexistent", &hash, "node-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_is_deterministic() {
        let hash1 = CustodyEngine::hash_content(b"deterministic content");
        let hash2 = CustodyEngine::hash_content(b"deterministic content");
        assert_eq!(hash1, hash2);
    }
}
