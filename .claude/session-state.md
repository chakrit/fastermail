# FasterMail Session State

Saved: 2026-03-25 (session 12)

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

9. ✅ **Mailbox resolution** — role aliases, fuzzy name matching, interactive disambiguation
   - New `src/cli/resolve.rs` with `resolve_mailbox()` function
   - Resolution: role alias → exact name → prefix → substring (case-insensitive)
   - Multiple matches: `inquire::Select` in Human mode, error with candidates in Json/Raw
   - Wired into `emails list`, `search --mailbox`, `move --to`
   - 8 unit tests for matching logic
   - MCP path unchanged (still uses action's `mailbox_name` field)

10. ✅ **Config file auth** — `~/.config/fastermail/config.toml` with 0600 perms
   - `4b89853` Add config file auth
   - New `src/config.rs`: resolve_token (env > config file), write_config, 0600 perms check
   - `fm config` — prints config path, token source, masked token (human + JSON)
   - `fm setup` — interactive inquire prompt, writes config, verifies connection
   - Updated `connect()` to use `config::resolve_token()`
   - 5 unit tests (parsing, path, permissions)

11. ✅ **README.md** — install/configure/usage guide for FastMail users
   - `ffdc8c3` Add README: install, configure, usage guide for FastMail users
   - Sections: install (cargo), configure (fm setup + env var), usage (shortcuts + full commands), output formats, MCP server
   - 122 lines, scannable 30-second quickstart

12. ✅ **Dotenv support** — `.env` + `.env.local` loading via dotenvy
   - `9b6b7b2` Add .env/.env.local support via dotenvy
   - Loads .env then .env.local at startup (local overrides base)
   - `.env.local` gitignored, `.env` tracked for dev defaults

13. ✅ **Communication prefs in CLAUDE.md** — ported from ACE project
   - `ecdf710` Add communication prefs to CLAUDE.md, update session state
   - Sections: Communication Style, Workflow (edit protocol, never-assume), Metrics, Response Completion
   - User also wants school `general-coding` skill updated with .env convention (see pending)

14. ✅ **Replace dotenvy with manual loader** — dropped dotenvy crate, wrote `load_dotenv()` in main.rs
   - `350aa84` Replace dotenvy with manual .env loader

## TODO — Not Started

### Immediate (this sprint)
- [ ] **Verify Phase 2 JMAP** — Run curl against session endpoint (now unblocked)
- [ ] **Update school `general-coding` skill** — add .env convention: `.env` committed with
      non-sensitive defaults, `.env.local` gitignored for real credentials, no `.env.example` pattern

### Later
- [ ] **Local caching layer** — cache mailbox lists, identities, etc. to avoid repeated JMAP calls
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
- Mailbox aliases: built-in role aliases + fuzzy name matching
- UX libs: indicatif + inquire + console (match ACE patterns)
- Output: human-friendly default, `--json` for scripting, `--raw` for debug
- Auto-detect: non-TTY stdout → JSON mode
- Auth: env var takes precedence over config file
- No default command → show help (not MCP server)
- Fix code for usability over raw JMAP compliance
- Response projection: filter to spec-defined fields
- Default mailbox: `emails list` defaults to inbox when `--mailbox` omitted

## Communication Issues (user flagged)

**Problem:** Claude has been acting without confirming first — reading files, starting edits,
and diving into debugging without asking. The Way of Work says "confirm plan → get approval
before starting" but Claude keeps skipping step 2.

**User instruction:** "We'll tackle your communication problem first." This is the top
priority for the next session — fix the behavior before doing any code work.

**Rules to follow strictly:**
1. State what you plan to do
2. STOP and wait for explicit "yes" / "go" / confirmation
3. Only then act
4. Do NOT read files, explore code, or start debugging as part of "planning" — that IS work
