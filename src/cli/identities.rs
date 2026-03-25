use clap::Subcommand;

use crate::actions::Context;
use crate::cli::io::Io;
use crate::error::Result;

#[derive(Subcommand)]
pub enum IdentityCommand {
    /// List sending identities
    List,
}

pub fn run(cmd: IdentityCommand, _ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        IdentityCommand::List => {
            io.error("identities list: not yet implemented");
        }
    }
    Ok(())
}
