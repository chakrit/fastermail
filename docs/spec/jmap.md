# JMAP Client Layer

## Session Discovery

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

## JMAP Request Format

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

## Authentication

Bearer token in the `Authorization` header for all requests. The token format is `fmu1-*`.

## Required JMAP Capabilities

| Capability URI                              | Domain     |
|---------------------------------------------|------------|
| `urn:ietf:params:jmap:core`                 | Core       |
| `urn:ietf:params:jmap:mail`                 | Email      |
| `urn:ietf:params:jmap:submission`           | Sending    |
| `urn:ietf:params:jmap:vacationresponse`     | Vacation Response |
| `https://www.fastmail.com/dev/maskedemail`  | Masked Email (FastMail-specific) |

### Phase 2

| Capability URI                              | Domain     |
|---------------------------------------------|------------|
| `urn:ietf:params:jmap:contacts`             | Contacts (RFC 9610, JSContact RFC 9553) |

**JMAP Contacts methods:** `AddressBook/get`, `ContactCard/get`, `ContactCard/query`,
`ContactCard/set`.

**JSContact flattening:** ContactCard uses JSContact structures (RFC 9553) — names are
objects with components, emails/phones are `Id[T]` maps. The action layer translates
between flat MCP tool params and JSContact objects. See individual tool specs for mapping.
