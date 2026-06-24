use crate::actions::{Action, Context};
use crate::error::Result;
use crate::mcp::types::Tool;

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
    /// Return faithful `Identity` data — every JMAP property verbatim, no projection.
    /// The CLI/MCP presenters apply `present::project_identity_list`.
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let response = ctx.jmap.identity_get(&ctx.account_id)?;
        let list: Vec<serde_json::Value> = response
            .list
            .into_iter()
            .map(serde_json::to_value)
            .collect::<std::result::Result<_, _>>()?;
        Ok(serde_json::Value::Array(list))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Context;
    use crate::jmap::client::JmapClient;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    fn ctx(mock: &MockJmap) -> Context {
        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        }
    }

    #[test]
    fn list_identities_returns_faithful_unprojected_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Identity/get",
            json!({
                "methodResponses": [["Identity/get", {
                    "list": [{
                        "id": "id1",
                        "name": "Alice",
                        "email": "alice@example.com",
                        "replyTo": null,
                        "textSignature": "sig1",
                        "mayDelete": true
                    }]
                }, "call-0"]]
            }),
        );

        let result = ListIdentities.run(&ctx(&mock)).expect("run");
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 1);

        // The action no longer projects: fields the presenter later drops are still here.
        assert_eq!(arr[0]["id"], "id1");
        assert_eq!(arr[0]["name"], "Alice");
        assert_eq!(arr[0]["email"], "alice@example.com");
        assert_eq!(arr[0]["textSignature"], "sig1");
        assert_eq!(arr[0]["mayDelete"], true);
    }

    #[test]
    fn list_identities_returns_empty_for_no_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Identity/get",
            json!({"methodResponses": [["Identity/get", {"list": []}, "call-0"]]}),
        );

        let result = ListIdentities
            .run(&ctx(&mock))
            .expect("empty list should succeed");
        let arr = result.as_array().expect("should be array");
        assert!(arr.is_empty(), "should return empty array");
    }
}
