# create_contact

Create a new contact.

## Parameters

| Param           | Type     | Required | Description                          |
|-----------------|----------|----------|--------------------------------------|
| `name`          | string   | yes      | Full name                            |
| `emails`        | object[] | no       | Array of `{type, address}`           |
| `phones`        | object[] | no       | Array of `{type, number}`            |
| `company`       | string   | no       | Organization name                    |
| `notes`         | string   | no       | Free-text notes                      |
| `addressBookId` | string   | no       | Target address book (default book if omitted) |

## JMAP

**Capability:** `urn:ietf:params:jmap:contacts`
**Method:** `ContactCard/set` (create)

The action translates flat params into a JSContact `Card`:
- `name` → `Name { full: name }`
- `emails` → `Id[EmailAddress]` map with `contexts` from `type` (`work`/`private`)
- `phones` → `Id[Phone]` map with `contexts` from `type`
- `company` → `Organization { name: company }` in the `organizations` map
- `notes` → `notes` string
- `addressBookId` → `addressBookIds: { id: true }`

Server generates `uid` and `version`.

## Returns

| Field       | Type   | Description    |
|-------------|--------|----------------|
| `contactId` | string | New contact ID |

## Error Cases

- Missing `name` → `isError: true`, "name is required".
- JMAP error → `isError: true` with JMAP error message.
