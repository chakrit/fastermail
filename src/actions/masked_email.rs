use crate::actions::{project_fields, project_fields_array, Action, Context};
use crate::error::{Error, Result};
use crate::mcp::types::Tool;

const LIST_FIELDS: &[&str] = &["id", "email", "forDomain", "description", "state", "createdAt"];
const CREATE_FIELDS: &[&str] = &["id", "email"];

const CAPABILITY: &str = "https://www.fastmail.com/dev/maskedemail";

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list_masked_emails".to_string(),
            description: "List all masked (disposable) email addresses".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "state": {
                        "type": "string",
                        "description": "Filter: pending, enabled, disabled, deleted"
                    }
                }
            }),
        },
        Tool {
            name: "create_masked_email".to_string(),
            description: "Create a new masked email address".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "forDomain": { "type": "string", "description": "Domain this address is for" },
                    "description": { "type": "string", "description": "Human-readable label" },
                    "emailPrefix": { "type": "string", "description": "Preferred prefix for the address" }
                }
            }),
        },
        Tool {
            name: "update_masked_email".to_string(),
            description: "Enable, disable, or delete a masked email address".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Masked email ID" },
                    "state": { "type": "string", "description": "New state: enabled, disabled, deleted" }
                },
                "required": ["id", "state"]
            }),
        },
    ]
}

pub struct ListMaskedEmails {
    pub state: String,
}

impl Action for ListMaskedEmails {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        // Validate state before making the JMAP call to avoid wasting a round trip.
        if !self.state.is_empty() {
            let valid_states = ["pending", "enabled", "disabled", "deleted"];
            if !valid_states.contains(&self.state.as_str()) {
                return Err(Error::InvalidParams(
                    "state must be pending, enabled, disabled, or deleted".to_string(),
                ));
            }
        }

        let data = ctx.jmap.call_one(
            CAPABILITY,
            "MaskedEmail/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        let list = data.get("list").cloned().unwrap_or(serde_json::json!([]));

        if self.state.is_empty() {
            return Ok(project_fields_array(&list, LIST_FIELDS));
        }

        let filtered: Vec<&serde_json::Value> = list
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|m| m.get("state").and_then(|s| s.as_str()) == Some(&self.state))
                    .collect()
            })
            .unwrap_or_default();

        Ok(project_fields_array(&serde_json::json!(filtered), LIST_FIELDS))
    }
}

pub struct CreateMaskedEmail {
    pub for_domain: String,
    pub description: String,
    pub email_prefix: String,
}

impl Action for CreateMaskedEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let mut create_obj = serde_json::json!({ "state": "enabled" });

        if !self.for_domain.is_empty() {
            create_obj["forDomain"] = serde_json::json!(self.for_domain);
        }
        if !self.description.is_empty() {
            create_obj["description"] = serde_json::json!(self.description);
        }
        if !self.email_prefix.is_empty() {
            create_obj["emailPrefix"] = serde_json::json!(self.email_prefix);
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "create": { "new-masked": create_obj }
        });

        let data = ctx.jmap.call_one(CAPABILITY, "MaskedEmail/set", args)?;

        let created = data
            .get("created")
            .and_then(|c| c.get("new-masked"))
            .cloned()
            .unwrap_or(serde_json::json!({}));

        Ok(project_fields(&created, CREATE_FIELDS))
    }
}

pub struct UpdateMaskedEmail {
    pub id: String,
    pub state: String,
}

impl Action for UpdateMaskedEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.id.is_empty() {
            return Err(Error::InvalidParams("id is required".to_string()));
        }

        if self.state.is_empty() {
            return Err(Error::InvalidParams("state is required".to_string()));
        }

        let valid_states = ["enabled", "disabled", "deleted"];
        if !valid_states.contains(&self.state.as_str()) {
            return Err(Error::InvalidParams(
                "state must be enabled, disabled, or deleted".to_string(),
            ));
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "update": {
                self.id.clone(): { "state": self.state }
            }
        });

        ctx.jmap.call_one(CAPABILITY, "MaskedEmail/set", args)?;

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

    #[test]
    fn list_masked_emails_returns_all() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/get",
            json!({
                "methodResponses": [["MaskedEmail/get", {
                    "list": [
                        {
                            "id": "me1",
                            "email": "abc@fastmail.com",
                            "forDomain": "example.com",
                            "description": "Test",
                            "state": "enabled",
                            "createdAt": "2026-01-01",
                            "extraField": "ignored"
                        },
                        {
                            "id": "me2",
                            "email": "def@fastmail.com",
                            "forDomain": "other.com",
                            "description": "Second",
                            "state": "disabled",
                            "createdAt": "2026-02-01",
                            "anotherExtra": "also ignored"
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

        let result = ListMaskedEmails { state: String::new() }
            .run(&ctx)
            .expect("run");
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 2);

        for item in arr {
            let obj = item.as_object().expect("object");
            for key in obj.keys() {
                assert!(
                    LIST_FIELDS.contains(&key.as_str()),
                    "unexpected field: {key}"
                );
            }
        }

        assert_eq!(arr[0]["id"], "me1");
        assert_eq!(arr[0]["email"], "abc@fastmail.com");
        assert_eq!(arr[0]["forDomain"], "example.com");
        assert_eq!(arr[0]["description"], "Test");
        assert_eq!(arr[0]["state"], "enabled");
        assert_eq!(arr[0]["createdAt"], "2026-01-01");
        assert!(arr[0].get("extraField").is_none());

        assert_eq!(arr[1]["id"], "me2");
        assert_eq!(arr[1]["email"], "def@fastmail.com");
        assert!(arr[1].get("anotherExtra").is_none());
    }

    #[test]
    fn list_masked_emails_filters_by_state() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/get",
            json!({
                "methodResponses": [["MaskedEmail/get", {
                    "list": [
                        {
                            "id": "me1",
                            "email": "abc@fastmail.com",
                            "forDomain": "example.com",
                            "description": "Enabled one",
                            "state": "enabled",
                            "createdAt": "2026-01-01"
                        },
                        {
                            "id": "me2",
                            "email": "def@fastmail.com",
                            "forDomain": "other.com",
                            "description": "Disabled one",
                            "state": "disabled",
                            "createdAt": "2026-02-01"
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

        let result = ListMaskedEmails { state: "enabled".to_string() }
            .run(&ctx)
            .expect("run");
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "me1");
        assert_eq!(arr[0]["state"], "enabled");
    }

    #[test]
    fn list_masked_emails_rejects_invalid_state() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = ListMaskedEmails { state: "bogus".to_string() }
            .run(&ctx)
            .expect_err("should reject invalid state");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn create_masked_email_succeeds() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {
                    "created": {
                        "new-masked": {
                            "id": "me-new",
                            "email": "new@fastmail.com"
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

        let result = CreateMaskedEmail {
            for_domain: String::new(),
            description: String::new(),
            email_prefix: String::new(),
        }
        .run(&ctx)
        .expect("run");

        let obj = result.as_object().expect("object");
        assert_eq!(obj["id"], "me-new");
        assert_eq!(obj["email"], "new@fastmail.com");
        for key in obj.keys() {
            assert!(
                CREATE_FIELDS.contains(&key.as_str()),
                "unexpected field: {key}"
            );
        }
    }

    #[test]
    fn create_masked_email_includes_optional_fields() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {
                    "created": {
                        "new-masked": {
                            "id": "me-opt",
                            "email": "opt@fastmail.com"
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

        let result = CreateMaskedEmail {
            for_domain: "mysite.com".to_string(),
            description: "My site login".to_string(),
            email_prefix: "mysite".to_string(),
        }
        .run(&ctx)
        .expect("run");

        assert_eq!(result["id"], "me-opt");
        assert_eq!(result["email"], "opt@fastmail.com");
    }

    #[test]
    fn update_masked_email_requires_id() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = UpdateMaskedEmail {
            id: String::new(),
            state: "enabled".to_string(),
        }
        .run(&ctx)
        .expect_err("should require id");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn update_masked_email_requires_state() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = UpdateMaskedEmail {
            id: "me1".to_string(),
            state: String::new(),
        }
        .run(&ctx)
        .expect_err("should require state");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn update_masked_email_rejects_invalid_state() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let err = UpdateMaskedEmail {
            id: "me1".to_string(),
            state: "bogus".to_string(),
        }
        .run(&ctx)
        .expect_err("should reject invalid state");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn update_masked_email_succeeds() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {
                    "updated": {
                        "me1": null
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

        let result = UpdateMaskedEmail {
            id: "me1".to_string(),
            state: "disabled".to_string(),
        }
        .run(&ctx)
        .expect("run");

        assert_eq!(result["success"], true);
    }
}
