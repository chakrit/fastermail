mod actions;
mod error;
mod jmap;
mod mcp;
mod recorder;

use std::process;

fn main() {
    let token = match std::env::var("FASTMAIL_API_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            eprintln!("error: FASTMAIL_API_TOKEN environment variable not set");
            process::exit(1);
        }
    };

    let (client, session) = match jmap::client::JmapClient::connect(&token) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("error: failed to connect to FastMail: {e}");
            process::exit(1);
        }
    };

    let account_id = match session.primary_account_id() {
        Some(id) => id.to_string(),
        None => {
            eprintln!("error: no primary account found in JMAP session");
            process::exit(1);
        }
    };

    eprintln!("[fastermail] connected as {} (account: {})", session.username, account_id);

    let recorder = recorder::Recorder::from_env();

    let ctx = actions::Context {
        jmap: client,
        account_id,
        recorder,
    };

    if let Err(e) = mcp::server::run(ctx) {
        eprintln!("error: server loop failed: {e}");
        process::exit(1);
    }
}
