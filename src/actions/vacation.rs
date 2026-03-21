use crate::actions::{Action, Context};
use crate::error::Result;
use crate::mcp::types::Tool;

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

        Ok(vacation)
    }
}

pub struct SetVacationResponse {
    pub is_enabled: Option<bool>,
    pub from_date: String,
    pub to_date: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: String,
}

impl Action for SetVacationResponse {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let is_enabled = self.is_enabled.ok_or_else(|| {
            crate::error::Error::InvalidParams("isEnabled is required".to_string())
        })?;

        let mut update = serde_json::json!({
            "isEnabled": is_enabled
        });

        if !self.from_date.is_empty() {
            update["fromDate"] = serde_json::json!(self.from_date);
        }
        if !self.to_date.is_empty() {
            update["toDate"] = serde_json::json!(self.to_date);
        }
        if !self.subject.is_empty() {
            update["subject"] = serde_json::json!(self.subject);
        }
        if !self.text_body.is_empty() {
            update["textBody"] = serde_json::json!(self.text_body);
        }
        if !self.html_body.is_empty() {
            update["htmlBody"] = serde_json::json!(self.html_body);
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
