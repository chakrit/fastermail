use clap::Subcommand;

use crate::actions::Context;
use crate::cli::io::Io;
use crate::error::Result;

#[derive(Subcommand)]
pub enum EmailCommand {
    /// List emails from a mailbox
    List {
        /// Mailbox (ID, role alias, or name)
        #[arg(short = 'm', long)]
        mailbox: Option<String>,

        /// Max results
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: u32,

        /// Include body content
        #[arg(long)]
        include_body: bool,
    },

    /// Search emails with filters
    Search {
        /// Full-text search
        #[arg(short = 'q', long)]
        keyword: Option<String>,

        /// Sender address filter
        #[arg(long)]
        from: Option<String>,

        /// Recipient address filter
        #[arg(long)]
        to: Option<String>,

        /// Subject filter
        #[arg(long)]
        subject: Option<String>,

        /// Restrict to mailbox (ID, role alias, or name)
        #[arg(short = 'm', long)]
        mailbox: Option<String>,

        /// Filter for emails with attachments
        #[arg(long)]
        has_attachment: bool,

        /// Date lower bound (YYYY-MM-DD)
        #[arg(long)]
        after: Option<String>,

        /// Date upper bound (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,

        /// Max results
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: u32,

        /// Include body content
        #[arg(long)]
        include_body: bool,
    },

    /// Get full body of a single email
    Get {
        /// Email ID
        email_id: String,

        /// Body format: text, html, or both
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Move emails between mailboxes
    Move {
        /// Email IDs to move
        #[arg(required = true)]
        email_ids: Vec<String>,

        /// Target mailbox (ID, role alias, or name)
        #[arg(long)]
        to: String,
    },

    /// Set/unset flags on emails
    Flag {
        /// Email IDs
        #[arg(required = true)]
        email_ids: Vec<String>,

        /// Flag: seen, flagged, answered, or draft
        #[arg(long)]
        flag: String,

        /// Unset the flag (default: set)
        #[arg(long)]
        unset: bool,
    },

    /// Delete emails
    Delete {
        /// Email IDs to delete
        #[arg(required = true)]
        email_ids: Vec<String>,

        /// Permanently delete (skip trash)
        #[arg(long)]
        permanent: bool,
    },

    /// Compose and send an email
    Send {
        /// Recipient (repeatable)
        #[arg(long, required = true)]
        to: Vec<String>,

        /// Subject line
        #[arg(long)]
        subject: String,

        /// Body text (reads from stdin if omitted)
        #[arg(long)]
        body: Option<String>,

        /// CC recipient (repeatable)
        #[arg(long)]
        cc: Vec<String>,

        /// BCC recipient (repeatable)
        #[arg(long)]
        bcc: Vec<String>,

        /// Body is HTML
        #[arg(long)]
        html: bool,

        /// Email ID being replied to
        #[arg(long)]
        reply_to: Option<String>,
    },
}

pub fn run(cmd: EmailCommand, _ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        EmailCommand::List { ref mailbox, limit, include_body } => {
            let _ = (mailbox, limit, include_body);
            io.error("emails list: not yet implemented");
        }
        EmailCommand::Search { .. } => {
            io.error("emails search: not yet implemented");
        }
        EmailCommand::Get { email_id, format } => {
            let _ = (email_id, format);
            io.error("emails get: not yet implemented");
        }
        EmailCommand::Move { email_ids, to } => {
            let _ = (email_ids, to);
            io.error("emails move: not yet implemented");
        }
        EmailCommand::Flag { email_ids, flag, unset } => {
            let _ = (email_ids, flag, unset);
            io.error("emails flag: not yet implemented");
        }
        EmailCommand::Delete { email_ids, permanent } => {
            let _ = (email_ids, permanent);
            io.error("emails delete: not yet implemented");
        }
        EmailCommand::Send { .. } => {
            io.error("emails send: not yet implemented");
        }
    }
    Ok(())
}
