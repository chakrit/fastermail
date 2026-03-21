# create_contact

**Phase 2 — CardDAV** (FastMail does not yet expose contacts via JMAP)

Create a new contact.

## Parameters

| Param          | Type     | Required | Description              |
|----------------|----------|----------|--------------------------|
| `name`         | string   | yes      | Full name                |
| `emails`       | object[] | no       | Array of `{type, value}` |
| `phones`       | object[] | no       | Array of `{type, value}` |
| `company`      | string   | no       | Organization name        |
| `notes`        | string   | no       | Free-text notes          |
| `addressBookId`| string   | no       | Target address book      |

## Protocol

**CardDAV:** PUT a new vCard to the address book URL.
**Future JMAP:** `ContactCard/set`

## Returns

| Field       | Type   | Description      |
|-------------|--------|------------------|
| `contactId` | string | New contact ID   |

## Error Cases

- Missing `name` → `isError: true`, "name is required".
- CardDAV/JMAP error → `isError: true` with error message.
