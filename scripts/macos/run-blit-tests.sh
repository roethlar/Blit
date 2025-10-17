#!/usr/bin/env bash
# macOS-friendly test runner that mirrors the Windows PowerShell helper.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${1:-$(cd "${SCRIPT_DIR}/../.." && pwd)}"

cd "${REPO_ROOT}"

export RUST_BACKTRACE=1
export LANG=C.UTF-8
export LC_ALL=C.UTF-8

LOG_DIR="${REPO_ROOT}/logs"
mkdir -p "${LOG_DIR}"
TIMESTAMP="$(date -u +"%Y%m%d-%H%M%SZ")"

run_step() {
  local name="$1"
  shift
  local slug
  slug="$(echo "${name}" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9-' '-')"
  local log_file="${LOG_DIR}/${slug}-${TIMESTAMP}.log"

  echo "==> ${name}"
  if "$@" 2>&1 | tee "${log_file}"; then
    echo "--> Logs: ${log_file}"
  else
    echo "Step '${name}' failed. See ${log_file}" >&2
    exit 1
  fi
}

run_step "cargo fmt -- --check" cargo fmt -- --check
run_step "cargo check" cargo check
run_step "cargo test -p blit-core" cargo test -p blit-core
run_step "cargo test workspace" cargo test
