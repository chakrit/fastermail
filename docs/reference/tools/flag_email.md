# flag_email

Set/unset flags (keywords) on emails.

## Parameters

| Param      | Type     | Required | Description                    |
|------------|----------|----------|--------------------------------|
| `emailIds` | string[] | yes      | Email IDs                      |
| `flag`     | string   | yes      | Flag: `seen`, `flagged`, `answered`, `draft` |
| `value`    | boolean  | yes      | Set (true) or unset (false)    |

## JMAP

**Capability:** `urn:ietf:params:jmap:mail`
**Method:** `Email/set` (update)

Map flag names to JMAP keywords:
- `seen` → `$seen`
- `flagged` → `$flagged`
- `answered` → `$answered`
- `draft` → `$draft`

Update `keywords/<keyword>` to `value` for each email.

## Returns

| Field     | Type    | Description               |
|-----------|---------|---------------------------|
| `updated` | integer | Number of emails updated  |

## Error Cases

- Missing required params → `isError: true`, "{field} is required".
- Invalid flag name → `isError: true`, "invalid flag: {flag}".
- JMAP error → `isError: true` with error details.
