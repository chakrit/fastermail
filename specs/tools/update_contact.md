# update_contact

**Phase 2 — CardDAV** (FastMail does not yet expose contacts via JMAP)

Update an existing contact.

## Parameters

| Param      | Type     | Required | Description              |
|------------|----------|----------|--------------------------|
| `contactId`| string   | yes      | Contact ID               |
| `name`     | string   | no       | Updated name             |
| `emails`   | object[] | no       | Updated emails           |
| `phones`   | object[] | no       | Updated phones           |
| `company`  | string   | no       | Updated organization     |
| `notes`    | string   | no       | Updated notes            |

## Protocol

**CardDAV:** GET the existing vCard, modify, PUT back.
**Future JMAP:** `ContactCard/set`

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `success` | boolean | Whether update succeeded |

## Error Cases

- Missing `contactId` → `isError: true`, "contactId is required".
- Contact not found → `isError: true`, "contact not found: {id}".
- CardDAV/JMAP error → `isError: true` with error message.
