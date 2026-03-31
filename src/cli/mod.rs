pub mod contacts;
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
use crate::config;

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

    /// Manage contacts
    Contacts {
        #[command(subcommand)]
        action: contacts::ContactCommand,
    },

    /// Print current configuration
    Config,

    /// Interactive first-time setup
    Setup,

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
            Some(Command::Config) => {
                let io = Io::new(OutputMode::detect(self.json, self.raw));
                Self::print_config(&io)
            }
            Some(Command::Setup) => {
                let io = Io::new(OutputMode::detect(self.json, self.raw));
                Self::run_setup(&io)
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
            Command::Mcp | Command::Config | Command::Setup => unreachable!(),
            Command::Contacts { action } => contacts::run(action, ctx, io),

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

    fn print_config(io: &Io) -> crate::error::Result<()> {
        let path = config::config_path();
        let path_str = path.display().to_string();
        let file_exists = path.exists();

        // Resolve token to show source
        let (token_display, source_display) = match config::resolve_token() {
            Ok((token, source)) => {
                let masked = mask_token(&token);
                let source_str = match source {
                    config::TokenSource::EnvVar => "FASTMAIL_API_TOKEN env var",
                    config::TokenSource::ConfigFile => "config file",
                };
                (masked, source_str.to_string())
            }
            Err(_) => ("(not set)".to_string(), "none".to_string()),
        };

        if io.mode() != OutputMode::Human {
            let obj = serde_json::json!({
                "config_path": path_str,
                "config_exists": file_exists,
                "token_source": source_display,
                "token": token_display,
            });
            io.json(&obj);
        } else {
            io.data(&format!(
                "{} {}",
                console::style("Config file:").bold(),
                if file_exists {
                    &path_str
                } else {
                    "(not created)"
                }
            ));
            io.data(&format!(
                "{}     {}",
                console::style("Token from:").bold(),
                source_display
            ));
            io.data(&format!(
                "{}         {}",
                console::style("Token:").bold(),
                token_display
            ));
        }
        Ok(())
    }

    fn run_setup(io: &Io) -> crate::error::Result<()> {
        if io.mode() != OutputMode::Human {
            return Err(crate::error::Error::InvalidParams(
                "setup requires an interactive terminal".to_string(),
            ));
        }

        let path = config::config_path();
        io.data(&format!(
            "Config will be saved to: {}",
            console::style(path.display()).dim()
        ));

        // Check if config already exists with a token
        if let Ok((existing_token, config::TokenSource::ConfigFile)) = config::resolve_token() {
            if !existing_token.is_empty() {
                io.warn(&format!(
                    "Config file already has a token ({})",
                    mask_token(&existing_token)
                ));
                let confirm = inquire::Confirm::new("Overwrite existing token?")
                    .with_default(false)
                    .prompt()
                    .map_err(|e| {
                        crate::error::Error::InvalidParams(format!("prompt cancelled: {e}"))
                    })?;
                if !confirm {
                    io.hint("Setup cancelled");
                    return Ok(());
                }
            }
        }

        io.hint("Create a FastMail API token at: https://app.fastmail.com/settings/security/tokens");
        io.hint("Required scope: JMAP access");
        io.separator();

        let token = inquire::Text::new("API Token:")
            .with_help_message("Paste your FastMail API token (starts with fmu1-)")
            .prompt()
            .map_err(|e| {
                crate::error::Error::InvalidParams(format!("prompt cancelled: {e}"))
            })?;

        let token = token.trim().to_string();
        if token.is_empty() {
            return Err(crate::error::Error::InvalidParams(
                "token cannot be empty".to_string(),
            ));
        }

        let written_path = config::write_config(&token)?;
        io.done(&format!("Saved to {} (permissions: 0600)", written_path.display()));

        // Verify connection
        io.separator();
        let spinner = io.progress("Verifying connection...");
        match crate::connect() {
            Ok(_ctx) => {
                Io::finish_progress(spinner);
                io.done("Connected to FastMail successfully");
            }
            Err(e) => {
                Io::finish_progress(spinner);
                io.warn(&format!("Token saved but connection failed: {e}"));
                io.hint("Check your token and try again");
            }
        }

        Ok(())
    }
}

/// Mask a token for display: show first 8 chars + "..." + last 4 chars.
fn mask_token(token: &str) -> String {
    if token.len() <= 12 {
        return "*".repeat(token.len());
    }
    format!("{}...{}", &token[..8], &token[token.len() - 4..])
}
