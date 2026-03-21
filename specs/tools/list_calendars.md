# list_calendars

**Phase 2 — CalDAV** (FastMail does not yet expose calendars via JMAP)

List all calendars.

## Parameters

None.

## Protocol

**CalDAV:** PROPFIND on the calendar-home-set URL.
**Future JMAP:** `Calendar/get`

## Returns

Array of calendars:

| Field       | Type    | Description      |
|-------------|---------|------------------|
| `id`        | string  | Calendar ID      |
| `name`      | string  | Display name     |
| `color`     | string  | Calendar color   |
| `isDefault` | boolean | Default calendar |

## Error Cases

- CalDAV/JMAP error → `isError: true` with error message.
