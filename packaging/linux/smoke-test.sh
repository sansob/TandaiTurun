#!/usr/bin/env bash
# Runs inside a clean distribution container after installing the TandaiTurun
# package (or with SMOKE_APPIMAGE=<file>): the command line must work, a PDF
# must be exported, and the app must start and draw a window.
set -euo pipefail
if [ -n "${SMOKE_APPIMAGE:-}" ]; then
  APP=("$SMOKE_APPIMAGE")
else
  APP=(tandaiturun)
  test -f /usr/share/applications/tandaiturun.desktop
  test -f /usr/share/icons/hicolor/256x256/apps/tandaiturun.png
fi
"${APP[@]}" --version
export HOME=$(mktemp -d)
printf '# Smoke test\n\nHello **TandaiTurun**.\n\n```mermaid\nflowchart LR\n  A --> B\n```\n' > "$HOME/smoke.md"
"${APP[@]}" --export-pdf "$HOME/smoke.md" "$HOME/smoke.pdf"
head -c 5 "$HOME/smoke.pdf" | grep -q '%PDF-'
export TANDAITURUN_SCENE=read TANDAITURUN_SCREENSHOT=${SMOKE_SCREENSHOT:-/tmp/tandaiturun.png} LIBGL_ALWAYS_SOFTWARE=1
timeout 90 xvfb-run -a -s "-screen 0 1440x900x24" "${APP[@]}" "$HOME/smoke.md"
test -s "$TANDAITURUN_SCREENSHOT"
echo "app started and drew a window: $TANDAITURUN_SCREENSHOT"
