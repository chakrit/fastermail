# AFK implementation plan + session resume (2026-06-21)

## Resume state
- **16 commits** this session on `main` (`2fbd0b1..HEAD`), all green
  (build / test / clippy, `#![deny(warnings)]`), tree clean. **UNPUSHED** — push
  is chakrit-gated: `git push gh main`. (Prior AFK handoff: `.afk.log`, gitignored.)
- This session: full audit + readability/typed-enum refactors + the contacts &
  vacation full splits + a repo-wide rustfmt; then a 1-by-1 design walk on backup
  primitives → `../decisions/2026-06-21-jmap-library-and-backup-primitives.md`.

## Next session: `/ace-afk` — implement all planned stuff (per chakrit)
Build the planned primitives **one slice at a time**, each gated by
build + test + clippy, committed at seams. **Do not push.** Honor the two
philosophies in the decision doc (mirror JMAP names; sugar is a separate optional
layer). Do NOT auto-rewrite committed code for the naming-alignment open question.

### Slice 1 (first) — Email anchor pagination (concrete, no generic traits)
- `EmailId` newtype.
- **Base:** `JmapClient::email_query(filter, sort, page) -> EmailQueryResponse`
  with JMAP-faithful fields (`ids`, `query_state`, `position`, `total`); `Page`
  enum `{ Position(u64), Anchor { id: EmailId, offset: i64 } }`. Pull the raw
  `Email/query` out of today's `query_and_fetch`.
- **Sugar:** `EmailEnumerator` — sync `Iterator<Item = Result<EmailId>>`, anchor
  paging (anchor = last id, offset 1), default sort `receivedAt` asc + id tiebreak,
  terminates on a short final page.
- **Tests (MockJmap):** multi-window stitch, termination, single page, mid-stream
  error propagation, id-dedup on overlap.
- **Landing — RESOLVE A/B before building** (A recommended): (A) bin-internal —
  route `GetEmails`/`SearchEmails` through `email_query` and add an `--all` path
  using the enumerator (closes the `-n` truncation gap); (B) lib-first — stand up
  `src/lib.rs` and export it as the first public API.
- Note: paging splits the one-shot `Email/query`→`Email/get` back-reference into
  per-window `query` then `get`.

### Roadmap after slice 1 (one at a time)
1. Generalize → `Queryable` trait + generic `enumerate<R>` once Mailbox needs it.
2. `Changeable` + `changes_since` iterator (incremental).
3. `Blob` download → raw `.eml` + `mail-parser` L2 utils (body + attachments).
4. `lib` target / public API surface (the "both") — formalize once 1–2 primitives
   exist to shape it.

### AFK guardrails
- `#![deny(warnings)]`: new code can't be dead — wire each primitive to a caller
  (or expose via the lib) in the same slice.
- Don't push. Don't auto-rewrite committed code for the naming-alignment question.
- Log blockers to `.afk.log`; pick up the next unblocked slice.
