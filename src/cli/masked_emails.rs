use crate::json;
use clap::Subcommand;

use crate::actions::masked_email::{CreateMaskedEmail, ListMaskedEmails, UpdateMaskedEmail};
use crate::actions::{Action, Context};
use crate::cli::io::{Io, OutputMode};
use crate::error::Result;
use crate::present::{self, MaskedEmailState};

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
            let state = state.map(|s| MaskedEmailState::parse(&s)).transpose()?;
            let spinner = io.progress("Fetching masked emails…");
            let action = ListMaskedEmails;
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = present::project_masked_email_list(&result?, state);

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
                let id = json::str_at(item, "/id").unwrap_or("?");
                let email = item.get("email").and_then(|v| v.as_str()).unwrap_or("");
                let domain = item.get("forDomain").and_then(|v| v.as_str()).unwrap_or("");
                let state = item.get("state").and_then(|v| v.as_str()).unwrap_or("");
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
            let value = present::project_masked_email_create(&result?);

            if io.mode() == OutputMode::Human {
                let email = json::str_at(&value, "/email").unwrap_or("?");
                let id = json::str_at(&value, "/id").unwrap_or("?");
                io.done(&format!("Created: {email} (ID: {id})"));
            } else {
                io.json(&value);
            }
        }
        MaskedEmailCommand::Update { id, state } => {
            let parsed_state = MaskedEmailState::parse_settable(&state)?;
            let spinner = io.progress("Updating masked email…");
            let action = UpdateMaskedEmail {
                id,
                state: parsed_state,
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            result?;

            if io.mode() == OutputMode::Human {
                io.done(&format!("State updated to: {state}"));
            } else {
                io.json(&present::set_ok());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::io::OutputMode;
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
    // These pin the exact emitted bytes — list (full projection), create (the asymmetric
    // {id,email}-only projection), and update ({success}) — so relocating projection
    // stays byte-identical.

    #[test]
    fn golden_list_json_projects_fields() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "MaskedEmail/get",
            serde_json::json!({
                "methodResponses": [["MaskedEmail/get", {
                    "list": [
                        {
                            "id": "me1",
                            "email": "abc@fastmail.com",
                            "forDomain": "example.com",
                            "description": "Test",
                            "state": "enabled",
                            "createdAt": "2026-01-01",
                            "lastMessageAt": "2026-03-01",
                            "url": "https://example.com"
                        },
                        {
                            "id": "me2",
                            "email": "def@fastmail.com",
                            "forDomain": "other.com",
                            "description": "Second",
                            "state": "disabled",
                            "createdAt": "2026-02-01",
                            "lastMessageAt": null
                        }
                    ]
                }, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(MaskedEmailCommand::List { state: None }, &ctx, &io).expect("list should succeed");

        let expected = serde_json::json!([
            {
                "id": "me1",
                "email": "abc@fastmail.com",
                "forDomain": "example.com",
                "description": "Test",
                "state": "enabled",
                "createdAt": "2026-01-01"
            },
            {
                "id": "me2",
                "email": "def@fastmail.com",
                "forDomain": "other.com",
                "description": "Second",
                "state": "disabled",
                "createdAt": "2026-02-01"
            }
        ]);
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json masked email list output bytes drifted"
        );
    }

    #[test]
    fn golden_create_json_projects_id_and_email_only() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "MaskedEmail/set",
            serde_json::json!({
                "methodResponses": [["MaskedEmail/set", {
                    "created": {
                        "new-masked": {
                            "id": "me-new",
                            "email": "new@fastmail.com",
                            "state": "enabled",
                            "forDomain": "mysite.com",
                            "description": "My site login",
                            "createdAt": "2026-06-25"
                        }
                    }
                }, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(
            MaskedEmailCommand::Create {
                domain: Some("mysite.com".to_string()),
                description: Some("My site login".to_string()),
                prefix: None,
            },
            &ctx,
            &io,
        )
        .expect("create should succeed");

        // Create is asymmetric vs list: only {id, email} survive (state/forDomain/
        // description/createdAt the server returns are projected out).
        let expected = serde_json::json!({
            "id": "me-new",
            "email": "new@fastmail.com"
        });
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json masked email create output bytes drifted"
        );
    }

    #[test]
    fn golden_update_json_returns_success() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "MaskedEmail/set",
            serde_json::json!({
                "methodResponses": [["MaskedEmail/set", {"updated": {"me1": null}}, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(
            MaskedEmailCommand::Update {
                id: "me1".to_string(),
                state: "disabled".to_string(),
            },
            &ctx,
            &io,
        )
        .expect("update should succeed");

        let expected = serde_json::json!({ "success": true });
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json masked email update output bytes drifted"
        );
    }
}
