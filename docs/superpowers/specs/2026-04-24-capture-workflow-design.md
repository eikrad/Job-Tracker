# Capture Workflow Design

Date: 2026-04-24
Status: Approved for planning
Scope: Faster daily solo workflow by reducing browser context switching

## Problem Statement

The current app already supports adding jobs, AI-assisted extraction from pasted text, reminders, and job search. The primary daily friction is context switching between browser job pages and Job Tracker when saving opportunities quickly.

The design goal is to reduce the time and mental overhead from "job discovered in browser" to "job saved in tracker" while keeping data quality acceptable and preserving local-first reliability.

## Goals

1. Minimize browser-to-app switching cost during capture.
2. Keep capture behavior consistent across all entry points.
3. Preserve user control through editable confirmation before save.
4. Gracefully degrade when extraction/fetch fails.
5. Deliver in small, testable phases with commit checkpoints.

## Non-Goals

1. Perfect extraction on every job site from day one.
2. Full browser-extension ecosystem support in the first rollout.
3. Requiring cloud sync for capture to function.
4. Large unrelated refactors outside capture boundaries.

## Rollout Strategy

### Phase 1 - One-click Capture from URL

- Add a URL input and capture action.
- Fetch listing content/metadata and produce a draft.
- Show editable confirmation card.
- Save as a normal job entry (default status `Interesting`).

### Phase 2 - Quick Capture Drawer

- Add a global drawer accessible from main views.
- Reuse the same capture pipeline as Phase 1.
- Optimize for keyboard-first, low-interruption capture.

### Phase 3 - Browser Companion Inbox

- Add lightweight browser handoff into a local inbox queue.
- Process queue items in-app in batches.
- Accept/edit/dismiss with the same confirm/save behavior.

## Architecture and Boundaries

### Capture Domain Module

Introduce a capture domain module that owns lifecycle and business rules:

- state progression:
  - `idle`
  - `fetching`
  - `extracted`
  - `confirmed`
  - `saved`
  - `failed`

This module must be UI-agnostic and shared by URL bar, drawer, and inbox processing.

### Adapters

1. `ListingFetcherAdapter`
   - Input: URL
   - Output: raw listing text + metadata (when available)
   - Responsibility: retrieval/parsing boundaries only
2. `ExtractionAdapter`
   - Input: raw listing payload
   - Output: normalized draft job fields + confidence hints
   - Responsibility: mapping, normalization, confidence notes
3. `InboxAdapter`
   - Input: browser-origin payload
   - Output: queued capture item records
   - Responsibility: queue persistence and state updates

### UI Components

1. `CaptureFromUrlBar` (Phase 1)
2. `QuickCaptureDrawer` (Phase 2)
3. `CaptureInboxPanel` (Phase 3)
4. `CapturePreviewCard` (shared confirm/edit UI)

The preview card is the single confirmation surface across all phases.

### Persistence

- Existing jobs table remains the source of truth for saved jobs.
- Add capture queue/attempt storage for:
  - source URL
  - capture channel (`url_bar`, `drawer`, `browser_inbox`)
  - status (`pending`, `processing`, `failed`, `accepted`, `dismissed`)
  - timestamps
  - failure reason
  - duplicate hint metadata (when applicable)

## Detailed Data Flow

### Phase 1 URL Capture

1. User pastes URL and triggers capture.
2. `ListingFetcherAdapter` retrieves readable content.
3. `ExtractionAdapter` creates normalized draft.
4. `CapturePreviewCard` displays editable fields and confidence hints.
5. User confirms save.
6. Job is inserted with default status `Interesting` unless user changes it.
7. Optionally navigate to full detail page after save.

### Phase 2 Drawer Capture

1. User opens global drawer from any main view.
2. Same fetch -> extract -> preview -> confirm pipeline runs.
3. On save, drawer closes and returns user to original context state.

### Phase 3 Inbox Processing

1. Browser handoff writes `pending` item into local inbox.
2. User opens inbox panel and selects items.
3. Each item goes through shared preview/confirm path.
4. Accepted items become jobs; dismissed items remain auditable as dismissed.

## Error Handling and Recovery

1. **Blocked/unreadable URL**
   - Show fallback manual mini-form.
   - Preserve source URL.
   - Require `company` + `status`.
2. **Partial extraction**
   - Allow save with missing optional fields.
   - Highlight uncertain values for quick correction.
3. **Timeout/network errors**
   - Mark capture as failed with retry action.
   - Persist failure reason for diagnostics.
4. **Potential duplicates**
   - Prompt user with options:
     - merge/update existing
     - save as new anyway
     - cancel

## Validation Rules

Required for capture-origin saves:

- `company`
- `status`
- `url`

Optional normalized fields:

- title
- location
- salary range
- contact details
- tags
- notes

All captures should retain metadata about source channel and timestamps.

## Testing Strategy

### Unit Tests

- Capture state machine transitions.
- Extraction normalization and field mapping.
- Duplicate decision branching logic.

### Integration Tests

- Adapter success path.
- Partial extraction path.
- Blocked/unreadable retrieval path.
- Timeout/retry path.

### UI Tests

- Phase 1 URL bar happy path.
- Phase 2 drawer keyboard-only capture.
- Phase 3 inbox batch accept/edit/dismiss.
- Duplicate prompt actions.

## Acceptance Criteria

### Phase 1

- User can paste URL, edit draft, and save quickly in a single flow.
- Capture failures provide actionable fallback, not dead ends.

### Phase 2

- User can complete capture without leaving current dashboard context.
- Keyboard-first flow is fully usable end to end.

### Phase 3

- Browser-origin items appear in inbox and are processable in batches.
- No data loss during accept/edit/dismiss paths.

## Delivery and Commit Checkpoints

1. Commit: Phase 1 implementation + tests.
2. Commit: Phase 2 implementation + tests.
3. Commit: Phase 3 implementation + tests.

This spec commit is separate and precedes implementation planning.

## Risks and Mitigations

1. **Site variability**
   - Mitigation: strict fallback manual capture + confidence hints.
2. **UI complexity creep**
   - Mitigation: shared preview component and single capture domain flow.
3. **State inconsistencies between entry points**
   - Mitigation: central capture lifecycle module used by all channels.
4. **Scope creep**
   - Mitigation: enforce phased delivery and explicit non-goals.

## Open Decisions Resolved in This Spec

1. Implement all three approaches, phased.
2. Reuse one shared capture pipeline and confirmation UI.
3. Keep local-first behavior independent of cloud sync.
4. Commit between phases as explicit checkpoints.
