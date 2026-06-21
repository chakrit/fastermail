use crate::error::{Error, Result};
use crate::jmap::types::{BlobId, JmapRequest, JmapResponse, MethodCall, Session};

/// Cap on a single blob download. Larger than FastMail's 50MB upload limit, so a
/// fully-attached inbound message still fits.
const MAX_BLOB_SIZE: u64 = 100 * 1024 * 1024;

#[derive(Debug)]
pub struct JmapClient {
    api_url: String,
    token: String,
    download_url: String,
}

impl JmapClient {
    /// Fetch the JMAP session and create a client.
    /// Uses `JMAP_SESSION_URL` env var if set, otherwise defaults to FastMail.
    pub fn connect(token: &str) -> Result<(Self, Session)> {
        let session_url = std::env::var("JMAP_SESSION_URL")
            .unwrap_or_else(|_| "https://api.fastmail.com/jmap/session".to_string());

        Self::connect_to(&session_url, token)
    }

    /// Fetch the JMAP session from a specific URL and create a client.
    pub fn connect_to(session_url: &str, token: &str) -> Result<(Self, Session)> {
        log_debug!("jmap", "fetching session from {session_url}");

        let mut resp = ureq::get(session_url)
            .header("Authorization", &format!("Bearer {token}"))
            .call()?;

        let session: Session = resp.body_mut().read_json()?;

        log_debug!("jmap", "session ok, apiUrl={}", session.api_url);

        let api_url = session.api_url.clone();
        let client = Self {
            api_url,
            token: token.to_string(),
            download_url: session.download_url.clone(),
        };

        Ok((client, session))
    }

    /// Create a client with a known API URL (for testing).
    #[cfg(test)]
    pub fn new(api_url: String, token: String) -> Self {
        Self {
            api_url,
            token,
            download_url: String::new(),
        }
    }

    /// Execute a JMAP request with the given capabilities and method calls.
    pub fn call(&self, using: Vec<String>, method_calls: Vec<MethodCall>) -> Result<JmapResponse> {
        let methods: Vec<&str> = method_calls.iter().map(|(m, _, _)| m.as_str()).collect();
        log_debug!("jmap", "call: {:?}", methods);

        let req = JmapRequest {
            using,
            method_calls,
        };

        let mut resp = ureq::post(&self.api_url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .send_json(&req)?;

        let jmap_resp: JmapResponse = resp.body_mut().read_json()?;

        log_trace!(
            "jmap",
            "response: {} method(s)",
            jmap_resp.method_responses.len()
        );

        Ok(jmap_resp)
    }

    /// Convenience: make a single JMAP method call with core + one capability.
    pub fn call_one(
        &self,
        capability: &str,
        method: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let using = vec![
            "urn:ietf:params:jmap:core".to_string(),
            capability.to_string(),
        ];
        let method_calls = vec![(method.to_string(), args, "call-0".to_string())];

        let resp = self.call(using, method_calls)?;

        let (resp_method, resp_data, _) =
            resp.method_responses
                .into_iter()
                .next()
                .ok_or_else(|| Error::Jmap {
                    method: method.to_string(),
                    message: "empty response".to_string(),
                })?;

        if resp_method == "error" {
            let err_msg = resp_data
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();

            log_error!("jmap", "{method} error: {err_msg}");

            return Err(Error::Jmap {
                method: method.to_string(),
                message: err_msg,
            });
        }

        Ok(resp_data)
    }

    /// Download a blob's raw bytes via the session `downloadUrl` template.
    /// `name` and `content_type` fill the template's `{name}`/`{type}` vars
    /// (presentation hints); `name` must be URL-safe.
    pub fn download_blob(
        &self,
        account_id: &str,
        blob_id: &BlobId,
        name: &str,
        content_type: &str,
    ) -> Result<Vec<u8>> {
        let url = self
            .download_url
            .replace("{accountId}", account_id)
            .replace("{blobId}", blob_id.as_str())
            .replace("{name}", name)
            .replace("{type}", content_type);

        log_debug!("jmap", "download blob {}", blob_id.as_str());

        let mut resp = ureq::get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .call()?;

        let bytes = resp
            .body_mut()
            .with_config()
            .limit(MAX_BLOB_SIZE)
            .read_to_vec()?;

        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    #[test]
    fn session_discovery_extracts_account_id() {
        let mock = MockJmap::start();

        let (_, session) = JmapClient::connect_to(&mock.session_url(), "fake-token")
            .expect("session discovery should succeed");

        assert_eq!(session.primary_account_id(), Some(TEST_ACCOUNT_ID));
    }

    #[test]
    fn session_discovery_extracts_api_url() {
        let mock = MockJmap::start();

        let (client, _) = JmapClient::connect_to(&mock.session_url(), "fake-token")
            .expect("session discovery should succeed");

        assert!(
            client.api_url.starts_with(&mock.base_url()),
            "API URL should point at mock server, got: {}",
            client.api_url
        );
    }

    #[test]
    fn session_fetch_with_401_returns_error() {
        let server = httpmock::MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::prelude::GET).path("/jmap/session");
            then.status(401);
        });

        let err =
            JmapClient::connect_to(&format!("{}/jmap/session", server.base_url()), "bad-token")
                .expect_err("401 should produce an error");

        let msg = format!("{err:?}");
        assert!(
            msg.contains("401") || msg.contains("Http"),
            "error should indicate auth failure: {msg}"
        );
    }

    #[test]
    fn call_one_returns_response_data() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Mailbox/get",
            json!({
                "methodResponses": [
                    ["Mailbox/get", {
                        "accountId": TEST_ACCOUNT_ID,
                        "list": [
                            { "id": "mbox-1", "name": "Inbox", "role": "inbox" }
                        ]
                    }, "call-0"]
                ]
            }),
        );

        let (client, _) = JmapClient::connect_to(&mock.session_url(), "fake-token")
            .expect("session should succeed");

        let result = client
            .call_one(
                "urn:ietf:params:jmap:mail",
                "Mailbox/get",
                json!({
                    "accountId": TEST_ACCOUNT_ID,
                    "ids": null
                }),
            )
            .expect("call_one should succeed");

        let list = result["list"].as_array().expect("should have list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["name"], "Inbox");
    }

    #[test]
    fn call_one_maps_jmap_error_response() {
        let mock = MockJmap::start();
        mock.handle_method(
            "Email/get",
            json!({
                "methodResponses": [
                    ["error", { "type": "notFound" }, "call-0"]
                ]
            }),
        );

        let (client, _) = JmapClient::connect_to(&mock.session_url(), "fake-token")
            .expect("session should succeed");

        let err = client
            .call_one(
                "urn:ietf:params:jmap:mail",
                "Email/get",
                json!({
                    "accountId": TEST_ACCOUNT_ID,
                    "ids": ["nonexistent"]
                }),
            )
            .expect_err("JMAP error response should produce an error");

        let msg = format!("{err}");
        assert!(
            msg.contains("notFound"),
            "error should contain JMAP error type: {msg}"
        );
    }

    #[test]
    fn download_blob_fetches_raw_bytes() {
        let mock = MockJmap::start();
        let raw = b"From: a@b.com\r\nSubject: Hi\r\n\r\nbody";
        mock.handle_download(raw);

        let (client, _) = JmapClient::connect_to(&mock.session_url(), "fake-token")
            .expect("session should succeed");

        let bytes = client
            .download_blob(
                TEST_ACCOUNT_ID,
                &BlobId("blob-1".to_string()),
                "message.eml",
                "message/rfc822",
            )
            .expect("download should succeed");

        assert_eq!(bytes, raw);
    }
}
