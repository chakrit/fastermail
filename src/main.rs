#[macro_use]
mod logging;
mod actions;
mod cli;
mod config;
mod error;
mod jmap;
mod mcp;
mod recorder;

use std::process;

use clap::Parser;

use crate::actions::Context;
use crate::cli::Cli;

fn main() {
    // Load .env then .env.local (local overrides base). Missing files are fine.
    let _ = dotenvy::from_filename(".env");
    let _ = dotenvy::from_filename(".env.local");

    logging::init();

    let cli = Cli::parse();
    if let Err(e) = cli.run() {
        eprintln!("{} {e}", console::style("✗").red());
        process::exit(exit_code(&e));
    }
}

/// Map error types to exit codes per spec:
/// 1 = startup error, 2 = invalid arguments, 3 = API error.
fn exit_code(e: &error::Error) -> i32 {
    match e {
        error::Error::MissingToken => 1,
        error::Error::InvalidParams(_) => 2,
        error::Error::Jmap { .. } => 3,
        error::Error::Http(_) => 3,
        error::Error::Io(_) | error::Error::Json(_) => 1,
    }
}

/// Connect to FastMail and build a Context. Shared by CLI commands and MCP mode.
fn connect() -> error::Result<Context> {
    let (token, _source) = config::resolve_token()?;

    let (client, session) = jmap::client::JmapClient::connect(&token)?;

    let account_id = session
        .primary_account_id()
        .map(String::from)
        .ok_or_else(|| error::Error::Jmap {
            method: "session".to_string(),
            message: "no primary account found in JMAP session".to_string(),
        })?;

    log_info!("main", "connected as {} (account: {})", session.username, account_id);

    let recorder = recorder::Recorder::from_env();

    Ok(Context {
        jmap: client,
        account_id,
        recorder,
    })
}

/// Run the MCP stdio server.
fn run_mcp_server() -> error::Result<()> {
    let ctx = connect()?;
    mcp::server::run(ctx)
}
