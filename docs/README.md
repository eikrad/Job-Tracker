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

The `superpowers/` folder contains implementation plans and design specs from earlier feature work. These are reference material, not actively maintained.

| Path | What it covers |
|------|----------------|
| [superpowers/specs/2026-03-25-job-detail-enrichment-design.md](superpowers/specs/2026-03-25-job-detail-enrichment-design.md) | Job detail enrichment — design spec |
| [superpowers/plans/2026-03-25-job-detail-enrichment.md](superpowers/plans/2026-03-25-job-detail-enrichment.md) | Job detail enrichment — implementation plan |
| [superpowers/specs/2026-04-24-capture-workflow-design.md](superpowers/specs/2026-04-24-capture-workflow-design.md) | Quick-capture workflow — design spec |
| [superpowers/plans/2026-04-24-capture-workflow-implementation.md](superpowers/plans/2026-04-24-capture-workflow-implementation.md) | Quick-capture workflow — implementation plan |
