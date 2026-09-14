# Cross-device sync uses libSQL embedded replicas with a single writer

**Status:** Proposed · 2026-09-14

The desktop keeps a local SQLite file, but `rusqlite` is replaced by the `libsql` crate opened as an **embedded replica with offline writes**. The desktop is the *only* writer to the synced database: it writes locally, stays fully functional offline, and pushes on reconnect. Android reads that database over HTTP and never writes to it — phone edits go to a **separate** outbox database as intents, which the desktop drains and applies (ADR 0007). Mail-scan tables move to a third database file that is local-only and never syncs.

**Why:** Two constraints shape this, and both are satisfied by keeping exactly one writer.

The first is the id scheme. The schema uses `INTEGER PRIMARY KEY AUTOINCREMENT` across all 11 tables with 7 `last_insert_rowid()` call sites and `id: number` typing through the frontend. Any topology with two id-allocating writers forces a migration to distributed ids before a single row can sync. One allocator means ids never collide and none of that work is needed.

The second is that libSQL replicas sync WAL frames, not rows. Two replicas writing while offline diverge at the frame level and cannot be merged — `sync_offline` pushes when ahead and pulls otherwise, with no rebase. This is what makes concurrent offline writes unsafe. It is *not* an argument against offline writes as such: with a single writer the remote never advances independently, so the divergence case cannot arise and the desktop keeps writing offline at no risk.

Routing phone edits through a separate outbox database is what removes the second writer. It must be a separate database — a table inside the synced file, written over HTTP, would reintroduce exactly the divergence this avoids.

**Considered options:** Supabase (Postgres + RLS) and a self-hosted sync API, both from the earlier roadmap — each requires rewriting every query away from SQLite and building a sync protocol by hand; embedded replica with `remote_writes = true`, where the server allocates ids and the phone writes directly — simpler, but the desktop becomes read-only whenever it is offline, which is a poor trade for a local-first app; a one-way mirror with `rusqlite` untouched, avoiding the async conversion entirely but requiring a hand-written push (diffing on `updated_at`, tombstones for deletes) and leaving the phone read-only. Chose offline writes with a single writer.

**Consequences:** `libsql` is async, so every SQL call site and its callers become `async`/`.await` — the colouring propagates through `mail_scan/` (`scoring.rs`, `accept.rs`, `persist.rs`), which is the bulk of the diff and the riskiest part, since that module also orchestrates a threaded sidecar. Phone edits converge only while the desktop is running; close the laptop for a week and the phone accumulates unapplied intents against stale data, so the Android client must display its own pending intents optimistically. The single-writer rule is load-bearing: running two desktops against one synced database breaks it. libSQL syncs whole files, so mail-scan scoring caches would otherwise push thousands of rows per run — hence the local-only third database. Attachments stay local: `job_documents` holds filesystem paths, so PDFs never reach other devices. Migrations port unchanged; `PRAGMA user_version` and `AUTOINCREMENT` both work on libSQL.

The sync target is **provider-agnostic**. `sqld`, the libSQL server, is open source and self-hostable; this entire design works identically against an instance on European infrastructure. Changing providers is an endpoint and a token, not a code change. See ADR 0008 for which provider is used and why.
