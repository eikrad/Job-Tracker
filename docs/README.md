# docs/

Documentation for Job Tracker.

## Reference

| File | What it covers |
|------|----------------|
| [user-guide.md](user-guide.md) | Setup and usage: API keys, job search, Google Calendar, mail scan, data storage, import/export, installers and AppImage |
| [architecture.md](architecture.md) | App structure, tech stack, key data flows (capture, adding a job, AI extraction, job search, listing check, Google Calendar), CI setup — with Mermaid diagrams |
| [maintenance.md](maintenance.md) | Dependency versions, upgrade notes, and periodic maintenance tasks |
| [releasing.md](releasing.md) | Release process: Conventional Commits, release-please, staging → main |
| [adr/](adr/) | Architecture Decision Records — one decision per file, with rejected alternatives |
| [../CONTEXT.md](../CONTEXT.md) | Domain glossary (ubiquitous language) |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | Build setup, checks, sidecar build, branches and PRs |

## Where to start

- **New to the project?** Read [architecture.md](architecture.md) to see how the React UI, Rust/Tauri backend, SQLite database, and optional AI and calendar integrations connect.
- **Contributing?** See [CONTRIBUTING.md](../CONTRIBUTING.md) in the root for build setup, pre-commit hooks, platform prerequisites, and PR checklist.
- **Maintaining dependencies or CI?** See [maintenance.md](maintenance.md) for the latest upgrade notes and known pending upgrades.
- **Looking for the big picture?** The [README.md](../README.md) in the repo root has the overview and quick start; [user-guide.md](user-guide.md) has the detailed setup.

## Planning

Forward-looking design documents — not descriptions of the current codebase.

| File | What it covers |
|------|----------------|
| [refactor-sync-roadmap.md](refactor-sync-roadmap.md) | Planned performance refactors (phases A / B / C), Android support, and cross-device sync design |
| [superpowers/specs/2026-09-09-mail-scan-integration-design.md](superpowers/specs/2026-09-09-mail-scan-integration-design.md) | Design of the in-app Mail Scan (rev. 2), now shipped. Still the reference for the reasoning; see ADRs 0001–0005 and [CONTEXT.md](../CONTEXT.md) |
| [superpowers/plans/2026-09-09-mail-scan-integration.md](superpowers/plans/2026-09-09-mail-scan-integration.md) | Plan index for the shipped Mail Scan — PRs A (foundation), B (skeleton), C (scoring & inbox) |

`superpowers/plans/` and `superpowers/specs/` hold historical plans/specs for shipped features. For shipped behaviour see [architecture.md](architecture.md).
