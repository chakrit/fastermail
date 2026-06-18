# manage_mailbox

Create, rename, or delete mailboxes.

## Parameters

| Param       | Type   | Required | Description                              |
|-------------|--------|----------|------------------------------------------|
| `action`    | string | yes      | `create`, `rename`, or `delete`          |
| `name`      | string | yes*     | Name for create/rename (* required for create/rename) |
| `mailboxId` | string | yes*     | ID of mailbox (* required for rename/delete) |
| `parentId`  | string | no       | Parent mailbox ID for create             |

## JMAP

**Capability:** `urn:ietf:params:jmap:mail`
**Method:** `Mailbox/set`

- `create` → `Mailbox/set` with `create` containing `name` and optional `parentId`.
- `rename` → `Mailbox/set` with `update` setting `name` on the given `mailboxId`.
- `delete` → `Mailbox/set` with `destroy` containing the `mailboxId`.

## Returns

| Field       | Type   | Description                        |
|-------------|--------|------------------------------------|
| `success`   | boolean| Whether the operation succeeded    |
| `mailboxId` | string | ID of created/modified mailbox     |

## Error Cases

- Missing `action` → `isError: true`, "action is required".
- Invalid `action` → `isError: true`, "action must be create, rename, or delete".
- `create`/`rename` without `name` → `isError: true`, "name is required for {action}".
- `rename`/`delete` without `mailboxId` → `isError: true`, "mailboxId is required for {action}".
- JMAP error → `isError: true` with error details.
