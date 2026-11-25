#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 4 ]; then
  echo "Usage: $0 <version> <arch: x86_64|aarch64> <bin_dir> <out_dir>" >&2
  exit 1
fi

VERSION="${1#v}"
ARCH="$2"
BIN_DIR="$(cd "$3" && pwd)"
OUT_DIR="$(mkdir -p "$4" && cd "$4" && pwd)"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "$ARCH" in
  x86_64)
    DEB_ARCH="amd64"
    RPM_ARCH="x86_64"
    ;;
  aarch64)
    DEB_ARCH="arm64"
    RPM_ARCH="aarch64"
    ;;
  *)
    echo "Unsupported Linux arch: $ARCH (expected x86_64 or aarch64)" >&2
    exit 1
    ;;
esac

for bin in nvidia-smi-rt nvidia-smi-rt-gui; do
  if [ ! -f "$BIN_DIR/$bin" ]; then
    echo "Missing binary: $BIN_DIR/$bin" >&2
    exit 1
  fi
done

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

# 1. Tarball (.tar.gz)
TAR_STAGE="$WORK_DIR/nvidia-smi-rt-v${VERSION}-linux-${ARCH}"
mkdir -p "$TAR_STAGE"
cp "$BIN_DIR/nvidia-smi-rt" "$BIN_DIR/nvidia-smi-rt-gui" "$TAR_STAGE/"
chmod 755 "$TAR_STAGE/nvidia-smi-rt" "$TAR_STAGE/nvidia-smi-rt-gui"
cp "$REPO_ROOT/README.md" "$REPO_ROOT/LICENSE" "$TAR_STAGE/"
cp "$REPO_ROOT/packaging/linux/nvidia-smi-rt-gui.desktop" "$TAR_STAGE/"
cp "$REPO_ROOT/packaging/linux/nvidia-smi-rt.svg" "$TAR_STAGE/"

tar -C "$WORK_DIR" -czf "$OUT_DIR/nvidia-smi-rt-v${VERSION}-linux-${ARCH}.tar.gz" "nvidia-smi-rt-v${VERSION}-linux-${ARCH}"
echo "Created $OUT_DIR/nvidia-smi-rt-v${VERSION}-linux-${ARCH}.tar.gz"

# 2. Debian Package (.deb)
if command -v dpkg-deb >/dev/null 2>&1; then
  DEB_ROOT="$WORK_DIR/deb"
  mkdir -p "$DEB_ROOT/DEBIAN" \
           "$DEB_ROOT/usr/bin" \
           "$DEB_ROOT/usr/share/applications" \
           "$DEB_ROOT/usr/share/icons/hicolor/scalable/apps" \
           "$DEB_ROOT/usr/share/doc/nvidia-smi-rt"

  install -m 755 "$BIN_DIR/nvidia-smi-rt" "$DEB_ROOT/usr/bin/nvidia-smi-rt"
  install -m 755 "$BIN_DIR/nvidia-smi-rt-gui" "$DEB_ROOT/usr/bin/nvidia-smi-rt-gui"
  install -m 644 "$REPO_ROOT/packaging/linux/nvidia-smi-rt-gui.desktop" "$DEB_ROOT/usr/share/applications/nvidia-smi-rt-gui.desktop"
  install -m 644 "$REPO_ROOT/packaging/linux/nvidia-smi-rt.svg" "$DEB_ROOT/usr/share/icons/hicolor/scalable/apps/nvidia-smi-rt.svg"
  install -m 644 "$REPO_ROOT/LICENSE" "$DEB_ROOT/usr/share/doc/nvidia-smi-rt/copyright"
  install -m 644 "$REPO_ROOT/README.md" "$DEB_ROOT/usr/share/doc/nvidia-smi-rt/README.md"

  cat > "$DEB_ROOT/DEBIAN/control" <<EOF
Package: nvidia-smi-rt
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${DEB_ARCH}
Maintainer: jtstogner <https://github.com/jtstogner/nvidia-smi-rt>
Homepage: https://github.com/jtstogner/nvidia-smi-rt
Description: Real-time NVIDIA GPU telemetry dashboard (TUI & GUI) and process manager
 Interfaces directly with NVIDIA drivers via NVML (with automatic nvidia-smi
 CLI fallback) to deliver low-overhead GPU telemetry charts, gauges, multi-GPU
 fleet views, and an interactive process manager.
EOF

  dpkg-deb --root-owner-group --build "$DEB_ROOT" "$OUT_DIR/nvidia-smi-rt_${VERSION}_${DEB_ARCH}.deb"
  echo "Created $OUT_DIR/nvidia-smi-rt_${VERSION}_${DEB_ARCH}.deb"
else
  echo "Skipping .deb build: dpkg-deb not found"
fi

# 3. RPM Package (.rpm)
if command -v rpmbuild >/dev/null 2>&1; then
  RPM_TOP="$WORK_DIR/rpmbuild"
  mkdir -p "$RPM_TOP"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
  cp "$REPO_ROOT/packaging/linux/nvidia-smi-rt-gui.desktop" "$RPM_TOP/SOURCES/"
  cp "$REPO_ROOT/packaging/linux/nvidia-smi-rt.svg" "$RPM_TOP/SOURCES/"
  cp "$REPO_ROOT/LICENSE" "$RPM_TOP/SOURCES/"
  cp "$REPO_ROOT/README.md" "$RPM_TOP/SOURCES/"

  rpmbuild \
    --define "_topdir $RPM_TOP" \
    --define "_sourcedir $RPM_TOP/SOURCES" \
    --define "_version $VERSION" \
    --define "_srcbindir $BIN_DIR" \
    --target "$RPM_ARCH" \
    -bb "$REPO_ROOT/packaging/linux/nvidia-smi-rt.spec"

  find "$RPM_TOP/RPMS" -name '*.rpm' -exec cp {} "$OUT_DIR/" \;
  echo "Created RPM package(s) in $OUT_DIR"
else
  echo "Skipping .rpm build: rpmbuild not found"
fi

# 4. AppImage (.AppImage)
APPIMAGETOOL_BIN="${APPIMAGETOOL:-}"
if [ -z "$APPIMAGETOOL_BIN" ] && command -v appimagetool >/dev/null 2>&1; then
  APPIMAGETOOL_BIN="$(command -v appimagetool)"
fi

if [ -n "$APPIMAGETOOL_BIN" ]; then
  APPDIR="$WORK_DIR/AppDir"
  mkdir -p "$APPDIR/usr/bin" \
           "$APPDIR/usr/share/applications" \
           "$APPDIR/usr/share/icons/hicolor/scalable/apps"

  install -m 755 "$BIN_DIR/nvidia-smi-rt" "$APPDIR/usr/bin/nvidia-smi-rt"
  install -m 755 "$BIN_DIR/nvidia-smi-rt-gui" "$APPDIR/usr/bin/nvidia-smi-rt-gui"
  install -m 644 "$REPO_ROOT/packaging/linux/nvidia-smi-rt-gui.desktop" "$APPDIR/usr/share/applications/nvidia-smi-rt-gui.desktop"
  install -m 644 "$REPO_ROOT/packaging/linux/nvidia-smi-rt.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/nvidia-smi-rt.svg"
  install -m 644 "$REPO_ROOT/packaging/linux/nvidia-smi-rt-gui.desktop" "$APPDIR/nvidia-smi-rt-gui.desktop"
  install -m 644 "$REPO_ROOT/packaging/linux/nvidia-smi-rt.svg" "$APPDIR/nvidia-smi-rt.svg"
  ln -sf nvidia-smi-rt.svg "$APPDIR/.DirIcon"

  cat > "$APPDIR/AppRun" <<'EOF'
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="$HERE/usr/bin:$PATH"
if [ "${1:-}" = "--tui" ]; then
  shift
  exec "$HERE/usr/bin/nvidia-smi-rt" "$@"
fi
exec "$HERE/usr/bin/nvidia-smi-rt-gui" "$@"
EOF
  chmod 755 "$APPDIR/AppRun"

  APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$ARCH" "$APPIMAGETOOL_BIN" \
    "$APPDIR" \
    "$OUT_DIR/nvidia-smi-rt-v${VERSION}-linux-${ARCH}.AppImage"
  echo "Created $OUT_DIR/nvidia-smi-rt-v${VERSION}-linux-${ARCH}.AppImage"
else
  echo "Skipping .AppImage build: appimagetool not found"
fi
