use crate::actions::{project_fields_array, Action, Context};
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

        let list = data.get("list").cloned().unwrap_or(serde_json::json!([]));
        Ok(project_fields_array(&list, LIST_FIELDS))
    }
}
