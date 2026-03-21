# get_emails

Retrieve emails from a mailbox.

## Parameters

| Param          | Type    | Required | Description                     |
|----------------|---------|----------|---------------------------------|
| `mailboxId`    | string  | no       | Mailbox ID to fetch from        |
| `mailboxName`  | string  | no       | Mailbox name (resolved to ID)   |
| `limit`        | integer | no       | Max results (default 20)        |
| `includeBody`  | boolean | no       | Include body content (default false) |

At least one of `mailboxId` or `mailboxName` required.

## JMAP

**Capability:** `urn:ietf:params:jmap:mail`
**Methods:** `Mailbox/get` (if resolving name) → `Email/query` → `Email/get`

If `mailboxName` is provided, first resolve it to an ID via `Mailbox/get`, matching
case-insensitively.

## Returns

Array of emails:

| Field      | Type     | Description                    |
|------------|----------|--------------------------------|
| `id`       | string   | Email ID                       |
| `subject`  | string   | Email subject                  |
| `from`     | object[] | Sender addresses               |
| `to`       | object[] | Recipient addresses            |
| `date`     | string   | Received date (ISO 8601)       |
| `preview`  | string   | Short text preview             |
| `textBody` | string   | Plain text body (if includeBody) |
| `htmlBody` | string   | HTML body (if includeBody)     |

## Error Cases

- Neither `mailboxId` nor `mailboxName` provided → `isError: true`, "must provide mailboxId or mailboxName".
- `mailboxName` not found → `isError: true`, "mailbox not found: {name}".
- JMAP error → `isError: true` with JMAP error message.
