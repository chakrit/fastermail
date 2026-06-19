use clap::Subcommand;

use crate::actions::vacation::{FieldChange, GetVacationResponse, SetVacationResponse};
use crate::actions::{Action, Context};
use crate::cli::io::{Io, OutputMode};
use crate::error::{Error, Result};

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

pub fn run(cmd: VacationCommand, ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        VacationCommand::Get => {
            let spinner = io.progress("Fetching vacation settings…");
            let action = GetVacationResponse;
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;

            if io.mode() != OutputMode::Human {
                io.json(&value);
                return Ok(());
            }

            let enabled = value
                .get("isEnabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let status = if enabled {
                console::style("enabled").green().to_string()
            } else {
                console::style("disabled").dim().to_string()
            };

            io.done(&format!("Vacation auto-reply: {status}"));
            io.separator();

            for (key, label) in [
                ("fromDate", "From"),
                ("toDate", "To"),
                ("subject", "Subject"),
                ("textBody", "Text body"),
            ] {
                if let Some(v) = value.get(key).and_then(|v| v.as_str())
                    && !v.is_empty()
                {
                    io.data(&format!(
                        "{} {}",
                        console::style(format!("{label}:")).bold(),
                        v
                    ));
                }
            }
        }
        VacationCommand::Set {
            enabled,
            disabled,
            from,
            to,
            subject,
            text_body,
            html_body,
        } => {
            if !enabled && !disabled {
                return Err(Error::InvalidParams(
                    "must specify --enabled or --disabled".to_string(),
                ));
            }

            let is_enabled = enabled; // if disabled is true, enabled is false

            let spinner = io.progress("Updating vacation settings…");
            let action = SetVacationResponse {
                is_enabled: Some(is_enabled),
                from_date: FieldChange::from_opt(from),
                to_date: FieldChange::from_opt(to),
                subject: FieldChange::from_opt(subject),
                text_body: FieldChange::from_opt(text_body),
                html_body: FieldChange::from_opt(html_body),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            result?;

            if io.mode() == OutputMode::Human {
                let state = if is_enabled { "enabled" } else { "disabled" };
                io.done(&format!("Vacation auto-reply {state}"));
            } else {
                io.json(&serde_json::json!({ "success": true }));
            }
        }
    }
    Ok(())
}
