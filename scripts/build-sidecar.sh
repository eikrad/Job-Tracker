#!/usr/bin/env bash
#
# Freeze the mail-scan sidecar into a self-contained binary (spec §6.4, ADR 0002).
#
# The whole point of this step is that a release install must scan on a machine with
# **no Python at all**. Dev runs use `uv run` against the sources and never come near
# this script; only `npm run tauri build` needs it.
#
# Output:
#   src-tauri/binaries/jobtracker-mail-scan-<target-triple>   (Tauri externalBin naming)
#
# One-file, not one-dir. A one-dir bundle resolves its `_internal/` directory relative
# to the launcher, but Tauri's `externalBin` copies a *single file* next to the app
# binary — so a one-dir launcher arrives without its runtime and dies with
# "Failed to load Python shared library". One-file trades a little startup time for
# being the shape the packaging mechanism actually accepts.
#
# It also prints the SHA-256, which the build must bake in as
# JOBTRACKER_SIDECAR_SHA256 so a tampered or half-updated install refuses to run.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="src-tauri/binaries"
work_dir="target/sidecar-build"
name="jobtracker-mail-scan"

# Tauri appends the Rust target triple to an externalBin path; PyInstaller does not
# know about that, so the triple is resolved here and used for the final filename.
triple="${SIDECAR_TARGET_TRIPLE:-$(rustc -vV | awk '/^host:/ {print $2}')}"
if [[ -z "$triple" ]]; then
  echo "error: could not determine the Rust host triple (is rustc on PATH?)" >&2
  exit 1
fi

echo "==> Building mail-scan sidecar for $triple"

if ! command -v uv >/dev/null 2>&1; then
  echo "error: uv is required to build the sidecar (https://docs.astral.sh/uv/)" >&2
  exit 1
fi

mkdir -p "$out_dir" "$work_dir"

uv run --with pyinstaller pyinstaller \
  --noconfirm \
  --clean \
  --onefile \
  --name "$name" \
  --distpath "$work_dir/dist" \
  --workpath "$work_dir/build" \
  --specpath "$work_dir" \
  --paths python \
  --hidden-import mail_scan \
  --hidden-import mail_scan.cli \
  --hidden-import mail_scan.extractors.base \
  --hidden-import mail_scan.extractors.generic \
  --hidden-import mail_scan.extractors.indeed \
  --hidden-import mail_scan.html_text \
  --hidden-import mail_scan.urls \
  python/mail_scan/__main__.py

built="$work_dir/dist/$name"
if [[ ! -f "$built" ]]; then
  echo "error: PyInstaller did not produce $built" >&2
  exit 1
fi

target="$out_dir/$name-$triple"
cp "$built" "$target"
chmod +x "$target"

# Probe the artifact we actually ship, from a scrubbed environment, in a directory
# unrelated to the build tree. Probing the build-tree copy instead is how a broken
# bundle ships green: it passes there because its runtime happens to sit next to it.
echo "==> Probing the shipped sidecar with no Python on PATH"
(
  cd /
  env -i PATH=/usr/bin:/bin HOME=/nonexistent \
    "$repo_root/$target" probe --protocol 1 >/dev/null
) || {
  echo "error: the shipped sidecar cannot run standalone" >&2
  exit 1
}

sha="$(sha256sum "$target" | awk '{print $1}')"

echo
echo "==> Sidecar built: $target"
echo "==> SHA-256: $sha"
echo
echo "Bake the pin into the release build:"
echo "  JOBTRACKER_SIDECAR_SHA256=$sha npm run tauri build"
