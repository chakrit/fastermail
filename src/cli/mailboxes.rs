use crate::json;
use clap::Subcommand;

use crate::actions::mailbox::{ListMailboxes, ManageMailbox};
use crate::actions::{Action, Context};
use crate::cli::io::{Io, OutputMode};
use crate::error::Result;

#[derive(Subcommand)]
pub enum MailboxCommand {
    /// List all mailboxes
    List {
        /// Filter by role (inbox, sent, drafts, trash, junk, archive)
        #[arg(long)]
        role: Option<String>,
    },

    /// Create a mailbox
    Create {
        /// Mailbox name
        name: String,

        /// Parent mailbox ID
        #[arg(long)]
        parent_id: Option<String>,
    },

    /// Rename a mailbox
    Rename {
        /// Mailbox ID
        mailbox_id: String,

        /// New name
        new_name: String,
    },

    /// Delete a mailbox
    Delete {
        /// Mailbox ID
        mailbox_id: String,
    },
}

pub fn run(cmd: MailboxCommand, ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        MailboxCommand::List { role } => {
            let spinner = io.progress("Fetching mailboxes…");
            let action = ListMailboxes {
                role: role.unwrap_or_default(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            format_mailbox_list(io, &value);
        }
        MailboxCommand::Create { name, parent_id } => {
            let spinner = io.progress("Creating mailbox…");
            let action = ManageMailbox::Create {
                name: name.clone(),
                parent_id: parent_id.unwrap_or_default(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;

            if io.mode() == OutputMode::Human {
                let id = value
                    .get("mailboxId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                io.done(&format!("Created mailbox \"{name}\" (ID: {id})"));
            } else {
                io.json(&value);
            }
        }
        MailboxCommand::Rename {
            mailbox_id,
            new_name,
        } => {
            let spinner = io.progress("Renaming mailbox…");
            let action = ManageMailbox::Rename {
                mailbox_id,
                name: new_name.clone(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            result?;
            if io.mode() == OutputMode::Human {
                io.done(&format!("Renamed to \"{new_name}\""));
            } else {
                io.json(&serde_json::json!({ "success": true }));
            }
        }
        MailboxCommand::Delete { mailbox_id } => {
            let spinner = io.progress("Deleting mailbox…");
            let action = ManageMailbox::Delete {
                mailbox_id: mailbox_id.clone(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            result?;
            if io.mode() == OutputMode::Human {
                io.done(&format!("Deleted mailbox {mailbox_id}"));
            } else {
                io.json(&serde_json::json!({ "success": true }));
            }
        }
    }
    Ok(())
}

fn format_mailbox_list(io: &Io, value: &serde_json::Value) {
    if io.mode() != OutputMode::Human {
        io.json(value);
        return;
    }

    let mailboxes = match value.as_array() {
        Some(arr) => arr,
        None => {
            io.json(value);
            return;
        }
    };

    if mailboxes.is_empty() {
        io.warn("No mailboxes found");
        return;
    }

    io.done(&format!("{} mailbox(es)", mailboxes.len()));
    io.separator();

    io.data(&format!(
        "{:<40} {:<20} {:<10} {:>6} {:>6}",
        "ID", "NAME", "ROLE", "TOTAL", "UNREAD"
    ));
    io.data(&"─".repeat(84));

    for mb in mailboxes {
        let id = json::str_at(mb, "/id").unwrap_or("?");
        let name = json::str_at(mb, "/name").unwrap_or("?");
        let role = mb.get("role").and_then(|v| v.as_str()).unwrap_or("");
        let total = mb.get("totalEmails").and_then(|v| v.as_u64()).unwrap_or(0);
        let unread = mb.get("unreadEmails").and_then(|v| v.as_u64()).unwrap_or(0);

        io.data(&format!(
            "{:<40} {:<20} {:<10} {:>6} {:>6}",
            id, name, role, total, unread
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::io::OutputMode;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};

    fn mock_ctx(mock: &MockJmap) -> Context {
        let (client, _) =
            crate::jmap::client::JmapClient::connect_to(&mock.session_url(), "fake-token")
                .expect("session connect");
        Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        }
    }

    fn captured(buffer: &std::sync::Arc<std::sync::Mutex<Vec<u8>>>) -> String {
        String::from_utf8(buffer.lock().expect("buffer lock").clone()).expect("utf8 output")
    }

    fn mailbox_list_response() -> serde_json::Value {
        serde_json::json!({
            "methodResponses": [["Mailbox/get", {
                "list": [
                    {
                        "id": "mb1", "name": "Inbox", "role": "inbox",
                        "totalEmails": 42, "unreadEmails": 3, "parentId": null,
                        "sortOrder": 1, "totalThreads": 40, "unreadThreads": 2,
                        "myRights": {"mayRead": true}, "isSubscribed": true
                    },
                    {
                        "id": "mb2", "name": "Sent", "role": "sent",
                        "totalEmails": 10, "unreadEmails": 0, "parentId": null,
                        "sortOrder": 2, "totalThreads": 9, "unreadThreads": 0,
                        "myRights": {"mayRead": true}, "isSubscribed": true
                    }
                ]
            }, "call-0"]]
        })
    }

    // --- Presenter golden tests (byte-identity net for CLI --json projection) ---
    //
    // In Json mode the CLI emits `io.json(value)` = `to_string_pretty(value)` + newline.
    // These pin the exact emitted bytes — list (projected + role-filtered) and manage
    // (create/rename/delete) — so relocating projection stays byte-identical.

    #[test]
    fn golden_list_json_projects_fields() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method("Mailbox/get", mailbox_list_response());

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(MailboxCommand::List { role: None }, &ctx, &io).expect("list should succeed");

        let expected = serde_json::json!([
            {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 42, "unreadEmails": 3, "parentId": null},
            {"id": "mb2", "name": "Sent", "role": "sent", "totalEmails": 10, "unreadEmails": 0, "parentId": null}
        ]);
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json mailbox list output bytes drifted"
        );
    }

    #[test]
    fn golden_list_json_filters_by_role() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method("Mailbox/get", mailbox_list_response());

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(
            MailboxCommand::List {
                role: Some("inbox".to_string()),
            },
            &ctx,
            &io,
        )
        .expect("list should succeed");

        let expected = serde_json::json!([
            {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 42, "unreadEmails": 3, "parentId": null}
        ]);
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json role-filtered mailbox list output bytes drifted"
        );
    }

    #[test]
    fn golden_create_json_returns_id() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "Mailbox/set",
            serde_json::json!({
                "methodResponses": [["Mailbox/set", {
                    "created": {"new-mailbox": {"id": "mbox-new"}}
                }, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(
            MailboxCommand::Create {
                name: "Projects".to_string(),
                parent_id: None,
            },
            &ctx,
            &io,
        )
        .expect("create should succeed");

        let expected = serde_json::json!({ "success": true, "mailboxId": "mbox-new" });
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json mailbox create output bytes drifted"
        );
    }

    #[test]
    fn golden_rename_json_returns_success() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "Mailbox/set",
            serde_json::json!({
                "methodResponses": [["Mailbox/set", {"updated": {"mb1": null}}, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(
            MailboxCommand::Rename {
                mailbox_id: "mb1".to_string(),
                new_name: "NewName".to_string(),
            },
            &ctx,
            &io,
        )
        .expect("rename should succeed");

        let expected = serde_json::json!({ "success": true });
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json mailbox rename output bytes drifted"
        );
    }

    #[test]
    fn golden_delete_json_returns_success() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "Mailbox/set",
            serde_json::json!({
                "methodResponses": [["Mailbox/set", {"destroyed": ["mb-del"]}, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(
            MailboxCommand::Delete {
                mailbox_id: "mb-del".to_string(),
            },
            &ctx,
            &io,
        )
        .expect("delete should succeed");

        let expected = serde_json::json!({ "success": true });
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json mailbox delete output bytes drifted"
        );
    }
}
