use std::collections::HashMap;
use std::path::PathBuf;

use librarian_contracts::core_integration::NodeProjection;
use librarian_contracts::fleet::{
    CapabilityComparison, DiscoveryScanRequest, DiscoveryScanResult, FleetCapabilityView,
    FleetHealth, FleetHealthBreakdown, FleetInventory, FleetOverview, NodeInventoryEntry,
};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedInventory {
    nodes: Vec<NodeInventoryEntry>,
}

pub struct FleetService {
    known_nodes: Vec<NodeInventoryEntry>,
    persistence_path: PathBuf,
}

impl FleetService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let known_nodes = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedInventory>(&content) {
                    Ok(state) => state.nodes,
                    Err(_) => Vec::new(),
                },
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        FleetService {
            known_nodes,
            persistence_path,
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedInventory {
            nodes: self.known_nodes.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    fn derive_status(projection: &NodeProjection) -> String {
        projection.status.clone()
    }

    pub fn register_local_node(&mut self, projection: NodeProjection) {
        let entry = self.build_entry_from_projection(&projection);
        let status = Self::derive_status(&projection);
        let idx = self.known_nodes.iter().position(|n| n.node_id == entry.node_id);
        if let Some(i) = idx {
            let existing = &mut self.known_nodes[i];
            existing.display_name = entry.display_name;
            existing.status = status;
            existing.last_seen_at = Some(projection.generated_at.clone());
            existing.runtime_version = entry.runtime_version;
            existing.platform = entry.platform;
            existing.capability_count = entry.capability_count;
            existing.verified_capability_count = entry.verified_capability_count;
            existing.session_count = entry.session_count;
            existing.custody_envelope_count = entry.custody_envelope_count;
            existing.registered = entry.registered;
            existing.bootstrap_completed = entry.bootstrap_completed;
        } else {
            self.known_nodes.push(entry);
        }
        self.persist();
    }

    fn build_entry_from_projection(&self, projection: &NodeProjection) -> NodeInventoryEntry {
        let identity = projection.identity.as_object().cloned().unwrap_or_default();
        let registered = projection.registration.is_some();
        let capability_count = projection
            .capabilities
            .as_ref()
            .and_then(|c| c.as_object())
            .and_then(|o| o.get("capabilities"))
            .and_then(|c| c.as_array())
            .map(|a| a.len() as u32)
            .unwrap_or(0);

        NodeInventoryEntry {
            node_id: projection.node_id.clone(),
            display_name: identity
                .get("display_name")
                .and_then(|v| v.as_str())
                .unwrap_or(&projection.node_id)
                .to_string(),
            status: projection.status.clone(),
            last_seen_at: Some(projection.generated_at.clone()),
            runtime_version: projection.node_version.clone(),
            platform: identity
                .get("platform")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            capability_count,
            verified_capability_count: if projection.capabilities_verified {
                capability_count
            } else {
                0
            },
            session_count: projection.session_count,
            custody_envelope_count: projection.custody_envelope_count,
            registered,
            bootstrap_completed: projection.bootstrap_completed,
            last_health_status: Some(projection.status.clone()),
        }
    }

    pub fn get_local_entry(&self, node_id: &str) -> Option<NodeInventoryEntry> {
        self.known_nodes
            .iter()
            .find(|n| n.node_id == node_id)
            .cloned()
    }

    pub fn add_or_update_node(&mut self, entry: NodeInventoryEntry) {
        let idx = self.known_nodes.iter().position(|n| n.node_id == entry.node_id);
        if let Some(i) = idx {
            self.known_nodes[i] = entry;
        } else {
            self.known_nodes.push(entry);
        }
        self.persist();
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.known_nodes.retain(|n| n.node_id != node_id);
        self.persist();
    }

    pub fn get_inventory(&self) -> FleetInventory {
        let total_count = self.known_nodes.len() as u32;
        let online_count = self
            .known_nodes
            .iter()
            .filter(|n| n.status == "online")
            .count() as u32;
        let offline_count = self
            .known_nodes
            .iter()
            .filter(|n| n.status == "offline" || n.status == "unknown")
            .count() as u32;

        FleetInventory {
            nodes: self.known_nodes.clone(),
            total_count,
            online_count,
            offline_count,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_node(&self, node_id: &str) -> Option<NodeInventoryEntry> {
        self.known_nodes
            .iter()
            .find(|n| n.node_id == node_id)
            .cloned()
    }

    pub fn get_fleet_health(&self) -> FleetHealth {
        let total_nodes = self.known_nodes.len() as u32;
        let mut healthy_nodes = 0u32;
        let mut degraded_nodes = 0u32;
        let mut unhealthy_nodes = 0u32;
        let mut online_nodes = 0u32;
        let mut offline_nodes = 0u32;

        for node in &self.known_nodes {
            match node.status.as_str() {
                "online" => {
                    online_nodes += 1;
                    match node.last_health_status.as_deref() {
                        Some("healthy") | None => healthy_nodes += 1,
                        Some("degraded") => degraded_nodes += 1,
                        Some("unhealthy") => unhealthy_nodes += 1,
                        _ => healthy_nodes += 1,
                    }
                }
                "offline" => {
                    offline_nodes += 1;
                    unhealthy_nodes += 1;
                }
                _ => {
                    offline_nodes += 1;
                    unhealthy_nodes += 1;
                }
            }
        }

        FleetHealth {
            total_nodes,
            healthy_nodes,
            degraded_nodes,
            unhealthy_nodes,
            online_nodes,
            offline_nodes,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_health_breakdown(&self) -> Vec<FleetHealthBreakdown> {
        let mut component_counts: HashMap<String, (u32, u32, u32)> = HashMap::new();

        for node in &self.known_nodes {
            let status = node.last_health_status.as_deref().unwrap_or("unknown");
            let components = match status {
                "healthy" => {
                    vec![("identity", "healthy"), ("capabilities", "healthy")]
                }
                "degraded" => {
                    vec![("registration", "degraded")]
                }
                _ => {
                    vec![("connectivity", "unhealthy")]
                }
            };

            for (component, comp_status) in components {
                let entry = component_counts
                    .entry(component.to_string())
                    .or_insert((0, 0, 0));
                match comp_status {
                    "healthy" => entry.0 += 1,
                    "degraded" => entry.1 += 1,
                    _ => entry.2 += 1,
                }
            }
        }

        component_counts
            .into_iter()
            .map(|(component, (healthy, degraded, unhealthy))| FleetHealthBreakdown {
                component,
                healthy,
                degraded,
                unhealthy,
                total: healthy + degraded + unhealthy,
            })
            .collect()
    }

    pub fn get_capability_comparison(&self) -> Vec<CapabilityComparison> {
        let mut capability_map: HashMap<String, (Vec<String>, Vec<String>)> = HashMap::new();

        for node in &self.known_nodes {
            if node.capability_count > 0 {
                let entry = capability_map
                    .entry("general.capability".to_string())
                    .or_insert_with(|| (Vec::new(), Vec::new()));
                entry.0.push(node.node_id.clone());

                if node.verified_capability_count > 0 {
                    entry.1.push(node.node_id.clone());
                }
            }
        }

        let mut comparisons = Vec::new();
        for (capability_type, (nodes_with, verified_nodes)) in capability_map {
            comparisons.push(CapabilityComparison {
                capability_type,
                nodes_with_capability: nodes_with,
                total_nodes_with_capability: comparisons.len() as u32,
                total_nodes_verified: 0,
            });

            if let Some(last) = comparisons.last_mut() {
                let count = last.nodes_with_capability.len() as u32;
                let verified_count = verified_nodes.len() as u32;
                last.total_nodes_with_capability = count;
                last.total_nodes_verified = verified_count;
            }
        }

        comparisons
    }

    pub fn get_fleet_capability_view(&self) -> FleetCapabilityView {
        FleetCapabilityView {
            comparisons: self.get_capability_comparison(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn prepare_discovery_scan(&self) -> DiscoveryScanRequest {
        DiscoveryScanRequest {
            scan_id: Uuid::new_v4().to_string(),
            initiated_by: "fleet_service".to_string(),
            scan_method: "local_broadcast".to_string(),
            initiated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn process_scan_result(&mut self, result: DiscoveryScanResult) {
        let known_ids: std::collections::HashSet<String> =
            self.known_nodes.iter().map(|n| n.node_id.clone()).collect();

        let mut new_nodes = Vec::new();
        for node in &result.nodes {
            if !known_ids.contains(&node.node_id) {
                new_nodes.push(node.node_id.clone());
                self.known_nodes.push(node.clone());
            } else {
                let idx = self
                    .known_nodes
                    .iter()
                    .position(|n| n.node_id == node.node_id);
                if let Some(i) = idx {
                    self.known_nodes[i] = node.clone();
                }
            }
        }

        self.persist();
    }

    pub fn get_fleet_overview(&self) -> FleetOverview {
        let inventory = self.get_inventory();
        let health = self.get_fleet_health();

        FleetOverview {
            inventory,
            health,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn mark_offline(&mut self, node_id: &str) {
        if let Some(node) = self.known_nodes.iter_mut().find(|n| n.node_id == node_id) {
            node.status = "offline".to_string();
            node.last_health_status = Some("unhealthy".to_string());
            self.persist();
        }
    }

    pub fn mark_nodes_offline_not_seen_since(&mut self, cutoff: &str) {
        for node in &mut self.known_nodes {
            let last_seen = node.last_seen_at.as_deref().unwrap_or("");
            if last_seen < cutoff && node.status == "online" {
                node.status = "offline".to_string();
                node.last_health_status = Some("unhealthy".to_string());
            }
        }
        self.persist();
    }

    pub fn node_count(&self) -> usize {
        self.known_nodes.len()
    }

    pub fn all_nodes(&self) -> &[NodeInventoryEntry] {
        &self.known_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_projection(node_id: &str, status: &str) -> NodeProjection {
        NodeProjection {
            projection_id: uuid::Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            node_version: "0.1.0".to_string(),
            identity: serde_json::json!({
                "display_name": format!("node-{}", node_id),
                "platform": "test",
                "node_id": node_id,
                "runtime_version": "0.1.0",
                "contract_version": "1",
                "first_seen_at": "2026-07-15T12:00:00Z",
            }),
            registration: Some(serde_json::json!({"status": "registered"})),
            capabilities: Some(serde_json::json!({
                "capabilities": [{"type": "llm.inference", "available": true}]
            })),
            capabilities_verified: true,
            session_count: 0,
            bootstrap_completed: true,
            custody_envelope_count: 0,
            last_integrity_hash: None,
            status: status.to_string(),
        }
    }

    fn test_entry(node_id: &str, status: &str) -> NodeInventoryEntry {
        NodeInventoryEntry {
            node_id: node_id.to_string(),
            display_name: format!("node-{}", node_id),
            status: status.to_string(),
            last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
            runtime_version: "0.1.0".to_string(),
            platform: "test".to_string(),
            capability_count: 3,
            verified_capability_count: 3,
            session_count: 2,
            custody_envelope_count: 1,
            registered: true,
            bootstrap_completed: true,
            last_health_status: Some(if status == "online" { "healthy".to_string() } else { "unhealthy".to_string() }),
        }
    }

    fn test_service() -> (FleetService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fleet-inventory.json");
        let service = FleetService::new(path);
        (service, dir)
    }

    #[test]
    fn test_inventory_returns_correct_count_and_statuses() {
        let (mut service, _dir) = test_service();

        service.add_or_update_node(test_entry("node-a", "online"));
        service.add_or_update_node(test_entry("node-b", "online"));
        service.add_or_update_node(test_entry("node-c", "offline"));

        let inventory = service.get_inventory();
        assert_eq!(inventory.total_count, 3);
        assert_eq!(inventory.online_count, 2);
        assert_eq!(inventory.offline_count, 1);
        assert_eq!(inventory.nodes.len(), 3);
    }

    #[test]
    fn test_fleet_health_aggregates_correctly() {
        let (mut service, _dir) = test_service();

        service.add_or_update_node(test_entry("node-a", "online"));
        service.add_or_update_node(test_entry("node-b", "offline"));
        service.add_or_update_node(test_entry("node-c", "unknown"));

        let health = service.get_fleet_health();
        assert_eq!(health.total_nodes, 3);
        assert_eq!(health.healthy_nodes, 1); // node-a is online with healthy last_health
        assert_eq!(health.unhealthy_nodes, 2); // node-b and node-c are offline/unknown
        assert_eq!(health.online_nodes, 1);
        assert_eq!(health.offline_nodes, 2);
    }

    #[test]
    fn test_capability_comparison_shows_which_nodes_have_which() {
        let (mut service, _dir) = test_service();

        let mut a = test_entry("node-a", "online");
        a.capability_count = 3;
        a.verified_capability_count = 3;
        let mut b = test_entry("node-b", "online");
        b.capability_count = 2;
        b.verified_capability_count = 0;
        let mut c = test_entry("node-c", "offline");
        c.capability_count = 0;
        c.verified_capability_count = 0;

        service.add_or_update_node(a);
        service.add_or_update_node(b);
        service.add_or_update_node(c);

        let comparisons = service.get_capability_comparison();
        assert!(!comparisons.is_empty());

        for comp in &comparisons {
            assert!(comp.total_nodes_with_capability > 0);
            assert!(!comp.nodes_with_capability.is_empty());
        }
    }

    #[test]
    fn test_node_add_update_remove_lifecycle() {
        let (mut service, _dir) = test_service();

        assert_eq!(service.node_count(), 0);

        service.add_or_update_node(test_entry("node-a", "online"));
        assert_eq!(service.node_count(), 1);

        let mut updated = test_entry("node-a", "offline");
        updated.display_name = "node-a-updated".to_string();
        service.add_or_update_node(updated);
        assert_eq!(service.node_count(), 1);

        let entry = service.get_node("node-a").unwrap();
        assert_eq!(entry.display_name, "node-a-updated");
        assert_eq!(entry.status, "offline");

        service.remove_node("node-a");
        assert_eq!(service.node_count(), 0);
        assert!(service.get_node("node-a").is_none());
    }

    #[test]
    fn test_fleet_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fleet-inventory-persist.json");

        {
            let mut service = FleetService::new(path.clone());
            service.add_or_update_node(test_entry("node-a", "online"));
            service.add_or_update_node(test_entry("node-b", "offline"));
        }

        {
            let service = FleetService::new(path.clone());
            let inventory = service.get_inventory();
            assert_eq!(inventory.total_count, 2);
            assert_eq!(inventory.online_count, 1);
            assert_eq!(inventory.offline_count, 1);

            let node_a = service.get_node("node-a").unwrap();
            assert_eq!(node_a.status, "online");
        }
    }

    #[test]
    fn test_fleet_overview_includes_inventory_and_health() {
        let (mut service, _dir) = test_service();

        service.add_or_update_node(test_entry("node-a", "online"));
        service.add_or_update_node(test_entry("node-b", "offline"));

        let overview = service.get_fleet_overview();
        assert_eq!(overview.inventory.total_count, 2);
        assert_eq!(overview.health.total_nodes, 2);
        assert_eq!(overview.health.online_nodes, 1);
        assert_eq!(overview.health.offline_nodes, 1);
        assert!(!overview.generated_at.is_empty());
    }

    #[test]
    fn test_discovery_scan_processes_new_nodes() {
        let (mut service, _dir) = test_service();

        service.add_or_update_node(test_entry("existing-node", "online"));

        let result = DiscoveryScanResult {
            scan_id: "scan-001".to_string(),
            nodes_found: 2,
            nodes: vec![
                test_entry("existing-node", "online"),
                test_entry("new-node", "online"),
            ],
            new_nodes: vec!["new-node".to_string()],
            completed_at: chrono::Utc::now().to_rfc3339(),
        };

        service.process_scan_result(result);
        assert_eq!(service.node_count(), 2);

        let new_node = service.get_node("new-node").unwrap();
        assert_eq!(new_node.status, "online");
    }

    #[test]
    fn test_register_local_node_from_projection() {
        let (mut service, _dir) = test_service();

        let projection = test_projection("local-node", "online");
        service.register_local_node(projection);

        assert_eq!(service.node_count(), 1);
        let entry = service.get_node("local-node").unwrap();
        assert_eq!(entry.status, "online");
        assert!(entry.registered);
        assert!(entry.bootstrap_completed);
    }

    #[test]
    fn test_mark_offline() {
        let (mut service, _dir) = test_service();

        service.add_or_update_node(test_entry("node-a", "online"));
        service.mark_offline("node-a");

        let entry = service.get_node("node-a").unwrap();
        assert_eq!(entry.status, "offline");
        assert_eq!(entry.last_health_status, Some("unhealthy".to_string()));
    }

    #[test]
    fn test_get_fleet_health_empty() {
        let (service, _dir) = test_service();
        let health = service.get_fleet_health();
        assert_eq!(health.total_nodes, 0);
        assert_eq!(health.healthy_nodes, 0);
        assert_eq!(health.degraded_nodes, 0);
        assert_eq!(health.unhealthy_nodes, 0);
    }

    #[test]
    fn test_fleet_overview_generated_at_not_empty() {
        let (service, _dir) = test_service();
        let overview = service.get_fleet_overview();
        assert!(!overview.generated_at.is_empty());
    }

    #[test]
    fn test_discovery_prepare_scan_request() {
        let (service, _dir) = test_service();
        let request = service.prepare_discovery_scan();
        assert_eq!(request.scan_method, "local_broadcast");
        assert_eq!(request.initiated_by, "fleet_service");
        assert!(!request.scan_id.is_empty());
    }

    #[test]
    fn test_health_breakdown_empty() {
        let (service, _dir) = test_service();
        let breakdown = service.get_health_breakdown();
        assert!(breakdown.is_empty() || breakdown.iter().all(|b| b.total == 0));
    }

    #[test]
    fn test_fleet_capability_view_generated() {
        let (mut service, _dir) = test_service();

        let mut entry = test_entry("node-cap", "online");
        entry.capability_count = 5;
        entry.verified_capability_count = 3;
        service.add_or_update_node(entry);

        let view = service.get_fleet_capability_view();
        assert!(!view.generated_at.is_empty());
    }
}
