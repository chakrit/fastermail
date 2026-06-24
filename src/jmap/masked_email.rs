use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::jmap::client::JmapClient;

/// FastMail's non-urn capability string for the masked-email extension.
const MASKED_EMAIL_CAPABILITY: &str = "https://www.fastmail.com/dev/maskedemail";

/// A JMAP `MaskedEmail` id. Newtype so it can't be confused with other id strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaskedEmailId(pub String);

impl MaskedEmailId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MaskedEmailId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Faithful mirror of a FastMail `MaskedEmail` object: a zero-loss read shape. The id is
/// typed; every other property the server returns — `email`, `state`, `forDomain`,
/// `description`, `createdAt`, `lastMessageAt`, `url`, and any field FastMail adds later —
/// is preserved verbatim in `rest`. JMAP names 1:1. Presenters (CLI/MCP) decide what to
/// project (the create view is asymmetric: only `id`/`email`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskedEmail {
    pub id: MaskedEmailId,
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

/// Faithful mirror of a `MaskedEmail/get` response. With no `ids`, the server returns every
/// masked email in the account.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MaskedEmailGetResponse {
    pub list: Vec<MaskedEmail>,
    pub not_found: Vec<MaskedEmailId>,
    pub state: String,
}

/// Faithful mirror of a `MaskedEmail/set` response. `created`/`updated` map each id to the
/// server-completed object (or null); `destroyed` lists the removed ids; each `not_*` map
/// carries a JMAP `SetError` per id that failed. No projection — every field is preserved
/// verbatim. `Serialize` so the action can return it faithfully (the front-ends then dig
/// the created object and apply the L3 projection / `{success}` wrapper).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MaskedEmailSetResponse {
    pub old_state: Option<String>,
    pub new_state: Option<String>,
    pub created: serde_json::Map<String, serde_json::Value>,
    pub updated: serde_json::Map<String, serde_json::Value>,
    pub destroyed: Vec<MaskedEmailId>,
    pub not_created: serde_json::Map<String, serde_json::Value>,
    pub not_updated: serde_json::Map<String, serde_json::Value>,
    pub not_destroyed: serde_json::Map<String, serde_json::Value>,
}

impl MaskedEmailSetResponse {
    /// Surface the first partial-failure `SetError` (notCreated → notUpdated →
    /// notDestroyed, the order JMAP documents). `Ok(())` when the whole batch succeeded.
    pub fn check_errors(&self, method: &str) -> Result<()> {
        crate::jmap::check_set_errors(
            method,
            &self.not_created,
            &self.not_updated,
            &self.not_destroyed,
        )
    }
}

impl JmapClient {
    /// L1 accessor: a faithful `MaskedEmail/get` — no projection. With no `ids`, the server
    /// returns every masked email in the account; every property is preserved (typed id
    /// plus [`MaskedEmail::rest`]).
    pub fn masked_email_get(&self, account_id: &str) -> Result<MaskedEmailGetResponse> {
        let data = self.call_one(
            MASKED_EMAIL_CAPABILITY,
            "MaskedEmail/get",
            serde_json::json!({ "accountId": account_id }),
        )?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }

    /// L1 accessor: a faithful single `MaskedEmail/set` — create, update, and/or destroy in
    /// one call, no projection. `create`/`update` are JMAP object maps (creation-id →
    /// object, id → patch); `destroy` lists the ids to remove. The caller builds the JMAP
    /// payloads verbatim; typed builders are sugar layered on later. Mirrors
    /// [`JmapClient::email_set`].
    pub fn masked_email_set(
        &self,
        account_id: &str,
        create: Option<serde_json::Value>,
        update: Option<serde_json::Value>,
        destroy: &[MaskedEmailId],
    ) -> Result<MaskedEmailSetResponse> {
        let mut args = serde_json::json!({ "accountId": account_id });
        if let Some(create) = create {
            args["create"] = create;
        }
        if let Some(update) = update {
            args["update"] = update;
        }
        if !destroy.is_empty() {
            let ids: Vec<&str> = destroy.iter().map(MaskedEmailId::as_str).collect();
            args["destroy"] = serde_json::json!(ids);
        }

        let data = self.call_one(MASKED_EMAIL_CAPABILITY, "MaskedEmail/set", args)?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    fn client(mock: &MockJmap) -> JmapClient {
        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session connect");
        client
    }

    #[test]
    fn masked_email_get_returns_faithful_object_with_flattened_extras() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/get",
            json!({
                "methodResponses": [["MaskedEmail/get", {
                    "state": "s1",
                    "list": [{
                        "id": "me1",
                        "email": "abc@fastmail.com",
                        "forDomain": "example.com",
                        "description": "Test",
                        "state": "enabled",
                        "createdAt": "2026-01-01",
                        "lastMessageAt": "2026-03-01",
                        "url": "https://example.com"
                    }],
                    "notFound": []
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .masked_email_get(TEST_ACCOUNT_ID)
            .expect("masked_email_get");

        assert_eq!(resp.list.len(), 1);
        let masked = &resp.list[0];
        assert_eq!(masked.id, MaskedEmailId("me1".into()));
        // Every non-id field survives verbatim in `rest` — nothing is dropped, including
        // fields the L3 list presenter keeps (email/forDomain/description/state/createdAt)
        // and ones it projects out (lastMessageAt/url).
        assert_eq!(masked.rest.get("email"), Some(&json!("abc@fastmail.com")));
        assert_eq!(masked.rest.get("forDomain"), Some(&json!("example.com")));
        assert_eq!(masked.rest.get("description"), Some(&json!("Test")));
        assert_eq!(masked.rest.get("state"), Some(&json!("enabled")));
        assert_eq!(masked.rest.get("createdAt"), Some(&json!("2026-01-01")));
        assert_eq!(masked.rest.get("lastMessageAt"), Some(&json!("2026-03-01")));
        assert_eq!(masked.rest.get("url"), Some(&json!("https://example.com")));
        // The typed id is consumed by its field, not duplicated into `rest`.
        assert!(!masked.rest.contains_key("id"));
        assert_eq!(resp.state, "s1");
    }

    #[test]
    fn masked_email_get_returns_empty_for_no_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/get",
            json!({"methodResponses": [["MaskedEmail/get", {"list": []}, "call-0"]]}),
        );

        let resp = client(&mock)
            .masked_email_get(TEST_ACCOUNT_ID)
            .expect("masked_email_get");
        assert!(resp.list.is_empty());
    }

    #[test]
    fn masked_email_set_creates_faithful_object() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {
                    "created": {"new-masked": {
                        "id": "me-new",
                        "email": "new@fastmail.com",
                        "state": "enabled",
                        "forDomain": "mysite.com"
                    }}
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .masked_email_set(
                TEST_ACCOUNT_ID,
                Some(json!({ "new-masked": { "state": "enabled" } })),
                None,
                &[],
            )
            .expect("masked_email_set");
        // The created object is faithful — every server field survives (the L3 create
        // presenter projects it down to {id,email}).
        assert_eq!(resp.created["new-masked"]["id"], "me-new");
        assert_eq!(resp.created["new-masked"]["forDomain"], "mysite.com");
        assert_eq!(resp.created["new-masked"]["state"], "enabled");
        resp.check_errors("MaskedEmail/set").expect("no set errors");
    }

    #[test]
    fn masked_email_set_updates() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {"updated": {"me1": null}}, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .masked_email_set(
                TEST_ACCOUNT_ID,
                None,
                Some(json!({ "me1": { "state": "disabled" } })),
                &[],
            )
            .expect("masked_email_set");
        assert!(resp.updated.contains_key("me1"));
        resp.check_errors("MaskedEmail/set").expect("no set errors");
    }

    #[test]
    fn masked_email_set_surfaces_not_created_set_errors() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {
                    "created": {},
                    "notCreated": { "new-masked": { "type": "invalidProperties", "description": "bad" } }
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .masked_email_set(
                TEST_ACCOUNT_ID,
                Some(json!({ "new-masked": { "state": "enabled" } })),
                None,
                &[],
            )
            .expect("masked_email_set");
        let err = resp
            .check_errors("MaskedEmail/set")
            .expect_err("set error should surface");
        assert!(
            err.to_string().contains("bad"),
            "error should carry the SetError description: {err}"
        );
    }
}
