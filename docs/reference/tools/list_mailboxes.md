# list_mailboxes

List all mailboxes with metadata.

## Parameters

| Param  | Type   | Required | Description                          |
|--------|--------|----------|--------------------------------------|
| `role` | string | no       | Filter by role (inbox, sent, drafts, trash, junk, archive) |

## JMAP

**Capability:** `urn:ietf:params:jmap:mail`
**Method:** `Mailbox/get`

## Returns

Array of mailboxes:

| Field         | Type    | Description           |
|---------------|---------|-----------------------|
| `id`          | string  | Mailbox ID            |
| `name`        | string  | Display name          |
| `role`        | string  | Standard role or null |
| `totalEmails` | integer | Total email count     |
| `unreadEmails`| integer | Unread email count    |
| `parentId`    | string  | Parent mailbox ID     |

## Error Cases

- JMAP error → `isError: true` with JMAP error message.
