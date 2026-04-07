#!/usr/bin/env bash
# Build optimized release binaries for all workspace crates.
# Usage: ./scripts/build-release.sh [--target <TRIPLE>]
#
# Produces stripped binaries in target/release/ (or target/<TRIPLE>/release/).
# Requires: cargo, strip (Unix) or llvm-strip.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

TARGET=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            TARGET="$2"
            shift 2
            ;;
        *)
            echo "Usage: $0 [--target <TRIPLE>]" >&2
            exit 1
            ;;
    esac
done

cd "$ROOT_DIR"

CARGO_ARGS=(build --release)
if [[ -n "$TARGET" ]]; then
    CARGO_ARGS+=(--target "$TARGET")
    OUT_DIR="target/${TARGET}/release"
else
    OUT_DIR="target/release"
fi

echo "==> Building release binaries..."
cargo "${CARGO_ARGS[@]}"

BINARIES=(blit-cli blit-daemon blit-utils)

echo "==> Stripping binaries..."
for bin in "${BINARIES[@]}"; do
    BIN_PATH="${OUT_DIR}/${bin}"
    if [[ -f "$BIN_PATH" ]]; then
        strip "$BIN_PATH" 2>/dev/null || true
        SIZE=$(du -h "$BIN_PATH" | cut -f1)
        echo "    ${bin}: ${SIZE}"
    fi
done

echo "==> Release binaries in ${OUT_DIR}/"

# Create a tarball if requested via BUILD_TARBALL=1
if [[ "${BUILD_TARBALL:-0}" == "1" ]]; then
    VERSION=$(grep '^version' crates/blit-cli/Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
    ARCH=$(uname -m)
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    if [[ -n "$TARGET" ]]; then
        TARBALL_NAME="blit-${VERSION}-${TARGET}.tar.gz"
    else
        TARBALL_NAME="blit-${VERSION}-${OS}-${ARCH}.tar.gz"
    fi

    echo "==> Creating ${TARBALL_NAME}..."
    STAGING=$(mktemp -d)
    for bin in "${BINARIES[@]}"; do
        BIN_PATH="${OUT_DIR}/${bin}"
        if [[ -f "$BIN_PATH" ]]; then
            cp "$BIN_PATH" "$STAGING/"
        fi
    done
    cp COPYING.md "$STAGING/" 2>/dev/null || true
    cp README.md "$STAGING/" 2>/dev/null || true

    tar -czf "${OUT_DIR}/${TARBALL_NAME}" -C "$STAGING" .
    rm -rf "$STAGING"
    echo "    ${OUT_DIR}/${TARBALL_NAME}"
fi

echo "==> Done."
