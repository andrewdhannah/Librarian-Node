//! # Serialization Contract Types
//!
//! Deterministic serialization utilities for contract types.
//! Ensures that all contract types serialize consistently across
//! implementations and platforms.

use serde::{Deserialize, Serialize};

/// Schema version for serialization contracts.
pub const SERIALIZATION_CONTRACT_VERSION: &str = "1.0.0";

/// A versioned schema identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaId {
    /// Schema name.
    pub name: String,
    /// Semantic version.
    pub version: String,
}

/// Compatibility mode for schema evolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityMode {
    /// Backward compatible — new readers can read old data.
    Backward,
    /// Forward compatible — old readers can read new data (with unknown field preservation).
    Forward,
    /// Fully compatible (both directions).
    Full,
    /// No compatibility guarantee — breaking change.
    None,
}

/// A serialization envelope — wraps any contract type with version metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationEnvelope<T: Serialize> {
    /// Schema identifier.
    pub schema: SchemaId,
    /// Schema version for this envelope.
    pub envelope_version: String,
    /// Compatibility mode.
    pub compatibility: CompatibilityMode,
    /// ISO 8601 timestamp of serialization.
    pub serialized_at: String,
    /// The payload.
    pub payload: T,
}

impl<T: Serialize> SerializationEnvelope<T> {
    /// Create a new envelope.
    pub fn new(schema_name: &str, version: &str, payload: T) -> Self {
        Self {
            schema: SchemaId {
                name: schema_name.to_string(),
                version: version.to_string(),
            },
            envelope_version: SERIALIZATION_CONTRACT_VERSION.into(),
            compatibility: CompatibilityMode::Full,
            serialized_at: String::new(),
            payload,
        }
    }

    /// Serialize to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error>
    where
        T: Serialize,
    {
        serde_json::to_string(self)
    }

    /// Serialize to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error>
    where
        T: Serialize,
    {
        serde_json::to_string_pretty(self)
    }
}

/// Deserialize an envelope from JSON.
pub fn from_json_envelope<'a, T: Deserialize<'a>>(json: &'a str) -> Result<T, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serialize any contract type to a canonical JSON string.
/// This produces a stable, deterministic representation suitable for
/// hashing and comparison across implementations.
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    // Use a sorted-key serialization for deterministic output.
    let mut serializer = serde_json::Serializer::new(Vec::new());
    value.serialize(&mut serializer)?;
    let bytes = serializer.into_inner();
    // Re-parse and re-serialize with sorted keys for canonical form.
    let parsed: serde_json::Value = serde_json::from_slice(&bytes)?;
    serde_json::to_string(&parsed)
}

/// Compute a SHA-256 hash of the canonical JSON representation.
pub fn hash_canonical<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    use sha2::{Digest, Sha256};
    let canonical = to_canonical_json(value)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Forward compatibility fields — preserves unknown fields during
/// deserialization so old readers can process new data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardCompatible<T> {
    /// The known payload.
    pub value: T,
    /// Unknown fields preserved for forward compatibility.
    #[serde(flatten)]
    pub unknown_fields: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::NodeIdentity;

    #[test]
    fn test_serialization_envelope() {
        use crate::identity::{NodeId, NodeRole, PlatformId, Architecture};

        let identity = NodeIdentity {
            node_id: NodeId::new("test-node"),
            display_name: "Test".into(),
            role: NodeRole::Runtime,
            platform: PlatformId::Windows,
            architecture: Architecture::X8664,
            version: "0.1.0".into(),
            contract_version: "1.0.0".into(),
        };

        let envelope = SerializationEnvelope::new("NodeIdentity", "1.0.0", &identity);
        let json = envelope.to_json().unwrap();
        assert!(json.contains("NodeIdentity"));
        assert!(json.contains("serialized_at"));
    }

    #[test]
    fn test_canonical_json_deterministic() {
        use std::collections::BTreeMap;

        let mut map_a = BTreeMap::new();
        map_a.insert("z".to_string(), 1);
        map_a.insert("a".to_string(), 2);
        map_a.insert("m".to_string(), 3);

        let mut map_b = BTreeMap::new();
        map_b.insert("a".to_string(), 2);
        map_b.insert("m".to_string(), 3);
        map_b.insert("z".to_string(), 1);

        let json_a = to_canonical_json(&map_a).unwrap();
        let json_b = to_canonical_json(&map_b).unwrap();
        assert_eq!(json_a, json_b);
    }

    #[test]
    fn test_hash_canonical() {
        let data = serde_json::json!({"hello": "world", "number": 42});
        let hash = hash_canonical(&data).unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn test_forward_compatible() {
        let json = r#"{"value": {"name": "test"}, "unknown_field": "preserved"}"#;
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct Inner {
            name: String,
        }
        let fc: ForwardCompatible<Inner> = serde_json::from_str(json).unwrap();
        assert_eq!(fc.value.name, "test");
        assert!(fc.unknown_fields.contains_key("unknown_field"));
    }
}
