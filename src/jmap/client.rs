use crate::error::{Error, Result};
use crate::jmap::types::{JmapRequest, JmapResponse, MethodCall, Session};

pub struct JmapClient {
    api_url: String,
    token: String,
}

impl JmapClient {
    /// Fetch the JMAP session and create a client.
    /// Uses `JMAP_SESSION_URL` env var if set, otherwise defaults to FastMail.
    pub fn connect(token: &str) -> Result<(Self, Session)> {
        let session_url = std::env::var("JMAP_SESSION_URL")
            .unwrap_or_else(|_| "https://api.fastmail.com/jmap/session".to_string());

        log_debug!("jmap", "fetching session from {session_url}");

        let mut resp = ureq::get(&session_url)
            .header("Authorization", &format!("Bearer {token}"))
            .call()?;

        let session: Session = resp.body_mut().read_json()?;

        log_debug!("jmap", "session ok, apiUrl={}", session.api_url);

        let api_url = session.api_url.clone();
        let client = Self {
            api_url,
            token: token.to_string(),
        };

        Ok((client, session))
    }

    /// Create a client with a known API URL (for testing).
    #[cfg(test)]
    pub fn new(api_url: String, token: String) -> Self {
        Self { api_url, token }
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

        log_trace!("jmap", "response: {} method(s)", jmap_resp.method_responses.len());

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

        let (resp_method, resp_data, _) = resp
            .method_responses
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
}
