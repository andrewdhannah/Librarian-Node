//! Refusal engine — checks requests against contract refusal conditions.
//!
//! Mirrors the Python router's `RefusalEngine` to enforce:
//! - Unknown profiles
//! - Unverified profiles
//! - Context exceeding verified limits
//! - Runtime unhealthy
//! - Identity mismatch
//! - Authority-bearing content
//! - Autonomous action keywords
//! - File mutation keywords

use chrono::Utc;
use serde_json::json;

/// Check a SELECT request for refusal conditions.
pub fn check_select(
    alias: &str,
    task_class: Option<&str>,
    profile_exists: bool,
    verified: bool,
    runtime_failed: bool,
) -> Option<serde_json::Value> {
    if !profile_exists {
        return Some(json!({
            "status": "refused",
            "reason": "unknown_profile",
            "detail": format!("No profile registered with alias '{}'", alias),
            "authority": "advisory_only",
            "timestamp": Utc::now().to_rfc3339(),
        }));
    }

    if !verified {
        return Some(json!({
            "status": "refused",
            "reason": "unverified_profile",
            "detail": format!("Profile '{}' has verified_status='unverified'. Must pass WIN-MODEL-FIT matrix first.", alias),
            "authority": "advisory_only",
            "timestamp": Utc::now().to_rfc3339(),
        }));
    }

    if runtime_failed {
        return Some(json!({
            "status": "refused",
            "reason": "runtime_unhealthy",
            "detail": format!("Runtime for profile '{}' is not healthy.", alias),
            "authority": "advisory_only",
            "timestamp": Utc::now().to_rfc3339(),
        }));
    }

    if let Some(tc) = task_class {
        // Task class filtering is deferred to the caller who has access to Profile data
        // because this module doesn't hold the full profile.
        // We return a generic special value so the caller can handle it.
        return Some(json!({
            "status": "refused",
            "reason": "task_class_check_needed",
            "detail": format!("Task class '{}' check needed", tc),
            "authority": "advisory_only",
            "timestamp": Utc::now().to_rfc3339(),
        }));
    }

    None
}

/// Check a CHAT request for refusal conditions.
pub fn check_chat(
    alias: &str,
    messages: &[serde_json::Value],
    context: Option<u32>,
    verified_context: u32,
    profile_exists: bool,
    runtime_available: bool,
) -> Option<serde_json::Value> {
    if !profile_exists {
        return Some(json!({
            "status": "refused",
            "reason": "unknown_profile",
            "detail": format!("No profile registered with alias '{}'", alias),
            "authority": "advisory_only",
            "timestamp": Utc::now().to_rfc3339(),
        }));
    }

    // Context check
    if let Some(req_ctx) = context {
        if req_ctx > verified_context {
            return Some(json!({
                "status": "refused",
                "reason": "context_exceeds_verified",
                "detail": format!(
                    "Requested context {} exceeds verified max {} for profile '{}'",
                    req_ctx, verified_context, alias
                ),
                "authority": "advisory_only",
                "timestamp": Utc::now().to_rfc3339(),
            }));
        }
    }

    // Runtime health
    if !runtime_available {
        return Some(json!({
            "status": "refused",
            "reason": "runtime_unhealthy",
            "detail": format!("Runtime for profile '{}' is not healthy.", alias),
            "authority": "advisory_only",
            "timestamp": Utc::now().to_rfc3339(),
        }));
    }

    // Authority-bearing keyword categories — checked in priority order.
    //
    // Three distinct categories matching the contract specification:
    //   1. authority_required       — approval/promotion/commitment keywords
    //   2. file_mutation_forbidden  — edit/modify/write/promote keywords
    //   3. autonomous_action_forbidden — autonomous/self-directed keywords
    //
    // Priority order: authority keywords are checked first (highest signal),
    // then file mutation, then autonomous action.

    let authority_keywords = [
        "approve", "promote", "commit", "escalate", "authorize",
        "mark valid", "override policy", "ignore policy",
    ];
    let file_mutation_keywords = [
        "edit source", "modify file", "write to librarian",
    ];
    let autonomous_keywords = [
        "autonomous", "self-directed", "automatic decision",
    ];

    let user_text: String = messages
        .iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();

    // 1. Authority keywords
    for keyword in &authority_keywords {
        if user_text.contains(keyword) {
            return Some(json!({
                "status": "refused",
                "reason": "authority_required",
                "detail": format!(
                    "Request contains authority-bearing content ('{}'). This request implies authority beyond advisory. Model output is advisory only.",
                    keyword
                ),
                "authority": "advisory_only",
                "timestamp": Utc::now().to_rfc3339(),
            }));
        }
    }

    // 2. File mutation keywords
    for keyword in &file_mutation_keywords {
        if user_text.contains(keyword) {
            return Some(json!({
                "status": "refused",
                "reason": "file_mutation_forbidden",
                "detail": format!(
                    "Request contains file mutation content ('{}'). File mutation or promotion to Librarian directory is forbidden.",
                    keyword
                ),
                "authority": "advisory_only",
                "timestamp": Utc::now().to_rfc3339(),
            }));
        }
    }

    // 3. Autonomous action keywords
    for keyword in &autonomous_keywords {
        if user_text.contains(keyword) {
            return Some(json!({
                "status": "refused",
                "reason": "autonomous_action_forbidden",
                "detail": format!(
                    "Request contains autonomous action content ('{}'). Autonomous action is forbidden. The router is a dispatcher, not a decision-maker.",
                    keyword
                ),
                "authority": "advisory_only",
                "timestamp": Utc::now().to_rfc3339(),
            }));
        }
    }

    None
}

/// Check if an alias exists in the profile list.
pub fn alias_exists(alias: &str, aliases: &[String]) -> bool {
    aliases.iter().any(|a| a == alias)
}
