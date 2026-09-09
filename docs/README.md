# docs/

Documentation for Job Tracker.

## Reference

| File | What it covers |
|------|----------------|
| [architecture.md](architecture.md) | App structure, tech stack, key data flows (capture, adding a job, AI extraction, job search, listing check, Google Calendar), CI setup — with Mermaid diagrams |
| [maintenance.md](maintenance.md) | Dependency versions, upgrade notes, and periodic maintenance tasks |
| [adr/](adr/) | Architecture Decision Records — one decision per file, with rejected alternatives |
| [../CONTEXT.md](../CONTEXT.md) | Domain glossary (ubiquitous language) |

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
| [superpowers/specs/2026-09-09-mail-scan-integration-design.md](superpowers/specs/2026-09-09-mail-scan-integration-design.md) | **Active design.** In-app Mail Scan (rev. 2). See ADRs 0001–0005 and [CONTEXT.md](../CONTEXT.md) |
| [superpowers/plans/2026-09-09-mail-scan-integration.md](superpowers/plans/2026-09-09-mail-scan-integration.md) | Plan index — PRs A (foundation), B (skeleton), C (scoring & inbox) |

`superpowers/plans/` and `superpowers/specs/` otherwise hold historical plans/specs for shipped features. For shipped behaviour see [architecture.md](architecture.md).
