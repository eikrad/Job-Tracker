# Documentation index

One line per document. Start with the [README](../README.md) for the overview and quick
start.

## Current reference (describes the code as it is)

| Document | Purpose |
|---|---|
| [user-guide.md](user-guide.md) | How to use the app: API keys, adding and capturing jobs, job search, calendar, mail scan, backup, import/export, building installers |
| [architecture.md](architecture.md) | How the code is organised: components, source map, data flows (capture, extraction, search, listing check, calendar, mail scan, backup), SQLite tables, storage locations, CI |
| [../python/README.md](../python/README.md) | Contract of the mail-scan Python sidecar (config on stdin, NDJSON events on stdout) |
| [../CONTEXT.md](../CONTEXT.md) | Domain glossary: Capture Inbox, Mail Match, Fingerprint, Enrichment and other terms used in code and docs |
| [adr/](adr/) | Architecture Decision Records, one decision per file with rejected alternatives (records of past decisions; later ADRs may narrow earlier ones) |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | Developer setup, platform prerequisites, checks, sidecar build, branches, commit messages |
| [releasing.md](releasing.md) | Release process: Conventional Commits, release-please, `staging` → `main` |
| [maintenance.md](maintenance.md) | Dated log of dependency upgrades and security audits (newest first) |

### Architecture Decision Records

| ADR | Decision |
|---|---|
| [0001](adr/0001-no-imap-local-mail-folders-only.md) | Mail intake reads local mail folders only, never IMAP |
| [0002](adr/0002-python-sidecar-for-mail-scan.md) | Mail parsing runs in a Python sidecar (scope narrowed by 0004) |
| [0003](adr/0003-mail-match-inbox-separate-from-capture.md) | Mail Match Inbox is separate from the Capture Inbox |
| [0004](adr/0004-sidecar-without-network-or-secrets.md) | The sidecar has no network access and no secrets; scoring and fetching run in Rust |
| [0005](adr/0005-api-keys-in-os-keyring.md) | API keys live in the OS keyring, not in browser storage |

## Historical plans and specs (not kept up to date)

These record how features were designed. The features have shipped or the plans are
stale; for current behaviour read [architecture.md](architecture.md) and the code.

| Document | Status |
|---|---|
| [refactor-sync-roadmap.md](refactor-sync-roadmap.md) | Older roadmap (performance phases A/B/C, Android, cross-device sync). Not updated since mid-2026; phases B/C, Android and sync are not implemented, and some items (RSS parsing) no longer apply |
| [superpowers/specs/2026-03-25-job-detail-enrichment-design.md](superpowers/specs/2026-03-25-job-detail-enrichment-design.md) | Shipped: extra job fields, PDF opening, the `/job/:id` page |
| [superpowers/plans/2026-03-25-job-detail-enrichment.md](superpowers/plans/2026-03-25-job-detail-enrichment.md) | Implementation plan for the above |
| [superpowers/specs/2026-04-24-capture-workflow-design.md](superpowers/specs/2026-04-24-capture-workflow-design.md) | Shipped: URL capture, Quick capture drawer, Capture Inbox |
| [superpowers/plans/2026-04-24-capture-workflow-implementation.md](superpowers/plans/2026-04-24-capture-workflow-implementation.md) | Implementation plan for the above |
| [superpowers/specs/2026-09-09-mail-scan-integration-design.md](superpowers/specs/2026-09-09-mail-scan-integration-design.md) | Shipped: Mail Scan design (rev. 2); still useful for the reasoning, together with ADRs 0001–0005 |
| [superpowers/plans/2026-09-09-mail-scan-integration.md](superpowers/plans/2026-09-09-mail-scan-integration.md) | Plan index for the Mail Scan, linking PRs A (foundation), B (skeleton) and C (scoring and inbox) |

## Where to start

- **Using the app:** [user-guide.md](user-guide.md).
- **Changing the code:** [architecture.md](architecture.md), then [CONTEXT.md](../CONTEXT.md) for the vocabulary and [CONTRIBUTING.md](../CONTRIBUTING.md) for the checks.
- **Working on the mail scan:** the Mail scan section of [architecture.md](architecture.md), ADRs 0001–0005 and [python/README.md](../python/README.md).
- **Maintaining dependencies or CI:** the newest entry in [maintenance.md](maintenance.md).
