# list_identities

List sending identities (From addresses).

## Parameters

None.

## JMAP

**Capability:** `urn:ietf:params:jmap:submission`
**Method:** `Identity/get`

## Returns

Array of identities:

| Field     | Type   | Description             |
|-----------|--------|-------------------------|
| `id`      | string | Identity ID             |
| `name`    | string | Display name            |
| `email`   | string | Email address           |
| `replyTo` | string | Reply-to address        |

## Error Cases

- JMAP error → `isError: true` with JMAP error message.
