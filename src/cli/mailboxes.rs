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
    io.data(&format!("{}", "─".repeat(84)));

    for mb in mailboxes {
        let id = mb.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let name = mb.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let role = mb
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let total = mb
            .get("totalEmails")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let unread = mb
            .get("unreadEmails")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        io.data(&format!(
            "{:<40} {:<20} {:<10} {:>6} {:>6}",
            id, name, role, total, unread
        ));
    }
}
