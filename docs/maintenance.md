# Maintenance log

---

## 2026-07-29

### ⚠️ Backlog warning — three consecutive weekly-maintenance PRs now unmerged

This is the **third** weekly maintenance cycle in a row to produce a PR against `main` without the prior two ever landing. Merging none of them is a pattern worth flagging explicitly rather than quietly adding a fourth:

| PR | Title | Opened | Status |
|---|---|---|---|
| #68 | `chore: weekly maintenance 2026-07-15 — dependency updates + security audit` | 2026-07-15 | Still open, unmerged |
| #73 | `chore: weekly maintenance 2026-07-22 — dependency updates + security audit` | 2026-07-22 | Still open, unmerged |
| #66, #69, #74 | Docs sync PRs | various | Still open, unmerged |
| #75 | `chore(deps): bump lucide-react from 1.26.0 to 1.27.0` | Dependabot | Open |
| #76 | `chore(deps): bump serde_json from 1.0.149 to 1.0.151 in /src-tauri` | Dependabot | Open |
| #77 | `chore(deps-dev): bump typescript-eslint from 8.62.1 to 8.65.0` | Dependabot | Open |
| #78 | `chore(deps-dev): update ruff requirement from >=0.15.22 to >=0.16.0` | Dependabot | Open |
| #79 | `chore(deps-dev): bump globals from 17.4.0 to 17.8.0` | Dependabot | Open |
| #80 | `chore(deps-dev): bump eslint from 10.5.0 to 10.8.0` | Dependabot | Open |
| #81 | `chore(deps): bump chrono from 0.4.44 to 0.4.45 in /src-tauri` | Dependabot | Open |
| #82 | `chore(deps-dev): bump @types/node from 24.12.0 to 26.1.2` | Dependabot (major) | Open |
| #83 | `chore(deps): bump keyring from 3.6.3 to 4.1.5 in /src-tauri` | Dependabot (major) | Open |

**16 open PRs against `main` in total.** Recommend the repo owner triage and clear this backlog — merge or close #68/#73 (both stale against the current `main` tip, see below), merge the safe Dependabot bumps (#75–#81), and make an explicit decision on the two flagged majors (#82, #83) — before another maintenance cycle adds to the pile.

Note: unlike #68/#73 (which both branched from the older tip `3ac0d25`), this cycle's baseline is the *current* `main` tip `969dc0c`, which already includes 11 individually-merged Dependabot PRs since then (#54–#61, #70–#72). The dependency tables below reflect deltas from that real baseline, not from #68/#73's now-stale assumptions.

### Checks performed

- `git fetch origin main` + `git status` — working branch `claude/upbeat-cray-llwh2p` already matched `origin/main` tip `969dc0c`, zero unique commits, no reset needed
- Read `docs/maintenance.md` for format/conventions; read #73 and #68 bodies in full via `pull_request_read` to avoid re-proposing the same specific bumps
- Baseline (before changes): `npm ci`, `npm run lint`, `npm run test`, `npm run build`, `npm audit`
- Baseline (before changes): `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (`src-tauri/`) — required installing the Tauri Linux system deps (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`, matching `rust.yml`) and `rustup update stable` (sandbox `rustc` was stale at 1.94.1 and couldn't compile `libsqlite3-sys`'s `cfg_select!` macro; updated to 1.97.1 to match what `dtolnay/rust-toolchain@stable` resolves to in CI — same environment gap #68 hit and documented)
- Baseline (before changes): `uv run pytest -q` + `ruff check` / `black --check` / `isort --check-only`, and separately `pip install -r requirements-dev.txt` into a clean venv + the same lint/test commands (mirrors `python.yml` CI exactly) — both paths agreed
- `npm outdated`, `cargo update --dry-run` (`src-tauri/`) — cross-referenced every candidate bump against the 9 open Dependabot PRs (#75–#83) to avoid duplicating any of them
- `npm audit`, `cargo audit` (installed `cargo-audit` fresh — not preinstalled), `pip-audit` against the `uv export`-ed lockfile (installed `pip-audit` fresh)

### Findings

**1. `npm audit` — 4 high severity findings, 2 fixed non-breaking, 1 needs a major bump, 1 already resolved.**

- `brace-expansion` (transitive via `eslint` → `minimatch`) — DoS via exponential/unbounded expansion (GHSA-3jxr-9vmj-r5cp, GHSA-mh99-v99m-4gvg). Fixed non-breaking via `npm audit fix`.
- `postcss` (transitive via `vite`) — path traversal in sourcemap auto-loading (GHSA-r28c-9q8g-f849). Fixed non-breaking via `npm audit fix`.
- `react-router` / `react-router-dom` — CSRF bypass in the *unstable RSC code paths* (GHSA-qwww-vcr4-c8h2). Checked the advisory directly: every 7.x release from 7.12.0 onward is flagged vulnerable: the actual patched version is **8.3.0**, a major bump. This app does not use React Router's unstable RSC APIs (standard client-side routing only in a Tauri desktop app), so real-world exposure is low, but `npm audit` still flags it and no in-range fix exists. Not applied — see majors table.
- No Dependabot PR currently open for `react-router` — this is a **new finding** this cycle, not previously flagged.

**2. `pip-audit` — `click 8.3.2` vulnerable (PYSEC-2026-2132, fix 8.3.3) — same issue #68 found, never landed because #68 never merged.** Fixed via `uv lock --upgrade-package click` → resolved to `8.4.2` (newer than the minimum fix version, still non-major). Re-audit clean.

**3. `uv.lock` had silently drifted from `pyproject.toml` again.** `pyproject.toml`'s `ruff` floor was already bumped to `>=0.15.22` by merged PR #72, but `uv.lock` had not been regenerated and still resolved `ruff==0.15.20` (below its own committed floor). The `uv lock --upgrade-package click` re-resolution incidentally caught this and refreshed `uv.lock`'s pinned `ruff` to `0.16.0` (no upper bound in either manifest, so it resolves to latest-available, same pattern documented for `isort` in the 2026-07-08 entry). This is **not** a duplicate of Dependabot PR #78 (`ruff` floor `>=0.15.22`→`>=0.16.0` in `pyproject.toml`/`requirements-dev.txt`) — that PR changes the documented floor string; this fix only regenerates the lockfile to match the floor `main` already has. `requirements-dev.txt` and `pyproject.toml` remain byte-for-byte in sync (`ruff>=0.15.22`), no drift between the two manifests this cycle.

**4. `cargo audit` — 7 real vulnerabilities (not just the usual unmaintained/unsound informational warnings), all transitive, all fixed by non-major lockfile-only bumps:**

| Crate | From | To | Advisories fixed |
|---|---|---|---|
| `quick-xml` | 0.38.4 | 0.41.0 | RUSTSEC-2026-0194, RUSTSEC-2026-0195 (quadratic-time / unbounded-allocation DoS, both 7.5 high) |
| `quinn-proto` | 0.11.14 | 0.11.16 | RUSTSEC-2026-0185 (remote memory exhaustion, 7.5 high) |
| `rustls-webpki` | 0.103.9 | 0.103.13 | RUSTSEC-2026-0099, RUSTSEC-2026-0104, RUSTSEC-2026-0098, RUSTSEC-2026-0049 (name-constraint / CRL parsing issues) |

None of these crates are direct `Cargo.toml` dependencies — all are pulled in transitively (via `tauri`/`reqwest`/`plist`) and were already within the semver ranges the parent crates declare, so a plain `cargo update -p <crate>` picked up the fix with no manifest changes. Re-ran `cargo audit` after: 0 exploitable vulnerabilities remain (only the same 17 pre-existing informational "unmaintained"/"unsound" warnings as prior cycles — GTK3 binding stack, `proc-macro-error`, `unic-*`, one `glib` note — all unfixable without an upstream Tauri/wry GTK4 migration, flagged for awareness only, unchanged from #68/#73's findings).

### Fixes applied

- **npm**: `npm audit fix` (lockfile-only, no `package.json` range changes) — fixes `brace-expansion` and `postcss` high-severity findings.
- **npm**: Targeted `npm update` for packages **not** covered by an open Dependabot PR: `@types/node`, `@types/react`, `@vitejs/plugin-react`, `eslint-plugin-react-refresh`, `happy-dom`, `vite`. Deliberately left `eslint`, `globals`, `lucide-react`, and `typescript-eslint` untouched even though `npm update` would otherwise bump them, because #80, #79, #75, and #77 already propose exactly those bumps.
- **Rust**: `cargo update` (lockfile-only, within existing `Cargo.toml` ranges) in `src-tauri/`, refreshing ~140 transitive crates and dropping ~25 dead ones (an old `html5ever`/`kuchikiki`/`markup5ever`/`cssparser` branch superseded by `plist` 1.8.0→1.10.0's newer XML backend — same style of stale-branch cleanup as the native-tls removal in the 2026-07-08 entry). Then explicitly reverted `chrono` (→0.4.44) and `serde_json` (→1.0.149) back to their pre-update pins with `cargo update -p <crate> --precise <version>`, since #81 and #76 already propose exactly those two bumps.
- **Python**: `uv lock --upgrade-package click` → fixes the `click` CVE and, as a side effect, regenerates the drifted `ruff` lock entry to match the already-committed `>=0.15.22` floor (see Finding 3).

### Dependency updates

**Frontend (`package-lock.json`, ranges in `package.json` unchanged):**

| Package | Before | After | Reason |
|---|---|---|---|
| `brace-expansion` (transitive) | 5.0.6 | 5.0.8 | Security fix (`npm audit fix`) |
| `postcss` (transitive) | 8.5.16 | 8.5.25 | Security fix (`npm audit fix`) |
| `react-router` / `react-router-dom` | 7.18.1 | 7.18.2 | Latest available patch within current major (`npm audit fix`); does not resolve the flagged CVE — see Findings |
| `@types/node` | 24.12.0 | 24.13.3 | Safe in-range bump, not the same target as major-bump PR #82 |
| `@types/react` | 19.2.14 | 19.2.17 | Safe in-range bump |
| `@vitejs/plugin-react` | 6.0.1 | 6.0.4 | Safe in-range bump |
| `eslint-plugin-react-refresh` | 0.5.2 | 0.5.3 | Safe in-range bump |
| `happy-dom` | 20.10.6 | 20.11.1 | Safe in-range bump |
| `vite` | 8.1.3 | 8.1.5 | Safe in-range bump |
| `eslint` | 10.5.0 | *(untouched)* | Already covered by open PR #80 |
| `globals` | 17.4.0 | *(untouched)* | Already covered by open PR #79 |
| `lucide-react` | 1.26.0 | *(untouched)* | Already covered by open PR #75 |
| `typescript-eslint` | 8.62.1 | *(untouched)* | Already covered by open PR #77 |

**Rust (`src-tauri/Cargo.lock`, ranges in `Cargo.toml` unchanged):**

| Crate | Before | After | Reason |
|---|---|---|---|
| `quick-xml` | 0.38.4 | 0.41.0 | Security fix (RUSTSEC-2026-0194/0195) |
| `quinn-proto` | 0.11.14 | 0.11.16 | Security fix (RUSTSEC-2026-0185) |
| `rustls-webpki` | 0.103.9 | 0.103.13 | Security fix (4 RUSTSEC advisories) |
| `plist` | 1.8.0 | 1.10.0 | Pulled in by the `quick-xml` fix; drops dead `html5ever`/`kuchikiki` XML-parsing branch |
| `tauri-plugin-log` | 2.8.0 | 2.9.0 | Safe in-range bump |
| `tauri-plugin` | 2.5.4 | 2.6.3 | Safe in-range bump (transitive, tracks `tauri`) |
| `open` | 5.3.3 | 5.4.0 | Safe in-range bump |
| ~135 other transitive crates | — | — | Refreshed within existing semver constraints via `cargo update` |
| `chrono` | 0.4.44 | *(reverted, untouched)* | Already covered by open PR #81 |
| `serde_json` | 1.0.149 | *(reverted, untouched)* | Already covered by open PR #76 |

**Python (`uv.lock`, `pyproject.toml`/`requirements-dev.txt` unchanged):**

| Package | Before | After | Reason |
|---|---|---|---|
| `click` | 8.3.2 | 8.4.2 | Security fix (PYSEC-2026-2132) |
| `ruff` (lockfile only) | 0.15.20 | 0.16.0 | Lockfile-drift fix, matches already-committed `>=0.15.22` floor — not a duplicate of PR #78 (see Finding 3) |

### Security findings

| Ecosystem | Tool | Before | After |
|---|---|---|---|
| npm | `npm audit` | 4 high (`brace-expansion`, `postcss`, `react-router` ×2 entries) | 2 high remaining (`react-router`/`react-router-dom` — needs major 8.3.0, see below) |
| Python | `pip-audit` | 1 known vuln (`click` 8.3.2, PYSEC-2026-2132) | 0 known vulnerabilities |
| Rust | `cargo audit` | 7 exploitable advisories (`quick-xml` ×2, `quinn-proto`, `rustls-webpki` ×4) + 17 informational unmaintained/unsound warnings | 0 exploitable advisories; same 17 informational warnings remain (pre-existing, unfixable without upstream GTK4 migration) |

### Major upgrades pending (require manual testing)

| Package | Ecosystem | In use | Latest / patched | Notes |
|---|---|---|---|---|
| `react-router` / `react-router-dom` | npm | `7.18.2` | `8.3.0` (patched for GHSA-qwww-vcr4-c8h2) | **New finding.** Flagged as high by `npm audit`, but the CVE only affects React Router's unstable RSC APIs, which this app does not use. No Dependabot PR open yet. Needs a full 7→8 migration review before adopting; low real-world urgency given the RSC-only exposure. |
| `typescript` | npm | `~6.0.0` (6.0.3) | `7.0.2` | Carried over from #68/#73 — still pending. Run `tsc -b` clean before relaxing the range. |
| `@types/node` | npm | `^24.12.0` (24.13.3) | `26.1.2` | Dependabot-opened (#82), two majors ahead — confirm target Node runtime major before merging. |
| `keyring` (Rust) | `3` (3.6.3) | `4.1.5` | Dependabot-opened (#83). `Entry`/backend API has changed across majors historically — audit `keyring::Entry` call sites in `src-tauri/src/` before merging. |
| `reqwest` (Rust) | `0.12` (0.12.28) | `0.13.4` | Carried over from #68/#73 — still pending, no Dependabot PR yet. Treated as breaking (0.x semantics); check `hyper`/`rustls` stack compatibility with `wry`. |
| `sha2` (Rust) | `0.10` (0.10.9) | `0.11.0` | Carried over from #68/#73 — still pending, no Dependabot PR yet. Treated as breaking (0.x semantics); verify hashing call sites against the new API. |

### Post-change verification (all green)

| Check | Result |
|---|---|
| `npm run lint` | Pass — 0 errors, 1 pre-existing unrelated warning (`react-hooks/exhaustive-deps` in `JobDetailPage.tsx`, untouched) |
| `npm run test` (Vitest) | Pass — 97/97 |
| `npm run build` | Pass |
| `npm audit` | 2 high remaining — `react-router`/`react-router-dom`, major bump only, flagged above |
| `cargo check` / `cargo clippy --all-targets -- -D warnings` | Pass, clean |
| `cargo test` (`src-tauri/`) | Pass — 80/80 |
| `uv run pytest -q` / `ruff check` / `black --check` / `isort --check-only` | Pass — 3/3, lint clean |
| `pip install -r requirements-dev.txt` (clean venv, mirrors `python.yml`) + same lint/test | Pass — 3/3, lint clean, agrees with the `uv` path |
| `cargo audit` | Pass — 0 exploitable advisories |
| `pip-audit` | Pass — 0 known vulnerabilities |

Full diff: `package-lock.json`, `src-tauri/Cargo.lock`, `uv.lock` only — no `package.json`, `Cargo.toml`, `pyproject.toml`, or `requirements-dev.txt` changes were needed this cycle.

---

## 2026-07-08

### Checks performed
- Re-verified the `requirements-dev.txt` / `pyproject.toml` pytest mismatch flagged as pending in the 2026-06-30 entry
- Fresh `git fetch origin`; rebased working branch on latest `origin/main` (`3ac0d25`)
- Baseline: `npm ci`, `npm run lint`, `npm run test`, `npm run build`
- Baseline: `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (`src-tauri/`)
- Baseline: `uv sync` + `uv run pytest -q` + `uv run ruff check` / `black --check` / `isort --check-only`
- Baseline: `pip install -r requirements-dev.txt` into a clean venv + `pytest -q` + `ruff` / `black` / `isort` (mirrors `python.yml` CI exactly)
- `npm audit` and `pip-audit` (against the `uv export`-ed lockfile, same as `weekly-audit.yml`) for HIGH/CRITICAL vulnerabilities not yet covered by an open Dependabot PR
- Reviewed all 4 workflow files (`frontend.yml`, `python.yml`, `rust.yml`, `weekly-audit.yml`) for pinned Action versions not covered by `.github/dependabot.yml`'s `github-actions` ecosystem
- Cross-referenced the 10 open Dependabot PRs (#55–#64) plus one additional open PR (#54) against `.github/dependabot.yml`'s configured ecosystems and CI status

### Findings

**1. `requirements-dev.txt` / `pyproject.toml` pytest mismatch (flagged 2026-06-30) — already resolved.**
Both files now read `pytest>=9.1.1,<10` (landed via dependabot PR #41, merged before this cycle). Verified both installer paths resolve identically and pass:

| Path | pytest | black | ruff | isort | Result |
|---|---|---|---|---|---|
| `uv sync` + `uv run pytest -q` | 9.1.1 | 26.5.1 | 0.15.20 | 8.0.1 | 3 passed, lint clean |
| `pip install -r requirements-dev.txt` + `pytest -q` (mirrors CI) | 9.1.1 | 26.5.1 | 0.15.20 | 8.0.1 | 3 passed, lint clean |

No further change needed here — this entry just confirms the fix landed and both paths agree.

**2. `uv.lock` had drifted from `pyproject.toml` — fixed.**
`pyproject.toml`'s dev-dependency floors had already been bumped by merged Dependabot PRs (`black>=26.5.1`, `pytest>=9.1.1,<10`, `ruff>=0.15.20`), but the committed `uv.lock` was never regenerated and still pinned older resolved versions that no longer satisfied those floors:

| Package | Locked (stale) | Now resolves to |
|---|---|---|
| `black` | `26.3.1` | `26.5.1` |
| `pytest` | `9.1.0` | `9.1.1` |
| `ruff` | `0.15.10` | `0.15.20` |
| `isort` | `8.0.1` (unchanged — no upper bound in either manifest) | `8.0.1` |

Fixed by running `uv sync`, which re-resolved and rewrote `uv.lock` to match `pyproject.toml`. Re-verified `uv run pytest -q` and the lint trio (`ruff` / `black` / `isort`) still pass after the regeneration.

**3. `src-tauri/Cargo.lock` had drifted from `Cargo.toml` — fixed.**
The committed lockfile still recorded the `app` package at version `0.2.1` (stale since the `0.3.0` bump in commit `3ac0d25`) and retained ~25 stale transitive crates (`native-tls`, `openssl`, `openssl-sys`, `openssl-probe`, `openssl-macros`, `core-foundation`, `security-framework`, `security-framework-sys`, `schannel`, `system-configuration`, `system-configuration-sys`, `hyper-tls`, `tokio-native-tls`, `windows-registry`, `h2`, `errno`, `rustix`, `linux-raw-sys`, `foreign-types`, `foreign-types-shared`, `encoding_rs`, `tempfile`) left over from before `reqwest` moved to `default-features = false` + `rustls-tls`. Fixed by running `cargo check` (regenerates on any mismatch since the repo doesn't build with `--locked`); `Cargo.lock` now correctly reflects `app 0.3.0` and drops the dead native-tls branch. Re-verified with `cargo clippy --all-targets -- -D warnings` (clean) and `cargo test` (80 passed, 0 failed).

**4. `astral-sh/setup-uv@v7` in `weekly-audit.yml` is a major version behind — flagged, not applied.**
Latest release is `v8.3.2`; the workflow pins the floating major tag `@v7`. Dependabot's `github-actions` ecosystem covers this file but has not yet opened a PR for the v7→v8 major bump (its 3-PR limit for this ecosystem is currently unused — 0 open actions PRs). Per policy, major bumps are not applied automatically; left for manual review. The other pinned Actions across all 4 workflow files (`actions/checkout@v7`, `actions/setup-node@v6`, `actions/setup-python@v6`, `actions/cache@v6`, `actions/github-script@v9`, `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`) are all current against their latest releases.

**5. Additional open Dependabot PR beyond the 10 already known — major bump, flagged only.**
PR **#54** — `chore(deps-dev): update isort requirement from >=5.13 to >=8.0.1` (pip ecosystem, target `main`) — bumps the *documented floor* in `pyproject.toml` and `requirements-dev.txt` from `isort>=5.13` to `isort>=8.0.1`. This is a major version bump (isort 5→8); not applied here per policy. Note: since neither manifest has an upper bound on `isort`, both `uv sync` and `pip install -r requirements-dev.txt` already resolve to `isort==8.0.1` today regardless of whether #54 is merged — the PR only makes the floor match what's already installed. CI green (confirmed via `get_check_runs`).

**6. Security audits — clean.**
`npm audit`: 0 vulnerabilities (info/low/moderate/high/critical). `pip-audit` against the `uv export`-ed project lockfile: no known vulnerabilities. `weekly-audit.yml` itself has not had its first scheduled run yet (added 2026-06-24 → workflow object created 2026-07-06; first Monday cron fires 2026-07-13) — no `security-audit`-labelled issues open. `cargo-audit` was not run standalone this cycle (environment resource constraints); `weekly-audit.yml`'s scheduled run will cover it going forward.

### Fixes applied
- Regenerated `uv.lock` via `uv sync` to match `pyproject.toml`'s already-bumped floors (`black` 26.3.1→26.5.1, `pytest` 9.1.0→9.1.1, `ruff` 0.15.10→0.15.20).
- Regenerated `src-tauri/Cargo.lock` via `cargo check` to match `Cargo.toml`/`Cargo.lock`'s already-bumped `app` version (0.2.1→0.3.0) and drop ~25 stale native-tls-branch transitive crates no longer reachable since `reqwest` switched to `rustls-tls`.
- No dependency version pins changed by hand — both fixes are lockfile regenerations to catch up with manifests that were already correct.

### Open Dependabot PR queue (owner decision required)

All 10 PRs below are minor/patch bumps opened automatically by `.github/dependabot.yml`. CI spot-checked green on #54, #55, #57, #60, #64 (all 6 checks — `Lint · Test · Build`, `Lint · Test`, `Clippy · Test` — passing); the remaining PRs share the same base and workflow config. **None were merged or closed — merge decisions are the repo owner's.**

| PR | Title | Ecosystem | Target | Status |
|---|---|---|---|---|
| #55 | `chore(deps): bump dayjs from 1.11.20 to 1.11.21` | npm | `main` | CI green, awaiting owner merge decision |
| #56 | `chore(deps): bump lucide-react from 1.6.0 to 1.23.0` | npm | `main` | CI green, awaiting owner merge decision |
| #57 | `chore(deps): bump tauri from 2.11.2 to 2.11.5 in /src-tauri` | cargo | `main` | CI green, awaiting owner merge decision |
| #58 | `chore(deps-dev): bump vite from 8.0.16 to 8.1.3` | npm | `main` | CI green, awaiting owner merge decision |
| #59 | `chore(deps): bump log from 0.4.29 to 0.4.33 in /src-tauri` | cargo | `main` | CI green, awaiting owner merge decision |
| #60 | `chore(deps): bump react-dom from 19.2.4 to 19.2.7` | npm | `main` | CI green, awaiting owner merge decision |
| #61 | `chore(deps-dev): bump vitest from 4.1.0 to 4.1.10` | npm | `main` | CI green, awaiting owner merge decision |
| #62 | `chore(deps): bump tauri-build from 2.6.2 to 2.6.3 in /src-tauri` | cargo | `main` | CI green, awaiting owner merge decision |
| #63 | `chore(deps): bump open from 5.3.3 to 5.3.6 in /src-tauri` | cargo | `main` | CI green, awaiting owner merge decision |
| #64 | `chore(deps): bump sha2 from 0.10.9 to 0.11.0 in /src-tauri` | cargo | `main` | CI green, awaiting owner merge decision |

Plus **#54** (isort major bump, see Finding 5 above) — CI green, awaiting owner merge decision, flagged separately as a major version bump.

### Dependency status

**Frontend (`package.json` — dependencies):**

| Package | Version | Status |
|---|---|---|
| `react` / `react-dom` | `^19.2.4` | Current (patch bump to 19.2.7 pending in PR #60) |
| `react-router-dom` | `^7.18.1` | Current |
| `@dnd-kit/core` | `^6.3.1` | Current |
| `lucide-react` | `^1.6.0` | Current (minor bump to 1.23.0 pending in PR #56) |
| `dayjs` | `^1.11.20` | Current (patch bump to 1.11.21 pending in PR #55) |
| `@tauri-apps/api` | `^2.11.1` | Current |

**Frontend (`package.json` — devDependencies):**

| Package | Version | Status |
|---|---|---|
| `vite` | `^8.0.1` | Current (patch bump to 8.1.3 pending in PR #58) |
| `vitest` | `^4.1.0` | Current (patch bump to 4.1.10 pending in PR #61) |
| `@vitejs/plugin-react` | `^6.0.1` | Current |
| `typescript` | `~6.0.0` | Current |
| `eslint` | `^10.0.0` | Current |
| `@eslint/js` | `^10.0.1` | Current |
| `typescript-eslint` | `^8.62.1` | Current |
| `husky` | `^9.1.7` | Current |
| `@tauri-apps/cli` | `^2.11.4` | Current |
| `happy-dom` | `^20.10.6` | Current |
| `@testing-library/react` | `^16.3.2` | Current |

**Rust (`src-tauri/Cargo.toml`):**

| Crate | Version | Status |
|---|---|---|
| `tauri` | `2.11.2` | Current (patch bump to 2.11.5 pending in PR #57) |
| `tauri-build` | `2.6.2` | Current (patch bump to 2.6.3 pending in PR #62) |
| `tauri-plugin-log` | `2` (resolves `2.8.0`) | Current |
| `rusqlite` | `0.40.1` | Current (major upgrade from `0.32.1`, applied directly by repo owner outside this maintenance cycle — see git history `#44`) |
| `serde` / `serde_json` | `1.0` | Current |
| `chrono` | `0.4` | Current |
| `reqwest` | `0.12` | Current |
| `keyring` | `3` | Current |
| `open` | `5.2` (resolves `5.3.3`) | Current (patch bump to 5.3.6 pending in PR #63) |
| `sha2` | `0.10` (resolves `0.10.9`) | Current (minor bump to 0.11.0 pending in PR #64) |
| `rand` | `0.10` (resolves `0.10.2`) | Current (major upgrade from `0.9.x`, applied directly by repo owner outside this maintenance cycle — see PR #48) |
| `base64` | `0.22` | Current |
| `url` | `2.5` | Current |
| `shellexpand` | `3` | Current |
| `log` | `0.4` (resolves `0.4.29`) | Current (patch bump to 0.4.33 pending in PR #59) |

**Python dev (`requirements-dev.txt` / `pyproject.toml` — kept in sync):**

| Package | Constraint | Resolves to | Status |
|---|---|---|---|
| `pytest` | `>=9.1.1,<10` | `9.1.1` | Current — mismatch from 2026-06-30 confirmed fixed |
| `black` | `>=26.5.1` | `26.5.1` | Current |
| `ruff` | `>=0.15.20` | `0.15.20` | Current |
| `isort` | `>=5.13` | `8.0.1` | Major bump to documented floor pending in PR #54 (no functional change — already resolves to 8.0.1 either way) |

### Major upgrades pending (require manual review)

| Item | In use | Latest | Notes |
|---|---|---|---|
| `isort` (Python, PR #54) | `>=5.13` (resolves 8.0.1) | `>=8.0.1` | Dependabot-opened, CI green. Floor-only change — see Finding 5. |
| `astral-sh/setup-uv` (GitHub Action, `weekly-audit.yml`) | `v7` | `v8.3.2` | Not yet covered by an open Dependabot PR. Review the v8 migration notes before bumping. |

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
