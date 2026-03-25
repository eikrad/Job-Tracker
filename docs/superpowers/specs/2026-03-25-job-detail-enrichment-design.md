# Job Detail Enrichment — Design Spec

**Date:** 2026-03-25
**Status:** Approved for implementation

---

## Overview

Three related improvements to the Job Tracker app:

1. **PDF opening** — documents saved in the app can now be opened with the system's default PDF viewer
2. **Richer data extraction** — 12 new fields extracted by AI (or entered manually) covering contact person, workplace location, and job characteristics
3. **Redesigned job detail UI** — sidebar stays for quick overview; a new full-page detail view shows all enriched data; all emoji replaced with Lucide outline icons

---

## 1. PDF Opening

### Problem
Documents (CV, cover letter) are saved to disk via `save_job_document` but there is no way to open them from the app.

### Solution
Add a new Tauri command `open_document(path: String)` that calls `open::that(path)` using the `open` crate (already in `Cargo.toml` at `open = "5.2"` — no new dependency needed). No new capability entry is required.

**Frontend:** Each document row in the list gets an "Open" button (Lucide `ExternalLink` icon) that calls `invoke("open_document", { path: doc.file_path })`. Add `open_document` to `src/lib/tauriApi.ts`.

---

## 2. New Data Fields

### 12 new columns added to the `jobs` table via SQLite migration

| Field | DB column | AI extractable | Group |
|---|---|---|---|
| Contact name | `contact_name` | Yes | Contact |
| Contact email | `contact_email` | Yes | Contact |
| Contact phone | `contact_phone` | Yes | Contact |
| Workplace street | `workplace_street` | Yes | Location |
| Workplace city | `workplace_city` | Yes | Location |
| Workplace postal code | `workplace_postal_code` | Yes | Location |
| Work mode | `work_mode` | Yes | Job details |
| Salary range | `salary_range` | Yes | Job details |
| Contract type | `contract_type` | Yes | Job details |
| Priority (1–3) | `priority` | No — manual only | Job details |
| Reference number | `reference_number` | Yes | Job details |
| Source | `source` | Sometimes | Job details |

All new columns are `TEXT` (nullable), except `priority` which is `INTEGER` (nullable, 1–3).

### DB migration
Extend the existing `migrate_jobs_columns` function in `db.rs` (which already uses the `PRAGMA table_info(jobs)` + conditional `ALTER TABLE` pattern). Read existing column names into a `Vec<String>`, then for each of the 12 new columns issue `ALTER TABLE jobs ADD COLUMN <name> <type>` only when the column is absent. **Do not use `IF NOT EXISTS`** — SQLite does not support that syntax on `ALTER TABLE`.

### Rust structs
`Job` and `NewJob` in `db.rs` get the 12 new optional fields. The `SQL_INSERT_JOB` and `SQL_UPDATE_JOB` queries are extended accordingly. `priority` is `Option<i64>` in Rust; all others are `Option<String>`.

### TypeScript types
`Job` and `NewJob` in `src/lib/types.ts` get the corresponding optional fields. `priority` is `number | undefined` (not in AI merge — see form section). `src/lib/db/schema.ts` (a documentation mirror) must have all 12 new columns added to the `jobs` array, **and** also the two pre-existing missing columns `interview_date` and `start_date` (added in a prior migration but never reflected in the mirror).

### AI extraction prompt
`buildExtractionPrompt` in `extractJobInfo.ts` is updated to request the new fields:

```
contact_name, contact_email, contact_phone,
workplace_street, workplace_city, workplace_postal_code,
work_mode (one of: Remote, Hybrid, On-site),
salary_range (free text, as stated in the ad),
contract_type (one of: Permanent, Fixed-term, Freelance, Internship),
reference_number, source
```

`normalizeLlmJobPartial` is extended to map the new keys (and common aliases) into `Partial<NewJob>`.

### Form field updates (`JobForm.tsx`)
Two constants in `JobForm.tsx` must be updated alongside the new fields:

- **`jobToNewJob`** — add all 12 new fields mapping `job.field ?? undefined`.
- **`MERGEABLE_JOB_FIELDS`** — add the 11 AI-extractable fields. Do **not** add `priority` (manual only, must never be overwritten by AI extraction).

---

## 3. UI Changes

### Icons — replace all emoji with Lucide React

Add `lucide-react` as a dependency. Replace all emoji used as icons throughout the app with the appropriate Lucide stroke icons (never Unicode star/pin/calendar characters — always the React component).

Key icon mappings:
- Calendar / dates → `Calendar`
- Status / tag → `Tag`
- Location → `MapPin`
- Work mode → `Monitor` (Remote), `Building2` (On-site), `Layers` (Hybrid), `HelpCircle` (Not specified)
- Documents → `FileText`
- Open file → `ExternalLink`
- Delete → `Trash2`
- Edit → `Pencil`
- Priority stars → `Star` with `fill="currentColor"` for filled, `fill="none"` for outline
- Arrow / navigate → `ArrowRight`

### i18n strings (`src/i18n/en.ts`)
All new UI labels must be added to `en.ts` rather than inlined in components. New strings needed include (at minimum): labels for all 12 new form fields, section titles ("Contact & Location", "Job Details"), sidebar button ("Full details"), detail page section headers, and select option labels for `work_mode` and `contract_type`.

### Sidebar — slim quick-overview

`JobDetailTimeline` keeps its role as the right sidebar. Its Props interface is simplified — remove `onSavedPdf`, `onExtract`, and `onUpdateJob` (document upload and edit form move to the detail page). Keep `onDeleteJob` for a quick delete action. Add a `onViewDetails: (jobId: number) => void` prop that the sidebar button calls to trigger navigation.

Sidebar Props after the change: remove `onSavedPdf`, `onExtract`, `onUpdateJob`, and `statuses` (all only needed by the edit form / upload, which move to the detail page). Keep `onDeleteJob`. Add `onViewDetails: (jobId: number) => void`.

Sidebar shows only:
- Company + title (heading)
- Status + priority stars (`Star` icons)
- Key dates (deadline, interview, start)
- Work mode + contract type
- Tags
- "Full details →" button (`ArrowRight` icon) that calls `onViewDetails(selected.id)`

Document list and edit form are removed from the sidebar.

### New full-page detail view

New route: `/job/:id` — add to `App.tsx` alongside existing routes. The app already uses `BrowserRouter`. In Tauri production builds, `BrowserRouter` requires the Tauri asset server to serve `index.html` for all paths. Verify this works in a production build; if not, migrate to `HashRouter` across the whole app (routes become `/#/job/123`).

New component: `JobDetailPage` at `src/pages/JobDetailPage.tsx`

**Data loading:** Read `id` from `useParams`, look up the job in the `JobTrackerContext` `jobs` array (`jobs.find(j => j.id === Number(id))`). Handle two edge cases: context not yet loaded (show loading state), job not found (show "not found" message or redirect to `/`). Also load `listJobDocuments(id)` and `listStatusHistory(id)` locally within the page.

Layout: three-column grid at desktop width, single column on narrow screens.

**Column 1 — Overview**
- Company, title, status, priority stars
- Work mode, contract type, salary range
- Dates (deadline, interview, start)
- Tags, reference number, source, detected language
- Edit (opens `JobForm` inline or navigates to `/jobs/edit/:id`) / Delete actions

**Column 2 — Contact & Location**
- Contact name, email (`mailto:` link), phone (`tel:` link)
- Workplace street, city, postal code

**Column 3 — Documents + History**
- Document list: name, type badge, Open button (`ExternalLink` icon, calls `open_document`), Delete button (`Trash2` icon)
- Upload new document (type selector + file input, calls `saveJobDocument`)
- Status history timeline

Navigation: "← Back" button in the page header calls `navigate(-1)` via `useNavigate`. The sidebar "Full details →" button navigates via `onViewDetails` which calls `navigate(`/job/${id}`)` in `DashboardPage`.

### Form — new field groups

`JobForm` gets two new collapsible sections (toggle open/closed with a button) added below the existing fields:

**Contact & Location** (collapsed by default)
- `contact_name`, `contact_email`, `contact_phone`
- `workplace_street`, `workplace_city`, `workplace_postal_code`

**Job Details** (collapsed by default)
- `work_mode` (select: Remote / Hybrid / On-site / Not specified)
- `contract_type` (select: Permanent / Fixed-term / Freelance / Internship / Not specified)
- `salary_range` (text input)
- `priority` (1–3 `Star` icon toggle — manual only, not populated by AI extract)
- `reference_number` (text input)
- `source` (text input)

After "Extract with AI", all extractable fields in these sections are populated. `priority` is never touched by extraction.

---

## 4. Routing

The app already uses `BrowserRouter` with routes at `/` and `/jobs/new`. Add:

- `/job/:id` → `JobDetailPage`

`DashboardPage` passes `onViewDetails={(id) => navigate(`/job/${id}`)}` to `JobDetailTimeline` via the existing prop chain. The `selected` state in `JobTrackerContext` is not required for `JobDetailPage` (it loads from the context's `jobs` array by id), but setting it on navigation preserves sidebar highlight state when returning to the board.

---

## 5. Non-goals

- No PDF preview inside the app (open with system viewer only)
- No map/geocoding for the workplace address
- No AI extraction of priority (always manual)
- No changes to CSV/JSON export format in this iteration (new fields are omitted from existing exports until a separate export update)
- No `HashRouter` migration unless production build testing reveals `BrowserRouter` path issues (test as part of step 9)

---

## Implementation order

1. DB migration in `migrate_jobs_columns` + Rust struct updates (`db.rs`)
2. TypeScript types (`src/lib/types.ts`, `src/lib/db/schema.ts`)
3. `open_document` Tauri command + `tauriApi.ts` binding
4. Extraction prompt + `normalizeLlmJobPartial` + `MERGEABLE_JOB_FIELDS` + `jobToNewJob` updates
5. `en.ts` — add all new labels
6. `lucide-react` install + emoji replacement throughout the app
7. `JobForm` — new Contact & Location and Job Details sections
8. `JobDetailTimeline` sidebar slim-down (remove upload/edit/statuses props, add `onViewDetails` prop)
9. `JobDetailPage` + `/job/:id` route in `App.tsx` — verify `BrowserRouter` works in production build; migrate to `HashRouter` if not
