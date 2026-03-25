# Job Detail Enrichment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 12 new job fields (contact, location, job details), PDF open button, outline icons (Lucide), and a full-page job detail view.

**Architecture:** DB migration extends the `jobs` table non-destructively; Rust and TypeScript types are updated in lockstep; the existing sidebar slims to a quick-overview; a new `/job/:id` page shows all rich detail; AI extraction prompt is extended to populate the new fields automatically.

**Tech Stack:** Tauri 2 + Rust (rusqlite, `open` crate), React 19 + TypeScript (Vite, react-router-dom v7, lucide-react), vitest, cargo test

---

## File map

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/src/db.rs` | Modify | Migration, Rust structs, SQL queries, `open_document` command |
| `src-tauri/src/lib.rs` | Modify | Register `open_document` handler |
| `src/lib/types.ts` | Modify | `Job` / `NewJob` TS types |
| `src/lib/db/schema.ts` | Modify | Schema documentation mirror |
| `src/lib/tauriApi.ts` | Modify | `openDocument` frontend binding |
| `src/features/extraction/extractJobInfo.ts` | Modify | Extraction prompt + normalizer |
| `src/features/extraction/extractJobInfo.test.ts` | Modify | Tests for new field extraction |
| `src/i18n/en.ts` | Modify | All new UI labels |
| `src/features/jobs/JobForm.tsx` | Modify | New collapsible sections, `jobToNewJob`, `MERGEABLE_JOB_FIELDS` |
| `src/features/jobs/JobDetailTimeline.tsx` | Modify | Slim sidebar, remove upload/edit props, add `onViewDetails` |
| `src/pages/DashboardPage.tsx` | Modify | Wire `onViewDetails` + `navigate` |
| `src/pages/JobDetailPage.tsx` | Create | Full-page job detail view |
| `src/App.tsx` | Modify | Add `/job/:id` route |

---

## Task 1: DB migration + Rust structs

**Files:**
- Modify: `src-tauri/src/db.rs`

### Step 1a: Add 12 columns to `migrate_jobs_columns`

- [ ] **Step 1: Extend `migrate_jobs_columns`**

In `src-tauri/src/db.rs`, replace the body of `migrate_jobs_columns` (currently lines 136–156) with:

```rust
fn migrate_jobs_columns(conn: &Connection) -> Result<(), String> {
  let mut stmt = conn
    .prepare("PRAGMA table_info(jobs)")
    .map_err(|e| e.to_string())?;
  let cols: Vec<String> = stmt
    .query_map([], |row| row.get::<_, String>(1))
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

  let new_cols: &[(&str, &str)] = &[
    ("interview_date", "TEXT"),
    ("start_date", "TEXT"),
    ("contact_name", "TEXT"),
    ("contact_email", "TEXT"),
    ("contact_phone", "TEXT"),
    ("workplace_street", "TEXT"),
    ("workplace_city", "TEXT"),
    ("workplace_postal_code", "TEXT"),
    ("work_mode", "TEXT"),
    ("salary_range", "TEXT"),
    ("contract_type", "TEXT"),
    ("priority", "INTEGER"),
    ("reference_number", "TEXT"),
    ("source", "TEXT"),
  ];
  for (col, col_type) in new_cols {
    if !cols.iter().any(|c| c == col) {
      conn
        .execute(&format!("ALTER TABLE jobs ADD COLUMN {col} {col_type}"), [])
        .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}
```

- [ ] **Step 2: Update `Job` struct** — add 12 new fields after `notes`:

```rust
pub contact_name: Option<String>,
pub contact_email: Option<String>,
pub contact_phone: Option<String>,
pub workplace_street: Option<String>,
pub workplace_city: Option<String>,
pub workplace_postal_code: Option<String>,
pub work_mode: Option<String>,
pub salary_range: Option<String>,
pub contract_type: Option<String>,
pub priority: Option<i64>,
pub reference_number: Option<String>,
pub source: Option<String>,
```

- [ ] **Step 3: Update `NewJob` struct** — add same 12 fields after `notes` (same types). `priority` is `Option<i64>` here too — it is manually set, not AI-extracted, but the form can write it.

- [ ] **Step 4: Update `SQL_INSERT_JOB` and `insert_new_job`**

Replace `SQL_INSERT_JOB`:
```rust
const SQL_INSERT_JOB: &str = r#"
INSERT INTO jobs (company, title, url, raw_text, status, deadline, interview_date, start_date, tags,
  detected_language, notes, contact_name, contact_email, contact_phone,
  workplace_street, workplace_city, workplace_postal_code,
  work_mode, salary_range, contract_type, priority, reference_number, source,
  created_at, updated_at)
VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)
"#;
```

Update `insert_new_job` params to add after `&payload.notes`:
```rust
&payload.contact_name,
&payload.contact_email,
&payload.contact_phone,
&payload.workplace_street,
&payload.workplace_city,
&payload.workplace_postal_code,
&payload.work_mode,
&payload.salary_range,
&payload.contract_type,
&payload.priority,
&payload.reference_number,
&payload.source,
```

- [ ] **Step 5: Update `list_jobs` SELECT and row mapping**

Add to the SELECT column list (after `notes`): `contact_name, contact_email, contact_phone, workplace_street, workplace_city, workplace_postal_code, work_mode, salary_range, contract_type, priority, reference_number, source`

Update the `Ok(Job { ... })` closure — columns shift: `pdf_path` is now index 25, `created_at` 26, `updated_at` 27. Map the 12 new fields from indices 12–23:
```rust
contact_name: row.get(12)?,
contact_email: row.get(13)?,
contact_phone: row.get(14)?,
workplace_street: row.get(15)?,
workplace_city: row.get(16)?,
workplace_postal_code: row.get(17)?,
work_mode: row.get(18)?,
salary_range: row.get(19)?,
contract_type: row.get(20)?,
priority: row.get(21)?,
reference_number: row.get(22)?,
source: row.get(23)?,
pdf_path: row.get(24)?,
created_at: row.get(25)?,
updated_at: row.get(26)?,
```

- [ ] **Step 6: Update `update_job` SET clause**

Extend the UPDATE SQL (after `notes = ?11`) to include the 12 new fields. Shift `updated_at` and `WHERE id` parameters accordingly:
```rust
"UPDATE jobs SET company=?1, title=?2, url=?3, raw_text=?4, status=?5,
  deadline=?6, interview_date=?7, start_date=?8, tags=?9,
  detected_language=?10, notes=?11,
  contact_name=?12, contact_email=?13, contact_phone=?14,
  workplace_street=?15, workplace_city=?16, workplace_postal_code=?17,
  work_mode=?18, salary_range=?19, contract_type=?20, priority=?21,
  reference_number=?22, source=?23,
  updated_at=?24
 WHERE id=?25"
```

Add the 12 new payload fields to `params!` between `payload.notes` and `now`.

- [ ] **Step 7: Update `sample_new_job` test helper** — add the 12 new `None` fields so the existing test still compiles:

```rust
fn sample_new_job(company: &str) -> NewJob {
  NewJob {
    company: company.to_string(),
    title: None, url: None, raw_text: None,
    status: "Interesting".to_string(),
    deadline: None, interview_date: None, start_date: None,
    tags: None, detected_language: None, notes: None,
    contact_name: None, contact_email: None, contact_phone: None,
    workplace_street: None, workplace_city: None, workplace_postal_code: None,
    work_mode: None, salary_range: None, contract_type: None,
    priority: None, reference_number: None, source: None,
  }
}
```

- [ ] **Step 8: Run Rust tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```
Expected: all 6 tests pass.

- [ ] **Step 9: Run Rust clippy**

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```
Expected: no errors.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): add 12 new job fields with non-destructive migration"
```

---

## Task 2: TypeScript types + schema mirror

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/db/schema.ts`

- [ ] **Step 1: Add 12 fields to `Job` type** in `src/lib/types.ts` after `notes`:

```ts
contact_name?: string | null;
contact_email?: string | null;
contact_phone?: string | null;
workplace_street?: string | null;
workplace_city?: string | null;
workplace_postal_code?: string | null;
work_mode?: string | null;
salary_range?: string | null;
contract_type?: string | null;
priority?: number | null;
reference_number?: string | null;
source?: string | null;
```

- [ ] **Step 2: Add 12 fields to `NewJob` type** (same names, all `?: string` except `priority?: number`):

```ts
contact_name?: string;
contact_email?: string;
contact_phone?: string;
workplace_street?: string;
workplace_city?: string;
workplace_postal_code?: string;
work_mode?: string;
salary_range?: string;
contract_type?: string;
priority?: number;
reference_number?: string;
source?: string;
```

- [ ] **Step 3: Update `src/lib/db/schema.ts`** — replace the `jobs` array to include all columns (add the 12 new ones plus the two missing pre-existing ones `interview_date` and `start_date`):

```ts
jobs: [
  "id", "company", "title", "url", "raw_text", "status",
  "deadline", "interview_date", "start_date",
  "tags", "detected_language", "notes",
  "contact_name", "contact_email", "contact_phone",
  "workplace_street", "workplace_city", "workplace_postal_code",
  "work_mode", "salary_range", "contract_type",
  "priority", "reference_number", "source",
  "pdf_path", "created_at", "updated_at",
],
```

- [ ] **Step 4: Run frontend tests**

```bash
npm test
```
Expected: 34 tests pass (type errors would manifest as compile failures in vitest).

- [ ] **Step 5: Commit**

```bash
git add src/lib/types.ts src/lib/db/schema.ts
git commit -m "feat(types): add 12 new job fields to Job, NewJob, schema mirror"
```

---

## Task 3: `open_document` Tauri command

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauriApi.ts`

- [ ] **Step 1: Add `open_document` command to `db.rs`** — append after the last command:

```rust
#[tauri::command]
pub fn open_document(path: String) -> Result<(), String> {
  open::that(&path).map_err(|e| format!("Could not open file: {e}"))
}
```

- [ ] **Step 2: Register in `lib.rs`** — add `db::open_document` to the `invoke_handler!` list.

- [ ] **Step 3: Add frontend binding** in `src/lib/tauriApi.ts`:

```ts
export async function openDocument(path: string): Promise<void> {
  return invoke("open_document", { path });
}
```

- [ ] **Step 4: Run clippy + tests**

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets && cargo test --manifest-path src-tauri/Cargo.toml
```
Expected: no errors, all 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs src/lib/tauriApi.ts
git commit -m "feat: add open_document Tauri command to open files with system default app"
```

---

## Task 4: Extraction prompt + normalizer + form field wiring

**Files:**
- Modify: `src/features/extraction/extractJobInfo.ts`
- Modify: `src/features/extraction/extractJobInfo.test.ts`
- Modify: `src/features/jobs/JobForm.tsx`

- [ ] **Step 1: Write failing tests for new field extraction** in `extractJobInfo.test.ts` — add to the `normalizeLlmJobPartial` describe block:

```ts
it("maps new contact and location fields", () => {
  expect(
    normalizeLlmJobPartial({
      contact_name: "Jana Hansen",
      contact_email: "jana@acme.dk",
      contact_phone: "+45 12 34 56 78",
      workplace_street: "Nørrebrogade 10",
      workplace_city: "Copenhagen",
      workplace_postal_code: "2200",
    }),
  ).toEqual({
    contact_name: "Jana Hansen",
    contact_email: "jana@acme.dk",
    contact_phone: "+45 12 34 56 78",
    workplace_street: "Nørrebrogade 10",
    workplace_city: "Copenhagen",
    workplace_postal_code: "2200",
  });
});

it("maps work_mode, salary_range, contract_type, reference_number, source", () => {
  expect(
    normalizeLlmJobPartial({
      work_mode: "Remote",
      salary_range: "65-75k DKK/mo",
      contract_type: "Permanent",
      reference_number: "JOB-2024-112",
      source: "LinkedIn",
    }),
  ).toEqual({
    work_mode: "Remote",
    salary_range: "65-75k DKK/mo",
    contract_type: "Permanent",
    reference_number: "JOB-2024-112",
    source: "LinkedIn",
  });
});

it("does not map priority (manual only)", () => {
  const result = normalizeLlmJobPartial({ priority: 2, company: "Acme" });
  expect(result).not.toHaveProperty("priority");
  expect(result.company).toBe("Acme");
});
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
npm test -- --reporter=verbose src/features/extraction/extractJobInfo.test.ts
```
Expected: the 3 new tests FAIL.

- [ ] **Step 3: Update `buildExtractionPrompt`** in `extractJobInfo.ts` — replace the `Return strict JSON with keys:` line:

```ts
return `Extract structured job information from the text below.
Input can be Danish, German, or English.
Return strict JSON with keys:
company, title, url, deadline, interview_date, start_date, tags, detected_language, notes,
contact_name, contact_email, contact_phone,
workplace_street, workplace_city, workplace_postal_code,
work_mode (one of: Remote, Hybrid, On-site, or omit if unknown),
salary_range (free text as stated in the ad, or omit if not mentioned),
contract_type (one of: Permanent, Fixed-term, Freelance, Internship, or omit if unknown),
reference_number (job reference/ID if mentioned, else omit),
source (where the job was posted if mentioned, else omit)
Dates as YYYY-MM-DD when known.
Use English field values when possible for normalized output.

Text:
${rawText}`;
```

- [ ] **Step 4: Update `normalizeLlmJobPartial`** — add after the `notes` mapping:

```ts
const contact_name = strField(raw, ["contact_name", "contactName", "contact"]);
if (contact_name) out.contact_name = contact_name;
const contact_email = strField(raw, ["contact_email", "contactEmail", "email"]);
if (contact_email) out.contact_email = contact_email;
const contact_phone = strField(raw, ["contact_phone", "contactPhone", "phone"]);
if (contact_phone) out.contact_phone = contact_phone;
const workplace_street = strField(raw, ["workplace_street", "street", "address"]);
if (workplace_street) out.workplace_street = workplace_street;
const workplace_city = strField(raw, ["workplace_city", "city"]);
if (workplace_city) out.workplace_city = workplace_city;
const workplace_postal_code = strField(raw, ["workplace_postal_code", "postalCode", "postal_code", "zip"]);
if (workplace_postal_code) out.workplace_postal_code = workplace_postal_code;
const work_mode = strField(raw, ["work_mode", "workMode", "remote", "location_type"]);
if (work_mode) out.work_mode = work_mode;
const salary_range = strField(raw, ["salary_range", "salaryRange", "salary", "compensation"]);
if (salary_range) out.salary_range = salary_range;
const contract_type = strField(raw, ["contract_type", "contractType", "employment_type", "contract"]);
if (contract_type) out.contract_type = contract_type;
// priority is intentionally excluded — manual only, never set from LLM output
const reference_number = strField(raw, ["reference_number", "referenceNumber", "ref", "job_ref", "job_id"]);
if (reference_number) out.reference_number = reference_number;
const source = strField(raw, ["source", "Source", "job_source", "platform"]);
if (source) out.source = source;
```

- [ ] **Step 5: Update `jobToNewJob` in `JobForm.tsx`** — add after `notes: j.notes ?? undefined`:

```ts
contact_name: j.contact_name ?? undefined,
contact_email: j.contact_email ?? undefined,
contact_phone: j.contact_phone ?? undefined,
workplace_street: j.workplace_street ?? undefined,
workplace_city: j.workplace_city ?? undefined,
workplace_postal_code: j.workplace_postal_code ?? undefined,
work_mode: j.work_mode ?? undefined,
salary_range: j.salary_range ?? undefined,
contract_type: j.contract_type ?? undefined,
priority: j.priority ?? undefined,
reference_number: j.reference_number ?? undefined,
source: j.source ?? undefined,
```

- [ ] **Step 6: Update `MERGEABLE_JOB_FIELDS` in `JobForm.tsx`** — add the 11 AI-extractable fields. Do **not** add `priority`:

```ts
const MERGEABLE_JOB_FIELDS: (keyof NewJob)[] = [
  "company", "title", "url", "raw_text",
  "deadline", "interview_date", "start_date",
  "tags", "detected_language", "notes",
  "contact_name", "contact_email", "contact_phone",
  "workplace_street", "workplace_city", "workplace_postal_code",
  "work_mode", "salary_range", "contract_type",
  "reference_number", "source",
];
```

- [ ] **Step 7: Run tests — verify they pass**

```bash
npm test
```
Expected: all 37 tests pass (34 existing + 3 new).

- [ ] **Step 8: Commit**

```bash
git add src/features/extraction/extractJobInfo.ts src/features/extraction/extractJobInfo.test.ts src/features/jobs/JobForm.tsx
git commit -m "feat(extraction): extract 11 new job fields; update form field wiring"
```

---

## Task 5: i18n labels

**Files:**
- Modify: `src/i18n/en.ts`

- [ ] **Step 1: Add new strings to `en.ts`**

Add a `jobDetail` section and extend `jobForm` and `detail`:

In the `jobForm` section, add after `saveFailed`:
```ts
// New field sections
contactSectionTitle: "Contact & Location",
jobDetailsSectionTitle: "Job Details",
contactNamePh: "Contact name",
contactEmailPh: "Contact email",
contactPhonePh: "Contact phone",
workplaceStreetPh: "Street",
workplaceCityPh: "City",
workplacePostalCodePh: "Postal code",
workModePh: "Work mode",
workModeRemote: "Remote",
workModeHybrid: "Hybrid",
workModeOnSite: "On-site",
workModeUnknown: "Not specified",
salaryRangePh: "Salary range",
contractTypePh: "Contract type",
contractTypePermanent: "Permanent",
contractTypeFixedTerm: "Fixed-term",
contractTypeFreelance: "Freelance",
contractTypeInternship: "Internship",
contractTypeUnknown: "Not specified",
priorityLabel: "Priority",
referenceNumberPh: "Reference number",
sourcePh: "Source (LinkedIn, company site…)",
```

Add a new `jobDetailPage` section at the top level:
```ts
jobDetailPage: {
  back: "← Back",
  editJob: "Edit",
  deleteJob: "Delete",
  sectionOverview: "Overview",
  sectionContact: "Contact & Location",
  sectionDocuments: "Documents & History",
  status: "Status",
  priority: "Priority",
  workMode: "Work mode",
  contractType: "Contract type",
  salaryRange: "Salary",
  deadline: "Apply by",
  interview: "Interview",
  start: "Start",
  tags: "Tags",
  referenceNumber: "Reference",
  source: "Source",
  language: "Language",
  contactName: "Contact",
  contactEmail: "Email",
  contactPhone: "Phone",
  address: "Address",
  openDocument: "Open",
  deleteDocument: "Remove",
  uploadDocument: "Upload document",
  history: "History",
  noDocuments: "No documents uploaded.",
  notSet: "—",
},
```

In the `detail` section, add:
```ts
viewFullDetails: "Full details",
```

- [ ] **Step 2: Run tests**

```bash
npm test
```
Expected: all 37 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/i18n/en.ts
git commit -m "feat(i18n): add labels for new job fields and detail page"
```

---

## Task 6: Install Lucide + replace emoji

**Files:**
- All component files that use emoji icons

- [ ] **Step 1: Install lucide-react**

```bash
npm install lucide-react
```

- [ ] **Step 2: Run tests to confirm nothing broke**

```bash
npm test
```
Expected: 37 tests pass.

- [ ] **Step 3: Replace emoji in `JobDetailTimeline.tsx`**

The `→` arrow in history items uses a text character — replace with the string `"→"` (keep as-is, it's a text arrow not an icon). No emoji in this file currently to replace. Skip.

- [ ] **Step 4: Check all TSX files for emoji**

```bash
grep -rn "📅\|📄\|🏢\|🏠\|🏷\|📌\|★\|☆\|→\|←\|↗" src/
```

Replace any found instances with Lucide components. Common substitutions (import from `lucide-react`):
- `📅` / dates → `<Calendar size={14} />`
- `📄` / document → `<FileText size={14} />`
- `🏢` / company/building → `<Building2 size={14} />`
- `🏠` / location → `<MapPin size={14} />`
- `🏷` / tag → `<Tag size={14} />`

- [ ] **Step 5: Commit**

```bash
git add package.json package-lock.json src/
git commit -m "feat(ui): install lucide-react; replace emoji with outline icons"
```

---

## Task 7: JobForm — new collapsible sections

**Files:**
- Modify: `src/features/jobs/JobForm.tsx`

- [ ] **Step 1: Add collapsed state for the two new sections**

In `JobForm`, add two state variables after the existing `useState` calls:
```tsx
const [contactOpen, setContactOpen] = useState(false);
const [jobDetailsOpen, setJobDetailsOpen] = useState(false);
```

- [ ] **Step 2: Add Contact & Location section** — insert after the notes textarea and before the extract error:

```tsx
<div className="fieldFull">
  <button
    type="button"
    className="btn btnGhost sectionToggle"
    onClick={() => setContactOpen((v) => !v)}
  >
    {contactOpen ? "▾" : "▸"} {en.jobForm.contactSectionTitle}
  </button>
  {contactOpen && (
    <div className="grid" style={{ marginTop: "0.5rem" }}>
      <input placeholder={en.jobForm.contactNamePh} value={form.contact_name ?? ""} onChange={(e) => update({ contact_name: e.target.value })} />
      <input placeholder={en.jobForm.contactEmailPh} value={form.contact_email ?? ""} onChange={(e) => update({ contact_email: e.target.value })} />
      <input placeholder={en.jobForm.contactPhonePh} value={form.contact_phone ?? ""} onChange={(e) => update({ contact_phone: e.target.value })} />
      <input placeholder={en.jobForm.workplaceStreetPh} value={form.workplace_street ?? ""} onChange={(e) => update({ workplace_street: e.target.value })} />
      <input placeholder={en.jobForm.workplaceCityPh} value={form.workplace_city ?? ""} onChange={(e) => update({ workplace_city: e.target.value })} />
      <input placeholder={en.jobForm.workplacePostalCodePh} value={form.workplace_postal_code ?? ""} onChange={(e) => update({ workplace_postal_code: e.target.value })} />
    </div>
  )}
</div>
```

- [ ] **Step 3: Add Job Details section** — insert after the Contact section:

```tsx
<div className="fieldFull">
  <button
    type="button"
    className="btn btnGhost sectionToggle"
    onClick={() => setJobDetailsOpen((v) => !v)}
  >
    {jobDetailsOpen ? "▾" : "▸"} {en.jobForm.jobDetailsSectionTitle}
  </button>
  {jobDetailsOpen && (
    <div className="grid" style={{ marginTop: "0.5rem" }}>
      <select value={form.work_mode ?? ""} onChange={(e) => update({ work_mode: e.target.value || undefined })}>
        <option value="">{en.jobForm.workModeUnknown}</option>
        <option value="Remote">{en.jobForm.workModeRemote}</option>
        <option value="Hybrid">{en.jobForm.workModeHybrid}</option>
        <option value="On-site">{en.jobForm.workModeOnSite}</option>
      </select>
      <select value={form.contract_type ?? ""} onChange={(e) => update({ contract_type: e.target.value || undefined })}>
        <option value="">{en.jobForm.contractTypeUnknown}</option>
        <option value="Permanent">{en.jobForm.contractTypePermanent}</option>
        <option value="Fixed-term">{en.jobForm.contractTypeFixedTerm}</option>
        <option value="Freelance">{en.jobForm.contractTypeFreelance}</option>
        <option value="Internship">{en.jobForm.contractTypeInternship}</option>
      </select>
      <input placeholder={en.jobForm.salaryRangePh} value={form.salary_range ?? ""} onChange={(e) => update({ salary_range: e.target.value })} />
      <input placeholder={en.jobForm.referenceNumberPh} value={form.reference_number ?? ""} onChange={(e) => update({ reference_number: e.target.value })} />
      <input placeholder={en.jobForm.sourcePh} value={form.source ?? ""} onChange={(e) => update({ source: e.target.value })} />
      <div>
        <span className="fieldLabelText">{en.jobForm.priorityLabel}</span>
        <div className="row" style={{ gap: "0.25rem", marginTop: "0.25rem" }}>
          {[1, 2, 3].map((n) => (
            <button
              key={n}
              type="button"
              className="btn btnGhost btnSm"
              onClick={() => update({ priority: form.priority === n ? undefined : n })}
              aria-pressed={form.priority === n}
              style={{ padding: "0.2rem" }}
            >
              <Star
                size={16}
                fill={form.priority != null && form.priority >= n ? "currentColor" : "none"}
              />
            </button>
          ))}
        </div>
      </div>
    </div>
  )}
</div>
```

Add `import { Star } from "lucide-react";` at the top of `JobForm.tsx`.

- [ ] **Step 4: Auto-expand sections after extraction** — in the `extract()` function, after `setForm(...)`, add:

```ts
if (result.partial.contact_name || result.partial.contact_email || result.partial.workplace_city) {
  setContactOpen(true);
}
if (result.partial.work_mode || result.partial.salary_range || result.partial.contract_type) {
  setJobDetailsOpen(true);
}
```

- [ ] **Step 5: Run tests + build**

```bash
npm test && npm run build
```
Expected: 37 tests pass, build succeeds.

- [ ] **Step 6: Commit**

```bash
git add src/features/jobs/JobForm.tsx
git commit -m "feat(form): add collapsible Contact & Location and Job Details sections"
```

---

## Task 8: Sidebar slim-down

**Files:**
- Modify: `src/features/jobs/JobDetailTimeline.tsx`
- Modify: `src/pages/DashboardPage.tsx`

- [ ] **Step 1: Update `Props` in `JobDetailTimeline.tsx`**

Replace the Props type:
```tsx
type Props = {
  selected?: Job;
  onDeleteJob: (jobId: number) => Promise<void>;
  onViewDetails: (jobId: number) => void;
};
```

Remove all imports and state that are no longer needed: `JobForm`, `DocType`, `JobDocument`, `deleteJobDocument`, `listJobDocuments`, `saveJobDocument`, `onSavedPdf`, `onExtract`, `onUpdateJob`, `statuses`, `documents`, `uploadDocType` state, `onUploadDocument`, `onDeleteDocument`, `editing` state.

Keep only: `history` state + `listStatusHistory` (for the brief history preview, or remove entirely from sidebar — simplest is to remove it too). Remove the history list from the sidebar view as well to keep it slim.

- [ ] **Step 2: Rewrite the component body**

The new sidebar shows: company + title heading, status + priority stars, key dates (deadline, interview, start), work mode + contract type, tags, "Full details" button, and delete button.

```tsx
export const JobDetailTimeline = memo(function JobDetailTimeline({
  selected,
  onDeleteJob,
  onViewDetails,
}: Props) {
  async function onDelete() {
    if (!selected) return;
    if (!window.confirm(en.alerts.deleteJobConfirm)) return;
    try {
      await onDeleteJob(selected.id);
    } catch (e) {
      window.alert(en.alerts.deleteJobFailed(e instanceof Error ? e.message : String(e)));
    }
  }

  if (!selected) {
    return (
      <section className="card">
        <h2>{en.detail.title}</h2>
        <p className="muted">{en.detail.selectJob}</p>
      </section>
    );
  }

  return (
    <section className="card">
      <h2>{en.detail.title}</h2>
      <p className="detailMeta">
        <strong>{selected.company}</strong>
        {selected.title ? ` — ${selected.title}` : ""}
      </p>
      <p className="detailMeta">
        {en.detail.status} <span>{selected.status}</span>
        {selected.priority != null && (
          <span style={{ marginLeft: "0.5rem" }}>
            {[1, 2, 3].map((n) => (
              <Star key={n} size={13} fill={selected.priority! >= n ? "currentColor" : "none"} style={{ display: "inline" }} />
            ))}
          </span>
        )}
      </p>
      {(selected.deadline || selected.interview_date || selected.start_date) && (
        <p className="detailMeta detailDates">
          {selected.deadline && <><Calendar size={13} style={{ display: "inline", marginRight: 3 }} />{en.detail.deadlineShort}: <span>{selected.deadline}</span></>}
          {selected.interview_date && <>{selected.deadline ? " · " : null}{en.detail.interviewShort}: <span>{selected.interview_date}</span></>}
          {selected.start_date && <>{(selected.deadline || selected.interview_date) ? " · " : null}{en.detail.startShort}: <span>{selected.start_date}</span></>}
        </p>
      )}
      {(selected.work_mode || selected.contract_type) && (
        <p className="detailMeta">
          <Monitor size={13} style={{ display: "inline", marginRight: 3 }} />
          {[selected.work_mode, selected.contract_type].filter(Boolean).join(" · ")}
        </p>
      )}
      {selected.tags && (
        <p className="detailMeta">
          <Tag size={13} style={{ display: "inline", marginRight: 3 }} />
          {selected.tags}
        </p>
      )}
      <div className="detailActions row" style={{ marginTop: "0.75rem" }}>
        <button type="button" className="btn btnPrimary btnSm" onClick={() => onViewDetails(selected.id)}>
          {en.detail.viewFullDetails} <ArrowRight size={13} style={{ display: "inline", marginLeft: 3 }} />
        </button>
        <button type="button" className="btn btnDanger btnSm" onClick={() => void onDelete()}>
          <Trash2 size={13} />
        </button>
      </div>
    </section>
  );
});
```

Add imports: `import { Star, Calendar, Monitor, Tag, ArrowRight, Trash2 } from "lucide-react";`

- [ ] **Step 3: Update `DashboardPage.tsx`**

Remove `onUpdateJob`, `onExtract`, `syncJobList`, `runBackup` from the destructure (they are still used elsewhere — only remove from the `JobDetailTimeline` props).

Add `useNavigate` from react-router-dom and `onViewDetails`:
```tsx
import { useNavigate } from "react-router-dom";
// inside DashboardPage:
const navigate = useNavigate();
```

Update the `<JobDetailTimeline>` JSX:
```tsx
<JobDetailTimeline
  selected={selected}
  onDeleteJob={onDeleteJob}
  onViewDetails={(id) => navigate(`/job/${id}`)}
/>
```

Also remove `onSavedPdf` from the `<JobDetailTimeline>` call — it no longer exists in Props. Keep `syncJobList` and `runBackup` for other uses in the page.

- [ ] **Step 4: Run build to catch type errors**

```bash
npm run build
```
Expected: build succeeds with no TypeScript errors.

- [ ] **Step 5: Commit**

```bash
git add src/features/jobs/JobDetailTimeline.tsx src/pages/DashboardPage.tsx
git commit -m "feat(sidebar): slim down to quick overview; add Full details navigation"
```

---

## Task 9: JobDetailPage + route

**Files:**
- Create: `src/pages/JobDetailPage.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create `JobDetailPage.tsx`**

```tsx
import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  ArrowLeft, Building2, Calendar, ExternalLink, FileText,
  Layers, MapPin, Monitor, Pencil, Star, Tag, Trash2,
} from "lucide-react";
import { useJobTracker } from "../context/JobTrackerContext";
import {
  deleteJobDocument, listJobDocuments, listStatusHistory,
  openDocument, saveJobDocument,
} from "../lib/tauriApi";
import { JobForm } from "../features/jobs/JobForm";
import { en } from "../i18n/en";
import type { DocType, JobDocument } from "../lib/types";

const DOC_TYPES: { value: DocType; label: string }[] = [
  { value: "cv", label: en.detail.docTypeCv },
  { value: "cover_letter", label: en.detail.docTypeCoverLetter },
  { value: "other", label: en.detail.docTypeOther },
];

function WorkModeIcon({ mode }: { mode?: string | null }) {
  if (mode === "Remote") return <Monitor size={14} />;
  if (mode === "On-site") return <Building2 size={14} />;
  if (mode === "Hybrid") return <Layers size={14} />;
  return null;
}

export function JobDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const {
    jobs, onDeleteJob, onUpdateJob, onExtract, statuses, syncJobList, runBackup,
  } = useJobTracker();

  const job = jobs.find((j) => j.id === Number(id));
  const [documents, setDocuments] = useState<JobDocument[]>([]);
  const [history, setHistory] = useState<Array<{ from_status: string | null; to_status: string; changed_at: string }>>([]);
  const [editing, setEditing] = useState(false);
  const [uploadDocType, setUploadDocType] = useState<DocType>("cv");

  useEffect(() => {
    if (!job) return;
    void listJobDocuments(job.id).then(setDocuments);
    void listStatusHistory(job.id).then(setHistory);
  }, [job?.id]);

  if (!job) {
    return (
      <div className="card" style={{ margin: "2rem auto", maxWidth: 480 }}>
        <p className="muted">Job not found.</p>
        <button className="btn btnGhost" onClick={() => navigate("/")}>
          <ArrowLeft size={14} style={{ marginRight: 4 }} /> {en.jobDetailPage.back}
        </button>
      </div>
    );
  }

  async function handleUpload(file?: File | null) {
    if (!file || !job) return;
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    const doc = await saveJobDocument(job.id, uploadDocType, file.name, bytes);
    setDocuments((prev) => [...prev, doc]);
    await syncJobList();
    runBackup();
  }

  async function handleDeleteDocument(docId: number) {
    await deleteJobDocument(docId);
    setDocuments((prev) => prev.filter((d) => d.id !== docId));
  }

  async function handleDeleteJob() {
    if (!job) return;
    if (!window.confirm(en.alerts.deleteJobConfirm)) return;
    try {
      await onDeleteJob(job.id);
      navigate("/");
    } catch (e) {
      window.alert(en.alerts.deleteJobFailed(e instanceof Error ? e.message : String(e)));
    }
  }

  const row = (label: string, value?: string | number | null) =>
    value != null && value !== "" ? (
      <div className="detailRow">
        <span className="detailRowLabel">{label}</span>
        <span>{String(value)}</span>
      </div>
    ) : null;

  return (
    <div className="jobDetailPage">
      <div className="jobDetailPageHeader">
        <button className="btn btnGhost btnSm" onClick={() => navigate(-1)}>
          <ArrowLeft size={14} style={{ marginRight: 4 }} />{en.jobDetailPage.back}
        </button>
        <h1>{job.company}{job.title ? ` — ${job.title}` : ""}</h1>
        <div className="row" style={{ gap: "0.5rem" }}>
          <button className="btn btnGhost btnSm" onClick={() => setEditing((v) => !v)}>
            <Pencil size={13} style={{ marginRight: 3 }} />{en.jobDetailPage.editJob}
          </button>
          <button className="btn btnDanger btnSm" onClick={() => void handleDeleteJob()}>
            <Trash2 size={13} />
          </button>
        </div>
      </div>

      {editing && (
        <div style={{ marginBottom: "1.5rem" }}>
          <JobForm
            key={job.id}
            statuses={statuses}
            editingJob={job}
            onSubmit={async () => false}
            onUpdateJob={async (jobId, payload) => {
              const ok = await onUpdateJob(jobId, payload);
              if (ok) setEditing(false);
              return ok;
            }}
            onEditClose={() => setEditing(false)}
            onExtract={onExtract}
            hideTitle
          />
        </div>
      )}

      <div className="jobDetailGrid">
        {/* Column 1 — Overview */}
        <section className="card">
          <p className="cardTitle">{en.jobDetailPage.sectionOverview}</p>
          <div className="detailRowList">
            {row(en.jobDetailPage.status, job.status)}
            {job.priority != null && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.priority}</span>
                <span>
                  {[1, 2, 3].map((n) => (
                    <Star key={n} size={14} fill={job.priority! >= n ? "currentColor" : "none"} style={{ display: "inline" }} />
                  ))}
                </span>
              </div>
            )}
            {job.work_mode && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.workMode}</span>
                <span><WorkModeIcon mode={job.work_mode} />{" "}{job.work_mode}</span>
              </div>
            )}
            {row(en.jobDetailPage.contractType, job.contract_type)}
            {row(en.jobDetailPage.salaryRange, job.salary_range)}
            {job.deadline && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.deadline}</span>
                <span><Calendar size={13} style={{ display: "inline", marginRight: 3 }} />{job.deadline}</span>
              </div>
            )}
            {row(en.jobDetailPage.interview, job.interview_date)}
            {row(en.jobDetailPage.start, job.start_date)}
            {job.tags && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.tags}</span>
                <span><Tag size={13} style={{ display: "inline", marginRight: 3 }} />{job.tags}</span>
              </div>
            )}
            {row(en.jobDetailPage.referenceNumber, job.reference_number)}
            {row(en.jobDetailPage.source, job.source)}
            {row(en.jobDetailPage.language, job.detected_language)}
          </div>
        </section>

        {/* Column 2 — Contact & Location */}
        <section className="card">
          <p className="cardTitle">{en.jobDetailPage.sectionContact}</p>
          <div className="detailRowList">
            {job.contact_name && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.contactName}</span>
                <span>{job.contact_name}</span>
              </div>
            )}
            {job.contact_email && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.contactEmail}</span>
                <a href={`mailto:${job.contact_email}`}>{job.contact_email}</a>
              </div>
            )}
            {job.contact_phone && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.contactPhone}</span>
                <a href={`tel:${job.contact_phone}`}>{job.contact_phone}</a>
              </div>
            )}
            {(job.workplace_street || job.workplace_city || job.workplace_postal_code) && (
              <div className="detailRow" style={{ alignItems: "flex-start" }}>
                <span className="detailRowLabel">{en.jobDetailPage.address}</span>
                <span>
                  <MapPin size={13} style={{ display: "inline", marginRight: 3 }} />
                  {[job.workplace_street, [job.workplace_postal_code, job.workplace_city].filter(Boolean).join(" ")].filter(Boolean).join(", ")}
                </span>
              </div>
            )}
          </div>
        </section>

        {/* Column 3 — Documents + History */}
        <section className="card">
          <p className="cardTitle">{en.jobDetailPage.sectionDocuments}</p>
          {documents.length === 0 ? (
            <p className="muted">{en.jobDetailPage.noDocuments}</p>
          ) : (
            <ul className="listPlain">
              {documents.map((doc) => {
                const label = DOC_TYPES.find((t) => t.value === doc.doc_type)?.label ?? doc.doc_type;
                return (
                  <li key={doc.id} className="row" style={{ gap: "0.5rem", alignItems: "center" }}>
                    <FileText size={14} style={{ opacity: 0.5, flexShrink: 0 }} />
                    <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{doc.original_name}</span>
                    <span className="tag">{label}</span>
                    <button
                      type="button"
                      className="btn btnGhost btnSm"
                      onClick={() => void openDocument(doc.file_path)}
                    >
                      <ExternalLink size={13} style={{ marginRight: 3 }} />{en.jobDetailPage.openDocument}
                    </button>
                    <button
                      type="button"
                      className="btn btnDanger btnSm"
                      onClick={() => void handleDeleteDocument(doc.id)}
                    >
                      <Trash2 size={13} />
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
          <div className="row" style={{ gap: "0.5rem", marginTop: "0.5rem", flexWrap: "wrap" }}>
            <select value={uploadDocType} onChange={(e) => setUploadDocType(e.target.value as DocType)} className="input">
              {DOC_TYPES.map((t) => <option key={t.value} value={t.value}>{t.label}</option>)}
            </select>
            <label className="btn btnGhost btnSm fileImport fileImportBlock">
              <span>{en.jobDetailPage.uploadDocument}</span>
              <input type="file" accept="application/pdf" className="visuallyHidden" onChange={(e) => void handleUpload(e.target.files?.[0])} />
            </label>
          </div>

          <p className="cardTitle" style={{ marginTop: "1rem" }}>{en.jobDetailPage.history}</p>
          <ul className="listPlain historyList">
            {history.map((h, idx) => (
              <li key={`${h.changed_at}-${idx}`}>
                {h.from_status ?? en.detail.newStatus} {"→"} {h.to_status} ({new Date(h.changed_at).toLocaleString()})
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Add CSS for new layout classes** in `src/App.css` (or wherever global styles live). Search for the existing CSS file:

```bash
ls src/*.css
```

Add at the end of the CSS file:
```css
.jobDetailPage {
  padding: 1.5rem;
  max-width: 1200px;
  margin: 0 auto;
}

.jobDetailPageHeader {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1.5rem;
  flex-wrap: wrap;
}

.jobDetailPageHeader h1 {
  flex: 1;
  font-size: 1.25rem;
  margin: 0;
}

.jobDetailGrid {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 1rem;
}

@media (max-width: 900px) {
  .jobDetailGrid {
    grid-template-columns: 1fr;
  }
}

.detailRowList {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.detailRow {
  display: flex;
  gap: 0.5rem;
  font-size: 0.88rem;
  align-items: center;
}

.detailRowLabel {
  color: var(--text-muted, #888);
  min-width: 90px;
  flex-shrink: 0;
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}
```

- [ ] **Step 3: Add `/job/:id` route in `App.tsx`**

Add import: `import { JobDetailPage } from "./pages/JobDetailPage";`

Add route inside `<Routes>` before the `*` catch-all:
```tsx
<Route path="/job/:id" element={<JobDetailPage />} />
```

- [ ] **Step 4: Run full build**

```bash
npm run build
```
Expected: build succeeds, no TypeScript errors.

- [ ] **Step 5: Test BrowserRouter in production build**

```bash
npm run tauri:build 2>&1 | tail -20
```

Then open the built app and navigate to a job detail page, then refresh. If the page shows blank or a 404, add `"dangerouslyAllowBrowsersToAccessLocalhost": false` and test — if still broken, migrate to `HashRouter` by replacing `BrowserRouter` with `HashRouter` in `App.tsx` (import from `react-router-dom`).

- [ ] **Step 6: Run all tests**

```bash
npm test && cargo test --manifest-path src-tauri/Cargo.toml
```
Expected: 37 frontend tests, 6 Rust tests, all pass.

- [ ] **Step 7: Commit**

```bash
git add src/pages/JobDetailPage.tsx src/App.tsx src/App.css
git commit -m "feat: add full-page job detail view at /job/:id"
```

---

## Final verification

- [ ] Run `npm run verify` — all frontend lint, tests, build, Rust clippy/tests, and Python checks pass:

```bash
npm run verify
```

Expected output ends with `3 passed in 0.XXs`.
