use crate::actions::{project_fields, Action, Context};
use crate::error::Result;
use crate::mcp::types::Tool;

const GET_FIELDS: &[&str] = &["isEnabled", "fromDate", "toDate", "subject", "textBody", "htmlBody"];

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "get_vacation_response".to_string(),
            description: "Get the current vacation/auto-reply settings".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "set_vacation_response".to_string(),
            description: "Enable, disable, or update the vacation auto-reply".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "isEnabled": { "type": "boolean", "description": "Enable or disable auto-reply" },
                    "fromDate": { "type": "string", "description": "Start date (ISO 8601, UTC)" },
                    "toDate": { "type": "string", "description": "End date (ISO 8601, UTC)" },
                    "subject": { "type": "string", "description": "Auto-reply subject" },
                    "textBody": { "type": "string", "description": "Plain text auto-reply body" },
                    "htmlBody": { "type": "string", "description": "HTML auto-reply body" }
                },
                "required": ["isEnabled"]
            }),
        },
    ]
}

pub struct GetVacationResponse;

impl Action for GetVacationResponse {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let data = ctx.jmap.call_one(
            "urn:ietf:params:jmap:vacationresponse",
            "VacationResponse/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        let vacation = data
            .get("list")
            .and_then(|l| l.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .unwrap_or(serde_json::json!({}));

        Ok(project_fields(&vacation, GET_FIELDS))
    }
}

pub struct SetVacationResponse {
    pub is_enabled: Option<bool>,
    pub raw_args: serde_json::Value,
}

impl SetVacationResponse {
    /// If a key is present in the raw arguments, return its value for the JMAP update.
    /// Present with null or empty string -> set to null (clear the field).
    /// Present with a non-empty string -> set to that string.
    /// Absent -> don't include (leave unchanged).
    fn resolve_field(args: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
        match args.get(key) {
            None => None,
            Some(v) if v.is_null() => Some(serde_json::Value::Null),
            Some(v) => {
                let s = v.as_str().unwrap_or("");
                if s.is_empty() {
                    Some(serde_json::Value::Null)
                } else {
                    Some(serde_json::json!(s))
                }
            }
        }
    }
}

impl Action for SetVacationResponse {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let is_enabled = self.is_enabled.ok_or_else(|| {
            crate::error::Error::InvalidParams("isEnabled is required".to_string())
        })?;

        let mut update = serde_json::json!({
            "isEnabled": is_enabled
        });

        let fields = &["fromDate", "toDate", "subject", "textBody", "htmlBody"];
        for &field in fields {
            if let Some(value) = Self::resolve_field(&self.raw_args, field) {
                update[field] = value;
            }
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "update": { "singleton": update }
        });

        ctx.jmap.call_one(
            "urn:ietf:params:jmap:vacationresponse",
            "VacationResponse/set",
            args,
        )?;

        Ok(serde_json::json!({ "success": true }))
    }
}
