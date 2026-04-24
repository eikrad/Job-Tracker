#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
DESKTOP_FILE="${DESKTOP_DIR}/job-tracker.desktop"
TAURI_DESKTOP_TEMPLATE="${ROOT_DIR}/src-tauri/jobtracker.desktop"
RELEASE_BINARY="${ROOT_DIR}/src-tauri/target/release/job-tracker"

if [[ ! -f "${TAURI_DESKTOP_TEMPLATE}" ]]; then
  echo "Missing template: ${TAURI_DESKTOP_TEMPLATE}" >&2
  exit 1
fi

if [[ -x "${RELEASE_BINARY}" ]]; then
  EXEC_CMD="${RELEASE_BINARY}"
elif command -v npm >/dev/null 2>&1; then
  EXEC_CMD="sh -lc 'cd \"${ROOT_DIR}\" && npm run tauri:dev'"
else
  echo "No release binary found and npm is unavailable." >&2
  echo "Build once with: npm run tauri:build" >&2
  exit 1
fi

mkdir -p "${DESKTOP_DIR}"

awk -v exec_cmd="${EXEC_CMD}" '
  BEGIN { replaced = 0 }
  /^Exec=/ {
    print "Exec=" exec_cmd
    replaced = 1
    next
  }
  { print }
  END {
    if (!replaced) {
      print "Exec=" exec_cmd
    }
  }
' "${TAURI_DESKTOP_TEMPLATE}" > "${DESKTOP_FILE}"

chmod 755 "${DESKTOP_FILE}"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${DESKTOP_DIR}" >/dev/null 2>&1 || true
fi

echo "Desktop shortcut installed:"
echo "  ${DESKTOP_FILE}"
echo
echo "If it does not appear immediately, log out and back in or run:"
echo "  gtk-update-icon-cache -f -t ${XDG_DATA_HOME:-$HOME/.local/share}/icons || true"
