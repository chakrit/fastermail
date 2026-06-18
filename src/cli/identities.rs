use clap::Subcommand;
use crate::json;

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
                let id = json::str_at(ident, "/id").unwrap_or("?");
                let name = json::str_at(ident, "/name").unwrap_or("");
                let email = json::str_at(ident, "/email").unwrap_or("");

                io.data(&format!("{:<40} {:<24} {}", id, name, email));
            }
        }
    }
    Ok(())
}
