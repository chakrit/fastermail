#![deny(warnings)]

#[macro_use]
mod logging;
mod actions;
mod cli;
mod config;
mod error;
mod jmap;
mod json;
mod mcp;
mod recorder;
#[cfg(test)]
mod testutil;

use std::fs;
use std::process;

use clap::Parser;

use crate::actions::Context;
use crate::cli::Cli;
use crate::cli::io::{Io, TerminalGuard};

fn main() {
    // Load .env then .env.local (local overrides base). Missing files are fine.
    load_dotenv(".env");
    load_dotenv(".env.local");

    logging::init();
    let _guard = TerminalGuard::new();

    let cli = Cli::parse();
    if let Err(e) = cli.run() {
        Io::error(&e.to_string());
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

/// Parse a dotenv file and set env vars. Skips missing files, blank lines, and comments.
/// Later files override earlier ones (call .env first, then .env.local).
fn load_dotenv(path: &str) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() {
                // SAFETY: called before any threads are spawned (top of main).
                unsafe { std::env::set_var(key, value) };
            }
        }
    }
}
