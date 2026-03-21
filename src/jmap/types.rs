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
    fn parse_session_response() {
        let json = r#"{
            "username": "user@fastmail.com",
            "apiUrl": "https://api.fastmail.com/jmap/api/",
            "downloadUrl": "https://api.fastmail.com/jmap/download/{accountId}/{blobId}/{name}",
            "uploadUrl": "https://api.fastmail.com/jmap/upload/{accountId}/",
            "primaryAccounts": {
                "urn:ietf:params:jmap:core": "abc123",
                "urn:ietf:params:jmap:mail": "abc123"
            },
            "accounts": {
                "abc123": { "name": "user@fastmail.com", "isReadOnly": false }
            }
        }"#;

        let session: Session =
            serde_json::from_str(json).expect("should parse session");

        assert_eq!(session.api_url, "https://api.fastmail.com/jmap/api/");
        assert_eq!(
            session.primary_account_id(),
            Some("abc123")
        );
        assert!(!session.accounts["abc123"].is_read_only);
    }

    #[test]
    fn parse_jmap_response() {
        let json = r#"{
            "methodResponses": [
                ["Mailbox/get", {"accountId": "abc", "list": []}, "call-0"]
            ],
            "sessionState": "state1"
        }"#;

        let resp: JmapResponse =
            serde_json::from_str(json).expect("should parse JMAP response");

        assert_eq!(resp.method_responses.len(), 1);
        assert_eq!(resp.method_responses[0].0, "Mailbox/get");
        assert_eq!(resp.method_responses[0].2, "call-0");
    }

    #[test]
    fn serialize_jmap_request() {
        let req = JmapRequest {
            using: vec![
                "urn:ietf:params:jmap:core".to_string(),
                "urn:ietf:params:jmap:mail".to_string(),
            ],
            method_calls: vec![(
                "Mailbox/get".to_string(),
                serde_json::json!({"accountId": "abc"}),
                "call-0".to_string(),
            )],
        };

        let json = serde_json::to_string(&req).expect("should serialize JMAP request");

        assert!(json.contains("\"using\""));
        assert!(json.contains("\"methodCalls\""));
        assert!(json.contains("Mailbox/get"));
    }

    #[test]
    fn back_reference_builds_correctly() {
        let bref = back_reference("call-0", "Email/query", "/ids");

        assert_eq!(bref["resultOf"], "call-0");
        assert_eq!(bref["name"], "Email/query");
        assert_eq!(bref["path"], "/ids");
    }

    #[test]
    fn session_without_primary_account() {
        let session = Session::default();
        assert!(session.primary_account_id().is_none());
    }
}
