//! L3 presenters: project faithful JMAP `Email` data into the shape the CLI and MCP
//! emit. The read actions return faithful `Email` objects (every property verbatim); the
//! presenters here select the view's fields and resolve body part references into inline
//! strings. Both front-ends share one projection so their output stays identical.
//!
//! Each read view owns two things: the JMAP `properties` an action should request (so the
//! `Email/get` is scoped to what the view needs) and the projection applied to the result
//! before emit/render.

use crate::jmap::email::BodyFetch;

// --- Shared field-selection scaffolding (reused across resources) ---
//
// The generic L3 projection: select a static `&[&str]` of fields from a faithful JMAP
// object or array of objects. Identity, vacation, mailbox, and masked_email all project
// by such a static list; only contact needs a bespoke flatten. This is the L3 home for
// field selection that the data-layer `actions::project_fields*` will retire into once
// every resource has migrated.

/// Project a single JSON object down to `fields`, preserving only those keys (in `fields`
/// order). A non-object value is returned unchanged.
pub fn project_object(value: &serde_json::Value, fields: &[&str]) -> serde_json::Value {
    let Some(map) = value.as_object() else {
        return value.clone();
    };
    let mut result = serde_json::Map::new();
    for &field in fields {
        if let Some(v) = map.get(field) {
            result.insert(field.to_string(), v.clone());
        }
    }
    serde_json::Value::Object(result)
}

/// Project a faithful JMAP value to `fields`: each element of an array (or the value
/// itself, if a single object) is reduced to the selected keys. A non-array, non-object
/// value is returned unchanged.
pub fn project_list(value: &serde_json::Value, fields: &[&str]) -> serde_json::Value {
    match value.as_array() {
        Some(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|item| project_object(item, fields))
                .collect(),
        ),
        None => project_object(value, fields),
    }
}

/// JMAP `Identity` properties for a list row. The faithful `Identity` carries more
/// (`bcc`/`textSignature`/`htmlSignature`/`mayDelete`); the view keeps these.
pub const IDENTITY_LIST_FIELDS: &[&str] = &["id", "name", "email", "replyTo"];

/// Project a faithful `Identity/get` list into the CLI/MCP display shape.
pub fn project_identity_list(value: &serde_json::Value) -> serde_json::Value {
    project_list(value, IDENTITY_LIST_FIELDS)
}

/// JMAP `Email` properties for a list/search row.
pub const EMAIL_LIST_PROPERTIES: &[&str] =
    &["id", "subject", "from", "to", "receivedAt", "preview"];

/// JMAP `Email` properties for a list/search row when bodies are requested. Adds the
/// body part references and the raw `bodyValues` map the projection resolves against.
pub const EMAIL_LIST_BODY_PROPERTIES: &[&str] = &[
    "id",
    "subject",
    "from",
    "to",
    "receivedAt",
    "preview",
    "textBody",
    "htmlBody",
    "bodyValues",
];

/// JMAP `Email` properties for a single-email body view.
pub const EMAIL_BODY_PROPERTIES: &[&str] = &[
    "id",
    "subject",
    "from",
    "to",
    "receivedAt",
    "textBody",
    "htmlBody",
    "bodyValues",
];

/// The `properties` an `Email/get` for a list/search view should request.
pub fn email_list_properties(include_body: bool) -> &'static [&'static str] {
    if include_body {
        EMAIL_LIST_BODY_PROPERTIES
    } else {
        EMAIL_LIST_PROPERTIES
    }
}

/// Body-value fetch flags for a list/search view. List bodies resolve text and HTML.
pub fn email_list_body_fetch(include_body: bool) -> BodyFetch {
    BodyFetch {
        text: include_body,
        html: include_body,
        all: false,
    }
}

/// Project a list/search result (faithful array of `Email` objects) into display shape.
/// Each element is reshaped in place: body part references resolve to strings, a `date`
/// field is synthesized, and the raw `bodyValues` map is dropped.
pub fn project_email_list(value: &mut serde_json::Value) {
    if let Some(emails) = value.as_array_mut() {
        for email in emails.iter_mut() {
            extract_body_content(email);
        }
    }
}

/// Project a single faithful `Email` object into body-view display shape.
pub fn project_email_body(value: &mut serde_json::Value) {
    extract_body_content(value);
}

/// Transform `textBody`/`htmlBody` from arrays of JMAP part references into their actual
/// content strings, add a `date` field from `receivedAt`, and drop the raw `bodyValues`.
fn extract_body_content(email: &mut serde_json::Value) {
    let Some(obj) = email.as_object_mut() else {
        return;
    };

    if let Some(received) = obj.get("receivedAt").cloned() {
        obj.insert("date".to_string(), received);
    }

    let body_values = obj.get("bodyValues").cloned();
    for key in ["textBody", "htmlBody"] {
        if let Some(content) = resolve_body_part(obj.get(key), body_values.as_ref()) {
            obj.insert(key.to_string(), content);
        }
    }

    // Raw bodyValues is an implementation detail consumers don't need.
    obj.remove("bodyValues");
}

/// Resolve a `textBody`/`htmlBody` part-reference array to its content string by looking
/// up the first part's `partId` in the `bodyValues` map.
fn resolve_body_part(
    body: Option<&serde_json::Value>,
    body_values: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let part_id = body?
        .as_array()
        .and_then(|parts| parts.first())
        .and_then(|first| first.get("partId"))
        .and_then(|p| p.as_str())?;

    body_values?
        .get(part_id)
        .and_then(|v| v.get("value"))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn project_email_body_resolves_parts_and_synthesizes_date() {
        let mut email = json!({
            "id": "e001",
            "subject": "Test",
            "from": [{"email": "a@b.com"}],
            "to": [{"email": "c@d.com"}],
            "receivedAt": "2026-01-01T00:00:00Z",
            "textBody": [{"partId": "p1"}],
            "htmlBody": [{"partId": "p2"}],
            "bodyValues": {
                "p1": {"value": "plain text body"},
                "p2": {"value": "<p>html body</p>"}
            }
        });

        project_email_body(&mut email);

        assert_eq!(email["textBody"], "plain text body");
        assert_eq!(email["htmlBody"], "<p>html body</p>");
        assert_eq!(email["date"], "2026-01-01T00:00:00Z");
        assert!(
            email.get("bodyValues").is_none(),
            "bodyValues should be removed"
        );
    }

    #[test]
    fn project_email_list_projects_each_element() {
        let mut list = json!([
            {
                "id": "e001",
                "receivedAt": "2026-01-01T00:00:00Z",
                "textBody": [{"partId": "p1"}],
                "bodyValues": { "p1": {"value": "body one"} }
            },
            {
                "id": "e002",
                "receivedAt": "2026-01-02T00:00:00Z",
                "textBody": [{"partId": "p1"}],
                "bodyValues": { "p1": {"value": "body two"} }
            }
        ]);

        project_email_list(&mut list);

        let arr = list.as_array().expect("array");
        assert_eq!(arr[0]["textBody"], "body one");
        assert_eq!(arr[0]["date"], "2026-01-01T00:00:00Z");
        assert!(arr[0].get("bodyValues").is_none());
        assert_eq!(arr[1]["textBody"], "body two");
    }

    #[test]
    fn project_handles_missing_body_gracefully() {
        let mut email = json!({ "id": "e001", "subject": "no body" });
        project_email_body(&mut email);
        assert_eq!(email["id"], "e001");
        assert!(email.get("date").is_none());
    }

    #[test]
    fn list_properties_switch_on_body() {
        assert_eq!(email_list_properties(false), EMAIL_LIST_PROPERTIES);
        assert_eq!(email_list_properties(true), EMAIL_LIST_BODY_PROPERTIES);
        assert!(!email_list_body_fetch(false).text);
        assert!(email_list_body_fetch(true).text && email_list_body_fetch(true).html);
    }

    #[test]
    fn project_list_selects_fields_per_element() {
        let faithful = json!([
            { "id": "id1", "name": "Alice", "email": "a@b.com", "textSignature": "sig" },
            { "id": "id2", "name": "Bob", "email": "c@d.com", "mayDelete": false }
        ]);
        let projected = project_list(&faithful, &["id", "name"]);
        assert_eq!(
            projected,
            json!([{ "id": "id1", "name": "Alice" }, { "id": "id2", "name": "Bob" }])
        );
    }

    #[test]
    fn project_list_handles_single_object_and_passthrough() {
        let obj = json!({ "id": "x", "extra": 1 });
        assert_eq!(project_list(&obj, &["id"]), json!({ "id": "x" }));
        // A non-array, non-object value passes through unchanged.
        assert_eq!(project_list(&json!("hi"), &["id"]), json!("hi"));
    }

    #[test]
    fn project_identity_list_drops_extras() {
        let faithful = json!([{
            "id": "id1",
            "name": "Alice",
            "email": "alice@example.com",
            "replyTo": null,
            "bcc": null,
            "textSignature": "sig1",
            "htmlSignature": "<p>sig1</p>",
            "mayDelete": true
        }]);
        let projected = project_identity_list(&faithful);
        assert_eq!(
            projected,
            json!([{
                "id": "id1",
                "name": "Alice",
                "email": "alice@example.com",
                "replyTo": null
            }])
        );
    }
}
