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

    #[test]
    fn manage_mailbox_requires_action() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let action = ManageMailbox {
            action: String::new(),
            name: String::new(),
            mailbox_id: String::new(),
            parent_id: String::new(),
        };
        let err = action.run(&ctx).expect_err("should fail without action");

        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_create_requires_name() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let action = ManageMailbox {
            action: "create".to_string(),
            name: String::new(),
            mailbox_id: String::new(),
            parent_id: String::new(),
        };
        let err = action.run(&ctx).expect_err("should fail without name");

        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_rename_requires_mailbox_id() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let action = ManageMailbox {
            action: "rename".to_string(),
            name: "NewName".to_string(),
            mailbox_id: String::new(),
            parent_id: String::new(),
        };
        let err = action
            .run(&ctx)
            .expect_err("should fail without mailbox_id");

        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_rename_requires_name() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let action = ManageMailbox {
            action: "rename".to_string(),
            name: String::new(),
            mailbox_id: "mb1".to_string(),
            parent_id: String::new(),
        };
        let err = action.run(&ctx).expect_err("should fail without name");

        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_delete_requires_mailbox_id() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let action = ManageMailbox {
            action: "delete".to_string(),
            name: String::new(),
            mailbox_id: String::new(),
            parent_id: String::new(),
        };
        let err = action
            .run(&ctx)
            .expect_err("should fail without mailbox_id");

        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn manage_mailbox_rejects_invalid_action() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let action = ManageMailbox {
            action: "archive".to_string(),
            name: String::new(),
            mailbox_id: String::new(),
            parent_id: String::new(),
        };
        let err = action
            .run(&ctx)
            .expect_err("should reject invalid action");

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

        let action = ManageMailbox {
            action: "create".to_string(),
            name: "Projects".to_string(),
            mailbox_id: String::new(),
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

        let action = ManageMailbox {
            action: "delete".to_string(),
            name: String::new(),
            mailbox_id: "mb-del".to_string(),
            parent_id: String::new(),
        };
        let result = action.run(&ctx).expect("delete should succeed");

        assert_eq!(result["success"], true);
        assert_eq!(result["mailboxId"], "mb-del");
    }
}
