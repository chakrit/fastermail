use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct JmapRequest {
    pub using: Vec<String>,
    #[serde(rename = "methodCalls")]
    pub method_calls: Vec<MethodCall>,
}

pub type MethodCall = (String, serde_json::Value, String);

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct JmapResponse {
    #[serde(rename = "methodResponses")]
    pub method_responses: Vec<MethodResponse>,
    #[serde(rename = "sessionState")]
    pub session_state: String,
}

pub type MethodResponse = (String, serde_json::Value, String);

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Session {
    pub username: String,
    #[serde(rename = "apiUrl")]
    pub api_url: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,
    #[serde(rename = "primaryAccounts")]
    pub primary_accounts: std::collections::HashMap<String, String>,
    pub accounts: std::collections::HashMap<String, Account>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Account {
    pub name: String,
    #[serde(rename = "isReadOnly")]
    pub is_read_only: bool,
}

impl Session {
    pub fn primary_account_id(&self) -> Option<&str> {
        self.primary_accounts
            .get("urn:ietf:params:jmap:core")
            .map(|s| s.as_str())
    }
}

/// Build a back-reference for chaining JMAP method calls.
pub fn back_reference(result_of: &str, name: &str, path: &str) -> serde_json::Value {
    serde_json::json!({
        "resultOf": result_of,
        "name": name,
        "path": path
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_reference_builds_correct_structure() {
        let bref = back_reference("call-0", "Email/query", "/ids");

        assert_eq!(bref["resultOf"], "call-0");
        assert_eq!(bref["name"], "Email/query");
        assert_eq!(bref["path"], "/ids");
    }

    #[test]
    fn primary_account_id_returns_core_account() {
        let mut session = Session::default();
        session.primary_accounts.insert(
            "urn:ietf:params:jmap:core".to_string(),
            "acct-123".to_string(),
        );
        session.primary_accounts.insert(
            "urn:ietf:params:jmap:mail".to_string(),
            "acct-456".to_string(),
        );

        assert_eq!(session.primary_account_id(), Some("acct-123"));
    }

    #[test]
    fn primary_account_id_returns_none_when_empty() {
        let session = Session::default();

        assert!(session.primary_account_id().is_none());
    }
}
