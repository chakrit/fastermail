# Layering rearchitect — audit + plan (2026-06-21)

**Status: PROPOSED — three locks pending chakrit before any code.**
To resume: `/ace` → read this note → confirm the three locks (below) → start at
step 1 of the path. chakrit asked to work the *entire path* in a new session.

Companions: design rulings in
`../decisions/2026-06-21-jmap-library-and-backup-primitives.md`; the
backup-primitives build log in `2026-06-21-afk-implementation-plan.md`.

## Where we are (shipped; all on `main`, UNPUSHED — push is chakrit-gated)

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

## Three locks (decide before code; my recommendations)

1. **Read shape** — typed struct + `#[serde(flatten)] rest: Map` (**REC**: types
   where they help, zero loss by default) vs raw `Value` passthrough (max
   transparency, less ergonomic).
2. **Actions → typed lib operations** returning `impl Serialize`/concrete types;
   CLI + MCP render. (Core move — confirm.) chakrit already chose **typed L1
   mutations up front**, which is the write half of this.
3. **Migration style** — strangler: resource-by-resource, green commit each, app
   works throughout (**REC**) vs big-bang.

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
  = one L1 read accessor, lands in step 2's Email slice. Without it incremental
  can't start (`--since 0` → `cannotCalculateChanges`, verified live).
- **"Expose all JMAP fields"** (chakrit) = the faithful L1 reads in step 2.
- **The decision doc's open naming question** (Flag vs `keyword`, `FieldChange`,
  flat `Contact`) collapses into steps 2–3: invented nouns move to L3 or take JMAP
  names. No longer "forward-only deferred" — the rearchitect addresses it.

## Minor carry-forwards

- httpmock 0.8 `body_includes` silently fails to match substrings containing `:` —
  paginated-window mocks key on the colon-free quoted anchor value (see
  `MockJmap::handle_method_matching`). Candidate to promote into `rust-coding` via
  ace-school (not done).
- `--mailbox` help claims it accepts an "ID", but `resolve_mailbox` only handles
  role aliases + names; a raw id errors. Pre-existing. Fix when the CLI is touched
  in step 2/3.
