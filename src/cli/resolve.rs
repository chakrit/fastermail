use crate::actions::Context;
use crate::cli::io::{Io, OutputMode};
use crate::error::{Error, Result};
use crate::json;

/// Built-in role aliases that map to JMAP mailbox role values.
const ROLE_ALIASES: &[(&str, &str)] = &[
    ("inbox", "inbox"),
    ("sent", "sent"),
    ("drafts", "drafts"),
    ("trash", "trash"),
    ("junk", "junk"),
    ("spam", "junk"),
    ("archive", "archive"),
];

/// Resolve a user-provided mailbox string to a JMAP mailbox ID.
///
/// Resolution order:
/// 0. Exact JMAP id match (the stable, unambiguous handle — reaches duplicate-named mailboxes)
/// 1. Role alias lookup (e.g. "inbox" → find mailbox with role "inbox")
/// 2. Exact name match (case-insensitive)
/// 3. Prefix match (case-insensitive)
/// 4. Substring match (case-insensitive)
///
/// If multiple matches are found:
/// - Human mode: interactive disambiguation via `inquire::Select`
/// - Json/Raw mode: error with candidate list
pub fn resolve_mailbox(input: &str, ctx: &Context, io: &Io) -> Result<String> {
    if input.is_empty() {
        return Err(Error::InvalidParams("mailbox is required".to_string()));
    }

    // Fetch all mailboxes once via the faithful L1 accessor. Resolution reads `id`,
    // `role`, and `name` — all present in the faithful `Mailbox`, so no projection is
    // needed here. (Dedup against `actions::find_mailbox_id_by_*` is deferred to step 3.)
    let spinner = io.progress("Resolving mailbox…");
    let response = ctx.jmap.mailbox_get(&ctx.account_id);
    Io::finish_progress(spinner);
    let mailboxes: Vec<serde_json::Value> = response?
        .list
        .into_iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<_, _>>()?;
    let mailboxes = mailboxes.as_slice();

    // Step 0: Exact JMAP id match. The stable, unambiguous handle (the `--mailbox` help
    // advertises "ID"); wins over role/name so duplicate-named mailboxes stay reachable.
    if let Some(id) = find_by_id(mailboxes, input) {
        return Ok(id);
    }

    // Step 1: Check if input is a role alias
    let input_lower = input.to_lowercase();
    // Role alias that resolves to a mailbox wins; otherwise fall through to name matching.
    if let Some(role) = role_for_alias(&input_lower)
        && let Some(id) = find_by_role(mailboxes, role)
    {
        return Ok(id);
    }

    // Step 2-4: Name matching (exact → prefix → substring)
    let candidates = match_by_name(mailboxes, &input_lower);

    match candidates.len() {
        0 => Err(Error::InvalidParams(format!(
            "no mailbox matching: {input}"
        ))),
        1 => Ok(candidates[0].0.clone()),
        _ => disambiguate(&candidates, input, io),
    }
}

/// Look up the JMAP role for a built-in alias. Returns None if not a known alias.
fn role_for_alias(input: &str) -> Option<&'static str> {
    ROLE_ALIASES
        .iter()
        .find(|(alias, _)| *alias == input)
        .map(|(_, role)| *role)
}

/// Find a mailbox by exact JMAP id (case-sensitive). Returns None if no id matches.
fn find_by_id(mailboxes: &[serde_json::Value], id: &str) -> Option<String> {
    mailboxes.iter().find_map(|m| {
        let m_id = json::str_at(m, "/id")?;
        (m_id == id).then(|| m_id.to_string())
    })
}

/// Find a mailbox by its JMAP role field.
fn find_by_role(mailboxes: &[serde_json::Value], role: &str) -> Option<String> {
    mailboxes.iter().find_map(|m| {
        let m_role = json::str_at(m, "/role").unwrap_or("");
        if m_role == role {
            json::str_at(m, "/id").map(String::from)
        } else {
            None
        }
    })
}

/// Match mailboxes by name: exact, then prefix, then substring (case-insensitive).
/// Returns (id, name) pairs. Stops at the first tier that produces results.
fn match_by_name(mailboxes: &[serde_json::Value], input: &str) -> Vec<(String, String)> {
    let entries: Vec<(String, String)> = mailboxes
        .iter()
        .filter_map(|m| {
            let id = json::str_at(m, "/id")?;
            let name = json::str_at(m, "/name")?;
            Some((id.to_string(), name.to_string()))
        })
        .collect();

    // Exact match (case-insensitive)
    let exact: Vec<(String, String)> = entries
        .iter()
        .filter(|(_, name)| name.to_lowercase() == input)
        .cloned()
        .collect();
    if !exact.is_empty() {
        return exact;
    }

    // Prefix match
    let prefix: Vec<(String, String)> = entries
        .iter()
        .filter(|(_, name)| name.to_lowercase().starts_with(input))
        .cloned()
        .collect();
    if !prefix.is_empty() {
        return prefix;
    }

    // Substring match
    entries
        .iter()
        .filter(|(_, name)| name.to_lowercase().contains(input))
        .cloned()
        .collect()
}

/// Disambiguate multiple mailbox matches.
/// Human mode: interactive selection. Non-interactive: error with candidates.
fn disambiguate(candidates: &[(String, String)], input: &str, io: &Io) -> Result<String> {
    if io.mode() == OutputMode::Human {
        let options: Vec<String> = candidates
            .iter()
            .map(|(id, name)| format!("{name}  ({id})"))
            .collect();

        let selection =
            inquire::Select::new(&format!("Multiple mailboxes match \"{input}\":"), options)
                .prompt()
                .map_err(|e| Error::InvalidParams(format!("selection cancelled: {e}")))?;

        // Extract ID from the selection string "Name  (id)"
        let id = candidates
            .iter()
            .find(|(id, name)| format!("{name}  ({id})") == selection)
            .map(|(id, _)| id.clone())
            .ok_or_else(|| Error::InvalidParams("invalid selection".to_string()))?;

        Ok(id)
    } else {
        let names: Vec<&str> = candidates.iter().map(|(_, name)| name.as_str()).collect();
        Err(Error::InvalidParams(format!(
            "ambiguous mailbox \"{input}\", candidates: {}",
            names.join(", ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_for_alias() {
        assert_eq!(role_for_alias("inbox"), Some("inbox"));
        assert_eq!(role_for_alias("spam"), Some("junk"));
        assert_eq!(role_for_alias("archive"), Some("archive"));
        assert_eq!(role_for_alias("projects"), None);
    }

    #[test]
    fn test_find_by_role() {
        let mailboxes = vec![
            serde_json::json!({"id": "mb-1", "name": "Inbox", "role": "inbox"}),
            serde_json::json!({"id": "mb-2", "name": "Sent", "role": "sent"}),
        ];
        assert_eq!(find_by_role(&mailboxes, "inbox"), Some("mb-1".to_string()));
        assert_eq!(find_by_role(&mailboxes, "drafts"), None);
    }

    #[test]
    fn test_find_by_id() {
        // Two mailboxes share the name "Crypto"; only their ids tell them apart.
        let mailboxes = vec![
            serde_json::json!({"id": "P8k", "name": "Crypto", "role": null}),
            serde_json::json!({"id": "P4k", "name": "Crypto", "role": null}),
        ];
        assert_eq!(find_by_id(&mailboxes, "P8k"), Some("P8k".to_string()));
        assert_eq!(find_by_id(&mailboxes, "P4k"), Some("P4k".to_string()));
        assert_eq!(find_by_id(&mailboxes, "P8K"), None); // ids are case-sensitive
        assert_eq!(find_by_id(&mailboxes, "nope"), None);
    }

    #[test]
    fn test_match_by_name_exact() {
        let mailboxes = vec![
            serde_json::json!({"id": "mb-1", "name": "Projects"}),
            serde_json::json!({"id": "mb-2", "name": "Personal"}),
        ];
        let result = match_by_name(&mailboxes, "projects");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "mb-1");
    }

    #[test]
    fn test_match_by_name_prefix() {
        let mailboxes = vec![
            serde_json::json!({"id": "mb-1", "name": "Projects"}),
            serde_json::json!({"id": "mb-2", "name": "Personal"}),
        ];
        let result = match_by_name(&mailboxes, "proj");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "mb-1");
    }

    #[test]
    fn test_match_by_name_substring() {
        let mailboxes = vec![
            serde_json::json!({"id": "mb-1", "name": "Projects"}),
            serde_json::json!({"id": "mb-2", "name": "Personal"}),
        ];
        let result = match_by_name(&mailboxes, "ject");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "mb-1");
    }

    #[test]
    fn test_match_by_name_multiple_prefix() {
        let mailboxes = vec![
            serde_json::json!({"id": "mb-1", "name": "Projects"}),
            serde_json::json!({"id": "mb-2", "name": "Promotions"}),
        ];
        let result = match_by_name(&mailboxes, "pro");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_match_by_name_no_match() {
        let mailboxes = vec![serde_json::json!({"id": "mb-1", "name": "Projects"})];
        let result = match_by_name(&mailboxes, "xyz");
        assert!(result.is_empty());
    }

    #[test]
    fn test_exact_beats_prefix() {
        // "In" should match "In" exactly, not prefix-match "Inbox"
        let mailboxes = vec![
            serde_json::json!({"id": "mb-1", "name": "Inbox"}),
            serde_json::json!({"id": "mb-2", "name": "In"}),
        ];
        let result = match_by_name(&mailboxes, "in");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "mb-2");
    }
}
