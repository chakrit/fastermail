# Tool Index

Each tool is an action struct implementing the unit-of-work pattern. Tool names use
snake_case. Parameters use camelCase (matching JMAP conventions in the MCP schema).

## Phase 1 — JMAP (Available Now)

### Email

| Tool | Description | Spec |
|------|-------------|------|
| `list_mailboxes` | List all mailboxes with metadata | [list_mailboxes.md](list_mailboxes.md) |
| `get_emails` | Retrieve emails from a mailbox | [get_emails.md](get_emails.md) |
| `search_emails` | Search emails with filters | [search_emails.md](search_emails.md) |
| `get_email_body` | Get full body of a single email | [get_email_body.md](get_email_body.md) |
| `send_email` | Compose and send an email | [send_email.md](send_email.md) |
| `move_email` | Move emails between mailboxes | [move_email.md](move_email.md) |
| `delete_email` | Delete emails | [delete_email.md](delete_email.md) |
| `flag_email` | Set/unset flags on emails | [flag_email.md](flag_email.md) |
| `manage_mailbox` | Create, rename, or delete mailboxes | [manage_mailbox.md](manage_mailbox.md) |

### Vacation Response

| Tool | Description | Spec |
|------|-------------|------|
| `get_vacation_response` | Get current vacation/auto-reply settings | [get_vacation_response.md](get_vacation_response.md) |
| `set_vacation_response` | Enable/disable/update vacation auto-reply | [set_vacation_response.md](set_vacation_response.md) |

### Identity

| Tool | Description | Spec |
|------|-------------|------|
| `list_identities` | List sending identities (From addresses) | [list_identities.md](list_identities.md) |

### Masked Email (FastMail-specific)

| Tool | Description | Spec |
|------|-------------|------|
| `list_masked_emails` | List all masked email addresses | [list_masked_emails.md](list_masked_emails.md) |
| `create_masked_email` | Create a new masked email address | [create_masked_email.md](create_masked_email.md) |
| `update_masked_email` | Enable/disable/delete a masked email | [update_masked_email.md](update_masked_email.md) |

## Phase 2 — CardDAV/CalDAV (FastMail lacks JMAP support)

### Contacts

| Tool | Description | Spec |
|------|-------------|------|
| `list_address_books` | List all address books | [list_address_books.md](list_address_books.md) |
| `get_contacts` | Get contacts from an address book | [get_contacts.md](get_contacts.md) |
| `search_contacts` | Search contacts | [search_contacts.md](search_contacts.md) |
| `create_contact` | Create a new contact | [create_contact.md](create_contact.md) |
| `update_contact` | Update an existing contact | [update_contact.md](update_contact.md) |
| `delete_contact` | Delete a contact | [delete_contact.md](delete_contact.md) |

### Calendars

| Tool | Description | Spec |
|------|-------------|------|
| `list_calendars` | List all calendars | [list_calendars.md](list_calendars.md) |
| `get_events` | Get calendar events | [get_events.md](get_events.md) |
| `search_events` | Search calendar events | [search_events.md](search_events.md) |
| `create_event` | Create a calendar event | [create_event.md](create_event.md) |
| `update_event` | Update a calendar event | [update_event.md](update_event.md) |
| `delete_event` | Delete a calendar event | [delete_event.md](delete_event.md) |
