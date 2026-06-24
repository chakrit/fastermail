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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};

    fn mock_ctx(mock: &MockJmap) -> Context {
        let (client, _) =
            crate::jmap::client::JmapClient::connect_to(&mock.session_url(), "fake-token")
                .expect("session connect");
        Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        }
    }

    fn captured(buffer: &std::sync::Arc<std::sync::Mutex<Vec<u8>>>) -> String {
        String::from_utf8(buffer.lock().expect("buffer lock").clone()).expect("utf8 output")
    }

    // --- Presenter golden tests (byte-identity net for CLI --json projection) ---
    //
    // In Json mode the CLI emits `io.json(value)` = `to_string_pretty(value)` + newline.
    // These pin the exact emitted bytes — get (projected fields) and set (success) — so
    // relocating projection stays byte-identical.

    #[test]
    fn golden_get_json_projects_fields() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "VacationResponse/get",
            serde_json::json!({
                "methodResponses": [["VacationResponse/get", {
                    "list": [{
                        "id": "singleton",
                        "isEnabled": true,
                        "fromDate": "2026-01-01T00:00:00Z",
                        "toDate": "2026-01-15T00:00:00Z",
                        "subject": "OOO",
                        "textBody": "Away",
                        "htmlBody": "<p>Away</p>"
                    }]
                }, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(VacationCommand::Get, &ctx, &io).expect("get should succeed");

        let expected = serde_json::json!({
            "isEnabled": true,
            "fromDate": "2026-01-01T00:00:00Z",
            "toDate": "2026-01-15T00:00:00Z",
            "subject": "OOO",
            "textBody": "Away",
            "htmlBody": "<p>Away</p>"
        });
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json vacation get output bytes drifted"
        );
    }

    #[test]
    fn golden_set_json_returns_success() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "VacationResponse/set",
            serde_json::json!({
                "methodResponses": [["VacationResponse/set", {
                    "updated": {"singleton": null}
                }, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(
            VacationCommand::Set {
                enabled: true,
                disabled: false,
                from: None,
                to: None,
                subject: Some("On vacation".to_string()),
                text_body: None,
                html_body: None,
            },
            &ctx,
            &io,
        )
        .expect("set should succeed");

        let expected = serde_json::json!({ "success": true });
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json vacation set output bytes drifted"
        );
    }
}
