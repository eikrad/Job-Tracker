# Job Tracker

Local-first desktop app for tracking job applications from discovery through outcomes.

## Language

### Capture & mail intake

**Capture Inbox**:
A queue of browser-handed-off job URLs waiting to be fetched, extracted, and accepted or dismissed.
_Avoid_: Mail Match Inbox, email queue, general inbox

**Mail Match Inbox**:
A review queue of scored and enriched candidates from local job-alert mail folders. Tabs cover pending items, revocable dismissals, and scan history. Distinct from Capture Inbox.
_Avoid_: Capture Inbox, email inbox, draft jobs

**Mail Match**:
One candidate listing from a Mail Scan (score, reasons, draft fields), not yet a Job. Accept opens a prefilled job form that creates a Job in status Interesting when the user confirms.
_Avoid_: Job, application, capture item

**Job**:
A tracked application opportunity on the board, with status in the configured workflow.
_Avoid_: Listing, match, vacancy (when referring to something already saved in the tracker)

**Mail Match Fingerprint**:
Stable listing identity using tiered keys: a strong key (`board:external_id` or canonical URL) when available, else a weak key (`company|title|city` after normalization). Clustering follows strong-key identity first; weak-only collisions merge; differing strong keys with the same weak key are near-duplicates, not merges.
_Avoid_: Job id, email message id as sole identity, OR-over-two-keys dedup

**Near-duplicate**:
Two listings that share a weak key but have different strong keys; shown together for human dismiss, never auto-merged.
_Avoid_: Duplicate Job, same fingerprint

**Dismissed Mail Match**:
A fingerprint the user suppressed from the inbox; revocable from the Dismissed view and visible in run summaries.
_Avoid_: Permanent silent suppress, soft hide without restore

**Enrichment**:
Filling a Mail Match draft toward Job fields (deadline, contacts, workplace, salary, etc.) from listing text when fetchable; may be complete, partial, or failed.
_Avoid_: Scoring, Capture URL extraction (different pipeline)

**Candidate Profile**:
User-owned short and full markdown documents in app data used for pass-1 and pass-2 scoring; replaceable in Settings, never committed, excluded from export/backup by default.
_Avoid_: Resume/CV as product terms for these files, hardcoded profile in source

**Scored Sighting**:
A recorded score for a fingerprint at a pass against profile content hash, prompt version, and model; under-cutoff sightings re-score only when profile hash or prompt version changes (not on model switch alone).
_Avoid_: Job, Mail Match

**Mail Match Update Suggestion**:
An inbox item proposing patches to an existing Job. Accept fills only fields that are still blank on the live Job, refreshes `mail_score*` columns, never changes status or priority; the patch is recomputed at accept time if the Job changed.
_Avoid_: Silent sync, auto-update, writing priority from the LLM

**Mail Score**:
Advisory fit score from the mail pipeline stored on the Job (`mail_score`, `mail_score_reason`, `mail_scored_at`); never auto-written into priority.
_Avoid_: Priority, keyword score as the product term for this field

**Mail Scan**:
A user-triggered run that reads local Thunderbird folders via a network-less Python sidecar, then scores, fetches, and enriches in Rust, streaming progress into the Mail Match Inbox. No markdown report in v1.
_Avoid_: IMAP sync, scheduled crawl, weekly CLI export
