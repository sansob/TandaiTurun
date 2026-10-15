#!/usr/bin/env bash
# Makes the README screenshots: each scripted scene of the app, under Xvfb.
#   scripts/screenshots.sh            (writes docs/screenshots/*.png)
#   OUT=dir BIN=target/debug/tandaiturun scripts/screenshots.sh
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=${OUT:-docs/screenshots}
BIN=${BIN:-target/release/tandaiturun}
mkdir -p "$OUT"
export HOME=$(mktemp -d) LIBGL_ALWAYS_SOFTWARE=1
for scene in read edit thumbs dark search bahasa home; do
  file=examples/showcase.md
  [ "$scene" = home ] && file=
  TANDAITURUN_SCENE=$scene TANDAITURUN_SCREENSHOT="$OUT/$scene.png" \
    timeout 90 xvfb-run -a -s "-screen 0 1440x900x24" "$BIN" $file
done
ls -l "$OUT"
