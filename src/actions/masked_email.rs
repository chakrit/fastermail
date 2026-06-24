use crate::actions::{Action, Context};
use crate::error::{Error, Result};
use crate::jmap::masked_email::MaskedEmailId;
use crate::mcp::types::Tool;
use crate::present::MaskedEmailState;

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

/// List masked emails. Returns the faithful, unfiltered `MaskedEmail/get` list; the L3
/// presenter selects the view fields and applies any state filter.
pub struct ListMaskedEmails;

impl Action for ListMaskedEmails {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let resp = ctx.jmap.masked_email_get(&ctx.account_id)?;
        Ok(serde_json::to_value(resp.list)?)
    }
}

/// Create a masked email. Returns the faithful `MaskedEmail/set` response; the L3 presenter
/// projects the created object down to the asymmetric `{id, email}` create view.
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

        let resp = ctx.jmap.masked_email_set(
            &ctx.account_id,
            Some(serde_json::json!({ "new-masked": create_obj })),
            None,
            &[],
        )?;
        resp.check_errors("MaskedEmail/set")?;
        Ok(serde_json::to_value(resp)?)
    }
}

/// Enable/disable/delete a masked email. Returns the faithful `MaskedEmail/set` response;
/// the L3 presenter emits the `{success}` wrapper.
pub struct UpdateMaskedEmail {
    pub id: String,
    pub state: MaskedEmailState,
}

impl Action for UpdateMaskedEmail {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        if self.id.is_empty() {
            return Err(Error::InvalidParams("id is required".to_string()));
        }

        let update = serde_json::json!({
            self.id.clone(): { "state": self.state.label() }
        });

        let resp = ctx.jmap.masked_email_set(
            &ctx.account_id,
            None,
            Some(update),
            &[] as &[MaskedEmailId],
        )?;
        resp.check_errors("MaskedEmail/set")?;
        Ok(serde_json::to_value(resp)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Context;
    use crate::jmap::client::JmapClient;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    fn ctx_for(mock: &MockJmap) -> Context {
        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        }
    }

    #[test]
    fn list_masked_emails_returns_faithful_list() {
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
                            "lastMessageAt": "2026-03-01"
                        }
                    ]
                }, "call-0"]]
            }),
        );

        let ctx = ctx_for(&mock);
        let result = ListMaskedEmails.run(&ctx).expect("run");
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        // The action returns FAITHFUL data — extras the L3 presenter projects out are
        // still present here.
        assert_eq!(arr[0]["id"], "me1");
        assert_eq!(arr[0]["lastMessageAt"], "2026-03-01");
    }

    #[test]
    fn create_masked_email_returns_faithful_set_response() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {
                    "created": {
                        "new-masked": {
                            "id": "me-new",
                            "email": "new@fastmail.com",
                            "state": "enabled",
                            "forDomain": "mysite.com"
                        }
                    }
                }, "call-0"]]
            }),
        );

        let ctx = ctx_for(&mock);
        let result = CreateMaskedEmail {
            for_domain: "mysite.com".to_string(),
            description: String::new(),
            email_prefix: String::new(),
        }
        .run(&ctx)
        .expect("run");

        // FAITHFUL set response: the full created object survives (the L3 create presenter
        // projects it down to {id,email}).
        assert_eq!(result["created"]["new-masked"]["id"], "me-new");
        assert_eq!(result["created"]["new-masked"]["forDomain"], "mysite.com");
    }

    #[test]
    fn create_masked_email_surfaces_set_errors() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {
                    "created": {},
                    "notCreated": { "new-masked": { "type": "forbidden", "description": "nope" } }
                }, "call-0"]]
            }),
        );

        let ctx = ctx_for(&mock);
        let err = CreateMaskedEmail {
            for_domain: String::new(),
            description: String::new(),
            email_prefix: String::new(),
        }
        .run(&ctx)
        .expect_err("should surface set error");
        assert!(err.to_string().contains("nope"));
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
            state: MaskedEmailState::Enabled,
        }
        .run(&ctx)
        .expect_err("should require id");
        assert!(matches!(err, Error::InvalidParams(_)));
    }

    #[test]
    fn update_masked_email_returns_faithful_set_response() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {"updated": {"me1": null}}, "call-0"]]
            }),
        );

        let ctx = ctx_for(&mock);
        let result = UpdateMaskedEmail {
            id: "me1".to_string(),
            state: MaskedEmailState::Disabled,
        }
        .run(&ctx)
        .expect("run");

        assert!(
            result["updated"]
                .as_object()
                .expect("updated map")
                .contains_key("me1")
        );
    }

    #[test]
    fn update_masked_email_surfaces_set_errors() {
        let mock = MockJmap::start();
        mock.handle_method(
            "MaskedEmail/set",
            json!({
                "methodResponses": [["MaskedEmail/set", {
                    "updated": {},
                    "notUpdated": { "me1": { "type": "notFound", "description": "gone" } }
                }, "call-0"]]
            }),
        );

        let ctx = ctx_for(&mock);
        let err = UpdateMaskedEmail {
            id: "me1".to_string(),
            state: MaskedEmailState::Disabled,
        }
        .run(&ctx)
        .expect_err("should surface set error");
        assert!(err.to_string().contains("gone"));
    }
}
