# get_vacation_response

Get the current vacation/auto-reply settings.

## Parameters

None.

## JMAP

**Capability:** `urn:ietf:params:jmap:vacationresponse`
**Method:** `VacationResponse/get`

Request the singleton vacation response object for the account.

## Returns

| Field       | Type    | Description                   |
|-------------|---------|-------------------------------|
| `isEnabled` | boolean | Whether auto-reply is active  |
| `fromDate`  | string  | Start date (ISO 8601, UTC)    |
| `toDate`    | string  | End date (ISO 8601, UTC)      |
| `subject`   | string  | Auto-reply subject            |
| `textBody`  | string  | Plain text auto-reply body    |
| `htmlBody`  | string  | HTML auto-reply body          |

## Error Cases

- JMAP error → `isError: true` with JMAP error message.
