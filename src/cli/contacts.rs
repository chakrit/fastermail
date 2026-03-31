use clap::Subcommand;

use crate::actions::contact::{
    CreateContact, DeleteContact, GetContacts, ListAddressBooks, SearchContacts, UpdateContact,
};
use crate::actions::{Action, Context};
use crate::cli::io::{Io, OutputMode};
use crate::error::Result;

#[derive(Subcommand)]
pub enum ContactCommand {
    /// List contacts
    List {
        /// Filter by address book ID
        #[arg(long)]
        address_book: Option<String>,

        /// Max results (default 50)
        #[arg(short = 'n', long)]
        limit: Option<u32>,
    },

    /// Search contacts
    Search {
        /// Search text
        query: String,

        /// Max results (default 20)
        #[arg(short = 'n', long)]
        limit: Option<u32>,
    },

    /// Create a new contact
    Create {
        /// Full name
        name: String,

        /// Email address (repeatable: "work:a@b.com" or "a@b.com")
        #[arg(long, num_args = 1..)]
        email: Vec<String>,

        /// Phone number (repeatable: "work:+1234" or "+1234")
        #[arg(long, num_args = 1..)]
        phone: Vec<String>,

        /// Organization name
        #[arg(long)]
        company: Option<String>,

        /// Free-text notes
        #[arg(long)]
        notes: Option<String>,

        /// Target address book ID
        #[arg(long)]
        address_book: Option<String>,
    },

    /// Update an existing contact
    Update {
        /// Contact ID
        contact_id: String,

        /// Updated full name
        #[arg(long)]
        name: Option<String>,

        /// Updated emails (replaces all; repeatable: "work:a@b.com" or "a@b.com")
        #[arg(long, num_args = 1..)]
        email: Vec<String>,

        /// Updated phones (replaces all; repeatable: "work:+1234" or "+1234")
        #[arg(long, num_args = 1..)]
        phone: Vec<String>,

        /// Updated organization name
        #[arg(long)]
        company: Option<String>,

        /// Updated notes
        #[arg(long)]
        notes: Option<String>,
    },

    /// Delete a contact
    Delete {
        /// Contact ID
        contact_id: String,
    },

    /// List address books
    #[command(name = "address-books")]
    AddressBooks,
}

pub fn run(cmd: ContactCommand, ctx: &Context, io: &Io) -> Result<()> {
    match cmd {
        ContactCommand::List {
            address_book,
            limit,
        } => {
            let spinner = io.progress("Fetching contacts…");
            let action = GetContacts {
                address_book_id: address_book.unwrap_or_default(),
                limit: limit.unwrap_or(0),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            format_contact_list(io, &value);
        }

        ContactCommand::Search { query, limit } => {
            let spinner = io.progress("Searching contacts…");
            let action = SearchContacts {
                query,
                limit: limit.unwrap_or(0),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;
            format_contact_list(io, &value);
        }

        ContactCommand::Create {
            name,
            email,
            phone,
            company,
            notes,
            address_book,
        } => {
            let spinner = io.progress("Creating contact…");
            let action = CreateContact {
                name: name.clone(),
                emails: parse_typed_values(&email, "address"),
                phones: parse_typed_values(&phone, "number"),
                company: company.unwrap_or_default(),
                notes: notes.unwrap_or_default(),
                address_book_id: address_book.unwrap_or_default(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;

            if io.mode() == OutputMode::Human {
                let id = value
                    .get("contactId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                io.done(&format!("Created contact \"{name}\" (ID: {id})"));
            } else {
                io.json(&value);
            }
        }

        ContactCommand::Update {
            contact_id,
            name,
            email,
            phone,
            company,
            notes,
        } => {
            let spinner = io.progress("Updating contact…");
            let action = UpdateContact {
                contact_id,
                name: name.clone().unwrap_or_default(),
                emails: parse_typed_values(&email, "address"),
                phones: parse_typed_values(&phone, "number"),
                company: company.clone().unwrap_or_default(),
                notes: notes.clone().unwrap_or_default(),
                has_emails: !email.is_empty(),
                has_phones: !phone.is_empty(),
                has_company: company.is_some(),
                has_notes: notes.is_some(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            result?;

            if io.mode() == OutputMode::Human {
                io.done("Contact updated");
            } else {
                io.json(&serde_json::json!({ "success": true }));
            }
        }

        ContactCommand::Delete { contact_id } => {
            let spinner = io.progress("Deleting contact…");
            let action = DeleteContact {
                contact_id: contact_id.clone(),
            };
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            result?;

            if io.mode() == OutputMode::Human {
                io.done(&format!("Deleted contact {contact_id}"));
            } else {
                io.json(&serde_json::json!({ "success": true }));
            }
        }

        ContactCommand::AddressBooks => {
            let spinner = io.progress("Fetching address books…");
            let action = ListAddressBooks;
            let result = action.run(ctx);
            Io::finish_progress(spinner);
            let value = result?;

            if io.mode() != OutputMode::Human {
                io.json(&value);
                return Ok(());
            }

            let books = match value.as_array() {
                Some(arr) => arr,
                None => {
                    io.json(&value);
                    return Ok(());
                }
            };

            if books.is_empty() {
                io.warn("No address books found");
                return Ok(());
            }

            io.done(&format!("{} address book(s)", books.len()));
            io.separator();

            io.data(&format!(
                "{:<40} {:<24} {:<10} {}",
                "ID", "NAME", "DEFAULT", "DESCRIPTION"
            ));
            io.data(&format!("{}", "─".repeat(90)));

            for book in books {
                let id = book.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let name = book.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let is_default = book
                    .get("isDefault")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let desc = book
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                io.data(&format!(
                    "{:<40} {:<24} {:<10} {}",
                    id,
                    name,
                    if is_default { "yes" } else { "" },
                    desc
                ));
            }
        }
    }
    Ok(())
}

fn format_contact_list(io: &Io, value: &serde_json::Value) {
    if io.mode() != OutputMode::Human {
        io.json(value);
        return;
    }

    let contacts = match value.as_array() {
        Some(arr) => arr,
        None => {
            io.json(value);
            return;
        }
    };

    if contacts.is_empty() {
        io.warn("No contacts found");
        return;
    }

    io.done(&format!("{} contact(s)", contacts.len()));
    io.separator();

    io.data(&format!(
        "{:<40} {:<24} {:<28} {}",
        "ID", "NAME", "EMAIL", "COMPANY"
    ));
    io.data(&format!("{}", "─".repeat(100)));

    for contact in contacts {
        let id = contact.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let name = contact
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let email = contact
            .get("emails")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("address"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let company = contact
            .get("company")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        io.data(&format!(
            "{:<40} {:<24} {:<28} {}",
            id, name, email, company
        ));
    }
}

/// Parse "type:value" or "value" strings into JSON objects.
/// e.g. "work:a@b.com" → {"type": "work", "address": "a@b.com"}
/// e.g. "+1234" → {"type": "other", "number": "+1234"}
fn parse_typed_values(inputs: &[String], value_key: &str) -> Vec<serde_json::Value> {
    inputs
        .iter()
        .map(|s| {
            let (ctx_type, value) = match s.split_once(':') {
                Some((t, v)) if t == "work" || t == "private" => (t, v),
                _ => ("other", s.as_str()),
            };
            serde_json::json!({ "type": ctx_type, value_key: value })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typed_values_with_type_prefix() {
        let input = vec!["work:a@b.com".to_string()];
        let result = parse_typed_values(&input, "address");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "work");
        assert_eq!(result[0]["address"], "a@b.com");
    }

    #[test]
    fn parse_typed_values_without_prefix() {
        let input = vec!["a@b.com".to_string()];
        let result = parse_typed_values(&input, "address");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "other");
        assert_eq!(result[0]["address"], "a@b.com");
    }

    #[test]
    fn parse_typed_values_private_prefix() {
        let input = vec!["private:+1234".to_string()];
        let result = parse_typed_values(&input, "number");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "private");
        assert_eq!(result[0]["number"], "+1234");
    }

    #[test]
    fn parse_typed_values_unknown_prefix_treated_as_value() {
        let input = vec!["home:+1234".to_string()];
        let result = parse_typed_values(&input, "number");
        assert_eq!(result[0]["type"], "other");
        assert_eq!(result[0]["number"], "home:+1234");
    }

    #[test]
    fn parse_typed_values_empty_input() {
        let result = parse_typed_values(&[], "address");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_typed_values_multiple() {
        let input = vec![
            "work:a@b.com".to_string(),
            "private:c@d.com".to_string(),
            "e@f.com".to_string(),
        ];
        let result = parse_typed_values(&input, "address");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0]["type"], "work");
        assert_eq!(result[1]["type"], "private");
        assert_eq!(result[2]["type"], "other");
    }
}
