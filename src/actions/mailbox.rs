use crate::actions::{project_fields_array, Action, Context};
use crate::error::{Error, Result};
use crate::mcp::types::Tool;

const LIST_FIELDS: &[&str] = &["id", "name", "role", "totalEmails", "unreadEmails", "parentId"];

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list_mailboxes".to_string(),
            description: "List all mailboxes with metadata".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Filter by role (inbox, sent, drafts, trash, junk, archive)",
                        "enum": ["inbox", "sent", "drafts", "trash", "junk", "archive"]
                    }
                }
            }),
        },
        Tool {
            name: "manage_mailbox".to_string(),
            description: "Create, rename, or delete mailboxes".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "create, rename, or delete",
                        "enum": ["create", "rename", "delete"]
                    },
                    "name": {
                        "type": "string",
                        "description": "Mailbox name (required for create/rename)"
                    },
                    "mailboxId": {
                        "type": "string",
                        "description": "Mailbox ID (required for rename/delete)"
                    },
                    "parentId": {
                        "type": "string",
                        "description": "Parent mailbox ID for create"
                    }
                },
                "required": ["action"]
            }),
        },
    ]
}

pub struct ListMailboxes {
    pub role: String,
}

impl Action for ListMailboxes {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let args = serde_json::json!({
            "accountId": ctx.account_id,
        });

        let data = ctx.jmap.call_one(
            "urn:ietf:params:jmap:mail",
            "Mailbox/get",
            args,
        )?;

        let list = data.get("list").cloned().unwrap_or(serde_json::json!([]));

        if self.role.is_empty() {
            return Ok(project_fields_array(&list, LIST_FIELDS));
        }

        let filtered: Vec<&serde_json::Value> = list
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some(&self.role))
                    .collect()
            })
            .unwrap_or_default();

        Ok(project_fields_array(&serde_json::json!(filtered), LIST_FIELDS))
    }
}

pub struct ManageMailbox {
    pub action: String,
    pub name: String,
    pub mailbox_id: String,
    pub parent_id: String,
}

impl Action for ManageMailbox {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.action.is_empty() {
            return Err(Error::InvalidParams("action is required".to_string()));
        }

        match self.action.as_str() {
            "create" => {
                if self.name.is_empty() {
                    return Err(Error::InvalidParams(
                        "name is required for create".to_string(),
                    ));
                }

                let mut create_obj = serde_json::json!({ "name": self.name });
                if !self.parent_id.is_empty() {
                    create_obj["parentId"] = serde_json::json!(self.parent_id);
                }

                let args = serde_json::json!({
                    "accountId": ctx.account_id,
                    "create": { "new-mailbox": create_obj }
                });

                let data = ctx.jmap.call_one(
                    "urn:ietf:params:jmap:mail",
                    "Mailbox/set",
                    args,
                )?;

                Ok(serde_json::json!({
                    "success": true,
                    "mailboxId": data.get("created")
                        .and_then(|c| c.get("new-mailbox"))
                        .and_then(|m| m.get("id"))
                }))
            }
            "rename" => {
                if self.mailbox_id.is_empty() {
                    return Err(Error::InvalidParams(
                        "mailboxId is required for rename".to_string(),
                    ));
                }
                if self.name.is_empty() {
                    return Err(Error::InvalidParams(
                        "name is required for rename".to_string(),
                    ));
                }

                let args = serde_json::json!({
                    "accountId": ctx.account_id,
                    "update": {
                        self.mailbox_id.clone(): { "name": self.name }
                    }
                });

                ctx.jmap.call_one(
                    "urn:ietf:params:jmap:mail",
                    "Mailbox/set",
                    args,
                )?;

                Ok(serde_json::json!({
                    "success": true,
                    "mailboxId": self.mailbox_id
                }))
            }
            "delete" => {
                if self.mailbox_id.is_empty() {
                    return Err(Error::InvalidParams(
                        "mailboxId is required for delete".to_string(),
                    ));
                }

                let args = serde_json::json!({
                    "accountId": ctx.account_id,
                    "destroy": [self.mailbox_id]
                });

                ctx.jmap.call_one(
                    "urn:ietf:params:jmap:mail",
                    "Mailbox/set",
                    args,
                )?;

                Ok(serde_json::json!({
                    "success": true,
                    "mailboxId": self.mailbox_id
                }))
            }
            _ => Err(Error::InvalidParams(
                "action must be create, rename, or delete".to_string(),
            )),
        }
    }
}
