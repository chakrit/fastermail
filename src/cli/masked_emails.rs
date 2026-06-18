use clap::Subcommand;

use crate::actions::masked_email::{CreateMaskedEmail, ListMaskedEmails, UpdateMaskedEmail};
use crate::actions::{Action, Context};
use crate::cli::io::{Io, OutputMode};
use crate::error::Result;

#[derive(Subcommand)]
pub enum MaskedEmailCommand {
    /// List masked email addresses
    List {
        /// Filter: pending, enabled, disabled, deleted
        #[arg(long)]
        state: Option<String>,
    },

    /// Create a new masked email address
    Create {
        /// Domain this address is for
        #[arg(long)]
        domain: Option<String>,

        /// Human-readable label
        #[arg(long)]
        description: Option<String>,

        /// Preferred prefix for the address
        #[arg(long)]
        prefix: Option<String>,
    },

    /// Enable/disable/delete a masked email
    Update {
        /// Masked email ID
        id: String,

        /// New state: enabled, disabled, or deleted
        #[arg(long)]
        state: String,
    },
}

pub fn run(cmd: MaskedEmailCommand, ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        MaskedEmailCommand::List { state } => {
            let spinner = io.progress("Fetching masked emails…");
            let action = ListMaskedEmails {
                state: state.unwrap_or_default(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;

            if io.mode() != OutputMode::Human {
                io.json(&value);
                return Ok(());
            }

            let items = match value.as_array() {
                Some(arr) => arr,
                None => {
                    io.json(&value);
                    return Ok(());
                }
            };

            if items.is_empty() {
                io.warn("No masked emails found");
                return Ok(());
            }

            io.done(&format!("{} masked email(s)", items.len()));
            io.separator();

            io.data(&format!(
                "{:<40} {:<32} {:<16} {:<10} {}",
                "ID", "EMAIL", "DOMAIN", "STATE", "DESCRIPTION"
            ));
            io.data(&"─".repeat(110));

            for item in items {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let email = item
                    .get("email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let domain = item
                    .get("forDomain")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let state = item
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let desc = item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                io.data(&format!(
                    "{:<40} {:<32} {:<16} {:<10} {}",
                    id, email, domain, state, desc
                ));
            }
        }
        MaskedEmailCommand::Create {
            domain,
            description,
            prefix,
        } => {
            let spinner = io.progress("Creating masked email…");
            let action = CreateMaskedEmail {
                for_domain: domain.unwrap_or_default(),
                description: description.unwrap_or_default(),
                email_prefix: prefix.unwrap_or_default(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;

            if io.mode() == OutputMode::Human {
                let email = value
                    .get("email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let id = value.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                io.done(&format!("Created: {email} (ID: {id})"));
            } else {
                io.json(&value);
            }
        }
        MaskedEmailCommand::Update { id, state } => {
            let spinner = io.progress("Updating masked email…");
            let action = UpdateMaskedEmail {
                id,
                state: state.clone(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            result?;

            if io.mode() == OutputMode::Human {
                io.done(&format!("State updated to: {state}"));
            } else {
                io.json(&serde_json::json!({ "success": true }));
            }
        }
    }
    Ok(())
}
