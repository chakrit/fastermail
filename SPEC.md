# FasterMail — Full MCP Server Specification

## Overview

FasterMail is an MCP (Model Context Protocol) server written in Rust that exposes FastMail's
APIs to AI assistants. It communicates over stdio using JSON-RPC 2.0.

**Phase 1 (JMAP):** Email, sending, vacation response, masked email — all available via JMAP today.
**Phase 2 (CardDAV/CalDAV):** Contacts and calendars — FastMail does not yet expose these via
JMAP (only CardDAV/CalDAV). When FastMail enables JMAP for contacts/calendars, Phase 2 tools
can migrate to JMAP.

## Design Decisions

- **Auth**: `FASTMAIL_API_TOKEN` environment variable, read at startup. No configure tool,
  no config file. Fail fast with a clear error if unset.
- **Transport**: stdio only (newline-delimited JSON-RPC 2.0).
- **Dependencies**: Minimize for fast compile times. No MCP SDK crate — implement the
  thin JSON-RPC + MCP layer from scratch.
- **Architecture**: Unit-of-work pattern — each MCP tool maps to an action struct with a
  `run(&self, ctx: &Context) -> Result<T>` method.
- **Distribution**: Linux + macOS binaries (x86_64 + aarch64) and Docker images.

---

## 1. Protocol Layer

### 1.1 Transport — stdio

- Read newline-delimited JSON-RPC 2.0 messages from stdin.
- Write newline-delimited JSON-RPC 2.0 messages to stdout.
- Never write non-JSON-RPC content to stdout. Logs go to stderr.

### 1.2 JSON-RPC 2.0 Message Types

**Request** (client → server or server → client):
```json
{ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { ... } }
```

**Response**:
```json
{ "jsonrpc": "2.0", "id": 1, "result": { ... } }
```

**Error response**:
```json
{ "jsonrpc": "2.0", "id": 1, "error": { "code": -32601, "message": "Method not found" } }
```

**Notification** (no `id`, no response expected):
```json
{ "jsonrpc": "2.0", "method": "notifications/initialized" }
```

### 1.3 Initialization Handshake

Three-step sequence before any other messages:

1. Client sends `initialize` request with `protocolVersion`, `capabilities`, `clientInfo`.
2. Server responds with its `protocolVersion` (`2025-11-25`), `capabilities`, `serverInfo`.
3. Client sends `notifications/initialized` notification.

Server capabilities declared:
```json
{
  "tools": { "listChanged": false }
}
```

No resources, no prompts, no sampling — tools only.

### 1.4 Methods the Server Must Handle

| Method                  | Type         | Description                        |
|-------------------------|--------------|------------------------------------|
| `initialize`            | Request      | Handshake, return capabilities     |
| `notifications/initialized` | Notification | Client confirms init complete |
| `ping`                  | Request      | Respond with `{ "result": {} }`    |
| `tools/list`            | Request      | Return all tool definitions        |
| `tools/call`            | Request      | Execute a tool, return result      |

### 1.5 Error Codes

| Code     | Meaning              |
|----------|----------------------|
| `-32700` | Parse error          |
| `-32600` | Invalid request      |
| `-32601` | Method not found     |
| `-32602` | Invalid params       |
| `-32603` | Internal error       |

Tool execution errors return a successful response with `isError: true` in the result content.

---

## 2. JMAP Client Layer

### 2.1 Session Discovery

On startup (after reading `FASTMAIL_API_TOKEN`), fetch the JMAP session:

```
GET https://api.fastmail.com/jmap/session
Authorization: Bearer <token>
```

The session response contains:
- `primaryAccounts` — map of capability URI → account ID.
- `accounts` — account metadata.
- `apiUrl` — the endpoint for JMAP method calls (typically `https://api.fastmail.com/jmap/api/`).
- `capabilities` — server-level capability declarations.

Extract the primary account ID from `primaryAccounts["urn:ietf:params:jmap:core"]`.

### 2.2 JMAP Request Format

All JMAP calls are POST to `apiUrl`:

```json
{
  "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/query", { "accountId": "...", "filter": { ... } }, "call-0"],
    ["Email/get", { "accountId": "...", "#ids": { "resultOf": "call-0", "name": "Email/query", "path": "/ids" } }, "call-1"]
  ]
}
```

The `using` array declares which capabilities are needed. The `methodCalls` array supports
back-references (`#ids` with `resultOf`) for chaining queries.

### 2.3 Authentication

Bearer token in the `Authorization` header for all requests. The token format is `fmu1-*`.

### 2.4 Required JMAP Capabilities

| Capability URI                              | Domain     |
|---------------------------------------------|------------|
| `urn:ietf:params:jmap:core`                 | Core       |
| `urn:ietf:params:jmap:mail`                 | Email      |
| `urn:ietf:params:jmap:submission`           | Sending    |
| `urn:ietf:params:jmap:vacationresponse`     | Vacation Response |
| `https://www.fastmail.com/dev/maskedemail`  | Masked Email (FastMail-specific) |

**Not yet available via JMAP** (use CardDAV/CalDAV):
- `urn:ietf:params:jmap:contacts` — Contacts
- `urn:ietf:params:jmap:calendars` — Calendars

---

## 3. MCP Tools

Each tool is an action struct implementing the unit-of-work pattern. The tool names use
snake_case. Parameters use camelCase (matching JMAP conventions in the MCP schema).

### 3.1 Email Tools

#### `list_mailboxes`

List all mailboxes with metadata.

| Param  | Type   | Required | Description                          |
|--------|--------|----------|--------------------------------------|
| `role` | string | no       | Filter by role (inbox, sent, drafts, trash, junk, archive) |

JMAP method: `Mailbox/get`

Returns: Array of mailboxes with `id`, `name`, `role`, `totalEmails`, `unreadEmails`,
`parentId`.

#### `get_emails`

Retrieve emails from a mailbox.

| Param          | Type    | Required | Description                     |
|----------------|---------|----------|---------------------------------|
| `mailboxId`    | string  | no       | Mailbox ID to fetch from        |
| `mailboxName`  | string  | no       | Mailbox name (resolved to ID)   |
| `limit`        | integer | no       | Max results (default 20)        |
| `includeBody`  | boolean | no       | Include body content (default false) |

At least one of `mailboxId` or `mailboxName` required.

JMAP methods: `Mailbox/get` (if resolving name) → `Email/query` → `Email/get`

Returns: Array of emails with `id`, `subject`, `from`, `to`, `date`, `preview`,
and optionally `textBody`/`htmlBody`.

#### `search_emails`

Search emails with filters.

| Param           | Type    | Required | Description                    |
|-----------------|---------|----------|--------------------------------|
| `keyword`       | string  | no       | Full-text search               |
| `from`          | string  | no       | Sender address filter          |
| `to`            | string  | no       | Recipient address filter       |
| `subject`       | string  | no       | Subject filter                 |
| `mailboxId`     | string  | no       | Restrict to mailbox            |
| `hasAttachment` | boolean | no       | Filter by attachment presence  |
| `after`         | string  | no       | Date lower bound (YYYY-MM-DD)  |
| `before`        | string  | no       | Date upper bound (YYYY-MM-DD)  |
| `limit`         | integer | no       | Max results (default 20)       |
| `includeBody`   | boolean | no       | Include body content           |

At least one filter param required.

JMAP methods: `Email/query` → `Email/get`

#### `get_email_body`

Get full body of a single email.

| Param    | Type   | Required | Description                           |
|----------|--------|----------|---------------------------------------|
| `emailId`| string | yes      | Email ID                              |
| `format` | string | no       | `text`, `html`, or `both` (default `text`) |

JMAP method: `Email/get` with `fetchTextBodyValues`, `fetchHTMLBodyValues`, and/or `bodyProperties`

#### `send_email`

Compose and send an email.

| Param    | Type     | Required | Description                |
|----------|----------|----------|----------------------------|
| `to`     | string[] | yes      | Recipient addresses        |
| `subject`| string   | yes      | Email subject              |
| `body`   | string   | yes      | Email body                 |
| `cc`     | string[] | no       | CC recipients              |
| `bcc`    | string[] | no       | BCC recipients             |
| `isHtml` | boolean  | no       | Body is HTML (default false) |
| `inReplyTo` | string | no     | Email ID being replied to  |

JMAP methods: `Email/set` (create draft) → `EmailSubmission/set` (submit)

#### `move_email`

Move emails between mailboxes.

| Param         | Type     | Required | Description            |
|---------------|----------|----------|------------------------|
| `emailIds`    | string[] | yes      | Email IDs to move      |
| `mailboxId`   | string   | yes      | Destination mailbox ID |

JMAP method: `Email/set` (update mailboxIds property)

#### `delete_email`

Delete emails (move to Trash, or permanently delete if already in Trash).

| Param      | Type     | Required | Description       |
|------------|----------|----------|-------------------|
| `emailIds` | string[] | yes      | Email IDs to delete |
| `permanent`| boolean  | no       | Skip trash (default false) |

JMAP method: `Email/set` (update or destroy)

#### `flag_email`

Set/unset flags (keywords) on emails.

| Param      | Type     | Required | Description                    |
|------------|----------|----------|--------------------------------|
| `emailIds` | string[] | yes      | Email IDs                      |
| `flag`     | string   | yes      | Flag: `seen`, `flagged`, `answered`, `draft` |
| `value`    | boolean  | yes      | Set (true) or unset (false)    |

JMAP method: `Email/set` (update keywords)

#### `manage_mailbox`

Create, rename, or delete mailboxes.

| Param    | Type   | Required | Description                              |
|----------|--------|----------|------------------------------------------|
| `action` | string | yes      | `create`, `rename`, or `delete`          |
| `name`   | string | yes*     | Name for create/rename (* required for create/rename) |
| `mailboxId` | string | yes*  | ID of mailbox (* required for rename/delete) |
| `parentId`  | string | no    | Parent mailbox ID for create             |

JMAP method: `Mailbox/set`

### 3.2 Vacation Response Tools

#### `get_vacation_response`

Get the current vacation/auto-reply settings.

JMAP method: `VacationResponse/get`

Returns: `isEnabled`, `fromDate`, `toDate`, `subject`, `textBody`, `htmlBody`.

#### `set_vacation_response`

Enable, disable, or update the vacation auto-reply.

| Param       | Type    | Required | Description                        |
|-------------|---------|----------|------------------------------------|
| `isEnabled` | boolean | yes      | Enable or disable auto-reply       |
| `fromDate`  | string  | no       | Start date (ISO 8601, UTC)         |
| `toDate`    | string  | no       | End date (ISO 8601, UTC)           |
| `subject`   | string  | no       | Auto-reply subject                 |
| `textBody`  | string  | no       | Plain text auto-reply body         |
| `htmlBody`  | string  | no       | HTML auto-reply body               |

JMAP method: `VacationResponse/set`

### 3.3 Contact Tools (Phase 2 — CardDAV)

#### `list_address_books`

List all address books.

JMAP method: `AddressBook/get`

Returns: Array with `id`, `name`, `isDefault`.

#### `get_contacts`

Get contacts from an address book.

| Param          | Type    | Required | Description                  |
|----------------|---------|----------|------------------------------|
| `addressBookId`| string  | no       | Filter by address book       |
| `limit`        | integer | no       | Max results (default 50)     |

JMAP methods: `ContactCard/query` → `ContactCard/get`

#### `search_contacts`

Search contacts by name, email, phone, etc.

| Param    | Type   | Required | Description           |
|----------|--------|----------|-----------------------|
| `query`  | string | yes      | Search text           |
| `limit`  | integer| no       | Max results (default 20) |

JMAP methods: `ContactCard/query` (with text filter) → `ContactCard/get`

#### `create_contact`

Create a new contact.

| Param          | Type     | Required | Description              |
|----------------|----------|----------|--------------------------|
| `name`         | string   | yes      | Full name                |
| `emails`       | object[] | no       | Array of `{type, value}` |
| `phones`       | object[] | no       | Array of `{type, value}` |
| `company`      | string   | no       | Organization name        |
| `notes`        | string   | no       | Free-text notes          |
| `addressBookId`| string   | no       | Target address book      |

JMAP method: `ContactCard/set`

#### `update_contact`

Update an existing contact.

| Param      | Type   | Required | Description              |
|------------|--------|----------|--------------------------|
| `contactId`| string | yes      | Contact ID               |
| `name`     | string | no       | Updated name             |
| `emails`   | object[]| no      | Updated emails           |
| `phones`   | object[]| no      | Updated phones           |
| `company`  | string | no       | Updated organization     |
| `notes`    | string | no       | Updated notes            |

JMAP method: `ContactCard/set`

#### `delete_contact`

Delete a contact.

| Param      | Type   | Required | Description |
|------------|--------|----------|-------------|
| `contactId`| string | yes      | Contact ID  |

JMAP method: `ContactCard/set` (destroy)

### 3.4 Calendar Tools (Phase 2 — CalDAV)

#### `list_calendars`

List all calendars.

JMAP method: `Calendar/get`

Returns: Array with `id`, `name`, `color`, `isDefault`.

#### `get_events`

Get calendar events.

| Param        | Type    | Required | Description                    |
|--------------|---------|----------|--------------------------------|
| `calendarId` | string  | no       | Filter by calendar             |
| `after`      | string  | no       | Start date (YYYY-MM-DD)        |
| `before`     | string  | no       | End date (YYYY-MM-DD)          |
| `limit`      | integer | no       | Max results (default 50)       |

JMAP methods: `CalendarEvent/query` (with `expandRecurrences`) → `CalendarEvent/get`

#### `search_events`

Search calendar events.

| Param   | Type    | Required | Description           |
|---------|---------|----------|-----------------------|
| `query` | string  | yes      | Search text           |
| `after` | string  | no       | Start date bound      |
| `before`| string  | no       | End date bound        |
| `limit` | integer | no       | Max results (default 20) |

JMAP methods: `CalendarEvent/query` → `CalendarEvent/get`

#### `create_event`

Create a calendar event.

| Param        | Type     | Required | Description                |
|--------------|----------|----------|----------------------------|
| `title`      | string   | yes      | Event title                |
| `start`      | string   | yes      | ISO 8601 datetime          |
| `duration`   | string   | yes      | ISO 8601 duration (e.g. `PT1H`) |
| `calendarId` | string   | no       | Target calendar            |
| `description`| string   | no       | Event description          |
| `location`   | string   | no       | Event location             |
| `participants`| object[]| no       | Array of `{name, email}`   |
| `timeZone`   | string   | no       | IANA timezone              |

JMAP method: `CalendarEvent/set`

#### `update_event`

Update a calendar event.

| Param        | Type   | Required | Description                |
|--------------|--------|----------|----------------------------|
| `eventId`    | string | yes      | Event ID                   |
| `title`      | string | no       | Updated title              |
| `start`      | string | no       | Updated start time         |
| `duration`   | string | no       | Updated duration           |
| `description`| string | no       | Updated description        |
| `location`   | string | no       | Updated location           |

JMAP method: `CalendarEvent/set`

#### `delete_event`

Delete a calendar event.

| Param    | Type    | Required | Description                    |
|----------|---------|----------|--------------------------------|
| `eventId`| string  | yes      | Event ID                       |
| `notify` | boolean | no       | Send cancellation to participants (default true) |

JMAP method: `CalendarEvent/set` (destroy) with `sendSchedulingMessages`

### 3.5 Identity Tools

#### `list_identities`

List sending identities (From addresses).

JMAP method: `Identity/get`

Returns: Array with `id`, `name`, `email`, `replyTo`.

### 3.6 Masked Email Tools

FastMail-specific extension (`https://www.fastmail.com/dev/maskedemail`).

#### `list_masked_emails`

List all masked (disposable) email addresses.

| Param   | Type   | Required | Description                  |
|---------|--------|----------|------------------------------|
| `state` | string | no       | Filter: `pending`, `enabled`, `disabled`, `deleted` |

JMAP method: `MaskedEmail/get`

Returns: Array with `id`, `email`, `forDomain`, `description`, `state`, `createdAt`.

#### `create_masked_email`

Create a new masked email address.

| Param         | Type   | Required | Description                         |
|---------------|--------|----------|-------------------------------------|
| `forDomain`   | string | no       | Domain this address is for          |
| `description` | string | no       | Human-readable label                |
| `emailPrefix` | string | no       | Preferred prefix for the address    |

JMAP method: `MaskedEmail/set`

#### `update_masked_email`

Enable, disable, or delete a masked email address.

| Param   | Type   | Required | Description                              |
|---------|--------|----------|------------------------------------------|
| `id`    | string | yes      | Masked email ID                          |
| `state` | string | yes      | New state: `enabled`, `disabled`, `deleted` |

JMAP method: `MaskedEmail/set`

---

## 4. Project Structure

```
fastermail/
├── Cargo.toml
├── Dockerfile
├── SPEC.md
├── .skills/
│   ├── general-coding.md
│   └── rust-coding.md
├── src/
│   ├── main.rs              # Entry point: read env, init session, run server loop
│   ├── error.rs             # Single error enum for the crate
│   ├── mcp/
│   │   ├── mod.rs           # MCP module root
│   │   ├── types.rs         # JSON-RPC & MCP types (Request, Response, Tool, etc.)
│   │   ├── server.rs        # stdio read/write loop, dispatch to handlers
│   │   └── handler.rs       # Route tools/list and tools/call to actions
│   ├── jmap/
│   │   ├── mod.rs           # JMAP module root
│   │   ├── client.rs        # HTTP client, session management, JMAP request builder
│   │   └── types.rs         # JMAP request/response types, filter builders
│   └── actions/
│       ├── mod.rs           # Action trait + registry
│       ├── email.rs         # Email action structs (GetEmails, SearchEmails, SendEmail, etc.)
│       ├── mailbox.rs       # Mailbox action structs
│       ├── vacation.rs      # Vacation response action structs
│       ├── masked_email.rs  # Masked email action structs
│       ├── identity.rs      # Identity action structs
│       ├── contact.rs       # Contact action structs (Phase 2 — CardDAV)
│       └── calendar.rs      # Calendar action structs (Phase 2 — CalDAV)
```

### 4.1 Key Types

```rust
// Context passed to all actions
struct Context {
    jmap: JmapClient,
    account_id: String,
}

// Action trait — unit-of-work pattern
trait Action {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value>;
}

// Single crate-level error enum
enum Error {
    Io(std::io::Error),
    Http(/* http client error */),
    Jmap { method: String, message: String },
    InvalidParams(String),
    MissingToken,
}
```

---

## 5. Dependencies

Guiding principle: minimize compile time.

| Crate           | Purpose                        | Why this one                    |
|-----------------|--------------------------------|---------------------------------|
| `serde`         | Serialization                  | Required, no alternative        |
| `serde_json`    | JSON parsing                   | Required, no alternative        |
| `ureq`          | HTTP client                    | Blocking, minimal, fast compile. No async runtime needed — stdio is inherently sequential |
| `thiserror`     | Error derive macros            | Tiny, zero runtime cost         |

**No async runtime.** The MCP stdio server reads one message, processes it, writes a response.
There is no concurrency — `ureq` (blocking HTTP) is sufficient and avoids pulling in `tokio`
(~30s compile time penalty).

---

## 6. Distribution

### 6.1 Binary Targets

| Target                        | OS    | Arch    |
|-------------------------------|-------|---------|
| `x86_64-unknown-linux-gnu`    | Linux | x86_64  |
| `aarch64-unknown-linux-gnu`   | Linux | aarch64 |
| `x86_64-apple-darwin`         | macOS | x86_64  |
| `aarch64-apple-darwin`        | macOS | aarch64 |

Cross-compilation via `cross` or CI matrix.

### 6.2 Docker

```dockerfile
FROM rust:1-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/fastermail /usr/local/bin/
ENTRYPOINT ["fastermail"]
```

Multi-arch image (`linux/amd64` + `linux/arm64`).

### 6.3 Versioning

Version lives in `Cargo.toml`. Binary reads it via `env!("CARGO_PKG_VERSION")`.
Bump with `cargo set-version`. Tag releases as `v{version}`.

---

## 7. Startup Flow

1. Read `FASTMAIL_API_TOKEN` from env. If unset, print error to stderr and exit 1.
2. Fetch JMAP session from `https://api.fastmail.com/jmap/session`.
3. Extract `apiUrl` and primary `accountId`.
4. Enter stdio read loop — wait for `initialize` request.
5. Respond with capabilities, wait for `initialized` notification.
6. Enter main loop: read request → dispatch → write response.
7. On stdin EOF, clean up and exit 0.

---

## 8. Error Strategy

- **Startup errors** (missing token, session fetch failure): print to stderr, exit 1.
- **Protocol errors** (malformed JSON, unknown method): JSON-RPC error response.
- **Tool errors** (JMAP call failed, invalid params): successful JSON-RPC response with
  `isError: true` and descriptive text content — lets the LLM retry with adjusted params.
- **One error enum** for the entire crate. No nested wrapper enums.
