use crate::actions::{Action, Context};
use crate::error::{Error, Result};
use crate::jmap::types::back_reference;
use crate::mcp::types::Tool;

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "get_emails".to_string(),
            description: "Retrieve emails from a mailbox".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "mailboxId": { "type": "string", "description": "Mailbox ID" },
                    "mailboxName": { "type": "string", "description": "Mailbox name (resolved to ID)" },
                    "limit": { "type": "integer", "description": "Max results (default 20)" },
                    "includeBody": { "type": "boolean", "description": "Include body content" }
                }
            }),
        },
        Tool {
            name: "search_emails".to_string(),
            description: "Search emails with filters".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "Full-text search" },
                    "from": { "type": "string", "description": "Sender address filter" },
                    "to": { "type": "string", "description": "Recipient address filter" },
                    "subject": { "type": "string", "description": "Subject filter" },
                    "mailboxId": { "type": "string", "description": "Restrict to mailbox" },
                    "hasAttachment": { "type": "boolean", "description": "Filter by attachment presence" },
                    "after": { "type": "string", "description": "Date lower bound (YYYY-MM-DD)" },
                    "before": { "type": "string", "description": "Date upper bound (YYYY-MM-DD)" },
                    "limit": { "type": "integer", "description": "Max results (default 20)" },
                    "includeBody": { "type": "boolean", "description": "Include body content" }
                }
            }),
        },
        Tool {
            name: "get_email_body".to_string(),
            description: "Get full body of a single email".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "emailId": { "type": "string", "description": "Email ID" },
                    "format": { "type": "string", "description": "text, html, or both (default text)" }
                },
                "required": ["emailId"]
            }),
        },
        Tool {
            name: "send_email".to_string(),
            description: "Compose and send an email".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "to": { "type": "array", "items": { "type": "string" }, "description": "Recipient addresses" },
                    "subject": { "type": "string", "description": "Email subject" },
                    "body": { "type": "string", "description": "Email body" },
                    "cc": { "type": "array", "items": { "type": "string" }, "description": "CC recipients" },
                    "bcc": { "type": "array", "items": { "type": "string" }, "description": "BCC recipients" },
                    "isHtml": { "type": "boolean", "description": "Body is HTML (default false)" },
                    "inReplyTo": { "type": "string", "description": "Email ID being replied to" }
                },
                "required": ["to", "subject", "body"]
            }),
        },
        Tool {
            name: "move_email".to_string(),
            description: "Move emails between mailboxes".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "emailIds": { "type": "array", "items": { "type": "string" }, "description": "Email IDs to move" },
                    "mailboxId": { "type": "string", "description": "Destination mailbox ID" }
                },
                "required": ["emailIds", "mailboxId"]
            }),
        },
        Tool {
            name: "delete_email".to_string(),
            description: "Delete emails (move to Trash or permanently delete)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "emailIds": { "type": "array", "items": { "type": "string" }, "description": "Email IDs to delete" },
                    "permanent": { "type": "boolean", "description": "Skip trash (default false)" }
                },
                "required": ["emailIds"]
            }),
        },
        Tool {
            name: "flag_email".to_string(),
            description: "Set/unset flags (keywords) on emails".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "emailIds": { "type": "array", "items": { "type": "string" }, "description": "Email IDs" },
                    "flag": { "type": "string", "description": "Flag: seen, flagged, answered, draft" },
                    "value": { "type": "boolean", "description": "Set (true) or unset (false)" }
                },
                "required": ["emailIds", "flag", "value"]
            }),
        },
    ]
}

pub struct GetEmails {
    pub mailbox_id: String,
    pub mailbox_name: String,
    pub limit: u32,
    pub include_body: bool,
}

impl Action for GetEmails {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let mailbox_id = self.resolve_mailbox_id(ctx)?;
        let limit = if self.limit == 0 { 20 } else { self.limit };

        let using = vec![
            "urn:ietf:params:jmap:core".to_string(),
            "urn:ietf:params:jmap:mail".to_string(),
        ];

        let query_args = serde_json::json!({
            "accountId": ctx.account_id,
            "filter": { "inMailbox": mailbox_id },
            "sort": [{ "property": "receivedAt", "isAscending": false }],
            "limit": limit
        });

        let mut get_args = serde_json::json!({
            "accountId": ctx.account_id,
            "#ids": back_reference("call-0", "Email/query", "/ids"),
            "properties": ["id", "subject", "from", "to", "receivedAt", "preview"]
        });

        if self.include_body {
            get_args["fetchTextBodyValues"] = serde_json::json!(true);
            get_args["properties"] = serde_json::json!(
                ["id", "subject", "from", "to", "receivedAt", "preview", "textBody", "htmlBody", "bodyValues"]
            );
        }

        let method_calls = vec![
            ("Email/query".to_string(), query_args, "call-0".to_string()),
            ("Email/get".to_string(), get_args, "call-1".to_string()),
        ];

        let resp = ctx.jmap.call(using, method_calls)?;

        let email_data = resp
            .method_responses
            .iter()
            .find(|(m, _, _)| m == "Email/get")
            .map(|(_, data, _)| data.get("list").cloned().unwrap_or(serde_json::json!([])))
            .unwrap_or(serde_json::json!([]));

        Ok(email_data)
    }
}

impl GetEmails {
    fn resolve_mailbox_id(&self, ctx: &Context) -> Result<String> {
        if !self.mailbox_id.is_empty() {
            return Ok(self.mailbox_id.clone());
        }

        if self.mailbox_name.is_empty() {
            return Err(Error::InvalidParams(
                "must provide mailboxId or mailboxName".to_string(),
            ));
        }

        let data = ctx.jmap.call_one(
            "urn:ietf:params:jmap:mail",
            "Mailbox/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        let target = self.mailbox_name.to_lowercase();
        let list = data.get("list").and_then(|l| l.as_array());

        list.and_then(|mailboxes| {
            mailboxes.iter().find_map(|m| {
                let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("");
                if name.to_lowercase() == target {
                    m.get("id").and_then(|id| id.as_str()).map(String::from)
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| {
            Error::InvalidParams(format!("mailbox not found: {}", self.mailbox_name))
        })
    }
}

pub struct SearchEmails {
    pub keyword: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub mailbox_id: String,
    pub has_attachment: Option<bool>,
    pub after: String,
    pub before: String,
    pub limit: u32,
    pub include_body: bool,
}

impl Action for SearchEmails {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let filter = self.build_filter()?;
        let limit = if self.limit == 0 { 20 } else { self.limit };

        let using = vec![
            "urn:ietf:params:jmap:core".to_string(),
            "urn:ietf:params:jmap:mail".to_string(),
        ];

        let query_args = serde_json::json!({
            "accountId": ctx.account_id,
            "filter": filter,
            "sort": [{ "property": "receivedAt", "isAscending": false }],
            "limit": limit
        });

        let mut get_args = serde_json::json!({
            "accountId": ctx.account_id,
            "#ids": back_reference("call-0", "Email/query", "/ids"),
            "properties": ["id", "subject", "from", "to", "receivedAt", "preview"]
        });

        if self.include_body {
            get_args["fetchTextBodyValues"] = serde_json::json!(true);
        }

        let method_calls = vec![
            ("Email/query".to_string(), query_args, "call-0".to_string()),
            ("Email/get".to_string(), get_args, "call-1".to_string()),
        ];

        let resp = ctx.jmap.call(using, method_calls)?;

        let email_data = resp
            .method_responses
            .iter()
            .find(|(m, _, _)| m == "Email/get")
            .map(|(_, data, _)| data.get("list").cloned().unwrap_or(serde_json::json!([])))
            .unwrap_or(serde_json::json!([]));

        Ok(email_data)
    }
}

impl SearchEmails {
    fn build_filter(&self) -> Result<serde_json::Value> {
        let mut filter = serde_json::Map::new();

        if !self.keyword.is_empty() {
            filter.insert("text".to_string(), serde_json::json!(self.keyword));
        }
        if !self.from.is_empty() {
            filter.insert("from".to_string(), serde_json::json!(self.from));
        }
        if !self.to.is_empty() {
            filter.insert("to".to_string(), serde_json::json!(self.to));
        }
        if !self.subject.is_empty() {
            filter.insert("subject".to_string(), serde_json::json!(self.subject));
        }
        if !self.mailbox_id.is_empty() {
            filter.insert("inMailbox".to_string(), serde_json::json!(self.mailbox_id));
        }
        if let Some(has_att) = self.has_attachment {
            filter.insert("hasAttachment".to_string(), serde_json::json!(has_att));
        }
        if !self.after.is_empty() {
            filter.insert(
                "after".to_string(),
                serde_json::json!(format!("{}T00:00:00Z", self.after)),
            );
        }
        if !self.before.is_empty() {
            filter.insert(
                "before".to_string(),
                serde_json::json!(format!("{}T23:59:59Z", self.before)),
            );
        }

        if filter.is_empty() {
            return Err(Error::InvalidParams(
                "at least one search filter required".to_string(),
            ));
        }

        Ok(serde_json::Value::Object(filter))
    }
}

pub struct GetEmailBody {
    pub email_id: String,
    pub format: String,
}

impl Action for GetEmailBody {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.email_id.is_empty() {
            return Err(Error::InvalidParams("emailId is required".to_string()));
        }

        let format = if self.format.is_empty() {
            "text"
        } else {
            &self.format
        };

        let mut args = serde_json::json!({
            "accountId": ctx.account_id,
            "ids": [self.email_id],
            "properties": ["id", "subject", "textBody", "htmlBody", "bodyValues"]
        });

        match format {
            "text" => args["fetchTextBodyValues"] = serde_json::json!(true),
            "html" => args["fetchHTMLBodyValues"] = serde_json::json!(true),
            "both" => args["fetchAllBodyValues"] = serde_json::json!(true),
            _ => {
                return Err(Error::InvalidParams(
                    "format must be text, html, or both".to_string(),
                ))
            }
        }

        let data = ctx
            .jmap
            .call_one("urn:ietf:params:jmap:mail", "Email/get", args)?;

        let email = data
            .get("list")
            .and_then(|l| l.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .ok_or_else(|| {
                Error::InvalidParams(format!("email not found: {}", self.email_id))
            })?;

        Ok(email)
    }
}

pub struct SendEmail {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub is_html: bool,
    pub in_reply_to: String,
}

impl Action for SendEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.to.is_empty() {
            return Err(Error::InvalidParams("to is required".to_string()));
        }
        if self.subject.is_empty() {
            return Err(Error::InvalidParams("subject is required".to_string()));
        }
        if self.body.is_empty() {
            return Err(Error::InvalidParams("body is required".to_string()));
        }

        let to_addrs: Vec<serde_json::Value> = self
            .to
            .iter()
            .map(|addr| serde_json::json!({ "email": addr }))
            .collect();

        let body_type = if self.is_html { "text/html" } else { "text/plain" };
        let body_part = serde_json::json!({
            "partId": "body",
            "type": body_type
        });

        let mut email_obj = serde_json::json!({
            "from": [{ "email": "" }],
            "to": to_addrs,
            "subject": self.subject,
            "keywords": { "$draft": true },
            "bodyValues": {
                "body": { "value": self.body }
            }
        });

        if self.is_html {
            email_obj["htmlBody"] = serde_json::json!([body_part]);
        } else {
            email_obj["textBody"] = serde_json::json!([body_part]);
        }

        if !self.cc.is_empty() {
            let cc_addrs: Vec<serde_json::Value> = self
                .cc
                .iter()
                .map(|addr| serde_json::json!({ "email": addr }))
                .collect();
            email_obj["cc"] = serde_json::json!(cc_addrs);
        }

        if !self.bcc.is_empty() {
            let bcc_addrs: Vec<serde_json::Value> = self
                .bcc
                .iter()
                .map(|addr| serde_json::json!({ "email": addr }))
                .collect();
            email_obj["bcc"] = serde_json::json!(bcc_addrs);
        }

        let using = vec![
            "urn:ietf:params:jmap:core".to_string(),
            "urn:ietf:params:jmap:mail".to_string(),
            "urn:ietf:params:jmap:submission".to_string(),
        ];

        let create_args = serde_json::json!({
            "accountId": ctx.account_id,
            "create": { "draft": email_obj }
        });

        let submit_args = serde_json::json!({
            "accountId": ctx.account_id,
            "create": {
                "submission": {
                    "emailId": "#draft",
                    "identityId": ""
                }
            }
        });

        let method_calls = vec![
            ("Email/set".to_string(), create_args, "call-0".to_string()),
            (
                "EmailSubmission/set".to_string(),
                submit_args,
                "call-1".to_string(),
            ),
        ];

        let resp = ctx.jmap.call(using, method_calls)?;

        let email_id = resp
            .method_responses
            .iter()
            .find(|(m, _, _)| m == "Email/set")
            .and_then(|(_, data, _)| {
                data.get("created")
                    .and_then(|c| c.get("draft"))
                    .and_then(|d| d.get("id"))
                    .and_then(|id| id.as_str())
            })
            .unwrap_or("")
            .to_string();

        Ok(serde_json::json!({
            "success": true,
            "emailId": email_id
        }))
    }
}

pub struct MoveEmail {
    pub email_ids: Vec<String>,
    pub mailbox_id: String,
}

impl Action for MoveEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.email_ids.is_empty() {
            return Err(Error::InvalidParams("emailIds is required".to_string()));
        }
        if self.mailbox_id.is_empty() {
            return Err(Error::InvalidParams("mailboxId is required".to_string()));
        }

        let mut update = serde_json::Map::new();
        for id in &self.email_ids {
            update.insert(
                id.clone(),
                serde_json::json!({
                    "mailboxIds": { self.mailbox_id.clone(): true }
                }),
            );
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "update": update
        });

        ctx.jmap
            .call_one("urn:ietf:params:jmap:mail", "Email/set", args)?;

        Ok(serde_json::json!({ "moved": self.email_ids.len() }))
    }
}

pub struct DeleteEmail {
    pub email_ids: Vec<String>,
    pub permanent: bool,
}

impl Action for DeleteEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.email_ids.is_empty() {
            return Err(Error::InvalidParams("emailIds is required".to_string()));
        }

        if self.permanent {
            let args = serde_json::json!({
                "accountId": ctx.account_id,
                "destroy": self.email_ids
            });

            ctx.jmap
                .call_one("urn:ietf:params:jmap:mail", "Email/set", args)?;
        } else {
            let trash_data = ctx.jmap.call_one(
                "urn:ietf:params:jmap:mail",
                "Mailbox/get",
                serde_json::json!({ "accountId": ctx.account_id }),
            )?;

            let trash_id = trash_data
                .get("list")
                .and_then(|l| l.as_array())
                .and_then(|arr| {
                    arr.iter().find_map(|m| {
                        if m.get("role").and_then(|r| r.as_str()) == Some("trash") {
                            m.get("id").and_then(|id| id.as_str()).map(String::from)
                        } else {
                            None
                        }
                    })
                })
                .ok_or_else(|| Error::Jmap {
                    method: "Mailbox/get".to_string(),
                    message: "trash mailbox not found".to_string(),
                })?;

            let mut update = serde_json::Map::new();
            for id in &self.email_ids {
                update.insert(
                    id.clone(),
                    serde_json::json!({
                        "mailboxIds": { trash_id.clone(): true }
                    }),
                );
            }

            let args = serde_json::json!({
                "accountId": ctx.account_id,
                "update": update
            });

            ctx.jmap
                .call_one("urn:ietf:params:jmap:mail", "Email/set", args)?;
        }

        Ok(serde_json::json!({ "deleted": self.email_ids.len() }))
    }
}

pub struct FlagEmail {
    pub email_ids: Vec<String>,
    pub flag: String,
    pub value: bool,
}

impl Action for FlagEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.email_ids.is_empty() {
            return Err(Error::InvalidParams("emailIds is required".to_string()));
        }
        if self.flag.is_empty() {
            return Err(Error::InvalidParams("flag is required".to_string()));
        }

        let keyword = match self.flag.as_str() {
            "seen" => "$seen",
            "flagged" => "$flagged",
            "answered" => "$answered",
            "draft" => "$draft",
            _ => {
                return Err(Error::InvalidParams(format!(
                    "invalid flag: {}",
                    self.flag
                )))
            }
        };

        let keyword_path = format!("keywords/{keyword}");
        let mut update = serde_json::Map::new();
        for id in &self.email_ids {
            update.insert(
                id.clone(),
                serde_json::json!({ keyword_path.clone(): self.value }),
            );
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "update": update
        });

        ctx.jmap
            .call_one("urn:ietf:params:jmap:mail", "Email/set", args)?;

        Ok(serde_json::json!({ "updated": self.email_ids.len() }))
    }
}
