#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
DESKTOP_FILE="${DESKTOP_DIR}/JobTracker.desktop"
TAURI_DESKTOP_TEMPLATE="${ROOT_DIR}/src-tauri/jobtracker.desktop"
RELEASE_BINARY="${ROOT_DIR}/src-tauri/target/release/app"
ICON_SRC="${ROOT_DIR}/src-tauri/icons/128x128.png"
ICON_THEME_BASE="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"

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

install_icons() {
  if [[ ! -f "${ICON_SRC}" ]]; then
    echo "Warning: icon not found at ${ICON_SRC}; launcher may show a generic icon." >&2
    return 0
  fi

  for size_dir in 32x32 128x128 256x256; do
    case "${size_dir}" in
      32x32) size_px=32 ;;
      128x128) size_px=128 ;;
      256x256) size_px=256 ;;
    esac
    dest_dir="${ICON_THEME_BASE}/${size_dir}/apps"
    mkdir -p "${dest_dir}"
    magick "${ICON_SRC}" -resize "${size_px}x${size_px}" "${dest_dir}/app.png"
  done

  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${ICON_THEME_BASE}" >/dev/null 2>&1 || true
  fi
}

mkdir -p "${DESKTOP_DIR}"
install_icons

awk -v exec_cmd="${EXEC_CMD}" '
  BEGIN { replaced_exec = 0 }
  /^Exec=/ {
    print "Exec=" exec_cmd
    replaced_exec = 1
    next
  }
  { print }
  END {
    if (!replaced_exec) {
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
echo "If it does not appear immediately, log out and back in or restart your app menu."
