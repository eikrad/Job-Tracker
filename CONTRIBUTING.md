# Contributing to Job Tracker

Thanks for helping out. This document describes how to build the project, run checks, and open pull requests.

## Prerequisites

- **Node.js** 20+ and npm
- **Rust** stable (`rustup`, `cargo`)
- **Python** 3.12+ with **pip** (`python3 -m pip` — on Arch: `sudo pacman -S python-pip`) for contract tests and the `verify` / pre-commit hook
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

The `prepare` script runs **Husky** and wires a **pre-commit** hook. On each commit, `npm run verify` runs automatically (frontend + Rust + Python — same intent as GitHub Actions). If hooks are missing after clone, run `npm run prepare` or `npx husky`.

To run the full suite manually anytime:

```bash
npm run verify
```

Emergency skip (avoid if possible): `HUSKY=0 git commit …`

## Running the app

| Command | Purpose |
|--------|---------|
| `npm run tauri:dev` | Full desktop app (SQLite, native APIs). **Use this for most feature work.** |
| `npm run dev` | Vite only in the browser — UI-only; no Tauri commands or DB. |
| `npm run tauri:build` | Release build (artifacts under `src-tauri/target/release/`). |

## Tauri release bundles (Linux, AppImage)

`npm run tauri:build` compiles the Rust binary and, by default, **bundles** installers according to `bundle.targets` in [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json).

### Artifact layout

- **Binary:** `src-tauri/target/release/<executable name>` (from `package.productName` / Tauri config).
- **Installers:** `src-tauri/target/release/bundle/<format>/` (e.g. `deb/`, `rpm/`, `nsis/`, `msi/`, `macos/`, `dmg/`).

### Why AppImage is off by default

For **AppImage**, the Tauri bundler runs **linuxdeploy**. If `linuxdeploy` is not installed, not on `PATH`, or not marked executable, you get:

```text
failed to run linuxdeploy
```

That blocks the **entire** `tauri build` when `"appimage"` (or `"all"` on Linux) is requested. This repo therefore lists explicit targets (**deb**, **rpm**, plus Windows/macOS formats) instead of `"all"`, so a normal Linux contributor build does not require linuxdeploy.

### Enabling AppImage

1. Install **linuxdeploy** and the **AppImage plugin** as described in the official guide: [Tauri — AppImage](https://v2.tauri.app/distribute/appimage/).
2. Ensure both are on **`PATH`** and executable (`chmod +x` if you downloaded release binaries).
3. Add `"appimage"` to the `bundle.targets` array in `src-tauri/tauri.conf.json`.
4. Run `npm run tauri:build` again; expect output under `src-tauri/target/release/bundle/appimage/`.

### Changing targets for your fork

You can remove formats you do not need (e.g. only `deb` on Linux) or add others Tauri supports; see [Tauri distribute](https://v2.tauri.app/distribute/) for Snap, Flatpak, AUR, etc.

## Checks before opening a PR

Pre-commit already runs **`npm run verify`** (see above). For the same steps by hand:

```bash
npm run verify
```

Equivalent piecemeal:

```bash
npm run lint && npm run test && npm run build
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
cargo test --manifest-path src-tauri/Cargo.toml
python3 -m pip install -r requirements-dev.txt
npm run py:lint && npm run py:test
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
