pub mod emails;
pub mod identities;
pub mod io;
pub mod mailboxes;
pub mod masked_emails;
pub mod resolve;
pub mod vacation;

use clap::{Parser, Subcommand};

use crate::actions::Context;
use crate::cli::io::{Io, OutputMode};

/// FasterMail — FastMail CLI & MCP server
#[derive(Parser)]
#[command(name = "fm", version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Output as JSON (default when stdout is not a TTY)
    #[arg(long, global = true)]
    json: bool,

    /// Output raw JMAP response (debug)
    #[arg(long, global = true)]
    raw: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Run MCP server (stdio JSON-RPC)
    Mcp,

    /// Manage emails
    Emails {
        #[command(subcommand)]
        action: emails::EmailCommand,
    },

    /// Manage mailboxes
    Mailboxes {
        #[command(subcommand)]
        action: mailboxes::MailboxCommand,
    },

    /// List sending identities
    Identities {
        #[command(subcommand)]
        action: identities::IdentityCommand,
    },

    /// Manage vacation auto-reply
    Vacation {
        #[command(subcommand)]
        action: vacation::VacationCommand,
    },

    /// Manage masked email addresses
    #[command(name = "masked-emails")]
    MaskedEmails {
        #[command(subcommand)]
        action: masked_emails::MaskedEmailCommand,
    },

    // -- Top-level shortcuts for triage workflow --
    /// List emails (shortcut for `emails list`)
    Ls {
        /// Mailbox (ID, role alias, or name; defaults to inbox)
        mailbox: Option<String>,

        /// Max results
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: u32,

        /// Include body content
        #[arg(long)]
        include_body: bool,
    },

    /// Move emails (shortcut for `emails move`)
    Mv {
        /// Email IDs to move
        #[arg(required = true, num_args = 1..)]
        email_ids: Vec<String>,

        /// Target mailbox (ID, role alias, or name) — last positional arg
        #[arg(required = true)]
        mailbox: String,
    },

    /// Read an email (shortcut for `emails get`)
    Read {
        /// Email ID
        email_id: String,

        /// Body format: text, html, or both
        #[arg(long, default_value = "text")]
        format: String,
    },
}

impl Cli {
    pub fn run(self) -> crate::error::Result<()> {
        match self.command {
            None => {
                // No subcommand → show help via clap
                use clap::CommandFactory;
                let mut cmd = Self::command();
                cmd.print_help().map_err(crate::error::Error::Io)?;
                Ok(())
            }
            Some(Command::Mcp) => {
                crate::run_mcp_server()
            }
            Some(cmd) => {
                let io = Io::new(OutputMode::detect(self.json, self.raw));
                let ctx = crate::connect()?;
                Self::dispatch(cmd, &ctx, &io)
            }
        }
    }

    fn dispatch(cmd: Command, ctx: &Context, io: &Io) -> crate::error::Result<()> {
        match cmd {
            Command::Mcp => unreachable!(),

            Command::Emails { action } => emails::run(action, ctx, io),
            Command::Mailboxes { action } => mailboxes::run(action, ctx, io),
            Command::Identities { action } => identities::run(action, ctx, io),
            Command::Vacation { action } => vacation::run(action, ctx, io),
            Command::MaskedEmails { action } => masked_emails::run(action, ctx, io),

            // Top-level shortcuts delegate to the same handlers
            Command::Ls {
                mailbox,
                limit,
                include_body,
            } => emails::run(
                emails::EmailCommand::List {
                    mailbox: mailbox.or_else(|| Some("inbox".to_string())),
                    limit,
                    include_body,
                },
                ctx,
                io,
            ),
            Command::Mv {
                email_ids,
                mailbox,
            } => emails::run(
                emails::EmailCommand::Move {
                    email_ids,
                    to: mailbox,
                },
                ctx,
                io,
            ),
            Command::Read { email_id, format } => emails::run(
                emails::EmailCommand::Get { email_id, format },
                ctx,
                io,
            ),
        }
    }
}
