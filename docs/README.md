# docs/

Documentation for Job Tracker.

## Contents

| File | What it covers |
|------|----------------|
| [architecture.md](architecture.md) | App structure, tech stack, key data flows (adding a job, AI extraction, job search, Google Calendar), CI setup — with Mermaid diagrams |
| [maintenance.md](maintenance.md) | Dependency versions, upgrade notes, and periodic maintenance tasks |
| [refactor-sync-roadmap.md](refactor-sync-roadmap.md) | Performance refactor phases (A / B / C) and cross-device sync planning |

## Where to start

- **New to the project?** Read [architecture.md](architecture.md) to see how the React UI, Rust/Tauri backend, SQLite database, and optional AI and calendar integrations connect.
- **Contributing?** See [CONTRIBUTING.md](../CONTRIBUTING.md) in the root for build setup, pre-commit hooks, platform prerequisites, and PR checklist.
- **Maintaining dependencies or CI?** See [maintenance.md](maintenance.md) for the latest upgrade notes and known pending upgrades.
- **Planning future work?** See [refactor-sync-roadmap.md](refactor-sync-roadmap.md) for upcoming refactoring, Android support, and cross-device sync plans.
- **Looking for the big picture?** The [README.md](../README.md) in the repo root has the quick-start guide, feature overview, and full setup instructions.

## Historical planning docs

The `superpowers/` folder contains implementation plans and design specs from earlier development phases. These are reference material, not actively maintained.
