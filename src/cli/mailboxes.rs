use clap::Subcommand;

use crate::actions::Context;
use crate::cli::io::Io;
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

pub fn run(cmd: MailboxCommand, _ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        MailboxCommand::List { role } => {
            let _ = role;
            io.error("mailboxes list: not yet implemented");
        }
        MailboxCommand::Create { name, parent_id } => {
            let _ = (name, parent_id);
            io.error("mailboxes create: not yet implemented");
        }
        MailboxCommand::Rename { mailbox_id, new_name } => {
            let _ = (mailbox_id, new_name);
            io.error("mailboxes rename: not yet implemented");
        }
        MailboxCommand::Delete { mailbox_id } => {
            let _ = mailbox_id;
            io.error("mailboxes delete: not yet implemented");
        }
    }
    Ok(())
}
