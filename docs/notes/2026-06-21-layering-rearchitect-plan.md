# Layering rearchitect — audit + plan (2026-06-21)

**Status: STEP 1 DONE; STEP 2 Email DONE (both forks landed 2026-06-25, committed not
pushed — see the "fork A LANDED" section at the bottom); STEP 2 Identity DONE (migrated
2026-06-25, committed not pushed — see "Step 2 / Identity — MIGRATED" at the bottom); STEP 2
Vacation DONE (migrated 2026-06-25, committed not pushed — see "Step 2 / Vacation —
MIGRATED" at the bottom); STEP 2 Mailbox DONE (migrated 2026-06-25, committed not pushed —
see "Step 2 / Mailbox — MIGRATED" at the bottom); next: propagate the pattern to
masked_email, then contact.** The
three locks were adopted under chakrit's AFK delegation and **confirmed by chakrit** ("all
good", 2026-06-22). Shipped + **pushed to `gh/main`**: step 1 (lib/bin split, `f049938`);
`email_state` bootstrap primitive (`af96f9d`); faithful L1 `email_get` + typed `Email`
(`a8cb9d5`, the zero-loss read shape — types newtype ids, flattens the rest); spec sync
(`754a233`); dead_code cleanup (`0fc652e`).

**Docs pass done (2026-06-22):** README → `docs/guides/`, new everyday/scripting/backup
guides, notes synced. Standalone CLI work that landed alongside (not part of the
rearchitect): `resolve_mailbox` raw-id passthrough (`90321b4`); a resumable whole-account
backup script (`8fac74a`, shellcheck-/general-coding-audited in `a205f71`). **Pushed to
`gh/main` through `b7b4ef7`** (chakrit pushed mid-session); the audit fix `a205f71` is the
only substantive commit still local. The full backup is chakrit-triggered
(`scripts/backup-mail.sh`); the smoke test left 5 messages + start cursor `J1071504` in
gitignored `mail/`, resume-safe. **Next: implement step 2** with the now-confirmed forks
below.

**Two design forks — CONFIRMED by chakrit (2026-06-22)** (both define the per-resource
pattern; lock 1 settled the read *shape* via `email_get`, but NOT these):

- **(A) Read-projection relocation. → CONFIRMED: the REC.** Route the existing
  list/get/search through `email_get` and move projection — `extract_body_content`
  (part-ref→string, synth `date`, drop `bodyValues`), the property pin, the MCP
  token-trim — out of `actions/email.rs` into the CLI + MCP presenters (each owns
  its L3). Add a small
  body-fetch option to `email_get` (`Email/get` needs
  `fetchTextBodyValues`/`fetchHTMLBodyValues`/`fetchAllBodyValues`, args beyond
  `properties`); keep CLI/MCP output **byte-identical** (CLI renders table/body from
  `Email`; MCP keeps today's trimmed shape).
- **(B) Typed mutation API shape. → CONFIRMED: option (a).** Faithful L1
  `email_set(create, update, destroy) -> EmailSetResponse`, consistent with
  `email_get`/`email_query`. Higher-level typed ops (`email_move`, `email_set_keywords`,
  `email_destroy`) come later as sugar; not re-exporting `actions` as-is.

Once decided: finish step 2 on Email, propagate to the other five resources, then steps 3–4.

Companions: design rulings in
`../decisions/2026-06-21-jmap-library-and-backup-primitives.md`; the
backup-primitives build log in `2026-06-21-afk-implementation-plan.md`.

## Where we are (shipped + pushed to `gh/main`)

- Backup primitives, slices 1–3: pagination + `fm emails list/search --all`
  (`b37a8d4`); incremental `Email/changes` + `fm emails changes` (`85e85f9`); raw
  `.eml` blob download + `fm emails export` (`658f5e5`); checkpoints `50ce20d`,
  `9edd295`.
- Live read-only verification vs chakritw@fastmail.fm (`8a046ce`): `id`-sort
  tiebreak **accepted**; multi-window anchor stitch correct (Berlitz, 15 msgs / 8
  windows at temp page_size=2, no dupes, ascending); `export` byte-exact RFC822
  with attachments inline; `Email/changes` maps `cannotCalculateChanges`. Tree
  clean, 162 tests green, clippy/fmt clean.

## Diagnosis: there is no data layer (rearchitect warranted)

Every `actions/*` struct fuses three layers — it makes the raw JMAP call (L0),
projects + reshapes the result (L3 work), and returns MCP-wire JSON (L3 work). The
MCP handler looks "thin" only because the actions are pre-shaped *for* MCP — which
is exactly why a lib / backup / faithful-CLI consumer cannot reach the real data.
Same pattern across all 6 resources × ~14 actions. Confirmed by a 5-way module
audit (contact; mailbox+identity; masked_email+vacation; mcp; cli) plus direct
read of email/jmap/actions-mod.

## Audit map — concern → today | belongs

| Concern                                       | Today                          | Belongs                         |
|-----------------------------------------------|--------------------------------|---------------------------------|
| Raw JMAP call                                 | inline `call_one` per action   | **L1** typed accessor (lib)     |
| Field selection (`*_FIELDS`, `project_fields`)| data path                      | **L3** (CLI/MCP presenter)      |
| Reshaping (`extract_body_content`, flatten)   | data path                      | **L3**                          |
| MCP wrappers (`{success:true}`, `{xId}`)      | action return values           | **L3** (MCP)                    |
| `tools()` schemas                             | `actions/` (shared)            | **MCP** layer                   |
| Invented nouns (`Flag`, `FieldChange`, flat `Contact`) | actions               | L3 / rename to JMAP             |
| `resolve_mailbox`, input parsers              | CLI                            | **L1/sugar** (shared)           |
| Table rendering / `str_at` digging            | CLI                            | L3 (right spot; fragile on `Value`) |

## Field loss (the symptom that started this)

- **Email** ~12+: `cc`, `bcc`, `sender`, `sentAt`, `size`, `threadId`,
  `mailboxIds`, `keywords`, `hasAttachment`, `messageId`/`references`/`inReplyTo`,
  `headers`, `blobId`, `bodyStructure`.
- **Mailbox** 5: `sortOrder`, `totalThreads`, `unreadThreads`, `myRights`,
  `isSubscribed`.
- **Identity** 4: `bcc`, `textSignature`, `htmlSignature`, `mayDelete`.
- **MaskedEmail**: create returns only `id`+`email` (drops `state`, `forDomain`,
  `description`, `createdAt`) — asymmetric vs list.
- **Contact** ~20 JSContact fields: addresses, birthday(s), anniversary(ies),
  online, url(s), categories, uid, created, updated, kind, gender, jobTitle/titles,
  roles, pronouns, members, related, preferredLanguages, prodId — plus
  all-but-first organization.
- **AddressBook**: projected to `id`/`name`/`description`/`isDefault`.

Per-resource notes:
- `email.rs`: `email_get_args` pins 6 props; `extract_body_content` rewrites
  text/htmlBody→strings, synthesizes `date`, deletes `bodyValues`. The new L1 read
  primitives (`email_query`/`email_changes`/`email_blob_id`/`download_blob`) are
  proper L1, but the `get` step still projects.
- `contact.rs`: `From<WireCard>` flattens JSContact → lossy flat `Contact`;
  `ContactContext`/`ContactEmail`/`ContactPhone` are invented shapes; `tools()`
  schemas live here.
- `mailbox`/`identity`/`masked_email`/`vacation`: `*_FIELDS` consts +
  `project_fields` in the data path; `{success:true}`/`{xId}` MCP wrappers;
  `FieldChange` (vacation) invents a noun; no L1 accessors (direct `call_one`).
- CLI: ~40 fragile `str_at` render sites (input is `Value`, not a type); business
  logic stranded — `resolve_mailbox` (+`ROLE_ALIASES`), `parse_typed_values`
  (contacts), `MaskedEmailState::parse`. `io.rs` rendering + projection ARE
  correctly L3 (don't move those).

## Target architecture

- **L0** — `JmapClient::call`/`call_one`. Keep.
- **L1 (lib core)** — typed faithful accessors, **reads and writes**, JMAP names
  1:1, **all fields** via `#[serde(flatten)] rest: Map<String, Value>` (nothing
  dropped, incl. unmodeled / future FastMail fields). Newtype ids. No projection,
  no reshaping.
- **Sugar (lib)** — `EmailEnumerator`, changes feed, and multi-step *operations*
  (send = create+submit, delete-to-trash, …) returning **typed** values. This is
  where today's `actions` move, stripped of projection + JSON-shaping.
- **L2 (lib, later)** — `mail-parser` MIME view. Deferred (not needed for backup;
  raw `.eml` is lossless).
- **L3 (bin CLI + MCP, each owns its own)** — projection, the flattened Contact
  view, rendering, the `tools()` schemas, token-economy trimming, `resolve_mailbox`
  + prompts.
- `lib.rs` exports L0/L1/sugar/types; `fm` bin + MCP become thin L3 callers.

## Three locks — ADOPTED & CONFIRMED (chakrit, 2026-06-22)

1. **Read shape** → **typed struct + `#[serde(flatten)] rest: Map`** (the REC: types
   where they help, zero loss by default). Rejected raw `Value` passthrough.
2. **Actions → typed lib operations** returning concrete types; CLI + MCP render.
   chakrit already chose typed L1 mutations up front (the write half); adopted the read
   half to match.
3. **Migration style** → **strangler**: resource-by-resource, green commit each, app
   works throughout (the REC). Already in force (steps 1 + email_state landed green).

Rationale (the locks were adopted autonomously during the AFK run, then confirmed by
chakrit "all good" 2026-06-22): each had a standing recommendation, lock 2 was already
chakrit's call, and the brief delegated decisions. Read-shape work was deliberately capped
at Email (`email_get`); the projection-relocation + mutation *pattern* still awaits the two
forks in the status block before propagating to the other five resources.

## Path (strangler; green at every step)

0. Lock the three above.
1. **Stand up `src/lib.rs`** — move `jmap`/`error`/`json`/`logging`/`recorder`
   under the lib; bin depends on it; re-home the `#[macro_use] logging` macros and
   `#[cfg(test)] testutil`. Pure move, no behavior change. (Closes the
   lib-packaging item.)
2. **Per resource, Email first** — faithful L1 reads (all fields) + typed writes;
   operation returns typed; projection/reshape move into the CLI + MCP presenters.
   One commit per resource. ← **field-expansion + typed-mutations land here.**
3. Move `tools()` schemas to MCP; move `resolve_mailbox` + input parsers into lib
   sugar.
4. Delete the dead projection machinery (`project_fields*`) and the
   `Value`-returning `Action` trait once all resources are migrated.

Start at step 1 (small, unblocks everything, zero behavior change), then use Email
as the pattern slice for step 2. Multi-session effort, touches nearly every file.

## Subsumed by this plan

- **`email_state`** (incremental bootstrap: `Email/get ids:[]` → response `state`)
  = one L1 read accessor. **DONE** (`af96f9d`, standalone — not folded into step 2).
  Without it incremental can't start (`--since 0` → `cannotCalculateChanges`, verified live).
- **"Expose all JMAP fields"** (chakrit) = the faithful L1 reads in step 2.
- **The decision doc's open naming question** (Flag vs `keyword`, `FieldChange`,
  flat `Contact`) collapses into steps 2–3: invented nouns move to L3 or take JMAP
  names. No longer "forward-only deferred" — the rearchitect addresses it.

## Minor carry-forwards

- httpmock 0.8 `body_includes` silently fails to match substrings containing `:` —
  paginated-window mocks key on the colon-free quoted anchor value (see
  `MockJmap::handle_method_matching`). Candidate to promote into `rust-coding` via
  ace-school (not done).
- **Sharing a test harness across a lib/bin split** (generic Rust): a lib's
  `#[cfg(test)]` items can't cross into the bin crate's tests. Pattern used here —
  gate the harness behind a `testutil` feature (optional `httpmock` dep), then enable
  it for `cargo test` via a *self dev-dependency* (`fastermail = { path = ".",
  features = ["testutil"] }`); release builds never pull it. Documented in
  `../spec/testing.md`. Candidate to promote into `rust-coding` via ace-school (not done).
- ~~`--mailbox` help claims it accepts an "ID", but `resolve_mailbox` only handles
  role aliases + names; a raw id errors.~~ **DONE (`90321b4`)** — added a Step-0 exact
  id match; surfaced by the backup script needing to reach two folders both named
  "Crypto".
- `shellcheck -o all` flags SC2292 ("prefer `[[ ]]` over `[ ]`"), which contradicts
  `general-coding`'s POSIX-`sh` target. Use the default ruleset and reject SC2292.
  Candidate one-line caveat for general-coding's Shell section via ace-school (not done).

## Step 2 / Email — fork A LANDED (2026-06-25, committed, NOT pushed)

Read-projection relocation complete. Both step-2 Email forks (A read + B write) are
now done; Email is the finished pattern slice for the other five resources.

Commits (on `main`, ahead of `gh/main` — awaiting push):
- `889f6a1` — presenter golden tests + `Io` capture seam (the byte-identity net,
  landed FIRST against the still-projecting code, then kept green through the move).
- `2e88daa` — L1 `email_get` body-fetch via `BodyFetch { text, html, all }` (maps to
  `fetchTextBodyValues`/`fetchHTMLBodyValues`/`fetchAllBodyValues`; `default()` = no
  flags = prior behaviour). Lone non-test caller `email_blob_id` + 2 tests updated.
- `5602f75` — the relocation: reads route through L1 `email_query`/`EmailEnumerator`
  + `email_get` and return FAITHFUL `Email` data; projection moved to the presenter.

**Where the shared presenter-projection helper lives: `src/present.rs`** (new bin-side
L3 module, `mod present;` in `main.rs`). It owns: the view property lists
(`EMAIL_LIST_PROPERTIES` / `EMAIL_LIST_BODY_PROPERTIES` / `EMAIL_BODY_PROPERTIES` +
`email_list_properties`/`email_list_body_fetch` selectors) and the projection
(`project_email_list` / `project_email_body`, wrapping `extract_body_content` +
`resolve_body_part` moved verbatim out of `actions/email.rs`). Both front-ends call it:
CLI in `format_email_list`/`format_email_body` (project a clone before JSON-emit /
human-render); MCP in `dispatch_tool`'s `get_emails`/`search_emails`/`get_email_body`
arms (project before the handler `to_string_pretty`s the value).

Byte-identity: held. `serde_json::Value` has no `preserve_order`, so it is
BTreeMap-backed and serializes keys alphabetically regardless of insertion/wire order —
the raw-`Value` path and the typed-`Email` round-trip produce identical bytes (verified
empirically). The golden tests assert exact `to_string_pretty` bytes (MCP `text` payload
+ CLI `--json` capture), so a future reorder (e.g. if `preserve_order` were enabled)
would fail them. The list/search path is now two L1 calls (`email_query` then
`email_get`) instead of one back-referenced batch; output unchanged.

Test net seam: `Io::capturing(mode) -> (Io, Arc<Mutex<Vec<u8>>>)` captures `data`
output into a buffer (the `Sink::Buffer` variant is `#[cfg(test)]`, never in release).

**Remains: propagate this pattern to the other five resources** —
mailbox / identity / masked_email / vacation / contact. Each still fuses L0+L3 in its
`actions/*` (direct `call_one`, `*_FIELDS` + `project_fields` in the data path,
`{success:true}`/`{xId}` MCP wrappers, invented nouns like `FieldChange` / flat
`Contact`). For each: stand up faithful L1 accessors, return faithful data from the
action, add per-resource projection to `src/present.rs`, applied in CLI + MCP behind a
golden net captured first. Then steps 3–4 (move `tools()` schemas to MCP, move
`resolve_mailbox` + parsers to lib sugar, delete `project_fields*` + the
`Value`-returning `Action` trait once all resources migrate).

## Phase A audit (2026-06-25) — step-2 Email batch (`6eca4aa^..HEAD`)

Audited `6eca4aa`..`5602f75` (`src/`): the `email_set` accessor + `EmailSetResponse`,
move/delete/flag rewired onto it, `email_get` `BodyFetch`, and the read-projection
relocation to `src/present.rs` + the `Io` capture seam. **Verdict: clean batch.** Layering,
correctness, and byte-identity all hold; the golden tests genuinely pin exact bytes for both
presenters (body + list, parsed `Value` + raw string). No violations. Findings below are
borderline / cleanup, ranked; two stale-doc fixes already applied inline this audit.

Ranked fix-slices:

1. **[borderline] Action depends on the bin-side L3 `present` module.**
   `actions/email.rs:57-58` (`fetch_emails_by_ids`) calls `present::email_list_properties`
   / `present::email_list_body_fetch`; `:379` (`GetEmailBody`) reads
   `present::EMAIL_BODY_PROPERTIES`. The action — slated to become lib sugar — imports the
   view's property contract from L3. Intended per the plan ("present.rs owns the JMAP
   `properties` an action should request"), and harmless while actions are still `[bin]`,
   but it inverts the eventual lib→bin dependency: once actions move to the lib, they can't
   import a bin module. **Fix (slice, when actions migrate to lib):** the property/body-fetch
   contract is a *caller* concern — have the CLI/MCP pass the `properties` + `BodyFetch`
   *into* the action (or the L1 accessor) rather than the action reaching up into `present`.
   Until then, leave as-is.

2. **[borderline, justified] `EmailSetResponse::check_errors` duplicates the free
   `check_set_errors`.** `jmap/email.rs:139` (typed) vs `actions/mod.rs:57` (raw `Value`).
   Same notCreated→notUpdated→notDestroyed scan, first-description-wins. Justified: the typed
   one serves the migrated `email_set` path (move/delete/flag); the raw one still serves
   `SendEmail`, whose 2-method `Email/set`+`EmailSubmission/set` batch goes through `call`
   (loose `Value`s), not `email_set`. **Fix (slice):** migrate `SendEmail`'s draft-create
   onto `email_set` (or add a typed `EmailSubmission/set` accessor) and delete the raw
   `check_set_errors` once no raw `/set` caller remains. Folds into propagating the pattern.

3. **[out-of-scope, test gap] No golden test for the no-body list path.**
   `present::project_email_list` runs `extract_body_content` (synth `date`, drop
   `bodyValues`) even when `include_body=false`; both golden list tests
   (`cli/emails.rs:682`, `mcp/handler.rs:666`) use `includeBody=true`. A regression in the
   no-body projection (e.g. `date` synthesis) wouldn't be caught by a presenter golden.
   **Fix (slice):** add a `includeBody=false` golden to both presenters pinning the
   `date`-synthesized, body-less shape.

4. **[out-of-scope, test gap] `BodyFormat::Html` body-fetch flag not pinned end-to-end.**
   `email_get_emits_body_fetch_flag` (jmap/email.rs:714) pins only `all`; no test asserts
   `format: html` sends `fetchHTMLBodyValues` (and not text) through `GetEmailBody`. The
   `BodyFormat::body_fetch` mapping itself is untested. **Fix (slice):** add a unit test on
   `BodyFormat::{Text,Html,Both}::body_fetch()` (cheap, no mock).

5. **[DONE inline this audit] `docs/spec/architecture.md` stale post-relocation.** The file
   tree omitted `src/present.rs`, the `actions/` line still claimed "+ projection"
   unqualified, and the narrative said "projection currently still lives in `actions/`".
   Fixed: added the `present.rs` tree entry, qualified the `actions/` projection note as
   pre-Email-migration, and updated the narrative to record Email's projection now living in
   L3 `present.rs` (other five still in `actions/`). Reference tool docs
   (`get_emails.md`/`search_emails.md`: `Email/query → Email/get`) remain accurate.

Wire-change note (not a finding): the list/search read is now **two separate** round-trips
(`email_query` then `email_get`) rather than the prior single batched `Email/query` +
back-referenced `Email/get`. Faithful + intended (each L1 accessor is one call); output
byte-identical; covered by `mock_query_then_get` (two distinct method mocks). The
`jmap.md:25-35` back-reference example is a general JMAP illustration, still valid as such.

## Phase B audit (2026-06-25) — module-graph architecture

Scope: the whole module graph (`jmap/`, `actions/`, `cli/`, `mcp/`, `present.rs`,
`lib.rs`, `main.rs`, `testutil/`), broader than the Email diff. **Verdict: the graph is
clean of violations.** Email is a faithful pattern slice (L1 accessors + typed sets + an
L3 presenter owning projection, golden-pinned both front-ends). The remaining work is
*propagation*, not repair: the other five resources still fuse L0+L3 in `actions/*`,
exactly as the diagnosis predicted. **Verify gate green at audit start** (`cargo test`
178 unit + 27 doctests, `cargo clippy --all-targets`, `cargo fmt --check` all pass on exit
code). **No inline fixes applied** — every finding below is a multi-file slice that must go
through a golden-net-first migration, none qualifies as a low-risk inline cleanup.

### Per-resource layering map (loose `Value` → faithful type | projection → L3)

The five unmigrated resources, by ascending migration cost — this IS the recommended order:

| # | Resource     | L0 call site (today)                       | What's loose `Value`            | Projection to move to L3                     | Invented noun / wrapper        | Byte-identical risk |
|---|--------------|--------------------------------------------|----------------------------------|----------------------------------------------|--------------------------------|---------------------|
| 1 | **identity** | `call_one Identity/get` (`identity.rs:22`) | whole list                       | `LIST_FIELDS` (id,name,email,replyTo)        | none                           | low — list only, 1 tool, no `{success}` wrapper |
| 2 | **vacation** | `call_one VacationResponse/{get,set}`      | singleton get + update patch     | `GET_FIELDS` (6 fields)                       | `FieldChange` (Leave/Clear/Set) | low — singleton; `{success:true}` on set |
| 3 | **mailbox**  | `call_one Mailbox/{get,set}` (`mailbox.rs`)| list + create/rename/delete      | `LIST_FIELDS` (6) + role filter              | `{success:true}`/`{mailboxId}` | medium — feeds `resolve_mailbox`; role filter must stay |
| 4 | **masked_email** | `call_one MaskedEmail/{get,set}`       | list + created object            | `LIST_FIELDS` (6) + **asymmetric** `CREATE_FIELDS` (id,email) + state filter | `{success:true}` | medium — create projection drops state/forDomain/desc/createdAt (field-loss item) |
| 5 | **contact**  | `call Contact{Card/query→get}` + `ContactCard/set` | JSContact ContactCard          | `From<WireCard>` flatten + `AB_LIST_FIELDS`  | flat `Contact`, `ContactContext`, `ContactEmail`/`Phone`, `{contactId}`/`{success}` | **high** — the lossy JSContact flatten is the most invasive projection; ~20 JSContact fields dropped |

L1 accessors to stand up (JMAP names 1:1, `#[serde(flatten)] rest`, newtype ids):
`identity_get` → `Identity` + `IdentityId`; `vacation_get`/`vacation_set` → `VacationResponse`
(singleton); `mailbox_get`/`mailbox_set` → `Mailbox` + `MailboxId` + `MailboxSetResponse`;
`masked_email_get`/`masked_email_set` → `MaskedEmail` + `MaskedEmailId`; `contact_query`/
`contact_get`/`contact_set` + `address_book_get` → `ContactCard` (faithful JSContact) +
`ContactId`/`AddressBookId`. Each returns faithful data; the action returns it verbatim;
projection moves to a per-resource `present.rs` function applied in CLI + MCP behind a
golden captured FIRST.

### Propagation roadmap (the next rounds of slices)

**Order: identity → vacation → mailbox → masked_email → contact** (simplest/highest-leverage
first; contact last because its JSContact flatten is the deepest projection and carries the
most byte-identical risk). Rationale: identity/vacation are single-tool / singleton with
trivial projection and prove the shared scaffolding cheaply; mailbox unblocks moving
`resolve_mailbox` to lib sugar (step 3); masked_email's asymmetric create projection is a
self-contained field-loss fix; contact is the big one, done once the pattern is hardened.

**Shared L3/present scaffolding to factor out FIRST (one slice, before resource #1)** so it
isn't re-invented per resource:
- **`present::project_list(value, fields)`** — the generic list/object field-selection
  helper. This is `project_fields`/`project_fields_array` relocated from `actions/mod.rs`
  into `present.rs` as L3 (where field selection belongs per the audit map). identity,
  vacation, mailbox, masked_email all project by a static `&[&str]` — they share this one
  helper; only contact needs a bespoke flatten. The relocation is the deletion path for
  `project_fields*` from the data layer.
- **`present::set_success(id_field, id)` / `present::set_ok()`** — the `{success:true}` /
  `{xId: ...}` MCP-wrapper shapes, owned by L3 instead of returned from the action. Both
  front-ends call them after the typed `*_set` accessor returns.
- **Per-resource golden helpers** mirroring `mcp/handler.rs` `tool_call_text` + the CLI
  `Io::capturing` seam — already exist; reuse, don't duplicate. Each resource gets a CLI
  `--json` golden + an MCP `text` golden, captured against the still-projecting code, kept
  green through the move (the proven Email recipe).

**Where the lib/bin dep-inversion (#1 below) resolves in the sequence:** at **step 3**, when
the actions move from `[bin]` into lib sugar. The fix is to invert the property/body-fetch
contract — the CLI/MCP caller passes `properties` + `BodyFetch` *into* the L1 accessor,
rather than the action reaching up into `present`. Do this as the *first* lib-sugar slice
(it's Email-only today: `actions/email.rs:57-58,379`), before any resource's action moves to
lib, so the inversion never compounds across resources.

### Ranked architecture findings (all forward-looking slices; none inline)

1. **[high leverage] Five resources fuse L0+L3 — propagate the Email pattern.** Per the map
   above. Each is its own slice (golden-first → L1 accessor → faithful action → L3 present
   fn). This is the bulk of the remaining rearchitect. Order: identity, vacation, mailbox,
   masked_email, contact.

2. **[do first] Factor the shared L3 scaffolding before resource #1.** `present::project_list`
   (relocate `project_fields*`), `present::set_success`/`set_ok`. Prevents five copies of the
   wrapper/projection logic. Small slice, unblocks the rest.

3. **[medium] Lib/bin dep inversion (carried from Phase A #1).** `actions/email.rs` imports
   the bin-side `present` module for its property/body-fetch contract. Harmless while actions
   are `[bin]`; blocks the lib move. Resolve as the first step-3 lib-sugar slice by passing
   `properties`/`BodyFetch` into the accessor from the caller. Sequence it *before* any
   resource action migrates to lib.

4. **[medium] `find_mailbox_id_by_{role,name}` (actions/mod.rs) partially duplicate
   `resolve.rs`'s `find_by_role`/`find_by_id`/`match_by_name`.** Two mailbox-lookup
   implementations: the action helpers (exact role/name, used by `email.rs` send/delete and
   `GetEmails` name-resolve) and the richer CLI resolver (id → role → exact → prefix →
   substring + disambiguation). When `resolve_mailbox` moves to lib sugar (step 3), the action
   helpers should collapse into it — the CLI resolver is the superset. Until then, both are
   live (not dead). Folds into step 3.

5. **[low, cleanup-when-unblocked] `project_fields*` + raw `check_set_errors` delete path.**
   `project_fields`/`project_fields_array` (`actions/mod.rs:27-51`) are used by all five
   unmigrated resources — **NOT dead yet**, deletable only after all five migrate (then their
   logic lives in `present::project_list`). The raw `check_set_errors` (`actions/mod.rs:57`)
   serves contact + `SendEmail`'s 2-method `Email/set`+`EmailSubmission/set` batch (which goes
   through `call`, not `email_set`); deletable once contact migrates to a typed `*SetResponse`
   and `SendEmail`'s draft-create moves onto `email_set`/a typed `EmailSubmission/set`
   accessor (Phase A #2). The `Action` trait + its `Value` return type retire with the last
   resource (path step 4).

6. **[low, test gap — carried from Phase A #3/#4] Presenter seam coverage holes.**
   (a) No `includeBody=false` golden: `project_email_list` still runs `extract_body_content`
   (synth `date`, drop `bodyValues`) when bodies aren't requested; both list goldens use
   `includeBody=true`. (b) `BodyFormat::{Text,Html,Both}::body_fetch()` mapping untested — no
   test asserts `format:html` sends `fetchHTMLBodyValues` (only `all` is pinned, jmap/email.rs).
   Cheap slices: add a body-less list golden to both front-ends; add a unit test on
   `BodyFormat::body_fetch` (no mock). Fold into the next Email-adjacent slice.

7. **[note, not a finding] `Io::json` Raw == Json, by stale rationale.** `io.rs:171-176`
   comments "Raw currently outputs the same as Json since actions already project fields;
   true raw JMAP pass-through is future work." Post-Email that rationale is now only true for
   the five unmigrated resources; Email actions return faithful data and the presenter
   projects. Once all resources migrate, `--raw` *could* emit the faithful pre-projection
   `Value` (genuinely raw JMAP) — a real feature the rearchitect unlocks. Track as a
   post-migration enhancement; update the comment when contact lands.

### Module-boundary / simplification read (no findings)

- Import graph is acyclic and sane: `lib` (L0/L1/sugar) ← `bin` (`actions`/`cli`/`mcp`/
  `present`). The only upward edge is the documented dep-inversion (#3). `present.rs` is
  correctly bin-side L3; `jmap/` is lib L1; `mcp/handler.rs` + `cli/*` are thin callers.
- One responsibility per module holds. No over-engineering spotted — the typed
  `EmailSetResponse`/`BodyFetch`/`Page` and the `EmailEnumerator` sugar each earn their
  place; nothing to simplify away.
- `MockJmap` harness is healthy (`handle_method` / `handle_method_matching` /
  `handle_download`, 121 lines). The httpmock-`:`-substring caveat is already documented
  (minor carry-forward). No oversized test module needing a split; the biggest
  (`actions/email.rs` tests ~600 lines) tracks the biggest resource and is well-factored.

## Step 2 / Identity — MIGRATED (2026-06-25, committed, NOT pushed)

First of the five-resource propagation. Identity now mirrors the Email shape; **vacation
is next** (the order from the Phase B roadmap: identity → vacation → mailbox →
masked_email → contact).

Commits (on `main`, ahead of `gh/main` — awaiting push):
- `21a5e86` — presenter golden tests (the byte-identity net, captured FIRST against the
  still-projecting code, kept green through the move): MCP
  `golden_list_identities_projects_fields` (via `handle_tools_call` → `tool_call_text`)
  and CLI `golden_list_json_projects_fields` (via `Io::capturing(Json)`). Both pin exact
  `to_string_pretty` bytes; fixtures carry the dropped fields to prove they stay projected
  out.
- `aa8e54a` — `present::project_object` + `present::project_list` (the shared L3
  scaffolding, see below).
- `0d3ffa8` — the migration: faithful L1 `identity_get` + typed `Identity`/`IdentityId`/
  `IdentityGetResponse` (`src/jmap/identity.rs`, `pub mod identity;` in `jmap/mod.rs`);
  `ListIdentities` returns faithful data; projection moved to
  `present::project_identity_list`, applied in the MCP `list_identities` arm and the CLI
  `identities list` command.

**No `identity_set`:** identity has no mutation action (only `list_identities` /
`ListIdentities`). `Identity/set` accessor deferred until a mutation is actually needed.

**Shared L3 `present::` scaffolding now factored (the next slices reuse this):**
- `present::project_object(value, fields) -> Value` — single object → only the selected
  keys, in `fields` order; non-object passes through.
- `present::project_list(value, fields) -> Value` — array → project each element; a single
  object → project it; otherwise passthrough. **This is the generic helper vacation /
  mailbox / masked_email reuse** (each projects by a static `&[&str]`; only contact needs
  a bespoke JSContact flatten). Identity's view is `present::IDENTITY_LIST_FIELDS`
  (`[id,name,email,replyTo]`) + the thin `project_identity_list` wrapper — copy that
  shape per resource.

**How `project_fields*` was handled — DUPLICATED-PENDING (not relocated).** The plan
offered relocating `actions::project_fields`/`project_fields_array` into `present.rs`, but
they are **still called by the four unmigrated resources** (vacation/mailbox/masked_email
direct `project_fields_array`; contact via its own path). Relocating now would break them.
So `present::project_list`/`project_object` are the L3 replacements **added fresh** in
`present.rs`; the `actions/mod.rs` copies **stay in place** until vacation/mailbox/
masked_email migrate onto `present::project_list`. **Deletion path:** once those three (and
contact) no longer call `actions::project_fields*`, delete them from `actions/mod.rs`
(plan path step 4). Until then, two implementations coexist by design — the L3 one for
migrated resources, the data-layer one for the rest.

Byte-identity: held (the goldens stay green). `serde_json::Value` is BTreeMap-backed (no
`preserve_order`) so the faithful-`Identity` round-trip serializes keys alphabetically,
identical to the prior raw-`Value` projection — same as the Email finding.

Verify gate green: `cargo test` (185 unit across lib+bin + 27 doctests), `cargo clippy
--all-targets`, `cargo fmt --check` (exit 0) all pass.

## Loop checkpoint — 2026-06-25 (keep-going run)

Codified the "keep going" working agreement (`5825268`: CLAUDE.md `## Working Agreement` +
`docs/guides/keep-going.md`) and ran it end-to-end as a thin-orchestrator loop. Landed +
pushed to `gh/main`:
- Fork B: L1 `email_set` + move/delete/flag routed onto it (`6eca4aa`, `89f3cdc`).
- Fork A: read-projection relocated to `src/present.rs` (`889f6a1`..`7ef7b42`).
- Two-phase audit, both clean: Phase A `8f1d3d8`, Phase B + propagation roadmap `0658b4b`.
- Resource 1/5: identity migrated (`21a5e86`..`8866957`).

Process finding (caught by orchestrator re-verification): two slices split a `present::`
helper from its first wiring across commits — the helper-only commit (`aa8e54a`; fork A
similarly) does NOT compile under `#![deny(warnings)]` (dead_code), violating
green-at-every-step. HEAD stayed green throughout; only intermediate bisectability was hit.
Hardened the convention in `keep-going.md` (each commit independently passes the gate);
already-pushed history not rewritten.

Remaining propagation (roadmap order): **vacation → mailbox → masked_email → contact**. Each
copies the identity shape (a `present::<RES>_LIST_FIELDS` const + a thin
`project_<res>_list` wrapper over the generic `present::project_list`); golden-tests-first,
byte-identical. **contact is HELD for attended review** — highest byte-identical risk
(JSContact flatten, ~20 dropped fields). After all five migrate: delete the
`actions::project_fields*` copies (path step 4), then steps 3–4.

Loop paused at this checkpoint — resumes on "keep going".

## Step 2 / Vacation — MIGRATED (2026-06-25, committed, NOT pushed)

Resource 2/5. Vacation now mirrors the Email/Identity shape; **mailbox is next** (roadmap
order: ~~identity~~ → ~~vacation~~ → mailbox → masked_email → contact).

Commits (on `main`, ahead of `gh/main` — awaiting push):
- `787fa9e` — presenter golden tests (byte-identity net, captured FIRST against the
  still-projecting code, kept green through the move). Both get AND set, both front-ends:
  MCP `golden_get_vacation_projects_fields` / `golden_set_vacation_returns_success` (via
  `handle_tools_call` → `tool_call_text`); CLI `golden_get_json_projects_fields` /
  `golden_set_json_returns_success` (via `Io::capturing(Json)`). Each pins exact
  `to_string_pretty` bytes; the get fixture carries `id`/`htmlBody` to prove they stay
  projected out. This commit builds (tests against existing code).
- `99deada` — the migration: faithful L1 `vacation_get`/`vacation_set` + typed
  `VacationResponse`/`VacationResponseId`/`VacationGetResponse`/`VacationSetResponse`
  (`src/jmap/vacation.rs`, `pub mod vacation;` in `jmap/mod.rs`). Actions return faithful
  data; projection moved to `present::project_vacation`, applied in the MCP/CLI get arms;
  the set arms emit `present::set_ok()`.

**No singleton-id loss:** the singleton is JMAP id `"singleton"`; the get view projects it
out (`VACATION_FIELDS` = the 6 settable fields, the prior `GET_FIELDS`), matching the old
output exactly. The faithful `VacationResponse` keeps `id` + every field in `rest`.

**How `FieldChange` (the invented noun) was resolved — MOVED TO L3, not renamed.** JMAP has
no `FieldChange` concept; it expresses the three intents directly in the `update` patch
(omit the key = leave, write `null` = clear, write a value = set). So `FieldChange`
(Leave/Clear/Set) is L3 *input-parsing*: it translates optional CLI/MCP args into a JMAP
patch. Relocated the enum + the patch builder out of the data layer (`actions/vacation.rs`)
into `present.rs` as `present::FieldChange` + `present::build_vacation_update`. The L1
`vacation_set(account_id, update: Value)` is a pure faithful pass-through (mirrors
`email_set`'s update half) — no shaping in the data layer. The action assembles the patch
via the present helper, calls `vacation_set`, and surfaces SetErrors via
`VacationSetResponse::check_errors`.

**New shared L3 helper:** `present::set_ok() -> {"success": true}` — the `{success:true}`
MCP-wrapper shape, now emitted by both front-ends after the typed set returns rather than
returned from the action (per the Phase B "factor the shared scaffolding" finding). The
remaining unmigrated set actions (mailbox/masked_email) reuse this next.

Set output is now an L3 wrapper end-to-end: the action returns the faithful `updated` map;
CLI/MCP both emit `set_ok()`. Byte-identical to the prior `{success:true}` (goldens green).

Verify gate green: `cargo test` (33 lib + 163 bin + doctests), `cargo clippy
--all-targets`, `cargo fmt --check` (exit 0) all pass. Each commit independently builds
under `#![deny(warnings)]` (golden commit = tests-against-existing-code; migration commit =
helper + first use together). One pre-existing parallel-httpmock flake observed once in 7
runs (the documented shared-substring carry-forward) — not introduced here; stable
single-threaded and across repeated runs.

`actions::project_fields*` still live (mailbox/masked_email/contact use them); delete path
unchanged (after all five migrate).

## Step 2 / Mailbox — MIGRATED (2026-06-25, committed, NOT pushed)

Resource 3/5. Mailbox now mirrors the Email/Identity/Vacation shape; **masked_email is
next** (roadmap order: ~~identity~~ → ~~vacation~~ → ~~mailbox~~ → masked_email →
contact).

Commits (on `main`, ahead of `gh/main` — awaiting push):
- `0579b14` — presenter golden tests (byte-identity net, captured FIRST against the
  still-projecting code, kept green through the move). List AND manage, both front-ends:
  MCP `golden_list_mailboxes_projects_fields` / `golden_list_mailboxes_filters_by_role` /
  `golden_manage_mailbox_{create,rename,delete}_returns_id` (via `handle_tools_call` →
  `tool_call_text`); CLI `golden_list_json_projects_fields` /
  `golden_list_json_filters_by_role` / `golden_create_json_returns_id` /
  `golden_{rename,delete}_json_returns_success` (via `Io::capturing(Json)`). Each pins
  exact `to_string_pretty` bytes; list fixtures carry the 5 dropped fields
  (sortOrder/totalThreads/unreadThreads/myRights/isSubscribed) to prove they stay
  projected out. This commit builds (tests against existing code) — independently
  re-verified in a throwaway worktree.
- `6d286f2` — the migration: faithful L1 `mailbox_get`/`mailbox_set` + typed
  `Mailbox`/`MailboxId`/`MailboxGetResponse`/`MailboxSetResponse` (`src/jmap/mailbox.rs`,
  `pub mod mailbox;`). `mailbox_get` mirrors `identity_get`; `mailbox_set` mirrors
  `email_set` (create/update/destroy in one call); `MailboxSetResponse::check_errors`
  mirrors `EmailSetResponse`. Actions return faithful data; projection + role filter +
  wrappers moved to L3.

**Role filter moved to L3.** `ListMailboxes` is now a unit struct returning the faithful,
unfiltered list; `present::project_mailbox_list(value, role)` owns both the field
selection (`MAILBOX_LIST_FIELDS`, reusing the generic `project_list`) and the role filter.
The two action-level role-filter tests were relocated to `present`
(`project_mailbox_list_filters_by_role`) + the goldens; the remaining action list test now
asserts faithful (unprojected) data (`sortOrder` present).

**The `{success, mailboxId}` wrapper → L3 via `present::set_with_id(id_field, id)`** (new
shared helper, alongside `set_ok`). `ManageMailbox::run` returns the faithful
`Mailbox/set` response (no wrapper); `ManageMailbox::resolved_id(&value)` digs the
affected id (created id from the response, or the input id for rename/delete). The
front-ends wrap: MCP create/rename/delete all emit `{success, mailboxId}`; CLI create
emits it, CLI rename/delete emit bare `{success}` via `set_ok()` — **both prior shapes
preserved exactly** (the asymmetry is original behavior, pinned by the goldens).
`MailboxSetResponse` gained `Serialize` (faithful round-trip) so the action can return it
for `resolved_id` to read.

**resolve_mailbox now on L1.** `cli/resolve.rs` calls `ctx.jmap.mailbox_get(...)` directly
(was `Mailbox/get` via `ListMailboxes`), maps the faithful `Mailbox` list to `Value`s, and
runs the same find-by-id/role/name resolution unchanged — its 13 tests stay green, output
behavior-identical. **Dedup against `actions::find_mailbox_id_by_{role,name}` is DEFERRED
to step 3** (the CLI resolver is the superset; collapse the action helpers into it when
`resolve_mailbox` moves to lib sugar — Phase B finding #4).

Byte-identity: held (all 20 goldens green through the move). `serde_json::Value` is
BTreeMap-backed (no `preserve_order`), so the faithful-`Mailbox` round-trip serializes
keys alphabetically, identical to the prior raw-`Value` projection — same finding as
Email/Identity/Vacation.

Verify gate green: `cargo test` (38 lib + 174 bin + doctests), `cargo clippy
--all-targets`, `cargo fmt --check` (exit 0) all pass. **Each commit independently builds
under `#![deny(warnings)]`** — golden commit re-verified standalone in a throwaway
worktree (33 lib + 173 bin green, clippy/fmt clean); migration commit lands helper + first
use
together.

`actions::project_fields*` still live (masked_email/contact use them); delete path
unchanged (after all five migrate).
