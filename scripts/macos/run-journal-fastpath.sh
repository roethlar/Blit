#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

WORKSPACE="$(mktemp -d "${TMPDIR:-/tmp}/blit_journal_fastpath.XXXXXX")"
SRC="${WORKSPACE}/src"
DST="${WORKSPACE}/dst"
CONFIG_DIR="${WORKSPACE}/config"

mkdir -p "${SRC}" "${DST}" "${CONFIG_DIR}"
export BLIT_CONFIG_DIR="${CONFIG_DIR}"

echo "Workspace     : ${WORKSPACE}"
echo "Source dir    : ${SRC}"
echo "Destination   : ${DST}"
echo

echo "Generating 5000 files..."
for i in $(seq 1 5000); do
  printf 'payload %05d' "${i}" >"${SRC}/file_$(printf '%05d' "${i}").txt"
done

candidate_bins=(
  "${REPO_ROOT}/target/release/blit-cli"
  "${REPO_ROOT}/target/x86_64-apple-darwin/release/blit-cli"
  "${REPO_ROOT}/target/aarch64-apple-darwin/release/blit-cli"
)

BLIT_BIN=""
for candidate in "${candidate_bins[@]}"; do
  if [[ -x "${candidate}" ]]; then
    BLIT_BIN="${candidate}"
    break
  fi
done

if [[ -z "${BLIT_BIN}" ]]; then
  echo "blit-cli binary not found. Build it first: cargo build --release -p blit-cli --bin blit-cli" >&2
  exit 1
fi

echo
echo "Using blit-cli : ${BLIT_BIN}"
echo

run_mirror() {
  local label="$1"
  echo "== ${label} =="
  if ! "${BLIT_BIN}" mirror "${SRC}" "${DST}" --verbose; then
    echo "blit mirror failed during '${label}' run." >&2
    exit 1
  fi
  echo
}

run_mirror "Initial sync"
run_mirror "Zero-change sync"

echo "Done. Results remain under ${WORKSPACE}"
