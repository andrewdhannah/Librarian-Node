use super::session_service::SessionService;

pub fn require_active_session(
    session_service: &SessionService,
    session_id: &str,
) -> Result<(), String> {
    let session = session_service
        .get_session(session_id)
        .ok_or_else(|| "Session not found".to_string())?;
    if session.state != "active" {
        return Err(format!(
            "Session {} is not active (state: {})",
            session_id, session.state
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_service() -> SessionService {
        use crate::node::session_service::SessionService;
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_guard.json");
        SessionService::new(path)
    }

    #[test]
    fn test_guard_passes_for_active_session() {
        let mut service = test_service();
        use librarian_contracts::session::SessionStartRequest;
        let session = service.create_session(SessionStartRequest {
            node_id: "test-node".to_string(),
            agent_id: None,
            requested_capabilities: None,
            context: None,
        });
        service.activate_session(&session.session_id).unwrap();
        assert!(require_active_session(&service, &session.session_id).is_ok());
    }

    #[test]
    fn test_guard_rejects_created_session() {
        let mut service = test_service();
        use librarian_contracts::session::SessionStartRequest;
        let session = service.create_session(SessionStartRequest {
            node_id: "test-node".to_string(),
            agent_id: None,
            requested_capabilities: None,
            context: None,
        });
        let result = require_active_session(&service, &session.session_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not active"));
    }

    #[test]
    fn test_guard_rejects_closed_session() {
        let mut service = test_service();
        use librarian_contracts::session::SessionStartRequest;
        let session = service.create_session(SessionStartRequest {
            node_id: "test-node".to_string(),
            agent_id: None,
            requested_capabilities: None,
            context: None,
        });
        service.activate_session(&session.session_id).unwrap();
        service.close_session(&session.session_id).unwrap();
        let result = require_active_session(&service, &session.session_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not active"));
    }

    #[test]
    fn test_guard_rejects_nonexistent_session() {
        let service = test_service();
        let result = require_active_session(&service, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
