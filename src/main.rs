#[macro_use]
mod logging;
mod actions;
mod error;
mod jmap;
mod mcp;
mod recorder;

use std::process;

fn main() {
    logging::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--tools" => cmd_tools(),
            "--version" => cmd_version(),
            "--help" | "-h" => cmd_help(),
            other => {
                eprintln!("unknown command: {other}");
                cmd_help();
                process::exit(1);
            }
        }
        return;
    }

    run_server();
}

fn run_server() {
    let token = match std::env::var("FASTMAIL_API_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            log_error!("main", "FASTMAIL_API_TOKEN environment variable not set");
            process::exit(1);
        }
    };

    let (client, session) = match jmap::client::JmapClient::connect(&token) {
        Ok(result) => result,
        Err(e) => {
            log_error!("main", "failed to connect to FastMail: {e}");
            process::exit(1);
        }
    };

    let account_id = match session.primary_account_id() {
        Some(id) => id.to_string(),
        None => {
            log_error!("main", "no primary account found in JMAP session");
            process::exit(1);
        }
    };

    log_info!("main", "connected as {} (account: {})", session.username, account_id);

    let recorder = recorder::Recorder::from_env();

    let ctx = actions::Context {
        jmap: client,
        account_id,
        recorder,
    };

    if let Err(e) = mcp::server::run(ctx) {
        log_error!("main", "server loop failed: {e}");
        process::exit(1);
    }
}

/// Print all tool definitions as JSON (for debugging/inspection).
fn cmd_tools() {
    let tools = actions::tool_definitions();
    let json = serde_json::to_string_pretty(&tools).unwrap_or_default();
    println!("{json}");
}

/// Print version.
fn cmd_version() {
    println!("fastermail {}", env!("CARGO_PKG_VERSION"));
}

/// Print usage help.
fn cmd_help() {
    eprintln!("fastermail {} — FastMail MCP server", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  fastermail              Run MCP server (stdio)");
    eprintln!("  fastermail --tools      Print tool definitions as JSON");
    eprintln!("  fastermail --version    Print version");
    eprintln!("  fastermail --help       Print this help");
    eprintln!();
    eprintln!("ENVIRONMENT:");
    eprintln!("  FASTMAIL_API_TOKEN      FastMail API token (required)");
    eprintln!("  JMAP_SESSION_URL        Override JMAP session URL (testing)");
    eprintln!("  FASTERMAIL_RECORD_DIR   Record request/response to directory");
    eprintln!("  FASTERMAIL_LOG          Log level: error|warn|info|debug|trace");
}
