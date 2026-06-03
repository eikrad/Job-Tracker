# Maintenance log

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
