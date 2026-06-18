use std::io::{self, BufRead, Write};

use crate::actions::Context;
use crate::mcp::handler;
use crate::mcp::types::{
    JsonRpcRequest, JsonRpcResponse, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
};

/// Run the MCP server stdio loop.
pub fn run(ctx: Context) -> crate::error::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut initialized = false;

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        log_trace!("mcp", "recv: {line}");

        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                log_warn!("mcp", "parse error: {e}");
                let resp = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    PARSE_ERROR,
                    format!("parse error: {e}"),
                );
                write_response(&mut stdout, &resp, &ctx, "parse_error")?;
                continue;
            }
        };

        // Validate jsonrpc field is "2.0"
        if request.jsonrpc != "2.0" {
            log_warn!("mcp", "invalid jsonrpc version: {}", request.jsonrpc);
            let id = request.id.clone().unwrap_or(serde_json::Value::Null);
            let resp = JsonRpcResponse::error(
                id,
                INVALID_REQUEST,
                format!("invalid jsonrpc version: {}", request.jsonrpc),
            );
            write_response(&mut stdout, &resp, &ctx, "invalid_request")?;
            continue;
        }

        let method = request.method.clone();

        if request.id.is_none() {
            if let Some(rec) = &ctx.recorder {
                rec.record_mcp_request(&method, line);
            }
            log_debug!("mcp", "notification: {method}");
            handle_notification(&method, &mut initialized);
            continue;
        }

        if let Some(rec) = &ctx.recorder {
            rec.record_mcp_request(&method, line);
        }

        log_debug!("mcp", "request: {method}");

        let id = request.id.clone().unwrap_or(serde_json::Value::Null);
        let response = handle_request(&method, request.params, id, &ctx, &mut initialized);

        write_response(&mut stdout, &response, &ctx, &method)?;
    }

    log_info!("mcp", "stdin closed, shutting down");
    Ok(())
}

fn handle_request(
    method: &str,
    params: serde_json::Value,
    id: serde_json::Value,
    ctx: &Context,
    initialized: &mut bool,
) -> JsonRpcResponse {
    // Before initialization, only allow initialize and ping
    if !*initialized && method != "initialize" && method != "ping" {
        log_warn!("mcp", "method called before initialization: {method}");
        return JsonRpcResponse::error(
            id,
            METHOD_NOT_FOUND,
            "server not initialized, call initialize first".to_string(),
        );
    }

    match method {
        "initialize" => {
            let result = handler::handle_initialize(params);
            *initialized = true;
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
        _ => {
            log_warn!("mcp", "unknown method: {method}");
            JsonRpcResponse::error(id, METHOD_NOT_FOUND, format!("method not found: {method}"))
        }
    }
}

fn handle_notification(method: &str, initialized: &mut bool) {
    match method {
        "notifications/initialized" => {
            *initialized = true;
            log_info!("mcp", "client initialized");
        }
        _ => {
            log_warn!("mcp", "unknown notification: {method}");
        }
    }
}

fn write_response(
    stdout: &mut io::Stdout,
    response: &JsonRpcResponse,
    ctx: &Context,
    method: &str,
) -> io::Result<()> {
    let json = serde_json::to_string(response).map_err(|e| {
        io::Error::other(format!("serialize error: {e}"))
    })?;

    log_trace!("mcp", "send: {json}");

    if let Some(rec) = &ctx.recorder {
        rec.record_mcp_response(method, &json);
    }

    writeln!(stdout, "{json}")?;
    stdout.flush()
}
