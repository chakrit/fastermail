# update_event

**Phase 2 — CalDAV** (FastMail does not yet expose calendars via JMAP)

Update a calendar event.

## Parameters

| Param        | Type   | Required | Description                |
|--------------|--------|----------|----------------------------|
| `eventId`    | string | yes      | Event ID                   |
| `title`      | string | no       | Updated title              |
| `start`      | string | no       | Updated start time         |
| `duration`   | string | no       | Updated duration           |
| `description`| string | no       | Updated description        |
| `location`   | string | no       | Updated location           |

## Protocol

**CalDAV:** GET the existing iCalendar, modify, PUT back.
**Future JMAP:** `CalendarEvent/set`

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `success` | boolean | Whether update succeeded |

## Error Cases

- Missing `eventId` → `isError: true`, "eventId is required".
- Event not found → `isError: true`, "event not found: {id}".
- CalDAV/JMAP error → `isError: true` with error message.
