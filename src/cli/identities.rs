use crate::json;
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

            io.data(&format!("{:<40} {:<24} {}", "ID", "NAME", "EMAIL"));
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

    // --- Presenter golden test (byte-identity net for CLI --json projection) ---
    //
    // In Json mode the CLI emits `io.json(value)` = `to_string_pretty(value)` + newline.
    // This pins the exact emitted bytes so relocating projection stays byte-identical.

    #[test]
    fn golden_list_json_projects_fields() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "Identity/get",
            serde_json::json!({
                "methodResponses": [["Identity/get", {
                    "list": [
                        {
                            "id": "id1",
                            "name": "Alice",
                            "email": "alice@example.com",
                            "replyTo": null,
                            "bcc": null,
                            "textSignature": "sig1",
                            "htmlSignature": "<p>sig1</p>",
                            "mayDelete": true
                        },
                        {
                            "id": "id2",
                            "name": "Bob",
                            "email": "bob@example.com",
                            "replyTo": [{"email": "bob-reply@example.com"}],
                            "bcc": null,
                            "textSignature": "sig2",
                            "htmlSignature": "<p>sig2</p>",
                            "mayDelete": false
                        }
                    ]
                }, "call-0"]]
            }),
        );

        let (io, buffer) = Io::capturing(OutputMode::Json);
        run(IdentityCommand::List, &ctx, &io).expect("list should succeed");

        let expected = serde_json::json!([
            {
                "id": "id1",
                "name": "Alice",
                "email": "alice@example.com",
                "replyTo": null
            },
            {
                "id": "id2",
                "name": "Bob",
                "email": "bob@example.com",
                "replyTo": [{"email": "bob-reply@example.com"}]
            }
        ]);
        assert_eq!(
            captured(&buffer),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&expected).expect("pretty")
            ),
            "CLI --json identity list output bytes drifted"
        );
    }
}
