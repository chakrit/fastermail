---
name: general-coding
description: Language-agnostic coding standards. Load for any coding task in any language.
---

# General Coding

## Code Grouping (Typography for Code)

Apply print typography principles to code layout. Code is read far more than written — visual
structure should communicate intent before the reader parses any syntax.

- **Proximity** — statements that work together belong together. Group related lines with no
  blank lines between them. The absence of space signals "these are one thought."
- **Paragraph breaks** — separate groups with a single blank line. Each group is one logical
  step: setup, transformation, result. Two blank lines is too many inside a function.
- **Chunking** — aim for groups of 3-5 lines. A wall of 20 ungrouped lines is harder to read
  than four groups of five. Scattering every line with blank lines destroys grouping.
- **Rhythm** — alternate between density (grouped logic) and space (blank line separators).
  The reader's eye should skim group boundaries and understand flow at a glance.
- **Method structure** — ideal functions follow a three-act pattern: (1) preconditions —
  validate inputs, fail fast, (2) do work — core logic, (3) postconditions — verify results,
  return. Each act is its own group separated by blank lines.
- **Consistency** — apply the same grouping logic across the codebase. Similar functions
  should follow the same cadence.

```
# bad — wall of ungrouped lines
def process_order(order):
    if not order.items:
        raise ValueError("empty order")
    subtotal = sum(item.price for item in order.items)
    tax = subtotal * order.tax_rate
    total = subtotal + tax
    receipt = format_receipt(order, total)
    send_email(order.customer, receipt)
    return total

# good — three-act structure with visual grouping
def process_order(order):
    if not order.items:
        raise ValueError("empty order")

    subtotal = sum(item.price for item in order.items)
    tax = subtotal * order.tax_rate
    total = subtotal + tax

    receipt = format_receipt(order, total)
    send_email(order.customer, receipt)
    return total
```

## Coding Style

Naming:
- **Names must be unambiguous at the common callsite** — consider what the reader sees where
  the name is actually used. If the language or convention qualifies names with their
  namespace (`auth.Token`), don't repeat context already visible (`auth.AuthToken`). If names
  are typically imported bare, include enough context to be unambiguous on their own.

Clarity:
- Clarity over compression — prefer named variables for each branch over long chained
  expressions.
- When there are multiple possible sources for a value, compute each into a named variable
  first, then combine:
  ```
  # bad — reader must parse the whole chain to understand fallback order
  config = load_env() or load_file(path) or defaults()

  # good — each source is named, fallback logic reads naturally
  from_env = load_env()
  from_file = load_file(path)
  config = from_env or from_file or defaults()
  ```

Structure:
- Deep nesting is a code smell — 4 indentation levels is the practical maximum. Beyond that,
  refactor: (1) guard clauses to flatten conditionals, (2) extract into named helper functions,
  (3) replace conditional blocks with named variables. When branching is unavoidable, keep
  branch arms short.

## Abstraction Boundaries

- **Analyze before working around** — when an abstraction doesn't provide what's needed, don't
  route around it. Stop. Analyze where the boundary is inadequate — is the interface too
  narrow? Is responsibility misplaced? Propose a better abstraction before writing any code.
  Workarounds compound: each one obscures the real design flaw and makes the correct fix
  harder to apply later.

## Dependencies

- **Minimize import surface** — at every level (function, file, module), fewer imports means
  less coupling. When multiple callers need the same cluster of imports, rethink the
  abstraction boundaries — the concerns may not be factored cleanly.
- Prioritize fast build times when choosing libraries.
- Prefer small, focused libraries over feature-rich heavy ones.
- Library must be stable and well-maintained.
- Measure twice before adding a new dependency.

## Testing

- **TDD flow**: When a change warrants tests, write the failing test first, run it to confirm
  failure, then implement. Do not write tests and implementation together.
- **No tautological tests** — don't test trivial getters that just return a value. These
  restate the implementation and catch nothing. Test behavior that involves logic, branching,
  or composition.

## Self-Audit

After completing a large body of work (multi-file changes, refactors, new features), run a
self-audit before committing:

- Check for stale references: old type names, removed functions still called, dead imports.
- Verify consistency: if a pattern changed (e.g. function signatures), confirm all call sites
  were updated — not just the obvious ones.
- Look for hardcoded values that should inherit from context (e.g. a mode or config flag that
  the caller already has).
- Confirm visibility: fields/methods that were private should stay private unless there's a
  reason to widen access.
- Build and test before declaring done.
- Review test coverage of changed behavior: if logic moved, branching changed, or new code
  paths were introduced, verify existing tests still exercise them and add tests where gaps
  appear.

## Language-Specific Skills

Before starting any coding task, check available skills for a language-specific skill matching
the project's language (e.g. `rust-coding`, `typst-coding`). Load it before writing code —
language skills contain conventions, error handling patterns, and toolchain guidance that
general-coding does not cover.

## Spec-First Development

- **Read all relevant specs before starting any coding task** — design docs, PRDs, RFCs, ADRs,
  or whatever the project uses as source of truth. Flag deviations or missing coverage.
- Ask for directions before proceeding when implementation would differ from spec.

## Unit-of-Work Pattern

Prefer this pattern for organizing mutations (disk writes, git commands, process exec, network
calls) over heavier architectural abstractions (clean architecture, hexagonal, etc.):

- **Mutations only** — pure computation (building strings, merging data, validation) belongs
  elsewhere, not in units of work.
- Each unit of work is a struct with parameters as fields and a single `run` method.
- The `run` method receives a session/context object — no extra parameters.
- No extra parameters in `run()` — everything goes on the struct.
- Group units of work in a dedicated directory (e.g. `actions/`, `commands/`).

Why structs over bare functions:

- **Direct request mapping** — input parameters map directly onto struct fields. In HTTP APIs,
  the struct can double as the request deserialization target, enabling thin controllers that
  just declare which action they map to — the rest is boilerplate or fully automated.
- **Uniform signature** — every unit of work has the same shape (`struct` + `run(context)`),
  making it trivial to layer cross-cutting concerns (audit logging, RBAC checks, rate limiting)
  generically. Bare functions with varying signatures resist this.
- **Testability** — construct the struct with test params, call `run` with a mock context. No
  argument wiring, no partial application.
- **Serializability** — all params live on the struct, so the entire operation can be
  serialized to a job queue, event log, or audit trail. Deferred/async execution comes free.
- **Composability** — uniform signature means actions can be chained into pipelines or saga
  patterns programmatically.

## Edit Protocol

- State what you intend to change and where (declarative), then apply the edit immediately.
- Gather all questions and design decisions upfront before starting work, so execution can
  run unattended for as long as possible.
- Optimize for fast turnaround with minimal user interaction.

## Response Completion

After drafting every response, check the final sentence before sending:
1. If it contains a question mark — delete it.
2. If it offers to do something — delete it.
3. The response now ends on the previous sentence. That's the response.

The user will tell you what to do next. You never need to prompt for it.
