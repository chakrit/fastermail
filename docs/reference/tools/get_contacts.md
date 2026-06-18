# get_contacts

Get contacts from an address book.

## Parameters

| Param           | Type    | Required | Description              |
|-----------------|---------|----------|--------------------------|
| `addressBookId` | string  | no       | Filter by address book   |
| `limit`         | integer | no       | Max results (default 50) |

## JMAP

**Capability:** `urn:ietf:params:jmap:contacts`
**Methods:** `ContactCard/query` → `ContactCard/get` (back-reference)

Filter: if `addressBookId` provided, use `inAddressBook` filter condition.
Sort: `updated` descending.

## Returns

Array of contacts (flattened from JSContact `ContactCard`):

| Field          | Type     | Description                                    |
|----------------|----------|------------------------------------------------|
| `id`           | string   | Contact ID                                     |
| `name`         | string   | Full name (`name.full` or joined components)   |
| `emails`       | object[] | `[{type, address}]` — flattened from Id map    |
| `phones`       | object[] | `[{type, number}]` — flattened from Id map     |
| `company`      | string   | First organization name, or empty               |
| `addressBookIds` | string[] | Address book IDs this contact belongs to     |

The action flattens JSContact structures into this simpler shape. `type` is derived
from the `contexts` map (`work`, `private`) or defaults to `other`.

## Error Cases

- JMAP error → `isError: true` with JMAP error message.
