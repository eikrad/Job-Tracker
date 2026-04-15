# Refactor and Sync Roadmap

## Scope
This roadmap tracks performance and maintainability work in three refactor phases, plus Android support and cross-device sync.

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

## Android App Roadmap
Status: planned

### Milestones
1. Decide mobile stack (recommended: React Native + Expo for TypeScript alignment).
2. Define shared contracts (job model, validation, sync DTOs).
3. Build mobile MVP:
   - Job list and detail views
   - Add/edit jobs
   - Status updates
   - Deadline/calendar view
4. Add offline-first local storage + sync queue.
5. Ship beta and gather sync conflict telemetry.

## Cross-Device Sync Roadmap
Status: planned

### Architecture decision track
- Option A: Supabase (Auth + Postgres + RLS + storage metadata).
- Option B: Self-hosted API with sync endpoints.

### Milestones
1. Choose backend model and define source of truth.
2. Add stable IDs + sync metadata (`updated_at`, sync cursor/checkpoint, tombstones).
3. Implement desktop sync client (pull/push, conflict resolution, retry queue).
4. Add Android client against the same contracts.
5. Validate conflict handling with end-to-end test scenarios.

### Conflict strategy baseline
- Last-write-wins by canonical timestamp for simple fields.
- Tombstone propagation for deletes.
- Deterministic merge rules for list-like fields (e.g., tags).

## Priority Queue
1. Phase B context slicing and card decoupling.
2. Sync architecture decision (Supabase vs self-hosted).
3. Desktop sync integration before Android client development.
