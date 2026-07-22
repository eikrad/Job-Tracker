# Maintenance log

---

## 2026-07-22

### Checks performed
- Reset working branch to `origin/main` tip (`3ac0d25`) before making any changes
- Baseline (before changes): `npm ci`, `npm run lint`, `npm run test`, `npm run build`, `npm run py:test`, `cargo build --manifest-path src-tauri/Cargo.toml` (rust build), `cargo update --dry-run` (lockfile-wide check), `npm outdated`
- Reviewed `package.json` frontend and dev deps
- Reviewed `src-tauri/Cargo.toml` Rust deps + full `cargo update --dry-run` for transitive drift
- Reviewed `pyproject.toml` / `requirements-dev.txt` Python dev deps (both currently in sync, unlike the drift flagged on 2026-06-30)
- Checked for an open `security-audit`-labelled issue (from `.github/workflows/weekly-audit.yml`) to cross-reference — **none found, open or closed** — ran all three audits fresh instead of relying on stale results
- Installed `cargo-audit` (`cargo install cargo-audit --locked`, ~4.5 min build) and `pip-audit` (`pip install pip-audit`) since neither was preinstalled in this sandbox — both succeeded this cycle (contrast with prior cycles where this may have been skipped)

### PR/issue backlog — flagged for repo owner

**This repo currently has 18 open, unmerged PRs against `main` (#54–#72), and `main` has not absorbed any of them in over two weeks (oldest since 2026-07-06).** Every additional unmerged PR compounds review effort and risks conflicting lockfile changes. Breakdown:

| Category | PRs | Notes |
|---|---|---|
| Dependabot (npm) | #55 (dayjs), #56 (lucide-react), #58 (vite), #60 (react-dom), #61 (vitest) | Single-package bumps, safe to merge independently |
| Dependabot (cargo, `/src-tauri`) | #57 (tauri), #59 (log), #62 (tauri-build), #63 (open), #64 (sha2 0.10→0.11, crosses 0.x "major" boundary — review before merging) | All now also reflected in this cycle's lockfile-wide `cargo update`, except #64 (see Majors table) |
| Dependabot (pip) | #54 (isort ≥5.13→≥8.0.1), #72 (ruff ≥0.15.20→≥0.15.22) | #54 only formalizes the floor — `isort` already resolves to `8.0.1` in `uv.lock` under the existing unbounded constraint |
| Dependabot (github-actions) | #70 (setup-python), #71 (setup-node) | Not touched by this cycle (out of scope — CI action pins, not app deps) |
| Prior weekly-maintenance (unmerged) | #65 (2026-07-08), #68 (2026-07-15) | Neither landed — this explains why `docs/maintenance.md` on `main` jumps from 2026-06-30 straight to this entry with no 07-08/07-15 record |
| Docs (unmerged) | #66, #69 | Unrelated to dependency maintenance; still pending review |

**Recommendation: triage and merge or close #54–#72 before/alongside this PR.** The longer these sit, the more this cycle's lockfile-wide sync (below) will conflict with them on merge. This PR intentionally does **not** duplicate any single-package Dependabot bump already proposed above — it only does the broader lockfile-wide refresh (transitive deps, drift Dependabot's per-package PRs don't catch) plus the security audit.

### Fixes applied

- **npm** — `npm update` (lockfile-only, no `package.json` range changes): resolved 63 transitive packages, including `brace-expansion` which fixes the one npm-audit high-severity finding (see Security findings). `npm audit` is now clean (0 vulnerabilities, was 1 high).
- **Rust** — `cargo update` (lockfile-only, respects existing `Cargo.toml` ranges): refreshed 145+ crates across the dependency graph — this is exactly the "Dependabot per-package PRs miss transitive deps" gap flagged in the task; direct deps that moved within their existing semver range: `tauri` 2.11.2→2.11.5, `tauri-build` 2.6.2→2.6.3, `tauri-plugin-log` 2.8.0→2.9.0, `log` 0.4.29→0.4.33, `chrono` 0.4.44→0.4.45, `open` 5.3.3→5.4.0, `serde`/`serde_json`/`serde_repr`/`serde_with` patch bumps. `rand`'s stale 0.7/0.8/0.9 lockfile entries (orphaned from earlier merged PRs #44/#48) were also pruned.
- **Python** — `uv lock --upgrade` (respects existing `pyproject.toml` constraints): `black` 26.3.1→26.5.1, `ruff` 0.15.10→0.15.22, `pytest` 9.1.0→9.1.1, plus transitive `click`/`packaging`/`pathspec`/`platformdirs`. Note: `pyproject.toml` and `requirements-dev.txt` are **already in sync** this cycle (both `pytest>=9.1.1,<10`) — the drift flagged on 2026-06-30 was resolved at some point between then and now.

### Dependency status

**Frontend (`package.json` — dependencies):**

| Package | Version (range) | Resolved | Status |
|---|---|---|---|
| `react` / `react-dom` | `^19.2.4` | `19.2.8` | Current (within range) |
| `react-router-dom` | `^7.18.1` | `7.18.1` | Current |
| `@dnd-kit/core` | `^6.3.1` | `6.3.1` | Current |
| `lucide-react` | `^1.6.0` | `1.6.0` (installed) / `1.25.0` latest | Behind — Dependabot PR #56 open for this |
| `dayjs` | `^1.11.20` | `1.11.21` | Current — Dependabot PR #55 open for this |
| `@tauri-apps/api` | `^2.11.1` | `2.11.1` | Current |

**Frontend (`package.json` — devDependencies):**

| Package | Version (range) | Resolved | Status |
|---|---|---|---|
| `vite` | `^8.0.1` | `8.1.5` | Current (within range) — Dependabot PR #58 open, would only formalize |
| `vitest` | `^4.1.0` | `4.1.10` | Current (within range) — Dependabot PR #61 open, would only formalize |
| `@vitejs/plugin-react` | `^6.0.1` | `6.0.4` | Current |
| `typescript` | `~6.0.0` | `6.0.3` | Current (see Majors — 7.x not applied) |
| `eslint` / `@eslint/js` | `^10.0.x` | `10.7.0` / `10.0.1` | Current |
| `typescript-eslint` | `^8.62.1` | `8.65.0` | Current |
| `husky` | `^9.1.7` | `9.1.7` | Current |
| `@tauri-apps/cli` | `^2.11.4` | `2.11.4` | Current |
| `happy-dom` | `^20.10.6` | `20.11.1` | Current |
| `@testing-library/react` | `^16.3.2` | `16.3.2` | Current |
| `@types/node` | `^24.12.0` | `24.13.3` | Current (see Majors — 26.x not applied) |

**Rust (`src-tauri/Cargo.toml`):**

| Crate | Version (range) | Resolved | Status |
|---|---|---|---|
| `tauri` | `2.11` | `2.11.5` | Current — Dependabot PR #57 open, would only formalize |
| `tauri-build` | `2.5.6` | `2.6.3` | Current — Dependabot PR #62 open, would only formalize |
| `tauri-plugin-log` | `2` | `2.9.0` | Current |
| `rusqlite` | `0.40.1` | `0.40.1` | Current |
| `serde` / `serde_json` | `1.0` | `1.0.229` / `1.0.151` | Current |
| `chrono` | `0.4` | `0.4.45` | Current |
| `reqwest` | `0.12` | `0.12.28` | Current within range (see Majors — 0.13 not applied) |
| `keyring` | `3` | `3.6.3` | Current within range (see Majors — 4.x not applied — new finding this cycle) |
| `open` | `5.2` | `5.4.0` | Current — Dependabot PR #63 open, would only formalize |
| `sha2` | `0.10` | `0.10.9` | Current within range (see Majors — 0.11 not applied — Dependabot PR #64 open) |
| `rand` | `0.10` | `0.10.2` | Current |
| `base64` | `0.22` | `0.22.1` | Current |
| `url` | `2.5` | `2.5.8` | Current |
| `shellexpand` | `3` | `3.1.2` | Current |
| `log` | `0.4` | `0.4.33` | Current — Dependabot PR #59 open, would only formalize |

**Python dev (`pyproject.toml` / `requirements-dev.txt` — in sync):**

| Package | Constraint | Resolved | Status |
|---|---|---|---|
| `pytest` | `>=9.1.1,<10` | `9.1.1` | Current |
| `black` | `>=26.5.1` | `26.5.1` | Current |
| `ruff` | `>=0.15.20` | `0.15.22` | Current — Dependabot PR #72 open, would only formalize |
| `isort` | `>=5.13` | `8.0.1` | Resolves 3 majors ahead of the stated floor under the unbounded constraint — Dependabot PR #54 proposes raising the floor to `>=8.0.1` to match reality; no functional change needed from us |

### Security findings

- **npm audit** — 1 high-severity finding before this cycle: `brace-expansion` (`GHSA-3jxr-9vmj-r5cp`, CVSS 5.3, ReDoS via exponential-time expansion, affects `3.0.0–5.0.6`, transitive dev-only dependency). Not reachable in production app usage (dev-tooling only), but fixed anyway by `npm update` → **0 vulnerabilities now**.
- **pip-audit** — run against the `uv`-exported lockfile (`uv export --no-hashes -o requirements-export.txt`) using the project's `uv`-managed interpreter (`.venv/bin/python3`, Python 3.12.3) → **no known vulnerabilities found**.
- **cargo audit** — installed fresh this cycle (`cargo-audit v0.22.2`) and run against `src-tauri/Cargo.lock` (465 crates scanned) → **0 exploitable vulnerabilities**. 17 informational "unmaintained"/"unsound" advisories, all pre-existing and none with an available non-breaking fix:
  - `RUSTSEC-2024-0411..0420` (10 advisories): `atk`/`atk-sys`, `gdk`/`gdk-sys`, `gdkwayland-sys`, `gdkx11`/`gdkx11-sys`, `gtk`/`gtk-sys`, `gtk3-macros` `0.18.2` — the `gtk-rs` GTK3 bindings are unmaintained upstream. These are transitive, pulled in by the Linux system-tray feature stack; no drop-in replacement exists without a GTK4/Tauri-tray migration (out of scope for a dependency-sync cycle).
  - `RUSTSEC-2024-0419`: `proc-macro-error 1.0.4` unmaintained (transitive, build-time only).
  - `RUSTSEC-2025-0075/0080/0081/0098/0100` (5 advisories): `unic-char-property`/`unic-char-range`/`unic-common`/`unic-ucd-ident`/`unic-ucd-version` `0.9.0` unmaintained (transitive, Unicode data tables).
  - `RUSTSEC-2024-0429`: `glib 0.18.5` unsound iterator impl (`VariantStrIter`) — transitive via the GTK3 stack; no CVE, low practical risk for this app's usage (no direct `glib::Variant` iteration in `src-tauri/src/`).
  - None of these are blocking (`cargo audit` exit code `0` — warnings only, no hard vulnerabilities).
- No open `security-audit`-labelled GitHub issue exists (checked both open and closed) — nothing stale to reconcile; this cycle's findings above are the current ground truth.

### Major upgrades — flagged, NOT applied

| Package | In use | Latest | Notes |
|---|---|---|---|
| `typescript` | `~6.0.0` (`6.0.3`) | `7.0.2` | Crosses a major boundary; needs a dedicated `tsc --noEmit` pass across the codebase before adopting, per prior cycles' pattern for TS majors |
| `@types/node` | `^24.12.0` (`24.13.3`) | `26.1.1` | Two majors ahead; verify against the pinned Node runtime version before bumping |
| `sha2` (Rust) | `0.10` (`0.10.9`) | `0.11.0` | 0.x "minor" bump is breaking per semver convention for pre-1.0 crates; Dependabot PR #64 already proposes this — reviewed there, not duplicated here |
| `reqwest` (Rust) | `0.12` (`0.12.28`) | `0.13.4` | Same 0.x breaking-boundary situation; no Dependabot PR yet for the direct dependency — audit `src-tauri/src/` HTTP call sites (blocking client usage, TLS config) before bumping |
| `keyring` (Rust) | `3` (`3.6.3`) | `4.1.5` | New finding this cycle — no open Dependabot PR covers it. Keyring 4.x changed its credential-store API surface; audit every `keyring::Entry` call site in `src-tauri/src/` before upgrading |

### Post-change verification (must be green before commit)

| Check | Result |
|---|---|
| `npm run lint` | Pass — 0 errors, 1 pre-existing warning (`react-hooks/exhaustive-deps` in `JobDetailPage.tsx`, unrelated to this cycle) |
| `npm run test` (vitest) | Pass — 16 test files, 97 tests |
| `npm run build` | Pass — `tsc -b && vite build` succeeds |
| `npm run py:test` (pytest via uv) | Pass — 3 tests |
| `npm run py:lint` (ruff + black + isort via uv) | Pass — all checks clean |
| `npm audit` | Pass — 0 vulnerabilities (was 1 high) |
| `cargo build` / `cargo clippy` / `cargo test` (`src-tauri`) | **Environment-limited, not a regression** — fails at the `gdk-sys` build script because `gdk-3.0.pc` / GTK3 system libs are not installed in this sandbox (`pkg-config --exists gdk-3.0` → exit 1). This is pre-existing per `package.json`'s own `verify:rust` script, which already special-cases this and echoes a skip message when GTK prerequisites are absent. Not something this cycle can fix without system package installation outside repo scope. |
| `cargo audit` | Pass — 0 vulnerabilities, 17 pre-existing informational warnings (see Security findings) |
| `pip-audit` | Pass — 0 known vulnerabilities |



### Checks performed
- Reviewed `package.json` frontend and dev deps
- Reviewed `src-tauri/Cargo.toml` Rust deps
- Reviewed `requirements-dev.txt` / `pyproject.toml` Python dev deps
- Reviewed CI workflows in `.github/workflows/`
- Cross-referenced current `package.json` against previous maintenance log entries

### Infrastructure added

- **Dependabot** — Added `.github/dependabot.yml` to automate weekly PR generation for:
  - npm (root) — targets `main`
  - Cargo (`/src-tauri`) — targets `main`; also raises security alerts via GitHub Advisory DB
  - pip — targets `main` (covers `requirements-dev.txt` / `pyproject.toml` dev deps)
  - GitHub Actions — targets `main`

- **Weekly security audit** — Added `.github/workflows/weekly-audit.yml`. Runs every Monday
  at 06:00 UTC and can be triggered manually via `workflow_dispatch`:
  - Audits npm deps with `npm audit`
  - Audits Python deps with `pip-audit` against the `uv`-exported lockfile
  - Audits Rust deps with `cargo-audit` (binary cached between runs)
  - Writes a full report to the workflow step summary
  - If high- or critical-severity vulnerabilities are found, opens (or updates) a GitHub Issue
    labelled `security-audit` + `maintenance`

### Major upgrades completed (were pending last cycle)

The following upgrades listed as pending in 2026-06-10 are now reflected in the repo:

| Package | Was | Now | Notes |
|---|---|---|---|
| `typescript` | `~5.9.3` | `~6.0.0` | Applied; CI passes |
| `eslint` / `@eslint/js` | `^9.x` | `^10.0.x` | Applied; `eslint.config.js` updated |
| `rand` (Rust) | `0.8` | `0.9` | Applied; all call sites updated |

### Dependency status

**Frontend (`package.json` — dependencies):**

| Package | Version | Status |
|---|---|---|
| `react` / `react-dom` | `^19.2.4` | Current |
| `react-router-dom` | `^7.13.1` | Current |
| `@dnd-kit/core` | `^6.3.1` | Current |
| `lucide-react` | `^1.6.0` | Current |
| `dayjs` | `^1.11.20` | Current |
| `@tauri-apps/api` | `^2.11.0` | Current |

**Frontend (`package.json` — devDependencies):**

| Package | Version | Status |
|---|---|---|
| `vite` | `^8.0.1` | Current |
| `vitest` | `^4.1.0` | Current |
| `@vitejs/plugin-react` | `^6.0.1` | Current |
| `typescript` | `~6.0.0` | Current |
| `eslint` | `^10.0.0` | Current |
| `@eslint/js` | `^10.0.1` | Current |
| `typescript-eslint` | `^8.60.0` | Current |
| `husky` | `^9.1.7` | Current |
| `@tauri-apps/cli` | `^2.11.2` | Current |
| `happy-dom` | `^20.9.0` | Current |
| `@testing-library/react` | `^16.3.2` | Current |

**Rust (`src-tauri/Cargo.toml`):**

| Crate | Version | Status |
|---|---|---|
| `tauri` | `2.11` | Current |
| `tauri-build` | `2.5.6` | Current |
| `tauri-plugin-log` | `2` | Current |
| `rusqlite` | `0.32.1` | Current |
| `serde` / `serde_json` | `1.0` | Current |
| `chrono` | `0.4` | Current |
| `reqwest` | `0.12` | Current |
| `keyring` | `3` | Current |
| `open` | `5.2` | Current |
| `sha2` | `0.10` | Current |
| `rand` | `0.9` | Current (upgraded from 0.8) |
| `base64` | `0.22` | Current |
| `url` | `2.5` | Current |
| `shellexpand` | `3` | Current |

**Python dev (`requirements-dev.txt`):**

| Package | Constraint | Status |
|---|---|---|
| `pytest` | `>=8.0,<9` | Upper bound is conservative — relax to `>=9.0` after verifying test suite |
| `black` | `>=24.0` | Current |
| `ruff` | `>=0.8.0` | Current |
| `isort` | `>=5.13` | Current |

### Minor upgrade pending

| Package | Notes |
|---|---|
| `pytest` (Python) | Upper bound `<9` is conservative. Relax to `>=9.0` once the test suite is verified on pytest 9. |

---

## 2026-06-30

### Checks performed
- Re-checked the three "pending manual review" upgrades from the 2026-06-10 entry against the current `package.json`, `src-tauri/Cargo.toml`, and `pyproject.toml`
- Compared `pyproject.toml` against `requirements-dev.txt` (the two parallel Python dependency manifests used by `uv run pytest` and `pip install -r requirements-dev.txt` respectively)

### Findings

The upgrades flagged as pending on 2026-06-10 already landed in commit `2766401` ("chore: upgrade TypeScript 6, ESLint 10, rand 0.9, pytest 9", 2026-06-17) — the 2026-06-10 dependency table below is now out of date on these rows:

| Package | Then | Now |
|---|---|---|
| `typescript` | `~5.9.3` (outdated) | `~6.0.0` — Current |
| `eslint` / `@eslint/js` | `^9.39.4` (outdated) | `^10.0.0` / `^10.0.1` — Current |
| `rand` (Rust) | `0.8` (outdated) | `0.9` — Current |
| `pytest` (`pyproject.toml`) | `>=8.0,<9` (pinned) | `>=9.0,<10` — Current |

### New finding: `requirements-dev.txt` lags `pyproject.toml`

`pyproject.toml` already requires `pytest>=9.0,<10`, but `requirements-dev.txt` — the manifest used by `pip install -r requirements-dev.txt` in CI (`.github/workflows/python.yml`) and in the README/CONTRIBUTING.md manual setup steps — still pins `pytest>=8.0,<9`. The `uv run pytest` path and the `pip install` path can now resolve different pytest majors. Needs a follow-up commit bumping `requirements-dev.txt` to `pytest>=9.0,<10` to match.

### Fixes applied
None this cycle — this entry corrects the record only. The `requirements-dev.txt` mismatch above is flagged for a follow-up code change.

---

## 2026-06-10

### Checks performed
- Reviewed `package.json` frontend and dev deps
- Reviewed `src-tauri/Cargo.toml` Rust deps
- Reviewed `requirements-dev.txt` Python dev deps
- Reviewed CI workflows in `.github/workflows/`
- Compared versions against bandsearch-app and radiationsafety for cross-repo consistency

### Fixes applied

No automated fixes this cycle — all previously flagged major upgrades remain pending manual review (see table below).

### Dependency status

*(Superseded by the 2026-07-01 entry above — the four "Outdated" rows below were upgraded in commit `2766401`, kept here for history.)*

**Frontend (`package.json` — dependencies):**

| Package | Version | Status |
|---|---|---|
| `react` | `^19.2.4` | Current |
| `react-dom` | `^19.2.4` | Current |
| `react-router-dom` | `^7.13.1` | Current |
| `@dnd-kit/core` | `^6.3.1` | Current |
| `lucide-react` | `^1.6.0` | Current |
| `dayjs` | `^1.11.20` | Current |
| `@tauri-apps/api` | `^2.11.0` | Current |

**Frontend (`package.json` — devDependencies):**

| Package | Version | Status |
|---|---|---|
| `vite` | `^8.0.1` | Current |
| `vitest` | `^4.1.0` | Current |
| `@vitejs/plugin-react` | `^6.0.1` | Current |
| `typescript` | `~5.9.3` | **Outdated** (6.x available, breaking) |
| `eslint` | `^9.39.4` | **Outdated** (10.x available, breaking) |
| `@eslint/js` | `^9.39.4` | **Outdated** (10.x available, breaking) |
| `typescript-eslint` | `^8.60.0` | Current |
| `husky` | `^9.1.7` | Current |
| `@tauri-apps/cli` | `^2.11.2` | Current |
| `happy-dom` | `^20.9.0` | Current |
| `globals` | `^17.4.0` | Current |
| `@testing-library/react` | `^16.3.2` | Current |
| `@types/react` | `^19.2.14` | Current |
| `@types/node` | `^24.12.0` | Current |

**Rust (`src-tauri/Cargo.toml`):**

| Crate | Version | Status |
|---|---|---|
| `tauri` | `2.10.3` | Current |
| `tauri-build` | `2.5.6` | Current |
| `tauri-plugin-log` | `2` | Current |
| `rusqlite` | `0.32.1` | Current |
| `serde` / `serde_json` | `1.0` | Current |
| `chrono` | `0.4` | Current |
| `reqwest` | `0.12` | Current |
| `keyring` | `3` | Current |
| `open` | `5.2` | Current |
| `sha2` | `0.10` | Current |
| `rand` | `0.8` | **Outdated** (0.9 available, breaking) |
| `base64` | `0.22` | Current |
| `url` | `2.5` | Current |
| `shellexpand` | `3` | Current |
| `log` | `0.4` | Current |

**Python dev (`requirements-dev.txt`):**

| Package | Constraint | Status |
|---|---|---|
| `pytest` | `>=8.0,<9` | Pinned — verify 9.x before relaxing |
| `black` | `>=24.0` | Current |
| `ruff` | `>=0.8.0` | Current |
| `isort` | `>=5.13` | Current |

### Major upgrades pending (require manual testing)

| Package | In use | Latest | Notes |
|---|---|---|---|
| `eslint` / `@eslint/js` | `^9.x` | `10.x` | Review ESLint v10 migration guide, update `eslint.config.js` |
| `typescript` | `~5.9.x` | `6.x` | Breaking type-system changes — run `tsc --noEmit` and fix errors first |
| `rand` (Rust) | `0.8` | `0.9` | Breaking API changes — audit every `rand::` call site in `src-tauri/src/` |
| `pytest` | `>=8.0,<9` | `9.x` | Relax upper bound once 9.x is verified against the test suite |

---

## 2026-06-03

### Fixes applied

- **Tauri config version sync** — `src-tauri/tauri.conf.json` declared version `"0.1.0"` while `package.json` and `Cargo.toml` both show `0.2.1`. Updated to `0.2.1` so bundle metadata (installer filenames, update manifests) matches the declared app version.
- **CI** — Deleted duplicate `ci.yml`. It ran `rust-check` and `pytest` jobs identical to `rust.yml` and `python.yml`, causing Rust and Python checks to run twice on every push/PR targeting `main`. The three dedicated workflow files (`frontend.yml`, `rust.yml`, `python.yml`) already cover all branches.
- **npm** — Bumped `@tauri-apps/api` `^2.10.1` → `^2.11.0` and `@tauri-apps/cli` `^2.10.1` → `^2.11.2`; bumped `typescript-eslint` `^8.57.0` → `^8.60.0`.

### Major upgrades pending (require manual testing)

| Package | In use | Latest | Notes |
|---|---|---|---|
| `eslint` / `@eslint/js` | `^9.x` | `10.x` | Review ESLint v10 migration guide, update `eslint.config.js` |
| `typescript` | `~5.9.x` | `6.x` | Breaking type-system changes — run `tsc --noEmit` and fix errors first |
| `rand` (Rust) | `0.8` | `0.9` | Breaking API changes — audit every `rand::` call site in `src-tauri/src/` |

---

## 2026-05-27

### Fixes applied

- **CI** — Removed duplicate `frontend` job from `ci.yml`. It ran the same lint/test/build steps as `frontend.yml` on pushes to `main`. Action versions updated to `checkout@v6`, `setup-node@v6`, `setup-python@v6`.

### Major upgrades pending (require manual testing)

| Package | In use | Latest | Notes |
|---|---|---|---|
| `eslint` / `@eslint/js` | `^9.x` | `10.x` | Review ESLint v10 migration guide |
| `typescript` | `~5.9.x` | `6.x` | Breaking type-system changes |
| `rand` (Rust) | `0.8` | `0.9` | Breaking API changes in rand crate |
