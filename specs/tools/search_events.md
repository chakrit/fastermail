# search_events

**Phase 2 — CalDAV** (FastMail does not yet expose calendars via JMAP)

Search calendar events.

## Parameters

| Param   | Type    | Required | Description           |
|---------|---------|----------|-----------------------|
| `query` | string  | yes      | Search text           |
| `after` | string  | no       | Start date bound      |
| `before`| string  | no       | End date bound        |
| `limit` | integer | no       | Max results (default 20) |

## Protocol

**CalDAV:** REPORT with text-match and optional time-range filter.
**Future JMAP:** `CalendarEvent/query` → `CalendarEvent/get`

## Returns

Array of events matching the search query.

## Error Cases

- Missing `query` → `isError: true`, "query is required".
- CalDAV/JMAP error → `isError: true` with error message.
