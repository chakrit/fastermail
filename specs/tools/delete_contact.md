# delete_contact

Delete a contact.

## Parameters

| Param       | Type   | Required | Description |
|-------------|--------|----------|-------------|
| `contactId` | string | yes      | Contact ID  |

## JMAP

**Capability:** `urn:ietf:params:jmap:contacts`
**Method:** `ContactCard/set` (destroy)

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `success` | boolean | Whether delete succeeded |

## Error Cases

- Missing `contactId` → `isError: true`, "contactId is required".
- Contact not found → JMAP `notFound` error → `isError: true`, "contact not found: {id}".
- JMAP error → `isError: true` with JMAP error message.
