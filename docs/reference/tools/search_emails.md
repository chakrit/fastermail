# search_emails

Search emails with filters.

## Parameters

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

## JMAP

**Capability:** `urn:ietf:params:jmap:mail`
**Methods:** `Email/query` → `Email/get`

Build filter object dynamically from whichever params are provided. JMAP filter fields:
- `keyword` → `text` filter
- `from` → `from` filter
- `to` → `to` filter
- `subject` → `subject` filter
- `mailboxId` → `inMailbox` filter
- `hasAttachment` → `hasAttachment` filter
- `after` → `after` filter (convert YYYY-MM-DD to ISO 8601)
- `before` → `before` filter (convert YYYY-MM-DD to ISO 8601)

## Returns

Same as `get_emails`.

## Error Cases

- No filter params provided → `isError: true`, "at least one search filter required".
- JMAP error → `isError: true` with JMAP error message.
