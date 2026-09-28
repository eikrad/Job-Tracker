# Releasing

Versions and the CHANGELOG are written by [release-please](https://github.com/googleapis/release-please)
from Conventional Commit messages. Nobody edits version numbers by hand.

## How a release happens

1. On every push to `main`, the **Release** workflow (`.github/workflows/release-please.yml`)
   opens or updates a pull request titled `chore(main): release X.Y.Z`. It bumps the
   version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, syncs
   `package-lock.json` and `src-tauri/Cargo.lock`, and adds the `CHANGELOG.md` entry.
2. When you want to release, merge that pull request. release-please tags `vX.Y.Z` and
   publishes a GitHub release with the same notes.

Until you merge the release PR, it keeps collecting whatever lands on `main`. Installers
are not built by CI; build them locally with `npm run tauri:build:release`.

## Which version comes next

Before 1.0 (`bump-minor-pre-major` and `bump-patch-for-minor-pre-major` in
`release-please-config.json`):

| Commit | Next version |
|---|---|
| `fix: …` | 0.4.5 → 0.4.6 |
| `feat: …` | 0.4.5 → 0.4.6 |
| `feat!: …` or a `BREAKING CHANGE:` footer | 0.4.5 → 0.5.0 |
| `docs`, `chore`, `build`, `ci`, `test`, `refactor`, `style` | no release on its own |

`feat`, `fix`, `perf`, `security` and `revert` appear in the CHANGELOG; the other types are
hidden. From 1.0 on, `feat` bumps the minor and a breaking change the major version.

The **Commit messages** check fails a pull request with a commit that lacks a prefix,
since release-please would silently leave that commit out.

## One-time setup: the release token

Pull requests opened with the workflow's own `GITHUB_TOKEN` do not trigger other
workflows, so CI would never run on the release PR. The workflow therefore uses a token
stored as the `RELEASE_PLEASE_TOKEN` secret:

1. GitHub → Settings → Developer settings → Personal access tokens → **Fine-grained tokens**
   → Generate new token.
2. Repository access: **Only select repositories** → this repository.
3. Permissions: **Contents: Read and write**, **Pull requests: Read and write**.
4. Choose an expiry and set a reminder; the release workflow fails once it has expired.
5. In this repository: Settings → Secrets and variables → Actions → New repository secret,
   name `RELEASE_PLEASE_TOKEN`, value the token.

## Changing the next version by hand

Set `"release-as": "1.0.0"` for the `"."` package in `release-please-config.json`, merge it
to `main`, and remove the line again after the release.
