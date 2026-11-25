#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 4 ]; then
  echo "Usage: $0 <version> <arch: arm64|x86_64> <bin_dir> <out_dir>" >&2
  exit 1
fi

VERSION="${1#v}"
ARCH="$2"
BIN_DIR="$(cd "$3" && pwd)"
OUT_DIR="$(mkdir -p "$4" && cd "$4" && pwd)"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

for bin in nvidia-smi-rt nvidia-smi-rt-gui; do
  if [ ! -f "$BIN_DIR/$bin" ]; then
    echo "Missing binary: $BIN_DIR/$bin" >&2
    exit 1
  fi
done

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

# 1. Tarball (.tar.gz)
TAR_STAGE="$WORK_DIR/nvidia-smi-rt-v${VERSION}-macos-${ARCH}"
mkdir -p "$TAR_STAGE"
cp "$BIN_DIR/nvidia-smi-rt" "$BIN_DIR/nvidia-smi-rt-gui" "$TAR_STAGE/"
chmod 755 "$TAR_STAGE/nvidia-smi-rt" "$TAR_STAGE/nvidia-smi-rt-gui"
cp "$REPO_ROOT/README.md" "$REPO_ROOT/LICENSE" "$TAR_STAGE/"

tar -C "$WORK_DIR" -czf "$OUT_DIR/nvidia-smi-rt-v${VERSION}-macos-${ARCH}.tar.gz" "nvidia-smi-rt-v${VERSION}-macos-${ARCH}"
echo "Created $OUT_DIR/nvidia-smi-rt-v${VERSION}-macos-${ARCH}.tar.gz"

# 2. macOS .app bundle + .dmg
DMG_STAGE="$WORK_DIR/dmg"
APP_BUNDLE="$DMG_STAGE/NVIDIA-SMI-RT.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources"

cp "$BIN_DIR/nvidia-smi-rt-gui" "$APP_BUNDLE/Contents/MacOS/nvidia-smi-rt-gui"
cp "$BIN_DIR/nvidia-smi-rt" "$APP_BUNDLE/Contents/MacOS/nvidia-smi-rt"
chmod 755 "$APP_BUNDLE/Contents/MacOS/nvidia-smi-rt-gui" "$APP_BUNDLE/Contents/MacOS/nvidia-smi-rt"

sed "s/@VERSION@/${VERSION}/g" "$REPO_ROOT/packaging/macos/Info.plist.in" > "$APP_BUNDLE/Contents/Info.plist"

cp "$BIN_DIR/nvidia-smi-rt" "$DMG_STAGE/nvidia-smi-rt"
cp "$REPO_ROOT/README.md" "$REPO_ROOT/LICENSE" "$DMG_STAGE/"
ln -s /Applications "$DMG_STAGE/Applications"

DMG_PATH="$OUT_DIR/nvidia-smi-rt-v${VERSION}-macos-${ARCH}.dmg"
hdiutil create \
  -volname "NVIDIA-SMI-RT" \
  -srcfolder "$DMG_STAGE" \
  -ov \
  -format UDZO \
  "$DMG_PATH"
echo "Created $DMG_PATH"
