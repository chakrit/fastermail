use crate::actions::{
    Action, Context, check_set_errors, find_mailbox_id_by_name, find_mailbox_id_by_role,
};
use crate::error::{Error, Result};
use crate::jmap::email::{BodyFetch, EmailEnumerator, EmailId, Page, State};
use crate::json;
use crate::mcp::types::Tool;
use crate::present;

/// Default max results when the caller leaves `limit` unset (0).
const DEFAULT_LIMIT: u32 = 20;

/// Max ids per `Email/get` call; stays under JMAP's `maxObjectsInGet`.
const GET_BATCH: usize = 500;

/// `Email/query` page size when enumerating an entire filter for `--all`.
const ALL_PAGE_SIZE: u32 = 500;

/// Build JMAP address objects (`[{ "email": addr }, ...]`) from a list of addresses.
fn address_objects(addrs: &[String]) -> Vec<serde_json::Value> {
    addrs
        .iter()
        .map(|addr| serde_json::json!({ "email": addr }))
        .collect()
}

/// Query one filter window via the L1 `email_query` accessor, then fetch the matching
/// emails via L1 `email_get`, returning the faithful `Email` list. A `limit` of 0 falls
/// back to `DEFAULT_LIMIT`. The view's property/body-fetch contract comes from the L3
/// presenter.
fn query_and_fetch(
    ctx: &Context,
    filter: serde_json::Value,
    limit: u32,
    include_body: bool,
) -> Result<serde_json::Value> {
    let limit = if limit == 0 { DEFAULT_LIMIT } else { limit };

    let sort = serde_json::json!([{ "property": "receivedAt", "isAscending": false }]);
    let query = ctx.jmap.email_query(
        &ctx.account_id,
        filter,
        sort,
        Page::Position { position: 0, limit },
    )?;

    fetch_emails_by_ids(ctx, &query.ids, include_body)
}

/// `Email/get` a set of ids in batches via the L1 accessor, returning the faithful
/// `Email` list. The presenter selects the properties and body-fetch flags.
fn fetch_emails_by_ids(
    ctx: &Context,
    ids: &[EmailId],
    include_body: bool,
) -> Result<serde_json::Value> {
    let properties = present::email_list_properties(include_body);
    let body = present::email_list_body_fetch(include_body);

    let mut emails = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(GET_BATCH) {
        let resp = ctx
            .jmap
            .email_get(&ctx.account_id, chunk, Some(properties), body)?;
        for email in resp.list {
            emails.push(serde_json::to_value(email)?);
        }
    }

    Ok(serde_json::Value::Array(emails))
}

/// Enumerate every email matching `filter` via anchor paging, then fetch them.
/// Results are oldest-first — the immutable order anchor paging requires.
fn fetch_all(
    ctx: &Context,
    filter: serde_json::Value,
    include_body: bool,
) -> Result<serde_json::Value> {
    let enumerator = EmailEnumerator::new(&ctx.jmap, ctx.account_id.clone(), filter, ALL_PAGE_SIZE);
    let ids = enumerator.collect::<Result<Vec<EmailId>>>()?;
    fetch_emails_by_ids(ctx, &ids, include_body)
}

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
                    "format": { "type": "string", "description": "text, html, or both (default text)", "enum": ["text", "html", "both"] }
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
                    "flag": { "type": "string", "description": "Flag: seen, flagged, answered, draft", "enum": ["seen", "flagged", "answered", "draft"] },
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
    pub all: bool,
}

impl Action for GetEmails {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let mailbox_id = self.resolve_mailbox_id(ctx)?;
        let filter = serde_json::json!({ "inMailbox": mailbox_id });
        if self.all {
            fetch_all(ctx, filter, self.include_body)
        } else {
            query_and_fetch(ctx, filter, self.limit, self.include_body)
        }
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

        data.get("list")
            .and_then(|l| l.as_array())
            .and_then(|mailboxes| find_mailbox_id_by_name(mailboxes, &self.mailbox_name))
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
    pub all: bool,
}

impl Action for SearchEmails {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let filter = self.build_filter()?;
        if self.all {
            fetch_all(ctx, filter, self.include_body)
        } else {
            query_and_fetch(ctx, filter, self.limit, self.include_body)
        }
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

pub struct ChangesEmails {
    pub since: String,
    pub max_changes: Option<u32>,
}

impl Action for ChangesEmails {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.since.is_empty() {
            return Err(Error::InvalidParams("since state is required".to_string()));
        }

        let since = State(self.since.clone());
        let changes = ctx
            .jmap
            .email_changes(&ctx.account_id, &since, self.max_changes)?;

        Ok(serde_json::to_value(changes)?)
    }
}

/// Which body representation to fetch. Empty input defaults to text.
#[derive(Debug, Clone, Copy)]
pub enum BodyFormat {
    Text,
    Html,
    Both,
}

impl BodyFormat {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "" | "text" => Ok(Self::Text),
            "html" => Ok(Self::Html),
            "both" => Ok(Self::Both),
            _ => Err(Error::InvalidParams(
                "format must be text, html, or both".to_string(),
            )),
        }
    }

    /// The body-value fetch flags this format requires of `Email/get`.
    fn body_fetch(self) -> BodyFetch {
        match self {
            Self::Text => BodyFetch {
                text: true,
                ..Default::default()
            },
            Self::Html => BodyFetch {
                html: true,
                ..Default::default()
            },
            Self::Both => BodyFetch {
                all: true,
                ..Default::default()
            },
        }
    }
}

pub struct GetEmailBody {
    pub email_id: String,
    pub format: BodyFormat,
}

impl Action for GetEmailBody {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.email_id.is_empty() {
            return Err(Error::InvalidParams("emailId is required".to_string()));
        }

        let id = EmailId(self.email_id.clone());
        let resp = ctx.jmap.email_get(
            &ctx.account_id,
            std::slice::from_ref(&id),
            Some(present::EMAIL_BODY_PROPERTIES),
            self.format.body_fetch(),
        )?;

        let email =
            resp.list.into_iter().next().ok_or_else(|| {
                Error::InvalidParams(format!("email not found: {}", self.email_id))
            })?;

        // Faithful data — the presenter resolves body parts before display.
        Ok(serde_json::to_value(email)?)
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

        let identity = self.resolve_identity(ctx)?;
        let identity_id =
            identity
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Jmap {
                    method: "Identity/get".to_string(),
                    message: "no sending identity found".to_string(),
                })?;
        let from_email = identity.get("email").and_then(|v| v.as_str()).unwrap_or("");
        let from_name = identity.get("name").and_then(|v| v.as_str()).unwrap_or("");

        let to_addrs = address_objects(&self.to);

        let body_type = if self.is_html {
            "text/html"
        } else {
            "text/plain"
        };
        let body_part = serde_json::json!({
            "partId": "body",
            "type": body_type
        });

        let mut email_obj = serde_json::json!({
            "from": [{ "name": from_name, "email": from_email }],
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
            email_obj["cc"] = serde_json::json!(address_objects(&self.cc));
        }

        if !self.bcc.is_empty() {
            email_obj["bcc"] = serde_json::json!(address_objects(&self.bcc));
        }

        if !self.in_reply_to.is_empty() {
            self.apply_reply_headers(&mut email_obj, ctx)?;
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
                    "identityId": identity_id
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

        // Check for draft creation failures in Email/set response.
        if let Some((_, email_data, _)) = resp
            .method_responses
            .iter()
            .find(|(m, _, _)| m == "Email/set")
        {
            check_set_errors(email_data, "Email/set")?;
        }

        // Check for submission failures in EmailSubmission/set response.
        if let Some((_, sub_data, _)) = resp
            .method_responses
            .iter()
            .find(|(m, _, _)| m == "EmailSubmission/set")
        {
            check_set_errors(sub_data, "EmailSubmission/set")?;
        }

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

impl SendEmail {
    fn resolve_identity(&self, ctx: &Context) -> Result<serde_json::Value> {
        let data = ctx.jmap.call_one(
            "urn:ietf:params:jmap:submission",
            "Identity/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        data.get("list")
            .and_then(|l| l.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .ok_or_else(|| Error::Jmap {
                method: "Identity/get".to_string(),
                message: "no sending identity found".to_string(),
            })
    }

    fn apply_reply_headers(&self, email_obj: &mut serde_json::Value, ctx: &Context) -> Result<()> {
        let original = ctx.jmap.call_one(
            "urn:ietf:params:jmap:mail",
            "Email/get",
            serde_json::json!({
                "accountId": ctx.account_id,
                "ids": [self.in_reply_to],
                "properties": ["messageId", "references"]
            }),
        )?;

        let orig_email = match json::array_at(&original, "/list").and_then(|a| a.first()) {
            Some(e) => e,
            None => return Ok(()),
        };

        let msg_id = match json::array_at(orig_email, "/messageId").and_then(|a| a.first()) {
            Some(id) => id,
            None => return Ok(()),
        };

        email_obj["header:In-Reply-To:asMessageIds"] = serde_json::json!([msg_id]);

        let mut refs: Vec<serde_json::Value> = orig_email
            .get("references")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        refs.push(msg_id.clone());
        email_obj["header:References:asMessageIds"] = serde_json::json!(refs);

        Ok(())
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

        let resp = ctx.jmap.email_set(
            &ctx.account_id,
            None,
            Some(serde_json::Value::Object(update)),
            &[],
        )?;
        resp.check_errors("Email/set")?;

        Ok(serde_json::json!({ "moved": resp.updated.len() }))
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
            let ids: Vec<EmailId> = self
                .email_ids
                .iter()
                .map(|id| EmailId(id.clone()))
                .collect();
            let resp = ctx.jmap.email_set(&ctx.account_id, None, None, &ids)?;
            resp.check_errors("Email/set")?;
            return Ok(serde_json::json!({ "deleted": resp.destroyed.len() }));
        }

        let trash_id = self.resolve_trash_id(ctx)?;

        let mut update = serde_json::Map::new();
        for id in &self.email_ids {
            update.insert(
                id.clone(),
                serde_json::json!({
                    "mailboxIds": { trash_id.clone(): true }
                }),
            );
        }

        let resp = ctx.jmap.email_set(
            &ctx.account_id,
            None,
            Some(serde_json::Value::Object(update)),
            &[],
        )?;
        resp.check_errors("Email/set")?;

        Ok(serde_json::json!({ "deleted": resp.updated.len() }))
    }
}

impl DeleteEmail {
    fn resolve_trash_id(&self, ctx: &Context) -> Result<String> {
        let trash_data = ctx.jmap.call_one(
            "urn:ietf:params:jmap:mail",
            "Mailbox/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        trash_data
            .get("list")
            .and_then(|l| l.as_array())
            .and_then(|arr| find_mailbox_id_by_role(arr, "trash"))
            .ok_or_else(|| Error::Jmap {
                method: "Mailbox/get".to_string(),
                message: "trash mailbox not found".to_string(),
            })
    }
}

/// An IMAP-style keyword flag settable on an email.
#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Seen,
    Flagged,
    Answered,
    Draft,
}

impl Flag {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "seen" => Ok(Self::Seen),
            "flagged" => Ok(Self::Flagged),
            "answered" => Ok(Self::Answered),
            "draft" => Ok(Self::Draft),
            "" => Err(Error::InvalidParams("flag is required".to_string())),
            _ => Err(Error::InvalidParams(format!("invalid flag: {s}"))),
        }
    }

    fn keyword(self) -> &'static str {
        match self {
            Self::Seen => "$seen",
            Self::Flagged => "$flagged",
            Self::Answered => "$answered",
            Self::Draft => "$draft",
        }
    }
}

pub struct FlagEmail {
    pub email_ids: Vec<String>,
    pub flag: Flag,
    pub value: bool,
}

impl Action for FlagEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.email_ids.is_empty() {
            return Err(Error::InvalidParams("emailIds is required".to_string()));
        }

        let keyword_path = format!("keywords/{}", self.flag.keyword());
        let mut update = serde_json::Map::new();
        for id in &self.email_ids {
            update.insert(
                id.clone(),
                serde_json::json!({ keyword_path.clone(): self.value }),
            );
        }

        let resp = ctx.jmap.email_set(
            &ctx.account_id,
            None,
            Some(serde_json::Value::Object(update)),
            &[],
        )?;
        resp.check_errors("Email/set")?;

        Ok(serde_json::json!({ "updated": resp.updated.len() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Context;
    use crate::jmap::client::JmapClient;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    fn validation_ctx() -> Context {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        }
    }

    fn mock_ctx(mock: &MockJmap) -> Context {
        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session connect");
        Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        }
    }

    /// Register the L1 `Email/query` → `Email/get` pair the read actions now make as two
    /// separate calls: a query window of one id, then its faithful object.
    fn mock_query_then_get(mock: &MockJmap) {
        mock.handle_method(
            "Email/query",
            json!({
                "methodResponses": [["Email/query", {"ids": ["e001"]}, "call-0"]]
            }),
        );
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [["Email/get", {"list": [{"id": "e001", "subject": "Hello"}]}, "call-0"]]
            }),
        );
    }

    #[test]
    fn get_emails_requires_mailbox_id_or_name() {
        let ctx = validation_ctx();
        let action = GetEmails {
            mailbox_id: "".to_string(),
            mailbox_name: "".to_string(),
            limit: 10,
            include_body: false,
            all: false,
        };
        let err = action.run(&ctx).expect_err("should fail without mailbox");
        let msg = err.to_string();
        assert!(
            msg.contains("mailboxId") || msg.contains("mailboxName"),
            "error should mention mailboxId: {msg}"
        );
    }

    #[test]
    fn get_emails_uses_mailbox_id_directly() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock_query_then_get(&mock);

        let action = GetEmails {
            mailbox_id: "mbox-1".to_string(),
            mailbox_name: "".to_string(),
            limit: 10,
            include_body: false,
            all: false,
        };
        let result = action.run(&ctx).expect("get_emails should succeed");
        let emails = result.as_array().expect("result should be array");
        assert!(!emails.is_empty(), "should return at least one email");
    }

    #[test]
    fn get_emails_defaults_limit_to_20() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock_query_then_get(&mock);

        let action = GetEmails {
            mailbox_id: "mbox-1".to_string(),
            mailbox_name: "".to_string(),
            limit: 0,
            include_body: false,
            all: false,
        };
        let result = action
            .run(&ctx)
            .expect("get_emails with limit=0 should succeed");
        assert!(result.is_array(), "result should be array");
    }

    #[test]
    fn get_emails_all_enumerates_then_fetches() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        // A single short query window ends enumeration; ids are then fetched.
        mock.handle_method_matching(
            "Email/query",
            "\"position\"",
            json!({
                "methodResponses": [
                    ["Email/query", {"ids": ["e001", "e002"], "queryState": "s", "position": 0}, "call-0"]
                ]
            }),
        );
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [
                    ["Email/get", {"list": [
                        {"id": "e001", "subject": "One"},
                        {"id": "e002", "subject": "Two"}
                    ]}, "call-0"]
                ]
            }),
        );

        let action = GetEmails {
            mailbox_id: "mbox-1".into(),
            mailbox_name: "".into(),
            limit: 20,
            include_body: false,
            all: true,
        };
        let result = action.run(&ctx).expect("get_emails --all should succeed");
        let emails = result.as_array().expect("result should be array");
        assert_eq!(emails.len(), 2, "both enumerated ids should be fetched");
        assert_eq!(emails[0]["subject"], "One");
        assert_eq!(emails[1]["subject"], "Two");
    }

    #[test]
    fn search_emails_requires_at_least_one_filter() {
        let ctx = validation_ctx();
        let action = SearchEmails {
            keyword: "".to_string(),
            from: "".to_string(),
            to: "".to_string(),
            subject: "".to_string(),
            mailbox_id: "".to_string(),
            has_attachment: None,
            after: "".to_string(),
            before: "".to_string(),
            limit: 10,
            include_body: false,
            all: false,
        };
        let err = action.run(&ctx).expect_err("should fail with no filters");
        let msg = err.to_string();
        assert!(msg.contains("filter"), "error should mention filter: {msg}");
    }

    #[test]
    fn search_emails_builds_keyword_filter() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock_query_then_get(&mock);

        let action = SearchEmails {
            keyword: "invoice".to_string(),
            from: "".to_string(),
            to: "".to_string(),
            subject: "".to_string(),
            mailbox_id: "".to_string(),
            has_attachment: None,
            after: "".to_string(),
            before: "".to_string(),
            limit: 10,
            include_body: false,
            all: false,
        };
        let result = action
            .run(&ctx)
            .expect("search with keyword should succeed");
        assert!(result.is_array(), "result should be array");
    }

    #[test]
    fn get_email_body_requires_email_id() {
        let ctx = validation_ctx();
        let action = GetEmailBody {
            email_id: "".to_string(),
            format: BodyFormat::Text,
        };
        let err = action.run(&ctx).expect_err("should fail without emailId");
        let msg = err.to_string();
        assert!(
            msg.contains("emailId"),
            "error should mention emailId: {msg}"
        );
    }

    #[test]
    fn get_email_body_rejects_invalid_format() {
        let err = BodyFormat::parse("xml").expect_err("should reject invalid format");
        let msg = err.to_string();
        assert!(msg.contains("format"), "error should mention format: {msg}");
    }

    /// The action now returns FAITHFUL JMAP data — body part references and the raw
    /// `bodyValues` map are preserved, NOT projected. Projection moved to the L3
    /// presenter (see `present::project_email_body` + the CLI/MCP golden tests).
    #[test]
    fn get_email_body_returns_faithful_data() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [
                    ["Email/get", {
                        "list": [{
                            "id": "e001",
                            "subject": "Test",
                            "receivedAt": "2026-01-01T00:00:00Z",
                            "textBody": [{"partId": "p1"}],
                            "bodyValues": { "p1": {"value": "plain text body"} }
                        }]
                    }, "call-0"]
                ]
            }),
        );

        let action = GetEmailBody {
            email_id: "e001".to_string(),
            format: BodyFormat::Text,
        };
        let result = action.run(&ctx).expect("get_email_body should succeed");
        // Faithful: part reference intact, bodyValues present, no synthetic `date`.
        assert_eq!(result["textBody"], json!([{"partId": "p1"}]));
        assert_eq!(result["bodyValues"]["p1"]["value"], "plain text body");
        assert!(
            result.get("date").is_none(),
            "action must not synthesize `date` — that is the presenter's job"
        );
    }

    #[test]
    fn send_email_requires_to() {
        let ctx = validation_ctx();
        let action = SendEmail {
            to: vec![],
            subject: "Hi".to_string(),
            body: "Hello".to_string(),
            cc: vec![],
            bcc: vec![],
            is_html: false,
            in_reply_to: "".to_string(),
        };
        let err = action.run(&ctx).expect_err("should fail without to");
        let msg = err.to_string();
        assert!(msg.contains("to"), "error should mention to: {msg}");
    }

    #[test]
    fn send_email_requires_subject() {
        let ctx = validation_ctx();
        let action = SendEmail {
            to: vec!["a@b.com".to_string()],
            subject: "".to_string(),
            body: "Hello".to_string(),
            cc: vec![],
            bcc: vec![],
            is_html: false,
            in_reply_to: "".to_string(),
        };
        let err = action.run(&ctx).expect_err("should fail without subject");
        let msg = err.to_string();
        assert!(
            msg.contains("subject"),
            "error should mention subject: {msg}"
        );
    }

    #[test]
    fn move_email_requires_email_ids() {
        let ctx = validation_ctx();
        let action = MoveEmail {
            email_ids: vec![],
            mailbox_id: "mbox-1".to_string(),
        };
        let err = action.run(&ctx).expect_err("should fail without emailIds");
        let msg = err.to_string();
        assert!(
            msg.contains("emailIds"),
            "error should mention emailIds: {msg}"
        );
    }

    #[test]
    fn move_email_requires_mailbox_id() {
        let ctx = validation_ctx();
        let action = MoveEmail {
            email_ids: vec!["e001".to_string()],
            mailbox_id: "".to_string(),
        };
        let err = action.run(&ctx).expect_err("should fail without mailboxId");
        let msg = err.to_string();
        assert!(
            msg.contains("mailboxId"),
            "error should mention mailboxId: {msg}"
        );
    }

    #[test]
    fn delete_email_requires_email_ids() {
        let ctx = validation_ctx();
        let action = DeleteEmail {
            email_ids: vec![],
            permanent: false,
        };
        let err = action.run(&ctx).expect_err("should fail without emailIds");
        let msg = err.to_string();
        assert!(
            msg.contains("emailIds"),
            "error should mention emailIds: {msg}"
        );
    }

    #[test]
    fn flag_email_requires_email_ids() {
        let ctx = validation_ctx();
        let action = FlagEmail {
            email_ids: vec![],
            flag: Flag::Seen,
            value: true,
        };
        let err = action.run(&ctx).expect_err("should fail without emailIds");
        let msg = err.to_string();
        assert!(
            msg.contains("emailIds"),
            "error should mention emailIds: {msg}"
        );
    }

    #[test]
    fn flag_email_rejects_invalid_flag() {
        let err = Flag::parse("invalid").expect_err("should reject invalid flag");
        let msg = err.to_string();
        assert!(
            msg.contains("invalid"),
            "error should mention invalid flag: {msg}"
        );
    }

    #[test]
    fn flag_email_maps_seen_to_keyword() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock.handle_method(
            "Email/set",
            json!({
                "methodResponses": [
                    ["Email/set", {"updated": {"e001": null}}, "call-0"]
                ]
            }),
        );

        let action = FlagEmail {
            email_ids: vec!["e001".to_string()],
            flag: Flag::Seen,
            value: true,
        };
        let result = action.run(&ctx).expect("flag_email should succeed");
        assert_eq!(result["updated"], 1, "should report one updated email");
    }

    #[test]
    fn send_email_requires_body() {
        let ctx = validation_ctx();
        let action = SendEmail {
            to: vec!["a@b.com".into()],
            subject: "Hi".into(),
            body: "".into(),
            cc: vec![],
            bcc: vec![],
            is_html: false,
            in_reply_to: "".into(),
        };
        let err = action.run(&ctx).expect_err("should fail with empty body");
        let msg = err.to_string();
        assert!(msg.contains("body"), "error should mention body: {msg}");
    }

    #[test]
    fn send_email_succeeds() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);

        mock.handle_method(
            "Identity/get",
            json!({
                "methodResponses": [["Identity/get", {
                    "list": [{"id": "ident-1", "email": "me@test.com", "name": "Test"}]
                }, "call-0"]]
            }),
        );
        mock.handle_method(
            "Email/set",
            json!({
                "methodResponses": [
                    ["Email/set", {"created": {"draft": {"id": "e-new"}}}, "call-0"],
                    ["EmailSubmission/set", {"created": {"submission": {"id": "sub-1"}}}, "call-1"]
                ]
            }),
        );

        let action = SendEmail {
            to: vec!["recipient@test.com".into()],
            subject: "Test Subject".into(),
            body: "Hello there".into(),
            cc: vec![],
            bcc: vec![],
            is_html: false,
            in_reply_to: "".into(),
        };
        let result = action.run(&ctx).expect("send_email should succeed");
        assert_eq!(result["success"], true, "should report success");
        assert_eq!(result["emailId"], "e-new", "should return created email id");
    }

    #[test]
    fn move_email_succeeds() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);

        mock.handle_method(
            "Email/set",
            json!({
                "methodResponses": [["Email/set", {
                    "updated": {"e001": null, "e002": null}
                }, "call-0"]]
            }),
        );

        let action = MoveEmail {
            email_ids: vec!["e001".into(), "e002".into()],
            mailbox_id: "mbox-dest".into(),
        };
        let result = action.run(&ctx).expect("move_email should succeed");
        assert_eq!(result["moved"], 2, "should report two moved emails");
    }

    #[test]
    fn delete_email_permanent_succeeds() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);

        mock.handle_method(
            "Email/set",
            json!({
                "methodResponses": [["Email/set", {
                    "destroyed": ["e001"]
                }, "call-0"]]
            }),
        );

        let action = DeleteEmail {
            email_ids: vec!["e001".into()],
            permanent: true,
        };
        let result = action.run(&ctx).expect("permanent delete should succeed");
        assert_eq!(result["deleted"], 1, "should report one deleted email");
    }

    #[test]
    fn delete_email_moves_to_trash() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);

        mock.handle_method(
            "Mailbox/get",
            json!({
                "methodResponses": [["Mailbox/get", {
                    "list": [{"id": "mbox-trash", "name": "Trash", "role": "trash"}]
                }, "call-0"]]
            }),
        );
        mock.handle_method(
            "Email/set",
            json!({
                "methodResponses": [["Email/set", {
                    "updated": {"e001": null}
                }, "call-0"]]
            }),
        );

        let action = DeleteEmail {
            email_ids: vec!["e001".into()],
            permanent: false,
        };
        let result = action.run(&ctx).expect("trash delete should succeed");
        assert_eq!(result["deleted"], 1, "should report one deleted email");
    }

    #[test]
    fn delete_email_trash_not_found() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);

        mock.handle_method(
            "Mailbox/get",
            json!({
                "methodResponses": [["Mailbox/get", {
                    "list": [{"id": "mbox-1", "name": "Inbox", "role": "inbox"}]
                }, "call-0"]]
            }),
        );

        let action = DeleteEmail {
            email_ids: vec!["e001".into()],
            permanent: false,
        };
        let err = action
            .run(&ctx)
            .expect_err("should fail without trash mailbox");
        let msg = err.to_string();
        assert!(msg.contains("trash"), "error should mention trash: {msg}");
    }

    #[test]
    fn get_emails_resolves_mailbox_by_name() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);

        mock.handle_method(
            "Mailbox/get",
            json!({
                "methodResponses": [["Mailbox/get", {
                    "list": [{"id": "mbox-inbox", "name": "Inbox"}]
                }, "call-0"]]
            }),
        );
        mock_query_then_get(&mock);

        let action = GetEmails {
            mailbox_id: "".into(),
            mailbox_name: "Inbox".into(),
            limit: 10,
            include_body: false,
            all: false,
        };
        let result = action.run(&ctx).expect("get_emails by name should succeed");
        let emails = result.as_array().expect("result should be array");
        assert!(!emails.is_empty(), "should return at least one email");
    }

    #[test]
    fn move_email_reports_partial_failure() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);

        mock.handle_method(
            "Email/set",
            json!({
                "methodResponses": [["Email/set", {
                    "updated": {"e001": null},
                    "notUpdated": {"e002": {"type": "notFound", "description": "email not found"}}
                }, "call-0"]]
            }),
        );

        let action = MoveEmail {
            email_ids: vec!["e001".into(), "e002".into()],
            mailbox_id: "mbox-1".into(),
        };
        let err = action
            .run(&ctx)
            .expect_err("should fail on partial failure");
        let msg = err.to_string();
        assert!(
            msg.contains("email not found"),
            "error should contain failure description: {msg}"
        );
    }

    #[test]
    fn get_emails_uses_mailbox_id_directly_returns_subject() {
        let mock = MockJmap::start();
        let ctx = mock_ctx(&mock);
        mock_query_then_get(&mock);

        let action = GetEmails {
            mailbox_id: "mbox-1".into(),
            mailbox_name: "".into(),
            limit: 10,
            include_body: false,
            all: false,
        };
        let result = action.run(&ctx).expect("get_emails should succeed");
        let emails = result.as_array().expect("result should be array");
        assert!(!emails.is_empty(), "should return at least one email");
        assert_eq!(
            emails[0]["subject"], "Hello",
            "first email subject should be Hello"
        );
    }
}
