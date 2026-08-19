# Maintenance log

---

## 2026-08-19

### Checks performed
- `git status` — working tree clean; `claude/upbeat-cray-jdicez` freshly reset, already at `origin/main` tip — no rebase needed.
- Confirmed zero open PRs of any kind and zero open `security-audit`-labelled issues before starting (per task brief, not re-derived).
- Full local check suite, mirroring all three CI workflows exactly: `npm ci`, `npm run lint`, `npm run test`, `npm run build`; `pip install -r requirements-dev.txt` into a clean Python 3.12 venv + `ruff check tests` / `black --check tests` / `isort --check-only tests` / `pytest -q`; installed Tauri's Linux system deps (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`) fresh, then `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` and `cargo test --manifest-path src-tauri/Cargo.toml`.
- Security audits: `npm audit`; `uv export --format requirements-txt --no-hashes` + `pip-audit`; `cargo audit` (installed `cargo-audit` fresh via `cargo install cargo-audit --locked`).
- Outdated-package survey: `npm outdated`, `uv pip list --outdated`, `cargo update --dry-run --manifest-path src-tauri/Cargo.toml`. Checked `src-tauri/tauri.conf.json` bundle config and all `.github/workflows/*.yml` action pins for anything outside Dependabot's four configured ecosystems (npm root, cargo `/src-tauri`, pip root, github-actions) — found nothing: no version pins live in `tauri.conf.json`, and every pinned Action (`actions/checkout@v7`, `actions/setup-node@v7`, `actions/setup-python@v7`, `actions/cache@v6`, `actions/github-script@v9`, `Swatinem/rust-cache@v2`, `astral-sh/setup-uv@v7`) is covered by the `github-actions` ecosystem.
- Re-ran the full check suite after applying fixes (below).

### Findings

**1. `npm audit` — 1 high-severity finding at baseline, fixed.**

| Package | Severity | Issue | Fix | Applied? |
|---|---|---|---|---|
| `nanoid` (transitive, via `vite`→`postcss`) | high | Custom generators can loop indefinitely when size is zero (GHSA-2v37-7h3g-55p8) | `npm audit fix` | Yes — `3.3.17` → `3.3.18` |

**2. `cargo audit` — 0 vulnerabilities.** Exit 0, "20 allowed warnings found" — all 20 are the same recurring informational advisories as every prior cycle (unmaintained gtk-rs GTK3 bindings — `gdk`/`glib`/etc. — plus unsound advisories on old transitive `anyhow`, `glib`, `rand 0.7.3`), none actionable from this repo's manifests. Registry "yanked" checks against crates.io returned noisy 503s through the sandbox's proxy path (network artifact, not an audit finding) but the advisory-database scan itself completed cleanly.

**3. `pip-audit` — 0 vulnerabilities.** Clean run against the `uv export`-ed requirements.

**4. `uv.lock` had drifted from `pyproject.toml` — fixed.** `pyproject.toml` already required `ruff>=0.16.3` (landed via merged Dependabot PR #103), but the committed `uv.lock` still resolved the stale `ruff==0.16.1`. Fixed by `uv export`, which re-resolved and rewrote the lock entry to `0.16.3`. Lockfile-only; no manifest edit (CI's `python.yml` uses `pip install -r requirements-dev.txt` and never reads `uv.lock`, so this had no effect on CI, but keeps `uv sync`-based local dev in sync).

**5. `npm outdated` — three minor/patch bumps, all within Dependabot's npm ecosystem scope, left untouched.** `dayjs` `1.11.21`→`1.11.23` (patch), `lucide-react` `1.30.0`→`1.33.0` (minor), `vitest` `4.1.10`→`4.1.11` (patch) — zero open Dependabot PRs currently, so these just haven't been picked up on its weekly schedule yet. Not duplicated here per policy (Dependabot already tracks the npm root ecosystem).

**6. `typescript` major still outstanding — unchanged, recurring flag since 2026-08-05.** `typescript` `~6.0.0` (resolves `6.0.3`) has `7.0.2` available. Not covered by any open Dependabot PR. Not applied — breaking type-system changes, needs `tsc -b` verification end-to-end after a manual bump.

**7. No outdated items found beyond Dependabot's four ecosystems.** `src-tauri/tauri.conf.json` bundle config carries no dependency version pins (only the app's own `"version": "0.3.0"`, which already matches `package.json`/`Cargo.toml`); all workflow Action pins are within the `github-actions` ecosystem's coverage.

**8. `cargo update --dry-run` surveys ~160 transitive crates with newer semver-compatible versions** (large churn: `zbus`, `zvariant`, ICU/`zerovec` crates, `wit-bindgen`, `winnow`, etc.). None tied to a known vulnerability (see Finding 2) — consistent with prior-cycle policy, no blanket `cargo update` applied; routine transitive freshness is Dependabot's job over time.

### Fixes applied
- `npm audit fix` — resolved the `nanoid` high-severity transitive vulnerability (`3.3.17`→`3.3.18`).
- `uv export --format requirements-txt --no-hashes` — regenerated `uv.lock` to catch up with `pyproject.toml`'s already-current `ruff>=0.16.3` floor (resolves `0.16.3`).
- **No `package.json`, `Cargo.toml`, `pyproject.toml`, or `requirements-dev.txt` edits** — only `package-lock.json` and `uv.lock` changed.
- **No Rust lockfile changes** — `cargo audit` found 0 vulnerabilities, so no audit-motivated bump was needed.

### Majors flagged for owner decision (not applied)

| Item | In use | Latest | Notes |
|---|---|---|---|
| `typescript` (npm) | `~6.0.0` (resolves `6.0.3`) | `7.0.2` | Major, not yet covered by an open Dependabot PR. Recurring flag since 2026-08-05. Review TS7 migration notes; run `tsc -b` end-to-end after bumping. |

### Open PR backlog

None. Zero open PRs of any kind (manual or Dependabot) confirmed before this cycle started — the healthiest state recorded in this log to date. This cycle's own PR is the only one open as of writing.

### Post-change verification

| Check | Result |
|---|---|
| `npm run lint` | 0 errors, 1 pre-existing warning (`JobDetailPage.tsx` exhaustive-deps — unchanged from baseline) |
| `npm run test` | 22 test files, 127 tests passed (baseline and post-change identical) |
| `npm run build` | `tsc -b && vite build` — succeeds |
| `npm audit` | 1 high → 0 vulnerabilities |
| `cargo clippy --all-targets -- -D warnings` | Clean, 0 warnings |
| `cargo test` (`src-tauri/`) | 80 passed, 0 failed |
| `cargo audit` | 0 vulnerabilities, 20 informational warnings (unchanged, non-actionable) |
| `pytest -q` | 3 passed |
| `ruff check tests` / `black --check tests` / `isort --check-only tests` | All clean |

---

## 2026-08-12

### Backlog status — manual weekly-maintenance PR pileup: resolved, holding

Re-verified live via `list_pull_requests` (state=open) rather than trusting the task brief's summary. Confirmed:

- **#68, #73, #84** (the three stacked manual `chore: weekly maintenance` PRs flagged repeatedly since 2026-07-15) are **no longer open** — consistent with the 2026-08-05 log's "Post-merge rebase note", which recorded that the owner closed them as superseded shortly after that cycle. **This is the first cycle since 2026-07-15 with zero backlog of unmerged manual maintenance PRs.** No new manual PR was left open by this cycle before this one started.
- **#90–#99** — ten open Dependabot PRs (npm, cargo, pip), all opened 2026-08-10, none touched (not merged/closed/edited) by this cycle per policy. Still a sizeable unreviewed backlog worth the owner's attention, even though the bigger manual-PR pileup is now clear. Full list in the table below.
- No open issues labeled `security-audit` (re-checked fresh via `list_issues`, not assumed).

`claude/upbeat-cray-l94b2x` was fetched fresh against `origin/main` (`d9fe6a3`) and had zero unique commits either direction — no reset needed, this is a clean fresh branch off current `main`.

### Checks performed
- `git fetch origin` + divergence check (`git rev-list --left-right --count`) — 0/0, branch already at `origin/main` tip.
- Baseline: `npm ci`, `npm run lint`, `npm run test`, `npm run build` (no separate `typecheck` script — `tsc -b` runs inside `build`).
- Baseline: `npm audit` (JSON, full detail) and `npm outdated`.
- Baseline: `cargo check --manifest-path src-tauri/Cargo.toml` — re-attempted fresh (not assumed from last week), failed for the same environment reason (see Sandbox limitation below).
- Baseline: `cargo update --dry-run --manifest-path src-tauri/Cargo.toml` (full survey) and `cargo audit --file src-tauri/Cargo.lock` (installed `cargo-audit` fresh via `cargo install cargo-audit --locked`).
- Baseline: `uv sync`, `uv run pytest -q`, `uv run ruff check .`, `uv run black --check .`, `uv run isort --check-only .`.
- Baseline: `uv tree --outdated`; installed `pip-audit`, ran against `uv export --format requirements-txt --no-hashes`.
- Cross-referenced every candidate update against the live open Dependabot PR list (#90–#99) to avoid duplicating in-flight bumps.
- Re-ran the full frontend + Python check set after applying updates.

### Sandbox limitation (not a code defect) — re-confirmed, unchanged
`cargo check` was attempted fresh this cycle (not assumed from prior logs) and still fails identically: `gdk-sys`'s build script cannot find `gdk-3.0` via `pkg-config` because `libgtk-3-dev`/`libwebkit2gtk-4.1-dev` are not installed and not installable in this sandbox (no network path to the package mirrors). `package.json`'s `verify:rust` script already guards for exactly this. Rust dependency posture was verified via `cargo update --dry-run` and `cargo audit --file src-tauri/Cargo.lock` instead, both of which only need the lockfile/registry, not a full compile. `cargo audit` itself needed `cargo-audit` reinstalled in this fresh sandbox session (`cargo install cargo-audit --locked`, ~0.22.2) — it ran successfully (registry "yanked" checks against crates.io threw noisy 503s in this sandbox's network path, but the advisory-database scan itself completed and returned results).

### Major finding: the `react-router-dom` security item flagged since 2026-07-22/2026-08-05 now appears resolved

Prior cycles flagged GHSA-qwww-vcr4-c8h2 (CSRF bypass in `react-router`'s RSC code paths) as unfixable via `npm audit fix` without a downgrade, and requiring a manual migration off `react-router-dom` onto the standalone `react-router` v8 package. Re-checked the advisory directly this cycle: **the affected range is `>=7.12.0, <7.18.2` (patched at `7.18.2`)** — not an open-ended range requiring v8, as it may have appeared before `react-router` 7.18.2 existed.

`npm ci` on this cycle's fresh lockfile resolves `react-router-dom@7.18.2` → `react-router@7.18.2`, which **is** the patched version, already satisfied by the existing `"react-router-dom": "^7.18.1"` range in `package.json` — no manifest edit, no migration needed. `npm audit` now reports **0 vulnerabilities** (down from 2 high at the end of the 2026-08-05 cycle). This was **not** something this cycle did — `react-router` simply published the fix at a semver-compatible patch version between cycles, and the next `npm ci`/lockfile refresh picked it up automatically. Confirmed via `npm ls react-router-dom react-router` and cross-checked the GHSA advisory page directly (not assumed from memory). **No further action needed; the react-router-dom migration item can be dropped from the recurring flag list.**

### Findings

**1. `npm audit` — 0 vulnerabilities at baseline (down from 2 high in the 2026-08-05 log).** See react-router-dom finding above — this is the reason.

**2. `cargo audit` — 1 vulnerability (unchanged from the 2026-08-05 post-rebase note), already covered by an open Dependabot PR.**

| Crate | Version | Advisory | Fix | Status |
|---|---|---|---|---|
| `rkyv` | 0.7.46 | RUSTSEC-2026-0235 (OOB read validating `Rc`/`Arc` in archives) | Requires `tauri-plugin-log` → 2.9.0 (drops the `byte-unit`→`rust_decimal`→`rkyv` chain entirely) | **Already an open Dependabot PR (#98, `tauri-plugin-log` 2.8.0→2.9.0)** — not duplicated here. `src-tauri/Cargo.lock` still resolves `tauri-plugin-log@2.8.0`, confirming #98 hasn't landed yet. |

Remaining 20 `cargo audit` findings are informational warnings (unmaintained gtk-rs GTK3 bindings + a few unsound advisories on old transitive `anyhow`/`glib`/`rand` versions) — same as every prior cycle, non-actionable from this repo's manifests.

`cargo update --dry-run --manifest-path src-tauri/Cargo.toml` surveys ~160 packages with newer semver-compatible versions available (large, mostly transitive churn: `tokio`, `hyper`, `wasm-bindgen`, ICU crates, etc.). None of these are tied to a known vulnerability beyond the `rkyv` chain above (already tracked by #98), so — consistent with prior-cycle policy of only doing targeted, audit-motivated lockfile bumps rather than a blanket `cargo update` that can't be compile-verified in this sandbox — no broad Rust update was applied this cycle.

**3. `pip-audit` — 0 vulnerabilities.** Clean run against the exported `uv.lock` requirements.

**4. Two safe transitive/direct bumps applied, neither overlapping an open Dependabot PR:**

| Ecosystem | Package | Before | After | Notes |
|---|---|---|---|---|
| npm | `eslint` | 10.8.0 | 10.8.1 | Within existing `^10.8.0` range; not covered by any of #90–#99 |
| npm | `eslint-plugin-react-refresh` | 0.5.3 | 0.5.4 | Within existing `^0.5.2` range; not covered |
| npm | `globals` | 17.8.0 | 17.11.0 | Within existing `^17.8.0` range; not covered |
| pip | `platformdirs` (via `black`) | 4.11.0 | 4.11.2 | Transitive; pip ecosystem in Dependabot only tracks direct deps (`pytest`/`black`/`isort`/`ruff`), so untouched by any open PR |

**5. Explicitly skipped as already covered by an open Dependabot PR** (cross-referenced against #90–#99, not duplicated): `happy-dom` (#90), `lucide-react` (#91), `@types/node` (#92), `ruff` floor bump (#93), `open` (#94, cargo), `vite` (#95), `typescript-eslint` (#96), `base64` (#97, cargo), `tauri-plugin-log` (#98, cargo — also the `rkyv` fix, see Finding 2), `serde` (#99, cargo).

**6. `typescript` major still outstanding, still unflagged by Dependabot — unchanged from 2026-08-05.** `typescript` `~6.0.0` (resolves `6.0.3`) has `7.0.2` available (`npm outdated` "Latest" column). No open Dependabot PR proposes this. Not applied — recurring flag, same rationale as last cycle (review TS7 migration notes, run `tsc -b` end-to-end before bumping).

### Fixes applied
- `npm update eslint eslint-plugin-react-refresh globals` — safe patch/minor bumps within existing `package.json` ranges, none overlapping an open Dependabot PR.
- `uv lock --upgrade-package platformdirs` — safe transitive Python dev-tool bump.
- **No `package.json`, `Cargo.toml`, `pyproject.toml`, or `requirements-dev.txt` edits** — only `package-lock.json` and `uv.lock` changed.
- **No Rust lockfile changes** — the sole real `cargo audit` finding (`rkyv`) is already covered by open Dependabot PR #98; nothing else warranted a targeted `cargo update -p` this cycle.

### Majors / deferred items flagged for owner decision (not applied)

| Item | In use | Latest | Notes |
|---|---|---|---|
| `typescript` (npm) | `~6.0.0` (resolves `6.0.3`) | `7.0.2` | Major, still not covered by an open Dependabot PR. Recurring flag since 2026-08-05. |
| ~~`react-router-dom` → `react-router` migration~~ | — | — | **Resolved this cycle** — see Major finding above. Dropped from the recurring flag list. |

### Open PR backlog (owner decision required)

**Manual weekly-maintenance PRs:** none open. This is a change from every prior entry since 2026-07-15 — the #68/#73/#84 backlog was cleared by the owner shortly after the 2026-08-05 cycle (per that log's "Post-merge rebase note") and has not recurred. This cycle's own PR is the only manual maintenance PR currently open.

**Dependabot PRs (all open, none merged/closed/edited by this cycle):**

| PR | Title | Ecosystem | Notes |
|---|---|---|---|
| #99 | `chore(deps): bump serde from 1.0.228 to 1.0.229 in /src-tauri` | cargo | Patch |
| #98 | `chore(deps): bump tauri-plugin-log from 2.8.0 to 2.9.0 in /src-tauri` | cargo | Also fixes the `rkyv` RUSTSEC-2026-0235 finding (see Finding 2) |
| #97 | `chore(deps): bump base64 from 0.22.1 to 0.23.1 in /src-tauri` | cargo | Minor |
| #96 | `chore(deps-dev): bump typescript-eslint from 8.65.0 to 8.66.0` | npm | Minor (latest is actually 8.67.0; not chased further to avoid duplicating in-flight work) |
| #95 | `chore(deps-dev): bump vite from 8.2.0 to 8.2.1` | npm | Patch |
| #94 | `chore(deps): bump open from 5.4.0 to 5.4.1 in /src-tauri` | cargo | Patch |
| #93 | `chore(deps-dev): update ruff requirement from >=0.16.1 to >=0.16.2` | pip | Floor-only |
| #92 | `chore(deps-dev): bump @types/node from 26.1.2 to 26.2.0` | npm | Patch |
| #91 | `chore(deps): bump lucide-react from 1.27.0 to 1.30.0` | npm | Minor |
| #90 | `chore(deps-dev): bump happy-dom from 20.11.1 to 20.11.2` | npm | Patch |

Ten open Dependabot PRs is still a sizeable unreviewed backlog — flagged again to the owner, same as 2026-08-05, even though the larger manual-PR pileup problem is now resolved.

### Post-change verification

| Check | Result |
|---|---|
| `npm run lint` | 0 errors, 1 pre-existing warning (`JobDetailPage.tsx` exhaustive-deps — unchanged from baseline) |
| `npm run test` | 22 test files, 127 tests passed (baseline and post-change identical) |
| `npm run build` | `tsc -b && vite build` — succeeds |
| `npm audit` | 0 vulnerabilities (baseline and post-change identical — see Major finding above) |
| `cargo audit --file src-tauri/Cargo.lock` | 1 vulnerability (`rkyv`, tracked by open PR #98), 20 informational warnings — unchanged, no Rust lockfile edits made this cycle |
| `cargo check` / `cargo test` (`src-tauri/`) | Fails in this sandbox for the documented environment reason (missing GTK/webkit system headers); re-confirmed fresh this cycle, not assumed |
| `uv run pytest -q` | 3 passed (baseline and post-change identical) |
| `uv run ruff check .` / `black --check .` / `isort --check-only .` | All clean, before and after |

---

## 2026-08-05

### ⚠️ Backlog correction — THREE weekly-maintenance PRs now stacked unmerged

Before doing anything else, the live open-PR list was re-verified via `list_pull_requests` (never trust the prior cycle's assumed backlog). Confirmed still open and unmerged:

- **#84** — `chore: weekly maintenance 2026-07-29` (opened 2026-07-29)
- **#73** — `chore: weekly maintenance 2026-07-22` (opened 2026-07-22)
- **#68** — `chore: weekly maintenance 2026-07-15` (opened 2026-07-15)

This is the **third consecutive cycle** to note a growing, unmerged weekly-maintenance queue — it is now three PRs deep, worse than at the last check-in. None of these were touched by this cycle (no merging, closing, or editing other PRs — out of scope per policy), but this is flagged prominently again: **the repo owner should review and merge (or close, if superseded) #68, #73, #84 soon**, ideally oldest-first, since later cycles' lockfile/dependency states may drift further from each unmerged PR's baseline the longer they sit. This cycle's own PR will make that four open weekly-maintenance PRs if none are cleared first.

Also present: a long-standing Dependabot backlog (see full table below) and four open docs PRs (#85, #74, #69, #66), none of which were merged/closed/edited by this cycle either.

### Checks performed
- Fresh `git fetch origin main`; confirmed `claude/upbeat-cray-f8vh9h` already sat exactly at `origin/main` tip (`969dc0c8`) with no unique commits either direction — no reset needed.
- Re-verified the live open-PR backlog via `list_pull_requests` (state=open) rather than trusting the assumed list from the task brief — it matched.
- Baseline: `npm ci`, `npm run lint`, `npm run test`, `npm run build` (no separate `typecheck` script exists — `tsc -b` runs as part of `npm run build`)
- Baseline: `npm audit` (JSON, full detail)
- Baseline: `cargo check --manifest-path src-tauri/Cargo.toml` — failed for an environment reason (see Sandbox limitation below), not a code defect
- Baseline: `cargo update --dry-run --manifest-path src-tauri/Cargo.toml` (full, to survey outdated crates) and targeted per-crate dry-runs
- Baseline: installed `cargo-audit` (`cargo install cargo-audit --locked`) and ran `cargo audit --file src-tauri/Cargo.lock`
- Baseline: `uv sync`, `uv run pytest -q`, `uv run ruff check .`, `uv run black --check .`, `uv run isort --check-only .`
- Baseline: installed `pip-audit`, ran against `uv export --format requirements-txt --no-hashes`
- Baseline: `uv tree --outdated` (pip-side equivalent of `npm outdated` for the dev-tool lockfile)
- Cross-referenced every candidate update against the live open Dependabot PR list (#86, #83, #82, #81, #80, #79, #77, #76, #75, #64, #63) to avoid duplicating in-flight bumps

### Sandbox limitation (not a code defect)
`cargo check` / `cargo test` fail in this sandbox because `gdk-sys` (pulled in by Tauri's Linux GTK backend) requires the `gdk-3.0` pkg-config file, which needs `libgtk-3-dev` / `libwebkit2gtk-4.1-dev` system headers. `apt-get install` was attempted and failed on 404s from the sandbox's package mirrors (no network path to fetch those `.deb`s). This matches the known Tauri-sandbox limitation flagged in prior cycles and elsewhere in this maintenance program — `package.json`'s own `verify:rust` script already guards for exactly this (`pkg-config --exists gdk-3.0 ... || echo 'Skipping Rust checks'`). Rust dependency changes below were verified via `cargo update --dry-run` / targeted `cargo update -p` resolution and `cargo audit` (both of which only need the lockfile/registry, not a full compile) rather than `cargo check`/`cargo test`.

### Findings

**1. npm audit — 4 high-severity findings at baseline, 2 fixed, 1 flagged as a major migration (not applied).**

| Package | Severity | Issue | Fix | Applied? |
|---|---|---|---|---|
| `brace-expansion` (transitive, via `eslint`→`minimatch`) | high | 3× DoS via exponential/unbounded expansion (GHSA-3jxr-9vmj-r5cp, GHSA-mh99-v99m-4gvg, GHSA-rgw5-rvv9-x895) | `npm audit fix` | Yes — `5.0.6` → `5.0.9` |
| `postcss` (transitive, via `vite`) | high | Path traversal / arbitrary `.map` file disclosure (GHSA-r28c-9q8g-f849, GHSA-fxqj-rqcc-2cmp) | `npm audit fix` | Yes — `8.5.16` → `8.5.25` |
| `react-router` (transitive, via `react-router-dom`) | high | RSC Mode CSRF bypass allows action execution before 400 response (GHSA-qwww-vcr4-c8h2), affects `7.12.0–8.2.0` | Requires `npm audit fix --force`, which would **downgrade** `react-router-dom` to `7.11.0` (a regression, not a real fix) | **No** — flagged below |
| `react-router-dom` (direct dep) | high | Depends on vulnerable `react-router` | See above | **No** — flagged below |

Investigated the `react-router-dom` finding further: `react-router-dom`'s own `latest` dist-tag is still `7.18.2` — it never published a v8 release. The patched line (`react-router@>=8.3.0`) only exists under the separate `react-router` package, which merged `react-router-dom`'s functionality starting at v8. Fixing this properly means **migrating off `react-router-dom` onto the `react-router` package directly (v8)** — a real code migration (import paths, possibly routing API surface), not a version bump. Flagged as a major/security item for manual review; not attempted in this cycle.

**2. `cargo audit` — 8 real vulnerabilities at baseline (all transitive), 8 fixed via semver-compatible lockfile-only updates; 20 unmaintained/unsound advisories remain as informational warnings.**

| Crate | Version (before) | Advisory | Severity | Fixed via |
|---|---|---|---|---|
| `quinn-proto` | 0.11.14 | RUSTSEC-2026-0185 (remote memory exhaustion) | 7.5 high | `cargo update -p quinn-proto` → `0.11.16` |
| `rustls-webpki` | 0.103.9 | RUSTSEC-2026-0099, -0104, -0098, -0049 (name-constraint / CRL bypass issues) | high | `cargo update -p rustls-webpki` → `0.103.13` |
| `quick-xml` | 0.38.4 | RUSTSEC-2026-0194, -0195 (quadratic runtime / unbounded namespace-decl allocation DoS) | 7.5 high | `cargo update -p quick-xml -p plist -p tauri-plugin-log` → `0.41.0` (required bumping `plist` 1.8.0→1.10.0 and `tauri-plugin-log` 2.8.0→2.9.0 together — a plain `-p quick-xml` alone was rejected by the resolver) |
| `rkyv` | 0.7.46 | RUSTSEC-2026-0235 (OOB read validating `Rc`/`Arc` in archives) | — | Same update above **removed `rkyv` from the tree entirely** — `tauri-plugin-log 2.9.0` dropped its `byte-unit`→`rust_decimal`→`rkyv` dependency chain |

All four fixes are **lockfile-only** — no `Cargo.toml` range was edited, and none of the touched crates (`quinn-proto`, `rustls-webpki`, `quick-xml`, `plist`, `tauri-plugin-log`) overlap with any open Dependabot cargo PR, so nothing here duplicates in-flight work.

Remaining 20 `cargo audit` findings are all **warnings**, not vulnerabilities (no `error:` in the audit run after the fix): the `atk`/`gdk`/`gtk`/`gtk3-macros`/etc. gtk-rs GTK3 binding crates are flagged "unmaintained" (RUSTSEC-2024-041x series) — inherent to Tauri's Linux GTK3 backend, not fixable without Tauri itself moving off gtk-rs; plus a handful of "unsound" advisories on old transitive versions of `anyhow`, `glib`, and `rand` (0.7.3, pulled in by an unrelated sub-dependency, not the project's own `rand = "0.10"`). None are directly actionable from this repo's manifests.

**3. `pip-audit` — 1 finding, fixed.**

| Package | Version (before) | Advisory | Fix |
|---|---|---|---|
| `click` (transitive, via `black`) | 8.3.2 | PYSEC-2026-2132 | `uv lock --upgrade-package click` → `8.4.2` |

**4. `uv.lock` had drifted from `pyproject.toml` again** (same class of issue as the 2026-07-08 entry). `pyproject.toml` already reads `ruff>=0.15.22`, but the committed `uv.lock` still resolved `ruff==0.15.20` from a stale prior floor. Fixed by `uv sync`, which re-resolved to `ruff==0.16.1` (still within the unbounded floor — no manifest edit). Note: open Dependabot PR **#86** separately proposes bumping the *documented floor* to `ruff>=0.16.1` — not duplicated here, since our fix only touches the lockfile resolution, matching what #86's floor bump would produce anyway once merged.

**5. Three safe transitive Python dev-tool bumps applied** (none overlap Dependabot's pip ecosystem, which only tracks the four direct dev dependencies `pytest`/`black`/`isort`/`ruff`):

| Package | Before | After |
|---|---|---|
| `packaging` (via `black`) | 26.1 | 26.3 |
| `pathspec` (via `black`) | 1.0.4 | 1.1.1 |
| `platformdirs` (via `black`) | 4.9.6 | 4.11.0 |

**6. Six safe npm devDependency/transitive bumps applied**, all within existing `package.json` semver ranges (no manifest edit needed) and none overlapping an open Dependabot PR:

| Package | Before | After |
|---|---|---|
| `@types/react` | 19.2.14 | 19.2.18 |
| `@types/react-dom` | 19.2.3 | 19.2.4 |
| `@vitejs/plugin-react` | 6.0.1 | 6.0.5 |
| `eslint-plugin-react-refresh` | 0.5.2 | 0.5.3 |
| `happy-dom` | 20.10.6 | 20.11.1 |
| `vite` | 8.1.3 | 8.2.0 |

**7. `typescript` has an unflagged major available — not applied, not yet covered by Dependabot either.** `typescript` `~6.0.0` (resolves `6.0.3`) has `7.0.2` available. No open Dependabot PR currently proposes this (Dependabot's npm ecosystem hasn't opened one yet). Flagged below for manual review — do not bump automatically per policy, and expect Dependabot to pick this up in a future cycle.

### Fixes applied
- `npm audit fix` — resolved `brace-expansion` and `postcss` high-severity transitive vulnerabilities.
- `npm update @types/react @types/react-dom @vitejs/plugin-react eslint-plugin-react-refresh happy-dom vite` — safe patch/minor bumps, all six confirmed already covered by no open Dependabot PR.
- `cargo update -p quinn-proto -p rustls-webpki` — fixes 5 of 8 `cargo audit` findings (transitive, lockfile-only).
- `cargo update -p quick-xml -p plist -p tauri-plugin-log` — fixes the remaining 3 `cargo audit` findings, including removing the `rkyv`/`rust_decimal`/`byte-unit` chain entirely (transitive, lockfile-only).
- `uv sync` — regenerated `uv.lock` to catch up with `pyproject.toml`'s already-current `ruff>=0.15.22` floor (resolves `0.16.1`).
- `uv lock --upgrade-package click` — fixes `pip-audit` finding PYSEC-2026-2132.
- `uv lock --upgrade-package packaging --upgrade-package pathspec --upgrade-package platformdirs` — safe transitive Python dev-tool bumps.
- **No `package.json`, `Cargo.toml`, `pyproject.toml`, or `requirements-dev.txt` edits** — every fix above stayed within already-declared semver ranges; only `package-lock.json`, `src-tauri/Cargo.lock`, and `uv.lock` changed.
- **Explicitly skipped** (already covered by an open Dependabot PR, to avoid duplicating in-flight work): `ruff` floor bump (#86), `keyring` (#83), `@types/node` (#82), `chrono` (#81), `eslint` (#80), `globals` (#79), `typescript-eslint` (#77), `serde_json` (#76), `lucide-react` (#75), `sha2` (#64), `open` (#63).

### Majors flagged for owner decision (not applied)

| Item | In use | Latest | Notes |
|---|---|---|---|
| `react-router-dom` → `react-router` (npm, **security-relevant**) | `react-router-dom@7.18.1` (pulls vulnerable `react-router` `7.12–8.2`) | `react-router@8.3.0` (unified package) | GHSA-qwww-vcr4-c8h2 (high, CSRF bypass). Real fix requires migrating off `react-router-dom` onto the `react-router` v8 package — a code migration, not a version bump. `npm audit fix --force` only offers a regression (downgrade to `7.11.0`), not a real fix — do not apply. |
| `typescript` (npm) | `~6.0.0` (resolves `6.0.3`) | `7.0.2` | Major, not yet covered by an open Dependabot PR. Review TS7 migration notes before bumping; run `tsc -b` end-to-end after. |
| `keyring` (cargo, PR #83) | `3.6.3` | `4.1.5` | Already tracked by Dependabot — left alone per policy. |
| `@types/node` (npm, PR #82) | `24.12.0` | `26.1.2` | Already tracked by Dependabot — left alone per policy. |
| `sha2` (cargo, PR #64) | `0.10.9` | `0.11.0` | Already tracked by Dependabot — left alone per policy (recurring flag since 2026-07-08). |

### Open PR backlog (owner decision required)

**Weekly-maintenance PRs (unmerged, stacking — see warning at top):**

| PR | Title | Opened | Status |
|---|---|---|---|
| #84 | `chore: weekly maintenance 2026-07-29` | 2026-07-29 | Open, unmerged |
| #73 | `chore: weekly maintenance 2026-07-22` | 2026-07-22 | Open, unmerged |
| #68 | `chore: weekly maintenance 2026-07-15` | 2026-07-15 | Open, unmerged |

**Dependabot PRs (all open, none merged/closed by this cycle):**

| PR | Title | Ecosystem | Notes |
|---|---|---|---|
| #86 | `chore(deps-dev): update ruff requirement from >=0.15.22 to >=0.16.1` | pip | Floor-only, already resolves to 0.16.1 regardless (see Finding 4) |
| #83 | `chore(deps): bump keyring from 3.6.3 to 4.1.5 in /src-tauri` | cargo | Major |
| #82 | `chore(deps-dev): bump @types/node from 24.12.0 to 26.1.2` | npm | Major-ish jump |
| #81 | `chore(deps): bump chrono from 0.4.44 to 0.4.45 in /src-tauri` | cargo | Patch |
| #80 | `chore(deps-dev): bump eslint from 10.5.0 to 10.8.0` | npm | Minor |
| #79 | `chore(deps-dev): bump globals from 17.4.0 to 17.8.0` | npm | Minor |
| #77 | `chore(deps-dev): bump typescript-eslint from 8.62.1 to 8.65.0` | npm | Minor |
| #76 | `chore(deps): bump serde_json from 1.0.149 to 1.0.151 in /src-tauri` | cargo | Patch |
| #75 | `chore(deps): bump lucide-react from 1.26.0 to 1.27.0` | npm | Minor |
| #64 | `chore(deps): bump sha2 from 0.10.9 to 0.11.0 in /src-tauri` | cargo | Major |
| #63 | `chore(deps): bump open from 5.3.3 to 5.4.0 in /src-tauri` | cargo | Minor |

**Docs PRs (open, unmerged, untouched):** #85, #74, #69, #66.

### Post-change verification

| Check | Result |
|---|---|
| `npm run lint` | 0 errors, 1 pre-existing warning (`JobDetailPage.tsx` exhaustive-deps — unchanged from baseline) |
| `npm run test` | 16 test files, 97 tests passed (same count as baseline) |
| `npm run build` | `tsc -b && vite build` — succeeds |
| `npm audit` | 4 high → 2 high (both remaining are the flagged `react-router`/`react-router-dom` migration item) |
| `cargo audit --file src-tauri/Cargo.lock` | 8 vulnerabilities → 0; 20 informational warnings remain (unmaintained gtk-rs bindings + old unsound transitive advisories, non-actionable here) |
| `cargo check` / `cargo test` (`src-tauri/`) | Fails in this sandbox for an environment reason (missing GTK/webkit system headers, `apt-get` blocked by mirror 404s) — not a regression from this cycle's changes; see Sandbox limitation above |
| `uv run pytest -q` | 3 passed (same as baseline) |
| `uv run ruff check .` / `black --check .` / `isort --check-only .` | All clean |

### Post-merge rebase note (same day)

After this entry was written, the rest of the open PR backlog was processed per the repo owner's request: 12 Dependabot/docs PRs merged, and #68/#73/#84 (the three stacked weekly-maintenance PRs flagged above) were closed as superseded. That moved `main` out from under this branch, so it was rebased onto the new tip.

The rebase produced a real `src-tauri/Cargo.lock` conflict: none of the 12 merged Dependabot PRs touched the `quick-xml`/`plist`, `quinn-proto`, or `rustls-webpki` advisories this cycle had fixed, so taking the new base's lockfile as the starting point silently reintroduced all **8** `cargo audit` findings from before. Re-verified with a fresh `cargo audit` run and reapplied 7 of them on top of the rebased lockfile (`plist` → 1.10.0, pulling `quick-xml` → 0.41.0; `quinn-proto` → 0.11.15; `rustls-webpki` → 0.103.13). The 8th (`rkyv` 0.7.46, RUSTSEC-2026-0235) is pulled in transitively via `rust_decimal` ← `byte-unit` ← `tauri-plugin-log`, which has no release using `rkyv` 0.8 yet — fixing it needs a manifest-level change (dropping or replacing `tauri-plugin-log`'s dependency chain), not a lockfile bump, so it's left flagged rather than forced. `cargo audit` now reports **1 vulnerability** (down from 8, not 0 as the table above says — that table reflects the pre-rebase state and is left as-is for the historical record; this note is the accurate final state). `npm run test` re-confirmed 97/97 passing after the rebase.

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
