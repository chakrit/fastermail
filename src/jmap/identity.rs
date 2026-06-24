use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::jmap::client::JmapClient;

const SUBMISSION_CAPABILITY: &str = "urn:ietf:params:jmap:submission";

/// A JMAP `Identity` id. Newtype so it can't be confused with other id strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdentityId(pub String);

impl IdentityId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IdentityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Faithful mirror of a JMAP `Identity` object: a zero-loss read shape. The id is typed;
/// every other property JMAP returns — `name`, `email`, `replyTo`, `bcc`,
/// `textSignature`, `htmlSignature`, `mayDelete`, and any field FastMail adds later — is
/// preserved verbatim in `rest`. JMAP names 1:1. Presenters (CLI/MCP) decide what to
/// project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub id: IdentityId,
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

/// Faithful mirror of an `Identity/get` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IdentityGetResponse {
    pub list: Vec<Identity>,
    pub not_found: Vec<IdentityId>,
    pub state: String,
}

impl JmapClient {
    /// L1 accessor: a faithful `Identity/get` — no projection. With no `ids`, JMAP
    /// returns every identity in the account; every property is preserved (typed id plus
    /// [`Identity::rest`]).
    pub fn identity_get(&self, account_id: &str) -> Result<IdentityGetResponse> {
        let data = self.call_one(
            SUBMISSION_CAPABILITY,
            "Identity/get",
            serde_json::json!({ "accountId": account_id }),
        )?;
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
    fn identity_get_returns_faithful_identity_with_flattened_extras() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Identity/get",
            json!({
                "methodResponses": [["Identity/get", {
                    "state": "s1",
                    "list": [{
                        "id": "id1",
                        "name": "Alice",
                        "email": "alice@example.com",
                        "replyTo": null,
                        "bcc": null,
                        "textSignature": "sig1",
                        "htmlSignature": "<p>sig1</p>",
                        "mayDelete": true
                    }],
                    "notFound": []
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .identity_get(TEST_ACCOUNT_ID)
            .expect("identity_get");

        assert_eq!(resp.list.len(), 1);
        let identity = &resp.list[0];
        assert_eq!(identity.id, IdentityId("id1".into()));
        // Every non-id field survives verbatim in `rest` — nothing is dropped, including
        // the fields the L3 presenter projects out (bcc/signatures/mayDelete).
        assert_eq!(identity.rest.get("name"), Some(&json!("Alice")));
        assert_eq!(
            identity.rest.get("email"),
            Some(&json!("alice@example.com"))
        );
        assert_eq!(identity.rest.get("replyTo"), Some(&json!(null)));
        assert_eq!(identity.rest.get("bcc"), Some(&json!(null)));
        assert_eq!(identity.rest.get("textSignature"), Some(&json!("sig1")));
        assert_eq!(
            identity.rest.get("htmlSignature"),
            Some(&json!("<p>sig1</p>"))
        );
        assert_eq!(identity.rest.get("mayDelete"), Some(&json!(true)));
        // The typed id is consumed by its field, not duplicated into `rest`.
        assert!(!identity.rest.contains_key("id"));
        assert_eq!(resp.state, "s1");
    }

    #[test]
    fn identity_get_returns_empty_for_no_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Identity/get",
            json!({"methodResponses": [["Identity/get", {"list": []}, "call-0"]]}),
        );

        let resp = client(&mock)
            .identity_get(TEST_ACCOUNT_ID)
            .expect("identity_get");
        assert!(resp.list.is_empty());
    }
}
