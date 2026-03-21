use std::io::{self, BufRead, Write};

use crate::actions::Context;
use crate::mcp::handler;
use crate::mcp::types::*;

/// Run the MCP server stdio loop.
pub fn run(ctx: Context) -> crate::error::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                let resp = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    PARSE_ERROR,
                    format!("parse error: {e}"),
                );
                write_response(&mut stdout, &resp)?;
                continue;
            }
        };

        if request.id.is_none() {
            handle_notification(&request.method);
            continue;
        }

        let id = request.id.clone().unwrap_or(serde_json::Value::Null);
        let response = handle_request(&request.method, request.params, id.clone(), &ctx);

        write_response(&mut stdout, &response)?;
    }

    Ok(())
}

fn handle_request(
    method: &str,
    params: serde_json::Value,
    id: serde_json::Value,
    ctx: &Context,
) -> JsonRpcResponse {
    match method {
        "initialize" => {
            let result = handler::handle_initialize(params);
            JsonRpcResponse::success(id, result)
        }
        "ping" => JsonRpcResponse::success(id, serde_json::json!({})),
        "tools/list" => {
            let result = handler::handle_tools_list();
            JsonRpcResponse::success(id, result)
        }
        "tools/call" => {
            let result = handler::handle_tools_call(params, ctx);
            JsonRpcResponse::success(id, result)
        }
        _ => JsonRpcResponse::error(id, METHOD_NOT_FOUND, format!("method not found: {method}")),
    }
}

fn handle_notification(method: &str) {
    match method {
        "notifications/initialized" => {
            eprintln!("[fastermail] client initialized");
        }
        _ => {
            eprintln!("[fastermail] unknown notification: {method}");
        }
    }
}

fn write_response(stdout: &mut io::Stdout, response: &JsonRpcResponse) -> io::Result<()> {
    let json = serde_json::to_string(response).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("serialize error: {e}"))
    })?;

    writeln!(stdout, "{json}")?;
    stdout.flush()
}
