# create_masked_email

Create a new masked email address.

## Parameters

| Param         | Type   | Required | Description                         |
|---------------|--------|----------|-------------------------------------|
| `forDomain`   | string | no       | Domain this address is for          |
| `description` | string | no       | Human-readable label                |
| `emailPrefix` | string | no       | Preferred prefix for the address    |

## JMAP

**Capability:** `https://www.fastmail.com/dev/maskedemail`
**Method:** `MaskedEmail/set`

Create with `state: "enabled"`. The `forDomain` should be a bare domain (no path).

## Returns

| Field   | Type   | Description                |
|---------|--------|----------------------------|
| `id`    | string | New masked email ID        |
| `email` | string | The new masked address     |

## Error Cases

- Rate limit exceeded → `isError: true`, "rate limit exceeded".
- JMAP error → `isError: true` with JMAP error message.
