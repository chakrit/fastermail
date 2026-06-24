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

/// JMAP `VacationResponse` properties for the get view. The faithful `VacationResponse`
/// carries the typed `id` too; the view drops it (the singleton id `"singleton"` is
/// implicit).
pub const VACATION_FIELDS: &[&str] = &[
    "isEnabled",
    "fromDate",
    "toDate",
    "subject",
    "textBody",
    "htmlBody",
];

/// Project a faithful `VacationResponse` singleton into the CLI/MCP display shape.
pub fn project_vacation(value: &serde_json::Value) -> serde_json::Value {
    project_object(value, VACATION_FIELDS)
}

/// JMAP `Mailbox` properties for a list row. The faithful `Mailbox` carries more
/// (`sortOrder`/`totalThreads`/`unreadThreads`/`myRights`/`isSubscribed`); the view drops
/// these.
pub const MAILBOX_LIST_FIELDS: &[&str] = &[
    "id",
    "name",
    "role",
    "totalEmails",
    "unreadEmails",
    "parentId",
];

/// Project a faithful `Mailbox/get` list into the CLI/MCP display shape, optionally
/// filtered by JMAP `role` first. An empty `role` keeps every mailbox. The role filter is
/// an L3 view concern (the data layer returns the faithful, unfiltered list).
pub fn project_mailbox_list(value: &serde_json::Value, role: &str) -> serde_json::Value {
    if role.is_empty() {
        return project_list(value, MAILBOX_LIST_FIELDS);
    }
    let filtered: Vec<serde_json::Value> = value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|m| crate::json::str_at(m, "/role") == Some(role))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    project_list(&serde_json::Value::Array(filtered), MAILBOX_LIST_FIELDS)
}

/// FastMail `MaskedEmail` properties for a list row. The faithful `MaskedEmail` carries
/// more (`lastMessageAt`, `url`, and any field FastMail adds later); the view drops these.
pub const MASKED_EMAIL_LIST_FIELDS: &[&str] = &[
    "id",
    "email",
    "forDomain",
    "description",
    "state",
    "createdAt",
];

/// FastMail `MaskedEmail` properties for the create view — deliberately ASYMMETRIC vs the
/// list: only `id`/`email` survive (the server returns the full object on create, but the
/// create view drops `state`/`forDomain`/`description`/`createdAt`).
pub const MASKED_EMAIL_CREATE_FIELDS: &[&str] = &["id", "email"];

/// Lifecycle state of a masked email — an L3 input-parsing concern (the CLI/MCP validate
/// the state argument before it reaches the data layer, like [`FieldChange`] for vacation).
/// FastMail expresses the state directly as the `state` field; this enum just constrains
/// and labels the accepted values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskedEmailState {
    Pending,
    Enabled,
    Disabled,
    Deleted,
}

impl MaskedEmailState {
    /// Parse any lifecycle state — used for list filtering.
    pub fn parse(s: &str) -> crate::error::Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            "deleted" => Ok(Self::Deleted),
            _ => Err(crate::error::Error::InvalidParams(
                "state must be pending, enabled, disabled, or deleted".to_string(),
            )),
        }
    }

    /// Parse a settable state — used for updates. `pending` is auto-assigned by the
    /// server and cannot be set, so it is rejected here.
    pub fn parse_settable(s: &str) -> crate::error::Result<Self> {
        match s {
            "" => Err(crate::error::Error::InvalidParams(
                "state is required".to_string(),
            )),
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            "deleted" => Ok(Self::Deleted),
            _ => Err(crate::error::Error::InvalidParams(
                "state must be enabled, disabled, or deleted".to_string(),
            )),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Deleted => "deleted",
        }
    }
}

/// Project a faithful `MaskedEmail/get` list into the CLI/MCP display shape, optionally
/// filtered by lifecycle `state` first. `None` keeps every masked email. The state filter
/// is an L3 view concern (the data layer returns the faithful, unfiltered list).
pub fn project_masked_email_list(
    value: &serde_json::Value,
    state: Option<MaskedEmailState>,
) -> serde_json::Value {
    let Some(state) = state else {
        return project_list(value, MASKED_EMAIL_LIST_FIELDS);
    };
    let filtered: Vec<serde_json::Value> = value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|m| crate::json::str_at(m, "/state") == Some(state.label()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    project_list(
        &serde_json::Value::Array(filtered),
        MASKED_EMAIL_LIST_FIELDS,
    )
}

/// Project the created object out of a faithful `MaskedEmail/set` response into the create
/// view's `{id, email}` shape. Digs the (single) `created` entry — the front-ends never see
/// the creation-id key — and projects it; a response with no created object yields `{}`,
/// matching the prior action behaviour.
pub fn project_masked_email_create(set_response: &serde_json::Value) -> serde_json::Value {
    let created = set_response
        .get("created")
        .and_then(|c| c.as_object())
        .and_then(|map| map.values().next())
        .cloned()
        .unwrap_or(serde_json::json!({}));
    project_object(&created, MASKED_EMAIL_CREATE_FIELDS)
}

/// The `{success: true}` MCP-wrapper shape — an L3 concern, emitted by the front-ends
/// after a typed `*_set` accessor returns, not from the action itself.
pub fn set_ok() -> serde_json::Value {
    serde_json::json!({ "success": true })
}

/// The `{success: true, <id_field>: id}` MCP-wrapper shape — an L3 concern, emitted by the
/// front-ends after a typed `*_set` accessor returns. `id` is the affected object's id (a
/// created id, or the renamed/deleted id). A `None` id (e.g. a create response missing the
/// created object) writes JSON null, matching the prior action behaviour.
pub fn set_with_id(id_field: &str, id: Option<&str>) -> serde_json::Value {
    serde_json::json!({ "success": true, id_field: id })
}

/// A field-level change in a vacation update — an L3 input-parsing concern (JMAP has no
/// such noun). Optional CLI/MCP arguments carry three intents that JMAP expresses in its
/// `update` patch: omit the key (leave unchanged), write `null` (clear), or write a value
/// (set). [`build_vacation_update`] turns these into the verbatim JMAP patch the L1
/// `vacation_set` accessor sends.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum FieldChange {
    /// Field not provided — leave it unchanged (the key is omitted from the patch).
    #[default]
    Leave,
    /// Provided empty/null — clear it (the patch writes JSON null).
    Clear,
    /// Provided a value — set it.
    Set(String),
}

impl FieldChange {
    /// From an MCP argument: absent -> Leave; null or empty string -> Clear; else Set.
    pub fn from_arg(args: &serde_json::Value, key: &str) -> Self {
        match args.get(key) {
            None => Self::Leave,
            Some(v) if v.is_null() => Self::Clear,
            Some(v) => Self::from_opt(Some(v.as_str().unwrap_or("").to_string())),
        }
    }

    /// From a CLI optional argument: None -> Leave; empty -> Clear; else Set.
    pub fn from_opt(value: Option<String>) -> Self {
        match value {
            None => Self::Leave,
            Some(s) if s.is_empty() => Self::Clear,
            Some(s) => Self::Set(s),
        }
    }

    /// The JSON to write into the update patch, or None to omit the field.
    fn patch_value(&self) -> Option<serde_json::Value> {
        match self {
            Self::Leave => None,
            Self::Clear => Some(serde_json::Value::Null),
            Self::Set(s) => Some(serde_json::json!(s)),
        }
    }
}

/// Build the verbatim JMAP `VacationResponse/set` patch from the parsed inputs. `isEnabled`
/// is always written; each optional field is included per its [`FieldChange`]. The
/// resulting object is what the L1 `vacation_set` accessor sends under `"singleton"`.
pub fn build_vacation_update(
    is_enabled: bool,
    from_date: &FieldChange,
    to_date: &FieldChange,
    subject: &FieldChange,
    text_body: &FieldChange,
    html_body: &FieldChange,
) -> serde_json::Value {
    let mut update = serde_json::json!({ "isEnabled": is_enabled });
    let fields = [
        ("fromDate", from_date),
        ("toDate", to_date),
        ("subject", subject),
        ("textBody", text_body),
        ("htmlBody", html_body),
    ];
    for (key, change) in fields {
        if let Some(value) = change.patch_value() {
            update[key] = value;
        }
    }
    update
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
    fn project_vacation_drops_id_and_extras() {
        let faithful = json!({
            "id": "singleton",
            "isEnabled": true,
            "fromDate": "2026-01-01T00:00:00Z",
            "toDate": "2026-01-15T00:00:00Z",
            "subject": "OOO",
            "textBody": "Away",
            "htmlBody": "<p>Away</p>"
        });
        let projected = project_vacation(&faithful);
        assert_eq!(
            projected,
            json!({
                "isEnabled": true,
                "fromDate": "2026-01-01T00:00:00Z",
                "toDate": "2026-01-15T00:00:00Z",
                "subject": "OOO",
                "textBody": "Away",
                "htmlBody": "<p>Away</p>"
            })
        );
    }

    #[test]
    fn project_mailbox_list_drops_extras_and_keeps_all_without_role() {
        let faithful = json!([
            {
                "id": "mb1", "name": "Inbox", "role": "inbox",
                "totalEmails": 42, "unreadEmails": 3, "parentId": null,
                "sortOrder": 1, "totalThreads": 40, "unreadThreads": 2,
                "myRights": {"mayRead": true}, "isSubscribed": true
            },
            {
                "id": "mb2", "name": "Sent", "role": "sent",
                "totalEmails": 10, "unreadEmails": 0, "parentId": null,
                "sortOrder": 2, "totalThreads": 9, "unreadThreads": 0,
                "myRights": {"mayRead": true}, "isSubscribed": true
            }
        ]);
        let projected = project_mailbox_list(&faithful, "");
        assert_eq!(
            projected,
            json!([
                {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 42, "unreadEmails": 3, "parentId": null},
                {"id": "mb2", "name": "Sent", "role": "sent", "totalEmails": 10, "unreadEmails": 0, "parentId": null}
            ])
        );
    }

    #[test]
    fn project_mailbox_list_filters_by_role() {
        let faithful = json!([
            {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 42, "unreadEmails": 3, "parentId": null, "sortOrder": 1},
            {"id": "mb2", "name": "Sent", "role": "sent", "totalEmails": 10, "unreadEmails": 0, "parentId": null}
        ]);
        assert_eq!(
            project_mailbox_list(&faithful, "inbox"),
            json!([{"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 42, "unreadEmails": 3, "parentId": null}])
        );
        // A role with no matching mailbox projects to an empty array.
        assert_eq!(project_mailbox_list(&faithful, "archive"), json!([]));
    }

    #[test]
    fn set_with_id_wraps_id() {
        assert_eq!(
            set_with_id("mailboxId", Some("mb1")),
            json!({ "success": true, "mailboxId": "mb1" })
        );
        // A missing id writes null, matching the prior create-without-created behaviour.
        assert_eq!(
            set_with_id("mailboxId", None),
            json!({ "success": true, "mailboxId": null })
        );
    }

    #[test]
    fn project_masked_email_list_drops_extras() {
        let faithful = json!([{
            "id": "me1",
            "email": "abc@fastmail.com",
            "forDomain": "example.com",
            "description": "Test",
            "state": "enabled",
            "createdAt": "2026-01-01",
            "lastMessageAt": "2026-03-01",
            "url": "https://example.com"
        }]);
        let projected = project_masked_email_list(&faithful, None);
        assert_eq!(
            projected,
            json!([{
                "id": "me1",
                "email": "abc@fastmail.com",
                "forDomain": "example.com",
                "description": "Test",
                "state": "enabled",
                "createdAt": "2026-01-01"
            }])
        );
    }

    #[test]
    fn project_masked_email_list_filters_by_state() {
        let faithful = json!([
            {"id": "me1", "email": "a@fastmail.com", "forDomain": "x.com", "description": "", "state": "enabled", "createdAt": "2026-01-01"},
            {"id": "me2", "email": "b@fastmail.com", "forDomain": "y.com", "description": "", "state": "disabled", "createdAt": "2026-02-01"}
        ]);
        assert_eq!(
            project_masked_email_list(&faithful, Some(MaskedEmailState::Enabled)),
            json!([{"id": "me1", "email": "a@fastmail.com", "forDomain": "x.com", "description": "", "state": "enabled", "createdAt": "2026-01-01"}])
        );
        // A state with no matching entry projects to an empty array.
        assert_eq!(
            project_masked_email_list(&faithful, Some(MaskedEmailState::Deleted)),
            json!([])
        );
    }

    #[test]
    fn masked_email_state_parse_validates() {
        assert_eq!(
            MaskedEmailState::parse("enabled").expect("valid"),
            MaskedEmailState::Enabled
        );
        assert!(MaskedEmailState::parse("bogus").is_err());
        // `pending` parses for filtering but is rejected as a settable update.
        assert_eq!(
            MaskedEmailState::parse("pending").expect("valid"),
            MaskedEmailState::Pending
        );
        assert!(MaskedEmailState::parse_settable("pending").is_err());
        assert!(MaskedEmailState::parse_settable("").is_err());
        assert!(MaskedEmailState::parse_settable("bogus").is_err());
        assert_eq!(MaskedEmailState::Disabled.label(), "disabled");
    }

    #[test]
    fn project_masked_email_create_keeps_id_and_email_only() {
        // The faithful set response carries the full created object; the create view is
        // asymmetric — only id/email survive.
        let set_response = json!({
            "created": {"new-masked": {
                "id": "me-new",
                "email": "new@fastmail.com",
                "state": "enabled",
                "forDomain": "mysite.com",
                "description": "My site login",
                "createdAt": "2026-06-25"
            }}
        });
        assert_eq!(
            project_masked_email_create(&set_response),
            json!({ "id": "me-new", "email": "new@fastmail.com" })
        );
    }

    #[test]
    fn project_masked_email_create_handles_missing_created() {
        // No created object → empty object, matching the prior action behaviour.
        assert_eq!(
            project_masked_email_create(&json!({ "created": {} })),
            json!({})
        );
        assert_eq!(project_masked_email_create(&json!({})), json!({}));
    }

    #[test]
    fn field_change_leaves_absent() {
        let args = json!({"other": "value"});
        assert_eq!(FieldChange::from_arg(&args, "fromDate"), FieldChange::Leave);
    }

    #[test]
    fn field_change_clears_on_null() {
        let args = json!({"fromDate": null});
        assert_eq!(FieldChange::from_arg(&args, "fromDate"), FieldChange::Clear);
    }

    #[test]
    fn field_change_clears_on_empty_string() {
        let args = json!({"subject": ""});
        assert_eq!(FieldChange::from_arg(&args, "subject"), FieldChange::Clear);
    }

    #[test]
    fn field_change_sets_non_empty() {
        let args = json!({"subject": "hello"});
        assert_eq!(
            FieldChange::from_arg(&args, "subject"),
            FieldChange::Set("hello".to_string())
        );
    }

    #[test]
    fn field_change_from_opt() {
        assert_eq!(FieldChange::from_opt(None), FieldChange::Leave);
        assert_eq!(
            FieldChange::from_opt(Some(String::new())),
            FieldChange::Clear
        );
        assert_eq!(
            FieldChange::from_opt(Some("x".to_string())),
            FieldChange::Set("x".to_string())
        );
    }

    #[test]
    fn build_vacation_update_omits_clears_and_sets() {
        let update = build_vacation_update(
            true,
            &FieldChange::Leave,
            &FieldChange::Clear,
            &FieldChange::Set("On vacation".to_string()),
            &FieldChange::Leave,
            &FieldChange::Leave,
        );
        // isEnabled always present; Leave omits the key; Clear writes null; Set writes value.
        assert_eq!(update["isEnabled"], json!(true));
        assert!(update.get("fromDate").is_none(), "Leave omits the key");
        assert_eq!(update["toDate"], json!(null), "Clear writes null");
        assert_eq!(update["subject"], json!("On vacation"));
        assert!(update.get("textBody").is_none());
        assert!(update.get("htmlBody").is_none());
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
