# Job Tracker

[![CI](https://github.com/eikrad/Job-Tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/eikrad/Job-Tracker/actions/workflows/ci.yml)

Desktop app (**Tauri** + **React** + local **SQLite**) to track job applications, deadlines, application PDFs, and optional Gemini-assisted extraction.

## Prerequisites

- **Node.js** 20+ and npm
- **Rust** stable (`rustup`, `cargo`)
- **OS packages for Tauri** (WebKit + GTK on Linux). See the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

**Arch Linux** (example):

```bash
sudo pacman -S --needed base-devel curl wget openssl gtk3 libappindicator-gtk3 librsvg webkit2gtk-4.1 patchelf
```

## Quick start (how to run)

```bash
git clone https://github.com/eikrad/Job-Tracker.git
cd Job-Tracker
npm ci
npm run tauri:dev
```

This starts the Vite dev server and opens the **desktop window** (full app: SQLite, file storage, Tauri commands).

**Browser-only UI** (no database / no native features):

```bash
npm ci
npm run dev
```

Then open the URL Vite prints (e.g. `http://localhost:5173`). Use this only for quick UI tweaks.

**Production-style desktop build:**

```bash
npm ci
npm run tauri:build
```

Installable artifacts appear under `src-tauri/target/release/` (platform-dependent).

## Configuration

1. Copy [`fake.env`](fake.env) to `.env` if you want file-based config (optional).
2. **Gemini**: set `GEMINI_API_KEY` in `.env` or paste the key in the app (stored in browser local storage for that build).

## Google Calendar (API)

- **Template link**: opens Google Calendar compose (no OAuth).
- **Create via API**: paste an OAuth **access token** with scope `https://www.googleapis.com/auth/calendar.events`. Tokens expire; refresh via your OAuth client or [Google OAuth Playground](https://developers.google.com/oauthplayground/) for testing.

## Data storage

- **SQLite and uploaded PDFs** live in the OS app data directory for the Tauri app (not in this repo).
- The repo `storage/` folder is for optional manual files; see `.gitignore`.

## Import / export

- **Export**: JSON or CSV from the app header.
- **Import**: JSON array (same shape as export) or CSV from export; creates **new** rows (IDs are not preserved).

## CI & tests

GitHub Actions runs: `npm run lint`, `npm run test` (Vitest), `npm run build`, `cargo test`, then Python **`ruff check`**, **`black --check`**, **`isort --check-only`**, **`pytest`**.

### Frontend (Vitest)

```bash
npm ci
npm run test        # once
npm run test:watch  # during development
```

### Rust

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

### Python ([Ruff](https://docs.astral.sh/ruff/), [Black](https://black.readthedocs.io/), [isort](https://pycqa.github.io/isort/), [pytest](https://pytest.org/))

[Ruff](https://docs.astral.sh/ruff/) is the primary **linter** (fast, replaces much of Flake8 + plugins). **Black** formats code; **isort** sorts imports with the `black` profile so they agree.

```bash
pip install -r requirements-dev.txt
npm run py:lint    # ruff check + black --check + isort --check-only
npm run py:format  # isort + black (writes files)
npm run py:test    # pytest
```

Tool config: [`pyproject.toml`](pyproject.toml).

> **Forks:** Update the CI badge URL if your repo is not `eikrad/Job-Tracker`.

## Documentation & best practices

| Topic | Suggestion |
|--------|------------|
| **Secrets** | Never commit `.env` or API keys. Prefer env vars or local app storage; rotate keys if leaked. |
| **Pre-merge** | Run `npm run lint && npm run test && npm run build` and `cargo test --manifest-path src-tauri/Cargo.toml`; for Python changes run `npm run py:lint && npm run py:test`. |
| **Commits** | Small, focused commits with clear messages (`feat:`, `fix:`, `chore:`, …). |
| **UI copy** | Keep user-facing strings in English and centralized (see `src/i18n/en.ts`) so adding locales later stays easy. |
| **Tauri vs browser** | Develop features that need the DB or disk in **`npm run tauri:dev`**, not `npm run dev`. |
| **Dependencies** | Use `npm ci` in CI and for reproducible installs; bump lockfile when changing `package.json`. |

The README is intentionally short. For deeper topics, link out: [Tauri v2](https://v2.tauri.app/), [Vite](https://vite.dev/), [React](https://react.dev/).

## Tech stack

- React + TypeScript + Vite
- Tauri 2
- SQLite (rusqlite) in the Rust backend
