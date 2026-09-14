# Refactor and Sync Roadmap

## Scope
This roadmap tracks performance and maintainability work in three refactor phases, plus cross-device sync and Android support. The sync, Android, and data-residency decisions are recorded in ADRs 0006–0008; this file tracks the sequencing.

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
Status: planned — architecture decided (ADRs 0006, 0008)

### Decision
libSQL embedded replica with **offline writes and a single writer**. The desktop is the only writer to the synced database and stays fully functional offline; Android reads it and writes intents to a separate outbox database that the desktop drains. Provider is Turso Cloud, with each user supplying their own credentials. See [ADR 0006](adr/0006-turso-libsql-embedded-replica-for-sync.md) and [ADR 0008](adr/0008-sync-provider-and-data-residency.md).

This supersedes the earlier "Option A: Supabase / Option B: self-hosted API" track. Neither was chosen: both require rewriting every query away from SQLite and hand-building a sync protocol, where the embedded replica keeps the existing SQL, schema, and id scheme intact.

### Why a single writer
Two constraints, one answer. The schema uses `INTEGER PRIMARY KEY AUTOINCREMENT` across all 11 tables with 7 `last_insert_rowid()` call sites — two id-allocating writers would force a distributed-id migration before anything could sync. And libSQL replicas sync WAL frames rather than rows, so two replicas writing offline diverge unmergeably. One writer dissolves both problems and keeps the desktop writable offline.

The outbox **must** be a separate database. A table inside the synced file, written by the phone over HTTP, is a second writer and reintroduces the divergence.

### Milestones
1. Split mail-scan tables (`mail_scan_runs`, `mail_fingerprints`, `mail_fingerprint_aliases`, `mail_match_inbox`, `mail_match_dismissals`, `mail_scored_sightings`, `mail_source_cursors`) into a local-only database file. Worth doing on its own merits — libSQL syncs whole files, so scoring caches would otherwise push thousands of rows per scan run.
2. Convert `db.rs` from `rusqlite` to `libsql`, embedded replica with offline writes. Migrations port unchanged.
3. Propagate `async` through the remaining SQL call sites. `mail_scan/scoring.rs`, `accept.rs`, and `persist.rs` are the bulk of the diff and the riskiest part — that module also orchestrates a threaded sidecar.
4. Define the outbox schema (intent kind, target id or client UUID, payload, timestamp) and build the drain loop that applies intents via the existing `db.rs` functions.
5. Encryption at rest with a user-held key; credential and token handling via the existing keyring path (ADR 0005).
6. Decide attachment strategy. `job_documents` stores filesystem paths, so PDFs do not sync; either object storage or desktop-only in every client.

### Conflict strategy
Last-write-wins by timestamp, with the desktop as the single serialization point. No tombstones or merge rules are needed. Any future move to concurrent writers reopens both the id migration and the frame-divergence problem — see ADR 0006 before considering it.

### Known constraints
- One desktop only. Two desktops against one synced database break the single-writer rule.
- Phone edits converge only while the desktop is running.

## Android App Roadmap
Status: planned — approach decided (ADR 0007)

### Decision
A standalone Kotlin + Compose client over Turso's HTTP API (Ktor), reading the synced database and writing intents to the outbox. See [ADR 0007](adr/0007-android-as-thin-remote-client.md). This supersedes the earlier React Native + Expo recommendation.

Because the desktop applies intents through its own `create_job` / `update_job` / `update_job_status` functions, no write invariant is reimplemented on the phone — `status_history`, validation, and the `job_documents` cascade all happen on the desktop side as they do today.

### Scope
| Capability | On Android |
|---|---|
| View jobs, detail, deadlines | Yes |
| Status change (with history) | Yes — highest value |
| Notes, priority, tags, contact fields, dates | Yes |
| Create a job by hand | Yes — real id assigned when the desktop applies it |
| Delete a job | Yes — desktop performs the cascade |
| Attachments / PDFs | No — files on disk, not in the DB |
| Capture, mail scan, LLM extraction, calendar | No — desktop-only by design (ADRs 0001, 0002, 0004) |

### Milestones
1. Land sync milestones 1–4 — the desktop must be on libSQL with a working drain loop before the phone has anything to talk to.
2. Read-only MVP: job list, detail, deadline view.
3. Intent writes for status change and field edits, with optimistic display of unapplied intents.
4. Manual job creation and delete.
5. Local queue so intents can be recorded offline and replayed on reconnect. Deferred; explicitly not solved by making the phone a second replica writer (ADR 0006).

## Priority Queue
1. Phase B context slicing and card decoupling.
2. Mail-scan database split (sync milestone 1) — independently useful, unblocks the rest.
3. `rusqlite` → `libsql` conversion and the `async` propagation (sync milestones 2–3).
4. Outbox schema and drain loop (sync milestone 4).
5. Android read-only MVP, then intent writes.
