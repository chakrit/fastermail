use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::jmap::client::JmapClient;

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
    // L1 mirrors the full JMAP response; these are consumed by later slices
    // (incremental sync) and the future lib API, not by the current bin callers.
    #[allow(dead_code)]
    pub query_state: String,
    #[allow(dead_code)]
    pub position: u64,
    #[allow(dead_code)]
    pub total: Option<u64>,
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
}
