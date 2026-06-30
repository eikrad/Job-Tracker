# Maintenance log

---

## 2026-06-24

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
