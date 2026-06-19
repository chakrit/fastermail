use crate::actions::{project_fields, Action, Context};
use crate::error::Result;
use crate::mcp::types::Tool;

const GET_FIELDS: &[&str] = &["isEnabled", "fromDate", "toDate", "subject", "textBody", "htmlBody"];

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "get_vacation_response".to_string(),
            description: "Get the current vacation/auto-reply settings".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "set_vacation_response".to_string(),
            description: "Enable, disable, or update the vacation auto-reply".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "isEnabled": { "type": "boolean", "description": "Enable or disable auto-reply" },
                    "fromDate": { "type": "string", "description": "Start date (ISO 8601, UTC)" },
                    "toDate": { "type": "string", "description": "End date (ISO 8601, UTC)" },
                    "subject": { "type": "string", "description": "Auto-reply subject" },
                    "textBody": { "type": "string", "description": "Plain text auto-reply body" },
                    "htmlBody": { "type": "string", "description": "HTML auto-reply body" }
                },
                "required": ["isEnabled"]
            }),
        },
    ]
}

pub struct GetVacationResponse;

impl Action for GetVacationResponse {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let data = ctx.jmap.call_one(
            "urn:ietf:params:jmap:vacationresponse",
            "VacationResponse/get",
            serde_json::json!({ "accountId": ctx.account_id }),
        )?;

        let vacation = data
            .get("list")
            .and_then(|l| l.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        Ok(project_fields(&vacation, GET_FIELDS))
    }
}

/// A field-level change in a vacation update.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum FieldChange {
    /// Field not provided — leave it unchanged.
    #[default]
    Leave,
    /// Provided empty/null — clear it (set to JSON null).
    Clear,
    /// Provided a value — set it.
    Set(String),
}

impl FieldChange {
    /// From an MCP argument: absent -> Leave; null or empty string -> Clear; else Set.
    pub fn from_arg(args: &serde_json::Value, key: &str) -> Self {
        match args.get(key) {
            None => Self::Leave,
            Some(v) if v.is_null() => Self::Clear,
            Some(v) => Self::from_opt(Some(v.as_str().unwrap_or("").to_string())),
        }
    }

    /// From a CLI optional argument: None -> Leave; empty -> Clear; else Set.
    pub fn from_opt(value: Option<String>) -> Self {
        match value {
            None => Self::Leave,
            Some(s) if s.is_empty() => Self::Clear,
            Some(s) => Self::Set(s),
        }
    }

    /// The JSON to write into the update patch, or None to omit the field.
    fn patch_value(&self) -> Option<serde_json::Value> {
        match self {
            Self::Leave => None,
            Self::Clear => Some(serde_json::Value::Null),
            Self::Set(s) => Some(serde_json::json!(s)),
        }
    }
}

#[derive(Default)]
pub struct SetVacationResponse {
    pub is_enabled: Option<bool>,
    pub from_date: FieldChange,
    pub to_date: FieldChange,
    pub subject: FieldChange,
    pub text_body: FieldChange,
    pub html_body: FieldChange,
}

impl Action for SetVacationResponse {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let is_enabled = self.is_enabled.ok_or_else(|| {
            crate::error::Error::InvalidParams("isEnabled is required".to_string())
        })?;

        let mut update = serde_json::json!({ "isEnabled": is_enabled });

        let fields = [
            ("fromDate", &self.from_date),
            ("toDate", &self.to_date),
            ("subject", &self.subject),
            ("textBody", &self.text_body),
            ("htmlBody", &self.html_body),
        ];
        for (key, change) in fields {
            if let Some(value) = change.patch_value() {
                update[key] = value;
            }
        }

        let args = serde_json::json!({
            "accountId": ctx.account_id,
            "update": { "singleton": update }
        });

        ctx.jmap.call_one(
            "urn:ietf:params:jmap:vacationresponse",
            "VacationResponse/set",
            args,
        )?;

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
    fn get_vacation_response_returns_projected_fields() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/get",
            json!({"methodResponses": [["VacationResponse/get", {"list": [{"isEnabled": true, "fromDate": "2026-01-01", "toDate": "2026-01-15", "subject": "OOO", "textBody": "Away", "htmlBody": "<p>Away</p>", "extraField": "ignored"}]}, "call-0"]]}),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = GetVacationResponse.run(&ctx).expect("run should succeed");

        assert_eq!(result["isEnabled"], true);
        assert_eq!(result["fromDate"], "2026-01-01");
        assert_eq!(result["toDate"], "2026-01-15");
        assert_eq!(result["subject"], "OOO");
        assert_eq!(result["textBody"], "Away");
        assert_eq!(result["htmlBody"], "<p>Away</p>");
        assert!(result.get("extraField").is_none(), "extraField must be excluded");
    }

    #[test]
    fn get_vacation_response_returns_empty_when_no_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/get",
            json!({"methodResponses": [["VacationResponse/get", {"list": []}, "call-0"]]}),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let result = GetVacationResponse.run(&ctx).expect("run should succeed");

        assert_eq!(result, json!({}));
    }

    #[test]
    fn set_vacation_response_requires_is_enabled() {
        let client = JmapClient::new("http://localhost:0".to_string(), "fake".to_string());
        let ctx = Context {
            jmap: client,
            account_id: "test".to_string(),
            recorder: None,
        };

        let action = SetVacationResponse {
            is_enabled: None,
            ..Default::default()
        };

        let err = action.run(&ctx).expect_err("should fail without isEnabled");
        assert!(
            err.to_string().contains("isEnabled"),
            "error should mention isEnabled: {err}"
        );
    }

    #[test]
    fn set_vacation_response_enables_successfully() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/set",
            json!({"methodResponses": [["VacationResponse/set", {"updated": {"singleton": null}}, "call-0"]]}),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let action = SetVacationResponse {
            is_enabled: Some(true),
            ..Default::default()
        };

        let result = action.run(&ctx).expect("run should succeed");
        assert_eq!(result["success"], true);
    }

    #[test]
    fn set_vacation_response_disables_successfully() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/set",
            json!({"methodResponses": [["VacationResponse/set", {"updated": {"singleton": null}}, "call-0"]]}),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let action = SetVacationResponse {
            is_enabled: Some(false),
            ..Default::default()
        };

        let result = action.run(&ctx).expect("run should succeed");
        assert_eq!(result["success"], true);
    }

    #[test]
    fn field_change_leaves_absent() {
        let args = json!({"other": "value"});
        assert_eq!(FieldChange::from_arg(&args, "fromDate"), FieldChange::Leave);
    }

    #[test]
    fn field_change_clears_on_null() {
        let args = json!({"fromDate": null});
        assert_eq!(FieldChange::from_arg(&args, "fromDate"), FieldChange::Clear);
    }

    #[test]
    fn field_change_clears_on_empty_string() {
        let args = json!({"subject": ""});
        assert_eq!(FieldChange::from_arg(&args, "subject"), FieldChange::Clear);
    }

    #[test]
    fn field_change_sets_non_empty() {
        let args = json!({"subject": "hello"});
        assert_eq!(
            FieldChange::from_arg(&args, "subject"),
            FieldChange::Set("hello".to_string())
        );
    }

    #[test]
    fn set_vacation_response_includes_optional_fields() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/set",
            json!({"methodResponses": [["VacationResponse/set", {"updated": {"singleton": null}}, "call-0"]]}),
        );

        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        };

        let action = SetVacationResponse {
            is_enabled: Some(true),
            from_date: FieldChange::Set("2026-06-01T00:00:00Z".to_string()),
            subject: FieldChange::Set("On vacation".to_string()),
            text_body: FieldChange::Set("I'm away".to_string()),
            ..Default::default()
        };
        let result = action
            .run(&ctx)
            .expect("set with optional fields should succeed");
        assert_eq!(result["success"], true);
    }
}
