#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

CARGO_VERSION="$(awk -F'"' '/^version *= *"/ {print $2; exit}' Cargo.toml)"
VERSION=""
AUTO_YES=false
DRY_RUN=false
SKIP_TESTS=false

for arg in "$@"; do
  case "$arg" in
    --yes|-y)
      AUTO_YES=true
      ;;
    --dry-run)
      DRY_RUN=true
      ;;
    --skip-tests)
      SKIP_TESTS=true
      ;;
    --help|-h)
      cat <<EOF
Usage: $0 [version] [--yes] [--dry-run] [--skip-tests]

Tags and pushes a release to trigger the multi-OS GitHub Actions release workflow
(.github/workflows/release.yml) for Linux, Windows, and macOS packages.

Arguments:
  version       Semver version (e.g. 0.1.1 or v0.1.1). Defaults to Cargo.toml ($CARGO_VERSION).
  --yes, -y     Skip interactive confirmation prompt.
  --dry-run     Validate and run checks without creating or pushing git tags.
  --skip-tests  Skip local cargo test/build checks before tagging.
EOF
      exit 0
      ;;
    -*)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
    *)
      if [ -z "$VERSION" ]; then
        VERSION="${arg#v}"
      else
        echo "Unexpected extra argument: $arg" >&2
        exit 1
      fi
      ;;
  esac
done

if [ -z "$VERSION" ]; then
  VERSION="$CARGO_VERSION"
fi

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$ ]]; then
  echo "Invalid semver version: '$VERSION'" >&2
  exit 1
fi

if [ "$VERSION" != "$CARGO_VERSION" ]; then
  echo "Version mismatch: requested '$VERSION' but Cargo.toml has '$CARGO_VERSION'." >&2
  echo "Update Cargo.toml version first or pass '$CARGO_VERSION'." >&2
  exit 1
fi

TAG="v${VERSION}"

if [ -n "$(git status --porcelain)" ]; then
  echo "Working tree has uncommitted changes. Commit or stash them before releasing." >&2
  git status -s >&2
  exit 1
fi

CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$CURRENT_BRANCH" != "main" ] && [ "$AUTO_YES" = false ]; then
  echo "Warning: current branch is '$CURRENT_BRANCH' (expected 'main')."
fi

if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "Tag '$TAG' already exists locally." >&2
  exit 1
fi

if git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null 2>&1; then
  echo "Tag '$TAG' already exists on remote 'origin'." >&2
  exit 1
fi

if [ "$SKIP_TESTS" = false ]; then
  echo "Running local verification checks..."
  cargo test
  cargo test --features gui
  cargo build --features gui --bins
fi

echo ""
echo "=== Release Summary ==="
echo "  Branch:  $CURRENT_BRANCH"
echo "  Commit:  $(git rev-parse --short HEAD) ($(git log -1 --pretty=%s))"
echo "  Version: $VERSION"
echo "  Tag:     $TAG"
echo "  Remote:  $(git remote get-url origin)"
echo ""

if [ "$DRY_RUN" = true ]; then
  echo "[dry-run] Would run:"
  echo "  git tag -a \"$TAG\" -m \"Release $TAG\""
  echo "  git push origin \"$CURRENT_BRANCH\""
  echo "  git push origin \"$TAG\""
  exit 0
fi

if [ "$AUTO_YES" = false ]; then
  read -r -p "Create tag '$TAG' and push to origin to publish GitHub Release? [y/N] " reply
  if [[ ! "$reply" =~ ^[Yy]$ ]]; then
    echo "Release cancelled."
    exit 1
  fi
fi

git tag -a "$TAG" -m "Release $TAG"
git push origin "$CURRENT_BRANCH"
git push origin "$TAG"

echo ""
echo "Release tag '$TAG' pushed!"
echo "GitHub Actions workflow: https://github.com/jtstogner/nvidia-smi-rt/actions/workflows/release.yml"
echo "GitHub Release page:     https://github.com/jtstogner/nvidia-smi-rt/releases/tag/${TAG}"
