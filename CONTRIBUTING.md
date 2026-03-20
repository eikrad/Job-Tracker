# Contributing to Job Tracker

Thanks for helping out. This document describes how to build the project, run checks, and open pull requests.

## Prerequisites

- **Node.js** 20+ and npm
- **Rust** stable (`rustup`, `cargo`)
- **OS packages** required by [Tauri v2](https://v2.tauri.app/start/prerequisites/) (WebKit + GTK on Linux)

On **Arch Linux**, for example:

```bash
sudo pacman -S --needed base-devel curl wget openssl gtk3 libappindicator-gtk3 librsvg webkit2gtk-4.1 patchelf
```

On **Windows**: Node.js 20+, Rust via [rustup](https://rustup.rs/) (`x86_64-pc-windows-msvc`), and **Microsoft C++ Build Tools** with the “Desktop development with C++” workload — see [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows). WebView2 is included on recent Windows 10/11.

## Setup

```bash
git clone https://github.com/eikrad/Job-Tracker.git
cd Job-Tracker
npm ci
```

## Running the app

| Command | Purpose |
|--------|---------|
| `npm run tauri:dev` | Full desktop app (SQLite, native APIs). **Use this for most feature work.** |
| `npm run dev` | Vite only in the browser — UI-only; no Tauri commands or DB. |
| `npm run tauri:build` | Release build (artifacts under `src-tauri/target/release/`). |

## Checks before opening a PR

Run what CI runs locally:

```bash
# Frontend
npm run lint
npm run test
npm run build

# Rust (from repo root)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
cargo test --manifest-path src-tauri/Cargo.toml

# Python (contract tests + style)
pip install -r requirements-dev.txt
npm run py:lint
npm run py:test
```

Fix issues or explain in the PR why something is intentionally skipped.

## Pull requests

1. **Scope:** One logical change per PR when possible (easier review and bisect).
2. **Description:** What changed and why; link issues if any.
3. **Tests:** Add or update tests when behavior changes (Vitest, Rust `#[test]`, or pytest for `tests/`).
4. **Secrets:** Do not commit API keys, `.env` with real values, or personal data.

## Commit messages

Prefer clear, conventional prefixes when it fits:

- `feat:` new user-facing behavior
- `fix:` bug fix
- `chore:` tooling, deps, config
- `docs:` documentation only
- `test:` tests only
- `refactor:` behavior unchanged

Example: `fix: validate import JSON before bulk insert`

## App identifier

The Tauri **bundle identifier** is `com.github.eikrad.jobtracker` (see `src-tauri/tauri.conf.json`). If you fork for your own distribution, change it to a domain you control (e.g. `com.yourname.jobtracker`) to avoid clashes with updates and OS integration.

## Questions

Open a [GitHub issue](https://github.com/eikrad/Job-Tracker/issues) for bugs or design discussion.
