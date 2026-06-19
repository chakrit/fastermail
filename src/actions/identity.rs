use crate::actions::{Action, Context, project_fields_array};
use crate::error::Result;
use crate::mcp::types::Tool;

const LIST_FIELDS: &[&str] = &["id", "name", "email", "replyTo"];

pub fn tools() -> Vec<Tool> {
    vec![Tool {
        name: "list_identities".to_string(),
        description: "List sending identities (From addresses)".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }]
}

pub struct ListIdentities;

impl Action for ListIdentities {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let data = ctx.jmap.call_one(
            "urn:ietf:params:jmap:submission",
            "Identity/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        let list = data
            .get("list")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        Ok(project_fields_array(&list, LIST_FIELDS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Context;
    use crate::jmap::client::JmapClient;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    #[test]
    fn list_identities_returns_projected_fields() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Identity/get",
            json!({
                "methodResponses": [["Identity/get", {
                    "list": [
                        {
                            "id": "id1",
                            "name": "Alice",
                            "email": "alice@example.com",
                            "replyTo": null,
                            "textSignature": "sig1"
                        },
                        {
                            "id": "id2",
                            "name": "Bob",
                            "email": "bob@example.com",
                            "replyTo": [{"email": "bob-reply@example.com"}],
                            "textSignature": "sig2"
                        }
                    ]
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = ListIdentities.run(&ctx).expect("run");
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 2);

        // Verify only LIST_FIELDS are present (id, name, email, replyTo)
        for item in arr {
            let obj = item.as_object().expect("object");
            for key in obj.keys() {
                assert!(
                    LIST_FIELDS.contains(&key.as_str()),
                    "unexpected field: {key}"
                );
            }
        }

        // Spot-check values
        assert_eq!(arr[0]["id"], "id1");
        assert_eq!(arr[0]["name"], "Alice");
        assert_eq!(arr[0]["email"], "alice@example.com");
        assert!(arr[0]["replyTo"].is_null());
        assert!(arr[0].get("textSignature").is_none());

        assert_eq!(arr[1]["id"], "id2");
        assert_eq!(arr[1]["email"], "bob@example.com");
        assert!(arr[1].get("textSignature").is_none());
    }

    #[test]
    fn list_identities_returns_empty_for_no_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Identity/get",
            json!({"methodResponses": [["Identity/get", {"list": []}, "call-0"]]}),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = ListIdentities.run(&ctx).expect("empty list should succeed");
        let arr = result.as_array().expect("should be array");
        assert!(arr.is_empty(), "should return empty array");
    }
}
