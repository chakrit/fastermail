use clap::Subcommand;

use crate::actions::Context;
use crate::cli::io::Io;
use crate::error::Result;

#[derive(Subcommand)]
pub enum VacationCommand {
    /// Get vacation/auto-reply settings
    Get,

    /// Enable/disable/update vacation auto-reply
    Set {
        /// Enable auto-reply
        #[arg(long, conflicts_with = "disabled")]
        enabled: bool,

        /// Disable auto-reply
        #[arg(long, conflicts_with = "enabled")]
        disabled: bool,

        /// Start date (ISO 8601)
        #[arg(long)]
        from: Option<String>,

        /// End date (ISO 8601)
        #[arg(long)]
        to: Option<String>,

        /// Auto-reply subject
        #[arg(long)]
        subject: Option<String>,

        /// Plain text body
        #[arg(long)]
        text_body: Option<String>,

        /// HTML body
        #[arg(long)]
        html_body: Option<String>,
    },
}

pub fn run(cmd: VacationCommand, _ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        VacationCommand::Get => {
            io.error("vacation get: not yet implemented");
        }
        VacationCommand::Set { .. } => {
            io.error("vacation set: not yet implemented");
        }
    }
    Ok(())
}
