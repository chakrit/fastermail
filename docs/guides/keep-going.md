# Keep Going — Slice Loop & Audit Cadence

The standing autonomous workflow for **"keep going"**. Self-contained: the audit
passes are written here as procedures — follow this guide. The orchestrator is a
thin re-spawner; every slice and every audit runs in a fresh subagent. The
orchestrator only does cheap done-checks and re-spawns. Adapted from
`chakrit/kue`'s slice-loop for fastermail's stack.

## Standing autonomy grant (chakrit, 2026-06-25)

- **Autonomy.** Decide and proceed without the propose-then-wait gate, as long as
  work advances faithful JMAP semantics and the layered design. Pick the next
  slice, plan it, implement it. Ask only when genuinely blocked. While the loop is
  active this overrides the per-edit propose-then-wait gate in `CLAUDE.md`.
- **Resolve forks by philosophy, don't ask.** Default to the most faithful,
  JMAP-mirroring, layered option (L1 faithful + sugar/L3 on top; byte-identical
  presenter output). The Design philosophy already decides most forks; apply it
  and note the choice in the spec/note, not in a question. Surface a fork only
  when philosophy is genuinely silent **and** the choice is expensive to reverse.
- **Commit + push freely (attended).** Commit AND push on the current branch
  (`main` included) without asking, as part of advancing work — this repo
  overrides the global push-waits default (chakrit, 2026-06-25). Releases are
  attended via `scripts/release.sh` (no CI). **Don't pause at milestones** — a
  finished goal or clean checkpoint is not a reason to stop and ask "what next";
  resolve by philosophy and keep the loop driving. Push/release are
  attended-only: in AFK mode (below), commit but do NOT push.
- **Go fast.** Subagents, batched/parallel calls, concurrent edits — whatever
  genuinely speeds the work. Parallelize only when it actually helps.
- **Keep the durable record current as a restore point.** `docs/spec/` (source of
  truth), `docs/decisions/`, `docs/notes/` (breadcrumb), and `README.md` /
  `docs/reference/` (the user-facing contracts) are the crash-safe memory. A slice
  isn't done until its spec/note/log entry is written — a crash or `/clear` then
  leaves a clean restore point.
- **Two bindings autonomy never overrides:** no working-tree-overwriting git
  (`checkout` / `restore` / `reset --hard` without asking), and no environment
  mutation outside the project tree (global installs, `~/.config`, shell rc,
  package managers — sandbox or ask).

## Design philosophy the audits enforce

fastermail is a **transparent translation layer** (see `CLAUDE.md`). Every audit
checks this first, in priority order:

1. **Mirror JMAP / Fastmail vocabulary exactly.** JMAP method/field names 1:1; no
   invented noun where JMAP has one. Newtype ids (`EmailId`, `BlobId`, `State`) so
   distinct domains can't be swapped.
2. **Hold the layering.** L0 transport (`call` / `call_one`) → L1 faithful typed
   accessors (all fields via `#[serde(flatten)] rest`, no projection) → sugar
   (enumerators, multi-step typed operations) → L3 presenters (CLI render, MCP
   shape, token economy). Projection lives in L3, never in the data layer;
   presenters each own their L3.
3. **Sugar is optional, on top — never replacing the faithful accessors.** A lib /
   backup / CLI consumer must always be able to reach the raw, lossless data.

## The continuous slice loop ("keep going")

"keep going" / "keep it going" / "carry on" re-enters this loop from any fresh
session — new machine, new clone, after `/clear` — with no setup. A bare "go" /
"continue" is an ordinary nudge, not the trigger.

You are a thin orchestrator, not the implementer:

1. **Auto-compact on** so the loop survives long runs; accumulate only slice
   summaries.
2. **Re-orient from the durable record** — the latest `docs/notes/…` breadcrumb,
   the active plan note, the decisions. Trust them over conversation.
3. **Spawn one subagent per slice.** It runs the full ace workflow in fresh
   context (plan → TDD → verify → commit/push → update the plan note +
   breadcrumb). See **Slice** below.
4. **Cheaply verify, don't re-do.** Confirm the slice landed — tree clean, pushed
   (`HEAD == @{u}`), the verify gate green, breadcrumb updated — with a light
   check. No full skill sweep; the subagent owns depth.
5. **Two-phase audit every 2–3 slices.** Spawn audit subagents — Phase A then B,
   **sequentially** (both edit the plan note; parallel would collide). Fold
   findings into the plan note as ranked fix-slices; fix-slices count as
   implementation slices next round. Don't let audits stall forward motion, don't
   skip them.
6. **Loop or stop.** Verify passes + slices remain → spawn the next. Stop only at
   a genuine blocker, a failed verify the subagent couldn't fix, or an empty plan
   — leaving the breadcrumb pointing at the next step.

No manual `/ace-save` or `/clear` between slices — the subagent boundary gives
fresh context, the breadcrumb gives continuity.

## Slice (per subagent)

Full workflow in fresh context: plan → TDD → implement → **verify** → commit/push
to `gh:main` → update the plan note + breadcrumb. The verify gate:

```
cargo test && cargo clippy --all-targets && cargo fmt --check
```

Gate `cargo fmt --check` on its **exit code**, not piped output — a piped
`--check` exits 0 even with a diff (`cargo fmt` to fix, then re-check).
`#![deny(warnings)]` means any warning already fails the build.

**Durable per-slice conventions (copy into every slice / audit prompt):**

- **Byte-identical CLI/MCP output is a hard gate.** Both CLI `--json` / raw
  (`io.json`) and MCP (`handle_tools_call` → `to_string_pretty`) emit the action
  `Value` **verbatim**. When relocating projection between layers, add presenter
  golden tests **first** (capture the current output), then move code, keep them
  green. The presenter layer is thinly tested — never trust action-level tests to
  catch a presenter regression. Watch `serde_json` field order on any typed
  round-trip.
- **Claims about live Fastmail/JMAP behavior are EMPIRICAL.** Verify against the
  live API or `MockJmap`, never assert from memory — capabilities are
  token-scoped, and the JMAP spec / live behavior is the authority (NEVER assume —
  verify). An honest "unconfirmed" beats a confident wrong answer.
- **Confirm the push, don't assert it.** Before reporting "pushed", check
  `git push` shows `main -> main` (or `HEAD == @{u}`). A "pushed" claim without it
  is unverified.
- **Commit at checkpoints, not only at the end.** A subagent that crashes or hits
  a transient API error loses ALL uncommitted work. Commit at natural green seams;
  a few checkpoint commits beat total loss. Recover from GIT STATE, never memory
  (`git rev-parse HEAD` vs `@{u}` + `git status --porcelain`). Treat transient API
  errors / rate-limit returns as retry-NOW, never wait-it-out.
- **Strangler — green at every step.** One green commit per slice; the app builds
  and the tests pass throughout. The test suite + the byte-identical gate are the
  oracle.
- **Each commit independently passes the gate.** Don't split a helper from its first
  caller across commits — a checkpoint that adds an unused fn/const trips
  `#![deny(warnings)]` dead_code, leaving a non-building commit (breaks `git bisect` +
  crash recovery). Land a helper in the same commit as its first use. Observed twice
  (fork A, identity): a subagent committed `present::` helpers one commit before wiring.
- **Minimal deps.** Don't add a crate where std or an existing dep does — fast
  compiles are a standing constraint.

## Phase A — Code-quality audit (the batch since the last audit)

Scope: the slices landed since the previous audit. Check:

- **Correctness** — behavior matches JMAP / Fastmail; edge + error cases handled,
  not just the happy path. Byte-identical output preserved where a refactor claims
  to preserve it.
- **Layering (check FIRST)** — no projection or JSON-shaping leaking into L1;
  faithful accessors stay faithful; presenters own their L3; sugar doesn't replace
  the raw accessors. An invented noun where JMAP has a name → finding.
- **Rust idioms / skill compliance** — `rust-coding` + `general-coding` (hard
  blockers); `Result` / `Option` idioms; newtypes over loose strings for distinct
  id domains; naming and readability.
- **DRY / reuse** — no duplicated logic that should share a helper (e.g. the four
  raw `Email/set` sites consolidated onto `email_set`).
- **Test strength** — `MockJmap` tests pin edges and errors, not smoke; new
  behavior has a test that would fail without it.
- **Spec accuracy** — `docs/spec/`, the plan note, and the reference contracts
  match the code.

Output: fold findings into the plan note as ranked fix-slices. Apply only LOW-RISK
fixes inline; if you do, re-run the full verify gate and commit.

## Phase B — Architecture / refactor / cleanup audit (the module graph)

Scope: cross-cutting design, broader than the recent diff. Check:

- **Layering integrity (top-level concern).** Across `jmap/` / `actions/` /
  `cli/` / `mcp/`, where are projection, field selection, or MCP-shaping sitting
  in the wrong layer? Where could a loose `Value` be a faithful type or a newtype?
  Push toward L0/L1/sugar/L3 with each concern in its home (see the rearchitect
  plan note).
- **Module boundaries** — import edges sane, no cycles, one responsibility per
  module; the lib/bin split clean (the lib holds the API; the `fm` bin + MCP are
  thin L3 callers).
- **Refactor / cleanup** — dead code (e.g. `project_fields*` once a resource is
  migrated), duplication across modules, functions in the wrong place.
- **Simplification** — complexity that can be removed; over-engineering.
- **Test/fixture health** — coverage gaps at the seams (especially the
  thinly-tested presenter layer); oversized test modules; `MockJmap` harness debt.

Output: fold findings into the plan note as ranked architecture fix-slices; large
refactors become their own planned slices. Apply only low-risk cleanups inline
(re-verify + commit).

## Unattended mode (AFK / nightshift)

Triggered by `/ace-afk`, "afk", "keep going while I'm gone", "overnight",
"nightshift". Run the slice loop, but replace every propose/confirm gate with a
hard safety **envelope** — stay strictly inside it:

- **No global-state mutation** (already standing).
- **No outward-facing or irreversible actions** — no `push`, publish, release,
  deploy, mail, or destructive API calls. `push` is the canonical "needs a human"
  act.
- **No working-tree destruction** (already standing).
- **Commit, don't push.** Land green slices on the current branch; pushing waits
  for a human. Overrides the attended push grant above.
- **Don't block — log it.** When work needs a human (ambiguous spec, unsafe
  judgment call, envelope boundary), append a blocker to `.afk.log` at repo root —
  *what* (task + where it stopped), *why it needs a human*, *what you'd do* (so a
  one-word reply unblocks it) — then pick up the next unblocked slice. Never stall
  on one item.
- **Stop** when out of unblocked work or token budget; write a run summary to
  `.afk.log`.

Full skill: `ace-afk` in the school.

## Releases (attended, local only — CI/GitHub Actions banned)

`scripts/release.sh` builds and publishes via `gh`; no cadence, no CI, ever.
Requires a clean tree (commit first). Cut from current `main` HEAD.

## Notes

- The orchestrator's only between-step job is the cheap done-check (git state +
  one verify run), never the deep work. `HEAD == @{u}` is the only "pushed"; a
  subagent's "pushed" claim is never trusted on its word.
- **Independently re-verify high-stakes claims.** Byte-identical drop-in, push,
  and release claims the orchestrator re-runs DIRECTLY (re-run the verify gate,
  re-check `@{u}`) before they enter the durable record. A high-stakes subagent
  claim is a hypothesis, not a fact.
- **Mid-run editor diagnostics are stale — verify at HEAD.** While a slice subagent
  runs, the LSP/rustc diagnostics snapshot its in-flight edits (a helper added a
  commit before its wiring, etc.) and surface `dead_code`/`E0603`/`E0277` that are
  NOT true of the committed HEAD. Seen 5× in one run; HEAD built clean every time.
  The authoritative check is `cargo` at HEAD, never the editor diagnostic.
- **"User-gated" is a high bar.** Re-examine an audit's "needs-the-user" verdict
  by philosophy before deferring — most resolve autonomously. Surface a fork only
  when philosophy is genuinely silent AND the choice is expensive to reverse.
  Default: resolve-and-proceed.
</content>
