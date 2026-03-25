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
Add a new Tauri command `open_document(path: String)` that uses `tauri-plugin-opener` to open the file with the OS default application. Add the plugin to `Cargo.toml` and register the capability.

**Frontend:** Each document row in the list gets an "Open" button (with a Lucide `ExternalLink` icon) that calls `invoke("open_document", { path: doc.file_path })`.

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
A new migration step in `db::init_db` adds the columns with `ALTER TABLE jobs ADD COLUMN IF NOT EXISTS ...` (SQLite supports this non-destructively).

### Rust structs
`Job` and `NewJob` in `db.rs` get the 12 new optional fields. The `SQL_INSERT_JOB` and `SQL_UPDATE_JOB` queries are extended accordingly.

### TypeScript types
`Job` and `NewJob` in `src/lib/types.ts` get the corresponding optional fields. `NewJob` excludes `priority` from the LLM-mergeable fields list (manual only).

### AI extraction prompt
`buildExtractionPrompt` in `extractJobInfo.ts` is updated to request the new fields:

```
contact_name, contact_email, contact_phone,
workplace_street, workplace_city, workplace_postal_code,
work_mode (Remote/Hybrid/On-site),
salary_range (free text, as stated in the ad),
contract_type (Permanent/Fixed-term/Freelance/Internship),
reference_number, source
```

`normalizeLlmJobPartial` is extended to map the new keys into `Partial<NewJob>`.

---

## 3. UI Changes

### Icons — replace all emoji with Lucide React

Add `lucide-react` as a dependency. Replace all emoji used as icons throughout the app (sidebar, detail view, form hints, etc.) with the appropriate Lucide stroke icons. This gives a consistent look: single weight, no colour, scales with font size.

Key icon mappings:
- Calendar / dates → `Calendar`
- Status/tag → `Tag`
- Location → `MapPin`
- Work mode → `Monitor` (remote) / `Building2` (on-site)
- Documents → `FileText`
- Open file → `ExternalLink`
- Delete → `Trash2`
- Edit → `Pencil`
- Priority stars → `Star` (filled vs outline via `fill`)
- Arrow/navigate → `ArrowRight`

### Sidebar — slim quick-overview

`JobDetailTimeline` keeps its role as the right sidebar but shows only the essential glance info:

- Company + title (heading)
- Status + priority stars (★★☆)
- Key dates (deadline, interview, start)
- Work mode + contract type
- Tags
- "Full details →" button that navigates to the detail page

Contact info, address, documents upload, and history are removed from the sidebar. Documents list (read-only, open buttons) may optionally remain for convenience.

### New full-page detail view

New route: `/job/:id`

New component: `JobDetailPage` at `src/pages/JobDetailPage.tsx`

Layout: three-column grid at desktop width, single column on narrow screens.

**Column 1 — Overview**
- Company, title, status, priority
- Work mode, contract type, salary range
- Dates (deadline, interview, start)
- Tags, reference number, source, detected language
- Edit / Delete actions

**Column 2 — Contact & Location**
- Contact name, email (mailto link), phone (tel link)
- Workplace street, city, postal code

**Column 3 — Documents + History**
- Document list: name, type badge, Open button (ExternalLink icon), Delete button
- Upload new document (type selector + file input)
- Status history timeline

Navigation: a "← Back" button in the page header returns to the board. The sidebar "Full details →" button and clicking a job title in the table view navigate to this page.

### Form — new field groups

`JobForm` gets two new collapsible sections added below the existing fields:

**Contact & Location** (collapsed by default, expands on click)
- contact_name, contact_email, contact_phone
- workplace_street, workplace_city, workplace_postal_code

**Job Details** (collapsed by default)
- work_mode (select: Remote / Hybrid / On-site / Not specified)
- contract_type (select: Permanent / Fixed-term / Freelance / Internship / Not specified)
- salary_range (text input)
- priority (1–3 star toggle, manual only — not in AI merge)
- reference_number (text input)
- source (text input)

After "Extract with AI", all extractable fields in these sections are populated automatically.

---

## 4. Routing

The app currently has no routing beyond the dashboard. Add `react-router-dom` routes (already installed):

- `/` → `DashboardPage` (existing)
- `/job/:id` → `JobDetailPage` (new)

`AppHeader` or `DashboardPage` passes the selected job's id when navigating to the detail page.

---

## 5. Non-goals

- No PDF preview inside the app (open with system viewer only)
- No map/geocoding for the workplace address
- No AI extraction of priority (always manual)
- No changes to export/import format in this iteration (CSV/JSON export stays as-is; new fields are just omitted from existing exports until a separate export update)

---

## Implementation order

1. DB migration + Rust structs (foundation for everything)
2. TypeScript types + tauriApi additions
3. `open_document` Tauri command
4. Extraction prompt + normalizer updates
5. `lucide-react` install + emoji replacement
6. `JobForm` new sections
7. Sidebar slim-down
8. `JobDetailPage` + routing
