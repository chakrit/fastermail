# delete_contact

**Phase 2 — CardDAV** (FastMail does not yet expose contacts via JMAP)

Delete a contact.

## Parameters

| Param      | Type   | Required | Description |
|------------|--------|----------|-------------|
| `contactId`| string | yes      | Contact ID  |

## Protocol

**CardDAV:** DELETE the vCard resource.
**Future JMAP:** `ContactCard/set` (destroy)

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `success` | boolean | Whether delete succeeded |

## Error Cases

- Missing `contactId` → `isError: true`, "contactId is required".
- Contact not found → `isError: true`, "contact not found: {id}".
- CardDAV/JMAP error → `isError: true` with error message.
