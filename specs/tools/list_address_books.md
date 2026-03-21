# list_address_books

**Phase 2 — CardDAV** (FastMail does not yet expose contacts via JMAP)

List all address books.

## Parameters

None.

## Protocol

**CardDAV:** PROPFIND on the addressbook-home-set URL.
**Future JMAP:** `AddressBook/get`

## Returns

Array of address books:

| Field       | Type    | Description        |
|-------------|---------|--------------------|
| `id`        | string  | Address book ID    |
| `name`      | string  | Display name       |
| `isDefault` | boolean | Default address book |

## Error Cases

- CardDAV/JMAP error → `isError: true` with error message.
