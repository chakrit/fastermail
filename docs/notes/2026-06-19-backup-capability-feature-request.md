# Backup capability — feature request (from notes.claude)

**RESOLVED (2026-06-22).** Shipped as **primitives, not a monolithic `fm backup`**. The
approach is the
[JMAP-library decision](../decisions/2026-06-21-jmap-library-and-backup-primitives.md).
All four missing pieces below now exist as CLI/lib accessors: blob/attachment download +
raw RFC822 (`fm emails export`), incremental sync (`fm emails changes`), and pagination
(`fm emails list/search --all`). The "full local backup" goal is delivered by
`scripts/backup-mail.sh` (`8fac74a`), which composes them — see
[`../guides/backup.md`](../guides/backup.md). The `fm backup --to <dir>` surface in
"Desired output shape" was **superseded** by this primitives-first approach; the consumer
(`notes` repo `src/mail/`) wraps the primitives or the script instead.

---

Captured from `chakrit.notes.claude` over the ace-connect bridge on 2026-06-19.
The consumer is the `notes` repo's `src/mail/`, which will wrap `fm backup`
(Maildir/.eml into a git-crypt'd tree). Until this exists, backup stays on
Fastmail's native export.

## Goal
Full local backup of Fastmail into the notes repo at `src/mail/`, git-crypt'd.
~67k messages, 33 folders. MUST include attachments. Incremental going forward
(not a full re-pull each run).

## Why this is a fastermail change
`fm` is operate-only today and structurally can't back up. Four missing
primitives:

1. **Blob / attachment download** — call JMAP `Blob/get` / `downloadUrl`; `fm`
   never touches it today (`--has-attachment` is only a search *filter*, not a
   fetch). Need: download + write attachment bytes.
2. **Raw MIME / RFC822** — `--raw` only dumps the JMAP JSON response (parsed
   `bodyValues`), not the original message. Need true `.eml` export (`Email/get`
   with the raw-MIME blobId, then download that blob). Raw MIME carries
   attachments inline.
3. **Incremental sync** — JMAP has `Email/changes` + state tokens; `fm` exposes
   no command. Need: persist per-mailbox JMAP state, pull only changes since
   last sync (added / updated / destroyed).
4. **Pagination cursor** — output is capped by `-n <limit>` with no way to page
   the full set. Need `Email/query` position/anchor paging so nothing is
   silently truncated at 67k scale.

## Desired output shape
- Maildir (one immutable file per message) or `<folder>/YYYY/MM/<message-id>.eml`,
  attachments inline via raw MIME.
- A small per-mailbox state file for incremental.
- Suggested surface: `fm backup --to <dir> [--full | --since-state] [-m <mb>]`.

## Coordination
chakrit drives this build in the fastermail session directly. notes.claude is
the consumer. This note captures the request; the build decisions (scope, order,
surface) are being walked separately.

> 🤖 Captured by Claude from chakrit's notes agent, on chakrit's behalf.
