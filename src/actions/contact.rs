use crate::actions::{check_set_errors, project_fields_array, Action, Context};
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

// -- JSContact helpers --

/// Flatten a JSContact ContactCard into a simple JSON shape for MCP consumers.
fn flatten_contact(card: &serde_json::Value) -> serde_json::Value {
    let id = card.get("id").cloned().unwrap_or(serde_json::json!(""));

    let name = extract_full_name(card);

    let emails = flatten_email_map(card.get("emails"));
    let phones = flatten_phone_map(card.get("phones"));
    let company = extract_company(card);

    let address_book_ids: Vec<&str> = card
        .get("addressBookIds")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().map(|k| k.as_str()).collect())
        .unwrap_or_default();

    serde_json::json!({
        "id": id,
        "name": name,
        "emails": emails,
        "phones": phones,
        "company": company,
        "addressBookIds": address_book_ids,
    })
}

/// Extract full name from JSContact Name object.
/// Prefers `name.full`, falls back to joining components.
fn extract_full_name(card: &serde_json::Value) -> String {
    let Some(name_obj) = card.get("name") else {
        return String::new();
    };

    if let Some(full) = name_obj.get("full").and_then(|v| v.as_str())
        && !full.is_empty()
    {
        return full.to_string();
    }

    name_obj
        .get("components")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.get("value").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// Flatten JSContact Id[EmailAddress] map into [{type, address}].
fn flatten_email_map(emails: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    let Some(map) = emails.and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    map.values()
        .filter_map(|entry| {
            let address = entry.get("address").and_then(|v| v.as_str())?;
            let ctx_type = context_type(entry);
            Some(serde_json::json!({ "type": ctx_type, "address": address }))
        })
        .collect()
}

/// Flatten JSContact Id[Phone] map into [{type, number}].
fn flatten_phone_map(phones: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    let Some(map) = phones.and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    map.values()
        .filter_map(|entry| {
            let number = entry.get("number").and_then(|v| v.as_str())?;
            let ctx_type = context_type(entry);
            Some(serde_json::json!({ "type": ctx_type, "number": number }))
        })
        .collect()
}

/// Derive a type label from JSContact `contexts` map.
fn context_type(entry: &serde_json::Value) -> &str {
    let Some(contexts) = entry.get("contexts").and_then(|v| v.as_object()) else {
        return "other";
    };

    if contexts.contains_key("work") {
        "work"
    } else if contexts.contains_key("private") {
        "private"
    } else {
        "other"
    }
}

/// Extract first organization name from JSContact organizations map.
fn extract_company(card: &serde_json::Value) -> String {
    card.get("organizations")
        .and_then(|v| v.as_object())
        .and_then(|m| m.values().next())
        .and_then(|org| org.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Build JSContact emails map from flat [{type, address}] array.
fn build_email_map(emails: &[serde_json::Value]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (i, entry) in emails.iter().enumerate() {
        let address = entry
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let ctx_type = entry
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("other");

        let mut email_obj = serde_json::json!({
            "@type": "EmailAddress",
            "address": address,
        });

        if ctx_type != "other" {
            email_obj["contexts"] = serde_json::json!({ ctx_type: true });
        }

        map.insert(format!("e{i}"), email_obj);
    }
    serde_json::Value::Object(map)
}

/// Build JSContact phones map from flat [{type, number}] array.
fn build_phone_map(phones: &[serde_json::Value]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (i, entry) in phones.iter().enumerate() {
        let number = entry
            .get("number")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let ctx_type = entry
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("other");

        let mut phone_obj = serde_json::json!({
            "@type": "Phone",
            "number": number,
        });

        if ctx_type != "other" {
            phone_obj["contexts"] = serde_json::json!({ ctx_type: true });
        }

        map.insert(format!("p{i}"), phone_obj);
    }
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
    pub emails: Vec<serde_json::Value>,
    pub phones: Vec<serde_json::Value>,
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
            card["emails"] = build_email_map(&self.emails);
        }

        if !self.phones.is_empty() {
            card["phones"] = build_phone_map(&self.phones);
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
    pub name: String,
    pub emails: Vec<serde_json::Value>,
    pub phones: Vec<serde_json::Value>,
    pub company: String,
    pub notes: String,
    pub has_emails: bool,
    pub has_phones: bool,
    pub has_company: bool,
    pub has_notes: bool,
}

impl Action for UpdateContact {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.contact_id.is_empty() {
            return Err(Error::InvalidParams("contactId is required".to_string()));
        }

        let has_updates = !self.name.is_empty()
            || self.has_emails
            || self.has_phones
            || self.has_company
            || self.has_notes;

        if !has_updates {
            return Err(Error::InvalidParams(
                "at least one field to update is required".to_string(),
            ));
        }

        let mut patches = serde_json::Map::new();

        if !self.name.is_empty() {
            patches.insert(
                "name".to_string(),
                serde_json::json!({ "@type": "Name", "full": self.name }),
            );
        }

        if self.has_emails {
            patches.insert("emails".to_string(), build_email_map(&self.emails));
        }

        if self.has_phones {
            patches.insert("phones".to_string(), build_phone_map(&self.phones));
        }

        if self.has_company {
            if self.company.is_empty() {
                patches.insert("organizations".to_string(), serde_json::json!(null));
            } else {
                patches.insert(
                    "organizations".to_string(),
                    serde_json::json!({
                        "o0": { "@type": "Organization", "name": self.company }
                    }),
                );
            }
        }

        if self.has_notes {
            if self.notes.is_empty() {
                patches.insert("notes".to_string(), serde_json::json!(null));
            } else {
                patches.insert("notes".to_string(), serde_json::json!(self.notes));
            }
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
    fn flatten_email_map_extracts_addresses() {
        let emails = json!({
            "e0": { "address": "a@b.com", "contexts": { "work": true } },
            "e1": { "address": "c@d.com", "contexts": { "private": true } },
            "e2": { "address": "e@f.com" },
        });
        let result = flatten_email_map(Some(&emails));
        assert_eq!(result.len(), 3);

        let addresses: Vec<&str> = result
            .iter()
            .filter_map(|e| e.get("address").and_then(|v| v.as_str()))
            .collect();
        assert!(addresses.contains(&"a@b.com"));
        assert!(addresses.contains(&"c@d.com"));
        assert!(addresses.contains(&"e@f.com"));
    }

    #[test]
    fn flatten_email_map_returns_empty_for_none() {
        assert!(flatten_email_map(None).is_empty());
    }

    #[test]
    fn flatten_phone_map_extracts_numbers() {
        let phones = json!({
            "p0": { "number": "+1234", "contexts": { "work": true } },
        });
        let result = flatten_phone_map(Some(&phones));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["number"], "+1234");
        assert_eq!(result[0]["type"], "work");
    }

    #[test]
    fn extract_company_gets_first_org() {
        let card = json!({
            "organizations": {
                "o0": { "name": "Acme Corp" }
            }
        });
        assert_eq!(extract_company(&card), "Acme Corp");
    }

    #[test]
    fn extract_company_returns_empty_when_missing() {
        let card = json!({});
        assert_eq!(extract_company(&card), "");
    }

    #[test]
    fn build_email_map_creates_jscontact_structure() {
        let input = vec![
            json!({ "type": "work", "address": "a@b.com" }),
            json!({ "address": "c@d.com" }),
        ];
        let map = build_email_map(&input);
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
    fn build_phone_map_creates_jscontact_structure() {
        let input = vec![json!({ "type": "private", "number": "+5678" })];
        let map = build_phone_map(&input);
        let obj = map.as_object().expect("should be object");
        assert_eq!(obj.len(), 1);
        assert_eq!(obj["p0"]["number"], "+5678");
        assert_eq!(obj["p0"]["contexts"]["private"], true);
    }

    #[test]
    fn context_type_returns_work() {
        let entry = json!({ "contexts": { "work": true } });
        assert_eq!(context_type(&entry), "work");
    }

    #[test]
    fn context_type_returns_private() {
        let entry = json!({ "contexts": { "private": true } });
        assert_eq!(context_type(&entry), "private");
    }

    #[test]
    fn context_type_returns_other_when_missing() {
        let entry = json!({});
        assert_eq!(context_type(&entry), "other");
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
            emails: vec![json!({ "type": "work", "address": "bob@example.com" })],
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
            name: "New Name".to_string(),
            emails: Vec::new(),
            phones: Vec::new(),
            company: String::new(),
            notes: String::new(),
            has_emails: false,
            has_phones: false,
            has_company: false,
            has_notes: false,
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
            name: String::new(),
            emails: Vec::new(),
            phones: Vec::new(),
            company: String::new(),
            notes: String::new(),
            has_emails: false,
            has_phones: false,
            has_company: false,
            has_notes: false,
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
            name: "Updated Name".to_string(),
            emails: Vec::new(),
            phones: Vec::new(),
            company: String::new(),
            notes: String::new(),
            has_emails: false,
            has_phones: false,
            has_company: false,
            has_notes: false,
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
    fn build_email_map_empty_input() {
        let map = build_email_map(&[]);
        let obj = map.as_object().expect("should be object");
        assert!(obj.is_empty());
    }

    #[test]
    fn build_phone_map_empty_input() {
        let map = build_phone_map(&[]);
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
            name: String::new(),
            emails: Vec::new(),
            phones: Vec::new(),
            company: String::new(),
            notes: String::new(),
            has_emails: false,
            has_phones: false,
            has_company: true,
            has_notes: true,
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
            name: String::new(),
            emails: vec![json!({"type": "work", "address": "new@example.com"})],
            phones: vec![json!({"type": "private", "number": "+9999"})],
            company: String::new(),
            notes: String::new(),
            has_emails: true,
            has_phones: true,
            has_company: false,
            has_notes: false,
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
            name: "X".to_string(),
            emails: Vec::new(),
            phones: Vec::new(),
            company: String::new(),
            notes: String::new(),
            has_emails: false,
            has_phones: false,
            has_company: false,
            has_notes: false,
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
