# delete_event

**Phase 2 — CalDAV** (FastMail does not yet expose calendars via JMAP)

Delete a calendar event.

## Parameters

| Param    | Type    | Required | Description                    |
|----------|---------|----------|--------------------------------|
| `eventId`| string  | yes      | Event ID                       |
| `notify` | boolean | no       | Send cancellation to participants (default true) |

## Protocol

**CalDAV:** DELETE the iCalendar resource. If `notify` is true, send iTIP CANCEL to participants.
**Future JMAP:** `CalendarEvent/set` (destroy) with `sendSchedulingMessages`

## Returns

| Field     | Type    | Description              |
|-----------|---------|--------------------------|
| `success` | boolean | Whether delete succeeded |

## Error Cases

- Missing `eventId` → `isError: true`, "eventId is required".
- Event not found → `isError: true`, "event not found: {id}".
- CalDAV/JMAP error → `isError: true` with error message.
