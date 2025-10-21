#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$REPO_ROOT/target/macos"}

exec "$SCRIPT_DIR/bench_local_mirror.sh" "$@"
