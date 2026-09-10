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

# One-dir rather than one-file: one-file unpacks to a temp directory on every launch,
# which is slower and trips some endpoint-protection tools. One-dir is also easier to
# hash meaningfully, because the launcher binary is stable.
uv run --with pyinstaller pyinstaller \
  --noconfirm \
  --clean \
  --onedir \
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

built="$work_dir/dist/$name/$name"
if [[ ! -f "$built" ]]; then
  echo "error: PyInstaller did not produce $built" >&2
  exit 1
fi

# Sanity-check the frozen binary before shipping it: a sidecar that cannot answer
# `probe` is one the app will reject at runtime anyway, and better to find out here.
echo "==> Probing the frozen sidecar"
"$built" probe --protocol 1 >/dev/null

target="$out_dir/$name-$triple"
cp "$built" "$target"
chmod +x "$target"

# Copy the one-dir support files next to the launcher.
rm -rf "$out_dir/$name-support"
cp -R "$work_dir/dist/$name" "$out_dir/$name-support"
rm -f "$out_dir/$name-support/$name"

sha="$(sha256sum "$target" | awk '{print $1}')"

echo
echo "==> Sidecar built: $target"
echo "==> SHA-256: $sha"
echo
echo "Bake the pin into the release build:"
echo "  JOBTRACKER_SIDECAR_SHA256=$sha npm run tauri build"
