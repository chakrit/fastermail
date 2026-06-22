# Layering rearchitect — audit + plan (2026-06-21)

**Status: STEP 1 DONE; STEP 2 PAUSED at two forks (as of 2026-06-22).** The three locks
were adopted under chakrit's AFK delegation and **confirmed by chakrit** ("all good",
2026-06-22). Shipped + **pushed to `gh/main`**: step 1 (lib/bin split, `f049938`);
`email_state` bootstrap primitive (`af96f9d`); faithful L1 `email_get` + typed `Email`
(`a8cb9d5`, the zero-loss read shape — types newtype ids, flattens the rest); spec sync
(`754a233`); dead_code cleanup (`0fc652e`).

**Next session: a docs pass first** (chakrit's call), then resume the rearchitect at the
two forks below.

**Two design forks that need chakrit before resuming step 2** (both define the
per-resource pattern; lock 1 settled the read *shape* via `email_get`, but NOT these):

- **(A) Read-projection relocation.** Route the existing list/get/search through `email_get`
  and move projection — `extract_body_content` (part-ref→string, synth `date`, drop
  `bodyValues`), the property pin, the MCP token-trim — out of `actions/email.rs` into the
  CLI + MCP presenters (each owns its L3). Forces: how to model body-value fetching in L1
  (`Email/get` needs `fetchTextBodyValues`/`fetchHTMLBodyValues`/`fetchAllBodyValues`, args
  beyond `properties`). **REC:** add a small body-fetch option to `email_get`; keep CLI/MCP
  output byte-identical (CLI renders table/body from `Email`; MCP keeps today's trimmed
  shape).
- **(B) Typed mutation API shape** (the decision doc's open fork). (a) faithful L1
  `email_set(create, update, destroy) -> EmailSetResponse`; (b) higher-level typed ops
  (`email_move`, `email_set_keywords`, `email_destroy`); (c) re-export `actions` as-is.
  **REC:** (a), consistent with `email_get`/`email_query`; add sugar ops later.

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
- `--mailbox` help claims it accepts an "ID", but `resolve_mailbox` only handles
  role aliases + names; a raw id errors. Pre-existing. Fix when the CLI is touched
  in step 2/3.
