# Refactor and Sync Roadmap

## Scope
This roadmap tracks performance and maintainability work in three refactor phases, plus cross-device sync and Android support. The sync and Android architecture decisions are recorded in ADRs 0006 and 0007; this file tracks the sequencing.

## Phase A: Quick Safe Pass
Status: in progress

### Completed in this pass
- Stabilized job-search result keys to avoid remount churn.
- Reduced render hot-path work in search cards with memoized derived values.
- Reduced job-search state update bursts by grouping platform loading/error initialization.
- Added DB indexes for common sort/filter patterns.
- Wrapped bulk imports in a transaction.
- Wrapped status updates + history writes in a transaction.
- Extracted shared filename/path helper logic in Rust storage functions.

### Verification checklist
- [x] `npm run build`
- [x] `npm run lint`
- [x] `npm run test`
- [x] `cargo check` (in `src-tauri`)
- [x] `cargo test` (in `src-tauri`)

### Known follow-up from verification
- ESLint shows existing warnings in `src/pages/JobDetailPage.tsx` (`react-hooks/exhaustive-deps`), not introduced by this pass.

## Phase B: Medium Refactor
Status: planned

### Milestones
1. Split broad app context into narrower slices (or selector-based state) to reduce global rerenders.
2. Remove global context dependency from per-result card components by passing narrow action props.
3. Move job-search async transitions to a reducer/state-machine style flow.
4. Replace selected dynamic JSON structures with typed DTOs where straightforward.

### Risk
- Medium: moderate changes to state wiring and component APIs.

## Phase C: Deep Refactor
Status: planned

### Milestones
1. Consolidate DB access strategy and shared persistence boundaries.
2. Convert startup migration/query hotspots to set-based SQL where applicable.
3. Reduce allocation-heavy parsing paths in RSS/text processing.
4. Harden module boundaries between UI, domain logic, and persistence.

### Risk
- Medium-high: broad architectural change surface.

## Cross-Device Sync Roadmap
Status: planned — architecture decided (ADR 0006)

### Decision
Turso (libSQL) with the desktop as an embedded replica in `remote_writes` mode. Turso is the source of truth; reads are served from the local file; a background interval pulls frames down. See [ADR 0006](adr/0006-turso-libsql-embedded-replica-for-sync.md) for the rejected alternatives (Supabase, self-hosted API, libSQL offline writes, one-way mirror).

This supersedes the earlier "Option A: Supabase / Option B: self-hosted API" track. Neither was chosen: both require rewriting every query away from SQLite and hand-building a sync protocol, where the embedded replica keeps the existing SQL, schema, and id scheme intact.

### Why the id scheme drove the decision
The schema uses `INTEGER PRIMARY KEY AUTOINCREMENT` across all 11 tables, with 7 `last_insert_rowid()` call sites and `id: number` typing through the frontend. Any topology with more than one writer allocating ids forces a migration to distributed ids before a single row can sync. Remote writes let the server allocate ids, which removes that migration entirely.

The cost of that choice: **the desktop is read-only when offline.** This is the main assumption to revisit if it proves painful in practice.

### Milestones
1. Split mail-scan tables (`mail_scan_runs`, `mail_fingerprints`, `mail_fingerprint_aliases`, `mail_match_inbox`, `mail_match_dismissals`, `mail_scored_sightings`, `mail_source_cursors`) into a local-only database file. Worth doing on its own merits — libSQL syncs whole files, so scoring caches would otherwise push thousands of rows per scan run to the cloud.
2. Convert `db.rs` from `rusqlite` to `libsql`, embedded replica with remote writes. Migrations port unchanged (`PRAGMA user_version` and `AUTOINCREMENT` both work on libSQL).
3. Propagate `async` through the remaining SQL call sites. `mail_scan/scoring.rs`, `accept.rs`, and `persist.rs` are the bulk of the diff and the riskiest part — that module also orchestrates a threaded sidecar.
4. Handle the offline-desktop path explicitly in the UI rather than failing writes silently.
5. Decide attachment strategy. `job_documents` stores filesystem paths, so PDFs do not sync; either move them to object storage or mark them desktop-only in every client.
6. Watch Turso row-read/write limits after the first sync-enabled release.

### Conflict strategy
Row-level last-write-wins by `updated_at`, which is what a single remote writer gives for free. No tombstones or merge rules are needed: both devices write to the same remote database, so there is nothing to reconcile. Note that libSQL replicas sync WAL frames, not rows — any future move to concurrent offline writes diverges at the frame level and cannot merge, which is why that mode was rejected.

## Android App Roadmap
Status: planned — approach decided (ADR 0007)

### Decision
A standalone Kotlin + Compose client talking to Turso over HTTP (Ktor), not a Tauri mobile target and not React Native. See [ADR 0007](adr/0007-android-as-thin-remote-client.md). This supersedes the earlier React Native + Expo recommendation.

### Scope
| Capability | On Android |
|---|---|
| View jobs, detail, deadlines | Yes |
| Status change (with history) | Yes — highest value, straightforward |
| Notes, priority, tags, contact fields, dates | Yes |
| Create a job by hand | Yes |
| Delete a job | No — cascade and file cleanup stay desktop-owned |
| Attachments / PDFs | No — files on disk, not in the DB |
| Capture, mail scan, LLM extraction, calendar | No — desktop-only by design (ADRs 0001, 0002, 0004) |

### Write invariants to reimplement
The phone bypasses the Tauri commands, and those commands are not thin SQL wrappers. Each of these is enforced in Rust today and must be reproduced as a batched transaction client-side, or the data drifts silently:

- `update_job_status` — `UPDATE jobs` plus a `status_history` INSERT in one transaction.
- `update_job` — validates non-empty company, bumps `updated_at`, and appends `status_history` when the status changed as a side effect of the edit.
- `delete_job` — hand-cascades to `job_documents` and `status_history`, then removes files from disk. The schema declares no foreign-key cascades. This is why delete is not shipped on Android.

### Milestones
1. Land the sync roadmap through milestone 3 — the desktop must be on Turso before a phone client has anything to talk to.
2. Read-only MVP: job list, detail, deadline view.
3. Add status change and field edits with the invariants above.
4. Add manual job creation.
5. Offline outbox — queue pending mutations, replay on reconnect. Deferred to v2; explicitly not solved by switching libSQL to offline writes (ADR 0006).

## Priority Queue
1. Phase B context slicing and card decoupling.
2. Mail-scan database split (sync milestone 1) — independently useful, unblocks the rest.
3. `rusqlite` → `libsql` conversion and the `async` propagation (sync milestones 2–3).
4. Android read-only MVP, then writes.
