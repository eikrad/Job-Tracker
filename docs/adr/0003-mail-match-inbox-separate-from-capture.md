# Mail Match Inbox is separate from Capture Inbox

**Status:** Accepted · 2026-09-09

Browser URL handoff stays in the Capture Inbox. Scored mail candidates use a distinct Mail Match Inbox with its own lifecycle (scores, enrichment completeness, update suggestions, permanent dismiss by Fingerprint).

**Why:** Capture is URL → fetch → extract → save. Mail Matches arrive already scored and partially enriched; merging the queues would blur review semantics and storage needs (SQLite vs localStorage-scale URL queue).

**Consequences:** Two review UIs/patterns may share presentation ideas but not the same persistence model or domain terms (see `CONTEXT.md`).
