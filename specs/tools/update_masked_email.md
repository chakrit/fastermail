# update_masked_email

Enable, disable, or delete a masked email address.

## Parameters

| Param   | Type   | Required | Description                              |
|---------|--------|----------|------------------------------------------|
| `id`    | string | yes      | Masked email ID                          |
| `state` | string | yes      | New state: `enabled`, `disabled`, `deleted` |

## JMAP

**Capability:** `https://www.fastmail.com/dev/maskedemail`
**Method:** `MaskedEmail/set`

Update the `state` property of the specified masked email.

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `success` | boolean | Whether update succeeded |

## Error Cases

- Missing `id` or `state` → `isError: true`, "{field} is required".
- Invalid `state` → `isError: true`, "state must be enabled, disabled, or deleted".
- JMAP error → `isError: true` with JMAP error message.
