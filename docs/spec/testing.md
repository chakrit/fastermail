# Test Strategy

All tests run without a real FastMail account or API token. A built-in mock JMAP
server simulates FastMail's HTTP responses in-process. Tests follow TDD: write a
failing test first, run it to confirm failure, then implement.

### 1 Test Dependencies

| Crate           | Purpose                          | Why this one                              |
|-----------------|----------------------------------|-------------------------------------------|
| `httpmock`      | Mock HTTP server                 | In-process, no async runtime, fast compile. Standalone server per test with request matching and canned responses. |
| `assert_json_diff` | JSON comparison in assertions | Structural diffs instead of string comparison — clear failure messages. |

Neither reaches the production binary. `assert_json_diff` is a plain dev-dependency;
`httpmock` is an optional dependency behind the `testutil` feature (off in release), which
the package enables for its own tests via a self dev-dependency. That indirection is what
lets the `fm` binary's tests — a separate crate from the `fastermail` library after the
lib split — reach the library's `testutil` harness; a plain `#[cfg(test)]` module cannot
cross the lib/bin crate boundary.

```toml
[features]
testutil = ["dep:httpmock"]

[dependencies]
httpmock = { version = "0.8", optional = true }

[dev-dependencies]
fastermail = { path = ".", features = ["testutil"] }  # enables testutil for `cargo test`
assert-json-diff = "2"
```

### 2 Mock JMAP Server

Lives in `src/testutil/mod.rs`, compiled under the `testutil` feature (enabled for all
test builds via the self dev-dependency above). Provides a reusable builder that
configures an `httpmock::MockServer` with FastMail-shaped endpoints.

#### 2.1 Design

```rust
// src/lib.rs

#[cfg(feature = "testutil")]
pub mod testutil;

// src/testutil/mod.rs
pub mod mock_jmap;
```

```rust
// src/testutil/mock_jmap.rs

use httpmock::MockServer;
use httpmock::Method::*;
use serde_json::{json, Value};

/// A configured mock FastMail JMAP server.
///
/// Serves a session endpoint and an API endpoint that handles JMAP method calls
/// with configurable responses.
pub struct MockJmap {
    server: MockServer,
}

/// Default account ID used across all tests.
pub const TEST_ACCOUNT_ID: &str = "u1234567";

impl MockJmap {
    /// Start a mock server with a default session response.
    pub fn start() -> Self {
        let server = MockServer::start();

        // Session endpoint — always present
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
    /// in the `methodCalls` array, it responds with `response_body`.
    pub fn handle_method(&self, method_name: &str, response_body: Value) {
        let method = method_name.to_string();
        self.server.mock(|when, then| {
            when.method(POST)
                .path("/jmap/api/")
                .json_body_includes(json!(method));
            then.status(200)
                .header("content-type", "application/json")
                .json_body(response_body);
        });
    }

    /// Base URL for constructing a JmapClient that points at this mock.
    pub fn base_url(&self) -> String {
        self.server.base_url()
    }

    /// Session URL for this mock server.
    pub fn session_url(&self) -> String {
        format!("{}/jmap/session", self.server.base_url())
    }

    fn default_session(server: &MockServer) -> Value {
        json!({
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
```

#### 2.2 Usage Pattern

Every test that needs JMAP creates a `MockJmap`, configures method responses,
then builds a `JmapClient` pointing at the mock's URL:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    #[test]
    fn get_emails_returns_subjects() {
        let mock = MockJmap::start();
        mock.handle_method("Email/get", json!({
            "methodResponses": [
                ["Email/query", { "ids": ["e001"] }, "call-0"],
                ["Email/get", {
                    "list": [{
                        "id": "e001",
                        "subject": "Hello World",
                        "from": [{"name": "Alice", "email": "alice@example.com"}],
                        "receivedAt": "2026-01-15T10:00:00Z",
                        "preview": "This is the preview text."
                    }]
                }, "call-1"]
            ]
        }));

        let client = JmapClient::new(&mock.session_url(), "fake-token")
            .expect("session discovery should succeed against mock");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
        };

        let action = GetEmails {
            mailbox_id: Some("mbox-inbox".into()),
            limit: Some(10),
            ..Default::default()
        };

        let result = action.run(&ctx).expect("action should succeed");
        let emails = result.as_array().expect("result should be an array");
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0]["subject"], "Hello World");
    }
}
```

### 3 MCP Protocol Tests

Location: `src/mcp/types.rs` and `src/mcp/server.rs` (inline `#[cfg(test)]` modules).

These test the JSON-RPC parsing and MCP dispatch layer in isolation — no JMAP, no
network.

#### 3.1 What to Test

| Test                                  | Validates                                    |
|---------------------------------------|----------------------------------------------|
| Parse valid JSON-RPC request          | `id`, `method`, `params` extracted correctly |
| Reject JSON-RPC with missing `jsonrpc` field | Returns parse error (-32700)           |
| Reject JSON-RPC with non-2.0 version | Returns invalid request (-32600)             |
| Notification has no `id`              | Recognized as notification, not request      |
| Unknown method returns -32601         | Dispatch rejects unrecognized methods        |
| `initialize` returns server capabilities | Protocol version, `tools` capability      |
| `ping` returns empty result           | `{ "result": {} }`                           |
| `tools/list` returns all tool definitions | Each tool has `name`, `description`, `inputSchema` |
| `tools/call` with valid tool name dispatches | Correct action receives params         |
| `tools/call` with unknown tool name   | Returns `isError: true` with message         |
| `tools/call` with missing required params | Returns `isError: true` with message     |

#### 3.2 Test Pattern — Protocol Parsing

```rust
// In src/mcp/types.rs

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_valid_request() {
        let input = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let msg = JsonRpcMessage::parse(&input)
            .expect("valid JSON-RPC should parse");

        assert_eq!(msg.method, "tools/list");
        assert_eq!(msg.id, Some(json!(1)));
    }

    #[test]
    fn reject_missing_jsonrpc_field() {
        let input = json!({
            "id": 1,
            "method": "tools/list"
        });

        let err = JsonRpcMessage::parse(&input)
            .expect_err("missing jsonrpc field should fail");

        assert_eq!(err.code, -32700);
    }

    #[test]
    fn notification_has_no_id() {
        let input = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let msg = JsonRpcMessage::parse(&input)
            .expect("notification should parse");

        assert!(msg.id.is_none());
        assert!(msg.is_notification());
    }
}
```

#### 3.3 Test Pattern — Initialization Handshake

```rust
// In src/mcp/server.rs

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn initialize_returns_capabilities() {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "test-client", "version": "1.0" }
            }
        });

        let response = handle_message(&request)
            .expect("initialize should produce a response");

        assert_eq!(response["id"], 1);
        let result = &response["result"];
        assert_eq!(result["protocolVersion"], "2025-11-25");
        assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
        assert_eq!(result["serverInfo"]["name"], "fastermail");
    }

    #[test]
    fn unknown_method_returns_error() {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "resources/list",
            "params": {}
        });

        let response = handle_message(&request)
            .expect("unknown method should produce error response");

        assert_eq!(response["error"]["code"], -32601);
    }
}
```

### 4 JMAP Client Tests

Location: `src/jmap/client.rs` (inline `#[cfg(test)]` module).

Test the `JmapClient` against the mock server — session discovery, request
building, error handling.

#### 4.1 What to Test

| Test                                    | Validates                                |
|-----------------------------------------|------------------------------------------|
| Session discovery extracts account ID   | Parses `primaryAccounts` correctly       |
| Session discovery extracts API URL      | `apiUrl` stored for subsequent calls     |
| Session fetch with bad token returns error | 401 mapped to `Error::Http`            |
| Session fetch with malformed JSON       | Handled gracefully, not a panic          |
| JMAP request includes correct `using`   | Capability URIs match the methods used   |
| JMAP request includes `Authorization`   | Bearer token present in header           |
| Back-reference building                 | `#ids` / `resultOf` / `path` correct     |
| JMAP error response parsed             | `methodResponses` with error type detected |
| Network timeout handled                 | Returns `Error::Http`, not a panic       |

#### 4.2 Test Pattern

```rust
// In src/jmap/client.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};

    #[test]
    fn session_discovery_extracts_account_id() {
        let mock = MockJmap::start();

        let client = JmapClient::new(&mock.session_url(), "fake-token")
            .expect("session discovery should succeed");

        assert_eq!(client.account_id(), TEST_ACCOUNT_ID);
    }

    #[test]
    fn session_discovery_extracts_api_url() {
        let mock = MockJmap::start();

        let client = JmapClient::new(&mock.session_url(), "fake-token")
            .expect("session discovery should succeed");

        assert!(
            client.api_url().starts_with(&mock.base_url()),
            "API URL should point to the mock server"
        );
    }

    #[test]
    fn session_fetch_with_401_returns_error() {
        let server = httpmock::MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/jmap/session");
            then.status(401);
        });

        let err = JmapClient::new(
            &format!("{}/jmap/session", server.base_url()),
            "bad-token",
        )
        .expect_err("401 should produce an error");

        assert!(
            format!("{err:?}").contains("401").or(format!("{err:?}").contains("auth")),
            "error should indicate authentication failure"
        );
    }
}
```

### 5 Action/Tool Tests

Location: each file in `src/actions/` (inline `#[cfg(test)]` modules).

Each action struct gets its own tests verifying: correct JMAP method calls
generated, response parsing, parameter validation, and error cases.

#### 5.1 What to Test Per Action

| Category                 | Example                                              |
|--------------------------|------------------------------------------------------|
| Correct JMAP calls       | `GetEmails` sends `Email/query` + `Email/get`        |
| Response parsing         | Extracts `subject`, `from`, `date` from JMAP response |
| Parameter validation     | `get_emails` with neither `mailboxId` nor `mailboxName` returns error |
| Missing required params  | `send_email` without `to` returns `InvalidParams`    |
| JMAP error propagation   | JMAP `notFound` error mapped to tool error with `isError: true` |
| Default values           | `limit` defaults to 20 when omitted                  |
| Conditional logic        | `delete_email` with `permanent: true` destroys vs moves to trash |

#### 5.2 Test Pattern — Action With Mock

```rust
// In src/actions/mailbox.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::mock_jmap::{MockJmap, TEST_ACCOUNT_ID};
    use serde_json::json;

    #[test]
    fn list_mailboxes_returns_all_mailboxes() {
        let mock = MockJmap::start();
        mock.handle_method("Mailbox/get", json!({
            "methodResponses": [
                ["Mailbox/get", {
                    "accountId": TEST_ACCOUNT_ID,
                    "list": [
                        { "id": "mbox-1", "name": "Inbox", "role": "inbox",
                          "totalEmails": 42, "unreadEmails": 3 },
                        { "id": "mbox-2", "name": "Sent", "role": "sent",
                          "totalEmails": 100, "unreadEmails": 0 }
                    ]
                }, "call-0"]
            ]
        }));

        let client = JmapClient::new(&mock.session_url(), "fake-token")
            .expect("session should succeed");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
        };

        let action = ListMailboxes { role: None };
        let result = action.run(&ctx).expect("list_mailboxes should succeed");

        let mailboxes = result.as_array().expect("result should be array");
        assert_eq!(mailboxes.len(), 2);
        assert_eq!(mailboxes[0]["name"], "Inbox");
        assert_eq!(mailboxes[0]["role"], "inbox");
    }

    #[test]
    fn list_mailboxes_filters_by_role() {
        let mock = MockJmap::start();
        mock.handle_method("Mailbox/get", json!({
            "methodResponses": [
                ["Mailbox/get", {
                    "accountId": TEST_ACCOUNT_ID,
                    "list": [
                        { "id": "mbox-1", "name": "Inbox", "role": "inbox",
                          "totalEmails": 42, "unreadEmails": 3 },
                        { "id": "mbox-2", "name": "Sent", "role": "sent",
                          "totalEmails": 100, "unreadEmails": 0 }
                    ]
                }, "call-0"]
            ]
        }));

        let client = JmapClient::new(&mock.session_url(), "fake-token")
            .expect("session should succeed");
        let ctx = Context {
            jmap: client,
            account_id: TEST_ACCOUNT_ID.to_string(),
        };

        let action = ListMailboxes { role: Some("inbox".into()) };
        let result = action.run(&ctx).expect("list_mailboxes should succeed");

        let mailboxes = result.as_array().expect("result should be array");
        assert_eq!(mailboxes.len(), 1);
        assert_eq!(mailboxes[0]["role"], "inbox");
    }
}
```

#### 5.3 Test Pattern — Parameter Validation

```rust
// In src/actions/email.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_emails_requires_mailbox_id_or_name() {
        // No mock needed — validation happens before any JMAP call
        let action = GetEmails {
            mailbox_id: None,
            mailbox_name: None,
            limit: None,
            include_body: None,
        };

        let err = action.validate()
            .expect_err("should reject when neither mailboxId nor mailboxName given");

        assert!(
            format!("{err}").contains("mailboxId"),
            "error should mention the missing parameter"
        );
    }

    #[test]
    fn send_email_requires_to_field() {
        let action = SendEmail {
            to: vec![],
            subject: "Test".into(),
            body: "Hello".into(),
            ..Default::default()
        };

        let err = action.validate()
            .expect_err("should reject empty 'to' list");

        assert!(
            format!("{err}").contains("to"),
            "error should mention the missing 'to' field"
        );
    }
}
```

### 6 What NOT to Test

Per the coding conventions, these are explicitly out of scope:

| Skip                                    | Reason                                       |
|-----------------------------------------|----------------------------------------------|
| Serde `Serialize`/`Deserialize` derives | Tests the serde crate, not our code          |
| Trivial getters (`fn id(&self) -> &str`) | Restate the implementation, catch nothing    |
| That `ureq` sends HTTP requests         | Tests the HTTP library, not our code         |
| JSON-RPC wire format round-trips        | Covered by serde — no custom serialization   |
| `Default` derive correctness            | Tests the compiler                           |

Focus test effort on: branching logic, parameter validation, response
transformation, error mapping, and dispatch routing.

### 7 Test Organization Summary

```
src/
├── config.rs              # #[cfg(test)] mod tests — token resolution
├── testutil/
│   ├── mod.rs              # #[cfg(test)] pub mod mock_jmap;
│   └── mock_jmap.rs        # MockJmap builder, canned session, helper methods
├── mcp/
│   ├── types.rs            # #[cfg(test)] mod tests — JSON-RPC parsing
│   └── handler.rs          # #[cfg(test)] mod tests — tool routing + dispatch
├── jmap/
│   ├── client.rs           # #[cfg(test)] mod tests — session, requests, errors
│   └── types.rs            # #[cfg(test)] mod tests — filter/request building logic
├── cli/
│   ├── resolve.rs          # #[cfg(test)] mod tests — mailbox resolution
│   └── contacts.rs         # #[cfg(test)] mod tests — typed-value parsing
└── actions/
    ├── mod.rs              # #[cfg(test)] mod tests — shared action helpers
    ├── email.rs            # #[cfg(test)] mod tests — email actions
    ├── mailbox.rs          # #[cfg(test)] mod tests — mailbox actions
    ├── vacation.rs         # #[cfg(test)] mod tests — vacation response actions
    ├── masked_email.rs     # #[cfg(test)] mod tests — masked email actions
    ├── identity.rs         # #[cfg(test)] mod tests — identity actions
    └── contact.rs          # #[cfg(test)] mod tests — contact actions
```

### 8 Conventions

- **`.expect("reason")` everywhere** — never `.unwrap()` in tests. The reason
  string is the first thing you see when a test fails.
- **One mock per test** — each `#[test]` starts its own `MockJmap`. No shared
  mutable state between tests. Tests run in parallel safely.
- **Assert behavior, not structure** — check that the right emails come back,
  not that the JSON has exactly N keys. Use `assert_json_diff` for structural
  comparison when matching large response shapes.
- **Name tests as sentences** — `fn list_mailboxes_filters_by_role()` reads as
  a spec. Avoid prefixes like `test_`.
- **Keep tests fast** — the mock server is in-process with no artificial delays.
  The full suite should run in under 5 seconds.