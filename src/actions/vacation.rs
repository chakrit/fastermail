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
            .unwrap_or(serde_json::json!({}));

        Ok(project_fields(&vacation, GET_FIELDS))
    }
}

pub struct SetVacationResponse {
    pub is_enabled: Option<bool>,
    pub raw_args: serde_json::Value,
}

impl SetVacationResponse {
    /// If a key is present in the raw arguments, return its value for the JMAP update.
    /// Present with null or empty string -> set to null (clear the field).
    /// Present with a non-empty string -> set to that string.
    /// Absent -> don't include (leave unchanged).
    fn resolve_field(args: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
        match args.get(key) {
            None => None,
            Some(v) if v.is_null() => Some(serde_json::Value::Null),
            Some(v) => {
                let s = v.as_str().unwrap_or("");
                if s.is_empty() {
                    Some(serde_json::Value::Null)
                } else {
                    Some(serde_json::json!(s))
                }
            }
        }
    }
}

impl Action for SetVacationResponse {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let is_enabled = self.is_enabled.ok_or_else(|| {
            crate::error::Error::InvalidParams("isEnabled is required".to_string())
        })?;

        let mut update = serde_json::json!({
            "isEnabled": is_enabled
        });

        let fields = &["fromDate", "toDate", "subject", "textBody", "htmlBody"];
        for &field in fields {
            if let Some(value) = Self::resolve_field(&self.raw_args, field) {
                update[field] = value;
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
            raw_args: json!({}),
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
            raw_args: json!({"isEnabled": true}),
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
            raw_args: json!({"isEnabled": false}),
        };

        let result = action.run(&ctx).expect("run should succeed");
        assert_eq!(result["success"], true);
    }

    #[test]
    fn resolve_field_returns_none_for_absent() {
        let args = json!({"other": "value"});
        let result = SetVacationResponse::resolve_field(&args, "fromDate");
        assert!(result.is_none(), "absent key should return None");
    }

    #[test]
    fn resolve_field_returns_null_for_null_value() {
        let args = json!({"fromDate": null});
        let result = SetVacationResponse::resolve_field(&args, "fromDate");
        assert_eq!(
            result.expect("should be Some"),
            serde_json::Value::Null,
            "null value should resolve to Null"
        );
    }

    #[test]
    fn resolve_field_returns_null_for_empty_string() {
        let args = json!({"subject": ""});
        let result = SetVacationResponse::resolve_field(&args, "subject");
        assert_eq!(
            result.expect("should be Some"),
            serde_json::Value::Null,
            "empty string should resolve to Null"
        );
    }

    #[test]
    fn resolve_field_returns_value_for_non_empty() {
        let args = json!({"subject": "hello"});
        let result = SetVacationResponse::resolve_field(&args, "subject");
        assert_eq!(
            result.expect("should be Some"),
            json!("hello"),
            "non-empty string should resolve to that string"
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
            raw_args: json!({"isEnabled": true, "fromDate": "2026-06-01T00:00:00Z", "subject": "On vacation", "textBody": "I'm away"}),
        };
        let result = action
            .run(&ctx)
            .expect("set with optional fields should succeed");
        assert_eq!(result["success"], true);
    }
}
