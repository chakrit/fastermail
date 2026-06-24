use crate::actions::{Action, Context};
use crate::error::Result;
use crate::mcp::types::Tool;
use crate::present::FieldChange;

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
    /// Return the faithful `VacationResponse` singleton — every JMAP property verbatim, no
    /// projection. The CLI/MCP presenters apply `present::project_vacation`. An empty
    /// account (no singleton in `list`) returns `{}`.
    fn run(&self, ctx: &Context) -> Result<serde_json::Value> {
        let response = ctx.jmap.vacation_get(&ctx.account_id)?;
        match response.list.into_iter().next() {
            Some(vacation) => Ok(serde_json::to_value(vacation)?),
            None => Ok(serde_json::json!({})),
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

        let update = crate::present::build_vacation_update(
            is_enabled,
            &self.from_date,
            &self.to_date,
            &self.subject,
            &self.text_body,
            &self.html_body,
        );

        let response = ctx.jmap.vacation_set(&ctx.account_id, update)?;
        response.check_errors("VacationResponse/set")?;

        // Faithful: the server-completed `updated` map. The `{success:true}` wrapper the
        // front-ends emit is an L3 concern (`present::set_ok`), not the action's output.
        Ok(serde_json::to_value(response.updated)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Context;
    use crate::jmap::client::JmapClient;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    fn ctx(mock: &MockJmap) -> Context {
        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session");
        Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
            recorder: None,
        }
    }

    #[test]
    fn get_vacation_response_returns_faithful_unprojected_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/get",
            json!({"methodResponses": [["VacationResponse/get", {"list": [{"id": "singleton", "isEnabled": true, "fromDate": "2026-01-01", "toDate": "2026-01-15", "subject": "OOO", "textBody": "Away", "htmlBody": "<p>Away</p>"}]}, "call-0"]]}),
        );

        let result = GetVacationResponse.run(&ctx(&mock)).expect("run");

        // The action no longer projects: the typed `id` and the field the presenter later
        // drops (`htmlBody`) are still present.
        assert_eq!(result["id"], "singleton");
        assert_eq!(result["isEnabled"], true);
        assert_eq!(result["fromDate"], "2026-01-01");
        assert_eq!(result["toDate"], "2026-01-15");
        assert_eq!(result["subject"], "OOO");
        assert_eq!(result["textBody"], "Away");
        assert_eq!(result["htmlBody"], "<p>Away</p>");
    }

    #[test]
    fn get_vacation_response_returns_empty_when_no_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/get",
            json!({"methodResponses": [["VacationResponse/get", {"list": []}, "call-0"]]}),
        );

        let result = GetVacationResponse
            .run(&ctx(&mock))
            .expect("run should succeed");

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

        let action = SetVacationResponse {
            is_enabled: Some(true),
            ..Default::default()
        };

        action.run(&ctx(&mock)).expect("run should succeed");
    }

    #[test]
    fn set_vacation_response_surfaces_set_errors() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/set",
            json!({"methodResponses": [["VacationResponse/set", {"updated": {}, "notUpdated": {"singleton": {"type": "invalidProperties", "description": "bad date"}}}, "call-0"]]}),
        );

        let action = SetVacationResponse {
            is_enabled: Some(true),
            ..Default::default()
        };

        let err = action
            .run(&ctx(&mock))
            .expect_err("set error should surface");
        assert!(
            err.to_string().contains("bad date"),
            "error should carry the SetError description: {err}"
        );
    }

    #[test]
    fn set_vacation_response_includes_optional_fields() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/set",
            json!({"methodResponses": [["VacationResponse/set", {"updated": {"singleton": null}}, "call-0"]]}),
        );

        let action = SetVacationResponse {
            is_enabled: Some(true),
            from_date: FieldChange::Set("2026-06-01T00:00:00Z".to_string()),
            subject: FieldChange::Set("On vacation".to_string()),
            text_body: FieldChange::Set("I'm away".to_string()),
            ..Default::default()
        };

        action
            .run(&ctx(&mock))
            .expect("set with optional fields should succeed");
    }
}
