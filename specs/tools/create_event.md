# create_event

**Phase 2 — CalDAV** (FastMail does not yet expose calendars via JMAP)

Create a calendar event.

## Parameters

| Param         | Type     | Required | Description                |
|---------------|----------|----------|----------------------------|
| `title`       | string   | yes      | Event title                |
| `start`       | string   | yes      | ISO 8601 datetime          |
| `duration`    | string   | yes      | ISO 8601 duration (e.g. `PT1H`) |
| `calendarId`  | string   | no       | Target calendar            |
| `description` | string   | no       | Event description          |
| `location`    | string   | no       | Event location             |
| `participants`| object[] | no       | Array of `{name, email}`   |
| `timeZone`    | string   | no       | IANA timezone              |

## Protocol

**CalDAV:** PUT a new iCalendar VEVENT to the calendar URL.
**Future JMAP:** `CalendarEvent/set`

## Returns

| Field     | Type   | Description    |
|-----------|--------|----------------|
| `eventId` | string | New event ID   |

## Error Cases

- Missing `title`, `start`, or `duration` → `isError: true`, "{field} is required".
- CalDAV/JMAP error → `isError: true` with error message.
