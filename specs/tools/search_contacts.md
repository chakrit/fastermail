# search_contacts

**Phase 2 — CardDAV** (FastMail does not yet expose contacts via JMAP)

Search contacts by name, email, phone, etc.

## Parameters

| Param    | Type   | Required | Description           |
|----------|--------|----------|-----------------------|
| `query`  | string | yes      | Search text           |
| `limit`  | integer| no       | Max results (default 20) |

## Protocol

**CardDAV:** REPORT with text-match filter.
**Future JMAP:** `ContactCard/query` (with text filter) → `ContactCard/get`

## Returns

Array of contacts matching the search query.

## Error Cases

- Missing `query` → `isError: true`, "query is required".
- CardDAV/JMAP error → `isError: true` with error message.
