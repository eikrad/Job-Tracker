# docs/

Documentation for Job Tracker.

## Contents

| File | What it covers |
|------|----------------|
| [architecture.md](architecture.md) | App structure, tech stack, key data flows (adding a job, AI extraction, job search, Google Calendar), CI setup — with Mermaid diagrams |
| [maintenance.md](maintenance.md) | Dependency versions and upgrade notes |
| [refactor-sync-roadmap.md](refactor-sync-roadmap.md) | Performance refactor phases (A / B / C) and cross-device sync planning |

## Where to start

- **New to the project?** Read [architecture.md](architecture.md) to see how the React UI, Rust/Tauri backend, SQLite database, and optional AI and calendar integrations connect.
- **Contributing?** See [CONTRIBUTING.md](../CONTRIBUTING.md) in the root for build setup, pre-commit hooks, platform prerequisites, and PR checklist.
- **Looking for a quick overview?** The [README.md](../README.md) at the repo root covers installation, quick start, all features, configuration, and CI.
- **Maintaining dependencies?** See [maintenance.md](maintenance.md) for upgrade notes and tooling details.
