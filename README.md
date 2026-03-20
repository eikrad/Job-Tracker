# Job Tracker

[![CI](https://github.com/eikrad/Job-Tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/eikrad/Job-Tracker/actions/workflows/ci.yml)
[![Alpha](https://img.shields.io/badge/stage-alpha-orange.svg)](https://github.com/eikrad/Job-Tracker)

Desktop app (**Tauri** + **React** + local **SQLite**) to track job applications, deadlines, application PDFs, and optional **AI-assisted extraction** (Google **Gemini** or **Mistral**).

Contributing (build, PR checklist, commits): see **[CONTRIBUTING.md](CONTRIBUTING.md)**. After `npm ci`, **pre-commit** runs **`npm run verify`** (lint/tests/build + Rust + Python) so local commits match CI before you push.

## Prerequisites

- **Node.js** 20+ and npm
- **Rust** stable (`rustup`, `cargo`)
- **OS packages for Tauri** (WebKit + GTK on Linux). See the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

**Arch Linux** (example):

```bash
sudo pacman -S --needed base-devel curl wget openssl gtk3 libappindicator-gtk3 librsvg webkit2gtk-4.1 patchelf
```

**Windows**

1. Install **Node.js 20+** (e.g. from [nodejs.org](https://nodejs.org/) or `winget install OpenJS.NodeJS.LTS`).
2. Install **Rust** via [rustup](https://rustup.rs/) (use the `x86_64-pc-windows-msvc` toolchain).
3. Install **Microsoft C++ Build Tools** for the Tauri/Rust native build: open [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and select the **“Desktop development with C++”** workload (or follow the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows)).
4. **WebView2** is bundled on current Windows 10/11; if the app fails to show a window, install the [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

From **PowerShell** or **cmd**, use the same commands as below (`git clone`, `npm ci`, `npm run tauri:dev`, etc.) in the project folder.

## Quick start (how to run)

```bash
git clone https://github.com/eikrad/Job-Tracker.git
cd Job-Tracker
npm ci
npm run tauri:dev
```

This starts the Vite dev server and opens the **desktop window** (full app: SQLite, file storage, Tauri commands). On Windows, run these commands in **PowerShell** or **Command Prompt** from the cloned directory.

In the app: **Dashboard** (Kanban / Table / Calendar) is the home route; **Add job** opens a dedicated page; **Settings** (gear) holds API keys, board column names, and import/export.

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

Installable artifacts appear under `src-tauri/target/release/` (platform-dependent; on Windows e.g. `.exe` / installer under that tree).

## Configuration

1. Copy [`fake.env`](fake.env) to `.env` if you want file-based config (optional).
2. **AI extraction**: choose **Gemini** or **Mistral** in the app and paste the matching API key (stored in local storage for that build). Keys in `.env` are optional for file-based tooling; the desktop UI does not read `.env` for these calls.
   - **Mistral**: sign up at [La Plateforme](https://console.mistral.ai/); the free **Experiment** tier is typically enough for occasional job-text extraction (high monthly token allowance; rate limits apply — see [Mistral help center](https://help.mistral.ai/)). Limits can change; check their current docs.
   - **Gemini**: Google AI Studio API key as before.

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

## Tech stack

- React + TypeScript + Vite
- Tauri 2
- SQLite (rusqlite) in the Rust backend
