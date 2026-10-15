#!/usr/bin/env bash
# Builds the .deb, .rpm and AppImage from target/release into <out-dir>.
#   packaging/linux/build-packages.sh <version> <x86_64|aarch64> <out-dir>
# Needs cargo-deb, cargo-generate-rpm and appimagetool
# (APPIMAGETOOL=/path/to/appimagetool).
set -euo pipefail
VERSION=$1 ARCH=$2 OUT=$3
cd "$(dirname "$0")/../.."
mkdir -p "$OUT"
desktop-file-validate packaging/linux/tandaiturun.desktop
cargo deb -p tandaiturun --no-build --no-strip -o "$OUT/"
# RPM versions cannot contain "-": 0.2.0-rc.1 -> 0.2.0~rc.1 (sorts before 0.2.0).
cargo generate-rpm -p crates/tandaiturun -o "$OUT/" -s "version = \"${VERSION//-/\~}\""
packaging/linux/make-appimage.sh "$VERSION" "$ARCH" "$OUT"
