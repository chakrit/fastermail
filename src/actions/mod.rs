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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn project_fields_picks_specified_keys() {
        let obj = json!({"a": 1, "b": 2, "c": 3});
        let result = project_fields(&obj, &["a", "c"]);
        assert_eq!(result, json!({"a": 1, "c": 3}));
    }

    #[test]
    fn project_fields_ignores_missing_keys() {
        let obj = json!({"a": 1});
        let result = project_fields(&obj, &["a", "b"]);
        assert_eq!(result, json!({"a": 1}));
    }

    #[test]
    fn project_fields_returns_empty_for_non_object() {
        let val = json!("hello");
        let result = project_fields(&val, &["a"]);
        assert_eq!(result, json!({}));
    }

    #[test]
    fn project_fields_array_projects_each_element() {
        let arr = json!([
            {"x": 1, "y": 2, "z": 3},
            {"x": 10, "y": 20, "z": 30}
        ]);
        let result = project_fields_array(&arr, &["x", "z"]);
        let items = result.as_array().expect("array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], json!({"x": 1, "z": 3}));
        assert_eq!(items[1], json!({"x": 10, "z": 30}));
    }

    #[test]
    fn project_fields_array_returns_clone_for_non_array() {
        let val = json!("hello");
        let result = project_fields_array(&val, &["a"]);
        assert_eq!(result, json!("hello"));
    }
}
