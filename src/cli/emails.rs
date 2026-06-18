use clap::Subcommand;
use crate::json;

use crate::actions::email::{
    BodyFormat, DeleteEmail, Flag, FlagEmail, GetEmailBody, GetEmails, MoveEmail, SearchEmails,
    SendEmail,
};
use crate::actions::{Action, Context};
use crate::cli::io::{Io, OutputMode};
use crate::cli::resolve::resolve_mailbox;
use crate::error::Result;

#[derive(Subcommand)]
pub enum EmailCommand {
    /// List emails from a mailbox
    List {
        /// Mailbox (ID, role alias, or name)
        #[arg(short = 'm', long)]
        mailbox: Option<String>,

        /// Max results
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: u32,

        /// Include body content
        #[arg(long)]
        include_body: bool,
    },

    /// Search emails with filters
    Search {
        /// Full-text search
        #[arg(short = 'q', long)]
        keyword: Option<String>,

        /// Sender address filter
        #[arg(long)]
        from: Option<String>,

        /// Recipient address filter
        #[arg(long)]
        to: Option<String>,

        /// Subject filter
        #[arg(long)]
        subject: Option<String>,

        /// Restrict to mailbox (ID, role alias, or name)
        #[arg(short = 'm', long)]
        mailbox: Option<String>,

        /// Filter for emails with attachments
        #[arg(long)]
        has_attachment: bool,

        /// Date lower bound (YYYY-MM-DD)
        #[arg(long)]
        after: Option<String>,

        /// Date upper bound (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,

        /// Max results
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: u32,

        /// Include body content
        #[arg(long)]
        include_body: bool,
    },

    /// Get full body of a single email
    Get {
        /// Email ID
        email_id: String,

        /// Body format: text, html, or both
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Move emails between mailboxes
    Move {
        /// Email IDs to move
        #[arg(required = true)]
        email_ids: Vec<String>,

        /// Target mailbox (ID, role alias, or name)
        #[arg(long)]
        to: String,
    },

    /// Set/unset flags on emails
    Flag {
        /// Email IDs
        #[arg(required = true)]
        email_ids: Vec<String>,

        /// Flag: seen, flagged, answered, or draft
        #[arg(long)]
        flag: String,

        /// Unset the flag (default: set)
        #[arg(long)]
        unset: bool,
    },

    /// Delete emails
    Delete {
        /// Email IDs to delete
        #[arg(required = true)]
        email_ids: Vec<String>,

        /// Permanently delete (skip trash)
        #[arg(long)]
        permanent: bool,
    },

    /// Compose and send an email
    Send {
        /// Recipient (repeatable)
        #[arg(long, required = true)]
        to: Vec<String>,

        /// Subject line
        #[arg(long)]
        subject: String,

        /// Body text (reads from stdin if omitted)
        #[arg(long)]
        body: Option<String>,

        /// CC recipient (repeatable)
        #[arg(long)]
        cc: Vec<String>,

        /// BCC recipient (repeatable)
        #[arg(long)]
        bcc: Vec<String>,

        /// Body is HTML
        #[arg(long)]
        html: bool,

        /// Email ID being replied to
        #[arg(long)]
        reply_to: Option<String>,
    },
}

pub fn run(cmd: EmailCommand, ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        EmailCommand::List {
            ref mailbox,
            limit,
            include_body,
        } => {
            let input = mailbox.clone().unwrap_or_else(|| "inbox".to_string());
            let mailbox_id = resolve_mailbox(&input, ctx, io)?;
            let spinner = io.progress("Fetching emails…");
            let action = GetEmails {
                mailbox_id,
                mailbox_name: String::new(),
                limit,
                include_body,
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            format_email_list(io, &value);
        }
        EmailCommand::Search {
            keyword,
            from,
            to,
            subject,
            mailbox,
            has_attachment,
            after,
            before,
            limit,
            include_body,
        } => {
            let mailbox_id = match mailbox {
                Some(ref input) => resolve_mailbox(input, ctx, io)?,
                None => String::new(),
            };
            let spinner = io.progress("Searching emails…");
            let action = SearchEmails {
                keyword: keyword.unwrap_or_default(),
                from: from.unwrap_or_default(),
                to: to.unwrap_or_default(),
                subject: subject.unwrap_or_default(),
                mailbox_id,
                has_attachment: if has_attachment { Some(true) } else { None },
                after: after.unwrap_or_default(),
                before: before.unwrap_or_default(),
                limit,
                include_body,
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            format_email_list(io, &value);
        }
        EmailCommand::Get { email_id, format } => {
            let format = BodyFormat::parse(&format)?;
            let spinner = io.progress("Fetching email…");
            let action = GetEmailBody { email_id, format };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            format_email_body(io, &value);
        }
        EmailCommand::Move { email_ids, to } => {
            let mailbox_id = resolve_mailbox(&to, ctx, io)?;
            let count = email_ids.len();
            let spinner = io.progress(&format!("Moving {count} email(s)…"));
            let action = MoveEmail {
                email_ids,
                mailbox_id,
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            format_action_result(io, &value, &format!("Moved to {to}"));
        }
        EmailCommand::Flag {
            email_ids,
            flag,
            unset,
        } => {
            let parsed_flag = Flag::parse(&flag)?;
            let verb = if unset { "Unsetting" } else { "Setting" };
            let spinner = io.progress(&format!("{verb} {flag}…"));
            let action = FlagEmail {
                email_ids,
                flag: parsed_flag,
                value: !unset,
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            let verb = if unset { "Unset" } else { "Set" };
            format_action_result(io, &value, &format!("{verb} {flag}"));
        }
        EmailCommand::Delete {
            email_ids,
            permanent,
        } => {
            let spinner = io.progress("Deleting emails…");
            let action = DeleteEmail {
                email_ids,
                permanent,
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            let label = if permanent {
                "Permanently deleted"
            } else {
                "Moved to Trash"
            };
            format_action_result(io, &value, label);
        }
        EmailCommand::Send {
            to,
            subject,
            body,
            cc,
            bcc,
            html,
            reply_to,
        } => {
            let body_text = match body {
                Some(b) => b,
                None => {
                    io.hint("Reading body from stdin…");
                    let mut buf = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                    buf
                }
            };

            let spinner = io.progress("Sending email…");
            let action = SendEmail {
                to,
                subject,
                body: body_text,
                cc,
                bcc,
                is_html: html,
                in_reply_to: reply_to.unwrap_or_default(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;

            if io.mode() == OutputMode::Human {
                let email_id = value
                    .get("emailId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                io.done(&format!("Sent (ID: {email_id})"));
            } else {
                io.json(&value);
            }
        }
    }
    Ok(())
}

/// Format an email list (array of email objects) for output.
fn format_email_list(io: &Io, value: &serde_json::Value) {
    if io.mode() != OutputMode::Human {
        io.json(value);
        return;
    }

    let emails = match value.as_array() {
        Some(arr) => arr,
        None => {
            io.json(value);
            return;
        }
    };

    if emails.is_empty() {
        io.warn("No emails found");
        return;
    }

    io.done(&format!("{} email(s)", emails.len()));
    io.separator();

    // Print table header
    io.data(&format!(
        "{:<14} {:<20} {:<24} {}",
        "ID", "DATE", "FROM", "SUBJECT"
    ));
    io.data(&"─".repeat(80));

    for email in emails {
        let id = email
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let date = email
            .get("date")
            .or_else(|| email.get("receivedAt"))
            .and_then(|v| v.as_str())
            .map(truncate_date)
            .unwrap_or_default();
        let from = email
            .get("from")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .map(format_address)
            .unwrap_or_default();
        let subject = email
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("(no subject)");

        io.data(&format!(
            "{:<14} {:<20} {:<24} {}",
            truncate(id, 13),
            date,
            truncate(&from, 23),
            truncate(subject, 40)
        ));
    }
}

/// Format a single email body for output.
fn format_email_body(io: &Io, value: &serde_json::Value) {
    if io.mode() != OutputMode::Human {
        io.json(value);
        return;
    }

    let subject = value
        .get("subject")
        .and_then(|v| v.as_str())
        .unwrap_or("(no subject)");
    let from = value
        .get("from")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .map(format_address)
        .unwrap_or_default();
    let date = value
        .get("date")
        .or_else(|| value.get("receivedAt"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    io.data(&format!(
        "{} {}",
        console::style("Subject:").bold(),
        subject
    ));
    io.data(&format!(
        "{}    {}",
        console::style("From:").bold(),
        from
    ));
    io.data(&format!(
        "{}    {}",
        console::style("Date:").bold(),
        date
    ));
    io.separator();

    // Show body content
    if let Some(text) = json::str_at(value, "/textBody") {
        io.data(text);
    } else if let Some(html) = json::str_at(value, "/htmlBody") {
        io.data(html);
    } else {
        io.warn("No body content");
    }
}

/// Format action results (moved/deleted/updated counts).
fn format_action_result(io: &Io, value: &serde_json::Value, label: &str) {
    if io.mode() != OutputMode::Human {
        io.json(value);
        return;
    }

    // Look for count fields: "moved", "deleted", "updated"
    let count = value
        .get("moved")
        .or_else(|| value.get("deleted"))
        .or_else(|| value.get("updated"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    io.done(&format!("{label}: {count} email(s)"));
}

/// Format a JMAP address object { "name": "...", "email": "..." } into a display string.
fn format_address(addr: &serde_json::Value) -> String {
    let name = json::str_at(addr, "/name").unwrap_or("");
    let email = json::str_at(addr, "/email").unwrap_or("");
    if name.is_empty() {
        email.to_string()
    } else {
        format!("{name} <{email}>")
    }
}

/// Truncate a datetime string to "YYYY-MM-DD HH:MM" for display.
fn truncate_date(s: &str) -> String {
    // Input: "2024-03-15T09:30:00Z" → "2024-03-15 09:30"
    s.replace('T', " ")
        .chars()
        .take(16)
        .collect()
}

/// Truncate a string to max length, appending "…" if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }

    let head: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_handles_multibyte_boundary() {
        // Cut point lands mid-multibyte-char; byte slicing would panic here.
        let truncated = truncate("café—münchen", 6);
        assert_eq!(truncated, "café—…");
    }

    #[test]
    fn truncate_leaves_short_strings_intact() {
        assert_eq!(truncate("inbox", 13), "inbox");
    }
}
