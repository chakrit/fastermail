# update_contact

Update an existing contact.

## Parameters

| Param       | Type     | Required | Description          |
|-------------|----------|----------|----------------------|
| `contactId` | string   | yes      | Contact ID           |
| `name`      | string   | no       | Updated full name    |
| `emails`    | object[] | no       | Updated emails       |
| `phones`    | object[] | no       | Updated phones       |
| `company`   | string   | no       | Updated organization |
| `notes`     | string   | no       | Updated notes        |

At least one field besides `contactId` is required.

## JMAP

**Capability:** `urn:ietf:params:jmap:contacts`
**Method:** `ContactCard/set` (update)

Same translation as `create_contact` — only provided fields are sent as patches.
JMAP `/set` update semantics: omitted properties are unchanged, explicit `null` clears.

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `success` | boolean | Whether update succeeded |

## Error Cases

- Missing `contactId` → `isError: true`, "contactId is required".
- No fields to update → `isError: true`, "at least one field to update is required".
- Contact not found → JMAP `notFound` error → `isError: true`, "contact not found: {id}".
- JMAP error → `isError: true` with JMAP error message.
