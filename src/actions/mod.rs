pub mod email;
pub mod identity;
pub mod mailbox;
pub mod masked_email;
pub mod vacation;

use crate::error::Result;
use crate::jmap::client::JmapClient;
use crate::mcp::types::Tool;
use crate::recorder::Recorder;

/// Context passed to all actions.
pub struct Context {
    pub jmap: JmapClient,
    pub account_id: String,
    pub recorder: Option<Recorder>,
}

/// Every MCP tool implements this trait.
pub trait Action {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value>;
}

/// Create a new JSON object containing only the specified fields from `obj`.
pub fn project_fields(obj: &serde_json::Value, fields: &[&str]) -> serde_json::Value {
    let mut result = serde_json::Map::new();
    if let Some(map) = obj.as_object() {
        for &field in fields {
            if let Some(value) = map.get(field) {
                result.insert(field.to_string(), value.clone());
            }
        }
    }
    serde_json::Value::Object(result)
}

/// Apply field projection to each element of a JSON array.
pub fn project_fields_array(arr: &serde_json::Value, fields: &[&str]) -> serde_json::Value {
    match arr.as_array() {
        Some(items) => {
            let projected: Vec<serde_json::Value> =
                items.iter().map(|item| project_fields(item, fields)).collect();
            serde_json::json!(projected)
        }
        None => arr.clone(),
    }
}

/// Return the list of all registered tool definitions.
pub fn tool_definitions() -> Vec<Tool> {
    let mut tools = Vec::new();

    tools.extend(mailbox::tools());
    tools.extend(email::tools());
    tools.extend(vacation::tools());
    tools.extend(identity::tools());
    tools.extend(masked_email::tools());

    tools
}
