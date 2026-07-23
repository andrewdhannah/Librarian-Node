use std::path::PathBuf;

use librarian_contracts::custody::{
    CustodyChain, CustodyMetadata, IntegrityError, IntegrityReport, ProvenanceGraph,
    ProvenanceLink, ProvenanceQuery, ProvenanceResult, ReceiptEnvelope, RetentionPolicy,
    RetentionResult,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    chain: Option<CustodyChain>,
    envelopes: Vec<ReceiptEnvelope>,
}

pub struct CustodyService {
    envelopes: Vec<ReceiptEnvelope>,
    chain: Option<CustodyChain>,
    persistence_path: PathBuf,
}

impl CustodyService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (chain, envelopes) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.chain, state.envelopes),
                    Err(_) => (None, Vec::new()),
                },
                Err(_) => (None, Vec::new()),
            }
        } else {
            (None, Vec::new())
        };

        CustodyService {
            envelopes,
            chain,
            persistence_path,
        }
    }

    pub fn append_receipt(
        &mut self,
        node_id: &str,
        receipt_type: &str,
        receipt_id: &str,
        receipt_payload: serde_json::Value,
        metadata: Option<CustodyMetadata>,
    ) -> ReceiptEnvelope {
        let envelope_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();

        let payload_str = serde_json::to_string(&receipt_payload).unwrap_or_default();
        let receipt_hash = format!("{:x}", sha2::Sha256::digest(payload_str.as_bytes()));

        let previous_envelope_id = self.envelopes.last().map(|e| e.envelope_id.clone());
        let previous_envelope_hash = self
            .envelopes
            .last()
            .map(|e| e.chain_hash.clone());

        let chain_hash = match (&previous_envelope_hash, &self.chain) {
            (Some(prev_hash), _) => {
                let combined = format!("{}{}", prev_hash, receipt_hash);
                format!("{:x}", sha2::Sha256::digest(combined.as_bytes()))
            }
            (None, _) => {
                format!("{:x}", sha2::Sha256::digest(receipt_hash.as_bytes()))
            }
        };

        let envelope = ReceiptEnvelope {
            envelope_id: envelope_id.clone(),
            node_id: node_id.to_string(),
            receipt_type: receipt_type.to_string(),
            receipt_id: receipt_id.to_string(),
            receipt_payload,
            receipt_hash,
            previous_envelope_id,
            previous_envelope_hash,
            chain_hash,
            timestamp: timestamp.clone(),
            metadata,
        };

        self.envelopes.push(envelope.clone());

        let first_envelope_id = self
            .envelopes
            .first()
            .map(|e| e.envelope_id.clone())
            .unwrap_or_default();
        let last_envelope_id = envelope_id;

        let chain_id = self
            .chain
            .as_ref()
            .map(|c| c.chain_id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let created_at = self
            .chain
            .as_ref()
            .map(|c| c.created_at.clone())
            .unwrap_or_else(|| timestamp.clone());

        self.chain = Some(CustodyChain {
            chain_id,
            node_id: node_id.to_string(),
            created_at,
            envelope_count: self.envelopes.len() as u32,
            first_envelope_id,
            last_envelope_id,
            last_chain_hash: envelope.chain_hash.clone(),
            status: "active".to_string(),
        });

        self.persist();
        envelope
    }

    pub fn get_chain(&self) -> Option<CustodyChain> {
        self.chain.clone()
    }

    pub fn get_envelope(&self, envelope_id: &str) -> Option<ReceiptEnvelope> {
        self.envelopes
            .iter()
            .find(|e| e.envelope_id == envelope_id)
            .cloned()
    }

    pub fn get_envelopes_by_type(&self, receipt_type: &str) -> Vec<ReceiptEnvelope> {
        self.envelopes
            .iter()
            .filter(|e| e.receipt_type == receipt_type)
            .cloned()
            .collect()
    }

    pub fn get_envelopes_by_time_range(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Vec<ReceiptEnvelope> {
        self.envelopes
            .iter()
            .filter(|e| {
                let after_from = match from {
                    Some(f) => e.timestamp.as_str() >= f,
                    None => true,
                };
                let before_to = match to {
                    Some(t) => e.timestamp.as_str() <= t,
                    None => true,
                };
                after_from && before_to
            })
            .cloned()
            .collect()
    }

    pub fn query_provenance(&self, query: &ProvenanceQuery) -> Vec<ProvenanceResult> {
        self.envelopes
            .iter()
            .filter(|e| {
                let node_match = match &query.node_id {
                    Some(nid) => e.node_id == *nid,
                    None => true,
                };
                let type_match = match &query.receipt_type {
                    Some(rt) => e.receipt_type == *rt,
                    None => true,
                };
                let after_from = match &query.from_timestamp {
                    Some(f) => e.timestamp.as_str() >= f.as_str(),
                    None => true,
                };
                let before_to = match &query.to_timestamp {
                    Some(t) => e.timestamp.as_str() <= t.as_str(),
                    None => true,
                };
                node_match && type_match && after_from && before_to
            })
            .map(|e| {
                let summary = format!("{}:{}", e.receipt_type, e.receipt_id);
                ProvenanceResult {
                    envelope: e.clone(),
                    receipt_type: e.receipt_type.clone(),
                    receipt_summary: summary,
                }
            })
            .collect()
    }

    pub fn get_provenance_graph(&self) -> ProvenanceGraph {
        let node_id = self
            .chain
            .as_ref()
            .map(|c| c.node_id.clone())
            .unwrap_or_default();

        let mut relationships = Vec::new();
        for envelope in &self.envelopes {
            if let Some(prev_id) = &envelope.previous_envelope_id {
                relationships.push(ProvenanceLink {
                    from_envelope_id: prev_id.clone(),
                    to_envelope_id: envelope.envelope_id.clone(),
                    relationship: "precedes".to_string(),
                });
            }
        }

        ProvenanceGraph {
            node_id,
            envelopes: self.envelopes.clone(),
            relationships,
        }
    }

    pub fn verify_integrity(&self) -> IntegrityReport {
        let chain_id = self
            .chain
            .as_ref()
            .map(|c| c.chain_id.clone())
            .unwrap_or_default();
        let node_id = self
            .chain
            .as_ref()
            .map(|c| c.node_id.clone())
            .unwrap_or_default();
        let envelope_count = self.envelopes.len() as u32;
        let mut errors = Vec::new();
        let mut envelopes_checked = 0u32;
        let mut expected_prev_hash: Option<String> = None;

        for envelope in &self.envelopes {
            envelopes_checked += 1;

            let payload_str = serde_json::to_string(&envelope.receipt_payload).unwrap_or_default();
            let computed_receipt_hash =
                format!("{:x}", sha2::Sha256::digest(payload_str.as_bytes()));

            if computed_receipt_hash != envelope.receipt_hash {
                errors.push(IntegrityError {
                    envelope_id: envelope.envelope_id.clone(),
                    error_type: "tampered_payload".to_string(),
                    details: format!(
                        "Receipt hash mismatch: expected {}, computed {}",
                        envelope.receipt_hash, computed_receipt_hash
                    ),
                });
                continue;
            }

            if let Some(ref expected_prev) = expected_prev_hash {
                match &envelope.previous_envelope_hash {
                    Some(actual_prev) if actual_prev == expected_prev => {}
                    Some(actual_prev) => {
                        errors.push(IntegrityError {
                            envelope_id: envelope.envelope_id.clone(),
                            error_type: "broken_chain".to_string(),
                            details: format!(
                                "Previous envelope hash mismatch: expected {}, got {}",
                                expected_prev, actual_prev
                            ),
                        });
                    }
                    None => {
                        errors.push(IntegrityError {
                            envelope_id: envelope.envelope_id.clone(),
                            error_type: "missing_previous".to_string(),
                            details: format!(
                                "Expected previous envelope hash {}, but none found",
                                expected_prev
                            ),
                        });
                    }
                }

                let expected_chain_hash_input =
                    format!("{}{}", expected_prev, computed_receipt_hash);
                let computed_chain_hash = format!(
                    "{:x}",
                    sha2::Sha256::digest(expected_chain_hash_input.as_bytes())
                );

                if computed_chain_hash != envelope.chain_hash {
                    errors.push(IntegrityError {
                        envelope_id: envelope.envelope_id.clone(),
                        error_type: "hash_mismatch".to_string(),
                        details: format!(
                            "Chain hash mismatch: expected {}, computed {}",
                            envelope.chain_hash, computed_chain_hash
                        ),
                    });
                }
            } else {
                let expected_chain_hash = format!(
                    "{:x}",
                    sha2::Sha256::digest(computed_receipt_hash.as_bytes())
                );
                if expected_chain_hash != envelope.chain_hash {
                    errors.push(IntegrityError {
                        envelope_id: envelope.envelope_id.clone(),
                        error_type: "hash_mismatch".to_string(),
                        details: format!(
                            "First envelope chain hash mismatch: expected {}, computed {}",
                            envelope.chain_hash, expected_chain_hash
                        ),
                    });
                }
            }

            expected_prev_hash = Some(envelope.chain_hash.clone());
        }

        IntegrityReport {
            chain_id,
            node_id,
            verified: errors.is_empty(),
            envelope_count,
            envelopes_checked,
            errors,
            verified_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn apply_retention(&mut self, policy: &RetentionPolicy) -> RetentionResult {
        let before = self.envelopes.len() as u32;

        if let Some(max_env) = policy.max_envelopes {
            while self.envelopes.len() > max_env as usize {
                self.envelopes.remove(0);
            }
        }

        if let Some(days) = policy.retention_days {
            let cutoff = chrono::Utc::now()
                - chrono::Duration::days(days as i64);
            self.envelopes.retain(|e| {
                match chrono::DateTime::parse_from_rfc3339(&e.timestamp) {
                    Ok(ts) => {
                        let utc_ts = ts.with_timezone(&chrono::Utc);
                        utc_ts > cutoff
                    }
                    Err(_) => true,
                }
            });
        }

        let after = self.envelopes.len() as u32;
        let deleted = before.saturating_sub(after);

        if let Some(ref mut chain) = self.chain {
            chain.envelope_count = after;
            if !self.envelopes.is_empty() {
                chain.first_envelope_id = self.envelopes.first().unwrap().envelope_id.clone();
                chain.last_envelope_id = self.envelopes.last().unwrap().envelope_id.clone();
                chain.last_chain_hash = self.envelopes.last().unwrap().chain_hash.clone();
            }
        }

        self.persist();

        RetentionResult {
            policy_id: policy.policy_id.clone(),
            envelopes_before: before,
            envelopes_after: after,
            archived: 0,
            deleted,
            applied_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn seed_identity(
        &mut self,
        node_id: &str,
        identity_payload: serde_json::Value,
        metadata: CustodyMetadata,
    ) {
        if self.chain.is_some() {
            return;
        }
        self.append_receipt(node_id, "identity", node_id, identity_payload, Some(metadata));
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            chain: self.chain.clone(),
            envelopes: self.envelopes.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_service() -> CustodyService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("custody.json");
        CustodyService::new(path)
    }

    fn make_envelope(service: &mut CustodyService, rtype: &str, rid: &str) -> ReceiptEnvelope {
        let payload = serde_json::json!({
            "receipt_id": rid,
            "node_id": "test-node",
        });
        service.append_receipt("test-node", rtype, rid, payload, None)
    }

    #[test]
    fn test_envelope_creation_and_hash() {
        let mut service = test_service();
        let payload = serde_json::json!({"id": "test-001"});
        let env = service.append_receipt(
            "test-node",
            "identity",
            "id-001",
            payload,
            None,
        );
        assert_eq!(env.receipt_type, "identity");
        assert_eq!(env.receipt_id, "id-001");
        assert_eq!(env.node_id, "test-node");
        assert!(!env.envelope_id.is_empty());
        assert!(!env.receipt_hash.is_empty());
        assert!(!env.chain_hash.is_empty());
        assert!(env.previous_envelope_id.is_none());
        assert!(env.previous_envelope_hash.is_none());
    }

    #[test]
    fn test_chain_linking() {
        let mut service = test_service();
        let e1 = make_envelope(&mut service, "identity", "id-001");
        let e2 = make_envelope(&mut service, "registration", "reg-001");

        assert_eq!(e2.previous_envelope_id, Some(e1.envelope_id.clone()));
        assert_eq!(e2.previous_envelope_hash, Some(e1.chain_hash.clone()));
        assert_ne!(e1.chain_hash, e2.chain_hash);
    }

    #[test]
    fn test_chain_hash_is_cumulative() {
        let mut service = test_service();
        let e1 = make_envelope(&mut service, "identity", "id-001");
        let e2 = make_envelope(&mut service, "registration", "reg-001");

        let expected_chain =
            format!("{:x}", sha2::Sha256::digest(format!("{}{}", e1.chain_hash, e2.receipt_hash).as_bytes()));
        assert_eq!(e2.chain_hash, expected_chain);
    }

    #[test]
    fn test_full_chain_integrity_verification_passes() {
        let mut service = test_service();
        make_envelope(&mut service, "identity", "id-001");
        make_envelope(&mut service, "registration", "reg-001");
        make_envelope(&mut service, "session", "sess-001");

        let report = service.verify_integrity();
        assert!(report.verified);
        assert_eq!(report.envelope_count, 3);
        assert_eq!(report.envelopes_checked, 3);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_integrity_detects_tampered_payload() {
        let mut service = test_service();
        make_envelope(&mut service, "identity", "id-001");

        let mut tampered = service.envelopes[0].clone();
        tampered.receipt_payload = serde_json::json!({"tampered": true});
        service.envelopes[0] = tampered;

        let report = service.verify_integrity();
        assert!(!report.verified);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].error_type, "tampered_payload");
    }

    #[test]
    fn test_integrity_detects_broken_link() {
        let mut service = test_service();
        make_envelope(&mut service, "identity", "id-001");
        make_envelope(&mut service, "registration", "reg-001");

        let mut broken = service.envelopes[1].clone();
        broken.previous_envelope_hash = Some("badhash".to_string());
        service.envelopes[1] = broken;

        let report = service.verify_integrity();
        assert!(!report.verified);
        assert!(report.errors.iter().any(|e| e.error_type == "broken_chain"));
    }

    #[test]
    fn test_provenance_query_by_type() {
        let mut service = test_service();
        make_envelope(&mut service, "identity", "id-001");
        make_envelope(&mut service, "registration", "reg-001");
        make_envelope(&mut service, "session", "sess-001");

        let query = ProvenanceQuery {
            node_id: None,
            receipt_type: Some("session".to_string()),
            from_timestamp: None,
            to_timestamp: None,
            limit: None,
        };

        let results = service.query_provenance(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].receipt_type, "session");
    }

    #[test]
    fn test_provenance_query_by_time_range() {
        let mut service = test_service();
        let e1 = make_envelope(&mut service, "identity", "id-001");
        let e2 = make_envelope(&mut service, "registration", "reg-001");

        let query = ProvenanceQuery {
            node_id: None,
            receipt_type: None,
            from_timestamp: Some(e1.timestamp.clone()),
            to_timestamp: Some(e2.timestamp.clone()),
            limit: None,
        };

        let results = service.query_provenance(&query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_retention_policy_application() {
        let mut service = test_service();
        for i in 0..5 {
            let rid = format!("id-{:03}", i);
            make_envelope(&mut service, "test", &rid);
        }

        assert_eq!(service.envelopes.len(), 5);

        let policy = RetentionPolicy {
            policy_id: "pol-001".to_string(),
            node_id: "test-node".to_string(),
            max_envelopes: Some(3),
            retention_days: None,
            auto_archive: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let result = service.apply_retention(&policy);
        assert_eq!(result.envelopes_before, 5);
        assert_eq!(result.envelopes_after, 3);
        assert_eq!(result.deleted, 2);
    }

    #[test]
    fn test_get_envelope() {
        let mut service = test_service();
        let e1 = make_envelope(&mut service, "identity", "id-001");

        let found = service.get_envelope(&e1.envelope_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().envelope_id, e1.envelope_id);

        let not_found = service.get_envelope("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_envelopes_by_type() {
        let mut service = test_service();
        make_envelope(&mut service, "identity", "id-001");
        make_envelope(&mut service, "session", "sess-001");
        make_envelope(&mut service, "session", "sess-002");

        let sessions = service.get_envelopes_by_type("session");
        assert_eq!(sessions.len(), 2);

        let identities = service.get_envelopes_by_type("identity");
        assert_eq!(identities.len(), 1);
    }

    #[test]
    fn test_get_chain() {
        let mut service = test_service();
        assert!(service.get_chain().is_none());

        make_envelope(&mut service, "identity", "id-001");
        let chain = service.get_chain();
        assert!(chain.is_some());
        let chain = chain.unwrap();
        assert_eq!(chain.envelope_count, 1);
        assert_eq!(chain.status, "active");
    }

    #[test]
    fn test_get_provenance_graph() {
        let mut service = test_service();
        make_envelope(&mut service, "identity", "id-001");
        make_envelope(&mut service, "registration", "reg-001");

        let graph = service.get_provenance_graph();
        assert_eq!(graph.envelopes.len(), 2);
        assert_eq!(graph.relationships.len(), 1);
        assert_eq!(graph.relationships[0].relationship, "precedes");
    }

    #[test]
    fn test_empty_service() {
        let service = test_service();
        assert!(service.get_chain().is_none());
        assert!(service.get_envelope("any").is_none());
        assert!(service.get_envelopes_by_type("test").is_empty());

        let report = service.verify_integrity();
        assert!(report.verified);
        assert_eq!(report.envelope_count, 0);
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("custody_persist.json");
        {
            let mut service = CustodyService::new(path.clone());
            make_envelope(&mut service, "identity", "id-001");
            make_envelope(&mut service, "registration", "reg-001");
        }
        {
            let service = CustodyService::new(path.clone());
            assert_eq!(service.envelopes.len(), 2);
            assert!(service.get_chain().is_some());
            let chain = service.get_chain().unwrap();
            assert_eq!(chain.envelope_count, 2);
        }
    }
}
