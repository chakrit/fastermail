# list_masked_emails

List all masked (disposable) email addresses.

## Parameters

| Param   | Type   | Required | Description                  |
|---------|--------|----------|------------------------------|
| `state` | string | no       | Filter: `pending`, `enabled`, `disabled`, `deleted` |

## JMAP

**Capability:** `https://www.fastmail.com/dev/maskedemail`
**Method:** `MaskedEmail/get`

If `state` is provided, filter results client-side after fetching all masked emails.

## Returns

Array of masked emails:

| Field         | Type   | Description                |
|---------------|--------|----------------------------|
| `id`          | string | Masked email ID            |
| `email`       | string | The masked email address   |
| `forDomain`   | string | Domain this address is for |
| `description` | string | Human-readable label       |
| `state`       | string | Current state              |
| `createdAt`   | string | Creation timestamp         |

## Error Cases

- Invalid `state` value → `isError: true`, "state must be pending, enabled, disabled, or deleted".
- JMAP error → `isError: true` with JMAP error message.
