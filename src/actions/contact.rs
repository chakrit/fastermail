use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::actions::{check_set_errors, project_fields_array, Action, Context};
use crate::json;
use crate::error::{Error, Result};
use crate::jmap::types::back_reference;
use crate::mcp::types::Tool;

const CAPABILITY: &str = "urn:ietf:params:jmap:contacts";

/// Default max results when the caller leaves `limit` unset (0).
const DEFAULT_GET_LIMIT: u32 = 50;
const DEFAULT_SEARCH_LIMIT: u32 = 20;

const AB_LIST_FIELDS: &[&str] = &["id", "name", "description", "isDefault"];

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list_address_books".to_string(),
            description: "List all address books".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "get_contacts".to_string(),
            description: "Get contacts from an address book".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "addressBookId": {
                        "type": "string",
                        "description": "Filter by address book ID"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default 50)"
                    }
                }
            }),
        },
        Tool {
            name: "search_contacts".to_string(),
            description: "Search contacts by name, email, phone, etc.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search text (searches all fields)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default 20)"
                    }
                },
                "required": ["query"]
            }),
        },
        Tool {
            name: "create_contact".to_string(),
            description: "Create a new contact".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Full name"
                    },
                    "emails": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "description": "work, private, or other" },
                                "address": { "type": "string", "description": "Email address" }
                            },
                            "required": ["address"]
                        },
                        "description": "Email addresses"
                    },
                    "phones": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "description": "work, private, or other" },
                                "number": { "type": "string", "description": "Phone number" }
                            },
                            "required": ["number"]
                        },
                        "description": "Phone numbers"
                    },
                    "company": {
                        "type": "string",
                        "description": "Organization name"
                    },
                    "notes": {
                        "type": "string",
                        "description": "Free-text notes"
                    },
                    "addressBookId": {
                        "type": "string",
                        "description": "Target address book (default if omitted)"
                    }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "update_contact".to_string(),
            description: "Update an existing contact".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "contactId": {
                        "type": "string",
                        "description": "Contact ID"
                    },
                    "name": {
                        "type": "string",
                        "description": "Updated full name"
                    },
                    "emails": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string" },
                                "address": { "type": "string" }
                            },
                            "required": ["address"]
                        },
                        "description": "Updated email addresses (replaces all)"
                    },
                    "phones": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string" },
                                "number": { "type": "string" }
                            },
                            "required": ["number"]
                        },
                        "description": "Updated phone numbers (replaces all)"
                    },
                    "company": {
                        "type": "string",
                        "description": "Updated organization name"
                    },
                    "notes": {
                        "type": "string",
                        "description": "Updated notes"
                    }
                },
                "required": ["contactId"]
            }),
        },
        Tool {
            name: "delete_contact".to_string(),
            description: "Delete a contact".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "contactId": {
                        "type": "string",
                        "description": "Contact ID to delete"
                    }
                },
                "required": ["contactId"]
            }),
        },
    ]
}

// -- Wire layer: JSContact ContactCard (RFC 9553), faithful deserialization --

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct WireCard {
    id: String,
    name: WireName,
    emails: BTreeMap<String, WireEmail>,
    phones: BTreeMap<String, WirePhone>,
    organizations: BTreeMap<String, WireOrg>,
    #[serde(rename = "addressBookIds")]
    address_book_ids: BTreeMap<String, bool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct WireName {
    full: String,
    components: Vec<WireNameComponent>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct WireNameComponent {
    value: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct WireEmail {
    address: String,
    contexts: BTreeMap<String, bool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct WirePhone {
    number: String,
    contexts: BTreeMap<String, bool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct WireOrg {
    name: String,
}

// -- Internal model: the flat shape MCP/CLI consumers see --

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ContactContext {
    Work,
    Private,
    Other,
}

impl ContactContext {
    /// JSContact records context as a `{ "work": true }` map; first recognized
    /// key wins, preserving the original precedence.
    fn from_map(contexts: &BTreeMap<String, bool>) -> Self {
        if contexts.contains_key("work") {
            Self::Work
        } else if contexts.contains_key("private") {
            Self::Private
        } else {
            Self::Other
        }
    }

    /// Parse the `type` label from a create/update input; unknown or absent maps to Other.
    fn from_label(label: &str) -> Self {
        match label {
            "work" => Self::Work,
            "private" => Self::Private,
            _ => Self::Other,
        }
    }

    /// The JSContact `contexts` key to emit, or None for Other (which omits contexts).
    fn wire_key(self) -> Option<&'static str> {
        match self {
            Self::Work => Some("work"),
            Self::Private => Some("private"),
            Self::Other => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ContactEmail {
    #[serde(rename = "type")]
    context: ContactContext,
    address: String,
}

impl ContactEmail {
    /// Parse a create/update input object (`{ "type", "address" }`).
    pub fn from_input(v: &serde_json::Value) -> Self {
        ContactEmail {
            context: ContactContext::from_label(json::str_at(v, "/type").unwrap_or("")),
            address: json::str_at(v, "/address").unwrap_or("").to_string(),
        }
    }

    /// Serialize to a JSContact EmailAddress object.
    fn to_wire(&self) -> serde_json::Value {
        let mut obj = serde_json::json!({ "@type": "EmailAddress", "address": self.address });
        if let Some(key) = self.context.wire_key() {
            obj["contexts"] = serde_json::json!({ key: true });
        }
        obj
    }
}

#[derive(Debug, Serialize)]
pub struct ContactPhone {
    #[serde(rename = "type")]
    context: ContactContext,
    number: String,
}

impl ContactPhone {
    /// Parse a create/update input object (`{ "type", "number" }`).
    pub fn from_input(v: &serde_json::Value) -> Self {
        ContactPhone {
            context: ContactContext::from_label(json::str_at(v, "/type").unwrap_or("")),
            number: json::str_at(v, "/number").unwrap_or("").to_string(),
        }
    }

    /// Serialize to a JSContact Phone object.
    fn to_wire(&self) -> serde_json::Value {
        let mut obj = serde_json::json!({ "@type": "Phone", "number": self.number });
        if let Some(key) = self.context.wire_key() {
            obj["contexts"] = serde_json::json!({ key: true });
        }
        obj
    }
}

/// A partial contact update: `None` leaves a field unchanged, `Some` sets it
/// (`Some` empty string / list clears it).
#[derive(Debug, Default)]
pub struct ContactPatch {
    pub name: Option<String>,
    pub emails: Option<Vec<ContactEmail>>,
    pub phones: Option<Vec<ContactPhone>>,
    pub company: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
struct Contact {
    id: String,
    name: String,
    emails: Vec<ContactEmail>,
    phones: Vec<ContactPhone>,
    company: String,
    #[serde(rename = "addressBookIds")]
    address_book_ids: Vec<String>,
}

impl From<WireCard> for Contact {
    fn from(card: WireCard) -> Self {
        let name = if card.name.full.is_empty() {
            card.name
                .components
                .iter()
                .map(|c| c.value.as_str())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            card.name.full
        };

        let emails = card
            .emails
            .into_values()
            .filter(|e| !e.address.is_empty())
            .map(|e| ContactEmail {
                context: ContactContext::from_map(&e.contexts),
                address: e.address,
            })
            .collect();

        let phones = card
            .phones
            .into_values()
            .filter(|p| !p.number.is_empty())
            .map(|p| ContactPhone {
                context: ContactContext::from_map(&p.contexts),
                number: p.number,
            })
            .collect();

        let company = card
            .organizations
            .into_values()
            .next()
            .map(|o| o.name)
            .unwrap_or_default();

        Contact {
            id: card.id,
            name,
            emails,
            phones,
            company,
            address_book_ids: card.address_book_ids.into_keys().collect(),
        }
    }
}

/// Flatten a JSContact ContactCard into the simple shape MCP/CLI consumers expect.
fn flatten_contact(card: &serde_json::Value) -> serde_json::Value {
    let wire: WireCard = serde_json::from_value(card.clone()).unwrap_or_default();
    serde_json::to_value(Contact::from(wire)).unwrap_or_else(|_| serde_json::json!({}))
}

// -- model -> JSContact wire (writes) --

/// Build JSContact emails map from typed emails.
fn email_map(emails: &[ContactEmail]) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = emails
        .iter()
        .enumerate()
        .map(|(i, e)| (format!("e{i}"), e.to_wire()))
        .collect();
    serde_json::Value::Object(map)
}

/// Build JSContact phones map from typed phones.
fn phone_map(phones: &[ContactPhone]) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = phones
        .iter()
        .enumerate()
        .map(|(i, p)| (format!("p{i}"), p.to_wire()))
        .collect();
    serde_json::Value::Object(map)
}

// -- Actions --

pub struct ListAddressBooks;

impl Action for ListAddressBooks {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let data = ctx.jmap.call_one(
            CAPABILITY,
            "AddressBook/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        let list = data.get("list").cloned().unwrap_or(serde_json::json!([]));
        Ok(project_fields_array(&list, AB_LIST_FIELDS))
    }
}

pub struct GetContacts {
    pub address_book_id: String,
    pub limit: u32,
}

/// Run a `ContactCard/query` → `ContactCard/get` pipeline for a filter, returning the
/// flattened contact list. The caller resolves `limit` (Get and Search differ in default).
fn query_and_flatten(
    ctx: &Context,
    filter: serde_json::Value,
    limit: u32,
) -> Result<serde_json::Value> {
    let using = vec!["urn:ietf:params:jmap:core".to_string(), CAPABILITY.to_string()];

    let query_args = serde_json::json!({
        "accountId": ctx.account_id,
        "filter": filter,
        "sort": [{ "property": "updated", "isAscending": false }],
        "limit": limit,
    });

    let get_args = serde_json::json!({
        "accountId": ctx.account_id,
        "#ids": back_reference("call-0", "ContactCard/query", "/ids"),
    });

    let method_calls = vec![
        ("ContactCard/query".to_string(), query_args, "call-0".to_string()),
        ("ContactCard/get".to_string(), get_args, "call-1".to_string()),
    ];

    let resp = ctx.jmap.call(using, method_calls)?;

    let list = resp
        .method_responses
        .iter()
        .find(|(m, _, _)| m == "ContactCard/get")
        .map(|(_, data, _)| data.get("list").cloned().unwrap_or(serde_json::json!([])))
        .unwrap_or(serde_json::json!([]));

    let contacts: Vec<serde_json::Value> = list
        .as_array()
        .map(|arr| arr.iter().map(flatten_contact).collect())
        .unwrap_or_default();

    Ok(serde_json::json!(contacts))
}

impl Action for GetContacts {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let limit = if self.limit == 0 {
            DEFAULT_GET_LIMIT
        } else {
            self.limit
        };

        let mut filter = serde_json::json!({});
        if !self.address_book_id.is_empty() {
            filter["inAddressBook"] = serde_json::json!(self.address_book_id);
        }

        query_and_flatten(ctx, filter, limit)
    }
}

pub struct SearchContacts {
    pub query: String,
    pub limit: u32,
}

impl Action for SearchContacts {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.query.is_empty() {
            return Err(Error::InvalidParams("query is required".to_string()));
        }

        let limit = if self.limit == 0 {
            DEFAULT_SEARCH_LIMIT
        } else {
            self.limit
        };

        let filter = serde_json::json!({ "text": self.query });
        query_and_flatten(ctx, filter, limit)
    }
}

pub struct CreateContact {
    pub name: String,
    pub emails: Vec<ContactEmail>,
    pub phones: Vec<ContactPhone>,
    pub company: String,
    pub notes: String,
    pub address_book_id: String,
}

impl Action for CreateContact {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.name.is_empty() {
            return Err(Error::InvalidParams("name is required".to_string()));
        }

        let mut card = serde_json::json!({
            "@type": "Card",
            "version": "1.0",
            "name": { "@type": "Name", "full": self.name },
        });

        if !self.emails.is_empty() {
            card["emails"] = email_map(&self.emails);
        }

        if !self.phones.is_empty() {
            card["phones"] = phone_map(&self.phones);
        }

        if !self.company.is_empty() {
            card["organizations"] = serde_json::json!({
                "o0": { "@type": "Organization", "name": self.company }
            });
        }

        if !self.notes.is_empty() {
            card["notes"] = serde_json::json!(self.notes);
        }

        if !self.address_book_id.is_empty() {
            card["addressBookIds"] = serde_json::json!({ self.address_book_id.clone(): true });
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "create": { "new-contact": card },
        });

        let data = ctx.jmap.call_one(CAPABILITY, "ContactCard/set", args)?;
        check_set_errors(&data, "ContactCard/set")?;

        let contact_id = data
            .get("created")
            .and_then(|c| c.get("new-contact"))
            .and_then(|c| c.get("id"))
            .cloned()
            .unwrap_or(serde_json::json!(null));

        Ok(serde_json::json!({ "contactId": contact_id }))
    }
}

pub struct UpdateContact {
    pub contact_id: String,
    pub patch: ContactPatch,
}

impl Action for UpdateContact {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.contact_id.is_empty() {
            return Err(Error::InvalidParams("contactId is required".to_string()));
        }

        let p = &self.patch;
        let has_updates = p.name.is_some()
            || p.emails.is_some()
            || p.phones.is_some()
            || p.company.is_some()
            || p.notes.is_some();

        if !has_updates {
            return Err(Error::InvalidParams(
                "at least one field to update is required".to_string(),
            ));
        }

        let mut patches = serde_json::Map::new();

        if let Some(name) = &p.name {
            patches.insert(
                "name".to_string(),
                serde_json::json!({ "@type": "Name", "full": name }),
            );
        }

        if let Some(emails) = &p.emails {
            patches.insert("emails".to_string(), email_map(emails));
        }

        if let Some(phones) = &p.phones {
            patches.insert("phones".to_string(), phone_map(phones));
        }

        if let Some(company) = &p.company {
            let org = if company.is_empty() {
                serde_json::json!(null)
            } else {
                serde_json::json!({ "o0": { "@type": "Organization", "name": company } })
            };
            patches.insert("organizations".to_string(), org);
        }

        if let Some(notes) = &p.notes {
            let value = if notes.is_empty() {
                serde_json::json!(null)
            } else {
                serde_json::json!(notes)
            };
            patches.insert("notes".to_string(), value);
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "update": { self.contact_id.clone(): serde_json::Value::Object(patches) },
        });

        let data = ctx.jmap.call_one(CAPABILITY, "ContactCard/set", args)?;
        check_set_errors(&data, "ContactCard/set")?;

        Ok(serde_json::json!({ "success": true }))
    }
}

pub struct DeleteContact {
    pub contact_id: String,
}

impl Action for DeleteContact {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.contact_id.is_empty() {
            return Err(Error::InvalidParams("contactId is required".to_string()));
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "destroy": [self.contact_id],
        });

        let data = ctx.jmap.call_one(CAPABILITY, "ContactCard/set", args)?;
        check_set_errors(&data, "ContactCard/set")?;

        Ok(serde_json::json!({ "success": true }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Context;
    use crate::jmap::client::JmapClient;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    // -- JSContact helper tests --

    #[test]
    fn flatten_contact_extracts_full_name() {
        let card = json!({
            "id": "c1",
            "name": { "full": "Alice Smith" },
            "addressBookIds": { "ab1": true },
        });
        let flat = flatten_contact(&card);
        assert_eq!(flat["name"], "Alice Smith");
        assert_eq!(flat["id"], "c1");
    }

    #[test]
    fn flatten_contact_joins_name_components() {
        let card = json!({
            "id": "c2",
            "name": {
                "components": [
                    { "kind": "given", "value": "Bob" },
                    { "kind": "surname", "value": "Jones" }
                ]
            },
        });
        let flat = flatten_contact(&card);
        assert_eq!(flat["name"], "Bob Jones");
    }

    #[test]
    fn flatten_contact_handles_missing_name() {
        let card = json!({ "id": "c3" });
        let flat = flatten_contact(&card);
        assert_eq!(flat["name"], "");
    }

    #[test]
    fn flatten_contact_extracts_emails_phones_company() {
        let card = json!({
            "id": "c1",
            "name": { "full": "Jane Roe" },
            "emails": {
                "e0": { "address": "a@b.com", "contexts": { "work": true } },
                "e1": { "address": "c@d.com" }
            },
            "phones": { "p0": { "number": "+1234", "contexts": { "private": true } } },
            "organizations": { "o0": { "name": "Acme Corp" } }
        });
        let flat = flatten_contact(&card);

        assert_eq!(flat["name"], "Jane Roe");
        assert_eq!(flat["company"], "Acme Corp");

        let emails = flat["emails"].as_array().expect("emails array");
        assert_eq!(emails.len(), 2);
        let work = emails
            .iter()
            .find(|e| e["address"] == "a@b.com")
            .expect("work email");
        assert_eq!(work["type"], "work");
        let other = emails
            .iter()
            .find(|e| e["address"] == "c@d.com")
            .expect("other email");
        assert_eq!(other["type"], "other");

        let phones = flat["phones"].as_array().expect("phones array");
        assert_eq!(phones.len(), 1);
        assert_eq!(phones[0]["number"], "+1234");
        assert_eq!(phones[0]["type"], "private");
    }

    #[test]
    fn email_map_creates_jscontact_structure() {
        let input = vec![
            ContactEmail::from_input(&json!({ "type": "work", "address": "a@b.com" })),
            ContactEmail::from_input(&json!({ "address": "c@d.com" })),
        ];
        let map = email_map(&input);
        let obj = map.as_object().expect("should be object");
        assert_eq!(obj.len(), 2);

        let e0 = &obj["e0"];
        assert_eq!(e0["address"], "a@b.com");
        assert_eq!(e0["contexts"]["work"], true);

        let e1 = &obj["e1"];
        assert_eq!(e1["address"], "c@d.com");
        assert!(e1.get("contexts").is_none(), "other type should omit contexts");
    }

    #[test]
    fn phone_map_creates_jscontact_structure() {
        let input = vec![ContactPhone::from_input(
            &json!({ "type": "private", "number": "+5678" }),
        )];
        let map = phone_map(&input);
        let obj = map.as_object().expect("should be object");
        assert_eq!(obj.len(), 1);
        assert_eq!(obj["p0"]["number"], "+5678");
        assert_eq!(obj["p0"]["contexts"]["private"], true);
    }

    // -- Action tests --

    #[test]
    fn list_address_books_returns_projected_fields() {
        let mock = MockJmap::start();
        mock.handle_method(
            "AddressBook/get",
            json!({
                "methodResponses": [["AddressBook/get", {
                    "list": [
                        {
                            "id": "ab1",
                            "name": "Personal",
                            "description": null,
                            "isDefault": true,
                            "sortOrder": 0,
                            "myRights": { "mayRead": true }
                        }
                    ]
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = ListAddressBooks.run(&ctx).expect("run");
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "ab1");
        assert_eq!(arr[0]["name"], "Personal");
        assert_eq!(arr[0]["isDefault"], true);
        assert!(arr[0].get("sortOrder").is_none(), "extra fields stripped");
        assert!(arr[0].get("myRights").is_none(), "extra fields stripped");
    }

    #[test]
    fn list_address_books_returns_empty() {
        let mock = MockJmap::start();
        mock.handle_method(
            "AddressBook/get",
            json!({"methodResponses": [["AddressBook/get", {"list": []}, "call-0"]]}),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = ListAddressBooks.run(&ctx).expect("run");
        let arr = result.as_array().expect("array");
        assert!(arr.is_empty());
    }

    #[test]
    fn get_contacts_flattens_jscontact() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/query",
            json!({
                "methodResponses": [
                    ["ContactCard/query", { "ids": ["c1"] }, "call-0"],
                    ["ContactCard/get", {
                        "list": [{
                            "id": "c1",
                            "name": { "full": "Alice" },
                            "emails": {
                                "e0": { "address": "alice@example.com", "contexts": { "work": true } }
                            },
                            "phones": {
                                "p0": { "number": "+1234", "contexts": { "private": true } }
                            },
                            "organizations": {
                                "o0": { "name": "Acme" }
                            },
                            "addressBookIds": { "ab1": true }
                        }]
                    }, "call-1"]
                ]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = GetContacts {
            address_book_id: String::new(),
            limit: 0,
        }
        .run(&ctx)
        .expect("run");

        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "c1");
        assert_eq!(arr[0]["name"], "Alice");
        assert_eq!(arr[0]["company"], "Acme");

        let emails = arr[0]["emails"].as_array().expect("emails array");
        assert_eq!(emails[0]["address"], "alice@example.com");
        assert_eq!(emails[0]["type"], "work");

        let phones = arr[0]["phones"].as_array().expect("phones array");
        assert_eq!(phones[0]["number"], "+1234");
        assert_eq!(phones[0]["type"], "private");

        let ab_ids = arr[0]["addressBookIds"].as_array().expect("addressBookIds");
        assert_eq!(ab_ids, &[json!("ab1")]);
    }

    #[test]
    fn search_contacts_requires_query() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = SearchContacts {
            query: String::new(),
            limit: 0,
        }
        .run(&ctx)
        .expect_err("should require query");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn search_contacts_returns_results() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/query",
            json!({
                "methodResponses": [
                    ["ContactCard/query", { "ids": ["c1"] }, "call-0"],
                    ["ContactCard/get", {
                        "list": [{
                            "id": "c1",
                            "name": { "full": "Alice Smith" },
                            "addressBookIds": { "ab1": true }
                        }]
                    }, "call-1"]
                ]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = SearchContacts {
            query: "alice".to_string(),
            limit: 0,
        }
        .run(&ctx)
        .expect("run");

        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Alice Smith");
    }

    #[test]
    fn create_contact_requires_name() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = CreateContact {
            name: String::new(),
            emails: Vec::new(),
            phones: Vec::new(),
            company: String::new(),
            notes: String::new(),
            address_book_id: String::new(),
        }
        .run(&ctx)
        .expect_err("should require name");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn create_contact_returns_id() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/set",
            json!({
                "methodResponses": [["ContactCard/set", {
                    "created": { "new-contact": { "id": "c-new" } }
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = CreateContact {
            name: "Bob".to_string(),
            emails: vec![ContactEmail::from_input(
                &json!({ "type": "work", "address": "bob@example.com" }),
            )],
            phones: Vec::new(),
            company: "Acme".to_string(),
            notes: "A note".to_string(),
            address_book_id: String::new(),
        }
        .run(&ctx)
        .expect("run");

        assert_eq!(result["contactId"], "c-new");
    }

    #[test]
    fn update_contact_requires_contact_id() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = UpdateContact {
            contact_id: String::new(),
            patch: ContactPatch {
                name: Some("New Name".to_string()),
                ..Default::default()
            },
        }
        .run(&ctx)
        .expect_err("should require contactId");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn update_contact_requires_at_least_one_field() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = UpdateContact {
            contact_id: "c1".to_string(),
            patch: ContactPatch::default(),
        }
        .run(&ctx)
        .expect_err("should require at least one field");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn update_contact_succeeds() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/set",
            json!({
                "methodResponses": [["ContactCard/set", {
                    "updated": { "c1": null }
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = UpdateContact {
            contact_id: "c1".to_string(),
            patch: ContactPatch {
                name: Some("Updated Name".to_string()),
                ..Default::default()
            },
        }
        .run(&ctx)
        .expect("run");

        assert_eq!(result["success"], true);
    }

    #[test]
    fn delete_contact_requires_contact_id() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = DeleteContact {
            contact_id: String::new(),
        }
        .run(&ctx)
        .expect_err("should require contactId");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn delete_contact_succeeds() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/set",
            json!({
                "methodResponses": [["ContactCard/set", {
                    "destroyed": ["c1"]
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = DeleteContact {
            contact_id: "c1".to_string(),
        }
        .run(&ctx)
        .expect("run");

        assert_eq!(result["success"], true);
    }

    #[test]
    fn get_contacts_returns_empty_list() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/query",
            json!({
                "methodResponses": [
                    ["ContactCard/query", { "ids": [] }, "call-0"],
                    ["ContactCard/get", { "list": [] }, "call-1"]
                ]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = GetContacts {
            address_book_id: "ab1".to_string(),
            limit: 10,
        }
        .run(&ctx)
        .expect("run");

        let arr = result.as_array().expect("array");
        assert!(arr.is_empty());
    }

    #[test]
    fn flatten_contact_with_no_emails_phones_orgs() {
        let card = json!({
            "id": "c-sparse",
            "name": { "full": "Sparse Contact" },
            "addressBookIds": { "ab1": true },
        });
        let flat = flatten_contact(&card);
        assert_eq!(flat["id"], "c-sparse");
        assert_eq!(flat["name"], "Sparse Contact");
        assert!(flat["emails"].as_array().expect("emails").is_empty());
        assert!(flat["phones"].as_array().expect("phones").is_empty());
        assert_eq!(flat["company"], "");
    }

    #[test]
    fn email_map_empty_input() {
        let map = email_map(&[]);
        let obj = map.as_object().expect("should be object");
        assert!(obj.is_empty());
    }

    #[test]
    fn phone_map_empty_input() {
        let map = phone_map(&[]);
        let obj = map.as_object().expect("should be object");
        assert!(obj.is_empty());
    }

    #[test]
    fn update_contact_clears_company_with_null() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/set",
            json!({
                "methodResponses": [["ContactCard/set", {
                    "updated": { "c1": null }
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = UpdateContact {
            contact_id: "c1".to_string(),
            patch: ContactPatch {
                company: Some(String::new()),
                notes: Some(String::new()),
                ..Default::default()
            },
        }
        .run(&ctx)
        .expect("clearing fields should succeed");

        assert_eq!(result["success"], true);
    }

    #[test]
    fn update_contact_with_emails_and_phones() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/set",
            json!({
                "methodResponses": [["ContactCard/set", {
                    "updated": { "c1": null }
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = UpdateContact {
            contact_id: "c1".to_string(),
            patch: ContactPatch {
                emails: Some(vec![ContactEmail::from_input(
                    &json!({"type": "work", "address": "new@example.com"}),
                )]),
                phones: Some(vec![ContactPhone::from_input(
                    &json!({"type": "private", "number": "+9999"}),
                )]),
                ..Default::default()
            },
        }
        .run(&ctx)
        .expect("update with emails/phones should succeed");

        assert_eq!(result["success"], true);
    }

    #[test]
    fn update_contact_surfaces_set_error() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/set",
            json!({
                "methodResponses": [["ContactCard/set", {
                    "notUpdated": {
                        "c1": {
                            "type": "notFound",
                            "description": "contact not found"
                        }
                    }
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let err = UpdateContact {
            contact_id: "c1".to_string(),
            patch: ContactPatch {
                name: Some("X".to_string()),
                ..Default::default()
            },
        }
        .run(&ctx)
        .expect_err("should surface notUpdated error");

        let msg = err.to_string();
        assert!(msg.contains("contact not found"), "error: {msg}");
    }

    #[test]
    fn delete_contact_surfaces_set_error() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/set",
            json!({
                "methodResponses": [["ContactCard/set", {
                    "notDestroyed": {
                        "c1": {
                            "type": "notFound",
                            "description": "contact not found"
                        }
                    }
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let err = DeleteContact {
            contact_id: "c1".to_string(),
        }
        .run(&ctx)
        .expect_err("should surface notDestroyed error");

        let msg = err.to_string();
        assert!(msg.contains("contact not found"), "error: {msg}");
    }

    #[test]
    fn create_contact_surfaces_set_error() {
        let mock = MockJmap::start();
        mock.handle_method(
            "ContactCard/set",
            json!({
                "methodResponses": [["ContactCard/set", {
                    "notCreated": {
                        "new-contact": {
                            "type": "invalidProperties",
                            "description": "name is too long"
                        }
                    }
                }, "call-0"]]
            }),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let err = CreateContact {
            name: "A".to_string(),
            emails: Vec::new(),
            phones: Vec::new(),
            company: String::new(),
            notes: String::new(),
            address_book_id: String::new(),
        }
        .run(&ctx)
        .expect_err("should surface set error");

        let msg = err.to_string();
        assert!(msg.contains("name is too long"), "error: {msg}");
    }
}
