#!/usr/bin/env bash
#
# Release build: freeze the mail-scan sidecar, then bundle it with its hash pinned.
#
# `externalBin` lives in tauri.release.conf.json rather than tauri.conf.json on
# purpose. Tauri requires an externalBin to exist at build time, so putting it in the
# base config would break `cargo build`, `npm run verify`, and CI — none of which need
# a frozen sidecar, because dev runs go through `uv run`.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

./scripts/build-sidecar.sh

triple="${SIDECAR_TARGET_TRIPLE:-$(rustc -vV | awk '/^host:/ {print $2}')}"
binary="src-tauri/binaries/jobtracker-mail-scan-$triple"
sha="$(sha256sum "$binary" | awk '{print $1}')"

echo "==> Bundling with sidecar pin $sha"
JOBTRACKER_SIDECAR_SHA256="$sha" \
  npx tauri build --config src-tauri/tauri.release.conf.json "$@"
