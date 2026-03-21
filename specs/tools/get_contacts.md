# get_contacts

**Phase 2 — CardDAV** (FastMail does not yet expose contacts via JMAP)

Get contacts from an address book.

## Parameters

| Param          | Type    | Required | Description                  |
|----------------|---------|----------|------------------------------|
| `addressBookId`| string  | no       | Filter by address book       |
| `limit`        | integer | no       | Max results (default 50)     |

## Protocol

**CardDAV:** REPORT on the address book URL with addressbook-query.
**Future JMAP:** `ContactCard/query` → `ContactCard/get`

## Returns

Array of contacts with `id`, `name`, `emails`, `phones`, `company`.

## Error Cases

- CardDAV/JMAP error → `isError: true` with error message.
