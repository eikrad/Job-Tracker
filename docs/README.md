# docs/

Documentation for **Job Tracker** — a local-first desktop app (Tauri + React + SQLite) for tracking job applications, deadlines, PDFs, and in-app job search in one place.

## Contents

| File | What it covers |
|------|----------------|
| [architecture.md](architecture.md) | App structure, tech stack, and key data flows (adding a job, AI extraction, job search, Google Calendar) — with Mermaid diagrams |
| [maintenance.md](maintenance.md) | Dependency versions, upgrade notes, and periodic maintenance tasks |

## Where to start

- **New to the project?** Read [architecture.md](architecture.md) to see how the React UI, Rust/Tauri backend, SQLite database, and optional AI and calendar integrations connect.
- **Contributing?** See [CONTRIBUTING.md](../CONTRIBUTING.md) in the root for build setup, pre-commit hooks, platform prerequisites, and PR checklist.
- **Maintaining dependencies or CI?** See [maintenance.md](maintenance.md) for the latest upgrade notes and known pending upgrades.
- **Looking for the big picture?** The [README.md](../README.md) in the repo root has the quick-start guide, feature overview, and full setup instructions.

## Future plans

| File | What it covers |
|------|----------------|
| [refactor-sync-roadmap.md](refactor-sync-roadmap.md) | Planned performance refactor phases (A / B / C) and cross-device sync |
