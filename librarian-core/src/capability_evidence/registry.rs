//! Adapter registry — manages multiple evaluator adapters.
//!
//! Tracks registered evaluators and provides lookup, listing, and
//! fixture enumeration. Adapters are registered once and can then
//! be looked up by their evaluator_id.

use std::collections::HashMap;

use super::adapter::EvaluatorAdapter;

/// Registry of capability evaluator adapters.
pub struct AdapterRegistry {
    adapters: HashMap<String, Box<dyn EvaluatorAdapter>>,
}

impl AdapterRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Create a registry pre-populated with the given adapters.
    ///
    /// If multiple adapters share the same evaluator_id, only the
    /// first one is kept and the rest are silently ignored.
    pub fn from_adapters(adapters: Vec<Box<dyn EvaluatorAdapter>>) -> Self {
        let mut registry = Self::new();
        for adapter in adapters {
            registry.register(adapter);
        }
        registry
    }

    /// Register an adapter.
    ///
    /// If an adapter with the same evaluator_id is already registered,
    /// the existing one is kept and the new one is ignored (returns false).
    /// Returns true if the adapter was successfully registered.
    pub fn register(&mut self, adapter: Box<dyn EvaluatorAdapter>) -> bool {
        let id = adapter.evaluator_id().to_string();
        if self.adapters.contains_key(&id) {
            return false;
        }
        self.adapters.insert(id, adapter);
        true
    }

    /// Get an adapter by evaluator_id.
    pub fn get(&self, evaluator_id: &str) -> Option<&dyn EvaluatorAdapter> {
        self.adapters.get(evaluator_id).map(|b| b.as_ref())
    }

    /// Check if an evaluator is registered.
    pub fn contains(&self, evaluator_id: &str) -> bool {
        self.adapters.contains_key(evaluator_id)
    }

    /// List all registered evaluator IDs.
    pub fn list_evaluators(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.adapters.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Number of registered adapters.
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }

    /// Total fixture count across all registered evaluators.
    pub fn total_fixture_count(&self) -> usize {
        self.adapters.values().map(|a| a.fixture_count()).sum()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::adapter::AdapterError;
    use super::super::models::{CapabilityFixture, CapabilityResult, ValidationMethod};

    struct TestAdapter {
        id: String,
        version: String,
        upstream: String,
        count: usize,
    }

    impl EvaluatorAdapter for TestAdapter {
        fn evaluator_id(&self) -> &str { &self.id }
        fn evaluator_version(&self) -> &str { &self.version }
        fn upstream_project(&self) -> &str { &self.upstream }
        fn fixture_count(&self) -> usize { self.count }
        fn fixture_at(&self, index: usize) -> Result<CapabilityFixture, AdapterError> {
            if index >= self.count {
                return Err(AdapterError::FixtureIndexOutOfBounds {
                    evaluator_id: self.id.clone(),
                    index,
                    total: self.count,
                });
            }
            Ok(CapabilityFixture {
                fixture_id: format!("{}-{}", self.id, index),
                version: self.version.clone(),
                category: "test".to_string(),
                description: format!("Fixture {}", index),
                prompt: "p".to_string(),
                expected_outcome: "o".to_string(),
                validation: ValidationMethod::Contains { expected: "o".to_string() },
            })
        }
        fn evaluate_fixture(
            &self,
            _fixture: &CapabilityFixture,
            _output: &str,
        ) -> CapabilityResult {
            CapabilityResult::Pass
        }
    }

    fn make_adapter(id: &str, count: usize) -> Box<dyn EvaluatorAdapter> {
        Box::new(TestAdapter {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            upstream: "test".to_string(),
            count,
        })
    }

    #[test]
    fn test_new_registry_is_empty() {
        let r = AdapterRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn test_register_single() {
        let mut r = AdapterRegistry::new();
        assert!(r.register(make_adapter("a", 3)));
        assert_eq!(r.len(), 1);
        assert!(!r.is_empty());
    }

    #[test]
    fn test_register_duplicate_id_ignored() {
        let mut r = AdapterRegistry::new();
        assert!(r.register(make_adapter("a", 3)));
        assert!(!r.register(make_adapter("a", 5))); // duplicate ID ignored
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_get_adapter() {
        let mut r = AdapterRegistry::new();
        r.register(make_adapter("a", 3));
        let a = r.get("a").unwrap();
        assert_eq!(a.evaluator_id(), "a");
        assert!(r.get("nonexistent").is_none());
    }

    #[test]
    fn test_contains() {
        let mut r = AdapterRegistry::new();
        r.register(make_adapter("a", 3));
        assert!(r.contains("a"));
        assert!(!r.contains("b"));
    }

    #[test]
    fn test_list_evaluators_sorted() {
        let mut r = AdapterRegistry::new();
        r.register(make_adapter("c", 1));
        r.register(make_adapter("a", 1));
        r.register(make_adapter("b", 1));
        assert_eq!(r.list_evaluators(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_total_fixture_count() {
        let mut r = AdapterRegistry::new();
        r.register(make_adapter("a", 3));
        r.register(make_adapter("b", 5));
        assert_eq!(r.total_fixture_count(), 8);
    }

    #[test]
    fn test_from_adapters() {
        let adapters: Vec<Box<dyn EvaluatorAdapter>> = vec![
            make_adapter("a", 1),
            make_adapter("b", 2),
        ];
        let r = AdapterRegistry::from_adapters(adapters);
        assert_eq!(r.len(), 2);
    }
}
