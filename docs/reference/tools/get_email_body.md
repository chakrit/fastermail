# get_email_body

Get full body of a single email.

## Parameters

| Param    | Type   | Required | Description                           |
|----------|--------|----------|---------------------------------------|
| `emailId`| string | yes      | Email ID                              |
| `format` | string | no       | `text`, `html`, or `both` (default `text`) |

## JMAP

**Capability:** `urn:ietf:params:jmap:mail`
**Method:** `Email/get`

Request properties depend on `format`:
- `text` → `fetchTextBodyValues: true`
- `html` → `fetchHTMLBodyValues: true`
- `both` → `fetchAllBodyValues: true`

## Returns

| Field      | Type   | Description                     |
|------------|--------|---------------------------------|
| `id`       | string | Email ID                        |
| `subject`  | string | Email subject                   |
| `textBody` | string | Plain text body (if text/both)  |
| `htmlBody` | string | HTML body (if html/both)        |

## Error Cases

- Missing `emailId` → `isError: true`, "emailId is required".
- Invalid `format` value → `isError: true`, "format must be text, html, or both".
- Email not found → `isError: true`, "email not found: {id}".
