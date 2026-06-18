# list_address_books

List all address books.

## Parameters

None.

## JMAP

**Capability:** `urn:ietf:params:jmap:contacts`
**Method:** `AddressBook/get`

## Returns

Array of address books:

| Field         | Type    | Description          |
|---------------|---------|----------------------|
| `id`          | string  | Address book ID      |
| `name`        | string  | Display name         |
| `description` | string  | Description or null  |
| `isDefault`   | boolean | Default address book |

## Error Cases

- JMAP error → `isError: true` with JMAP error message.
