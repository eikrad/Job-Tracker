# Mail Scan PR A — Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. **Do not start coding until the user approves this plan.**

**Status:** Active · PR 1 of 3 · [index](2026-09-09-mail-scan-integration.md)
**Goal:** Ship the security and persistence foundations of design rev. 2 (spec §12 phases 1–3): a versioned migration runner, API keys in the OS keyring, one Rust LLM client, and an SSRF-hardened fetch for untrusted URLs. Valuable on its own even if the mail pipeline is never built.
**Source:** Spec §6.1, §6.3, §7.1, §8 · ADR 0004 · ADR 0005

**Architecture:** Secrets and network egress move into Rust. The frontend sets and clears keys and learns only *whether* one is configured. Job-form extraction calls a Tauri command. Fetches of **untrusted** URLs go through one guarded helper; calls to **known API endpoints** (SerpAPI, Brave) keep their own client and their auth headers.

---

## Hazards found in the current codebase

These are why several tasks below are larger than they look. Each is verified against the code, not assumed.

| # | Hazard | Evidence | Handled in |
|---|--------|----------|------------|
| H1 | **WAL breaks the existing backup.** `backup_to_folder` does `fs::copy` of `data/app.db` only. Under WAL, recent commits live in `app.db-wal`; copying the main file alone yields a stale or unusable backup — a silent data-loss bug introduced *by* this PR. | `db.rs` `backup_to_folder` | A1 |
| H2 | **Keyring tests cannot run in CI.** `rust.yml` runs `cargo test` on `ubuntu-latest`, which has no D-Bus secret service. Tests that touch the real keyring fail the build. | `.github/workflows/rust.yml` | A2 |
| H3 | **Forcing SerpAPI/Brave through an SSRF wrapper breaks job search.** Those are trusted API endpoints called *with* auth headers (`X-Subscription-Token`, `api_key` query). A wrapper that strips auth and enforces a text-only content type is the wrong shape for them. | `job_search.rs`, `listing_check.rs` | A5 |
| H4 | **Search keys still cross the IPC boundary.** `fetch_job_search_results` and friends take `serp_api_key` / `brave_search_api_key` as command parameters from the frontend. Moving storage to the keyring without dropping these parameters leaves ADR 0005 unmet. | `job_search.rs` command signatures | A3, A5 |
| H5 | **`googleAccessToken` is also in `localStorage`** — a bearer token, at least as sensitive as an API key, and not in the original key list. | `useJobTrackerState.ts:66` | A2 (scope), A3 |
| H6 | **Browser-only dev mode loses AI extraction.** `npm run dev` is documented as "UI-only; no Tauri commands" — after A4 the Extract button has no backend there. | `CONTRIBUTING.md` | A4 |
| H7 | **A migration runner cannot tell a fresh DB from an existing one by `user_version` alone** — both read `0` today. Probing the schema is required, or every existing install re-runs baseline DDL. | `db.rs` `init_db` | A1 |

---

## File map

| Path | Role |
|------|------|
| `src-tauri/src/migrations.rs` (new) | Ordered `MIGRATIONS`, `user_version` driver, baseline detection |
| `src-tauri/src/db.rs` | Call the runner; per-connection pragmas; WAL-safe backup |
| `src/lib/db/schema.ts` | Mirror all `jobs` columns (currently missing `listing_status`, `listing_checked_at`) |
| `src-tauri/src/secrets.rs` (new) | `SecretStore` trait, keyring backend, `0600` file fallback, in-memory test backend, `redact()` |
| `src-tauri/src/llm/mod.rs`, `provider.rs`, `client.rs` (new) | Provider registry, HTTP client, `extract_job_info` command |
| `src-tauri/prompts/extract.md`, `prompts/schemas/extract.json` (new) | Prompt + response schema assets, `include_str!` |
| `src-tauri/src/net.rs` (new) | `fetch_untrusted` (guarded) and `api_client()` (trusted endpoints) |
| `src-tauri/src/listing_check.rs`, `job_search.rs` | Use the right one of the two; drop key parameters |
| `src/features/extraction/extractJobInfo.ts` | Keep `normalizeLlmJobPartial`; replace provider `fetch` calls with one `invoke` |
| `src/hooks/useJobTrackerState.ts`, `SettingsModal.tsx`, `tauriApi.ts`, `i18n/en.ts` | Key status UX, migration on launch, no secrets in state |

---

## Task A1 — Migration runner, pragmas, WAL-safe backup, schema mirror 🔴

**Produces:** `migrations::run(&mut Connection)`, `PRAGMA user_version` versioning, correct per-connection pragmas, a backup that is valid under WAL, and a test that stops schema-mirror drift.
**Commit:** `feat(db): versioned migration runner with WAL-safe backup`

- [ ] **Step 1 — red:** cargo test builds a fixture DB shaped like the current released schema (all `migrate_jobs_columns` columns present, `user_version = 0`), runs `migrations::run`, asserts `user_version` reaches the latest and no column is dropped. A second call is a no-op (idempotence).
- [ ] **Step 2 — red:** the same test with an **empty** file asserts a fresh DB reaches the identical schema. Compare the two resulting `PRAGMA table_info(jobs)` sets for equality — this is what proves H7 is handled.
- [ ] **Step 3 — red:** mirror test parses the column list out of `src/lib/db/schema.ts` and asserts every `PRAGMA table_info(jobs)` name appears. **This fails today** on `listing_status` and `listing_checked_at`; that is the point.
- [ ] **Step 4 — red (H1):** backup test — open the DB, insert a job, do **not** checkpoint, run the backup, open the copied file, assert the job is present. Fails once WAL is on and the backup still `fs::copy`s a single file.
- [ ] **Step 5 — implement:**
  - Pragmas, with the distinction made explicit in code comments: `journal_mode = WAL` is a **persistent database property** (set once, on the first connection); `busy_timeout = 5000`, `foreign_keys = ON`, `synchronous = NORMAL` are **per-connection** and must be set in `connection()` every time. `foreign_keys` matters from PR B onward, where cascades appear.
  - Baseline detection: if `user_version = 0`, probe for the `jobs` table. Present ⇒ stamp the baseline version without re-running DDL. Absent ⇒ run `m0001` fully. Never infer from `user_version` alone.
  - Each migration in its own transaction; forward-only; a failing step leaves `user_version` at the last good value.
  - Backup: replace `fs::copy` of `app.db` with `VACUUM INTO ?1` against the destination path. This is atomic, produces a single consistent file with no `-wal`/`-shm` sidecars, and needs no checkpoint. Keep the PDF copy loop as is.
- [ ] **Step 6:** update `schema.ts`; `cargo test` green; launch the app and confirm jobs still list, create, and update.
- [ ] **Step 7:** commit.

**Rollback:** WAL is reversible (`PRAGMA journal_mode = DELETE`) but the `-wal` file must be checkpointed first. Note that in the PR description; do not ship a downgrade path.

---

## Task A2 — Secrets module with a testable backend 🔴

**Produces:** Tauri commands `llm_key_set`, `llm_key_status`, `llm_key_clear`; a `SecretStore` abstraction; `redact()`.
**Commit:** `feat(secrets): store provider API keys in the OS keyring`

**Scope decision (H5):** covers `gemini`, `mistral`, `scaleway`, `serpapi`, `brave`, **and** `google_access_token`. Excluding a bearer token from a "no secrets in localStorage" change would leave the constraint half-met. The Google *refresh* token is already in the keyring (`google_oauth.rs`) — this aligns the access token with it.

- [ ] **Step 1 — red (H2):** define `trait SecretStore { fn set/get_status/clear }` with three impls: `KeyringStore`, `FileStore` (`0600` under app data), `MemoryStore`. **All unit tests use `MemoryStore`** so `cargo test` passes on a CI runner with no secret service. Add one `#[ignore]`d integration test that exercises the real keyring for local runs.
- [ ] **Step 2 — red:** `llm_key_status` returns `{ configured: bool, backend: "keyring" | "file" }` and *never* the secret. Assert the serialized JSON contains no key material — a type-level guarantee plus a test, because this is the whole point of ADR 0005.
- [ ] **Step 3 — red:** `redact()` — a 40-char token inside an error string becomes `***`; short non-secret text is untouched; the function is total (no panic on empty/unicode).
- [ ] **Step 4 — implement:** service name `JobTracker-<provider>`, reusing the `keyring::Entry` pattern from `google_oauth.rs`. Backend selection at startup: try keyring, fall back to file, remember which, expose it via `backend` so Settings can say so rather than downgrading silently.
- [ ] **Step 5:** commit.

---

## Task A3 — Launch migration + Settings UX 🟡

**Produces:** no secrets left in `localStorage`; Settings shows configured / not configured with Replace and Remove.
**Commit:** `feat(settings): migrate API keys out of localStorage into the keyring`

> **This task can destroy the user's keys if written naively.** "Write to keyring, then delete from localStorage" loses the key whenever the write silently fails — exactly the headless-Linux case the fallback exists for. The order below is not optional.

- [ ] **Step 1 — red:** migration unit tests (mocked `invoke`) covering four cases:
  1. localStorage has a key, keyring write succeeds, **read-back via `llm_key_status` confirms `configured: true`** ⇒ only then remove the localStorage entry.
  2. Keyring write fails ⇒ **localStorage entry is kept**, a visible warning names the provider, the app stays usable.
  3. Key already present in the store ⇒ do **not** overwrite; drop the stale localStorage copy after confirming the store has one.
  4. Migration runs twice ⇒ second run is a no-op.
- [ ] **Step 2 — red:** no component holds a secret in React state. Assert `SettingsModal` renders no `value={...}` bound to a stored secret and that `useJobTrackerState` exposes status booleans, not key strings.
- [ ] **Step 3 — implement:** Replace opens a write-only field → `llm_key_set` → clear the input → refresh status. Remove calls `llm_key_clear` behind a confirm. Show the `backend` when it is `file`, with one sentence on why.
- [ ] **Step 4 (H4):** drop `serp_api_key` / `brave_search_api_key` / `serpApiKey` parameters from the `tauriApi.ts` wrappers and their call sites; Rust reads them from the store. Ship this together with A5 so the command signatures change once.
- [ ] **Step 5:** all new strings in `i18n/en.ts`; commit.

---

## Task A4 — Provider registry and Rust LLM extraction 🔴

**Produces:** data-driven provider registry; `extract_job_info(raw_text, provider)` command; Scaleway / Mistral / Gemini paths; TS keeps only normalization.
**Commit:** `feat(llm): Rust provider registry and extraction behind the keyring`

**Verified provider facts (Scaleway docs, 2026-09-09):** base URL `https://api.scaleway.ai/v1`, `POST /v1/chat/completions`, `Authorization: Bearer`, OpenAI-compatible. Prefer `response_format: {type: "json_schema", …}` — Scaleway's own docs call `json_object` "legacy, non-deterministic … not recommended". **The model id is not verified**: resolve it via `GET /v1/models` during this task and treat the result as a default, never a constant.

- [ ] **Step 1 — red:** cargo tests against a local stub HTTP server (no real provider calls in CI): Scaleway request carries Bearer auth and schema-mode `response_format`; Mistral keeps its current shape; Gemini keeps `x-goog-api-key` and `generateContent`; a 401 maps to `E_LLM_AUTH`, a 404 model to `E_LLM_MODEL`; a response containing `"priority": 3` is normalized **without** `priority` (spec §0.1).
- [ ] **Step 2 — red:** stub returns fenced ```json — parsing still succeeds. Port the existing tolerance from `parsePartialNewJobFromLlmText`; a Rust normalizer that is stricter than the TS one is a regression.
- [ ] **Step 3 — red (vitest):** the Extract button invokes the Tauri command and no longer calls `fetch` with a key.
- [ ] **Step 4 — implement:** registry as data (`base_url`, `model_id`, `auth`, `json_mode`, `keyring_service`), `base_url`/`model_id` overridable in Settings → advanced. Prompt and schema as `include_str!` assets under `src-tauri/prompts/`. Exhaustive `match` on the provider enum.
- [ ] **Step 5 (H6):** in browser-only `npm run dev`, `invoke` is unavailable — catch it and show "AI extraction requires the desktop app (`npm run tauri:dev`)" instead of an opaque failure. Add a line to `CONTRIBUTING.md`.
- [ ] **Step 6:** Settings **Test connection** button: one cheap round-trip reporting the resolved model name. This is how a wrong model id gets diagnosed in seconds.
- [ ] **Step 7:** commit.

---

## Task A5 — Two HTTP helpers, not one 🟡

**Produces:** `net::fetch_untrusted(url)` with the spec §6.3 guards, and `net::api_client()` for known endpoints.
**Commit:** `feat(net): SSRF-hardened fetch for untrusted listing URLs`

> **H3 is the trap here.** `job_search.rs` calls `serpapi.com` and `api.search.brave.com` **with auth headers**, and `listing_check.rs` calls SerpAPI for Indeed. A single wrapper that strips auth headers and demands a text content type would break job search outright. The guards exist to defend against *attacker-supplied* URLs; a hardcoded API host is not one.

- [ ] **Step 1 — red:** table tests for `fetch_untrusted` rejecting `file://`, `ftp://`, `http://127.0.0.1`, `http://169.254.169.254`, `http://[::1]`, `http://10.0.0.1`, a public→private **redirect** (re-validated after each hop, not only at the start), a body over 2 MiB (aborted mid-stream, not buffered then measured), and a non-text `Content-Type`.
- [ ] **Step 2 — red:** `api_client()` reaches an allowlisted host **with** its auth header intact, and refuses a host that is not on its allowlist.
- [ ] **Step 3 — implement:** shared builder for timeouts (15 s connect / 30 s total) and the existing user-agent; `fetch_untrusted` adds resolve-and-check, max 5 redirects, streamed size cap, content-type allowlist, no cookie store, no auth.
- [ ] **Step 4 — retrofit, carefully:** `listing_check::detect_status` fetches arbitrary listing URLs ⇒ `fetch_untrusted`; its SerpAPI call ⇒ `api_client()`. `job_search` provider calls ⇒ `api_client()`; `fetch_job_search_result_page_text` fetches result URLs ⇒ `fetch_untrusted`. **Record the pre-change classification for a handful of real listing URLs and compare after** — the 2 MiB cap and content-type check can change `detect_status` outcomes, and a silent regression here is invisible until a user's listing status goes wrong.
- [ ] **Step 5 (H4):** read SerpAPI/Brave keys from the secret store inside Rust; delete the key parameters from those commands.
- [ ] **Step 6:** commit.

---

## Task A6 — Python tooling reach, docs, verify gate 🟢

**Commit:** `docs: document keyring secrets and the Rust LLM client`

- [ ] **Prepare for PR B's Python package.** `pyproject.toml` scopes isort `src_paths`, ruff `src`, and the `py:lint` / `py:format` npm scripts to `tests` only; `python.yml` runs `ruff check tests`. Widen all of these to `["tests", "python"]` **now**, while the diff is trivial, instead of inside the sidecar PR.
- [ ] `docs/architecture.md`: keys live in the OS keyring; LLM calls happen in Rust; two HTTP helpers and when to use which.
- [ ] `CONTRIBUTING.md`: extraction needs `npm run tauri:dev`; `verify:rust` can skip silently.
- [ ] `npm run verify` green **and** the Rust half actually executed (see index).
- [ ] `git diff main --unified=0 | grep -E '^\+.*(TODO|FIXME|HACK|XXX)'` — resolve or track.
- [ ] PR description: foundations only, mail scan not included; call out that key storage moved and what happens if the keyring is unavailable.

---

## Spec coverage

| Spec item | Task |
|-----------|------|
| §7.1 migration runner, pragmas | A1 |
| Schema mirror drift (`listing_status`, `listing_checked_at`) | A1 |
| WAL-safe backup (hazard found during planning) | A1 |
| §6.1 / ADR 0005 keyring | A2, A3 |
| §8 provider registry, Scaleway, `json_schema` | A4 |
| Extraction without keys in the webview | A4 |
| §6.3 SSRF guards | A5 |
| `priority` never written from LLM output (§0.1) | A4 |
| Python tooling reach for PR B | A6 |

## Out of scope

Sidecar, NDJSON protocol, mail tables, fingerprints, scoring, enrichment, inbox UI — PRs B and C.

## Success criteria

1. Fresh and upgraded databases converge on an identical schema; running migrations twice changes nothing.
2. A backup taken with uncommitted WAL content restores every row.
3. No provider key, search key, or Google access token remains in `localStorage`; a failed keyring write keeps the key and says so.
4. `llm_key_status` cannot return key material — by type, and under test.
5. Extraction works through Rust on all three providers; a wrong model id is diagnosable via Test connection.
6. The SSRF suite passes and job search still returns results.
7. `cargo test` passes on a runner with no secret service.
