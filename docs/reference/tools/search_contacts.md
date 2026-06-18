# search_contacts

Search contacts by name, email, phone, etc.

## Parameters

| Param   | Type    | Required | Description              |
|---------|---------|----------|--------------------------|
| `query` | string  | yes      | Search text              |
| `limit` | integer | no       | Max results (default 20) |

## JMAP

**Capability:** `urn:ietf:params:jmap:contacts`
**Methods:** `ContactCard/query` → `ContactCard/get` (back-reference)

Filter: `text` filter condition (searches all text fields, case-insensitive).
Sort: `updated` descending.

## Returns

Same shape as `get_contacts` — array of flattened contacts.

## Error Cases

- Missing `query` → `isError: true`, "query is required".
- JMAP error → `isError: true` with JMAP error message.
