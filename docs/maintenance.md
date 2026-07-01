# Maintenance log

---

## 2026-07-01

### Checks performed
- Reviewed `package.json` frontend and dev deps (`npm outdated`, `npm audit`)
- Reviewed `src-tauri/Cargo.toml` / `Cargo.lock` Rust deps (`cargo update --dry-run`, manual crate review — `cargo audit` could not fetch the RustSec advisory-db in this environment, blocked by org egress policy on the raw `git` clone of `github.com`; no findings from manual review of direct dependency versions)
- Reviewed `requirements-dev.txt` / `pyproject.toml` / `uv.lock` Python dev deps (`uv run pip-audit`)
- Reviewed all three CI workflows in `.github/workflows/`
- Checked CI status on `main` (all green as of the PR #22 merge, commit `72cd62c`)
- Verified whether the "Major upgrades pending" items from 2026-06-10 were actually applied, per commit `2766401` (PR #22, merged 2026-06-18)

### Stale-doc correction: 2026-06-10 "major upgrades pending" was already resolved

Commit `2766401` ("chore: upgrade TypeScript 6, ESLint 10, rand 0.9, pytest 9", merged via PR #22 on 2026-06-18) already applied **three of the four** items flagged on 2026-06-10:

| Package | 06-10 status | Verified current state |
|---|---|---|
| `typescript` | flagged outdated (6.x available) | `~6.0.0` in `package.json` — **applied** |
| `eslint` / `@eslint/js` | flagged outdated (10.x available) | `eslint ^10.x`, `@eslint/js ^10.x` in `package.json` — **applied** |
| `rand` (Rust) | flagged outdated (0.9 available) | `rand = "0.9"` in `src-tauri/Cargo.toml` — **applied** |
| `pytest` | pinned `<9`, flagged for relaxing | `pyproject.toml` / `uv.lock` already `>=9.0,<10` / `9.1.0` — **applied**, but `requirements-dev.txt` was left at `>=8.0,<9` (see Fixes applied) |

All four are confirmed resolved in the dependency manifests; local `npm run verify`-equivalent checks (lint, test, build, clippy, cargo test, ruff/black/isort, pytest) all pass against the upgraded versions. The pending table below is updated to reflect the true current state (empty — no majors currently flagged).

### Fixes applied

- **Security fix — stale `requirements-dev.txt` pytest pin**: `requirements-dev.txt` still pinned `pytest>=8.0,<9`, resolving to `pytest 8.4.2`, while `pyproject.toml`/`uv.lock` (used by `npm run verify` via `uv sync`) had already moved to `pytest>=9.0,<10` in PR #22. `uv run pip-audit -r requirements-dev.txt` flagged `pytest 8.4.2` under **CVE-2025-71176**, fixed in `9.0.3`. `.github/workflows/python.yml` installs from `requirements-dev.txt` directly (`pip install -r requirements-dev.txt`), so CI was testing against a vulnerable, older pytest than local dev used — a real drift introduced when PR #22 updated `pyproject.toml` but missed this file. Bumped `requirements-dev.txt` to `pytest>=9.0,<10` to match. Re-ran `pip-audit` — clean, no known vulnerabilities.
- **CI** — Bumped `actions/checkout@v6` → `v7` in all three workflows (`frontend.yml`, `python.yml`, `rust.yml`). GitHub shipped `actions/checkout@v7` in June 2026 with pwn-request protections for `pull_request_target`/`workflow_run` triggers, plus a security backport deadline of 2026-07-16 for v6 and earlier. This repo's workflows only use `push`/`pull_request` triggers so the pwn-request risk doesn't directly apply, but the bump is a drop-in, non-breaking version update and keeps CI ahead of the deprecation. `actions/setup-node@v6` and `actions/setup-python@v6` are already current; `Swatinem/rust-cache@v2` major pin already tracks the latest `v2.9.0` release.
- **npm** — Bumped all patch/minor-outdated frontend deps to latest within existing semver ranges: `@tauri-apps/api` `^2.11.0` → `^2.11.1`, `@tauri-apps/cli` `^2.11.2` → `^2.11.4`, `@types/react` `^19.2.14` → `^19.2.17`, `@vitejs/plugin-react` `^6.0.1` → `^6.0.3`, `dayjs` `^1.11.20` → `^1.11.21`, `eslint` `^10.x` → `^10.6.0`, `@eslint/js` → `^10.0.1` (own release cadence, independent of `eslint`'s), `eslint-plugin-react-refresh` `^0.5.2` → `^0.5.3`, `globals` `^17.4.0` → `^17.7.0`, `happy-dom` `^20.9.0` → `^20.10.6`, `lucide-react` `^1.6.0` → `^1.23.0`, `react`/`react-dom` `^19.2.4` → `^19.2.7`, `react-router-dom` `^7.13.1` → `^7.18.1`, `typescript-eslint` `^8.60.0` → `^8.62.1`, `vite` `^8.0.1` → `^8.1.2`, `vitest` `^4.1.0` → `^4.1.9`. `package-lock.json` refreshed. `npm audit` clean before and after (0 vulnerabilities).
- **Rust** — No `Cargo.toml` version changes needed; `cargo update --dry-run` shows only transitive/indirect crate bumps within existing constraints (lockfile not touched). `cargo audit` could not run (see Checks performed); manual review of direct dependencies found nothing outdated beyond what PR #22 already resolved (`rand 0.9`).
- **Python** — No `pyproject.toml`/`uv.lock` changes needed (already current from PR #22); only `requirements-dev.txt` needed the pytest sync described above.

### Dependency status

**Frontend (`package.json` — dependencies):** all current (`react` `^19.2.7`, `react-dom` `^19.2.7`, `react-router-dom` `^7.18.1`, `@dnd-kit/core` `^6.3.1`, `lucide-react` `^1.23.0`, `dayjs` `^1.11.21`, `@tauri-apps/api` `^2.11.1`).

**Frontend (`package.json` — devDependencies):** all current (`vite` `^8.1.2`, `vitest` `^4.1.9`, `@vitejs/plugin-react` `^6.0.3`, `typescript` `~6.0.0`, `eslint` `^10.6.0`, `@eslint/js` `^10.0.1`, `typescript-eslint` `^8.62.1`, `husky` `^9.1.7`, `@tauri-apps/cli` `^2.11.4`, `happy-dom` `^20.10.6`, `globals` `^17.7.0`, `@testing-library/react` `^16.3.2`, `@types/react` `^19.2.17`, `@types/node` `^24.12.0` — note `@types/node` 26.x exists but is a major bump tracking Node types beyond our Node 20 CI target; left as-is, not flagged as urgent).

**Rust (`src-tauri/Cargo.toml`):** all current, including `rand = "0.9"` (applied in PR #22).

**Python dev (`requirements-dev.txt` / `pyproject.toml`):** `pytest>=9.0,<10` in both files now (was inconsistent — see Fixes applied); `black`, `isort`, `ruff` all current.

### Major upgrades pending (require manual testing)

None. All previously flagged items (`eslint`/`@eslint/js` 9→10, `typescript` 5.9→6, `rand` 0.8→0.9, `pytest` 8→9) were already applied in PR #22 (commit `2766401`, merged 2026-06-18) and are verified present in the manifests as of this cycle.

### Local verification (equivalent to CI)

- `npm run lint` — pass (1 pre-existing warning, 0 errors)
- `npm run test` (vitest) — pass, 97/97 tests, 16/16 files
- `npm run build` (tsc -b && vite build) — pass
- `cargo clippy --all-targets -- -D warnings` (with GTK/WebKit dev libs installed) — pass, 0 warnings
- `cargo test` — pass, 70/70 tests
- `uv run ruff check tests` / `uv run black --check tests` / `uv run isort --check-only tests` — pass
- `uv run pytest -q` — pass, 3/3 tests
- `uv run pip-audit -r requirements-dev.txt` — clean after fix (was 1 finding, CVE-2025-71176, before fix)
- `npm audit` — clean, 0 vulnerabilities (before and after)

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
