pub mod client;
pub mod email;
pub mod identity;
pub mod mailbox;
pub mod types;
pub mod vacation;

use crate::error::{Error, Result};

/// Surface the first partial-failure `SetError` from a typed `*/set` response, scanning
/// `notCreated → notUpdated → notDestroyed` (the order JMAP documents) and returning the
/// first entry's `description` (or the map key as a fallback) as [`Error::Jmap`]. `Ok(())`
/// when the whole batch succeeded. The per-resource `*SetResponse::check_errors` methods
/// delegate here so the scan lives in one place.
pub fn check_set_errors(
    method: &str,
    not_created: &serde_json::Map<String, serde_json::Value>,
    not_updated: &serde_json::Map<String, serde_json::Value>,
    not_destroyed: &serde_json::Map<String, serde_json::Value>,
) -> Result<()> {
    for (key, map) in [
        ("notCreated", not_created),
        ("notUpdated", not_updated),
        ("notDestroyed", not_destroyed),
    ] {
        if let Some(err) = map.values().next() {
            let desc = err
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or(key);
            return Err(Error::Jmap {
                method: method.to_string(),
                message: desc.to_string(),
            });
        }
    }
    Ok(())
}
