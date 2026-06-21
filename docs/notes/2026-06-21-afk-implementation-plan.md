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

## Progress (AFK run, 2026-06-21)
- **Slice 1 DONE** — Email anchor pagination. `src/jmap/email.rs`: `EmailId`,
  `Page`, `EmailQueryResponse`, `JmapClient::email_query` (L1); `EmailEnumerator`
  iterator (sugar). Landing **A'**: `fm emails list/search --all` (+ `fm ls --all`)
  enumerate then batch `Email/get`. Default limited path kept its one-shot
  query→get back-reference (chose NOT to reroute it through `email_query` — that
  would add a round-trip to the common case for no gain; the enumerator is
  `email_query`'s caller instead). MCP stays limited (`all: false`). Commit
  `b37a8d4`.
- **⚠ VERIFY ON LIVE API**: the enumerator sorts `receivedAt` asc **+ `id`
  tiebreak**, per the decision doc. RFC 8621 does NOT list `id` as a sortable
  `Email/query` property — FastMail may reject it. MockJmap can't catch this. If
  live rejects, drop the `id` tiebreak and lean on the existing id-dedup (small
  skip risk on identical-`receivedAt` collisions). One-line fix in
  `EmailEnumerator::new`.
- **Test caveat captured in code**: httpmock 0.8 `body_includes` silently fails on
  substrings containing `:`; paginated-window mocks key on the colon-free quoted
  anchor value. See `MockJmap::handle_method_matching` doc.

## Roadmap remaining (one at a time)
- **Generalize → `Queryable` + generic `enumerate<R>`** — DEFERRED until a 2nd
  consumer (Mailbox) actually needs paging; premature now (33 mailboxes fit one
  page). Per decision-doc "genericization timing."
- **Incremental — `Email/changes` + state** — NEXT. Dep-free L1 accessor
  (`oldState`/`newState`/`hasMoreChanges`/`created`/`updated`/`destroyed`) +
  optional changes iterator; caller = a new `fm emails changes --since <state>`
  (CLI names are build-time defaults per the doc).
- **Blob download → raw `.eml`** — `Email/get` raw-MIME `blobId` → download via
  session `downloadUrl`. L1 raw bytes need no new dep; `mail-parser` (L2 parsed
  view) is a separate optional layer fm stops at.
- **`lib` target / public API** — formalize once 1–2 more primitives exist.

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
