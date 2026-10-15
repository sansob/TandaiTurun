#!/usr/bin/env bash
# Builds TandaiTurun.app (universal binary) and a .dmg.
# Usage: packaging/macos/make-dmg.sh <version> <out-dir>
# Expects target/{aarch64,x86_64}-apple-darwin/release/tandaiturun.
set -euo pipefail
VERSION="$1"
OUT="$2"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
STAGE="$(mktemp -d)"
APP="$STAGE/TandaiTurun.app"

mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$OUT"
lipo -create \
  "$ROOT/target/aarch64-apple-darwin/release/tandaiturun" \
  "$ROOT/target/x86_64-apple-darwin/release/tandaiturun" \
  -output "$APP/Contents/MacOS/tandaiturun"
# Apple requires numeric versions (x.y.z) in Info.plist: drop "-rc.1" etc.
sed "s/__VERSION__/${VERSION%%-*}/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"
cp "$ROOT/assets/icon.icns" "$APP/Contents/Resources/icon.icns"
mkdir -p "$APP/Contents/Resources/licenses" && cp "$ROOT"/assets/fonts/OFL-*.txt "$APP/Contents/Resources/licenses/"

# Ad-hoc signature so Apple Silicon runs it; users still see the
# "unidentified developer" prompt until the project has a Developer ID.
codesign --force --deep --sign - "$APP" || true

ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "TandaiTurun $VERSION" -srcfolder "$STAGE" -ov -format UDZO \
  "$OUT/TandaiTurun-$VERSION-macos-universal.dmg"
