use crate::actions::{Action, Context};
use crate::error::{Error, Result};
use crate::mcp::types::Tool;

const CAPABILITY: &str = "https://www.fastmail.com/dev/maskedemail";

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list_masked_emails".to_string(),
            description: "List all masked (disposable) email addresses".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "state": {
                        "type": "string",
                        "description": "Filter: pending, enabled, disabled, deleted"
                    }
                }
            }),
        },
        Tool {
            name: "create_masked_email".to_string(),
            description: "Create a new masked email address".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "forDomain": { "type": "string", "description": "Domain this address is for" },
                    "description": { "type": "string", "description": "Human-readable label" },
                    "emailPrefix": { "type": "string", "description": "Preferred prefix for the address" }
                }
            }),
        },
        Tool {
            name: "update_masked_email".to_string(),
            description: "Enable, disable, or delete a masked email address".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Masked email ID" },
                    "state": { "type": "string", "description": "New state: enabled, disabled, deleted" }
                },
                "required": ["id", "state"]
            }),
        },
    ]
}

pub struct ListMaskedEmails {
    pub state: String,
}

impl Action for ListMaskedEmails {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let data = ctx.jmap.call_one(
            CAPABILITY,
            "MaskedEmail/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        let list = data.get("list").cloned().unwrap_or(serde_json::json!([]));

        if self.state.is_empty() {
            return Ok(list);
        }

        let valid_states = ["pending", "enabled", "disabled", "deleted"];
        if !valid_states.contains(&self.state.as_str()) {
            return Err(Error::InvalidParams(
                "state must be pending, enabled, disabled, or deleted".to_string(),
            ));
        }

        let filtered: Vec<&serde_json::Value> = list
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|m| m.get("state").and_then(|s| s.as_str()) == Some(&self.state))
                    .collect()
            })
            .unwrap_or_default();

        Ok(serde_json::json!(filtered))
    }
}

pub struct CreateMaskedEmail {
    pub for_domain: String,
    pub description: String,
    pub email_prefix: String,
}

impl Action for CreateMaskedEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let mut create_obj = serde_json::json!({ "state": "enabled" });

        if !self.for_domain.is_empty() {
            create_obj["forDomain"] = serde_json::json!(self.for_domain);
        }
        if !self.description.is_empty() {
            create_obj["description"] = serde_json::json!(self.description);
        }
        if !self.email_prefix.is_empty() {
            create_obj["emailPrefix"] = serde_json::json!(self.email_prefix);
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "create": { "new-masked": create_obj }
        });

        let data = ctx.jmap.call_one(CAPABILITY, "MaskedEmail/set", args)?;

        let created = data
            .get("created")
            .and_then(|c| c.get("new-masked"))
            .cloned()
            .unwrap_or(serde_json::json!({}));

        Ok(created)
    }
}

pub struct UpdateMaskedEmail {
    pub id: String,
    pub state: String,
}

impl Action for UpdateMaskedEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.id.is_empty() {
            return Err(Error::InvalidParams("id is required".to_string()));
        }

        let valid_states = ["enabled", "disabled", "deleted"];
        if !valid_states.contains(&self.state.as_str()) {
            return Err(Error::InvalidParams(
                "state must be enabled, disabled, or deleted".to_string(),
            ));
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "update": {
                self.id.clone(): { "state": self.state }
            }
        });

        ctx.jmap.call_one(CAPABILITY, "MaskedEmail/set", args)?;

        Ok(serde_json::json!({ "success": true }))
    }
}
