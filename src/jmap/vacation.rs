use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::jmap::client::JmapClient;

const VACATION_CAPABILITY: &str = "urn:ietf:params:jmap:vacationresponse";

/// A JMAP `VacationResponse` id. The resource is a singleton, so JMAP uses the literal id
/// `"singleton"`; the newtype keeps it from being confused with other id strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VacationResponseId(pub String);

impl VacationResponseId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VacationResponseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Faithful mirror of a JMAP `VacationResponse` object: a zero-loss read shape. The id is
/// typed; every other property JMAP returns — `isEnabled`, `fromDate`, `toDate`,
/// `subject`, `textBody`, `htmlBody`, and any field FastMail adds later — is preserved
/// verbatim in `rest`. JMAP names 1:1. Presenters (CLI/MCP) decide what to project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VacationResponse {
    pub id: VacationResponseId,
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

/// Faithful mirror of a `VacationResponse/get` response. The singleton lives in `list`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VacationGetResponse {
    pub list: Vec<VacationResponse>,
    pub not_found: Vec<VacationResponseId>,
    pub state: String,
}

/// Faithful mirror of a `VacationResponse/set` response. `updated` maps the singleton id
/// to the server-completed object (or null); each `not_*` map carries a JMAP `SetError`
/// per id that failed. No projection — every field is preserved verbatim.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VacationSetResponse {
    pub old_state: Option<String>,
    pub new_state: Option<String>,
    pub updated: serde_json::Map<String, serde_json::Value>,
    pub not_created: serde_json::Map<String, serde_json::Value>,
    pub not_updated: serde_json::Map<String, serde_json::Value>,
    pub not_destroyed: serde_json::Map<String, serde_json::Value>,
}

impl VacationSetResponse {
    /// Surface the first partial-failure `SetError` (notCreated → notUpdated →
    /// notDestroyed, the order JMAP documents), mirroring [`EmailSetResponse::check_errors`].
    /// `Ok(())` when the whole batch succeeded.
    ///
    /// [`EmailSetResponse::check_errors`]: crate::jmap::email::EmailSetResponse::check_errors
    pub fn check_errors(&self, method: &str) -> Result<()> {
        for (key, map) in [
            ("notCreated", &self.not_created),
            ("notUpdated", &self.not_updated),
            ("notDestroyed", &self.not_destroyed),
        ] {
            if let Some(err) = map.values().next() {
                let desc = err
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or(key);
                return Err(Error::Jmap {
                    method: method.to_string(),
                    message: desc.to_string(),
                });
            }
        }
        Ok(())
    }
}

impl JmapClient {
    /// L1 accessor: a faithful `VacationResponse/get` — no projection. The vacation
    /// response is a singleton; JMAP returns it in `list`. Every property is preserved
    /// (typed id plus [`VacationResponse::rest`]).
    pub fn vacation_get(&self, account_id: &str) -> Result<VacationGetResponse> {
        let data = self.call_one(
            VACATION_CAPABILITY,
            "VacationResponse/get",
            serde_json::json!({ "accountId": account_id }),
        )?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }

    /// L1 accessor: a faithful single `VacationResponse/set` update of the singleton, no
    /// projection. `update` is the JMAP patch written verbatim under the `"singleton"`
    /// id; the caller builds it. Mirrors [`JmapClient::email_set`]'s update half.
    pub fn vacation_set(
        &self,
        account_id: &str,
        update: serde_json::Value,
    ) -> Result<VacationSetResponse> {
        let args = serde_json::json!({
            "accountId": account_id,
            "update": { "singleton": update },
        });
        let data = self.call_one(VACATION_CAPABILITY, "VacationResponse/set", args)?;
        let response = serde_json::from_value(data)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    fn client(mock: &MockJmap) -> JmapClient {
        let (client, _) =
            JmapClient::connect_to(&mock.session_url(), "fake-token").expect("session connect");
        client
    }

    #[test]
    fn vacation_get_returns_faithful_singleton_with_flattened_extras() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/get",
            json!({
                "methodResponses": [["VacationResponse/get", {
                    "state": "s1",
                    "list": [{
                        "id": "singleton",
                        "isEnabled": true,
                        "fromDate": "2026-01-01T00:00:00Z",
                        "toDate": "2026-01-15T00:00:00Z",
                        "subject": "OOO",
                        "textBody": "Away",
                        "htmlBody": "<p>Away</p>"
                    }],
                    "notFound": []
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .vacation_get(TEST_ACCOUNT_ID)
            .expect("vacation_get");

        assert_eq!(resp.list.len(), 1);
        let vacation = &resp.list[0];
        assert_eq!(vacation.id, VacationResponseId("singleton".into()));
        // Every non-id field survives verbatim in `rest` — nothing is dropped, including
        // `htmlBody`, which the L3 presenter projects out.
        assert_eq!(vacation.rest.get("isEnabled"), Some(&json!(true)));
        assert_eq!(
            vacation.rest.get("fromDate"),
            Some(&json!("2026-01-01T00:00:00Z"))
        );
        assert_eq!(vacation.rest.get("subject"), Some(&json!("OOO")));
        assert_eq!(vacation.rest.get("textBody"), Some(&json!("Away")));
        assert_eq!(vacation.rest.get("htmlBody"), Some(&json!("<p>Away</p>")));
        // The typed id is consumed by its field, not duplicated into `rest`.
        assert!(!vacation.rest.contains_key("id"));
        assert_eq!(resp.state, "s1");
    }

    #[test]
    fn vacation_get_returns_empty_for_no_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/get",
            json!({"methodResponses": [["VacationResponse/get", {"list": []}, "call-0"]]}),
        );

        let resp = client(&mock)
            .vacation_get(TEST_ACCOUNT_ID)
            .expect("vacation_get");
        assert!(resp.list.is_empty());
    }

    #[test]
    fn vacation_set_updates_singleton() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/set",
            json!({
                "methodResponses": [["VacationResponse/set", {
                    "oldState": "s1",
                    "newState": "s2",
                    "updated": {"singleton": null}
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .vacation_set(TEST_ACCOUNT_ID, json!({ "isEnabled": true }))
            .expect("vacation_set");
        assert_eq!(resp.new_state, Some("s2".to_string()));
        assert!(resp.updated.contains_key("singleton"));
        resp.check_errors("VacationResponse/set")
            .expect("no set errors");
    }

    #[test]
    fn vacation_set_surfaces_not_updated_set_errors() {
        let mock = MockJmap::start();
        mock.handle_method(
            "VacationResponse/set",
            json!({
                "methodResponses": [["VacationResponse/set", {
                    "updated": {},
                    "notUpdated": { "singleton": { "type": "invalidProperties", "description": "bad" } }
                }, "call-0"]]
            }),
        );

        let resp = client(&mock)
            .vacation_set(TEST_ACCOUNT_ID, json!({ "isEnabled": true }))
            .expect("vacation_set");
        let err = resp
            .check_errors("VacationResponse/set")
            .expect_err("set error should surface");
        assert!(
            err.to_string().contains("bad"),
            "error should carry the SetError description: {err}"
        );
    }
}
