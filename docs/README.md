# docs/

Documentation for Job Tracker.

## Reference

| File | What it covers |
|------|----------------|
| [architecture.md](architecture.md) | App structure, tech stack, key data flows (capture, adding a job, AI extraction, job search, listing check, Google Calendar), CI setup — with Mermaid diagrams |
| [maintenance.md](maintenance.md) | Dependency versions, upgrade notes, and periodic maintenance tasks |

## Where to start

- **New to the project?** Read [architecture.md](architecture.md) to see how the React UI, Rust/Tauri backend, SQLite database, and optional AI and calendar integrations connect.
- **Contributing?** See [CONTRIBUTING.md](../CONTRIBUTING.md) in the root for build setup, pre-commit hooks, platform prerequisites, and PR checklist.
- **Maintaining dependencies or CI?** See [maintenance.md](maintenance.md) for the latest upgrade notes and known pending upgrades.
- **Looking for the big picture?** The [README.md](../README.md) in the repo root has the quick-start guide, feature overview, and full setup instructions.

## Planning

Forward-looking design documents — not descriptions of the current codebase.

| File | What it covers |
|------|----------------|
| [refactor-sync-roadmap.md](refactor-sync-roadmap.md) | Planned performance refactors (phases A / B / C), Android support, and cross-device sync design |

`superpowers/plans/` and `superpowers/specs/` hold historical implementation plans and design specs for features that have since shipped (e.g. quick capture, job detail enrichment). They're kept as an audit trail of *why* a feature was built a certain way, not as current specs — for the shipped behavior, see [architecture.md](architecture.md) instead.
