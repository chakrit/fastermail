# set_vacation_response

Enable, disable, or update the vacation auto-reply.

## Parameters

| Param       | Type    | Required | Description                        |
|-------------|---------|----------|------------------------------------|
| `isEnabled` | boolean | yes      | Enable or disable auto-reply       |
| `fromDate`  | string  | no       | Start date (ISO 8601, UTC)         |
| `toDate`    | string  | no       | End date (ISO 8601, UTC)           |
| `subject`   | string  | no       | Auto-reply subject                 |
| `textBody`  | string  | no       | Plain text auto-reply body         |
| `htmlBody`  | string  | no       | HTML auto-reply body               |

## JMAP

**Capability:** `urn:ietf:params:jmap:vacationresponse`
**Method:** `VacationResponse/set`

Update the singleton vacation response object. Only provided fields are updated.

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `success` | boolean | Whether update succeeded |

## Error Cases

- Missing `isEnabled` → `isError: true`, "isEnabled is required".
- JMAP error → `isError: true` with JMAP error message.
