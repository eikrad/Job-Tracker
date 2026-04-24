# Capture Workflow Implementation Plan

> For execution: implement phase-by-phase, run verification at each checkpoint, and commit between phases as approved.

**Source spec:** `docs/superpowers/specs/2026-04-24-capture-workflow-design.md`  
**Goal:** Reduce browser context switching by delivering URL capture, quick drawer capture, and browser inbox capture on a shared pipeline.

---

## 0) Pre-flight and constraints

- [ ] Keep changes scoped to capture workflow only (no unrelated refactors).
- [ ] Reuse existing app conventions (state hooks, i18n, Tauri API boundaries, tests).
- [ ] Preserve local-first behavior.
- [ ] Keep required save fields for capture-origin items explicit (`company`, `status`, `url`).

---

## 1) Shared capture foundation (before phase features)

### 1.1 Domain and types
- [ ] Add capture domain types and state machine lifecycle:
  - `idle`, `fetching`, `extracted`, `confirmed`, `saved`, `failed`
- [ ] Define normalized draft payload type for extracted fields.
- [ ] Define capture channel enum/type:
  - `url_bar`, `drawer`, `browser_inbox`

### 1.2 Adapter interfaces
- [ ] Add `ListingFetcherAdapter` interface (URL -> raw content/metadata).
- [ ] Add `ExtractionAdapter` interface (raw payload -> normalized draft + confidence hints).
- [ ] Add `InboxAdapter` interface (browser payload -> queued capture items).

### 1.3 Shared UI primitive
- [ ] Implement `CapturePreviewCard`:
  - editable fields
  - confidence hints (read-only indicator)
  - confirm/cancel actions
- [ ] Ensure it is reusable across all entry points.

### 1.4 Persistence scaffolding
- [ ] Add local persistence for capture queue/attempt records with:
  - source URL
  - channel
  - status
  - timestamps
  - failure reason
  - duplicate hint metadata
- [ ] Keep jobs table as source of truth for accepted saves.

### 1.5 Verification gate
- [ ] Unit tests for state transitions.
- [ ] Unit tests for normalization mapping and validation rules.
- [ ] Build/lint/tests pass (`npm run verify:frontend`).

---

## 2) Phase 1 - One-click Capture from URL

### 2.1 UX and wiring
- [ ] Add `CaptureFromUrlBar` to target page(s) (Dashboard and/or Add Job entry).
- [ ] Flow:
  1. paste URL
  2. fetch + extract
  3. open `CapturePreviewCard`
  4. confirm save
- [ ] Default saved status to `Interesting` (editable in preview).

### 2.2 Validation and fallback
- [ ] Enforce required fields at save time (`company`, `status`, `url`).
- [ ] If fetch fails or page unreadable:
  - show manual fallback mini-form
  - preserve URL
  - keep save path available

### 2.3 Duplicate handling
- [ ] Add duplicate detection hook against existing jobs.
- [ ] Prompt with options:
  - merge/update existing
  - save as new anyway
  - cancel

### 2.4 Verification gate
- [ ] Integration tests:
  - successful URL capture
  - unreadable/blocked URL fallback
  - timeout/retry
- [ ] UI tests for happy path + duplicate dialog behavior.
- [ ] Ensure no regressions in existing dashboard/add-job flows.

### 2.5 Commit checkpoint
- [ ] Commit Phase 1 changes with tests.

---

## 3) Phase 2 - Quick Capture Drawer

### 3.1 Drawer component
- [ ] Implement `QuickCaptureDrawer` accessible globally from core views.
- [ ] Reuse Phase 1 shared pipeline and `CapturePreviewCard`.
- [ ] Preserve current page state/context when opening/closing drawer.

### 3.2 Keyboard-first behavior
- [ ] Add keyboard-friendly flow:
  - URL paste focus on open
  - tab-order through editable fields
  - enter/primary shortcut for save
- [ ] Ensure focus return to prior context on close.

### 3.3 Verification gate
- [ ] UI tests for drawer flow and keyboard-only path.
- [ ] Regression test: board/table/calendar interaction state is preserved.
- [ ] Lint/build/test suite green.

### 3.4 Commit checkpoint
- [ ] Commit Phase 2 changes with tests.

---

## 4) Phase 3 - Browser Companion Inbox

### 4.1 Inbound capture path
- [ ] Add lightweight local handoff endpoint/path for browser-origin payloads.
- [ ] Persist incoming items as `pending` in capture inbox storage.

### 4.2 Inbox UI
- [ ] Implement `CaptureInboxPanel` with batch actions:
  - accept
  - edit + accept
  - dismiss
- [ ] Accepted items must use the same preview/confirm/save pipeline.

### 4.3 Auditing and reliability
- [ ] Keep dismissed/failed item metadata auditable.
- [ ] Add retry behavior for failed processing.
- [ ] Surface failure reasons clearly in inbox UI.

### 4.4 Verification gate
- [ ] Integration tests for inbox queue lifecycle.
- [ ] UI tests for batch processing and no-data-loss behavior.
- [ ] Regression tests around normal job creation/edit remain green.

### 4.5 Commit checkpoint
- [ ] Commit Phase 3 changes with tests.

---

## 5) Cross-phase acceptance checklist

- [ ] Capture flow is consistent across URL bar, drawer, and inbox.
- [ ] Manual fallback path works when extraction/fetch cannot complete.
- [ ] Duplicate handling is deterministic and user-controlled.
- [ ] Required capture-origin validation is enforced.
- [ ] No critical regressions in dashboard, job form, or detail views.

---

## 6) Execution order summary

1. Shared foundation
2. Phase 1 feature + tests + commit
3. Phase 2 feature + tests + commit
4. Phase 3 feature + tests + commit
5. Final full verification run

---

## 7) Final verification command set

- [ ] `npm run verify`
- [ ] If Rust/Tauri environment available: `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] Manual smoke pass:
  - URL capture happy path
  - unreadable URL fallback
  - drawer keyboard capture
  - inbox batch accept/edit/dismiss
