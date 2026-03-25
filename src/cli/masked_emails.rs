use clap::Subcommand;

use crate::actions::Context;
use crate::cli::io::Io;
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

pub fn run(cmd: MaskedEmailCommand, _ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        MaskedEmailCommand::List { state } => {
            let _ = state;
            io.error("masked-emails list: not yet implemented");
        }
        MaskedEmailCommand::Create { domain, description, prefix } => {
            let _ = (domain, description, prefix);
            io.error("masked-emails create: not yet implemented");
        }
        MaskedEmailCommand::Update { id, state } => {
            let _ = (id, state);
            io.error("masked-emails update: not yet implemented");
        }
    }
    Ok(())
}
