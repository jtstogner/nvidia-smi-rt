#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "Usage: $0 <version> <dist_dir>" >&2
  exit 1
fi

VERSION="${1#v}"
DIST_DIR="$(cd "$2" && pwd)"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

sha_for() {
  local file="$1"
  if [ ! -f "$DIST_DIR/$file" ]; then
    echo "Missing artifact for manifest generation: $DIST_DIR/$file" >&2
    exit 1
  fi
  sha256sum "$DIST_DIR/$file" | awk '{print $1}'
}

SHA_LINUX_X64="$(sha_for "nvidia-smi-rt-v${VERSION}-linux-x86_64.tar.gz")"
SHA_LINUX_ARM64="$(sha_for "nvidia-smi-rt-v${VERSION}-linux-aarch64.tar.gz")"
SHA_MACOS_ARM64="$(sha_for "nvidia-smi-rt-v${VERSION}-macos-arm64.tar.gz")"
SHA_MACOS_X64="$(sha_for "nvidia-smi-rt-v${VERSION}-macos-x86_64.tar.gz")"
SHA_WIN_X64="$(sha_for "nvidia-smi-rt-v${VERSION}-windows-x86_64.zip")"

render_template() {
  local tmpl="$1"
  local out="$2"
  sed \
    -e "s/@VERSION@/${VERSION}/g" \
    -e "s/@SHA256_LINUX_X86_64@/${SHA_LINUX_X64}/g" \
    -e "s/@SHA256_LINUX_ARM64@/${SHA_LINUX_ARM64}/g" \
    -e "s/@SHA256_MACOS_ARM64@/${SHA_MACOS_ARM64}/g" \
    -e "s/@SHA256_MACOS_X86_64@/${SHA_MACOS_X64}/g" \
    -e "s/@SHA256_WINDOWS_X86_64@/${SHA_WIN_X64}/g" \
    "$tmpl" > "$out"
  echo "Generated $out"
}

render_template "$REPO_ROOT/packaging/homebrew/nvidia-smi-rt.rb.in" "$DIST_DIR/nvidia-smi-rt.rb"
render_template "$REPO_ROOT/packaging/arch/PKGBUILD.in" "$DIST_DIR/PKGBUILD"
render_template "$REPO_ROOT/packaging/scoop/nvidia-smi-rt.json.in" "$DIST_DIR/nvidia-smi-rt.json"

(
  cd "$DIST_DIR"
  rm -f SHA256SUMS.txt
  sha256sum -- * > SHA256SUMS.txt
)
echo "Generated $DIST_DIR/SHA256SUMS.txt"
