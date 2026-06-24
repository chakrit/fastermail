use fastermail::{log_debug, log_trace, log_warn};

use crate::actions::{
    self, Action, Context, contact, email, identity, mailbox, masked_email, vacation,
};
use crate::error::Error;
use crate::json;
use crate::mcp::types::{
    InitializeParams, InitializeResult, ServerCapabilities, ServerInfo, ToolCallParams,
    ToolCallResult, ToolsCapability, ToolsListResult,
};
use crate::present;

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
    let call: ToolCallParams = match serde_json::from_value(params) {
        Ok(c) => c,
        Err(e) => {
            log_warn!("handler", "malformed tools/call params: {e}");
            return serde_json::to_value(ToolCallResult::error(format!(
                "malformed tool call params: {e}"
            )))
            .unwrap_or_default();
        }
    };

    if call.name.is_empty() {
        return serde_json::to_value(ToolCallResult::error("tool name is required".to_string()))
            .unwrap_or_default();
    }

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
            let action = mailbox::ManageMailbox::parse(
                &str_param(args, "action"),
                str_param(args, "name"),
                str_param(args, "mailboxId"),
                str_param(args, "parentId"),
            )?;
            action.run(ctx)
        }
        "get_emails" => {
            let action = email::GetEmails {
                mailbox_id: str_param(args, "mailboxId"),
                mailbox_name: str_param(args, "mailboxName"),
                limit: u32_param(args, "limit"),
                include_body: bool_param(args, "includeBody"),
                all: false,
            };
            let mut value = action.run(ctx)?;
            present::project_email_list(&mut value);
            Ok(value)
        }
        "search_emails" => {
            let action = email::SearchEmails {
                keyword: str_param(args, "keyword"),
                from: str_param(args, "from"),
                to: str_param(args, "to"),
                subject: str_param(args, "subject"),
                mailbox_id: str_param(args, "mailboxId"),
                has_attachment: json::bool_at(args, "/hasAttachment"),
                after: str_param(args, "after"),
                before: str_param(args, "before"),
                limit: u32_param(args, "limit"),
                include_body: bool_param(args, "includeBody"),
                all: false,
            };
            let mut value = action.run(ctx)?;
            present::project_email_list(&mut value);
            Ok(value)
        }
        "get_email_body" => {
            let action = email::GetEmailBody {
                email_id: str_param(args, "emailId"),
                format: email::BodyFormat::parse(&str_param(args, "format"))?,
            };
            let mut value = action.run(ctx)?;
            present::project_email_body(&mut value);
            Ok(value)
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
            if args.get("value").is_none() {
                return Err(Error::InvalidParams("value is required".to_string()));
            }
            let action = email::FlagEmail {
                email_ids: str_array_param(args, "emailIds"),
                flag: email::Flag::parse(&str_param(args, "flag"))?,
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
                is_enabled: json::bool_at(args, "/isEnabled"),
                from_date: vacation::FieldChange::from_arg(args, "fromDate"),
                to_date: vacation::FieldChange::from_arg(args, "toDate"),
                subject: vacation::FieldChange::from_arg(args, "subject"),
                text_body: vacation::FieldChange::from_arg(args, "textBody"),
                html_body: vacation::FieldChange::from_arg(args, "htmlBody"),
            };
            action.run(ctx)
        }
        "list_identities" => {
            let action = identity::ListIdentities;
            let value = action.run(ctx)?;
            Ok(present::project_identity_list(&value))
        }
        "list_masked_emails" => {
            let state = match str_param(args, "state").as_str() {
                "" => None,
                s => Some(masked_email::MaskedEmailState::parse(s)?),
            };
            let action = masked_email::ListMaskedEmails { state };
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
                state: masked_email::MaskedEmailState::parse_settable(&str_param(args, "state"))?,
            };
            action.run(ctx)
        }
        "list_address_books" => {
            let action = contact::ListAddressBooks;
            action.run(ctx)
        }
        "get_contacts" => {
            let action = contact::GetContacts {
                address_book_id: str_param(args, "addressBookId"),
                limit: u32_param(args, "limit"),
            };
            action.run(ctx)
        }
        "search_contacts" => {
            let action = contact::SearchContacts {
                query: str_param(args, "query"),
                limit: u32_param(args, "limit"),
            };
            action.run(ctx)
        }
        "create_contact" => {
            let action = contact::CreateContact {
                name: str_param(args, "name"),
                emails: json_array_param(args, "emails")
                    .iter()
                    .map(contact::ContactEmail::from_input)
                    .collect(),
                phones: json_array_param(args, "phones")
                    .iter()
                    .map(contact::ContactPhone::from_input)
                    .collect(),
                company: str_param(args, "company"),
                notes: str_param(args, "notes"),
                address_book_id: str_param(args, "addressBookId"),
            };
            action.run(ctx)
        }
        "update_contact" => {
            let name = str_param(args, "name");
            let patch = contact::ContactPatch {
                name: (!name.is_empty()).then_some(name),
                emails: args.get("emails").map(|_| {
                    json_array_param(args, "emails")
                        .iter()
                        .map(contact::ContactEmail::from_input)
                        .collect()
                }),
                phones: args.get("phones").map(|_| {
                    json_array_param(args, "phones")
                        .iter()
                        .map(contact::ContactPhone::from_input)
                        .collect()
                }),
                company: args.get("company").map(|_| str_param(args, "company")),
                notes: args.get("notes").map(|_| str_param(args, "notes")),
            };
            let action = contact::UpdateContact {
                contact_id: str_param(args, "contactId"),
                patch,
            };
            action.run(ctx)
        }
        "delete_contact" => {
            let action = contact::DeleteContact {
                contact_id: str_param(args, "contactId"),
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
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as u32
}

fn bool_param(args: &serde_json::Value, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn json_array_param(args: &serde_json::Value, key: &str) -> Vec<serde_json::Value> {
    args.get(key)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
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
        let tools = result["tools"]
            .as_array()
            .expect("tools should be an array");

        let tool_names: Vec<&str> = tools
            .iter()
            .filter_map(|t| json::str_at(t, "/name"))
            .collect();

        assert!(tool_names.contains(&"list_mailboxes"));
        assert!(tool_names.contains(&"get_emails"));
        assert!(tool_names.contains(&"search_emails"));
        assert!(tool_names.contains(&"send_email"));
        assert!(tool_names.contains(&"get_vacation_response"));
        assert!(tool_names.contains(&"list_identities"));
        assert!(tool_names.contains(&"list_masked_emails"));
        assert!(tool_names.contains(&"list_address_books"));
        assert!(tool_names.contains(&"get_contacts"));
        assert!(tool_names.contains(&"search_contacts"));
        assert!(tool_names.contains(&"create_contact"));
        assert!(tool_names.contains(&"update_contact"));
        assert!(tool_names.contains(&"delete_contact"));
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
        let err = dispatch_tool("nonexistent_tool", &serde_json::json!({}), &test_ctx());

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
        assert!(
            msg.contains("invalid flag"),
            "error should mention invalid flag: {msg}"
        );
    }

    #[test]
    fn dispatch_search_emails_requires_filter() {
        let err = dispatch_tool("search_emails", &serde_json::json!({}), &test_ctx());

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("filter"),
            "error should mention missing filter: {msg}"
        );
    }

    #[test]
    fn dispatch_get_email_body_requires_email_id() {
        let err = dispatch_tool("get_email_body", &serde_json::json!({}), &test_ctx());

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

    #[test]
    fn json_array_param_extracts_array() {
        let args = serde_json::json!({
            "emails": [{"address": "a@b.com"}, {"address": "c@d.com"}]
        });
        let result = json_array_param(&args, "emails");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["address"], "a@b.com");
        assert!(json_array_param(&args, "missing").is_empty());
    }

    #[test]
    fn dispatch_search_contacts_requires_query() {
        let err = dispatch_tool("search_contacts", &serde_json::json!({}), &test_ctx());

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("query"),
            "error should mention missing query: {msg}"
        );
    }

    #[test]
    fn dispatch_create_contact_requires_name() {
        let err = dispatch_tool("create_contact", &serde_json::json!({}), &test_ctx());

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("name"),
            "error should mention missing name: {msg}"
        );
    }

    #[test]
    fn dispatch_delete_contact_requires_contact_id() {
        let err = dispatch_tool("delete_contact", &serde_json::json!({}), &test_ctx());

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("contactId"),
            "error should mention missing contactId: {msg}"
        );
    }

    #[test]
    fn dispatch_update_contact_requires_fields() {
        let err = dispatch_tool(
            "update_contact",
            &serde_json::json!({ "contactId": "c1" }),
            &test_ctx(),
        );

        assert!(err.is_err());
        let msg = err.expect_err("should be error").to_string();
        assert!(
            msg.contains("at least one field"),
            "error should mention missing fields: {msg}"
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

    /// A context wired to a started `MockJmap`, for tests that exercise the full
    /// dispatch → JMAP → projection path.
    fn mock_ctx(mock: &crate::testutil::mock_jmap::MockJmap) -> Context {
        let (client, _) =
            crate::jmap::client::JmapClient::connect_to(&mock.session_url(), "fake-token")
                .expect("session connect");
        Context {
            jmap: client,
            account_id: crate::testutil::mock_jmap::TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        }
    }

    /// Extract the projected JSON `text` payload an MCP tool call wraps. This is the
    /// exact byte string the MCP server emits to the client.
    fn tool_call_text(response: &serde_json::Value) -> String {
        response
            .pointer("/content/0/text")
            .and_then(|t| t.as_str())
            .expect("tool result should carry text content")
            .to_string()
    }

    // --- Presenter golden tests (byte-identity net for the projection layer) ---
    //
    // The MCP server emits `to_string_pretty(projected_value)`. These pin that exact
    // output — both as a parsed Value (catches a field reorder) and as the raw string
    // (catches any byte change) — so relocating projection between layers can be proven
    // byte-identical.

    #[test]
    fn golden_get_email_body_projects_resolved_body() {
        let mock = crate::testutil::mock_jmap::MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "Email/get",
            serde_json::json!({
                "methodResponses": [
                    ["Email/get", {
                        "list": [{
                            "id": "e001",
                            "subject": "Test",
                            "from": [{"email": "a@b.com"}],
                            "to": [{"email": "c@d.com"}],
                            "receivedAt": "2026-01-01T00:00:00Z",
                            "textBody": [{"partId": "p1"}],
                            "htmlBody": [{"partId": "p2"}],
                            "bodyValues": {
                                "p1": {"value": "plain text body"},
                                "p2": {"value": "<p>html body</p>"}
                            }
                        }]
                    }, "call-0"]
                ]
            }),
        );

        let response = handle_tools_call(
            serde_json::json!({
                "name": "get_email_body",
                "arguments": { "emailId": "e001" }
            }),
            &ctx,
        );

        let text = tool_call_text(&response);
        let expected = serde_json::json!({
            "id": "e001",
            "subject": "Test",
            "from": [{"email": "a@b.com"}],
            "to": [{"email": "c@d.com"}],
            "receivedAt": "2026-01-01T00:00:00Z",
            "date": "2026-01-01T00:00:00Z",
            "textBody": "plain text body",
            "htmlBody": "<p>html body</p>"
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&text).expect("text should be JSON"),
            expected,
            "projected body Value drifted"
        );
        // Pin the exact bytes — catches a field reorder or formatting change.
        assert_eq!(
            text,
            serde_json::to_string_pretty(&expected).expect("pretty"),
            "MCP body output bytes drifted"
        );
    }

    #[test]
    fn golden_get_emails_with_body_projects_each() {
        let mock = crate::testutil::mock_jmap::MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "Email/query",
            serde_json::json!({
                "methodResponses": [["Email/query", {"ids": ["e001"]}, "call-0"]]
            }),
        );
        mock.handle_method(
            "Email/get",
            serde_json::json!({
                "methodResponses": [
                    ["Email/get", {
                        "list": [{
                            "id": "e001",
                            "subject": "Hello",
                            "from": [{"email": "a@b.com"}],
                            "to": [{"email": "c@d.com"}],
                            "receivedAt": "2026-01-01T00:00:00Z",
                            "preview": "Hello there",
                            "textBody": [{"partId": "p1"}],
                            "htmlBody": [{"partId": "p2"}],
                            "bodyValues": {
                                "p1": {"value": "plain text body"},
                                "p2": {"value": "<p>html body</p>"}
                            }
                        }]
                    }, "call-0"]
                ]
            }),
        );

        let response = handle_tools_call(
            serde_json::json!({
                "name": "get_emails",
                "arguments": { "mailboxId": "mb1", "includeBody": true }
            }),
            &ctx,
        );

        let text = tool_call_text(&response);
        let expected = serde_json::json!([{
            "id": "e001",
            "subject": "Hello",
            "from": [{"email": "a@b.com"}],
            "to": [{"email": "c@d.com"}],
            "receivedAt": "2026-01-01T00:00:00Z",
            "date": "2026-01-01T00:00:00Z",
            "preview": "Hello there",
            "textBody": "plain text body",
            "htmlBody": "<p>html body</p>"
        }]);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&text).expect("text should be JSON"),
            expected,
            "projected list Value drifted"
        );
        assert_eq!(
            text,
            serde_json::to_string_pretty(&expected).expect("pretty"),
            "MCP list output bytes drifted"
        );
    }

    #[test]
    fn golden_list_identities_projects_fields() {
        let mock = crate::testutil::mock_jmap::MockJmap::start();
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

        let response = handle_tools_call(
            serde_json::json!({
                "name": "list_identities",
                "arguments": {}
            }),
            &ctx,
        );

        let text = tool_call_text(&response);
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
            serde_json::from_str::<serde_json::Value>(&text).expect("text should be JSON"),
            expected,
            "projected identity list Value drifted"
        );
        assert_eq!(
            text,
            serde_json::to_string_pretty(&expected).expect("pretty"),
            "MCP identity list output bytes drifted"
        );
    }
}
