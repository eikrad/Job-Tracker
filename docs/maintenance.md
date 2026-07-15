# Maintenance log

---

## 2026-07-15

### Checks performed

- **Baseline (before any changes):** `npm ci`; `npm run lint` (frontend ESLint); `npm run test` (Vitest); `npm run build` (`tsc -b && vite build`); `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo test --manifest-path src-tauri/Cargo.toml`; `uv sync --quiet`; `npm run py:lint` (ruff/black/isort); `npm run py:test` (pytest).
- Local sandbox was initially missing Tauri's Linux build prerequisites (`libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `libsoup-3.0-dev`, `librsvg2-dev`) — installed them so `cargo check`/`cargo test` could run at all (mirrors the system deps step already present in `.github/workflows/rust.yml`).
- Local `rustc` was pinned to `1.94.1` (2026-03-25), too old to compile `libsqlite3-sys 0.38.1`'s build script (`cfg_select!` requires a newer stable compiler). Ran `rustup update stable` → `1.97.0` (2026-07-07) to match what `dtolnay/rust-toolchain@stable` resolves to in CI. This was a local-environment-only gap, not a repo issue.
- CI health check via `mcp__github__actions_list` (branch=`main`, last 5 runs per workflow): `frontend.yml`, `python.yml`, `rust.yml` all green on the latest `main` commit (`3ac0d25`, run `29286933332`/`29286933280`/`29286933259`, 2026-07-13). `weekly-audit.yml` green on its last scheduled run (2026-07-13).
- Dependency inventory: `npm outdated`; `cargo update --manifest-path src-tauri/Cargo.toml --dry-run`; `cargo info <crate>` for each direct `src-tauri/Cargo.toml` dependency to check for majors outside the dry-run's manifest-constrained view; `uv pip list --outdated`; diffed `pyproject.toml`'s `[dependency-groups] dev` block against `requirements-dev.txt` (byte-for-byte match this cycle — the drift flagged on 2026-06-30 was already fixed in an earlier cycle, no action needed).
- Security audit: `npm audit`; `pip-audit` against a `uv export --format requirements-txt --no-hashes` snapshot; `cargo audit --file src-tauri/Cargo.lock` (installed `cargo-audit 0.22.2` via `cargo install cargo-audit --locked`, not previously cached in this sandbox).
- Post-update compatibility re-check: reran the entire baseline list above after applying updates.

### Fixes applied

- **npm — safe minor/patch bump:** `npm update` (package.json ranges untouched; see table below).
- **Rust — safe minor/patch bump:** `cargo update --manifest-path src-tauri/Cargo.toml` (Cargo.toml ranges untouched; ~100 transitive/direct crates refreshed within existing semver constraints, notably `tauri` 2.11.2→2.11.5 and `tauri-plugin-log` 2.8.0→2.9.0).
- **Python — lockfile refresh:** `uv sync --quiet` re-resolved `uv.lock` to the newest versions satisfying `pyproject.toml`'s existing floors (`black`, `pytest`, `ruff` — see table below). No `pyproject.toml`/`requirements-dev.txt` edits needed.
- **Security fix — `click` (Python, transitive via `black`):** `pip-audit` flagged `click 8.3.2` for `PYSEC-2026-2132` (fixed in `8.3.3`). Rather than waiting on `black` to bump its own floor, applied a targeted `uv lock --upgrade-package click` (same "surgical transitive bump" approach used for the `plist`/quick-xml RUSTSEC fix in an earlier cycle) → `click 8.4.2`. Re-ran `pip-audit`: clean. `py:lint`/`py:test` re-verified green after the bump.
- No Rust CVE fixes were needed this cycle — `cargo audit` reported zero active vulnerability advisories (exit code 0); see Security findings below for informational "unmaintained" warnings that remain unfixable upstream.

### Security findings

| Ecosystem | Tool | Result |
|---|---|---|
| npm | `npm audit` | 0 vulnerabilities (before and after updates) |
| Python | `pip-audit` | 1 found → 1 fixed: `click 8.3.2` (`PYSEC-2026-2132`) → `8.4.2` via targeted `uv lock --upgrade-package click`. Clean on re-run. |
| Rust | `cargo audit` | 0 active vulnerability advisories. 17 informational warnings (unmaintained/unsound), listed below — no fix versions exist, flagged for awareness only. |

**Rust informational warnings (no action possible — no successor version published for the pinned major):**

| Crate | Advisory | Type | Notes |
|---|---|---|---|
| `gdkx11`, `gdkx11-sys`, `gtk`, `gtk-sys`, `gtk3-macros` (all `0.18.x`) | RUSTSEC-2024-0411/0417/0414/0415/0420 | unmaintained | Transitive via Tauri's Linux GTK3 backend (`wry`/`tao`). No fix without a GTK4 migration upstream in Tauri itself. |
| `proc-macro-error 1.0.4` | RUSTSEC-2024-0419 | unmaintained | Transitive build-time proc-macro dep, not shipped in the runtime binary. |
| `unic-char-property`, `unic-char-range`, `unic-common`, `unic-ucd-ident`, `unic-ucd-version` (all `0.9.0`) | RUSTSEC-2025-0081/0075/0080/0100/0098 | unmaintained | Transitive Unicode text-processing deps. |
| `glib 0.18.5` | RUSTSEC-2024-0429 | unsound | Narrow `Iterator`/`DoubleEndedIterator` unsoundness in `glib::VariantStrIter`; not exercised by this app's code paths. |

### Dependency status

**Frontend (`package.json` — dependencies), updated via `npm update`:**

| Package | Before | After | Status |
|---|---|---|---|
| `react` / `react-dom` | `19.2.4` | `19.2.7` | Updated (patch) |
| `dayjs` | `1.11.20` | `1.11.21` | Updated (patch) |
| `lucide-react` | `1.6.0` | `1.24.0` | Updated (minor, within `^1.6.0`) |
| `react-router-dom` | `7.18.1` | `7.18.1` | Current |
| `@dnd-kit/core` | `6.3.1` | `6.3.1` | Current |
| `@tauri-apps/api` | `2.11.1` | `2.11.1` | Current |

**Frontend (`package.json` — devDependencies), updated via `npm update`:**

| Package | Before | After | Status |
|---|---|---|---|
| `@types/node` | `24.12.0` | `24.13.3` | Updated (patch, within `^24.x`) — see major pending below |
| `@types/react` | `19.2.14` | `19.2.17` | Updated (patch) |
| `@vitejs/plugin-react` | `6.0.1` | `6.0.3` | Updated (patch) |
| `eslint-plugin-react-refresh` | `0.5.2` | `0.5.3` | Updated (patch) |
| `globals` | `17.4.0` | `17.7.0` | Updated (minor) |
| `typescript-eslint` | `8.62.1` | `8.64.0` | Updated (minor) |
| `vite` | `8.0.16` | `8.1.4` | Updated (minor) |
| `vitest` | `4.1.0` | `4.1.10` | Updated (patch) |
| `typescript` | `6.0.3` | `6.0.3` | Current (tilde range `~6.0.0` caps in-range updates) — see major pending below |
| `eslint` | `10.5.0` | `10.7.0` | Updated (minor) |
| `@eslint/js` | `10.0.1` | `10.0.1` | Current |
| `husky` | `9.1.7` | `9.1.7` | Current |
| `@tauri-apps/cli` | `2.11.4` | `2.11.4` | Current |
| `happy-dom` | `20.10.6` | `20.10.6` | Current |
| `@testing-library/react` | `16.3.2` | `16.3.2` | Current |

**Rust (`src-tauri/Cargo.toml`), refreshed via `cargo update` (lockfile-only, ranges untouched):**

| Crate | Before | After | Status |
|---|---|---|---|
| `tauri` | `2.11.2` | `2.11.5` | Updated (patch) |
| `tauri-build` | `2.6.2` | `2.6.3` | Updated (patch) |
| `tauri-plugin-log` | `2.8.0` | `2.9.0` | Updated (minor) |
| `open` | `5.3.3` | `5.4.0` | Updated (minor) |
| `chrono` | `0.4.44` | `0.4.45` | Updated (patch) |
| `rusqlite` / `libsqlite3-sys` | `0.40.1` / `0.38.1` | `0.40.1` / `0.38.1` | Current (already latest satisfying `0.40.1`) |
| `reqwest` | `0.12.28` | `0.12.28` | Current within `^0.12` — see major pending below |
| `keyring` | `3.6.3` | `3.6.3` | Current within `^3` — see major pending below |
| `sha2` | `0.10.9` | `0.10.9` | Current within `^0.10` — see major pending below |
| `serde` / `serde_json` / `log` / `base64` / `url` / `shellexpand` / `rand` | — | — | Current |

**Python dev (`pyproject.toml` / `requirements-dev.txt` — verified in sync, no drift):**

| Package | Constraint | Locked before | Locked after | Status |
|---|---|---|---|---|
| `pytest` | `>=9.1.1,<10` | `9.1.0` | `9.1.1` | Updated (lockfile refresh) |
| `black` | `>=26.5.1` | `26.3.1` | `26.5.1` | Updated (lockfile refresh) |
| `ruff` | `>=0.15.20` | `0.15.10` | `0.15.21` | Updated (lockfile refresh) |
| `isort` | `>=5.13` | `8.0.1` | `8.0.1` | Current |
| `click` (transitive, via `black`) | — | `8.3.2` | `8.4.2` | Security fix (`PYSEC-2026-2132`) |

### Major upgrades pending (require manual testing)

| Package | Ecosystem | In use | Latest | Notes |
|---|---|---|---|---|
| `typescript` | npm | `~6.0.0` (6.0.3) | `7.0.2` | Major version; review TS 7 migration/breaking-changes notes and run `tsc -b` clean before relaxing the `~6.0.0` range. |
| `@types/node` | npm | `^24.12.0` (24.13.3) | `26.1.1` | Two majors ahead; should track the Node.js runtime version this app actually targets rather than jumping blindly — confirm target Node major first. |
| `keyring` | Rust (`src-tauri/Cargo.toml`) | `3` (3.6.3) | `4.1.5` | Major version; `Entry`/backend API has changed across `keyring` 3→4 in past releases — audit every `keyring::Entry` call site in `src-tauri/src/` before bumping. |
| `reqwest` | Rust (`src-tauri/Cargo.toml`) | `0.12` (0.12.28) | `0.13.4` | Treated as breaking (0.x semantics); check for matching `hyper`/`rustls` stack compatibility with `wry`/`tauri`'s own `reqwest`-adjacent deps before bumping. |
| `sha2` | Rust (`src-tauri/Cargo.toml`) | `0.10` (0.10.9) | `0.11.0` | Treated as breaking (0.x semantics); low usage surface expected but verify hashing call sites still compile against the new API. |

Dependabot already opens individual per-package PRs for many of these; this entry exists to give explicit notice per the "flag major bumps, don't apply them" policy and to avoid duplicating single-package Dependabot PRs where they already cover a crate.

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
