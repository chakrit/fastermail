# send_email

Compose and send an email.

## Parameters

| Param       | Type     | Required | Description                |
|-------------|----------|----------|----------------------------|
| `to`        | string[] | yes      | Recipient addresses        |
| `subject`   | string   | yes      | Email subject              |
| `body`      | string   | yes      | Email body                 |
| `cc`        | string[] | no       | CC recipients              |
| `bcc`       | string[] | no       | BCC recipients             |
| `isHtml`    | boolean  | no       | Body is HTML (default false) |
| `inReplyTo` | string   | no       | Email ID being replied to  |

## JMAP

**Capabilities:** `urn:ietf:params:jmap:mail`, `urn:ietf:params:jmap:submission`
**Methods:** `Email/set` (create draft) → `EmailSubmission/set` (submit)

Two-step process in a single JMAP request:

1. `Email/set` — create the email object with `$draft` keyword, body parts, and recipients.
   Use back-reference creation ID (e.g., `#draft`).
2. `EmailSubmission/set` — create a submission referencing the draft via
   `emailId: "#draft"` and `identityId` from the user's primary identity.

If `inReplyTo` is provided, set the `In-Reply-To` and `References` headers from the
original email.

## Returns

| Field     | Type   | Description              |
|-----------|--------|--------------------------|
| `success` | boolean| Whether send succeeded   |
| `emailId` | string | ID of the sent email     |

## Error Cases

- Missing `to`, `subject`, or `body` → `isError: true`, "{field} is required".
- JMAP submission error → `isError: true` with error details.
