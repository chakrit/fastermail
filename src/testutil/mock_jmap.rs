use httpmock::prelude::*;
use serde_json::{Value, json};

pub const TEST_ACCOUNT_ID: &str = "u1234567";

/// A configured mock FastMail JMAP server.
///
/// Serves a session endpoint and an API endpoint that handles JMAP method calls
/// with configurable responses.
pub struct MockJmap {
    server: MockServer,
}

impl MockJmap {
    /// Start a mock server with a default session response.
    pub fn start() -> Self {
        let server = MockServer::start();

        server.mock(|when, then| {
            when.method(GET).path("/jmap/session");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(Self::default_session(&server));
        });

        Self { server }
    }

    /// Register a JMAP method call handler.
    ///
    /// When the mock receives a POST to `/jmap/api/` containing `method_name`
    /// in the body, it responds with `response_body`.
    pub fn handle_method(&self, method_name: &str, response_body: Value) {
        let method = method_name.to_string();
        self.server.mock(|when, then| {
            when.method(POST).path("/jmap/api/").body_includes(&method);
            then.status(200)
                .header("content-type", "application/json")
                .json_body(response_body);
        });
    }

    /// Register a handler that matches only requests whose body contains both
    /// `method_name` and `body_substr`. Lets successive paginated calls (which
    /// differ by `position`/`anchor`) return distinct windows.
    ///
    /// `body_substr` must not contain a colon: httpmock 0.8 `body_includes`
    /// silently fails to match substrings with `:`. Match on a colon-free token
    /// (e.g. the quoted anchor value `"e002"`, not `"anchor":"e002"`).
    pub fn handle_method_matching(
        &self,
        method_name: &str,
        body_substr: &str,
        response_body: Value,
    ) {
        let method = method_name.to_string();
        let substr = body_substr.to_string();
        self.server.mock(|when, then| {
            when.method(POST)
                .path("/jmap/api/")
                .body_includes(&method)
                .body_includes(&substr);
            then.status(200)
                .header("content-type", "application/json")
                .json_body(response_body);
        });
    }

    pub fn base_url(&self) -> String {
        self.server.base_url()
    }

    pub fn session_url(&self) -> String {
        format!("{}/jmap/session", self.server.base_url())
    }

    fn default_session(server: &MockServer) -> Value {
        json!({
            "username": "test@fastmail.com",
            "primaryAccounts": {
                "urn:ietf:params:jmap:core": TEST_ACCOUNT_ID,
                "urn:ietf:params:jmap:mail": TEST_ACCOUNT_ID,
                "urn:ietf:params:jmap:submission": TEST_ACCOUNT_ID,
                "urn:ietf:params:jmap:vacationresponse": TEST_ACCOUNT_ID,
                "https://www.fastmail.com/dev/maskedemail": TEST_ACCOUNT_ID
            },
            "accounts": {
                TEST_ACCOUNT_ID: {
                    "name": "test@fastmail.com",
                    "isPersonal": true,
                    "isReadOnly": false
                }
            },
            "apiUrl": format!("{}/jmap/api/", server.base_url()),
            "downloadUrl": format!("{}/jmap/download/", server.base_url()),
            "uploadUrl": format!("{}/jmap/upload/", server.base_url()),
            "capabilities": {
                "urn:ietf:params:jmap:core": {
                    "maxSizeUpload": 50000000,
                    "maxCallsInRequest": 64,
                    "maxObjectsInGet": 1000,
                    "maxObjectsInSet": 1000
                }
            }
        })
    }
}
