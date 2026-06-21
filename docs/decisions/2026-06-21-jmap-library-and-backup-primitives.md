# JMAP library architecture & backup primitives

Date: 2026-06-21 · Status: accepted (design; implementation pending)

## Context

`notes.claude` (a sibling agent, consumer at `notes/src/mail`) wants full Fastmail
backup — ~67k messages, 33 folders, with attachments, incremental. `fm` is
operate-only today and structurally can't back up (see
`../notes/2026-06-19-backup-capability-feature-request.md`). Decision: **do not
build `fm backup`.** Instead add primitives so any crate/CLI consumer implements
backup itself. Walked 1-by-1 with chakrit.

## Two guiding philosophies (apply to the whole API surface)

1. **Transparent translation.** fastermail adds no email-processing value — it's a
   pass-through between the user and Fastmail/JMAP, "a necessary evil." So names
   mirror JMAP / the FastMail API *exactly*: method and field names, `keyword`
   (not "flag"), `anchor` / `position` / `queryState`, `state` / `created` /
   `updated` / `destroyed`, `blobId` / `downloadUrl`. No fastermail-invented noun
   where JMAP already has one — terms stay searchable across JMAP docs.
2. **Idiomatic Rust sugar is a separate, OPTIONAL layer.** Rust-native ergonomics
   (iterators, newtypes, builders) sit *on top of* the direct JMAP accessors and
   never replace them. Inspired by good crates (paginators). Sync (`ureq`), not
   async.

## Layered design

- **L0 transport** — existing `call` / `call_one` (raw JSON method calls).
- **L1 direct JMAP accessors (faithful)** — typed per-resource calls, JMAP names
  1:1, JMAP-shaped responses (`ids`, `queryState`, `position`, `total`;
  `created`/`updated`/`destroyed`, `newState`, `hasMoreChanges`). Newtype ids
  (`EmailId`, `MailboxId`, `BlobId`, `State`, `QueryState`).
- **L2 (MIME, backup only)** — wrap `mail-parser` to expose a decoded best body +
  an attachment list (name / type / disposition / bytes / content-id). **fm stops
  here.**
- **Sugar (optional, separate module)** — a generic anchor enumerator and a
  changes feed as sync `Iterator`s, composed over L1. Capability traits
  (`Queryable`, `Changeable`) make it one generic impl over JMAP's uniform method
  classes; the type system then mirrors per-type capability (singletons like
  `VacationResponse` simply don't `impl Queryable`).
- **L3 (filenames, dedup, on-disk layout, rendering)** — NOT fm's. Each consumer
  (CLI, MCP, `notes/src/mail`) owns its own L3, because the three front-ends have
  genuinely different L3 needs (terminal vs AI-JSON vs git-crypt'd Maildir).

## Packaging

**Both** — fastermail becomes `lib` + `bin`. The lib holds the real API; the `fm`
bin and the MCP handler are thin callers; external consumers depend on the lib.

The lib's "real API" includes **mutations**, not just the read primitives
(chakrit, 2026-06-21). Reads are typed L1 accessors on `JmapClient` today;
mutations currently live only in `actions/` as JSON-returning `Action` structs.
Exposing them as typed L1 (`email_set`/`mailbox_set`/…) vs re-exporting `actions`
as-is is an open fork — see the plan note's lib roadmap item.

## Specific rulings (and the rejected alternative)

- **Backup format: raw `.eml`.** Lossless, byte-exact, attachments inline via the
  message `blobId` downloaded through `downloadUrl`. Rejected as the *primary*:
  parsed + extracted-attachments — lossy, N+1 downloads, not reconstructable.
  Extraction is an optional *consumer* L3 view, never fm's.
- **MIME parser: `mail-parser` (Stalwart).** `#![forbid(unsafe_code)]`,
  fuzzed + MIRI, org-backed, and its native body+attachment model *is* our L2
  shape. Accepted costs: a build-time proc-macro chain (`hashify` → `syn`/`indexmap`;
  compile cost, not binary) and an `Option`-not-`Result` parse API (a failure
  surfaces as a generic "unparseable" — no detail on the MCP path). Runner-up:
  `mailparse` (leaner deps, faster compile, `Result` errors, but we'd write the
  MIME-walk ourselves). Rejected: `email-parser` (21 `unsafe`, incl. unchecked on
  parser input; abandoned 2021), `eml-parser` (drags the full `regex` chain for a
  weaker line-scanner). All evaluated crates had 0 RUSTSEC advisories.
- **Pagination: anchor-based** (`Email/query` `anchor` + `anchorOffset`) — JMAP's
  own skip-proof mechanism; `position`/offset is the fragile one. Default sort
  `receivedAt` ascending + id tiebreak (immutable; new mail appends at the end).
  Dedup by id on collect.
- **Incremental: JMAP `state` + `Email/changes`** (`created`/`updated`/`destroyed`,
  `hasMoreChanges`); on `cannotCalculateChanges`, fall back to a full
  re-enumeration. Backup *policy* (destroyed = mirror vs retain, state-file
  location, full-vs-incremental scheduling) is the consumer's, not fm's.

## Genericization timing

Start **concrete** (Email), then extract the `Queryable`/`Changeable` traits +
generic `enumerate<R>` on second use (Mailbox). Not premature — "JMAP Queryable
resource" is a spec-grounded abstraction, not a guess.

## Open (deliberately not decided)

- **Existing-naming alignment vs forward-only.** This session's `Flag` (JMAP says
  *keyword*), the invented `FieldChange`, and the flattened `Contact` L3 view all
  conflict with philosophy #1. Decide later: rename existing to JMAP terms + move
  `Contact`-flatten to CLI-only, **or** apply the philosophy forward-only to new
  code. An AFK run must NOT auto-rewrite committed code for this — leave it for
  chakrit.
- **Slice-1 landing** — (A) bin-internal vs (B) lib-first; A recommended. See the
  plan note.
- Pagination *scope* (general `--all` on operate commands vs primitive-only) and
  the exact CLI/MCP command names — build-time defaults.

> 🤖 Drafted by Claude on chakrit's behalf.
