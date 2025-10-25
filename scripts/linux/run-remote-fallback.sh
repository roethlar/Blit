#!/usr/bin/env bash
set -euo pipefail

REMOTE=$1
MODULE=$2
TMPDIR=${TMPDIR:-/tmp}
WORKDIR=$(mktemp -d "$TMPDIR/blit_remote_fallback.XXXXXX")
SRC="$WORKDIR/src"
CONFIG="$WORKDIR/config"
mkdir -p "$SRC" "$CONFIG"

printf 'payload' > "$SRC/file.txt"

BLIT_BIN="$(dirname "$0")/../../target/release/blit-cli"

mirror() {
  "$BLIT_BIN" --config-dir "$CONFIG" mirror "$SRC" "$1" --verbose
}

mirror "$MODULE"
mirror "$MODULE"

rm -rf "$WORKDIR"
