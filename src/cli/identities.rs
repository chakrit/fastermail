use clap::Subcommand;

use crate::actions::identity::ListIdentities;
use crate::actions::{Action, Context};
use crate::cli::io::{Io, OutputMode};
use crate::error::Result;

#[derive(Subcommand)]
pub enum IdentityCommand {
    /// List sending identities
    List,
}

pub fn run(cmd: IdentityCommand, ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        IdentityCommand::List => {
            let spinner = io.progress("Fetching identities…");
            let action = ListIdentities;
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;

            if io.mode() != OutputMode::Human {
                io.json(&value);
                return Ok(());
            }

            let identities = match value.as_array() {
                Some(arr) => arr,
                None => {
                    io.json(&value);
                    return Ok(());
                }
            };

            if identities.is_empty() {
                io.warn("No identities found");
                return Ok(());
            }

            io.done(&format!("{} identity(ies)", identities.len()));
            io.separator();

            io.data(&format!(
                "{:<40} {:<24} {}",
                "ID", "NAME", "EMAIL"
            ));
            io.data(&"─".repeat(80));

            for ident in identities {
                let id = ident.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let name = ident.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let email = ident.get("email").and_then(|v| v.as_str()).unwrap_or("");

                io.data(&format!("{:<40} {:<24} {}", id, name, email));
            }
        }
    }
    Ok(())
}
