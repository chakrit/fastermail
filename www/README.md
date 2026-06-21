# www/

Public-facing site source for fastermail — the outward docs a *user* of the tool
reads. Plain markdown for now; the static-site generator (mdBook / Hugo / etc.) is
**not yet chosen** — pick one and wire a build when the content justifies it.

Distinct from `docs/`: `docs/` is the internal design record (spec, decisions,
notes) for contributors/agents; `www/` is the curated public surface.

- `architecture.md` — what fastermail is and how it's shaped (overview).
- `usage.md` — install, auth, the command surface, MCP registration.
