use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::jmap::client::JmapClient;
use crate::jmap::types::BlobId;

const MAIL_CAPABILITY: &str = "urn:ietf:params:jmap:mail";

/// A JMAP `Email` id. Newtype so it can't be confused with other id strings and
/// so a set of ids can dedup overlapping query windows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmailId(pub String);

impl EmailId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EmailId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A JMAP state token — the cursor for incremental sync. Carried between
/// `Email/changes` calls (`newState` of one becomes `sinceState` of the next).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State(pub String);

impl State {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A window into an `Email/query` result set. JMAP offers two positioning modes:
/// `Anchor` is skip-proof under concurrent inserts; `Position` is a plain offset.
pub enum Page {
    Position {
        position: u64,
        limit: u32,
    },
    Anchor {
        anchor: EmailId,
        anchor_offset: i64,
        limit: u32,
    },
}

/// Faithful mirror of an `Email/query` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EmailQueryResponse {
    pub ids: Vec<EmailId>,
    pub query_state: String,
    pub position: u64,
    pub total: Option<u64>,
}

/// Faithful mirror of an `Email/changes` response — the incremental delta since
/// a prior state. `has_more_changes` means another call (with `new_state` as the
/// cursor) is needed to drain the rest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EmailChangesResponse {
    pub old_state: State,
    pub new_state: State,
    pub has_more_changes: bool,
    pub created: Vec<EmailId>,
    pub updated: Vec<EmailId>,
    pub destroyed: Vec<EmailId>,
}

/// Faithful mirror of a JMAP `Email` object: a zero-loss read shape. The newtype ids are
/// typed; every other property JMAP returns — addresses, `keywords`, `mailboxIds`,
/// headers, `bodyStructure`/`bodyValues`, and any field FastMail adds later — is preserved
/// verbatim in `rest`. JMAP names 1:1. Richer typed fields can be promoted out of `rest`
/// later without losing data; presenters (CLI/MCP) decide what to project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Email {
    pub id: EmailId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob_id: Option<BlobId>,
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

/// Faithful mirror of an `Email/get` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EmailGetResponse {
    pub list: Vec<Email>,
    pub not_found: Vec<EmailId>,
    pub state: State,
}

/// Faithful mirror of an `Email/set` response. `created`/`updated` map each id to the
/// server-completed object (or null); `destroyed` lists the removed ids; each `not_*` map
/// carries a JMAP `SetError` per id that failed. State tokens bracket the mutation
/// (`old_state` may be null per JMAP). No projection — every field is preserved verbatim.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EmailSetResponse {
    pub old_state: Option<State>,
    pub new_state: Option<State>,
    pub created: serde_json::Map<String, serde_json::Value>,
    pub updated: serde_json::Map<String, serde_json::Value>,
    pub destroyed: Vec<EmailId>,
    pub not_created: serde_json::Map<String, serde_json::Value>,
    pub not_updated: serde_json::Map<String, serde_json::Value>,
    pub not_destroyed: serde_json::Map<String, serde_json::Value>,
}

impl EmailSetResponse {
    /// Surface the first partial-failure `SetError` (notCreated → notUpdated →
    /// notDestroyed, the order JMAP documents), as `check_set_errors` does for raw
    /// `/set` responses. `Ok(())` when the whole batch succeeded.
    pub fn check_errors(&self, method: &str) -> Result<()> {
        for (key, map) in [
            ("notCreated", &self.not_created),
            ("notUpdated", &self.not_updated),
            ("notDestroyed", &self.not_destroyed),
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
}

impl JmapClient {
    /// L1 accessor: a single `Email/query` call for one window of results.
    pub fn email_query(
        &self,
        account_id: &str,
        filter: serde_json::Value,
        sort: serde_json::Value,
        page: Page,
    ) -> Result<EmailQueryResponse> {
        let mut args = serde_json::json!({
            "accountId": account_id,
            "filter": filter,
            "sort": sort,
        });

        match page {
            Page::Position { position, limit } => {
                args["position"] = serde_json::json!(position);
                args["limit"] = serde_json::json!(limit);
            }
            Page::Anchor {
                anchor,
                anchor_offset,
                limit,
            } => {
                args["anchor"] = serde_json::json!(anchor.as_str());
                args["anchorOffset"] = serde_json::json!(anchor_offset);
                args["limit"] = serde_json::json!(limit);
            }
        }

        let data = self.call_one(MAIL_CAPABILITY, "Email/query", args)?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }

    /// L1 accessor: a single `Email/changes` call for the delta since `since`.
    /// A `cannotCalculateChanges` JMAP error (state too old) surfaces as `Err`;
    /// the consumer decides whether to fall back to a full re-enumeration.
    pub fn email_changes(
        &self,
        account_id: &str,
        since: &State,
        max_changes: Option<u32>,
    ) -> Result<EmailChangesResponse> {
        let mut args = serde_json::json!({
            "accountId": account_id,
            "sinceState": since.as_str(),
        });
        if let Some(max) = max_changes {
            args["maxChanges"] = serde_json::json!(max);
        }

        let data = self.call_one(MAIL_CAPABILITY, "Email/changes", args)?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }

    /// L1 accessor: a faithful `Email/get` for a set of ids — no projection. Every
    /// property JMAP returns is preserved (typed ids plus [`Email::rest`]). `properties`
    /// selects which JMAP properties to fetch; `None` returns JMAP's default set.
    pub fn email_get(
        &self,
        account_id: &str,
        ids: &[EmailId],
        properties: Option<&[&str]>,
    ) -> Result<EmailGetResponse> {
        let id_strs: Vec<&str> = ids.iter().map(EmailId::as_str).collect();
        let mut args = serde_json::json!({
            "accountId": account_id,
            "ids": id_strs,
        });
        if let Some(props) = properties {
            args["properties"] = serde_json::json!(props);
        }

        let data = self.call_one(MAIL_CAPABILITY, "Email/get", args)?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }

    /// L1 accessor: the `blobId` of an email's raw RFC822 content, for download
    /// via [`JmapClient::download_blob`]. The raw blob carries attachments inline.
    pub fn email_blob_id(&self, account_id: &str, email_id: &EmailId) -> Result<BlobId> {
        let response = self.email_get(
            account_id,
            std::slice::from_ref(email_id),
            Some(&["blobId"]),
        )?;
        response
            .list
            .into_iter()
            .next()
            .and_then(|email| email.blob_id)
            .ok_or_else(|| Error::Jmap {
                method: "Email/get".to_string(),
                message: format!("no blobId for email {email_id}"),
            })
    }

    /// L1 accessor: the current account-wide `Email` state token, for bootstrapping
    /// incremental sync. An `Email/get` with an empty `ids` list returns the `Email`
    /// state in its response; that token seeds the first [`JmapClient::email_changes`]
    /// call (without it, `--since 0` yields `cannotCalculateChanges`).
    pub fn email_state(&self, account_id: &str) -> Result<State> {
        let args = serde_json::json!({
            "accountId": account_id,
            "ids": [],
        });

        let data = self.call_one(MAIL_CAPABILITY, "Email/get", args)?;
        let state = data
            .get("state")
            .and_then(|s| s.as_str())
            .ok_or_else(|| Error::Jmap {
                method: "Email/get".to_string(),
                message: "no state token in Email/get response".to_string(),
            })?;

        Ok(State(state.to_string()))
    }

    /// L1 accessor: a faithful single `Email/set` — create, update, and/or destroy in
    /// one call, no projection. `create`/`update` are JMAP object maps (creation-id →
    /// email, id → patch); `destroy` lists the ids to remove. The caller builds the JMAP
    /// payloads verbatim; typed builders are sugar layered on later.
    pub fn email_set(
        &self,
        account_id: &str,
        create: Option<serde_json::Value>,
        update: Option<serde_json::Value>,
        destroy: &[EmailId],
    ) -> Result<EmailSetResponse> {
        let mut args = serde_json::json!({ "accountId": account_id });
        if let Some(create) = create {
            args["create"] = create;
        }
        if let Some(update) = update {
            args["update"] = update;
        }
        if !destroy.is_empty() {
            let ids: Vec<&str> = destroy.iter().map(EmailId::as_str).collect();
            args["destroy"] = serde_json::json!(ids);
        }

        let data = self.call_one(MAIL_CAPABILITY, "Email/set", args)?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }
}

/// Sugar over `email_query`: a sync iterator that walks every id matching a
/// filter via anchor paging. Imposes an immutable total order (`receivedAt`
/// ascending, id tiebreak) so windows never skip, and dedups ids across windows.
pub struct EmailEnumerator<'a> {
    client: &'a JmapClient,
    account_id: String,
    filter: serde_json::Value,
    sort: serde_json::Value,
    page_size: u32,

    buffer: VecDeque<EmailId>,
    anchor: Option<EmailId>,
    seen: HashSet<EmailId>,
    done: bool,
}

impl<'a> EmailEnumerator<'a> {
    pub fn new(
        client: &'a JmapClient,
        account_id: String,
        filter: serde_json::Value,
        page_size: u32,
    ) -> Self {
        let sort = serde_json::json!([
            { "property": "receivedAt", "isAscending": true },
            { "property": "id", "isAscending": true }
        ]);

        Self {
            client,
            account_id,
            filter,
            sort,
            page_size,
            buffer: VecDeque::new(),
            anchor: None,
            seen: HashSet::new(),
            done: false,
        }
    }

    /// Fetch the next window, buffering ids not seen in earlier windows. A window
    /// shorter than `page_size` (including an empty one) marks the stream done.
    fn fetch_window(&mut self) -> Result<()> {
        let page = match &self.anchor {
            None => Page::Position {
                position: 0,
                limit: self.page_size,
            },
            Some(anchor) => Page::Anchor {
                anchor: anchor.clone(),
                anchor_offset: 1,
                limit: self.page_size,
            },
        };

        let response = self.client.email_query(
            &self.account_id,
            self.filter.clone(),
            self.sort.clone(),
            page,
        )?;

        let window_len = response.ids.len() as u32;
        if let Some(last) = response.ids.last() {
            self.anchor = Some(last.clone());
        }
        for id in response.ids {
            if self.seen.insert(id.clone()) {
                self.buffer.push_back(id);
            }
        }

        if window_len < self.page_size {
            self.done = true;
        }
        Ok(())
    }
}

impl Iterator for EmailEnumerator<'_> {
    type Item = Result<EmailId>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(id) = self.buffer.pop_front() {
                return Some(Ok(id));
            }
            if self.done {
                return None;
            }
            if let Err(e) = self.fetch_window() {
                self.done = true;
                return Some(Err(e));
            }
        }
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

    fn query_response(ids: &[&str]) -> serde_json::Value {
        let ids: Vec<&str> = ids.to_vec();
        json!({
            "methodResponses": [
                ["Email/query", { "ids": ids, "queryState": "s1", "position": 0 }, "call-0"]
            ]
        })
    }

    fn enumerate(client: &JmapClient, page_size: u32) -> Result<Vec<EmailId>> {
        EmailEnumerator::new(client, TEST_ACCOUNT_ID.to_string(), json!({}), page_size).collect()
    }

    #[test]
    fn single_page_terminates_without_second_window() {
        let mock = MockJmap::start();
        mock.handle_method_matching("Email/query", "\"position\"", query_response(&["e001"]));

        let ids = enumerate(&client(&mock), 5).expect("enumerate");
        assert_eq!(ids, vec![EmailId("e001".into())]);
    }

    #[test]
    fn stitches_multiple_windows() {
        let mock = MockJmap::start();
        mock.handle_method_matching(
            "Email/query",
            "\"position\"",
            query_response(&["e001", "e002"]),
        );
        mock.handle_method_matching("Email/query", "\"e002\"", query_response(&["e003"]));

        let ids = enumerate(&client(&mock), 2).expect("enumerate");
        let strs: Vec<&str> = ids.iter().map(EmailId::as_str).collect();
        assert_eq!(strs, vec!["e001", "e002", "e003"]);
    }

    #[test]
    fn full_final_page_then_empty_window_terminates() {
        let mock = MockJmap::start();
        mock.handle_method_matching(
            "Email/query",
            "\"position\"",
            query_response(&["e001", "e002"]),
        );
        mock.handle_method_matching("Email/query", "\"e002\"", query_response(&[]));

        let ids = enumerate(&client(&mock), 2).expect("enumerate");
        let strs: Vec<&str> = ids.iter().map(EmailId::as_str).collect();
        assert_eq!(strs, vec!["e001", "e002"]);
    }

    #[test]
    fn dedups_ids_overlapping_between_windows() {
        let mock = MockJmap::start();
        mock.handle_method_matching(
            "Email/query",
            "\"position\"",
            query_response(&["e001", "e002"]),
        );
        mock.handle_method_matching("Email/query", "\"e002\"", query_response(&["e002", "e003"]));
        mock.handle_method_matching("Email/query", "\"e003\"", query_response(&[]));

        let ids = enumerate(&client(&mock), 2).expect("enumerate");
        let strs: Vec<&str> = ids.iter().map(EmailId::as_str).collect();
        assert_eq!(strs, vec!["e001", "e002", "e003"]);
    }

    #[test]
    fn propagates_midstream_error() {
        let mock = MockJmap::start();
        mock.handle_method_matching(
            "Email/query",
            "\"position\"",
            query_response(&["e001", "e002"]),
        );
        mock.handle_method_matching(
            "Email/query",
            "\"e002\"",
            json!({
                "methodResponses": [["error", { "type": "anchorNotFound" }, "call-0"]]
            }),
        );

        let err = enumerate(&client(&mock), 2).expect_err("midstream error should propagate");
        assert!(
            err.to_string().contains("anchorNotFound"),
            "error should carry JMAP type: {err}"
        );
    }

    #[test]
    fn email_changes_returns_delta() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/changes",
            json!({
                "methodResponses": [["Email/changes", {
                    "oldState": "s1",
                    "newState": "s2",
                    "hasMoreChanges": false,
                    "created": ["e001"],
                    "updated": ["e002"],
                    "destroyed": ["e003"]
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .email_changes(TEST_ACCOUNT_ID, &State("s1".into()), None)
            .expect("email_changes");
        assert_eq!(resp.new_state, State("s2".into()));
        assert!(!resp.has_more_changes);
        assert_eq!(resp.created, vec![EmailId("e001".into())]);
        assert_eq!(resp.updated, vec![EmailId("e002".into())]);
        assert_eq!(resp.destroyed, vec![EmailId("e003".into())]);
    }

    #[test]
    fn email_changes_propagates_cannot_calculate() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/changes",
            json!({
                "methodResponses": [["error", { "type": "cannotCalculateChanges" }, "call-0"]]
            }),
        );

        let err = client(&mock)
            .email_changes(TEST_ACCOUNT_ID, &State("old".into()), None)
            .expect_err("stale state should error");
        assert!(
            err.to_string().contains("cannotCalculateChanges"),
            "error should carry JMAP type: {err}"
        );
    }

    #[test]
    fn email_blob_id_reads_blob_id() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [["Email/get", {
                    "list": [{ "id": "e001", "blobId": "blob-xyz" }]
                }, "call-0"]]
            }),
        );

        let blob = client(&mock)
            .email_blob_id(TEST_ACCOUNT_ID, &EmailId("e001".into()))
            .expect("blobId");
        assert_eq!(blob, BlobId("blob-xyz".into()));
    }

    #[test]
    fn email_blob_id_errors_when_missing() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [["Email/get", { "list": [] }, "call-0"]]
            }),
        );

        let err = client(&mock)
            .email_blob_id(TEST_ACCOUNT_ID, &EmailId("nope".into()))
            .expect_err("missing email should error");
        assert!(
            err.to_string().contains("blobId"),
            "error should mention blobId: {err}"
        );
    }

    #[test]
    fn email_state_reads_top_level_state() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [["Email/get", {
                    "state": "st-42", "list": [], "notFound": []
                }, "call-0"]]
            }),
        );

        let state = client(&mock)
            .email_state(TEST_ACCOUNT_ID)
            .expect("email_state");
        assert_eq!(state, State("st-42".into()));
    }

    #[test]
    fn email_state_errors_when_state_missing() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [["Email/get", { "list": [] }, "call-0"]]
            }),
        );

        let err = client(&mock)
            .email_state(TEST_ACCOUNT_ID)
            .expect_err("missing state should error");
        assert!(
            err.to_string().contains("state token"),
            "error should mention the state token: {err}"
        );
    }

    #[test]
    fn email_get_returns_faithful_email_with_flattened_extras() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [["Email/get", {
                    "state": "s1",
                    "list": [{
                        "id": "e001",
                        "blobId": "b1",
                        "subject": "Hi",
                        "keywords": { "$seen": true },
                        "mailboxIds": { "mb1": true }
                    }],
                    "notFound": []
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .email_get(TEST_ACCOUNT_ID, &[EmailId("e001".into())], None)
            .expect("email_get");

        assert_eq!(resp.list.len(), 1);
        let email = &resp.list[0];
        assert_eq!(email.id, EmailId("e001".into()));
        assert_eq!(email.blob_id, Some(BlobId("b1".into())));
        // Non-typed fields survive verbatim in `rest` — nothing is dropped.
        assert_eq!(email.rest.get("subject"), Some(&json!("Hi")));
        assert_eq!(email.rest.get("keywords"), Some(&json!({ "$seen": true })));
        assert_eq!(email.rest.get("mailboxIds"), Some(&json!({ "mb1": true })));
        // Typed ids are consumed by their fields, not duplicated into `rest`.
        assert!(!email.rest.contains_key("id"));
        assert!(!email.rest.contains_key("blobId"));
        assert_eq!(resp.state, State("s1".into()));
    }

    #[test]
    fn email_get_reports_not_found_ids() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [["Email/get", {
                    "state": "s1", "list": [], "notFound": ["missing"]
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .email_get(TEST_ACCOUNT_ID, &[EmailId("missing".into())], None)
            .expect("email_get");
        assert!(resp.list.is_empty());
        assert_eq!(resp.not_found, vec![EmailId("missing".into())]);
    }

    #[test]
    fn email_set_parses_created_updated_destroyed() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/set",
            json!({
                "methodResponses": [["Email/set", {
                    "oldState": "s1",
                    "newState": "s2",
                    "created": { "draft": { "id": "e-new", "blobId": "b1" } },
                    "updated": { "e001": null },
                    "destroyed": ["e002"]
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .email_set(
                TEST_ACCOUNT_ID,
                Some(json!({ "draft": { "subject": "Hi" } })),
                Some(json!({ "e001": { "keywords/$seen": true } })),
                &[EmailId("e002".into())],
            )
            .expect("email_set");

        assert_eq!(resp.old_state, Some(State("s1".into())));
        assert_eq!(resp.new_state, Some(State("s2".into())));
        assert_eq!(
            resp.created.get("draft").and_then(|d| d.get("id")),
            Some(&json!("e-new"))
        );
        assert!(resp.updated.contains_key("e001"));
        assert_eq!(resp.destroyed, vec![EmailId("e002".into())]);
    }

    #[test]
    fn email_set_surfaces_not_updated_set_errors() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/set",
            json!({
                "methodResponses": [["Email/set", {
                    "updated": {},
                    "notUpdated": { "e001": { "type": "notFound", "description": "missing" } }
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .email_set(TEST_ACCOUNT_ID, None, Some(json!({ "e001": {} })), &[])
            .expect("email_set");

        assert!(resp.updated.is_empty());
        let err = resp
            .not_updated
            .get("e001")
            .expect("notUpdated entry present");
        assert_eq!(err.get("type"), Some(&json!("notFound")));
    }
}
