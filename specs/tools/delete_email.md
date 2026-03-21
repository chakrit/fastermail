# delete_email

Delete emails (move to Trash, or permanently delete if already in Trash).

## Parameters

| Param       | Type     | Required | Description                |
|-------------|----------|----------|----------------------------|
| `emailIds`  | string[] | yes      | Email IDs to delete        |
| `permanent` | boolean  | no       | Skip trash (default false) |

## JMAP

**Capability:** `urn:ietf:params:jmap:mail`
**Method:** `Email/set`

If `permanent` is false (default):
- Update `mailboxIds` to move the email to the Trash mailbox.
- Requires resolving the Trash mailbox ID via `Mailbox/get` with `role: "trash"`.

If `permanent` is true:
- Use `destroy` to permanently delete the emails.

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `deleted` | integer | Number of emails deleted  |

## Error Cases

- Missing `emailIds` → `isError: true`, "emailIds is required".
- JMAP error → `isError: true` with error details.
