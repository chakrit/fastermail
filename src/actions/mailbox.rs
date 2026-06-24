use crate::actions::{Action, Context};
use crate::error::{Error, Result};
use crate::jmap::mailbox::MailboxId;
use crate::mcp::types::Tool;

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

pub struct ListMailboxes;

impl Action for ListMailboxes {
    /// Return the faithful `Mailbox/get` list — every JMAP property verbatim, no
    /// projection and no role filter. The CLI/MCP presenters apply
    /// `present::project_mailbox_list` (which owns the field selection and the role
    /// filter). An empty account returns `[]`.
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let response = ctx.jmap.mailbox_get(&ctx.account_id)?;
        Ok(serde_json::to_value(response.list)?)
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
    pub fn parse(
        action: &str,
        name: String,
        mailbox_id: String,
        parent_id: String,
    ) -> Result<Self> {
        match action {
            "create" if name.is_empty() => Err(Error::InvalidParams(
                "name is required for create".to_string(),
            )),
            "create" => Ok(Self::Create { name, parent_id }),

            "rename" if mailbox_id.is_empty() => Err(Error::InvalidParams(
                "mailboxId is required for rename".to_string(),
            )),
            "rename" if name.is_empty() => Err(Error::InvalidParams(
                "name is required for rename".to_string(),
            )),
            "rename" => Ok(Self::Rename { mailbox_id, name }),

            "delete" if mailbox_id.is_empty() => Err(Error::InvalidParams(
                "mailboxId is required for delete".to_string(),
            )),
            "delete" => Ok(Self::Delete { mailbox_id }),

            "" => Err(Error::InvalidParams("action is required".to_string())),
            _ => Err(Error::InvalidParams(
                "action must be create, rename, or delete".to_string(),
            )),
        }
    }
}

/// The id JMAP creation key for a new mailbox in a `Mailbox/set` create map.
const CREATE_KEY: &str = "new-mailbox";

impl ManageMailbox {
    /// The mailbox id the operation affected, for the L3 `{success, mailboxId}` wrapper.
    /// For create it is dug from the faithful `created` object (or None if absent); for
    /// rename/delete it is the input id the caller already holds.
    pub fn resolved_id(&self, data: &serde_json::Value) -> Option<String> {
        match self {
            Self::Create { .. } => data
                .get("created")
                .and_then(|c| c.get(CREATE_KEY))
                .and_then(|m| m.get("id"))
                .and_then(|id| id.as_str())
                .map(String::from),
            Self::Rename { mailbox_id, .. } | Self::Delete { mailbox_id } => {
                Some(mailbox_id.clone())
            }
        }
    }
}

impl Action for ManageMailbox {
    /// Route through the L1 `mailbox_set` accessor and return the faithful
    /// `Mailbox/set` response — no projection, no `{success}` wrapper. The CLI/MCP
    /// presenters wrap it via `present::set_with_id`/`set_ok`; [`Self::resolved_id`]
    /// extracts the affected id. Surfaces a JMAP `SetError` as an error.
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let response = match self {
            Self::Create { name, parent_id } => {
                let mut create_obj = serde_json::json!({ "name": name });
                if !parent_id.is_empty() {
                    create_obj["parentId"] = serde_json::json!(parent_id);
                }
                ctx.jmap.mailbox_set(
                    &ctx.account_id,
                    Some(serde_json::json!({ CREATE_KEY: create_obj })),
                    None,
                    &[],
                )?
            }
            Self::Rename { mailbox_id, name } => ctx.jmap.mailbox_set(
                &ctx.account_id,
                None,
                Some(serde_json::json!({ mailbox_id.clone(): { "name": name } })),
                &[],
            )?,
            Self::Delete { mailbox_id } => ctx.jmap.mailbox_set(
                &ctx.account_id,
                None,
                None,
                &[MailboxId(mailbox_id.clone())],
            )?,
        };

        response.check_errors("Mailbox/set")?;
        Ok(serde_json::to_value(response)?)
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
    fn list_mailboxes_returns_faithful_unprojected_data() {
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

        let result = ListMailboxes.run(&ctx).expect("list should succeed");
        let arr = result.as_array().expect("result should be array");

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], "mb1");
        assert_eq!(arr[0]["name"], "Inbox");
        assert_eq!(arr[0]["role"], "inbox");
        assert_eq!(arr[1]["id"], "mb2");
        assert_eq!(arr[1]["name"], "Sent");

        // The action no longer projects or filters: every field survives, including the
        // one the presenter later drops (`sortOrder`). The role filter + field selection
        // now live in `present::project_mailbox_list` (tested there + in the goldens).
        assert_eq!(arr[0]["sortOrder"], 1, "faithful data keeps sortOrder");
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

        // The action returns faithful `Mailbox/set` data (no `{success}` wrapper); the
        // affected id comes from `resolved_id`. The front-ends do the L3 wrapping.
        assert_eq!(result["created"]["new-mailbox"]["id"], "mbox-new");
        assert_eq!(action.resolved_id(&result).as_deref(), Some("mbox-new"));
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

        assert_eq!(result["destroyed"][0], "mb-del");
        assert_eq!(action.resolved_id(&result).as_deref(), Some("mb-del"));
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

        assert!(result["updated"].get("mb1").is_some());
        assert_eq!(action.resolved_id(&result).as_deref(), Some("mb1"));
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

        assert_eq!(result["created"]["new-mailbox"]["id"], "mbox-child");
        assert_eq!(action.resolved_id(&result).as_deref(), Some("mbox-child"));
    }
}
