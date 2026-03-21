use crate::actions::{self, Action, Context};
use crate::actions::{email, identity, mailbox, masked_email, vacation};
use crate::error::Error;
use crate::mcp::types::*;

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

    let result = dispatch_tool(&call.name, args, ctx);

    match result {
        Ok(data) => {
            let text = serde_json::to_string_pretty(&data).unwrap_or_default();
            serde_json::to_value(ToolCallResult::text(text)).unwrap_or(serde_json::json!({}))
        }
        Err(e) => serde_json::to_value(ToolCallResult::error(e.to_string()))
            .unwrap_or(serde_json::json!({})),
    }
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
}
