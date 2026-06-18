use crate::actions::{project_fields_array, Action, Context};
use crate::error::{Error, Result};
use crate::mcp::types::Tool;

const CAPABILITY: &str = "urn:ietf:params:jmap:mail";
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
            CAPABILITY,
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

/// Create, rename, or delete a mailbox. The variant carries exactly the fields
/// its operation needs — no dead companion fields, no invalid action strings.
#[derive(Debug)]
pub enum ManageMailbox {
    Create { name: String, parent_id: String },
    Rename { mailbox_id: String, name: String },
    Delete { mailbox_id: String },
}

impl ManageMailbox {
    /// Parse the MCP `manage_mailbox` arguments into a variant, validating that
    /// each operation's required fields are present. This is the MCP trust boundary;
    /// CLI callers construct the variant directly.
    pub fn parse(action: &str, name: String, mailbox_id: String, parent_id: String) -> Result<Self> {
        match action {
            "create" if name.is_empty() => {
                Err(Error::InvalidParams("name is required for create".to_string()))
            }
            "create" => Ok(Self::Create { name, parent_id }),

            "rename" if mailbox_id.is_empty() => {
                Err(Error::InvalidParams("mailboxId is required for rename".to_string()))
            }
            "rename" if name.is_empty() => {
                Err(Error::InvalidParams("name is required for rename".to_string()))
            }
            "rename" => Ok(Self::Rename { mailbox_id, name }),

            "delete" if mailbox_id.is_empty() => {
                Err(Error::InvalidParams("mailboxId is required for delete".to_string()))
            }
            "delete" => Ok(Self::Delete { mailbox_id }),

            "" => Err(Error::InvalidParams("action is required".to_string())),
            _ => Err(Error::InvalidParams(
                "action must be create, rename, or delete".to_string(),
            )),
        }
    }
}

impl Action for ManageMailbox {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        match self {
            Self::Create { name, parent_id } => {
                let mut create_obj = serde_json::json!({ "name": name });
                if !parent_id.is_empty() {
                    create_obj["parentId"] = serde_json::json!(parent_id);
                }

                let args = serde_json::json!({
                    "accountId": ctx.account_id,
                    "create": { "new-mailbox": create_obj }
                });
                let data = ctx.jmap.call_one(CAPABILITY, "Mailbox/set", args)?;

                Ok(serde_json::json!({
                    "success": true,
                    "mailboxId": data.get("created")
                        .and_then(|c| c.get("new-mailbox"))
                        .and_then(|m| m.get("id"))
                }))
            }
            Self::Rename { mailbox_id, name } => {
                let args = serde_json::json!({
                    "accountId": ctx.account_id,
                    "update": { mailbox_id.clone(): { "name": name } }
                });
                ctx.jmap.call_one(CAPABILITY, "Mailbox/set", args)?;

                Ok(serde_json::json!({ "success": true, "mailboxId": mailbox_id }))
            }
            Self::Delete { mailbox_id } => {
                let args = serde_json::json!({
                    "accountId": ctx.account_id,
                    "destroy": [mailbox_id]
                });
                ctx.jmap.call_one(CAPABILITY, "Mailbox/set", args)?;

                Ok(serde_json::json!({ "success": true, "mailboxId": mailbox_id }))
            }
        }
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
    fn list_mailboxes_returns_all() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/get",
            json!({
                "methodResponses": [["Mailbox/get", {
                    "accountId": TEST_ACCOUNT_ID,
                    "list": [
                        {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 42, "unreadEmails": 3, "parentId": null, "sortOrder": 1},
                        {"id": "mb2", "name": "Sent", "role": "sent", "totalEmails": 10, "unreadEmails": 0, "parentId": null}
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

        let action = ListMailboxes {
            role: String::new(),
        };
        let result = action.run(&ctx).expect("list should succeed");
        let arr = result.as_array().expect("result should be array");

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], "mb1");
        assert_eq!(arr[0]["name"], "Inbox");
        assert_eq!(arr[0]["role"], "inbox");
        assert_eq!(arr[1]["id"], "mb2");
        assert_eq!(arr[1]["name"], "Sent");

        assert_eq!(arr[0]["totalEmails"], 42);
        assert_eq!(arr[0]["unreadEmails"], 3);
        assert!(arr[0]["parentId"].is_null());
        assert_eq!(arr[1]["totalEmails"], 10);
        assert_eq!(arr[1]["unreadEmails"], 0);
        assert!(arr[0].get("sortOrder").is_none(), "non-LIST_FIELDS should be stripped");
    }

    #[test]
    fn list_mailboxes_filters_by_role() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/get",
            json!({
                "methodResponses": [["Mailbox/get", {
                    "accountId": TEST_ACCOUNT_ID,
                    "list": [
                        {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 42, "unreadEmails": 3, "parentId": null},
                        {"id": "mb2", "name": "Sent", "role": "sent", "totalEmails": 10, "unreadEmails": 0, "parentId": null}
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

        let action = ListMailboxes {
            role: "inbox".to_string(),
        };
        let result = action.run(&ctx).expect("list should succeed");
        let arr = result.as_array().expect("result should be array");

        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Inbox");
        assert_eq!(arr[0]["role"], "inbox");
    }

    #[test]
    fn list_mailboxes_returns_empty_when_role_not_found() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/get",
            json!({
                "methodResponses": [["Mailbox/get", {
                    "accountId": TEST_ACCOUNT_ID,
                    "list": [
                        {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 42, "unreadEmails": 3, "parentId": null}
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

        let action = ListMailboxes {
            role: "archive".to_string(),
        };
        let result = action.run(&ctx).expect("list should succeed");
        let arr = result.as_array().expect("result should be array");

        assert!(arr.is_empty());
    }

    fn parse(action: &str, name: &str, mailbox_id: &str, parent_id: &str) -> Result<ManageMailbox> {
        ManageMailbox::parse(
            action,
            name.to_string(),
            mailbox_id.to_string(),
            parent_id.to_string(),
        )
    }

    #[test]
    fn manage_mailbox_requires_action() {
        let err = parse("", "", "", "").expect_err("should fail without action");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_create_requires_name() {
        let err = parse("create", "", "", "").expect_err("should fail without name");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_rename_requires_mailbox_id() {
        let err = parse("rename", "NewName", "", "").expect_err("should fail without mailbox_id");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_rename_requires_name() {
        let err = parse("rename", "", "mb1", "").expect_err("should fail without name");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_delete_requires_mailbox_id() {
        let err = parse("delete", "", "", "").expect_err("should fail without mailbox_id");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_rejects_invalid_action() {
        let err = parse("archive", "", "", "").expect_err("should reject invalid action");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_create_succeeds() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/set",
            json!({
                "methodResponses": [["Mailbox/set", {
                    "created": {"new-mailbox": {"id": "mbox-new"}}
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

        let action = ManageMailbox::Create {
            name: "Projects".to_string(),
            parent_id: String::new(),
        };
        let result = action.run(&ctx).expect("create should succeed");

        assert_eq!(result["success"], true);
        assert_eq!(result["mailboxId"], "mbox-new");
    }

    #[test]
    fn manage_mailbox_delete_succeeds() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/set",
            json!({
                "methodResponses": [["Mailbox/set", {
                    "destroyed": ["mb-del"]
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

        let action = ManageMailbox::Delete {
            mailbox_id: "mb-del".to_string(),
        };
        let result = action.run(&ctx).expect("delete should succeed");

        assert_eq!(result["success"], true);
        assert_eq!(result["mailboxId"], "mb-del");
    }

    #[test]
    fn manage_mailbox_rename_succeeds() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/set",
            json!({
                "methodResponses": [["Mailbox/set", {
                    "updated": {"mb1": null}
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

        let action = ManageMailbox::Rename {
            mailbox_id: "mb1".to_string(),
            name: "NewName".to_string(),
        };
        let result = action.run(&ctx).expect("rename should succeed");

        assert_eq!(result["success"], true);
        assert_eq!(result["mailboxId"], "mb1");
    }

    #[test]
    fn manage_mailbox_create_with_parent_id() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/set",
            json!({
                "methodResponses": [["Mailbox/set", {
                    "created": {"new-mailbox": {"id": "mbox-child"}}
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

        let action = ManageMailbox::Create {
            name: "Child".to_string(),
            parent_id: "mbox-parent".to_string(),
        };
        let result = action.run(&ctx).expect("create with parent should succeed");

        assert_eq!(result["success"], true);
        assert_eq!(result["mailboxId"], "mbox-child");
    }
}
