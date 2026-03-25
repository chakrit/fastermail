# FasterMail Session State

Saved: 2026-03-25 (session 6)

## Way of Work

1. **Memory check** — restate this workflow, list tasks, identify next one
2. **Confirm plan** — get user approval before starting
3. **Do the work**
4. **Self-audit** — check results against instructions/specs/conventions
5. **Repeat audit** until no gaps remain
6. **Prepare compaction note** — user compacts, cycle restarts

## Completed Tasks

1. ✅ **Full spec audit** — 4 parallel audits covering all code vs all specs
2. ✅ **All code fixes applied** — every audit finding addressed (254 ins, 65 del across 12 files)
3. ✅ **CLI spec written** — `specs/cli.md`
4. ✅ **DOCS.md written** — 5-minute explainer
5. ✅ **Committed** — 3 commits:
   - `db55e33` Fix audit findings: body extraction, response projection, protocol compliance
   - `d01b949` Add CLI spec: fm binary with resource-verb command structure
   - `e459584` Add DOCS.md and session state for continuity
6. ✅ **Updated specs/cli.md** — organizing focus, mailbox resolution, ACE UX patterns
   - `e907069` Update CLI spec: organizing focus, mailbox resolution, ACE UX patterns
7. ✅ **CLI skeleton implemented** — clap subcommands, Io/OutputMode, TerminalGuard, exit codes
   - `4dc8e45` Add CLI skeleton: fm binary with clap subcommands, Io struct, output modes
8. ✅ **Wire CLI handlers to actions** — all stubs replaced with real action calls + output formatting
   - 828 insertions, 130 deletions across 8 files
   - All 16 CLI commands wired to their action structs
   - Human mode: tables for lists, formatted headers for email body, status messages for mutations
   - Json/Raw mode: pretty-printed JSON via `Io::json()`
   - Spinners via `Io::progress()` for all async operations
   - Stdin body reading for `fm emails send` when `--body` omitted
   - Fixed `GetEmailBody` action to request `from`, `to`, `receivedAt` properties
   - Default `emails list` to inbox when `--mailbox` omitted

## TODO — Not Started

### Immediate (this sprint)
- [ ] **Mailbox resolution** — role aliases + fuzzy name matching in CLI
  - Currently: `emails list` uses action's exact-name match; `search`/`move` pass raw input as mailbox_id
  - Need: role alias lookup (inbox→role:inbox), prefix/substring match, interactive disambiguation via inquire
- [ ] **Config file auth** — `~/.config/fastermail/config.toml` with 0600 perms
- [ ] **README.md** — 30-second install/configure/use guide for regular FastMail users
- [ ] **Verify Phase 2 JMAP** — Run curl against session endpoint

### Later
- [ ] **Phase 2 spec rewrite**: CardDAV/CalDAV → JMAP (pending verification)
- [ ] **Test infrastructure**: mock JMAP, per-action unit tests, integration tests (big gap)
- [ ] **Dockerfile**: Multi-stage build for distribution
- [ ] **CI/release**: Cross-compilation for 4 targets (x86_64/aarch64 × linux/macos)
- [ ] **Phase 2 implementation**: Contacts + calendars tools
- [ ] **Raw output mode**: True JMAP response pass-through (currently same as Json)

## Key Decisions Made

- Binary name: `fm` (short), MCP mode via `fm mcp`
- CLI structure: resource-first then verb (`fm emails list`)
- CLI focus: email organization/triage, not composition
- Mailbox aliases: built-in role aliases + fuzzy name matching (not yet implemented)
- UX libs: indicatif + inquire + console (match ACE patterns)
- Output: human-friendly default, `--json` for scripting, `--raw` for debug
- Auto-detect: non-TTY stdout → JSON mode
- Auth: env var takes precedence over config file
- No default command → show help (not MCP server)
- Fix code for usability over raw JMAP compliance
- Response projection: filter to spec-defined fields
- Default mailbox: `emails list` defaults to inbox when `--mailbox` omitted
