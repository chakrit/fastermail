# FasterMail Session State

Saved: 2026-03-25

## Uncommitted Changes

12 files modified, 254 insertions, 65 deletions. Build clean, 21 tests pass, 0 warnings.

### Code Fixes Applied

**High severity (broken functionality → fixed):**
- `src/actions/email.rs`: Body content extraction — `get_emails`, `search_emails`, `get_email_body` now extract actual text from JMAP `bodyValues` instead of returning raw part-reference arrays
- `src/actions/email.rs`: `search_emails` now includes body properties when `includeBody=true`
- `src/actions/email.rs`: `get_emails` now fetches both text AND HTML body values (`fetchHTMLBodyValues`)

**Medium severity (incorrect but functional → fixed):**
- `src/actions/email.rs`: `move_email`/`delete_email`/`flag_email` return actual JMAP response counts instead of input array length
- `src/actions/email.rs`: `send_email` checks for JMAP-level submission failures (`notCreated`)
- `src/actions/email.rs`: Added `date` field mapped from `receivedAt` for user-friendliness
- `src/actions/mod.rs`: Added `project_fields()` and `project_fields_array()` helpers
- `src/actions/mailbox.rs`: `list_mailboxes` projects to spec-defined fields only
- `src/actions/vacation.rs`: `get_vacation_response` projects to spec-defined fields only
- `src/actions/vacation.rs`: `set_vacation_response` can now clear fields via null (uses `raw_args` + `resolve_field()`)
- `src/actions/identity.rs`: `list_identities` projects to spec-defined fields only
- `src/actions/masked_email.rs`: `list_masked_emails`/`create_masked_email` project to spec-defined fields; state validation moved before JMAP call
- `src/mcp/server.rs`: `jsonrpc: "2.0"` validation added
- `src/mcp/server.rs`: Handshake enforcement (must `initialize` before `tools/call`)
- `src/mcp/handler.rs`: `flag_email` validates `value` param is present (required)
- `src/mcp/handler.rs`: `set_vacation_response` dispatch passes `raw_args`

**Low severity (cosmetic/spec drift → fixed):**
- `src/actions/email.rs`, `src/actions/mailbox.rs`: Schema enum constraints added for `format`, `flag`, `role`
- `src/error.rs`: Removed `#[allow(dead_code)]` from `MissingToken`
- `src/main.rs`: `MissingToken` variant now properly constructed
- `src/mcp/types.rs`: Removed `#[allow(dead_code)]` from `INVALID_REQUEST`
- `src/recorder.rs`: JSON type fields changed to `mcp_req`/`mcp_resp` (was `mcp_request`/`mcp_response`)

### New Files (uncommitted)
- `specs/cli.md` — Full CLI spec (resource-verb structure, output modes, auth, per-command args, architecture)
- `DOCS.md` — 5-minute user-facing explainer of how FasterMail works

## Completed Tasks

1. **Full spec audit** — 4 parallel audits covering all code vs all specs
   - Email tools (7 tools): 3 high, 5 medium, 4 low severity findings
   - Non-email tools (8 tools): 1 systemic (response projection), 3 tool-specific issues
   - Protocol & architecture: 2 protocol, 6 testing, 3 distribution gaps
   - Phase 2 readiness: 40% ready, major finding about JMAP availability

2. **All code fixes applied** — every audit finding addressed

3. **CLI spec written** — `specs/cli.md` covers:
   - `fm` binary name, `fm mcp` for MCP mode
   - Resource-verb command structure (`fm emails list`, `fm mailboxes create`, etc.)
   - Three output modes: human tables (default), `--json`, `--raw`
   - Auth via env var + config file (`~/.config/fastermail/config.toml`)
   - Per-command argument definitions with short aliases
   - `src/cli/` module architecture, exit codes, stdin body input for send

4. **DOCS.md written** — covers architecture diagram, all 15 tools explained, auth, protocols (JMAP/MCP), output formats

5. **Phase 2 research** — FastMail likely supports contacts/calendars via JMAP with vendor-specific capability URIs (`https://www.fastmail.com/dev/contacts`, `https://www.fastmail.com/dev/calendars`). Needs verification via:
   ```bash
   curl -s https://api.fastmail.com/jmap/session \
     -H "Authorization: Bearer $FASTMAIL_API_TOKEN" | jq '.capabilities | keys'
   ```

## TODO — Not Started

### Immediate (this sprint)
- [ ] **Review cycle**: User reviews `specs/cli.md` and `DOCS.md`, iterate
- [ ] **Commit current changes**: ~250 lines of fixes + 2 new files
- [ ] **README.md**: 30-second install/configure/use guide for regular FastMail users
- [ ] **Verify Phase 2 JMAP**: Run curl against session endpoint
- [ ] **Implement CLI**: Add clap, `src/cli/` module, rename binary to `fm`, wire up subcommands

### Later
- [ ] **Phase 2 spec rewrite**: CardDAV/CalDAV → JMAP (pending verification)
- [ ] **Test infrastructure**: `src/testutil/mock_jmap.rs`, per-action unit tests, `tests/integration.rs` (big gap per testing.md)
- [ ] **Dockerfile**: Multi-stage build for distribution
- [ ] **CI/release**: Cross-compilation for 4 targets (x86_64/aarch64 × linux/macos)
- [ ] **Phase 2 implementation**: Contacts + calendars tools

## Key Decisions Made
- Binary name: `fm` (short), MCP mode via `fm mcp`
- CLI structure: resource-first then verb (`fm emails list`)
- Output: human-friendly default, `--json` for scripting, `--raw` for debug
- Auth: env var takes precedence over config file
- No default command → show help (not MCP server)
- Fix code for usability over raw JMAP compliance
- Response projection: filter to spec-defined fields
- DOCS.md for 5-min read, README for 30-sec quickstart, specs/ for implementers

## Audit Reports

Full audit transcripts are available at:
- Email tools: `/private/tmp/claude-501/-Users-chakrit-Documents-chakrit-fastermail/0da03771-a783-4e90-b365-0a4cbc5dd03e/tasks/a11c2e68d8ca0cb36.output`
- Non-email tools: `/private/tmp/claude-501/-Users-chakrit-Documents-chakrit-fastermail/0da03771-a783-4e90-b365-0a4cbc5dd03e/tasks/a6827d80a2be227aa.output`
- Protocol & architecture: `/private/tmp/claude-501/-Users-chakrit-Documents-chakrit-fastermail/0da03771-a783-4e90-b365-0a4cbc5dd03e/tasks/ad04cb2e52e6c8563.output`
- Phase 2 readiness: `/private/tmp/claude-501/-Users-chakrit-Documents-chakrit-fastermail/0da03771-a783-4e90-b365-0a4cbc5dd03e/tasks/a10582c403c0038ef.output`

(These are in /private/tmp and may not survive reboot.)
