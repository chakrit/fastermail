# move_email

Move emails between mailboxes.

## Parameters

| Param       | Type     | Required | Description            |
|-------------|----------|----------|------------------------|
| `emailIds`  | string[] | yes      | Email IDs to move      |
| `mailboxId` | string   | yes      | Destination mailbox ID |

## JMAP

**Capability:** `urn:ietf:params:jmap:mail`
**Method:** `Email/set` (update)

For each email, update `mailboxIds` to `{ "<mailboxId>": true }`, removing all other
mailbox memberships.

## Returns

| Field     | Type    | Description           |
|-----------|---------|-----------------------|
| `moved`   | integer | Number of emails moved |

## Error Cases

- Missing `emailIds` or `mailboxId` → `isError: true`, "{field} is required".
- JMAP error → `isError: true` with error details.
