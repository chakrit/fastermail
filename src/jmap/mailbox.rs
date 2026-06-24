use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::jmap::client::JmapClient;

const MAIL_CAPABILITY: &str = "urn:ietf:params:jmap:mail";

/// A JMAP `Mailbox` id. Newtype so it can't be confused with other id strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MailboxId(pub String);

impl MailboxId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MailboxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Faithful mirror of a JMAP `Mailbox` object: a zero-loss read shape. The id is typed;
/// every other property JMAP returns — `name`, `role`, `parentId`, `totalEmails`,
/// `unreadEmails`, `sortOrder`, `totalThreads`, `unreadThreads`, `myRights`,
/// `isSubscribed`, and any field FastMail adds later — is preserved verbatim in `rest`.
/// JMAP names 1:1. Presenters (CLI/MCP) decide what to project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mailbox {
    pub id: MailboxId,
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

/// Faithful mirror of a `Mailbox/get` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MailboxGetResponse {
    pub list: Vec<Mailbox>,
    pub not_found: Vec<MailboxId>,
    pub state: String,
}

/// Faithful mirror of a `Mailbox/set` response. `created`/`updated` map each id to the
/// server-completed object (or null); `destroyed` lists the removed ids; each `not_*` map
/// carries a JMAP `SetError` per id that failed. No projection — every field is preserved
/// verbatim. `Serialize` so the action can return it faithfully (the front-ends then dig
/// the affected id and apply the L3 `{success}` wrapper).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MailboxSetResponse {
    pub old_state: Option<String>,
    pub new_state: Option<String>,
    pub created: serde_json::Map<String, serde_json::Value>,
    pub updated: serde_json::Map<String, serde_json::Value>,
    pub destroyed: Vec<MailboxId>,
    pub not_created: serde_json::Map<String, serde_json::Value>,
    pub not_updated: serde_json::Map<String, serde_json::Value>,
    pub not_destroyed: serde_json::Map<String, serde_json::Value>,
}

impl MailboxSetResponse {
    /// Surface the first partial-failure `SetError` (notCreated → notUpdated →
    /// notDestroyed, the order JMAP documents), mirroring [`EmailSetResponse::check_errors`].
    /// `Ok(())` when the whole batch succeeded.
    ///
    /// [`EmailSetResponse::check_errors`]: crate::jmap::email::EmailSetResponse::check_errors
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
    /// L1 accessor: a faithful `Mailbox/get` — no projection. With no `ids`, JMAP
    /// returns every mailbox in the account; every property is preserved (typed id plus
    /// [`Mailbox::rest`]).
    pub fn mailbox_get(&self, account_id: &str) -> Result<MailboxGetResponse> {
        let data = self.call_one(
            MAIL_CAPABILITY,
            "Mailbox/get",
            serde_json::json!({ "accountId": account_id }),
        )?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }

    /// L1 accessor: a faithful single `Mailbox/set` — create, update, and/or destroy in
    /// one call, no projection. `create`/`update` are JMAP object maps (creation-id →
    /// mailbox, id → patch); `destroy` lists the ids to remove. The caller builds the JMAP
    /// payloads verbatim; typed builders are sugar layered on later. Mirrors
    /// [`JmapClient::email_set`].
    pub fn mailbox_set(
        &self,
        account_id: &str,
        create: Option<serde_json::Value>,
        update: Option<serde_json::Value>,
        destroy: &[MailboxId],
    ) -> Result<MailboxSetResponse> {
        let mut args = serde_json::json!({ "accountId": account_id });
        if let Some(create) = create {
            args["create"] = create;
        }
        if let Some(update) = update {
            args["update"] = update;
        }
        if !destroy.is_empty() {
            let ids: Vec<&str> = destroy.iter().map(MailboxId::as_str).collect();
            args["destroy"] = serde_json::json!(ids);
        }

        let data = self.call_one(MAIL_CAPABILITY, "Mailbox/set", args)?;
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
    fn mailbox_get_returns_faithful_mailbox_with_flattened_extras() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/get",
            json!({
                "methodResponses": [["Mailbox/get", {
                    "state": "s1",
                    "list": [{
                        "id": "mb1",
                        "name": "Inbox",
                        "role": "inbox",
                        "parentId": null,
                        "totalEmails": 42,
                        "unreadEmails": 3,
                        "sortOrder": 1,
                        "totalThreads": 40,
                        "unreadThreads": 2,
                        "myRights": {"mayRead": true},
                        "isSubscribed": true
                    }],
                    "notFound": []
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .mailbox_get(TEST_ACCOUNT_ID)
            .expect("mailbox_get");

        assert_eq!(resp.list.len(), 1);
        let mailbox = &resp.list[0];
        assert_eq!(mailbox.id, MailboxId("mb1".into()));
        // Every non-id field survives verbatim in `rest` — nothing is dropped, including
        // the 5 fields the L3 presenter projects out (sortOrder/totalThreads/
        // unreadThreads/myRights/isSubscribed).
        assert_eq!(mailbox.rest.get("name"), Some(&json!("Inbox")));
        assert_eq!(mailbox.rest.get("role"), Some(&json!("inbox")));
        assert_eq!(mailbox.rest.get("parentId"), Some(&json!(null)));
        assert_eq!(mailbox.rest.get("totalEmails"), Some(&json!(42)));
        assert_eq!(mailbox.rest.get("unreadEmails"), Some(&json!(3)));
        assert_eq!(mailbox.rest.get("sortOrder"), Some(&json!(1)));
        assert_eq!(mailbox.rest.get("totalThreads"), Some(&json!(40)));
        assert_eq!(mailbox.rest.get("unreadThreads"), Some(&json!(2)));
        assert_eq!(
            mailbox.rest.get("myRights"),
            Some(&json!({"mayRead": true}))
        );
        assert_eq!(mailbox.rest.get("isSubscribed"), Some(&json!(true)));
        // The typed id is consumed by its field, not duplicated into `rest`.
        assert!(!mailbox.rest.contains_key("id"));
        assert_eq!(resp.state, "s1");
    }

    #[test]
    fn mailbox_get_returns_empty_for_no_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/get",
            json!({"methodResponses": [["Mailbox/get", {"list": []}, "call-0"]]}),
        );

        let resp = client(&mock)
            .mailbox_get(TEST_ACCOUNT_ID)
            .expect("mailbox_get");
        assert!(resp.list.is_empty());
    }

    #[test]
    fn mailbox_set_creates_and_returns_id() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/set",
            json!({
                "methodResponses": [["Mailbox/set", {
                    "created": {"new-mailbox": {"id": "mbox-new"}}
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .mailbox_set(
                TEST_ACCOUNT_ID,
                Some(json!({ "new-mailbox": { "name": "Projects" } })),
                None,
                &[],
            )
            .expect("mailbox_set");
        assert_eq!(resp.created["new-mailbox"]["id"], "mbox-new");
        resp.check_errors("Mailbox/set").expect("no set errors");
    }

    #[test]
    fn mailbox_set_destroys() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/set",
            json!({
                "methodResponses": [["Mailbox/set", {"destroyed": ["mb-del"]}, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .mailbox_set(TEST_ACCOUNT_ID, None, None, &[MailboxId("mb-del".into())])
            .expect("mailbox_set");
        assert_eq!(resp.destroyed, vec![MailboxId("mb-del".into())]);
    }

    #[test]
    fn mailbox_set_surfaces_not_created_set_errors() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/set",
            json!({
                "methodResponses": [["Mailbox/set", {
                    "created": {},
                    "notCreated": { "new-mailbox": { "type": "invalidProperties", "description": "bad name" } }
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .mailbox_set(
                TEST_ACCOUNT_ID,
                Some(json!({ "new-mailbox": { "name": "" } })),
                None,
                &[],
            )
            .expect("mailbox_set");
        let err = resp
            .check_errors("Mailbox/set")
            .expect_err("set error should surface");
        assert!(
            err.to_string().contains("bad name"),
            "error should carry the SetError description: {err}"
        );
    }
}
