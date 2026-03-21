use crate::actions::{self, email, identity, mailbox, masked_email, vacation, Action, Context};
use crate::error::Error;
use crate::mcp::types::{
    InitializeParams, InitializeResult, ServerCapabilities, ServerInfo, ToolCallParams,
    ToolCallResult, ToolsCapability, ToolsListResult,
};

pub fn handle_initialize(params: serde_json::Value) -> serde_json::Value {
    let _init: InitializeParams = serde_json::from_value(params).unwrap_or_default();

    let result = InitializeResult {
        protocol_version: "2025-11-25".to_string(),
        capabilities: ServerCapabilities {
            tools: ToolsCapability {
                list_changed: false,
            },
        },
        server_info: ServerInfo {
            name: "fastermail".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    serde_json::to_value(result).unwrap_or(serde_json::json!({}))
}

pub fn handle_tools_list() -> serde_json::Value {
    let result = ToolsListResult {
        tools: actions::tool_definitions(),
    };

    serde_json::to_value(result).unwrap_or(serde_json::json!({}))
}

pub fn handle_tools_call(params: serde_json::Value, ctx: &Context) -> serde_json::Value {
    let call: ToolCallParams = serde_json::from_value(params).unwrap_or_default();
    let args = &call.arguments;

    log_debug!("handler", "tool call: {}", call.name);
    log_trace!("handler", "tool args: {}", call.arguments);

    let result = dispatch_tool(&call.name, args, ctx);

    let response = match &result {
        Ok(data) => {
            log_debug!("handler", "tool {} succeeded", call.name);
            let text = serde_json::to_string_pretty(data).unwrap_or_default();
            serde_json::to_value(ToolCallResult::text(text)).unwrap_or(serde_json::json!({}))
        }
        Err(e) => {
            log_warn!("handler", "tool {} failed: {e}", call.name);
            serde_json::to_value(ToolCallResult::error(e.to_string()))
                .unwrap_or(serde_json::json!({}))
        }
    };

    if let Some(rec) = &ctx.recorder {
        let request = serde_json::json!({
            "tool": call.name,
            "arguments": call.arguments,
        });
        let result_val = match &result {
            Ok(data) => data.clone(),
            Err(e) => serde_json::json!({ "error": e.to_string() }),
        };
        rec.record_jmap(&call.name, &request, &result_val);
    }

    response
}

fn dispatch_tool(
    name: &str,
    args: &serde_json::Value,
    ctx: &Context,
) -> Result<serde_json::Value, Error> {
    match name {
        "list_mailboxes" => {
            let action = mailbox::ListMailboxes {
                role: str_param(args, "role"),
            };
            action.run(ctx)
        }
        "manage_mailbox" => {
            let action = mailbox::ManageMailbox {
                action: str_param(args, "action"),
                name: str_param(args, "name"),
                mailbox_id: str_param(args, "mailboxId"),
                parent_id: str_param(args, "parentId"),
            };
            action.run(ctx)
        }
        "get_emails" => {
            let action = email::GetEmails {
                mailbox_id: str_param(args, "mailboxId"),
                mailbox_name: str_param(args, "mailboxName"),
                limit: u32_param(args, "limit"),
                include_body: bool_param(args, "includeBody"),
            };
            action.run(ctx)
        }
        "search_emails" => {
            let action = email::SearchEmails {
                keyword: str_param(args, "keyword"),
                from: str_param(args, "from"),
                to: str_param(args, "to"),
                subject: str_param(args, "subject"),
                mailbox_id: str_param(args, "mailboxId"),
                has_attachment: args.get("hasAttachment").and_then(|v| v.as_bool()),
                after: str_param(args, "after"),
                before: str_param(args, "before"),
                limit: u32_param(args, "limit"),
                include_body: bool_param(args, "includeBody"),
            };
            action.run(ctx)
        }
        "get_email_body" => {
            let action = email::GetEmailBody {
                email_id: str_param(args, "emailId"),
                format: str_param(args, "format"),
            };
            action.run(ctx)
        }
        "send_email" => {
            let action = email::SendEmail {
                to: str_array_param(args, "to"),
                subject: str_param(args, "subject"),
                body: str_param(args, "body"),
                cc: str_array_param(args, "cc"),
                bcc: str_array_param(args, "bcc"),
                is_html: bool_param(args, "isHtml"),
                in_reply_to: str_param(args, "inReplyTo"),
            };
            action.run(ctx)
        }
        "move_email" => {
            let action = email::MoveEmail {
                email_ids: str_array_param(args, "emailIds"),
                mailbox_id: str_param(args, "mailboxId"),
            };
            action.run(ctx)
        }
        "delete_email" => {
            let action = email::DeleteEmail {
                email_ids: str_array_param(args, "emailIds"),
                permanent: bool_param(args, "permanent"),
            };
            action.run(ctx)
        }
        "flag_email" => {
            let action = email::FlagEmail {
                email_ids: str_array_param(args, "emailIds"),
                flag: str_param(args, "flag"),
                value: bool_param(args, "value"),
            };
            action.run(ctx)
        }
        "get_vacation_response" => {
            let action = vacation::GetVacationResponse;
            action.run(ctx)
        }
        "set_vacation_response" => {
            let action = vacation::SetVacationResponse {
                is_enabled: bool_param(args, "isEnabled"),
                from_date: str_param(args, "fromDate"),
                to_date: str_param(args, "toDate"),
                subject: str_param(args, "subject"),
                text_body: str_param(args, "textBody"),
                html_body: str_param(args, "htmlBody"),
            };
            action.run(ctx)
        }
        "list_identities" => {
            let action = identity::ListIdentities;
            action.run(ctx)
        }
        "list_masked_emails" => {
            let action = masked_email::ListMaskedEmails {
                state: str_param(args, "state"),
            };
            action.run(ctx)
        }
        "create_masked_email" => {
            let action = masked_email::CreateMaskedEmail {
                for_domain: str_param(args, "forDomain"),
                description: str_param(args, "description"),
                email_prefix: str_param(args, "emailPrefix"),
            };
            action.run(ctx)
        }
        "update_masked_email" => {
            let action = masked_email::UpdateMaskedEmail {
                id: str_param(args, "id"),
                state: str_param(args, "state"),
            };
            action.run(ctx)
        }
        _ => Err(Error::InvalidParams(format!("unknown tool: {name}"))),
    }
}

fn str_param(args: &serde_json::Value, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn u32_param(args: &serde_json::Value, key: &str) -> u32 {
    args.get(key)
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32
}

fn bool_param(args: &serde_json::Value, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn str_array_param(args: &serde_json::Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_returns_correct_protocol_version() {
        let params = serde_json::json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" }
        });

        let result = handle_initialize(params);

        assert_eq!(result["protocolVersion"], "2025-11-25");
        assert_eq!(result["serverInfo"]["name"], "fastermail");
        assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
    }

    #[test]
    fn tools_list_returns_all_tools() {
        let result = handle_tools_list();
        let tools = result["tools"].as_array().expect("tools should be an array");

        let tool_names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();

        assert!(tool_names.contains(&"list_mailboxes"));
        assert!(tool_names.contains(&"get_emails"));
        assert!(tool_names.contains(&"search_emails"));
        assert!(tool_names.contains(&"send_email"));
        assert!(tool_names.contains(&"get_vacation_response"));
        assert!(tool_names.contains(&"list_identities"));
        assert!(tool_names.contains(&"list_masked_emails"));
    }

    #[test]
    fn str_param_extracts_string() {
        let args = serde_json::json!({ "name": "inbox" });
        assert_eq!(str_param(&args, "name"), "inbox");
        assert_eq!(str_param(&args, "missing"), "");
    }

    #[test]
    fn bool_param_extracts_bool() {
        let args = serde_json::json!({ "flag": true });
        assert!(bool_param(&args, "flag"));
        assert!(!bool_param(&args, "missing"));
    }

    #[test]
    fn str_array_param_extracts_array() {
        let args = serde_json::json!({ "to": ["a@b.com", "c@d.com"] });
        let result = str_array_param(&args, "to");
        assert_eq!(result, vec!["a@b.com", "c@d.com"]);
        assert!(str_array_param(&args, "missing").is_empty());
    }

    #[test]
    fn u32_param_defaults_to_zero() {
        let args = serde_json::json!({ "limit": 50 });
        assert_eq!(u32_param(&args, "limit"), 50);
        assert_eq!(u32_param(&args, "missing"), 0);
    }

    #[test]
    fn tools_list_every_tool_has_input_schema() {
        let result = handle_tools_list();
        let tools = result["tools"].as_array().expect("tools should be array");

        for tool in tools {
            let name = tool["name"].as_str().expect("tool should have name");
            assert!(
                tool.get("inputSchema").is_some(),
                "tool {name} missing inputSchema"
            );
            assert_eq!(
                tool["inputSchema"]["type"], "object",
                "tool {name} inputSchema should be object type"
            );
        }
    }

    #[test]
    fn dispatch_unknown_tool_returns_error() {
        let err = dispatch_tool(
            "nonexistent_tool",
            &serde_json::json!({}),
            &test_ctx(),
        );

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("nonexistent_tool"),
            "error should name the unknown tool: {msg}"
        );
    }

    #[test]
    fn dispatch_flag_email_rejects_invalid_flag() {
        let err = dispatch_tool(
            "flag_email",
            &serde_json::json!({
                "emailIds": ["e1"],
                "flag": "invalid_flag",
                "value": true
            }),
            &test_ctx(),
        );

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(msg.contains("invalid flag"), "error should mention invalid flag: {msg}");
    }

    #[test]
    fn dispatch_search_emails_requires_filter() {
        let err = dispatch_tool(
            "search_emails",
            &serde_json::json!({}),
            &test_ctx(),
        );

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("filter"),
            "error should mention missing filter: {msg}"
        );
    }

    #[test]
    fn dispatch_get_email_body_requires_email_id() {
        let err = dispatch_tool(
            "get_email_body",
            &serde_json::json!({}),
            &test_ctx(),
        );

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("emailId"),
            "error should mention missing emailId: {msg}"
        );
    }

    #[test]
    fn dispatch_get_email_body_rejects_invalid_format() {
        let err = dispatch_tool(
            "get_email_body",
            &serde_json::json!({ "emailId": "e1", "format": "xml" }),
            &test_ctx(),
        );

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("format"),
            "error should mention invalid format: {msg}"
        );
    }

    /// Create a test context with a JmapClient pointing at a non-existent server.
    /// Only usable for tests that validate parameters before making HTTP calls.
    fn test_ctx() -> Context {
        Context {
            jmap: crate::jmap::client::JmapClient::new(
                "http://localhost:0".to_string(),
                "test-token".to_string(),
            ),
            account_id: "test-account".to_string(),
            recorder: None,
        }
    }
}
