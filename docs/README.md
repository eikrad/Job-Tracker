# docs/

Documentation for Job Tracker.

## Reference

| File | What it covers |
|------|----------------|
| [architecture.md](architecture.md) | App structure, tech stack, key data flows (adding a job, AI extraction, job search, Google Calendar), CI setup — with Mermaid diagrams |
| [maintenance.md](maintenance.md) | Dependency versions, upgrade notes, and periodic maintenance tasks |

## Where to start

- **New to the project?** Read [architecture.md](architecture.md) to see how the React UI, Rust/Tauri backend, SQLite database, and optional AI and calendar integrations connect.
- **Contributing?** See [CONTRIBUTING.md](../CONTRIBUTING.md) in the root for build setup, pre-commit hooks, platform prerequisites, and PR checklist.
- **Maintaining dependencies or CI?** See [maintenance.md](maintenance.md) for the latest upgrade notes and known pending upgrades.
- **Looking for the big picture?** The [README.md](../README.md) in the repo root has the quick-start guide, feature overview, and full setup instructions.

## Planning

Mostly forward-looking design documents — not descriptions of the current codebase, with one exception noted below.

| File | What it covers |
|------|----------------|
| [refactor-sync-roadmap.md](refactor-sync-roadmap.md) | Phase A (quick performance pass) is **done and merged**. Phases B/C (deeper refactors), Android support, and cross-device sync are still just planned — none of that code exists yet |

## Historical design docs

`superpowers/plans/` and `superpowers/specs/` hold dated, one-off design specs and step-by-step implementation plans used to build specific past features (e.g. the job detail enrichment and capture workflow). They're a record of *how* those features were designed and shipped, not a live status board — checkboxes in the plan files are template artifacts and don't reflect current progress. The features themselves are implemented (see [architecture.md](architecture.md)).
