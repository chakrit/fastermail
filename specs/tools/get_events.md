# get_events

**Phase 2 — CalDAV** (FastMail does not yet expose calendars via JMAP)

Get calendar events.

## Parameters

| Param        | Type    | Required | Description                    |
|--------------|---------|----------|--------------------------------|
| `calendarId` | string  | no       | Filter by calendar             |
| `after`      | string  | no       | Start date (YYYY-MM-DD)        |
| `before`     | string  | no       | End date (YYYY-MM-DD)          |
| `limit`      | integer | no       | Max results (default 50)       |

## Protocol

**CalDAV:** REPORT with calendar-query and time-range filter.
**Future JMAP:** `CalendarEvent/query` (with `expandRecurrences`) → `CalendarEvent/get`

## Returns

Array of events with `id`, `title`, `start`, `end`, `duration`, `location`, `description`.

## Error Cases

- CalDAV/JMAP error → `isError: true` with error message.
