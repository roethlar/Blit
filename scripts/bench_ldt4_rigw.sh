#!/usr/bin/env bash
# bench_ldt4_rigw.sh -- registered ldt-4 adaptive rig-W evidence harness.
#
# This runner is intentionally additive.  Endpoint session roots, active
# destination containers, retained payloads, traces, and manifests are never
# removed or overwritten.  A failed arm leaves all evidence in place and
# voids the session.

set -euo pipefail
set -o noclobber
IFS=$'\n\t'
umask 077

readonly ARTIFACT_SHA='406a7e5854593b7a7a151f9b6d9cdf1be8a9cd77'
readonly BUILD_ID='406a7e585459'
readonly CARGO_LOCK_SHA='ec1ce3fbe4208c7f7993e27ed997555b60bfef46c4bcec323b90bf9e6b4daa52'
readonly ARM_COUNT=96
readonly Q_IP='10.1.10.54'
readonly Q_NIC='en8'
readonly WIN_SSH='michael@10.1.10.177'
readonly WIN_IP='10.1.10.177'
readonly WIN_NIC='Ethernet'
readonly Q_ARTIFACT_REPO='/Users/michael/Dev/blit_v2_artifact_406a7e5'
readonly Q_BLIT="$Q_ARTIFACT_REPO/target/release/blit"
readonly Q_DAEMON="$Q_ARTIFACT_REPO/target/release/blit-daemon"
readonly WIN_STAGE_ROOT='D:/blit-test/bins/406a7e5'
readonly WIN_BLIT="$WIN_STAGE_ROOT/blit.exe"
readonly WIN_STAGE_DAEMON="$WIN_STAGE_ROOT/blit-daemon.exe"
readonly WIN_ACTIVE_DAEMON='D:/blit-test/bins/active/blit-daemon.exe'
readonly Q_STAGE_ROOT='/Users/michael/blit-ldt4-staging'
readonly WIN_FIXTURE_STAGE='D:/blit-test/ldt4-staging'
readonly Q_SESSION_ROOT='/Users/michael/blit-ldt4-sessions'
readonly WIN_SESSION_ROOT='D:/blit-test/ldt4-sessions'
readonly DAEMON_PORT=9031
readonly MIN_FREE_BYTES=33000000000
readonly EVIDENCE_ROOT='/Users/michael/blit-ldt4-evidence'

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd -P)
SCRIPT_PATH=$SCRIPT_DIR/bench_ldt4_rigw.sh
ANALYZER=$SCRIPT_DIR/ldt4_rigw_analyze.py
EXPECTED_HARNESS_SHA=${EXPECTED_HARNESS_SHA:-}
SESSION_TAG=''
OUT_DIR=''
SELFTEST=${SELFTEST:-0}

SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=8 -o ServerAliveInterval=10 \
    -o ServerAliveCountMax=3 -o ControlMaster=auto \
    -o "ControlPath=$HOME/.ssh/cm-ldt4-%C" -o ControlPersist=300)

SESSION_STARTED=0
SESSION_COMPLETE=0
CURRENT_Q_DAEMON_PID=''
CURRENT_WIN_DAEMON_PID=''
CURRENT_WIN_LAUNCHER_PID=''
CURRENT_DAEMON_ENDPOINT=''
CURRENT_Q_CLIENT_PID=''
CURRENT_Q_CLIENT_RUN_ID=''
CURRENT_Q_CLIENT_COMMAND=''
CURRENT_WIN_CLIENT_PID=''
CURRENT_WIN_CLIENT_CONTROLLER_PID=''
CURRENT_WIN_CLIENT_RUN_ID=''
CURRENT_WIN_CLIENT_COMMAND=''
CURRENT_RUN_ID=''
WIN_SWAP_ATTEMPTED=0
WIN_HAD_PRIOR='unknown'
WIN_PRIOR_DAEMON_SHA='unknown'
WIN_STAGED_DAEMON_SHA='unknown'
WIN_PREP_COMPLETE=0
WIN_RESTORE_RECORD=''
WIN_PRIOR_DAEMON=''
WIN_TESTED_DAEMON=''
RUNS_CSV=''
CLIENT_RESULT=''

die() {
    printf 'ldt-4 harness: %s\n' "$*" >&2
    exit 1
}

note() {
    printf '[ldt-4] %s\n' "$*" >&2
}

wssh() {
    ssh -T -n "${SSH_MUX[@]}" "$WIN_SSH" "$@"
}

local_sha256() {
    shasum -a 256 -- "$1" | awk '{print $1}'
}

windows_sha256() {
    local guard
    guard=$(windows_path_guard_script)
    wssh "$guard
Assert-Ldt4PlainPath '$1' File | Out-Null
(Get-FileHash -Algorithm SHA256 -LiteralPath '$1' -ErrorAction Stop).Hash.ToLower()
" \
        | tr -d '\r' | tail -1
}

exclusive_line() {
    local path=$1
    shift
    [[ ! -e "$path" && ! -L "$path" ]] || die "refusing to overwrite $path"
    (set -o noclobber; printf '%s\n' "$*" > "$path")
}

append_line() {
    local path=$1
    shift
    [[ -f "$path" && ! -L "$path" ]] || die "append target is not a plain file: $path"
    printf '%s\n' "$*" >> "$path"
}

finite_nonnegative_number() {
    python3 - "$1" <<'PY'
import math
import sys

try:
    value = float(sys.argv[1])
except (TypeError, ValueError, OverflowError):
    raise SystemExit(1)
raise SystemExit(0 if math.isfinite(value) and value >= 0 else 1)
PY
}

assert_q_registered_path() {
    local path=$1 kind=${2:-any} allow_missing=${3:-false}
    python3 - "$path" "$kind" "$allow_missing" <<'PY'
import os
import pathlib
import stat
import sys

raw, kind, allow_missing_text = sys.argv[1:]
allow_missing = allow_missing_text == "true"
if allow_missing_text not in {"true", "false"}:
    raise SystemExit("invalid allow-missing value")
if kind not in {"any", "directory", "file"}:
    raise SystemExit("invalid registered-path kind")
path = pathlib.PurePosixPath(raw)
if not path.is_absolute() or ".." in path.parts or str(path) != raw.rstrip("/"):
    raise SystemExit(f"registered q path is not canonical absolute POSIX: {raw}")

current = pathlib.Path(path.root)
parts = path.parts[1:]
for index, part in enumerate(parts):
    current = current / part
    final = index == len(parts) - 1
    try:
        info = current.lstat()
    except FileNotFoundError:
        if allow_missing:
            raise SystemExit(0)
        raise SystemExit(f"registered q path is absent: {current}")
    if stat.S_ISLNK(info.st_mode):
        raise SystemExit(f"symlink in registered q path: {current}")
    if not final and not stat.S_ISDIR(info.st_mode):
        raise SystemExit(f"non-directory ancestor in registered q path: {current}")
    if final:
        if kind == "directory" and not stat.S_ISDIR(info.st_mode):
            raise SystemExit(f"registered q directory is not plain: {current}")
        if kind == "file" and not stat.S_ISREG(info.st_mode):
            raise SystemExit(f"registered q file is not plain: {current}")
PY
}

windows_path_guard_script() {
    cat <<'POWERSHELL'
function ConvertTo-Ldt4CanonicalPath {
  param([Parameter(Mandatory=$true)][string]$LiteralPath)
  if ([string]::IsNullOrWhiteSpace($LiteralPath)) { throw 'registered Windows path is empty' }
  $native = $LiteralPath.Replace([char]47,[char]92)
  if ($native.Split([char]92,[StringSplitOptions]::RemoveEmptyEntries) -contains '..') { throw "registered Windows path contains traversal: $LiteralPath" }
  if (-not [IO.Path]::IsPathFullyQualified($native)) { throw "registered Windows path is not fully qualified: $LiteralPath" }
  $canonical = [IO.Path]::GetFullPath($native)
  return $canonical.TrimEnd([char]92,[char]47)
}
function Assert-Ldt4PlainPath {
  param(
    [Parameter(Mandatory=$true)][string]$LiteralPath,
    [ValidateSet('Any','Directory','File')][string]$Kind = 'Any',
    [bool]$AllowMissing = $false
  )
  $canonical = ConvertTo-Ldt4CanonicalPath $LiteralPath
  $root = [IO.Path]::GetPathRoot($canonical)
  if ($root -notmatch '^[A-Za-z]:\\$') { throw "registered Windows path has unsupported root: $LiteralPath" }
  $rootItem = Get-Item -LiteralPath $root -Force -ErrorAction Stop
  if (-not $rootItem.PSIsContainer -or (($rootItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
    throw "registered Windows volume root is not plain: $root"
  }
  $current = $root.TrimEnd([char]92)
  $parts = $canonical.Substring($root.Length).Split([char]92,[StringSplitOptions]::RemoveEmptyEntries)
  for ($index = 0; $index -lt $parts.Count; $index++) {
    $current = $current + [char]92 + $parts[$index]
    $final = $index -eq ($parts.Count - 1)
    if (-not (Test-Path -LiteralPath $current)) {
      if ($AllowMissing) { return $canonical }
      throw "registered Windows path is absent: $current"
    }
    $item = Get-Item -LiteralPath $current -Force -ErrorAction Stop
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "reparse point in registered Windows path: $current"
    }
    if (-not $final -and -not $item.PSIsContainer) {
      throw "non-directory ancestor in registered Windows path: $current"
    }
    if ($final -and $Kind -eq 'Directory' -and -not $item.PSIsContainer) {
      throw "registered Windows directory is not plain: $current"
    }
    if ($final -and $Kind -eq 'File' -and ($item.PSIsContainer -or -not ($item -is [IO.FileInfo]))) {
      throw "registered Windows file is not plain: $current"
    }
  }
  return $canonical
}
function ConvertTo-Ldt4CommandLine {
  param([string]$CommandLine)
  if ($null -eq $CommandLine) { return '' }
  return (($CommandLine.Replace([char]92,[char]47).Replace([string][char]34,'') -replace '\s+',' ').Trim()).ToLowerInvariant()
}
POWERSHELL
}

assert_q_registered_paths() {
    local phase=${1:-boundary} allow_staged_missing=false
    if [[ "$phase" == preflight ]]; then allow_staged_missing=true; fi
    assert_q_registered_path "$Q_ARTIFACT_REPO" directory false \
        || session_void "$phase: unsafe q artifact repository path"
    assert_q_registered_path "$Q_BLIT" file false \
        || session_void "$phase: unsafe q client path"
    assert_q_registered_path "$Q_DAEMON" file false \
        || session_void "$phase: unsafe q daemon path"
    assert_q_registered_path "$Q_STAGE_ROOT" directory true \
        || session_void "$phase: unsafe q staging path"
    assert_q_registered_path "$Q_STAGE_ROOT/fixtures/src_large" directory "$allow_staged_missing" \
        || session_void "$phase: unsafe q staged large fixture path"
    assert_q_registered_path "$Q_STAGE_ROOT/fixtures/src_small" directory "$allow_staged_missing" \
        || session_void "$phase: unsafe q small fixture path"
    assert_q_registered_path "$Q_STAGE_ROOT/fixtures/src_mixed" directory "$allow_staged_missing" \
        || session_void "$phase: unsafe q staged mixed fixture path"
    assert_q_registered_path "$Q_SESSION_ROOT" directory true \
        || session_void "$phase: unsafe q session path"
    assert_q_registered_path "$EVIDENCE_ROOT" directory true \
        || session_void "$phase: unsafe evidence root path"
    [[ -z "$OUT_DIR" ]] || assert_q_registered_path "$OUT_DIR" directory true \
        || session_void "$phase: unsafe evidence session path"
}

assert_windows_registered_paths() {
    local phase=${1:-boundary} guard out allow_small_missing=0
    if [[ "$phase" == preflight ]]; then allow_small_missing=1; fi
    guard=$(windows_path_guard_script)
    out=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
Assert-Ldt4PlainPath '$WIN_STAGE_ROOT' Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_BLIT' File | Out-Null
Assert-Ldt4PlainPath '$WIN_STAGE_DAEMON' File | Out-Null
Assert-Ldt4PlainPath 'D:/blit-test/rigw-module/src_large' Directory | Out-Null
Assert-Ldt4PlainPath 'D:/blit-test/rigw-module/src_mixed' Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_FIXTURE_STAGE' Directory \$true | Out-Null
Assert-Ldt4PlainPath '$WIN_FIXTURE_STAGE/fixtures/src_small' Directory ([bool]$allow_small_missing) | Out-Null
Assert-Ldt4PlainPath '$WIN_SESSION_ROOT' Directory \$true | Out-Null
Assert-Ldt4PlainPath 'D:/blit-test/bins/active' Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_ACTIVE_DAEMON' File \$true | Out-Null
'PATHS-PLAIN'
") || session_void "$phase: unsafe registered Windows path: $out"
    [[ "$(printf '%s\n' "$out" | tr -d '\r' | tail -1)" == PATHS-PLAIN ]] \
        || session_void "$phase: Windows path guard result malformed"
}

mark_void() {
    local reason=$1
    if [[ "$SESSION_STARTED" == 1 && -n "$OUT_DIR" && -d "$OUT_DIR" \
        && ! -e "$OUT_DIR/SESSION-VOID" && ! -L "$OUT_DIR/SESSION-VOID" ]]; then
        (set -o noclobber; printf '%s\n' "$reason" > "$OUT_DIR/SESSION-VOID") || true
    fi
    note "SESSION-VOID: $reason"
}

session_void() {
    mark_void "$*"
    exit 1
}

require_full_sha() {
    case "$1" in
        [0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]* ) ;;
        *) return 1 ;;
    esac
    [[ ${#1} -eq 40 && "$1" != *[!0-9a-f]* ]]
}

require_safe_tag() {
    [[ -n "$1" && "$1" != *[!A-Za-z0-9._-]* && "$1" != .* ]]
}

fixture_source() {
    local direction=$1 fixture=$2
    case "$direction:$fixture" in
        q_to_windows:large) printf '%s\n' "$Q_STAGE_ROOT/fixtures/src_large" ;;
        q_to_windows:small) printf '%s\n' "$Q_STAGE_ROOT/fixtures/src_small" ;;
        q_to_windows:mixed) printf '%s\n' "$Q_STAGE_ROOT/fixtures/src_mixed" ;;
        windows_to_q:large) printf '%s\n' 'D:/blit-test/rigw-module/src_large' ;;
        windows_to_q:small) printf '%s\n' "$WIN_FIXTURE_STAGE/fixtures/src_small" ;;
        windows_to_q:mixed) printf '%s\n' 'D:/blit-test/rigw-module/src_mixed' ;;
        *) die "unregistered fixture $direction:$fixture" ;;
    esac
}

destination_root() {
    case "$1" in
        q_to_windows) printf '%s\n' "$WIN_SESSION_ROOT" ;;
        windows_to_q) printf '%s\n' "$Q_SESSION_ROOT" ;;
        *) die "unregistered direction $1" ;;
    esac
}

active_destination() {
    printf '%s/%s/active/%s\n' "$(destination_root "$1")" "$SESSION_TAG" "$2"
}

retained_destination() {
    printf '%s/%s/retained/%s\n' "$(destination_root "$1")" "$SESSION_TAG" "$2"
}

first_role_for_pair() {
    case "$1" in
        1|4|5|8) printf '%s\n' source_init ;;
        2|3|6|7) printf '%s\n' destination_init ;;
        *) die "pair outside 1..8: $1" ;;
    esac
}

second_role_for_pair() {
    case "$(first_role_for_pair "$1")" in
        source_init) printf '%s\n' destination_init ;;
        destination_init) printf '%s\n' source_init ;;
    esac
}

emit_schedule() {
    local cell direction fixture pair first second sequence=0
    for cell in \
        q_to_windows_large \
        windows_to_q_large \
        windows_to_q_small \
        q_to_windows_small \
        q_to_windows_mixed \
        windows_to_q_mixed
    do
        direction=${cell%_*}
        fixture=${cell##*_}
        for pair in 1 2 3 4 5 6 7 8; do
            first=$(first_role_for_pair "$pair")
            second=$(second_role_for_pair "$pair")
            sequence=$((sequence + 1))
            printf '%03d,%s,%s,%s,%s\n' "$sequence" "$cell" "$direction" "$fixture" "$first"
            sequence=$((sequence + 1))
            printf '%03d,%s,%s,%s,%s\n' "$sequence" "$cell" "$direction" "$fixture" "$second"
        done
    done
}

assert_schedule() {
    local schedule lines role_sequence
    schedule=$(emit_schedule)
    lines=$(printf '%s\n' "$schedule" | awk 'END { print NR }')
    [[ "$lines" -eq "$ARM_COUNT" ]] || die "schedule has $lines arms, expected $ARM_COUNT"
    role_sequence=$(printf '%s\n' "$schedule" | awk -F, '$2=="q_to_windows_large" {print $5}' | paste -sd, -)
    [[ "$role_sequence" == 'source_init,destination_init,destination_init,source_init,destination_init,source_init,source_init,destination_init,source_init,destination_init,destination_init,source_init,destination_init,source_init,source_init,destination_init' ]] \
        || die 'schedule is not eight adjacent ABBAABBA role pairs'
}

write_q_manifest() {
    local root=$1 output=$2 expected_child=${3:-}
    assert_q_registered_path "$root" directory false \
        || die "manifest root has an unsafe ancestor: $root"
    assert_q_registered_path "$(dirname "$output")" directory false \
        || die "manifest output parent has an unsafe ancestor: $output"
    assert_q_registered_path "$output" file true \
        || die "manifest output path has an unsafe ancestor: $output"
    [[ ! -e "$output" && ! -L "$output" ]] || die "refusing manifest overwrite: $output"
    python3 - "$root" "$output" "$expected_child" <<'PY'
import base64
import hashlib
import os
import pathlib
import stat
import sys

root = pathlib.Path(sys.argv[1])
output = pathlib.Path(sys.argv[2])
expected_child = sys.argv[3]

def assert_plain_ancestors(path, final_directory):
    if not path.is_absolute() or ".." in path.parts:
        raise SystemExit(f"non-canonical manifest path: {path}")
    current = pathlib.Path(path.anchor)
    for index, part in enumerate(path.parts[1:]):
        current = current / part
        info = current.lstat()
        if stat.S_ISLNK(info.st_mode):
            raise SystemExit(f"symlink in manifest path: {current}")
        final = index == len(path.parts[1:]) - 1
        if (not final or final_directory) and not stat.S_ISDIR(info.st_mode):
            raise SystemExit(f"non-directory in manifest path: {current}")

assert_plain_ancestors(root, True)
assert_plain_ancestors(output.parent, True)
if not root.is_dir() or root.is_symlink():
    raise SystemExit(f"manifest root is not a plain directory: {root}")
if expected_child:
    children = list(root.iterdir())
    if (len(children) != 1 or children[0].name != expected_child
            or not children[0].is_dir() or children[0].is_symlink()):
        raise SystemExit(f"landed container must contain exactly {expected_child}")
    root = children[0]
    assert_plain_ancestors(root, True)

rows = []
decoded = set()
folded = set()
def walk_error(error):
    raise error
for current, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
    dirs.sort()
    files.sort()
    for name in dirs:
        path = pathlib.Path(current, name)
        mode = path.lstat().st_mode
        if not stat.S_ISDIR(mode) or stat.S_ISLNK(mode):
            raise SystemExit(f"non-directory or symlink in manifest: {path}")
    for name in files:
        path = pathlib.Path(current, name)
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode):
            raise SystemExit(f"non-regular entry in manifest: {path}")
        relative = path.relative_to(root).as_posix()
        if relative in decoded or relative.casefold() in folded:
            raise SystemExit(f"duplicate or case-colliding path: {relative}")
        decoded.add(relative)
        folded.add(relative.casefold())
        digest = hashlib.sha256()
        with path.open("rb") as handle:
            for block in iter(lambda: handle.read(8 * 1024 * 1024), b""):
                digest.update(block)
        encoded = base64.b64encode(relative.encode("utf-8")).decode("ascii")
        rows.append((encoded, info.st_size, digest.hexdigest()))
rows.sort(key=lambda row: row[0])
with output.open("x", encoding="ascii", newline="") as handle:
    for encoded, size, digest in rows:
        handle.write(f"{encoded},{size},{digest}\n")
PY
}

write_windows_manifest() {
    local root=$1 remote_output=$2 expected_child=${3:-} guard
    guard=$(windows_path_guard_script)
    wssh "$guard
\$ErrorActionPreference = 'Stop'
\$root = Assert-Ldt4PlainPath '$root' Directory
Assert-Ldt4PlainPath '$remote_output' File \$true | Out-Null
\$outputParent = Split-Path -Parent (ConvertTo-Ldt4CanonicalPath '$remote_output')
Assert-Ldt4PlainPath \$outputParent Directory | Out-Null
if ('$expected_child') {
  \$children = @(Get-ChildItem -LiteralPath \$root -Force -ErrorAction Stop)
  if (\$children.Count -ne 1 -or -not \$children[0].PSIsContainer -or
      \$children[0].Name -cne '$expected_child' -or
      ((\$children[0].Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
    throw 'landed container layout mismatch'
  }
  \$root = \$children[0].FullName.TrimEnd([char]92,[char]47)
  Assert-Ldt4PlainPath \$root Directory | Out-Null
}
\$lines = [Collections.Generic.List[string]]::new()
\$folded = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach (\$item in @(Get-ChildItem -LiteralPath \$root -Recurse -Force -ErrorAction Stop)) {
  if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw \"reparse entry in manifest: \$(\$item.FullName)\"
  }
  if (\$item.PSIsContainer) { continue }
  if (-not (\$item -is [IO.FileInfo])) { throw \"non-file entry in manifest: \$(\$item.FullName)\" }
  \$relative = \$item.FullName.Substring(\$root.Length).TrimStart([char]92,[char]47).Replace([char]92,[char]47)
  if (-not \$folded.Add(\$relative)) { throw \"case-colliding path: \$relative\" }
  \$encoded = [Convert]::ToBase64String([Text.UTF8Encoding]::new(\$false,\$true).GetBytes(\$relative))
  \$digest = (Get-FileHash -Algorithm SHA256 -LiteralPath \$item.FullName -ErrorAction Stop).Hash.ToLower()
  \$lines.Add(\"\$encoded,\$([uint64]\$item.Length),\$digest\")
}
\$ordered = [string[]]\$lines.ToArray()
[Array]::Sort(\$ordered, [StringComparer]::Ordinal)
\$text = if (\$ordered.Count) { (\$ordered -join \"\`n\") + \"\`n\" } else { '' }
\$encoding = [Text.UTF8Encoding]::new(\$false,\$true)
\$stream = [IO.File]::Open('$remote_output',[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try {
  \$bytes = \$encoding.GetBytes(\$text)
  \$stream.Write(\$bytes,0,\$bytes.Length)
  \$stream.Flush(\$true)
} finally { \$stream.Dispose() }
\"M|\$(\$ordered.Count)|\$((Get-FileHash -Algorithm SHA256 -LiteralPath '$remote_output').Hash.ToLower())\"
" | tr -d '\r' | tail -1
}

decode_windows_file_payload() {
    python3 -c '
import base64
import binascii
import sys

prefix = b"LDT4-FILE-B64|"
raw = sys.stdin.buffer.read().replace(b"\r\n", b"\n").replace(b"\r", b"\n")
payloads = [line[len(prefix):] for line in raw.split(b"\n") if line.startswith(prefix)]
if len(payloads) != 1:
    raise SystemExit(f"expected exactly one tagged Windows file payload, got {len(payloads)}")
try:
    decoded = base64.b64decode(payloads[0], validate=True)
except (binascii.Error, ValueError) as exc:
    raise SystemExit(f"malformed tagged Windows file payload: {exc}") from exc
sys.stdout.buffer.write(decoded)
'
}

fetch_windows_file() {
    local remote=$1 local_path=$2 remote_hash local_hash guard
    assert_q_registered_path "$(dirname "$local_path")" directory false \
        || die "unsafe fetch output parent: $local_path"
    assert_q_registered_path "$local_path" file true \
        || die "unsafe fetch output ancestry: $local_path"
    [[ ! -e "$local_path" && ! -L "$local_path" ]] || die "refusing fetch overwrite: $local_path"
    remote_hash=$(windows_sha256 "$remote")
    guard=$(windows_path_guard_script)
    if ! wssh "$guard
Assert-Ldt4PlainPath '$remote' File | Out-Null
[Console]::Out.Write(\"\`nLDT4-FILE-B64|\" + [Convert]::ToBase64String([IO.File]::ReadAllBytes('$remote')) + \"\`n\")
" \
        | decode_windows_file_payload > "$local_path"; then
        die "tagged Windows file fetch failed: $remote"
    fi
    local_hash=$(local_sha256 "$local_path")
    [[ "$local_hash" == "$remote_hash" ]] || die "fetched file hash mismatch: $remote"
}

manifest_shape() {
    python3 - "$1" <<'PY'
import csv, sys
n = b = 0
with open(sys.argv[1], encoding="ascii", newline="") as handle:
    for row in csv.reader(handle):
        if len(row) != 3:
            raise SystemExit("bad manifest row")
        n += 1
        b += int(row[1])
print(f"{n},{b}")
PY
}

expected_shape() {
    case "$1" in
        large) printf '%s\n' '1,1073741824' ;;
        small) printf '%s\n' '10000,40960000' ;;
        mixed) printf '%s\n' '5001,547110912' ;;
        *) die "unregistered fixture $1" ;;
    esac
}

verify_harness_identity() {
    local head status
    assert_q_registered_path "$REPO_ROOT" directory false \
        || die "harness repository has an unsafe ancestor: $REPO_ROOT"
    assert_q_registered_path "$SCRIPT_PATH" file false \
        || die "harness script has an unsafe ancestor: $SCRIPT_PATH"
    assert_q_registered_path "$ANALYZER" file false \
        || die "analyzer has an unsafe ancestor: $ANALYZER"
    head=$(git -C "$REPO_ROOT" rev-parse HEAD) || die 'cannot resolve harness repository HEAD'
    [[ "$head" == "$EXPECTED_HARNESS_SHA" ]] \
        || die "reviewed harness SHA is $EXPECTED_HARNESS_SHA but running tree is $head"
    status=$(git -C "$REPO_ROOT" status --porcelain --untracked-files=all) \
        || die 'cannot inspect harness repository state'
    [[ -z "$status" ]] || die 'harness repository is not an exact clean reviewed tree'
    [[ -f "$ANALYZER" && ! -L "$ANALYZER" && -x "$ANALYZER" ]] \
        || die "reviewed analyzer is not a plain executable: $ANALYZER"
}

q_embeds_clean_build() {
    local binary=$1
    LC_ALL=C grep -aFq "+$BUILD_ID" "$binary" \
        && ! LC_ALL=C grep -aFq "+$BUILD_ID.dirty" "$binary"
}

verify_artifacts() {
    local artifact_head artifact_status lock_hash q_client_hash q_daemon_hash win_client_hash win_daemon_hash
    artifact_head=$(git -C "$Q_ARTIFACT_REPO" rev-parse HEAD) \
        || die 'cannot resolve q artifact repository HEAD'
    [[ "$artifact_head" == "$ARTIFACT_SHA" ]] \
        || die "q artifact repository is $artifact_head, expected $ARTIFACT_SHA"
    artifact_status=$(git -C "$Q_ARTIFACT_REPO" status --porcelain --untracked-files=all) \
        || die 'cannot inspect q artifact repository state'
    [[ -z "$artifact_status" ]] || die 'q artifact repository is not an exact clean tree'
    lock_hash=$(local_sha256 "$Q_ARTIFACT_REPO/Cargo.lock")
    [[ "$lock_hash" == "$CARGO_LOCK_SHA" ]] || die "artifact Cargo.lock hash changed: $lock_hash"
    [[ -x "$Q_BLIT" && -f "$Q_BLIT" && ! -L "$Q_BLIT" ]] || die "q client absent: $Q_BLIT"
    [[ -x "$Q_DAEMON" && -f "$Q_DAEMON" && ! -L "$Q_DAEMON" ]] || die "q daemon absent: $Q_DAEMON"
    q_embeds_clean_build "$Q_BLIT" || die 'q client does not embed the exact clean build id'
    q_embeds_clean_build "$Q_DAEMON" || die 'q daemon does not embed the exact clean build id'
    wssh "
\$ErrorActionPreference = 'Stop'
foreach (\$path in @('$WIN_BLIT','$WIN_STAGE_DAEMON')) {
  if (-not (Test-Path -LiteralPath \$path -PathType Leaf)) { throw \"artifact absent: \$path\" }
  \$item = Get-Item -LiteralPath \$path -Force -ErrorAction Stop
  if ((\$item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw \"artifact is a reparse point: \$path\" }
  if (-not (Select-String -LiteralPath \$path -SimpleMatch -Quiet -Pattern '+$BUILD_ID')) { throw \"build id absent: \$path\" }
  if (Select-String -LiteralPath \$path -SimpleMatch -Quiet -Pattern '+$BUILD_ID.dirty') { throw \"dirty build: \$path\" }
}
" >/dev/null || die 'Windows artifacts are absent or not the exact clean build'
    q_client_hash=$(local_sha256 "$Q_BLIT")
    q_daemon_hash=$(local_sha256 "$Q_DAEMON")
    win_client_hash=$(windows_sha256 "$WIN_BLIT")
    win_daemon_hash=$(windows_sha256 "$WIN_STAGE_DAEMON")
    printf '%s|%s|%s|%s|%s\n' "$q_client_hash" "$q_daemon_hash" "$win_client_hash" "$win_daemon_hash" "$lock_hash"
}

reserve_evidence() {
    assert_q_registered_path "$EVIDENCE_ROOT" directory true \
        || die "unsafe evidence root ancestry: $EVIDENCE_ROOT"
    assert_q_registered_path "$OUT_DIR" directory true \
        || die "unsafe evidence session ancestry: $OUT_DIR"
    if [[ ! -e "$EVIDENCE_ROOT" && ! -L "$EVIDENCE_ROOT" ]]; then
        mkdir "$EVIDENCE_ROOT" || die "cannot create evidence root $EVIDENCE_ROOT"
    fi
    [[ -d "$EVIDENCE_ROOT" && ! -L "$EVIDENCE_ROOT" ]] \
        || die "evidence root is not a plain directory: $EVIDENCE_ROOT"
    [[ ! -e "$OUT_DIR" && ! -L "$OUT_DIR" ]] || die "evidence session already exists: $OUT_DIR"
    mkdir "$OUT_DIR" || die "cannot reserve evidence session $OUT_DIR"
    assert_q_registered_path "$OUT_DIR" directory false \
        || die "reserved evidence session is not plain: $OUT_DIR"
    SESSION_STARTED=1
    mkdir "$OUT_DIR/manifests" "$OUT_DIR/manifests/source" "$OUT_DIR/manifests/landed" \
        "$OUT_DIR/endpoint" "$OUT_DIR/endpoint/q" \
        "$OUT_DIR/endpoint/windows" || session_void 'cannot initialize fresh evidence directories'
    RUNS_CSV="$OUT_DIR/runs.csv"
}

reserve_endpoint_sessions() {
    local q_session="$Q_SESSION_ROOT/$SESSION_TAG" guard
    assert_q_registered_path "$Q_SESSION_ROOT" directory true \
        || session_void 'q session root has an unsafe ancestor'
    assert_q_registered_path "$q_session" directory true \
        || session_void 'q endpoint session has an unsafe ancestor'
    [[ ! -e "$q_session" && ! -L "$q_session" ]] \
        || session_void "q session already exists: $q_session"
    if [[ ! -e "$Q_SESSION_ROOT" && ! -L "$Q_SESSION_ROOT" ]]; then
        mkdir "$Q_SESSION_ROOT" || session_void "cannot create $Q_SESSION_ROOT"
    fi
    [[ -d "$Q_SESSION_ROOT" && ! -L "$Q_SESSION_ROOT" ]] \
        || session_void 'q session root is not a plain directory'
    mkdir "$q_session" "$q_session/active" "$q_session/retained" \
        || session_void 'cannot reserve q endpoint session'
    assert_q_registered_path "$q_session" directory false \
        || session_void 'reserved q endpoint session is not plain'
    guard=$(windows_path_guard_script)
    wssh "$guard
\$ErrorActionPreference = 'Stop'
\$session = '$WIN_SESSION_ROOT/$SESSION_TAG'
\$root = '$WIN_SESSION_ROOT'
Assert-Ldt4PlainPath \$root Directory \$true | Out-Null
Assert-Ldt4PlainPath \$session Directory \$true | Out-Null
if (-not (Test-Path -LiteralPath \$root)) { New-Item -ItemType Directory -Path \$root -ErrorAction Stop | Out-Null }
Assert-Ldt4PlainPath \$root Directory | Out-Null
if (Test-Path -LiteralPath \$session) { throw 'Windows session already exists' }
New-Item -ItemType Directory -Path \$session -ErrorAction Stop | Out-Null
Assert-Ldt4PlainPath \$session Directory | Out-Null
foreach (\$child in @('active','retained','logs','manifests')) {
  New-Item -ItemType Directory -Path "\$session/\$child" -ErrorAction Stop | Out-Null
  Assert-Ldt4PlainPath "\$session/\$child" Directory | Out-Null
}
" >/dev/null || session_void 'cannot reserve Windows endpoint session'
}

initialize_evidence_files() {
    local hashes=$1 q_client_hash q_daemon_hash win_client_hash win_daemon_hash lock_hash
    IFS='|' read -r q_client_hash q_daemon_hash win_client_hash win_daemon_hash lock_hash <<<"$hashes"
    WIN_STAGED_DAEMON_SHA=$win_daemon_hash
    exclusive_line "$OUT_DIR/provenance.csv" 'name,sha'
    append_line "$OUT_DIR/provenance.csv" "artifact,$ARTIFACT_SHA"
    append_line "$OUT_DIR/provenance.csv" "harness,$EXPECTED_HARNESS_SHA"
    exclusive_line "$OUT_DIR/artifact-build.txt" \
        "artifact_sha=$ARTIFACT_SHA build_id=$BUILD_ID cargo_lock_sha256=$lock_hash q_artifact_repo=$Q_ARTIFACT_REPO"
    exclusive_line "$OUT_DIR/staging-manifest.csv" \
        'endpoint,role,artifact_sha,build_id,sha256,staged_path,runtime_path'
    append_line "$OUT_DIR/staging-manifest.csv" \
        "q,client,$ARTIFACT_SHA,$BUILD_ID,$q_client_hash,$Q_BLIT,$Q_BLIT"
    append_line "$OUT_DIR/staging-manifest.csv" \
        "q,daemon,$ARTIFACT_SHA,$BUILD_ID,$q_daemon_hash,$Q_DAEMON,$Q_DAEMON"
    append_line "$OUT_DIR/staging-manifest.csv" \
        "windows,client,$ARTIFACT_SHA,$BUILD_ID,$win_client_hash,$WIN_BLIT,$WIN_BLIT"
    append_line "$OUT_DIR/staging-manifest.csv" \
        "windows,daemon,$ARTIFACT_SHA,$BUILD_ID,$win_daemon_hash,$WIN_STAGE_DAEMON,$WIN_ACTIVE_DAEMON"
    exclusive_line "$OUT_DIR/fixture-manifests.csv" 'direction,fixture,source_manifest'
    exclusive_line "$RUNS_CSV" \
        'cell,direction,fixture,pair,initiator,run_id,session_id,duration_ms,files,bytes,source_path,active_destination_path,archive_path,source_manifest,landed_manifest,source_trace,destination_trace,exit,valid'
}

stage_fixtures() {
    local q_state win_state transport="$WIN_FIXTURE_STAGE/src-small.transport.tar" guard
    local fixture remote_source local_destination incoming_root incoming
    local q_manifest win_manifest remote_manifest q_shape win_shape
    local q_free fixture_bytes required_free
    assert_q_registered_path "$Q_STAGE_ROOT" directory true \
        || session_void 'q small-fixture staging has an unsafe ancestor'
    guard=$(windows_path_guard_script)
    q_state=absent
    [[ ! -e "$Q_STAGE_ROOT" && ! -L "$Q_STAGE_ROOT" ]] || q_state=present
    win_state=$(wssh "if (Test-Path -LiteralPath '$WIN_FIXTURE_STAGE') { 'present' } else { 'absent' }" \
        | tr -d '\r' | tail -1)
    [[ "$win_state" == present || "$win_state" == absent ]] \
        || session_void "cannot determine Windows staging state: $win_state"
    if [[ "$q_state" != "$win_state" ]]; then
        session_void "partial fixture staging retained: q=$q_state windows=$win_state"
    fi
    if [[ "$q_state" == absent ]]; then
        note 'staging the fixed small fixture into fresh retained namespaces'
        wssh "$guard
\$ErrorActionPreference = 'Stop'
\$source = 'D:/blit-test/bench-module/pull_src_small/src_small'
Assert-Ldt4PlainPath \$source Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_FIXTURE_STAGE' Directory \$true | Out-Null
if (Test-Path -LiteralPath '$WIN_FIXTURE_STAGE') { throw 'Windows staging root appeared concurrently' }
New-Item -ItemType Directory -Path '$WIN_FIXTURE_STAGE' -ErrorAction Stop | Out-Null
Assert-Ldt4PlainPath '$WIN_FIXTURE_STAGE' Directory | Out-Null
New-Item -ItemType Directory -Path '$WIN_FIXTURE_STAGE/fixtures' -ErrorAction Stop | Out-Null
Assert-Ldt4PlainPath '$WIN_FIXTURE_STAGE/fixtures' Directory | Out-Null
Copy-Item -LiteralPath \$source -Destination '$WIN_FIXTURE_STAGE/fixtures' -Recurse -ErrorAction Stop
if (Test-Path -LiteralPath '$transport') { throw 'transport archive already exists' }
& tar.exe -cf '$transport' -C '$WIN_FIXTURE_STAGE/fixtures/src_small' .
if (\$LASTEXITCODE -ne 0) { throw \"tar creation failed rc=\$LASTEXITCODE\" }
" >/dev/null || session_void 'Windows small-fixture staging failed; partial tree retained'
        mkdir "$Q_STAGE_ROOT" "$Q_STAGE_ROOT/fixtures" "$Q_STAGE_ROOT/fixtures/src_small" \
            || session_void 'q small-fixture staging path failed; partial tree retained'
        assert_q_registered_path "$Q_STAGE_ROOT/fixtures/src_small" directory false \
            || session_void 'q small-fixture staging created an unsafe path'
        wssh "[Console]::Out.Write([Convert]::ToBase64String([IO.File]::ReadAllBytes('$transport')))" \
            | python3 -c 'import base64,sys; base64.decode(sys.stdin.buffer,sys.stdout.buffer)' \
            | tar -xf - -C "$Q_STAGE_ROOT/fixtures/src_small" \
            || session_void 'q small-fixture extraction failed; partial tree retained'
    fi
    [[ -d "$Q_STAGE_ROOT/fixtures/src_small" && ! -L "$Q_STAGE_ROOT/fixtures/src_small" ]] \
        || session_void 'q staged small fixture is not a plain directory'
    wssh "$guard
Assert-Ldt4PlainPath '$WIN_FIXTURE_STAGE/fixtures/src_small' Directory | Out-Null
" >/dev/null || session_void 'Windows staged small fixture is not a plain directory'
    for fixture in large mixed; do
        remote_source=$(fixture_source windows_to_q "$fixture")
        local_destination=$(fixture_source q_to_windows "$fixture")
        assert_q_registered_path "$local_destination" directory true \
            || session_void "q staged $fixture fixture has an unsafe ancestor"
        if [[ ! -e "$local_destination" && ! -L "$local_destination" ]]; then
            incoming_root="$Q_SESSION_ROOT/$SESSION_TAG/incoming-fixtures"
            assert_q_registered_path "$incoming_root" directory true \
                || session_void "q incoming $fixture fixture has an unsafe ancestor"
            if [[ ! -e "$incoming_root" && ! -L "$incoming_root" ]]; then
                mkdir "$incoming_root" \
                    || session_void 'q incoming-fixture namespace creation failed'
            fi
            assert_q_registered_path "$incoming_root" directory false \
                || session_void 'q incoming-fixture namespace is not plain'
            incoming="$incoming_root/src_$fixture"
            [[ ! -e "$incoming" && ! -L "$incoming" ]] \
                || session_void "q incoming $fixture fixture already exists"
            q_manifest="$OUT_DIR/manifests/staging-q-$fixture.csv"
            win_manifest="$OUT_DIR/manifests/staging-windows-$fixture.csv"
            remote_manifest="$WIN_SESSION_ROOT/$SESSION_TAG/manifests/staging-source-$fixture.csv"
            write_windows_manifest "$remote_source" "$remote_manifest" >/dev/null \
                || session_void "Windows canonical $fixture manifest failed before staging"
            fetch_windows_file "$remote_manifest" "$win_manifest" \
                || session_void "Windows canonical $fixture manifest fetch failed before staging"
            fixture_bytes=$(expected_shape "$fixture")
            fixture_bytes=${fixture_bytes#*,}
            q_free=$(df -Pk "$Q_SESSION_ROOT" | awk 'NR==2 {printf "%.0f", $4 * 1024}')
            required_free=$((MIN_FREE_BYTES + fixture_bytes))
            awk -v free="$q_free" -v need="$required_free" 'BEGIN {exit !(free >= need)}' \
                || session_void "q free bytes $q_free cannot stage $fixture and retain $MIN_FREE_BYTES"
            note "copying the canonical Windows $fixture fixture into this session's retained incoming namespace"
            scp -r "${SSH_MUX[@]}" "$WIN_SSH:$remote_source" "$incoming_root/" \
                || session_void "q $fixture fixture staging failed; partial tree retained"
            assert_q_registered_path "$incoming" directory false \
                || session_void "q incoming $fixture fixture is not a plain directory"
            write_q_manifest "$incoming" "$q_manifest" \
                || session_void "q incoming $fixture manifest failed"
            q_shape=$(manifest_shape "$q_manifest")
            win_shape=$(manifest_shape "$win_manifest")
            [[ "$q_shape" == "$(expected_shape "$fixture")" && "$win_shape" == "$q_shape" ]] \
                || session_void "staged $fixture shape differs: q=$q_shape windows=$win_shape"
            cmp -s "$q_manifest" "$win_manifest" \
                || session_void "staged $fixture content differs from the canonical Windows source"
            [[ ! -e "$local_destination" && ! -L "$local_destination" ]] \
                || session_void "q staged $fixture destination appeared concurrently"
            rename_q_directory_exclusive "$incoming" "$local_destination" \
                || session_void "q staged $fixture promotion failed; validated incoming tree retained"
            [[ ! -e "$incoming" && ! -L "$incoming" ]] \
                || session_void "q staged $fixture promotion left the incoming tree in place"
            note "validated and promoted the canonical Windows $fixture fixture into q's stable retained path"
        fi
        assert_q_registered_path "$local_destination" directory false \
            || session_void "q staged $fixture fixture is not a plain directory"
    done
}

build_fixture_manifests() {
    local fixture q_rel q_abs win_rel win_abs remote_win q_shape win_shape
    for fixture in large small mixed; do
        q_rel="manifests/source/q_to_windows_${fixture}.csv"
        q_abs="$OUT_DIR/$q_rel"
        win_rel="manifests/source/windows_to_q_${fixture}.csv"
        win_abs="$OUT_DIR/$win_rel"
        remote_win="$WIN_SESSION_ROOT/$SESSION_TAG/manifests/source-${fixture}.csv"
        write_q_manifest "$(fixture_source q_to_windows "$fixture")" "$q_abs"
        write_windows_manifest "$(fixture_source windows_to_q "$fixture")" "$remote_win" >/dev/null \
            || session_void "Windows $fixture source manifest failed"
        fetch_windows_file "$remote_win" "$win_abs" \
            || session_void "Windows $fixture source manifest fetch failed"
        q_shape=$(manifest_shape "$q_abs")
        win_shape=$(manifest_shape "$win_abs")
        [[ "$q_shape" == "$(expected_shape "$fixture")" ]] \
            || session_void "q $fixture shape is $q_shape"
        [[ "$win_shape" == "$(expected_shape "$fixture")" ]] \
            || session_void "Windows $fixture shape is $win_shape"
        cmp -s "$q_abs" "$win_abs" \
            || session_void "q and Windows $fixture fixtures differ by path, size, or content"
        append_line "$OUT_DIR/fixture-manifests.csv" "q_to_windows,$fixture,$q_rel"
        append_line "$OUT_DIR/fixture-manifests.csv" "windows_to_q,$fixture,$win_rel"
        note "fixture $fixture content manifest verified on both endpoints ($q_shape)"
    done
}

q_port_closed() {
    ! lsof -nP -iTCP:"$DAEMON_PORT" -sTCP:LISTEN 2>/dev/null | grep -q .
}

windows_port_closed() {
    wssh "if (Get-NetTCPConnection -State Listen -LocalPort $DAEMON_PORT -ErrorAction SilentlyContinue) { exit 9 }" \
        >/dev/null 2>&1
}

ports_closed() {
    q_port_closed && windows_port_closed
}

q_quiet_gate() {
    local deadline=$((SECONDS + 120)) load bad spotlight auto tm_running
    while :; do
        bad=$(ps -axo comm= | awk '
            {name=$1; sub(/^.*\//,"",name)}
            name == "cargo" || name == "rustc" || name == "blit" || name == "blit-daemon" || name ~ /^codex($|-)/ {print name}' | head -1)
        [[ -z "$bad" ]] || session_void "q conflicting process: $bad"
        auto=$(defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null || printf unknown)
        [[ "$auto" == 0 ]] || session_void "q Time Machine AutoBackup changed to $auto"
        tm_running=$(tmutil status 2>/dev/null | awk -F'= ' '/Running/ {gsub(/[ ;]/,"",$2); print $2; exit}')
        [[ "$tm_running" == 0 ]] || session_void "q Time Machine began running"
        load=$(sysctl -n vm.loadavg | awk '{gsub(/[{}]/,""); print $1}')
        spotlight=$(ps -axo %cpu=,comm= | awk '
            $2 ~ /(mds|mds_stores|mdworker|mdbulkimport)$/ {sum += $1}
            END {printf "%.1f", sum + 0}')
        finite_nonnegative_number "$load" \
            || session_void "q quiet load sample is not finite numeric: $load"
        finite_nonnegative_number "$spotlight" \
            || session_void "q Spotlight CPU sample is not finite numeric: $spotlight"
        if awk -v value="$load" -v spot="$spotlight" \
            'BEGIN { exit !(value <= 3.0 && spot <= 10.0) }'; then
            printf 'q_load1=%s q_spotlight_cpu=%s time_machine_auto=%s time_machine_running=%s\n' \
                "$load" "$spotlight" "$auto" "$tm_running"
            return
        fi
        [[ "$SECONDS" -lt "$deadline" ]] \
            || session_void "q quiet gate timed out: load1=$load Spotlight=$spotlight"
        sleep 5
    done
}

windows_quiet_gate() {
    local out avg deadline=$((SECONDS + 120))
    while :; do
        out=$(wssh "
\$ErrorActionPreference = 'Stop'
\$bad = @(Get-Process cargo,rustc,blit,blit-daemon -ErrorAction SilentlyContinue)
if (\$bad.Count) { throw \"conflicting process: \$(\$bad.Name -join ',')\" }
\$samples = @(1..3 | ForEach-Object {
  \$value = (Get-CimInstance Win32_Processor -ErrorAction Stop | Measure-Object LoadPercentage -Average).Average
  if (\$null -eq \$value -or [double]::IsNaN([double]\$value) -or [double]::IsInfinity([double]\$value) -or [double]\$value -lt 0) { throw \"non-finite CPU sample: \$value\" }
  Start-Sleep -Seconds 1
  [double]\$value
})
\$avg = (\$samples | Measure-Object -Average).Average
if (\$null -eq \$avg -or [double]::IsNaN([double]\$avg) -or [double]::IsInfinity([double]\$avg) -or [double]\$avg -lt 0) { throw \"non-finite CPU average: \$avg\" }
\"windows_cpu_avg=\$([Math]::Round(\$avg,2))\"
") || session_void "Windows quiet gate failed: $out"
        out=$(printf '%s\n' "$out" | tr -d '\r' | tail -1)
        avg=${out#windows_cpu_avg=}
        [[ "$avg" =~ ^[0-9]+([.][0-9]+)?$ ]] \
            || session_void "Windows quiet CPU result malformed: $out"
        finite_nonnegative_number "$avg" \
            || session_void "Windows quiet CPU result is not finite: $out"
        if awk -v value="$avg" 'BEGIN {exit !(value <= 20.0)}'; then
            printf '%s\n' "$out"
            return
        fi
        [[ "$SECONDS" -lt "$deadline" ]] \
            || session_void "Windows CPU remained above 20%: $avg"
        sleep 5
    done
}

environment_gate() {
    local phase=$1 q_free win_free q_route q_route_raw q_route_mtu q_mtu q_media q_status q_mac q_arp q_peer auto tm_running win_topology quiet_q quiet_win q_mss q_mss_ip win_mss win_mss_ip win_mss_raw win_ps guard
    assert_q_registered_paths "$phase"
    assert_windows_registered_paths "$phase"
    ports_closed || session_void "$phase: daemon port is occupied"
    [[ "$(hostname)" == q.lan ]] || session_void "$phase: harness is not executing on q.lan"
    q_route_raw=$(route -n get "$WIN_IP") || session_void "$phase: q route probe failed"
    q_route=$(printf '%s\n' "$q_route_raw" | awk '/interface:/ {print $2; exit}')
    [[ "$q_route" == "$Q_NIC" ]] || session_void "$phase: q route uses $q_route, expected $Q_NIC"
    q_route_mtu=$(printf '%s\n' "$q_route_raw" | awk '/mtu/ {getline; print $(NF-1); exit}')
    [[ "$q_route_mtu" == 9000 ]] || session_void "$phase: q route MTU is $q_route_mtu"
    q_mtu=$(ifconfig "$Q_NIC" | awk '/mtu / {for(i=1;i<=NF;i++) if($i=="mtu") {print $(i+1); exit}}')
    [[ "$q_mtu" == 9000 ]] || session_void "$phase: q MTU is $q_mtu"
    q_media=$(ifconfig "$Q_NIC" | awk -F': ' '/media:/ {print $2; exit}')
    [[ "$q_media" == *10Gbase-T* ]] || session_void "$phase: q media is $q_media"
    ifconfig "$Q_NIC" | grep -q "inet $Q_IP " || session_void "$phase: q $Q_NIC does not own $Q_IP"
    q_status=$(ifconfig "$Q_NIC" | awk -F': ' '/status:/ {print $2; exit}')
    [[ "$q_status" == active ]] || session_void "$phase: q interface status is $q_status"
    q_mac=$(ifconfig "$Q_NIC" | awk '/ether / {print tolower($2); exit}')
    [[ "$q_mac" == '00:01:d2:19:04:a3' ]] || session_void "$phase: q MAC is $q_mac"
    ping -c 1 -W 1000 "$WIN_IP" >/dev/null || session_void "$phase: q cannot ping Windows"
    q_arp=$(arp -n "$WIN_IP") || session_void "$phase: q ARP probe failed"
    q_peer=$(printf '%s\n' "$q_arp" | awk -v nic="$Q_NIC" '$3=="at" && $5=="on" && $6==nic {print tolower($4)}')
    [[ "$q_peer" == '34:5a:60:3e:78:8b' ]] \
        || session_void "$phase: q $Q_NIC peer is ${q_peer:-absent}, expected Windows MAC"
    auto=$(defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null || printf unknown)
    [[ "$auto" == 0 ]] || session_void "$phase: Time Machine AutoBackup is $auto, expected manual"
    tm_running=$(tmutil status 2>/dev/null | awk -F'= ' '/Running/ {gsub(/[ ;]/,"",$2); print $2; exit}')
    [[ "$tm_running" == 0 ]] || session_void "$phase: Time Machine Running is ${tm_running:-unknown}"
    q_free=$(df -Pk "$Q_SESSION_ROOT" | awk 'NR==2 {printf "%.0f", $4 * 1024}')
    awk -v free="$q_free" -v need="$MIN_FREE_BYTES" 'BEGIN {exit !(free >= need)}' \
        || session_void "$phase: q free bytes $q_free below $MIN_FREE_BYTES"
    guard=$(windows_path_guard_script)
    win_topology=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
\$psVersion = \$PSVersionTable.PSVersion
if (\$psVersion -lt [version]'7.4') { throw \"PowerShell \$psVersion is below required 7.4\" }
Assert-Ldt4PlainPath '$WIN_FIXTURE_STAGE/fixtures/src_small' Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_SESSION_ROOT/$SESSION_TAG' Directory | Out-Null
if (\$env:COMPUTERNAME -ne 'NETWATCH-01') { throw \"unexpected host: \$(\$env:COMPUTERNAME)\" }
\$adapter = Get-NetAdapter -Name '$WIN_NIC' -ErrorAction Stop
if (\$adapter.Status -ne 'Up' -or \$adapter.LinkSpeed -notmatch '^10 Gbps$' -or \$adapter.ReceiveLinkSpeed -ne 10000000000 -or \$adapter.TransmitLinkSpeed -ne 10000000000 -or \$adapter.MacAddress -ne '34-5A-60-3E-78-8B') { throw \"adapter state: \$(\$adapter.Status) \$(\$adapter.LinkSpeed) \$(\$adapter.MacAddress)\" }
\$mtu = (Get-NetIPInterface -InterfaceAlias '$WIN_NIC' -AddressFamily IPv4 -ErrorAction Stop).NlMtu
if (\$mtu -ne 9000) { throw \"MTU=\$mtu\" }
\$ip = @(Get-NetIPAddress -InterfaceAlias '$WIN_NIC' -AddressFamily IPv4 -ErrorAction Stop | Where-Object IPAddress -eq '$WIN_IP')
if (\$ip.Count -ne 1) { throw 'registered IPv4 absent' }
\$route = Find-NetRoute -RemoteIPAddress '$Q_IP' | Select-Object -First 1
if (-not \$route -or \$route.InterfaceAlias -ne '$WIN_NIC') { throw \"route interface=\$(\$route.InterfaceAlias)\" }
if (\$route.IPAddress -ne '$WIN_IP') { throw \"route source=\$(\$route.IPAddress)\" }
if (-not (Test-Connection -ComputerName '$Q_IP' -Count 1 -Quiet -ErrorAction Stop)) { throw 'Windows cannot ping q' }
\$neighbor = @(Get-NetNeighbor -InterfaceAlias '$WIN_NIC' -IPAddress '$Q_IP' -ErrorAction Stop)
if (\$neighbor.Count -ne 1 -or \$neighbor[0].LinkLayerAddress -ne '00-01-D2-19-04-A3') { throw \"q neighbor=\$(\$neighbor.LinkLayerAddress -join ',')\" }
\$rule = @(Get-NetFirewallRule -DisplayName 'blit-otp12-daemon' -ErrorAction Stop)
if (\$rule.Count -ne 1 -or \$rule[0].Enabled -ne 'True' -or \$rule[0].Direction -ne 'Inbound' -or \$rule[0].Action -ne 'Allow') { throw 'firewall rule shape mismatch' }
\$program = @(\$rule[0] | Get-NetFirewallApplicationFilter -ErrorAction Stop)
\$actualProgram = if (\$program.Count -eq 1) { \$program[0].Program.Replace([char]92,[char]47) } else { '' }
if (\$actualProgram -ine '$WIN_ACTIVE_DAEMON') { throw \"firewall program=\$actualProgram\" }
\$drive = Get-PSDrive D -ErrorAction Stop
\"W|\$(\$drive.Free)|\$mtu|\$(\$adapter.LinkSpeed)|\$(\$route.InterfaceAlias)|\$actualProgram|\$(\$neighbor[0].LinkLayerAddress)|\$(\$psVersion.ToString())\"
") || session_void "$phase: Windows topology/firewall gate failed: $win_topology"
    win_topology=$(printf '%s\n' "$win_topology" | tr -d '\r' | tail -1)
    win_free=$(printf '%s\n' "$win_topology" | awk -F'|' '{print $2}')
    win_ps=$(printf '%s\n' "$win_topology" | awk -F'|' '{print $8}')
    [[ "$win_free" =~ ^[0-9]+$ ]] || session_void "$phase: Windows free bytes malformed"
    [[ "$win_ps" =~ ^[0-9]+[.][0-9]+([.][0-9]+)?([.][0-9]+)?$ ]] \
        || session_void "$phase: Windows PowerShell version malformed: $win_ps"
    awk -v free="$win_free" -v need="$MIN_FREE_BYTES" 'BEGIN {exit !(free >= need)}' \
        || session_void "$phase: Windows free bytes $win_free below $MIN_FREE_BYTES"
    quiet_q=$(q_quiet_gate)
    quiet_win=$(windows_quiet_gate)
    IFS=' ' read -r q_mss q_mss_ip <<<"$(python3 - "$WIN_IP" <<'PY'
import socket, sys
sock = socket.create_connection((sys.argv[1], 22), timeout=5)
try:
    print(sock.getsockopt(socket.IPPROTO_TCP, socket.TCP_MAXSEG), sock.getsockname()[0])
finally:
    sock.close()
PY
)"
    [[ "$q_mss" == 8948 && "$q_mss_ip" == "$Q_IP" ]] \
        || session_void "$phase: q-to-Windows MSS/source is $q_mss/$q_mss_ip"
    win_mss_raw=$(wssh "
\$ErrorActionPreference = 'Stop'
\$socket = [Net.Sockets.Socket]::new([Net.Sockets.AddressFamily]::InterNetwork,[Net.Sockets.SocketType]::Stream,[Net.Sockets.ProtocolType]::Tcp)
\$socket.Connect('$Q_IP',22)
\$bytes = \$socket.GetSocketOption([Net.Sockets.SocketOptionLevel]::Tcp,[Net.Sockets.SocketOptionName]4,4)
\$mss = [BitConverter]::ToInt32(\$bytes,0)
\"M|\$mss|\$(\$socket.LocalEndPoint.Address)\"
\$socket.Dispose()
") || session_void "$phase: Windows-to-q MSS probe failed"
    IFS='|' read -r _ win_mss win_mss_ip <<<"$(printf '%s\n' "$win_mss_raw" | tr -d '\r' | tail -1)"
    [[ "$win_mss" == 8960 && "$win_mss_ip" == "$WIN_IP" ]] \
        || session_void "$phase: Windows-to-q MSS/source is $win_mss/$win_mss_ip"
    exclusive_line "$OUT_DIR/environment-$phase.txt" \
        "phase=$phase q_ip=$Q_IP q_nic=$Q_NIC q_mtu=$q_mtu q_media=$q_media q_route=$q_route q_route_mtu=$q_route_mtu q_mac=$q_mac q_peer=$q_peer q_free=$q_free $quiet_q q_to_windows_mss=$q_mss windows_to_q_mss=$win_mss windows_powershell=$win_ps windows=$win_topology $quiet_win"
}

prepare_windows_runtime() {
    local out active_hash guard remote_record swap_record
    WIN_PRIOR_DAEMON="D:/blit-test/bins/active/retained-before-$SESSION_TAG-blit-daemon.exe"
    WIN_TESTED_DAEMON="D:/blit-test/bins/active/retained-tested-$SESSION_TAG-blit-daemon.exe"
    remote_record="$WIN_SESSION_ROOT/$SESSION_TAG/runtime-swap-intent.txt"
    WIN_SWAP_ATTEMPTED=1
    guard=$(windows_path_guard_script)
    out=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
\$active = '$WIN_ACTIVE_DAEMON'
\$prior = '$WIN_PRIOR_DAEMON'
\$tested = '$WIN_TESTED_DAEMON'
\$record = '$remote_record'
Assert-Ldt4PlainPath '$WIN_STAGE_DAEMON' File | Out-Null
Assert-Ldt4PlainPath (Split-Path -Parent (ConvertTo-Ldt4CanonicalPath \$active)) Directory | Out-Null
Assert-Ldt4PlainPath \$active File \$true | Out-Null
Assert-Ldt4PlainPath \$prior File \$true | Out-Null
Assert-Ldt4PlainPath \$tested File \$true | Out-Null
Assert-Ldt4PlainPath \$record File \$true | Out-Null
if (Test-Path -LiteralPath \$prior) { throw 'prior-retention target already exists' }
if (Test-Path -LiteralPath \$tested) { throw 'tested-retention target already exists' }
if (Test-Path -LiteralPath \$record) { throw 'runtime swap intent already exists' }
\$stagedHash = (Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_STAGE_DAEMON').Hash.ToLower()
if (\$stagedHash -cne '$WIN_STAGED_DAEMON_SHA') { throw 'staged daemon changed before baseline check' }
\$activeParent = Split-Path -Parent (ConvertTo-Ldt4CanonicalPath \$active)
\$retainedPriors = @(Get-ChildItem -LiteralPath \$activeParent -Force -ErrorAction Stop |
  Where-Object { \$_.Name -like 'retained-before-*-blit-daemon.exe' })
foreach (\$retainedPrior in \$retainedPriors) {
  if (\$retainedPrior.PSIsContainer -or
      ((\$retainedPrior.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
    throw \"unsafe retained-before daemon entry: \$(\$retainedPrior.FullName)\"
  }
}
if (\$retainedPriors.Count -ne 0) { throw 'unresolved retained-before daemon from an earlier session' }
\$hadPrior = Test-Path -LiteralPath \$active
if (\$hadPrior) {
  \$activeItem = Get-Item -LiteralPath \$active -Force -ErrorAction Stop
  if ((\$activeItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'active daemon is a reparse point' }
}
\$priorHash = if (\$hadPrior) { (Get-FileHash -Algorithm SHA256 -LiteralPath \$active).Hash.ToLower() } else { 'none' }
if (\$hadPrior -and \$priorHash -ceq \$stagedHash) { throw 'active daemon already matches staged test daemon; prior baseline is ambiguous' }
\$recordText = \"had_prior=\$([int]\$hadPrior) prior_sha=\$priorHash staged_sha=\$stagedHash\`n\"
\$recordStream = [IO.File]::Open(\$record,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try {
  \$recordBytes = [Text.Encoding]::ASCII.GetBytes(\$recordText)
  \$recordStream.Write(\$recordBytes,0,\$recordBytes.Length)
  \$recordStream.Flush(\$true)
} finally { \$recordStream.Dispose() }
Write-VolumeCache D -ErrorAction Stop
if (\$hadPrior) { [IO.File]::Move((ConvertTo-Ldt4CanonicalPath \$active),(ConvertTo-Ldt4CanonicalPath \$prior)) }
\$sourceStream = [IO.File]::Open((ConvertTo-Ldt4CanonicalPath '$WIN_STAGE_DAEMON'),[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
try {
  \$activeStream = [IO.File]::Open((ConvertTo-Ldt4CanonicalPath \$active),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try { \$sourceStream.CopyTo(\$activeStream); \$activeStream.Flush(\$true) } finally { \$activeStream.Dispose() }
} finally { \$sourceStream.Dispose() }
Assert-Ldt4PlainPath \$active File | Out-Null
\$activeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath \$active).Hash.ToLower()
if (\$activeHash -cne \$stagedHash) { throw 'active daemon hash differs from staged daemon' }
\"S|\$([int]\$hadPrior)|\$priorHash|\$activeHash\"
") || session_void "Windows active daemon preparation failed: $out"
    out=$(printf '%s\n' "$out" | tr -d '\r' | tail -1)
    IFS='|' read -r _ WIN_HAD_PRIOR WIN_PRIOR_DAEMON_SHA active_hash <<<"$out"
    [[ "$WIN_HAD_PRIOR" == 0 || "$WIN_HAD_PRIOR" == 1 ]] \
        || session_void "Windows active daemon state malformed: $out"
    [[ "$active_hash" == "$WIN_STAGED_DAEMON_SHA" ]] \
        || session_void "Windows staged/active daemon hash mismatch: $out"
    if [[ "$WIN_HAD_PRIOR" == 1 ]]; then
        [[ "$WIN_PRIOR_DAEMON_SHA" =~ ^[0-9a-f]{64}$ ]] \
            || session_void "Windows prior daemon hash malformed: $out"
    else
        [[ "$WIN_PRIOR_DAEMON_SHA" == none ]] \
            || session_void "Windows absent-prior state malformed: $out"
    fi
    fetch_windows_file "$remote_record" "$OUT_DIR/windows-runtime-swap.txt" \
        || session_void 'cannot fetch durable Windows runtime swap intent'
    swap_record=$(tr -d '\r\n' < "$OUT_DIR/windows-runtime-swap.txt")
    [[ "$swap_record" == "had_prior=$WIN_HAD_PRIOR prior_sha=$WIN_PRIOR_DAEMON_SHA staged_sha=$WIN_STAGED_DAEMON_SHA" ]] \
        || session_void "Windows runtime intent differs from preparation result: $swap_record"
    WIN_PREP_COMPLETE=1
}

restore_windows_runtime() {
    local mode=${1:-recovery} out guard remote_record
    [[ "$WIN_SWAP_ATTEMPTED" == 1 ]] || return 0
    [[ "$mode" == normal || "$mode" == recovery ]] || return 1
    if [[ "$mode" == normal ]]; then
        [[ "$WIN_PREP_COMPLETE" == 1 ]] || return 1
    fi
    remote_record="$WIN_SESSION_ROOT/$SESSION_TAG/runtime-swap-intent.txt"
    guard=$(windows_path_guard_script)
    out=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
\$active = '$WIN_ACTIVE_DAEMON'
\$prior = '$WIN_PRIOR_DAEMON'
\$tested = '$WIN_TESTED_DAEMON'
\$record = '$remote_record'
Assert-Ldt4PlainPath (Split-Path -Parent (ConvertTo-Ldt4CanonicalPath \$active)) Directory | Out-Null
Assert-Ldt4PlainPath \$active File \$true | Out-Null
Assert-Ldt4PlainPath \$prior File \$true | Out-Null
Assert-Ldt4PlainPath \$tested File \$true | Out-Null
Assert-Ldt4PlainPath \$record File \$true | Out-Null
if (-not (Test-Path -LiteralPath \$record)) {
  if ('$mode' -eq 'normal') { throw 'normal restoration requires durable swap intent' }
  'RESTORED|mode=recovery|state=untouched-no-intent'
  return
}
Assert-Ldt4PlainPath \$record File | Out-Null
\$recordText = (Get-Content -LiteralPath \$record -Raw -ErrorAction Stop).Trim()
\$recordMatch = [regex]::Match(\$recordText,'\Ahad_prior=([01]) prior_sha=(none|[0-9a-f]{64}) staged_sha=([0-9a-f]{64})\z')
if (-not \$recordMatch.Success) { throw 'runtime swap intent is malformed' }
\$hadPrior = \$recordMatch.Groups[1].Value -ceq '1'
\$expectedPrior = \$recordMatch.Groups[2].Value
\$stagedHash = \$recordMatch.Groups[3].Value
if (\$hadPrior -ne (\$expectedPrior -cne 'none')) { throw 'runtime swap intent prior state is inconsistent' }
if ('$WIN_STAGED_DAEMON_SHA' -match '^[0-9a-f]{64}$' -and \$stagedHash -cne '$WIN_STAGED_DAEMON_SHA') { throw 'local and durable staged daemon binding differ' }

\$priorExists = Test-Path -LiteralPath \$prior
\$testedExists = Test-Path -LiteralPath \$tested
\$activeExists = Test-Path -LiteralPath \$active
if (\$priorExists) {
  if (-not \$hadPrior) { throw 'prior retention exists for originally absent active daemon' }
  Assert-Ldt4PlainPath \$prior File | Out-Null
  \$priorHash = (Get-FileHash -Algorithm SHA256 -LiteralPath \$prior).Hash.ToLower()
  if (\$priorHash -cne \$expectedPrior) { throw 'prior daemon changed before restoration' }
}
if (\$testedExists) { Assert-Ldt4PlainPath \$tested File | Out-Null }
if (\$activeExists) {
  Assert-Ldt4PlainPath \$active File | Out-Null
  \$activeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath \$active).Hash.ToLower()
  \$originalAlreadyActive = \$hadPrior -and -not \$priorExists -and \$activeHash -ceq \$expectedPrior
  if (-not \$originalAlreadyActive) {
    if (\$testedExists) { throw 'active and tested retention are both occupied during recovery' }
    [IO.File]::Move((ConvertTo-Ldt4CanonicalPath \$active),(ConvertTo-Ldt4CanonicalPath \$tested))
    \$testedExists = \$true
    \$activeExists = \$false
  }
}
if (\$hadPrior) {
  if (\$priorExists) {
    if (\$activeExists) { throw 'active path occupied before prior restore' }
    [IO.File]::Move((ConvertTo-Ldt4CanonicalPath \$prior),(ConvertTo-Ldt4CanonicalPath \$active))
    \$activeExists = \$true
    \$priorExists = \$false
  }
  if (-not \$activeExists) { throw 'original active daemon was not restored' }
  Assert-Ldt4PlainPath \$active File | Out-Null
  \$restoredHash = (Get-FileHash -Algorithm SHA256 -LiteralPath \$active).Hash.ToLower()
  if (\$restoredHash -cne \$expectedPrior) { throw 'restored daemon differs from durable prior SHA' }
} else {
  if (\$priorExists) { throw 'originally absent active daemon has prior retention' }
  if (\$activeExists) { throw 'recovery recreated an originally absent active daemon' }
  \$restoredHash = 'none'
}
if (\$testedExists) {
  Assert-Ldt4PlainPath \$tested File | Out-Null
  \$testedHash = (Get-FileHash -Algorithm SHA256 -LiteralPath \$tested).Hash.ToLower()
} else { \$testedHash = 'none' }
if ('$mode' -eq 'normal' -and (-not \$testedExists -or \$testedHash -cne \$stagedHash)) { throw 'normal restoration did not retain the exact tested daemon' }
Write-VolumeCache D -ErrorAction Stop
\"RESTORED|mode=$mode|active=\$(Test-Path -LiteralPath \$active)|tested=\$(Test-Path -LiteralPath \$tested)|tested_sha=\$testedHash|restored_sha=\$restoredHash\"
") || { note "Windows runtime restoration failed: $out"; return 1; }
    WIN_SWAP_ATTEMPTED=0
    WIN_RESTORE_RECORD=$(printf '%s\n' "$out" | tr -d '\r' | tail -1)
    printf '%s\n' "$WIN_RESTORE_RECORD"
}

q_responder_for() {
    local direction=$1 initiator=$2
    [[ "$direction:$initiator" == 'q_to_windows:destination_init' \
        || "$direction:$initiator" == 'windows_to_q:source_init' ]]
}

responder_module_path() {
    local direction=$1 fixture=$2 initiator=$3
    if [[ "$initiator" == source_init ]]; then
        printf '%s/%s\n' "$(destination_root "$direction")" "$SESSION_TAG"
    else
        dirname "$(fixture_source "$direction" "$fixture")"
    fi
}

responder_read_only() {
    [[ "$1" == destination_init ]] && printf '%s\n' true || printf '%s\n' false
}

write_daemon_config_q() {
    local path=$1 module_path=$2 read_only=$3
    exclusive_line "$path" '[daemon]'
    append_line "$path" 'bind = "0.0.0.0"'
    append_line "$path" "port = $DAEMON_PORT"
    append_line "$path" 'no_mdns = true'
    append_line "$path" ''
    append_line "$path" '[[module]]'
    append_line "$path" 'name = "ldt4"'
    append_line "$path" "path = \"$module_path\""
    append_line "$path" "read_only = $read_only"
}

start_q_daemon() {
    local run_id=$1 module_path=$2 read_only=$3
    local arm_dir="$OUT_DIR/endpoint/q/$run_id" config stdout stderr command
    config="$arm_dir/daemon.toml"
    stdout="$arm_dir/daemon.out"
    stderr="$arm_dir/daemon.err"
    assert_q_registered_path "$arm_dir" directory false \
        || session_void "$run_id q daemon evidence directory is unsafe"
    assert_q_registered_path "$module_path" directory false \
        || session_void "$run_id q responder module path is unsafe"
    assert_q_registered_path "$Q_DAEMON" file false \
        || session_void "$run_id q daemon executable path is unsafe"
    write_daemon_config_q "$config" "$module_path" "$read_only"
    [[ ! -e "$stdout" && ! -L "$stdout" && ! -e "$stderr" && ! -L "$stderr" ]] \
        || session_void "$run_id q daemon logs already exist"
    BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id" \
        nohup "$Q_DAEMON" --config "$config" >"$stdout" 2>"$stderr" &
    CURRENT_Q_DAEMON_PID=$!
    exclusive_line "$arm_dir/daemon.pid" "$CURRENT_Q_DAEMON_PID"
    sleep 1
    kill -0 "$CURRENT_Q_DAEMON_PID" 2>/dev/null \
        || session_void "$run_id q daemon exited during startup"
    command=$(ps -p "$CURRENT_Q_DAEMON_PID" -o command=)
    [[ "$command" == "$Q_DAEMON --config $config" ]] \
        || session_void "$run_id q daemon identity mismatch: $command"
    nc -z -w 3 "$Q_IP" "$DAEMON_PORT" \
        || session_void "$run_id q daemon did not listen"
}

start_windows_daemon() {
    local run_id=$1 module_path=$2 read_only=$3 out remote_dir guard
    remote_dir="$WIN_SESSION_ROOT/$SESSION_TAG/logs/$run_id"
    guard=$(windows_path_guard_script)
    out=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
\$dir = '$remote_dir'
Assert-Ldt4PlainPath \$dir Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_ACTIVE_DAEMON' File | Out-Null
Assert-Ldt4PlainPath '$module_path' Directory | Out-Null
if ((Get-FileHash -Algorithm SHA256 -LiteralPath '$WIN_ACTIVE_DAEMON').Hash.ToLower() -cne '$WIN_STAGED_DAEMON_SHA') { throw 'active daemon hash changed before arm' }
\$config = \$dir + '/daemon.toml'
Assert-Ldt4PlainPath \$config File \$true | Out-Null
\$text = @('[daemon]','bind = \"0.0.0.0\"','port = $DAEMON_PORT','no_mdns = true','','[[module]]','name = \"ldt4\"','path = \"$module_path\"','read_only = $read_only') -join \"\`n\"
\$text += \"\`n\"
\$stream = [IO.File]::Open(\$config,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try { \$bytes=[Text.UTF8Encoding]::new(\$false).GetBytes(\$text); \$stream.Write(\$bytes,0,\$bytes.Length); \$stream.Flush(\$true) } finally { \$stream.Dispose() }
\$start = \$dir + '/start.cmd'
Assert-Ldt4PlainPath \$start File \$true | Out-Null
foreach (\$log in @(\$dir + '/daemon.out',\$dir + '/daemon.err')) {
  Assert-Ldt4PlainPath \$log File \$true | Out-Null
  \$logStream = [IO.File]::Open(\$log,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  \$logStream.Dispose()
}
\$startText = @('@echo off','set /a BLIT_LAUNCH_WAIT=0',':wait_for_launch_ok','if exist \"' + \$dir + '/launch.ok\" goto launch_ready','set /a BLIT_LAUNCH_WAIT+=1','if %BLIT_LAUNCH_WAIT% GEQ 15 exit /b 111','>nul 2>&1 ping -n 2 127.0.0.1','goto wait_for_launch_ok',':launch_ready','set BLIT_TRACE_SESSION_PHASES=1','set BLIT_TRACE_RUN_ID=$run_id','\"$WIN_ACTIVE_DAEMON\" --config \"' + \$config + '\" >> \"' + \$dir + '/daemon.out\" 2>> \"' + \$dir + '/daemon.err\"') -join \"\`r\`n\"
\$startStream = [IO.File]::Open(\$start,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try { \$bytes=[Text.Encoding]::ASCII.GetBytes(\$startText + \"\`r\`n\"); \$startStream.Write(\$bytes,0,\$bytes.Length); \$startStream.Flush(\$true) } finally { \$startStream.Dispose() }
\$launcherCommand = 'cmd.exe /d /c \"\"' + \$start + '\"\"'
\$expectedLauncher = ConvertTo-Ldt4CommandLine \$launcherCommand
\$expectedDaemon = ConvertTo-Ldt4CommandLine ('$WIN_ACTIVE_DAEMON --config ' + \$config)
\$created = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{CommandLine=\$launcherCommand}
if (\$created.ReturnValue -ne 0) { throw \"launcher create rc=\$(\$created.ReturnValue)\" }
\$launcherPid = [int]\$created.ProcessId
\$launcherPidPath = \$dir + '/launcher.pid'
\$pidStream = [IO.File]::Open(\$launcherPidPath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try { \$bytes=[Text.Encoding]::ASCII.GetBytes([string]\$launcherPid); \$pidStream.Write(\$bytes,0,\$bytes.Length); \$pidStream.Flush(\$true) } finally { \$pidStream.Dispose() }
\$okStream = [IO.File]::Open((\$dir + '/launch.ok'),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
\$okStream.Dispose()
Start-Sleep -Seconds 2
\$launcher = Get-CimInstance Win32_Process -Filter \"ProcessId=\$launcherPid\" -ErrorAction SilentlyContinue
\$actualLauncher = if (\$launcher) { ConvertTo-Ldt4CommandLine \$launcher.CommandLine } else { '' }
if (-not \$launcher -or \$launcher.Name -ine 'cmd.exe' -or \$actualLauncher -cne \$expectedLauncher) { throw \"launcher identity mismatch: \$actualLauncher\" }
\$children = @(Get-CimInstance Win32_Process -Filter \"ParentProcessId=\$launcherPid\" -ErrorAction Stop)
if (\$children.Count -ne 1) { throw \"daemon child count=\$(\$children.Count)\" }
\$daemon = \$children[0]
\$actual = if (\$daemon.ExecutablePath) { \$daemon.ExecutablePath.Replace([char]92,[char]47) } else { '' }
\$actualCommand = ConvertTo-Ldt4CommandLine \$daemon.CommandLine
if (\$daemon.Name -ine 'blit-daemon.exe' -or \$actual -ine '$WIN_ACTIVE_DAEMON' -or \$actualCommand -cne \$expectedDaemon) { throw \"daemon identity mismatch: \$actual \$actualCommand\" }
\$daemonPidPath = \$dir + '/daemon.pid'
\$pidStream = [IO.File]::Open(\$daemonPidPath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try { \$bytes=[Text.Encoding]::ASCII.GetBytes([string]\$daemon.ProcessId); \$pidStream.Write(\$bytes,0,\$bytes.Length); \$pidStream.Flush(\$true) } finally { \$pidStream.Dispose() }
\$identityPath = \$dir + '/daemon-identity.txt'
\$identityText = \"run_id=$run_id\`nlauncher_pid=\$launcherPid\`ndaemon_pid=\$(\$daemon.ProcessId)\`nlauncher_command=\$expectedLauncher\`ndaemon_command=\$expectedDaemon\`nconfig=\$((ConvertTo-Ldt4CanonicalPath \$config).Replace([char]92,[char]47))\`n\"
\$identityStream = [IO.File]::Open(\$identityPath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try { \$bytes=[Text.UTF8Encoding]::new(\$false).GetBytes(\$identityText); \$identityStream.Write(\$bytes,0,\$bytes.Length); \$identityStream.Flush(\$true) } finally { \$identityStream.Dispose() }
\"P|\$launcherPid|\$(\$daemon.ProcessId)\"
") || session_void "$run_id Windows daemon startup failed: $out"
    out=$(printf '%s\n' "$out" | tr -d '\r' | tail -1)
    IFS='|' read -r _ CURRENT_WIN_LAUNCHER_PID CURRENT_WIN_DAEMON_PID <<<"$out"
    [[ "$CURRENT_WIN_LAUNCHER_PID" =~ ^[0-9]+$ && "$CURRENT_WIN_DAEMON_PID" =~ ^[0-9]+$ ]] \
        || session_void "$run_id Windows daemon PID malformed: $out"
    nc -z -w 3 "$WIN_IP" "$DAEMON_PORT" \
        || session_void "$run_id Windows daemon did not listen"
}

stop_q_daemon() {
    local pid=$CURRENT_Q_DAEMON_PID command i expected
    [[ -n "$pid" ]] || return 0
    [[ "$pid" =~ ^[0-9]+$ ]] || { mark_void "invalid q daemon PID: $pid"; return 1; }
    expected="$Q_DAEMON --config $OUT_DIR/endpoint/q/$CURRENT_RUN_ID/daemon.toml"
    if kill -0 "$pid" 2>/dev/null; then
        command=$(ps -p "$pid" -o command=)
        [[ "$command" == "$expected" ]] \
            || { mark_void "refusing to stop unrecognized q PID $pid: $command"; return 1; }
        kill "$pid" || { mark_void "cannot signal q daemon PID $pid"; return 1; }
        for i in 1 2 3 4 5 6 7 8 9 10; do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.25
        done
        if kill -0 "$pid" 2>/dev/null; then
            command=$(ps -p "$pid" -o command=)
            [[ "$command" == "$expected" ]] \
                || { mark_void "q PID $pid changed identity during teardown"; return 1; }
            kill -KILL "$pid" || { mark_void "cannot kill exact q daemon PID $pid"; return 1; }
        fi
    fi
    wait "$pid" 2>/dev/null || true
    CURRENT_Q_DAEMON_PID=''
    q_port_closed || { mark_void 'q daemon port survived exact teardown'; return 1; }
}

stop_windows_daemon() {
    local pid=$CURRENT_WIN_DAEMON_PID launcher=$CURRENT_WIN_LAUNCHER_PID out recovered recovered_launcher recovered_pid guard cleanup_issue=0
    if [[ ( -z "$pid" || -z "$launcher" ) && -n "$CURRENT_RUN_ID" ]]; then
        guard=$(windows_path_guard_script)
        recovered=$(wssh "$guard
\$dir = '$WIN_SESSION_ROOT/$SESSION_TAG/logs/$CURRENT_RUN_ID'
Assert-Ldt4PlainPath \$dir Directory | Out-Null
\$launcherPath = \$dir + '/launcher.pid'
\$daemonPath = \$dir + '/daemon.pid'
\$launcher = if (Test-Path -LiteralPath \$launcherPath) { Assert-Ldt4PlainPath \$launcherPath File | Out-Null; (Get-Content -LiteralPath \$launcherPath -Raw).Trim() } else { '' }
\$daemon = if (Test-Path -LiteralPath \$daemonPath) { Assert-Ldt4PlainPath \$daemonPath File | Out-Null; (Get-Content -LiteralPath \$daemonPath -Raw).Trim() } else { '' }
\"R|\$launcher|\$daemon\"
" 2>/dev/null | tr -d '\r' | tail -1) || return 1
        IFS='|' read -r _ recovered_launcher recovered_pid <<<"$recovered"
        [[ -n "$launcher" ]] || launcher=$recovered_launcher
        [[ -n "$pid" ]] || pid=$recovered_pid
    fi
    [[ -n "$pid" || -n "$launcher" || -n "$CURRENT_RUN_ID" ]] || return 0
    if [[ -n "$pid" && ! "$pid" =~ ^[0-9]+$ ]]; then
        mark_void "invalid Windows daemon PID evidence: $pid"
        cleanup_issue=1
        pid=''
    fi
    if [[ -n "$launcher" && ! "$launcher" =~ ^[0-9]+$ ]]; then
        mark_void "invalid Windows launcher PID evidence: $launcher"
        cleanup_issue=1
        launcher=''
    fi
    pid=${pid:-0}
    launcher=${launcher:-0}
    [[ -n "$guard" ]] || guard=$(windows_path_guard_script)
    out=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
\$persistedDaemonPid = [int]$pid
\$launcher = [int]$launcher
\$dir = '$WIN_SESSION_ROOT/$SESSION_TAG/logs/$CURRENT_RUN_ID'
Assert-Ldt4PlainPath \$dir Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_ACTIVE_DAEMON' File | Out-Null
\$config = ConvertTo-Ldt4CanonicalPath (\$dir + '/daemon.toml')
\$start = ConvertTo-Ldt4CanonicalPath (\$dir + '/start.cmd')
Assert-Ldt4PlainPath \$config File | Out-Null
Assert-Ldt4PlainPath \$start File | Out-Null
\$expectedLauncher = ConvertTo-Ldt4CommandLine ('cmd.exe /d /c \"\"' + \$start + '\"\"')
\$expectedDaemon = ConvertTo-Ldt4CommandLine ('$WIN_ACTIVE_DAEMON --config ' + \$config)
if (\$launcher -le 0) {
  \$launcherMatches = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object {
    \$_.Name -ieq 'cmd.exe' -and (ConvertTo-Ldt4CommandLine \$_.CommandLine) -ceq \$expectedLauncher
  })
  if (\$launcherMatches.Count -gt 1) { throw \"ambiguous exact launcher recovery: \$(\$launcherMatches.ProcessId -join ',')\" }
  if (\$launcherMatches.Count -eq 1) { \$launcher = [int]\$launcherMatches[0].ProcessId }
}
\$d = if (\$persistedDaemonPid -gt 0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$persistedDaemonPid\" -ErrorAction SilentlyContinue } else { \$null }
\$c = if (\$launcher -gt 0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$launcher\" -ErrorAction SilentlyContinue } else { \$null }
if (\$c) {
  \$actualLauncher = ConvertTo-Ldt4CommandLine \$c.CommandLine
  if (\$c.Name -ine 'cmd.exe' -or \$actualLauncher -cne \$expectedLauncher) { throw \"refusing launcher PID identity: \$actualLauncher\" }
  \$ownedChildren = @(Get-CimInstance Win32_Process -Filter \"ParentProcessId=\$launcher\" -ErrorAction Stop | Where-Object {
    \$actualPath = if (\$_.ExecutablePath) { \$_.ExecutablePath.Replace([char]92,[char]47) } else { '' }
    \$_.Name -ieq 'blit-daemon.exe' -and \$actualPath -ieq '$WIN_ACTIVE_DAEMON' -and (ConvertTo-Ldt4CommandLine \$_.CommandLine) -ceq \$expectedDaemon
  })
  if (\$ownedChildren.Count -gt 1) { throw \"ambiguous exact owned daemon children: \$(\$ownedChildren.ProcessId -join ',')\" }
  if (-not \$d -and \$ownedChildren.Count -eq 1) { \$d = \$ownedChildren[0] }
  if (\$d -and \$ownedChildren.Count -eq 1 -and \$ownedChildren[0].ProcessId -ne \$d.ProcessId) { throw 'persisted daemon PID differs from sole launcher child' }
}
if (-not \$d -and \$persistedDaemonPid -le 0 -and \$launcher -gt 0) {
  \$daemonMatches = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object {
    \$actualPath = if (\$_.ExecutablePath) { \$_.ExecutablePath.Replace([char]92,[char]47) } else { '' }
    \$_.ParentProcessId -eq \$launcher -and \$_.Name -ieq 'blit-daemon.exe' -and \$actualPath -ieq '$WIN_ACTIVE_DAEMON' -and (ConvertTo-Ldt4CommandLine \$_.CommandLine) -ceq \$expectedDaemon
  })
  if (\$daemonMatches.Count -gt 1) { throw \"ambiguous exact daemon recovery: \$(\$daemonMatches.ProcessId -join ',')\" }
  if (\$daemonMatches.Count -eq 1) { \$d = \$daemonMatches[0] }
}
if (\$d) {
  \$actual = if (\$d.ExecutablePath) { \$d.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  \$actualCommand = ConvertTo-Ldt4CommandLine \$d.CommandLine
  if (\$actual -ine '$WIN_ACTIVE_DAEMON' -or \$d.Name -ine 'blit-daemon.exe' -or \$d.ParentProcessId -ne \$launcher -or \$actualCommand -cne \$expectedDaemon) { throw \"refusing daemon PID identity: \$actual \$actualCommand parent=\$(\$d.ParentProcessId)\" }
}
\$stoppedDaemonPid = if (\$d) { [int]\$d.ProcessId } else { 0 }
if (\$d) {
  Stop-Process -Id \$stoppedDaemonPid -Force -ErrorAction Stop
}
if (\$c -and (Get-Process -Id \$launcher -ErrorAction SilentlyContinue)) { Stop-Process -Id \$launcher -Force -ErrorAction Stop }
Start-Sleep -Milliseconds 300
\$late = if (\$launcher -gt 0) { @(Get-CimInstance Win32_Process -Filter \"ParentProcessId=\$launcher\" -ErrorAction Stop) } else { @() }
if (\$late.Count -gt 1) { throw \"ambiguous late process children: \$(\$late.ProcessId -join ',')\" }
foreach (\$child in \$late) {
  \$actualLate = if (\$child.ExecutablePath) { \$child.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  \$lateCommand = ConvertTo-Ldt4CommandLine \$child.CommandLine
  if (\$child.Name -ine 'blit-daemon.exe' -or \$actualLate -ine '$WIN_ACTIVE_DAEMON' -or \$lateCommand -cne \$expectedDaemon) { throw \"late child identity mismatch: \$actualLate \$lateCommand\" }
  Stop-Process -Id \$child.ProcessId -Force -ErrorAction Stop
}
if (\$late.Count) { Start-Sleep -Milliseconds 300 }
if (\$stoppedDaemonPid -gt 0 -and (Get-Process -Id \$stoppedDaemonPid -ErrorAction SilentlyContinue)) { throw 'daemon survived exact teardown' }
if (\$launcher -gt 0 -and (Get-Process -Id \$launcher -ErrorAction SilentlyContinue)) { throw 'launcher survived exact teardown' }
if (\$launcher -gt 0 -and (@(Get-CimInstance Win32_Process -Filter \"ParentProcessId=\$launcher\" -ErrorAction Stop)).Count -gt 0) { throw 'late child survived exact teardown' }
if (Get-NetTCPConnection -State Listen -LocalPort $DAEMON_PORT -ErrorAction SilentlyContinue) { throw 'daemon port survived teardown' }
'STOPPED'
") || { mark_void "Windows exact daemon teardown failed: $out"; return 1; }
    CURRENT_WIN_DAEMON_PID=''
    CURRENT_WIN_LAUNCHER_PID=''
    return "$cleanup_issue"
}

normalize_q_client_pid_list() {
    awk 'NF == 1 && $1 ~ /^[0-9]+$/ { print $1 }'
}

stop_q_client() {
    local run_id=$CURRENT_Q_CLIENT_RUN_ID pid=$CURRENT_Q_CLIENT_PID command environment pid_path identity_path i candidate matches match_count cleanup_issue=0
    [[ -n "$run_id" ]] || return 0
    pid_path="$OUT_DIR/endpoint/q/$run_id/client.pid"
    identity_path="$OUT_DIR/endpoint/q/$run_id/client-identity.json"
    if [[ -z "$pid" && -f "$pid_path" && ! -L "$pid_path" ]]; then
        pid=$(tr -d '\r\n' < "$pid_path")
    fi
    [[ -n "$CURRENT_Q_CLIENT_COMMAND" ]] \
        || { mark_void "$run_id q client expected command is absent"; return 1; }
    if [[ -n "$pid" && ! "$pid" =~ ^[0-9]+$ ]]; then
        mark_void "$run_id invalid q client PID evidence: $pid"
        cleanup_issue=1
        pid=''
    fi
    if [[ -z "$pid" ]]; then
        matches=''
        for candidate in $(ps -axo pid= | normalize_q_client_pid_list); do
            [[ "$candidate" =~ ^[0-9]+$ ]] || continue
            command=$(ps -ww -p "$candidate" -o command= 2>/dev/null || true)
            [[ "$command" == "$CURRENT_Q_CLIENT_COMMAND" ]] || continue
            environment=$(ps eww -p "$candidate" -o command= 2>/dev/null || true)
            [[ "$environment" == *"BLIT_TRACE_RUN_ID=$run_id"* \
                && "$environment" == *'BLIT_TRACE_SESSION_PHASES=1'* ]] || continue
            matches="${matches}${matches:+$'\n'}$candidate"
        done
        match_count=$(printf '%s\n' "$matches" | awk 'NF {count++} END {print count+0}')
        [[ "$match_count" -le 1 ]] \
            || { mark_void "$run_id ambiguous exact q client recovery: ${matches//$'\n'/,}"; return 1; }
        [[ "$match_count" -eq 0 ]] || pid=$matches
    fi
    [[ -n "$pid" ]] || {
        CURRENT_Q_CLIENT_RUN_ID=''
        CURRENT_Q_CLIENT_COMMAND=''
        return "$cleanup_issue"
    }
    if [[ -f "$identity_path" && ! -L "$identity_path" ]]; then
        python3 - "$identity_path" "$pid" "$run_id" "$CURRENT_Q_CLIENT_COMMAND" <<'PY' \
            || { mark_void "$run_id q client identity evidence is inconsistent"; return 1; }
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    identity = json.load(handle)
if (identity.get("pid") != int(sys.argv[2])
        or identity.get("run_id") != sys.argv[3]
        or identity.get("trace_run_id") != sys.argv[3]
        or identity.get("trace_session_phases") != "1"
        or identity.get("command") != sys.argv[4]):
    raise SystemExit(1)
PY
    fi
    if kill -0 "$pid" 2>/dev/null; then
        command=$(ps -ww -p "$pid" -o command=)
        environment=$(ps eww -p "$pid" -o command=)
        [[ "$command" == "$CURRENT_Q_CLIENT_COMMAND" \
            && "$environment" == *"BLIT_TRACE_RUN_ID=$run_id"* \
            && "$environment" == *'BLIT_TRACE_SESSION_PHASES=1'* ]] \
            || { mark_void "refusing to stop unrecognized q client PID $pid"; return 1; }
        kill "$pid" || { mark_void "cannot signal exact q client PID $pid"; return 1; }
        for i in 1 2 3 4 5 6 7 8 9 10; do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.25
        done
        if kill -0 "$pid" 2>/dev/null; then
            command=$(ps -ww -p "$pid" -o command=)
            environment=$(ps eww -p "$pid" -o command=)
            [[ "$command" == "$CURRENT_Q_CLIENT_COMMAND" \
                && "$environment" == *"BLIT_TRACE_RUN_ID=$run_id"* \
                && "$environment" == *'BLIT_TRACE_SESSION_PHASES=1'* ]] \
                || { mark_void "$run_id q client PID changed identity during teardown"; return 1; }
            kill -KILL "$pid" || { mark_void "cannot kill exact q client PID $pid"; return 1; }
        fi
        for i in 1 2 3 4 5 6 7 8 9 10; do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.1
        done
        kill -0 "$pid" 2>/dev/null \
            && { mark_void "$run_id q client survived exact teardown"; return 1; }
    fi
    CURRENT_Q_CLIENT_PID=''
    CURRENT_Q_CLIENT_RUN_ID=''
    CURRENT_Q_CLIENT_COMMAND=''
    return "$cleanup_issue"
}

stop_windows_client() {
    local run_id=$CURRENT_WIN_CLIENT_RUN_ID pid=$CURRENT_WIN_CLIENT_PID controller=$CURRENT_WIN_CLIENT_CONTROLLER_PID
    local remote_dir recovered recovered_controller recovered_pid out guard cleanup_issue=0
    [[ -n "$run_id" ]] || return 0
    remote_dir="$WIN_SESSION_ROOT/$SESSION_TAG/logs/$run_id"
    guard=$(windows_path_guard_script)
    recovered=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
\$dir = '$remote_dir'
Assert-Ldt4PlainPath \$dir Directory | Out-Null
\$controllerPath = \$dir + '/client-controller.pid'
\$clientPath = \$dir + '/client.pid'
\$controller = if (Test-Path -LiteralPath \$controllerPath) { Assert-Ldt4PlainPath \$controllerPath File | Out-Null; (Get-Content -LiteralPath \$controllerPath -Raw).Trim() } else { '' }
\$client = if (Test-Path -LiteralPath \$clientPath) { Assert-Ldt4PlainPath \$clientPath File | Out-Null; (Get-Content -LiteralPath \$clientPath -Raw).Trim() } else { '' }
\"R|\$controller|\$client\"
" 2>/dev/null | tr -d '\r' | tail -1) || return 1
    IFS='|' read -r _ recovered_controller recovered_pid <<<"$recovered"
    [[ -n "$controller" ]] || controller=$recovered_controller
    [[ -n "$pid" ]] || pid=$recovered_pid
    if [[ -n "$controller" && ! "$controller" =~ ^[0-9]+$ ]]; then
        mark_void "$run_id invalid Windows client controller PID evidence: $controller"
        cleanup_issue=1
        controller=''
    fi
    if [[ -n "$pid" && ! "$pid" =~ ^[0-9]+$ ]]; then
        mark_void "$run_id invalid Windows client PID evidence: $pid"
        cleanup_issue=1
        pid=''
    fi
    controller=${controller:-0}
    pid=${pid:-0}
    out=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
\$dir = '$remote_dir'
Assert-Ldt4PlainPath \$dir Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_BLIT' File | Out-Null
\$controllerPid = [int]$controller
\$clientPid = [int]$pid
\$controllerScript = ConvertTo-Ldt4CanonicalPath (\$dir + '/client-controller.ps1')
Assert-Ldt4PlainPath \$controllerScript File \$true | Out-Null
if (-not (Test-Path -LiteralPath \$controllerScript)) {
  if (\$controllerPid -gt 0 -or \$clientPid -gt 0) { throw 'client PID evidence exists without controller script' }
  'STOPPED'
  return
}
Assert-Ldt4PlainPath \$controllerScript File | Out-Null
\$pwshPath = (Get-Process -Id \$PID -ErrorAction Stop).Path
Assert-Ldt4PlainPath \$pwshPath File | Out-Null
\$expectedController = ConvertTo-Ldt4CommandLine ('\"' + \$pwshPath + '\" -NoLogo -NoProfile -NonInteractive -File \"' + \$controllerScript + '\"')
\$expectedClient = ConvertTo-Ldt4CommandLine '$CURRENT_WIN_CLIENT_COMMAND'
if (\$controllerPid -le 0) {
  \$controllerMatches = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object {
    \$actualPath = if (\$_.ExecutablePath) { \$_.ExecutablePath.Replace([char]92,[char]47) } else { '' }
    \$actualPath -ieq \$pwshPath.Replace([char]92,[char]47) -and (ConvertTo-Ldt4CommandLine \$_.CommandLine) -ceq \$expectedController
  })
  if (\$controllerMatches.Count -gt 1) { throw \"ambiguous exact client-controller recovery: \$(\$controllerMatches.ProcessId -join ',')\" }
  if (\$controllerMatches.Count -eq 1) { \$controllerPid = [int]\$controllerMatches[0].ProcessId }
}
\$controllerProcess = if (\$controllerPid -gt 0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$controllerPid\" -ErrorAction SilentlyContinue } else { \$null }
if (\$controllerProcess) {
  \$actualControllerPath = if (\$controllerProcess.ExecutablePath) { \$controllerProcess.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  if (\$actualControllerPath -ine \$pwshPath.Replace([char]92,[char]47) -or (ConvertTo-Ldt4CommandLine \$controllerProcess.CommandLine) -cne \$expectedController) { throw 'refusing client controller identity' }
}
\$clientProcess = if (\$clientPid -gt 0) { Get-CimInstance Win32_Process -Filter \"ProcessId=\$clientPid\" -ErrorAction SilentlyContinue } else { \$null }
if (-not \$clientProcess -and \$controllerPid -gt 0) {
  \$clientMatches = @(Get-CimInstance Win32_Process -Filter \"ParentProcessId=\$controllerPid\" -ErrorAction Stop | Where-Object {
    \$actualPath = if (\$_.ExecutablePath) { \$_.ExecutablePath.Replace([char]92,[char]47) } else { '' }
    \$_.Name -ieq 'blit.exe' -and \$actualPath -ieq '$WIN_BLIT' -and (ConvertTo-Ldt4CommandLine \$_.CommandLine) -ceq \$expectedClient
  })
  if (\$clientMatches.Count -gt 1) { throw \"ambiguous exact Windows client recovery: \$(\$clientMatches.ProcessId -join ',')\" }
  if (\$clientMatches.Count -eq 1) { \$clientProcess = \$clientMatches[0]; \$clientPid = [int]\$clientProcess.ProcessId }
}
if (\$clientProcess) {
  \$actualClientPath = if (\$clientProcess.ExecutablePath) { \$clientProcess.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  \$actualClientCommand = ConvertTo-Ldt4CommandLine \$clientProcess.CommandLine
  if (\$clientProcess.Name -ine 'blit.exe' -or \$actualClientPath -ine '$WIN_BLIT' -or \$actualClientCommand -cne \$expectedClient -or \$clientProcess.ParentProcessId -ne \$controllerPid) { throw \"refusing Windows client identity: \$actualClientPath \$actualClientCommand\" }
  Stop-Process -Id \$clientPid -Force -ErrorAction Stop
}
if (\$controllerProcess -and (Get-Process -Id \$controllerPid -ErrorAction SilentlyContinue)) {
  \$freshController = Get-CimInstance Win32_Process -Filter \"ProcessId=\$controllerPid\" -ErrorAction Stop
  if ((ConvertTo-Ldt4CommandLine \$freshController.CommandLine) -cne \$expectedController) { throw 'client controller identity changed during teardown' }
  Stop-Process -Id \$controllerPid -Force -ErrorAction Stop
}
Start-Sleep -Milliseconds 300
if (\$clientPid -gt 0 -and (Get-Process -Id \$clientPid -ErrorAction SilentlyContinue)) { throw 'Windows client survived exact teardown' }
if (\$controllerPid -gt 0 -and (Get-Process -Id \$controllerPid -ErrorAction SilentlyContinue)) { throw 'Windows client controller survived exact teardown' }
'STOPPED'
") || { mark_void "$run_id Windows exact client teardown failed: $out"; return 1; }
    CURRENT_WIN_CLIENT_PID=''
    CURRENT_WIN_CLIENT_CONTROLLER_PID=''
    CURRENT_WIN_CLIENT_RUN_ID=''
    CURRENT_WIN_CLIENT_COMMAND=''
    return "$cleanup_issue"
}

stop_current_client() {
    local rc=0
    stop_q_client || rc=1
    stop_windows_client || rc=1
    return "$rc"
}

stop_current_daemon() {
    case "$CURRENT_DAEMON_ENDPOINT" in
        q) stop_q_daemon || return 1 ;;
        windows) stop_windows_daemon || return 1 ;;
        '') return 0 ;;
        *)
            mark_void "invalid current daemon endpoint: $CURRENT_DAEMON_ENDPOINT"
            return 1
            ;;
    esac
    CURRENT_DAEMON_ENDPOINT=''
}

prepare_arm_dirs() {
    local run_id=$1 remote_dir="$WIN_SESSION_ROOT/$SESSION_TAG/logs/$run_id" guard
    assert_q_registered_path "$OUT_DIR/endpoint/q" directory false \
        || session_void "$run_id q evidence parent is unsafe"
    assert_q_registered_path "$OUT_DIR/endpoint/windows" directory false \
        || session_void "$run_id Windows evidence parent is unsafe"
    assert_q_registered_path "$OUT_DIR/endpoint/q/$run_id" directory true \
        || session_void "$run_id q evidence target ancestry is unsafe"
    assert_q_registered_path "$OUT_DIR/endpoint/windows/$run_id" directory true \
        || session_void "$run_id Windows evidence target ancestry is unsafe"
    [[ ! -e "$OUT_DIR/endpoint/q/$run_id" && ! -L "$OUT_DIR/endpoint/q/$run_id" ]] \
        || session_void "$run_id q evidence directory already exists"
    [[ ! -e "$OUT_DIR/endpoint/windows/$run_id" && ! -L "$OUT_DIR/endpoint/windows/$run_id" ]] \
        || session_void "$run_id Windows evidence directory already exists"
    mkdir "$OUT_DIR/endpoint/q/$run_id" "$OUT_DIR/endpoint/windows/$run_id" \
        || session_void "$run_id cannot reserve local arm evidence"
    assert_q_registered_path "$OUT_DIR/endpoint/q/$run_id" directory false \
        || session_void "$run_id reserved q evidence directory is unsafe"
    assert_q_registered_path "$OUT_DIR/endpoint/windows/$run_id" directory false \
        || session_void "$run_id reserved Windows evidence directory is unsafe"
    guard=$(windows_path_guard_script)
    wssh "$guard
\$ErrorActionPreference = 'Stop'
Assert-Ldt4PlainPath '$WIN_SESSION_ROOT/$SESSION_TAG/logs' Directory | Out-Null
Assert-Ldt4PlainPath '$remote_dir' Directory \$true | Out-Null
if (Test-Path -LiteralPath '$remote_dir') { throw 'remote arm evidence already exists' }
New-Item -ItemType Directory -Path '$remote_dir' -ErrorAction Stop | Out-Null
Assert-Ldt4PlainPath '$remote_dir' Directory | Out-Null
" >/dev/null || session_void "$run_id cannot reserve Windows arm evidence"
}

prepare_active_destination() {
    local direction=$1 fixture=$2 active guard
    active=$(active_destination "$direction" "$fixture")
    if [[ "$direction" == windows_to_q ]]; then
        assert_q_registered_path "$(dirname "$active")" directory false \
            || session_void "q active destination parent is unsafe: $active"
        assert_q_registered_path "$active" directory true \
            || session_void "q active destination ancestry is unsafe: $active"
        [[ ! -e "$active" && ! -L "$active" ]] \
            || session_void "active q destination already exists: $active"
        mkdir "$active" || session_void "cannot create q active destination: $active"
        assert_q_registered_path "$active" directory false \
            || session_void "created q active destination is unsafe: $active"
    else
        guard=$(windows_path_guard_script)
        wssh "$guard
\$ErrorActionPreference = 'Stop'
Assert-Ldt4PlainPath (Split-Path -Parent (ConvertTo-Ldt4CanonicalPath '$active')) Directory | Out-Null
Assert-Ldt4PlainPath '$active' Directory \$true | Out-Null
if (Test-Path -LiteralPath '$active') { throw 'active Windows destination already exists' }
New-Item -ItemType Directory -Path '$active' -ErrorAction Stop | Out-Null
Assert-Ldt4PlainPath '$active' Directory | Out-Null
" >/dev/null || session_void "cannot create Windows active destination: $active"
    fi
}

remote_source_argument() {
    local direction=$1 fixture=$2
    case "$direction" in
        q_to_windows) printf '%s:%s:/ldt4/src_%s\n' "$Q_IP" "$DAEMON_PORT" "$fixture" ;;
        windows_to_q) printf '%s:%s:/ldt4/src_%s\n' "$WIN_IP" "$DAEMON_PORT" "$fixture" ;;
        *) die "unregistered direction $direction" ;;
    esac
}

remote_destination_argument() {
    local direction=$1 fixture=$2
    case "$direction" in
        q_to_windows) printf '%s:%s:/ldt4/active/%s/\n' "$WIN_IP" "$DAEMON_PORT" "$fixture" ;;
        windows_to_q) printf '%s:%s:/ldt4/active/%s/\n' "$Q_IP" "$DAEMON_PORT" "$fixture" ;;
        *) die "unregistered direction $direction" ;;
    esac
}

client_source_argument() {
    local direction=$1 fixture=$2 initiator=$3
    case "$initiator" in
        source_init) fixture_source "$direction" "$fixture" ;;
        destination_init) remote_source_argument "$direction" "$fixture" ;;
        *) die "unregistered initiator $initiator" ;;
    esac
}

client_destination_argument() {
    local direction=$1 fixture=$2 initiator=$3
    case "$initiator" in
        source_init) remote_destination_argument "$direction" "$fixture" ;;
        destination_init) printf '%s/\n' "$(active_destination "$direction" "$fixture")" ;;
        *) die "unregistered initiator $initiator" ;;
    esac
}

run_q_client() {
    local run_id=$1 stdout=$2 stderr=$3 out pid_path identity_path exit_path arg command
    shift 3
    pid_path="$OUT_DIR/endpoint/q/$run_id/client.pid"
    identity_path="$OUT_DIR/endpoint/q/$run_id/client-identity.json"
    exit_path="$OUT_DIR/endpoint/q/$run_id/client-exit.txt"
    [[ ! -e "$stdout" && ! -L "$stdout" && ! -e "$stderr" && ! -L "$stderr" ]] \
        || session_void "$run_id q client logs already exist"
    assert_q_registered_path "$Q_BLIT" file false \
        || session_void "$run_id q client executable path is unsafe"
    assert_q_registered_path "$(dirname "$stdout")" directory false \
        || session_void "$run_id q client evidence path is unsafe"
    command=$Q_BLIT
    for arg in "$@"; do
        [[ "$arg" != *[$'\n\r\t ']* ]] \
            || session_void "$run_id q client argument cannot be represented exactly"
        command="$command $arg"
    done
    CURRENT_Q_CLIENT_RUN_ID=$run_id
    CURRENT_Q_CLIENT_COMMAND=$command
    CURRENT_Q_CLIENT_PID=''
    out=$(BLIT_TRACE_SESSION_PHASES=1 BLIT_TRACE_RUN_ID="$run_id" \
        python3 - "$stdout" "$stderr" "$pid_path" "$identity_path" "$exit_path" \
            "$run_id" "$Q_BLIT" "$@" <<'PY'
import json
import os
import pathlib
import signal
import subprocess
import sys
import time

stdout_path, stderr_path, pid_path, identity_path, exit_path, run_id = sys.argv[1:7]
argv = sys.argv[7:]
process = None
old_handlers = {}

def stop_owned(signum=None, frame=None):
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
    if signum is not None:
        raise SystemExit(128 + signum)

for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
    old_handlers[signum] = signal.signal(signum, stop_owned)
try:
    start = time.monotonic_ns()
    with open(stdout_path, "xb") as stdout, open(stderr_path, "xb") as stderr:
        process = subprocess.Popen(argv, stdout=stdout, stderr=stderr, env=os.environ.copy())
        with pathlib.Path(pid_path).open("x", encoding="ascii") as handle:
            handle.write(f"{process.pid}\n")
            handle.flush()
            os.fsync(handle.fileno())
        identity = {
            "argv": argv,
            "command": " ".join(argv),
            "executable": os.path.realpath(argv[0]),
            "pid": process.pid,
            "run_id": run_id,
            "trace_run_id": os.environ.get("BLIT_TRACE_RUN_ID"),
            "trace_session_phases": os.environ.get("BLIT_TRACE_SESSION_PHASES"),
        }
        if identity["trace_run_id"] != run_id or identity["trace_session_phases"] != "1":
            raise RuntimeError("client trace-session identity is not exact")
        if identity["executable"] != argv[0]:
            raise RuntimeError("client executable resolved away from registered path")
        with pathlib.Path(identity_path).open("x", encoding="utf-8") as handle:
            json.dump(identity, handle, sort_keys=True, separators=(",", ":"))
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        returncode = process.wait()
    elapsed_ms = max(1, round((time.monotonic_ns() - start) / 1_000_000))
    if process.poll() is None:
        raise RuntimeError("client was not reaped after normal exit")
    with pathlib.Path(exit_path).open("x", encoding="ascii") as handle:
        handle.write(f"pid={process.pid} rc={returncode} reaped=true\n")
        handle.flush()
        os.fsync(handle.fileno())
    print(f"R|{elapsed_ms}|{returncode}")
finally:
    stop_owned()
    for signum, handler in old_handlers.items():
        signal.signal(signum, handler)
PY
) || return 1
    [[ -f "$pid_path" && ! -L "$pid_path" ]] || return 1
    CURRENT_Q_CLIENT_PID=$(tr -d '\r\n' < "$pid_path")
    [[ "$CURRENT_Q_CLIENT_PID" =~ ^[0-9]+$ ]] || return 1
    CLIENT_RESULT=$(printf '%s\n' "$out" | tail -1)
    [[ "$CLIENT_RESULT" == R\|* ]] || return 1
    [[ -f "$exit_path" && ! -L "$exit_path" ]] || return 1
    CURRENT_Q_CLIENT_PID=''
    CURRENT_Q_CLIENT_RUN_ID=''
    CURRENT_Q_CLIENT_COMMAND=''
}

windows_client_controller_base64() {
    python3 - "$1" "$2" "$3" "$4" "$5" <<'PY'
import base64
import sys

remote_dir, executable, source, destination, run_id = sys.argv[1:]
values = [remote_dir, executable, source, destination, run_id]
if any("'" in value or "\r" in value or "\n" in value for value in values):
    raise SystemExit("unsafe Windows client controller value")
template = r'''$ErrorActionPreference = 'Stop'
function Normalize-Ldt4CommandLine([string]$value) {
  if ($null -eq $value) { return '' }
  return (($value.Replace([char]92,[char]47).Replace([string][char]34,'') -replace '\s+',' ').Trim()).ToLowerInvariant()
}
$dir = '__REMOTE_DIR__'
$process = $null
$stdoutStream = $null
$stderrStream = $null
try {
  $gate = $dir + '/client-launch.ok'
  for ($wait = 0; $wait -lt 150 -and -not (Test-Path -LiteralPath $gate); $wait++) { Start-Sleep -Milliseconds 100 }
  if (-not (Test-Path -LiteralPath $gate)) { throw 'client launch gate timed out' }
  $stdoutStream = [IO.File]::Open(($dir + '/client.out'),[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  $stderrStream = [IO.File]::Open(($dir + '/client.err'),[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  $info = [Diagnostics.ProcessStartInfo]::new()
  $info.FileName = '__EXECUTABLE__'
  $info.UseShellExecute = $false
  $info.RedirectStandardOutput = $true
  $info.RedirectStandardError = $true
  foreach ($argument in @('copy','__SOURCE__','__DESTINATION__','--yes')) { [void]$info.ArgumentList.Add($argument) }
  $info.Environment['BLIT_TRACE_SESSION_PHASES'] = '1'
  $info.Environment['BLIT_TRACE_RUN_ID'] = '__RUN_ID__'
  $process = [Diagnostics.Process]::new()
  $process.StartInfo = $info
  $clock = [Diagnostics.Stopwatch]::StartNew()
  if (-not $process.Start()) { throw 'client Process.Start returned false' }
  $pidStream = [IO.File]::Open(($dir + '/client.pid'),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try { $bytes=[Text.Encoding]::ASCII.GetBytes([string]$process.Id); $pidStream.Write($bytes,0,$bytes.Length); $pidStream.Flush($true) } finally { $pidStream.Dispose() }
  $observed = Get-CimInstance Win32_Process -Filter "ProcessId=$($process.Id)" -ErrorAction Stop
  $actualPath = if ($observed.ExecutablePath) { $observed.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  $actualCommand = Normalize-Ldt4CommandLine $observed.CommandLine
  $expectedCommand = Normalize-Ldt4CommandLine '__EXECUTABLE__ copy __SOURCE__ __DESTINATION__ --yes'
  if ($observed.Name -ine 'blit.exe' -or $actualPath -ine '__EXECUTABLE__' -or $actualCommand -cne $expectedCommand) { throw "client identity mismatch: $actualPath $actualCommand" }
  $identity = "run_id=__RUN_ID__`npid=$($process.Id)`nparent_pid=$PID`nexecutable=$actualPath`ncommand=$actualCommand`ntrace_session_phases=1`ntrace_run_id=__RUN_ID__`n"
  $identityStream = [IO.File]::Open(($dir + '/client-identity.txt'),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try { $bytes=[Text.UTF8Encoding]::new($false).GetBytes($identity); $identityStream.Write($bytes,0,$bytes.Length); $identityStream.Flush($true) } finally { $identityStream.Dispose() }
  $stdoutCopy = $process.StandardOutput.BaseStream.CopyToAsync($stdoutStream)
  $stderrCopy = $process.StandardError.BaseStream.CopyToAsync($stderrStream)
  $process.WaitForExit()
  $stdoutCopy.GetAwaiter().GetResult()
  $stderrCopy.GetAwaiter().GetResult()
  $clock.Stop()
  $elapsed = [Math]::Max([int64]1,[int64][Math]::Round($clock.Elapsed.TotalMilliseconds))
  $resultStream = [IO.File]::Open(($dir + '/client-result.txt'),[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  try { $bytes=[Text.Encoding]::ASCII.GetBytes("R|$elapsed|$($process.ExitCode)|$($process.Id)|reaped`n"); $resultStream.Write($bytes,0,$bytes.Length); $resultStream.Flush($true) } finally { $resultStream.Dispose() }
  exit 0
} catch {
  if ($process -and -not $process.HasExited) { $process.Kill($true); $process.WaitForExit() }
  $safe = ([string]$_).Replace("`r",' ').Replace("`n",' ')
  $resultStream = [IO.File]::Open(($dir + '/client-result.txt'),[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  try { $bytes=[Text.UTF8Encoding]::new($false).GetBytes("E|$safe`n"); $resultStream.Write($bytes,0,$bytes.Length); $resultStream.Flush($true) } finally { $resultStream.Dispose() }
  exit 1
} finally {
  if ($stdoutStream) { $stdoutStream.Dispose() }
  if ($stderrStream) { $stderrStream.Dispose() }
  if ($process) { $process.Dispose() }
}
'''
for token, value in {
    "__REMOTE_DIR__": remote_dir,
    "__EXECUTABLE__": executable,
    "__SOURCE__": source,
    "__DESTINATION__": destination,
    "__RUN_ID__": run_id,
}.items():
    template = template.replace(token, value)
sys.stdout.write(base64.b64encode(template.encode("utf-8")).decode("ascii"))
PY
}

run_windows_client() {
    local run_id=$1 source=$2 destination=$3 out remote_dir guard controller_b64 envelope_tag result_tag duration rc client_pid reaped extra
    remote_dir="$WIN_SESSION_ROOT/$SESSION_TAG/logs/$run_id"
    CURRENT_WIN_CLIENT_RUN_ID=$run_id
    CURRENT_WIN_CLIENT_COMMAND="$WIN_BLIT copy $source $destination --yes"
    CURRENT_WIN_CLIENT_PID=''
    CURRENT_WIN_CLIENT_CONTROLLER_PID=''
    guard=$(windows_path_guard_script)
    controller_b64=$(windows_client_controller_base64 "$remote_dir" "$WIN_BLIT" "$source" "$destination" "$run_id") \
        || return 1
    out=$(wssh "$guard
\$ErrorActionPreference = 'Stop'
\$dir = '$remote_dir'
Assert-Ldt4PlainPath \$dir Directory | Out-Null
Assert-Ldt4PlainPath '$WIN_BLIT' File | Out-Null
\$controller = \$dir + '/client-controller.ps1'
\$controllerPidPath = \$dir + '/client-controller.pid'
\$clientPidPath = \$dir + '/client.pid'
\$identityPath = \$dir + '/client-identity.txt'
\$resultPath = \$dir + '/client-result.txt'
\$stdoutPath = \$dir + '/client.out'
\$stderrPath = \$dir + '/client.err'
foreach (\$path in @(\$controller,\$controllerPidPath,\$clientPidPath,\$identityPath,\$resultPath,\$stdoutPath,\$stderrPath,\$dir + '/client-launch.ok')) {
  Assert-Ldt4PlainPath \$path File \$true | Out-Null
}
foreach (\$path in @(\$resultPath,\$stdoutPath,\$stderrPath)) {
  \$exclusive = [IO.File]::Open(\$path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  \$exclusive.Dispose()
}
\$controllerStream = [IO.File]::Open(\$controller,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try { \$bytes=[Convert]::FromBase64String('$controller_b64'); \$controllerStream.Write(\$bytes,0,\$bytes.Length); \$controllerStream.Flush(\$true) } finally { \$controllerStream.Dispose() }
\$pwshPath = (Get-Process -Id \$PID -ErrorAction Stop).Path
Assert-Ldt4PlainPath \$pwshPath File | Out-Null
\$controllerCommand = '\"' + \$pwshPath + '\" -NoLogo -NoProfile -NonInteractive -File \"' + \$controller + '\"'
\$expectedController = ConvertTo-Ldt4CommandLine \$controllerCommand
\$created = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{CommandLine=\$controllerCommand}
if (\$created.ReturnValue -ne 0) { throw \"client controller create rc=\$(\$created.ReturnValue)\" }
\$controllerPid = [int]\$created.ProcessId
\$pidStream = [IO.File]::Open(\$controllerPidPath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try { \$bytes=[Text.Encoding]::ASCII.GetBytes([string]\$controllerPid); \$pidStream.Write(\$bytes,0,\$bytes.Length); \$pidStream.Flush(\$true) } finally { \$pidStream.Dispose() }
\$controllerProcess = Get-CimInstance Win32_Process -Filter \"ProcessId=\$controllerPid\" -ErrorAction Stop
\$actualControllerPath = if (\$controllerProcess.ExecutablePath) { \$controllerProcess.ExecutablePath.Replace([char]92,[char]47) } else { '' }
if (\$actualControllerPath -ine \$pwshPath.Replace([char]92,[char]47) -or (ConvertTo-Ldt4CommandLine \$controllerProcess.CommandLine) -cne \$expectedController) { throw 'client controller identity mismatch' }
\$ok = [IO.File]::Open((\$dir + '/client-launch.ok'),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
\$ok.Dispose()
while (Get-Process -Id \$controllerPid -ErrorAction SilentlyContinue) { Start-Sleep -Milliseconds 200 }
\$result = (Get-Content -LiteralPath \$resultPath -Raw -ErrorAction Stop).Trim()
\"C|\$controllerPid|\$result\"
") || return 1
    out=$(printf '%s\n' "$out" | tr -d '\r' | tail -1)
    IFS='|' read -r envelope_tag CURRENT_WIN_CLIENT_CONTROLLER_PID result_tag duration rc client_pid reaped extra <<<"$out"
    [[ "$envelope_tag" == C && "$result_tag" == R \
        && "$CURRENT_WIN_CLIENT_CONTROLLER_PID" =~ ^[0-9]+$ \
        && "$duration" =~ ^[1-9][0-9]*$ && "$rc" =~ ^-?[0-9]+$ \
        && "$client_pid" =~ ^[0-9]+$ && "$reaped" == reaped && -z "$extra" ]] \
        || return 1
    CURRENT_WIN_CLIENT_PID=$client_pid
    CLIENT_RESULT="R|$duration|$rc"
    CURRENT_WIN_CLIENT_PID=''
    CURRENT_WIN_CLIENT_CONTROLLER_PID=''
    CURRENT_WIN_CLIENT_RUN_ID=''
    CURRENT_WIN_CLIENT_COMMAND=''
}

run_client() {
    local direction=$1 fixture=$2 initiator=$3 run_id=$4
    local q_dir="$OUT_DIR/endpoint/q/$run_id" source destination
    source=$(client_source_argument "$direction" "$fixture" "$initiator")
    destination=$(client_destination_argument "$direction" "$fixture" "$initiator")
    CLIENT_RESULT=''
    case "$direction:$initiator" in
        q_to_windows:source_init)
            run_q_client "$run_id" "$q_dir/client.out" "$q_dir/client.err" \
                copy "$source" "$destination" --yes
            ;;
        q_to_windows:destination_init)
            run_windows_client "$run_id" "$source" "$destination"
            ;;
        windows_to_q:source_init)
            run_windows_client "$run_id" "$source" "$destination"
            ;;
        windows_to_q:destination_init)
            run_q_client "$run_id" "$q_dir/client.out" "$q_dir/client.err" \
                copy "$source" "$destination" --yes
            ;;
        *) session_void "$run_id unregistered client layout $direction:$initiator" ;;
    esac
    [[ -n "$CLIENT_RESULT" ]] || return 1
}

collect_windows_component() {
    local run_id=$1 component=$2 remote_dir local_dir evidence
    remote_dir="$WIN_SESSION_ROOT/$SESSION_TAG/logs/$run_id"
    local_dir="$OUT_DIR/endpoint/windows/$run_id"
    fetch_windows_file "$remote_dir/$component.out" "$local_dir/$component.out" \
        || session_void "$run_id cannot fetch Windows $component stdout"
    fetch_windows_file "$remote_dir/$component.err" "$local_dir/$component.err" \
        || session_void "$run_id cannot fetch Windows $component stderr"
    case "$component" in
        daemon)
            for evidence in daemon.toml start.cmd launcher.pid daemon.pid daemon-identity.txt; do
                fetch_windows_file "$remote_dir/$evidence" "$local_dir/$evidence" \
                    || session_void "$run_id cannot fetch Windows daemon evidence $evidence"
            done
            ;;
        client)
            for evidence in client-controller.ps1 client-controller.pid client.pid \
                client-identity.txt client-result.txt; do
                fetch_windows_file "$remote_dir/$evidence" "$local_dir/$evidence" \
                    || session_void "$run_id cannot fetch Windows client evidence $evidence"
            done
            ;;
        *) session_void "$run_id unregistered Windows component evidence: $component" ;;
    esac
}

flush_q_destination() {
    python3 - "$1" <<'PY'
import os, pathlib, stat, sys
root = pathlib.Path(sys.argv[1])
if not root.is_dir() or root.is_symlink():
    raise SystemExit("unsafe q destination")
directories = []
def walk_error(error):
    raise error
for current, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
    directories.append(pathlib.Path(current))
    for name in dirs:
        path = pathlib.Path(current, name)
        if path.is_symlink() or not stat.S_ISDIR(path.lstat().st_mode):
            raise SystemExit(f"unsafe directory {path}")
    for name in files:
        path = pathlib.Path(current, name)
        if not stat.S_ISREG(path.lstat().st_mode):
            raise SystemExit(f"unsafe file {path}")
        fd = os.open(path, os.O_RDONLY)
        try: os.fsync(fd)
        finally: os.close(fd)
for path in reversed(directories):
    fd = os.open(path, os.O_RDONLY)
    try: os.fsync(fd)
    finally: os.close(fd)
PY
}

flush_windows_destination() {
    wssh "Write-VolumeCache D -ErrorAction Stop" >/dev/null
}

rename_q_directory_exclusive() {
    python3 - "$1" "$2" <<'PY'
import ctypes
import os
import pathlib
import sys

source = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
if not source.is_dir() or source.is_symlink() or destination.exists() or destination.is_symlink():
    raise SystemExit("exclusive q retention precondition failed")
libc = ctypes.CDLL(None, use_errno=True)
renameatx_np = libc.renameatx_np
renameatx_np.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
renameatx_np.restype = ctypes.c_int
AT_FDCWD = -2
RENAME_EXCL = 0x00000004
result = renameatx_np(
    AT_FDCWD, os.fsencode(source), AT_FDCWD, os.fsencode(destination), RENAME_EXCL
)
if result != 0:
    error = ctypes.get_errno()
    raise OSError(error, os.strerror(error), str(destination))
PY
}

verify_and_retain_destination() {
    local direction=$1 fixture=$2 run_id=$3 active archive source_rel landed_rel landed_abs remote_landed guard
    active=$(active_destination "$direction" "$fixture")
    archive=$(retained_destination "$direction" "$run_id")
    source_rel="manifests/source/${direction}_${fixture}.csv"
    landed_rel="manifests/landed/${run_id}.csv"
    landed_abs="$OUT_DIR/$landed_rel"
    if [[ "$direction" == windows_to_q ]]; then
        assert_q_registered_path "$active" directory false \
            || session_void "$run_id q active destination ancestry changed"
        assert_q_registered_path "$archive" directory true \
            || session_void "$run_id q retained destination ancestry is unsafe"
        flush_q_destination "$active" || session_void "$run_id q durability flush failed"
        write_q_manifest "$active" "$landed_abs" "src_$fixture" \
            || session_void "$run_id q landed manifest failed"
        cmp -s "$OUT_DIR/$source_rel" "$landed_abs" \
            || session_void "$run_id q landed content differs from source"
        [[ ! -e "$archive" && ! -L "$archive" ]] \
            || session_void "$run_id q retained path already exists"
        rename_q_directory_exclusive "$active" "$archive" \
            || session_void "$run_id q atomic exclusive retention rename failed"
        assert_q_registered_path "$archive" directory false \
            || session_void "$run_id q retained destination is unsafe"
        [[ ! -e "$active" && ! -L "$active" && -d "$archive" && ! -L "$archive" ]] \
            || session_void "$run_id q retention state mismatch"
        sync || session_void "$run_id q retention metadata flush failed"
    else
        flush_windows_destination || session_void "$run_id Windows durability flush failed"
        remote_landed="$WIN_SESSION_ROOT/$SESSION_TAG/manifests/landed-${run_id}.csv"
        write_windows_manifest "$active" "$remote_landed" "src_$fixture" >/dev/null \
            || session_void "$run_id Windows landed manifest failed"
        fetch_windows_file "$remote_landed" "$landed_abs" \
            || session_void "$run_id Windows landed manifest fetch failed"
        cmp -s "$OUT_DIR/$source_rel" "$landed_abs" \
            || session_void "$run_id Windows landed content differs from source"
        guard=$(windows_path_guard_script)
        wssh "$guard
\$ErrorActionPreference = 'Stop'
Assert-Ldt4PlainPath '$active' Directory | Out-Null
Assert-Ldt4PlainPath '$archive' Directory \$true | Out-Null
if (Test-Path -LiteralPath '$archive') { throw 'retained path already exists' }
[IO.Directory]::Move((ConvertTo-Ldt4CanonicalPath '$active'),(ConvertTo-Ldt4CanonicalPath '$archive'))
Write-VolumeCache D -ErrorAction Stop
if (Test-Path -LiteralPath '$active') { throw 'active destination survived retention rename' }
Assert-Ldt4PlainPath '$archive' Directory | Out-Null
" >/dev/null || session_void "$run_id Windows atomic retention rename failed"
    fi
    printf '%s|%s\n' "$landed_rel" "$archive"
}

extract_session_id() {
    local run_id=$1 source_log=$2 destination_log=$3
    python3 - "$run_id" "$source_log" "$destination_log" <<'PY'
import json, pathlib, re, sys
run_id = sys.argv[1]
ids = set()
for name in sys.argv[2:]:
    path = pathlib.Path(name)
    events = []
    for line in path.read_text(encoding="utf-8", errors="strict").splitlines():
        if line.startswith("[session-phase] "):
            event = json.loads(line[len("[session-phase] "):])
            if event.get("run_id") != run_id:
                raise SystemExit(f"foreign run id in {path}: {event.get('run_id')}")
            ids.add(event.get("session_id"))
            events.append(event)
    if not events:
        raise SystemExit(f"no trace events in {path}")
if len(ids) != 1:
    raise SystemExit(f"expected one session id, got {ids}")
session_id = next(iter(ids))
if not isinstance(session_id, str) or not re.fullmatch(r"[0-9a-f]{16}", session_id):
    raise SystemExit(f"malformed session id: {session_id!r}")
print(session_id)
PY
}

runtime_boundary_gate() {
    local sequence=$1 cell=$2 pair=$3 q_quiet win_quiet q_free win_free
    assert_q_registered_paths "$cell pair $pair runtime boundary"
    assert_windows_registered_paths "$cell pair $pair runtime boundary"
    ports_closed || session_void "$cell pair $pair: daemon port occupied at runtime boundary"
    q_quiet=$(q_quiet_gate)
    win_quiet=$(windows_quiet_gate)
    q_free=$(df -Pk "$Q_SESSION_ROOT" | awk 'NR==2 {printf "%.0f", $4 * 1024}')
    win_free=$(wssh "(Get-PSDrive D -ErrorAction Stop).Free" | tr -d '\r' | tail -1)
    [[ "$win_free" =~ ^[0-9]+$ ]] || session_void "$cell pair $pair: Windows free bytes malformed"
    awk -v free="$q_free" -v need="$MIN_FREE_BYTES" 'BEGIN {exit !(free >= need)}' \
        || session_void "$cell pair $pair: q free bytes $q_free below $MIN_FREE_BYTES"
    awk -v free="$win_free" -v need="$MIN_FREE_BYTES" 'BEGIN {exit !(free >= need)}' \
        || session_void "$cell pair $pair: Windows free bytes $win_free below $MIN_FREE_BYTES"
    append_line "$OUT_DIR/runtime-gates.csv" \
        "$sequence,$cell,$pair,$q_free,$win_free,${q_quiet// /;},${win_quiet// /;}"
}

run_arm() {
    local sequence=$1 cell=$2 direction=$3 fixture=$4 initiator=$5 pair=$6
    local run_id result tag duration rc extra module_path read_only windows_component
    local source_trace_rel destination_trace_rel source_trace_abs destination_trace_abs
    local session_id retention landed_rel archive files bytes source_path active source_manifest
    printf -v run_id 'ldt4-%03d' "$sequence"
    CURRENT_RUN_ID=$run_id
    prepare_arm_dirs "$run_id"
    prepare_active_destination "$direction" "$fixture"
    module_path=$(responder_module_path "$direction" "$fixture" "$initiator")
    read_only=$(responder_read_only "$initiator")
    if q_responder_for "$direction" "$initiator"; then
        CURRENT_DAEMON_ENDPOINT=q
        start_q_daemon "$run_id" "$module_path" "$read_only"
        windows_component=client
    else
        CURRENT_DAEMON_ENDPOINT=windows
        start_windows_daemon "$run_id" "$module_path" "$read_only"
        windows_component=daemon
    fi
    run_client "$direction" "$fixture" "$initiator" "$run_id" \
        || session_void "$run_id client wrapper failed"
    result=$CLIENT_RESULT
    sleep 0.25
    stop_current_client || session_void "$run_id client survived owned execution"
    stop_current_daemon || session_void "$run_id responder teardown failed"
    collect_windows_component "$run_id" "$windows_component"
    IFS='|' read -r tag duration rc extra <<<"$result"
    [[ "$tag" == R && "$duration" =~ ^[1-9][0-9]*$ && "$rc" =~ ^[0-9]+$ && -z "$extra" ]] \
        || session_void "$run_id client result malformed: $result"
    [[ "$rc" == 0 ]] || session_void "$run_id client failed rc=$rc"

    case "$direction:$initiator" in
        q_to_windows:source_init)
            source_trace_rel="endpoint/q/$run_id/client.err"
            destination_trace_rel="endpoint/windows/$run_id/daemon.err"
            ;;
        q_to_windows:destination_init)
            source_trace_rel="endpoint/q/$run_id/daemon.err"
            destination_trace_rel="endpoint/windows/$run_id/client.err"
            ;;
        windows_to_q:source_init)
            source_trace_rel="endpoint/windows/$run_id/client.err"
            destination_trace_rel="endpoint/q/$run_id/daemon.err"
            ;;
        windows_to_q:destination_init)
            source_trace_rel="endpoint/windows/$run_id/daemon.err"
            destination_trace_rel="endpoint/q/$run_id/client.err"
            ;;
        *) session_void "$run_id trace ownership mapping absent" ;;
    esac
    source_trace_abs="$OUT_DIR/$source_trace_rel"
    destination_trace_abs="$OUT_DIR/$destination_trace_rel"
    session_id=$(extract_session_id "$run_id" "$source_trace_abs" "$destination_trace_abs") \
        || session_void "$run_id trace correlation failed"
    retention=$(verify_and_retain_destination "$direction" "$fixture" "$run_id") \
        || session_void "$run_id integrity/retention failed"
    IFS='|' read -r landed_rel archive extra <<<"$retention"
    [[ -n "$landed_rel" && -n "$archive" && -z "$extra" ]] \
        || session_void "$run_id retention result malformed"
    IFS=',' read -r files bytes <<<"$(expected_shape "$fixture")"
    source_path=$(fixture_source "$direction" "$fixture")
    active=$(active_destination "$direction" "$fixture")
    source_manifest="manifests/source/${direction}_${fixture}.csv"
    append_line "$RUNS_CSV" \
        "$cell,$direction,$fixture,$pair,$initiator,$run_id,$session_id,$duration,$files,$bytes,$source_path,$active,$archive,$source_manifest,$landed_rel,$source_trace_rel,$destination_trace_rel,0,yes"
    note "$run_id complete: $cell pair=$pair $initiator duration=${duration}ms session=$session_id retained=$archive"
    CURRENT_RUN_ID=''
}

write_measurements_complete() {
    local marker="$OUT_DIR/MEASUREMENTS-COMPLETE"
    [[ ! -e "$marker" && ! -L "$marker" ]] || session_void 'measurement marker already exists'
    (set -o noclobber; printf 'artifact_sha=%s\nharness_sha=%s\narm_count=96\n' \
        "$ARTIFACT_SHA" "$EXPECTED_HARNESS_SHA" > "$marker") \
        || session_void 'cannot create exclusive measurement marker'
}

write_final_evidence_inventory() {
    local inventory="$OUT_DIR/FINAL-SHA256.csv"
    assert_q_registered_path "$OUT_DIR" directory false \
        || session_void 'final inventory evidence root is unsafe'
    assert_q_registered_path "$inventory" file true \
        || session_void 'final inventory output ancestry is unsafe'
    python3 - "$OUT_DIR" "$inventory" <<'PY'
import base64
import hashlib
import os
import pathlib
import stat
import sys

root = pathlib.Path(sys.argv[1])
output = pathlib.Path(sys.argv[2])
if not root.is_absolute() or ".." in root.parts or output.parent != root:
    raise SystemExit("final inventory paths are not canonical")
if output.exists() or output.is_symlink():
    raise SystemExit("final inventory already exists")

current = pathlib.Path(root.anchor)
for part in root.parts[1:]:
    current = current / part
    info = current.lstat()
    if stat.S_ISLNK(info.st_mode) or not stat.S_ISDIR(info.st_mode):
        raise SystemExit(f"unsafe final inventory ancestor: {current}")

rows = []
def walk_error(error):
    raise error
for current_text, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
    current_path = pathlib.Path(current_text)
    current_info = current_path.lstat()
    if stat.S_ISLNK(current_info.st_mode) or not stat.S_ISDIR(current_info.st_mode):
        raise SystemExit(f"unsafe evidence directory: {current_path}")
    dirs.sort()
    files.sort()
    for name in dirs:
        path = current_path / name
        info = path.lstat()
        if stat.S_ISLNK(info.st_mode) or not stat.S_ISDIR(info.st_mode):
            raise SystemExit(f"non-plain evidence directory: {path}")
    for name in files:
        path = current_path / name
        if path == output:
            continue
        before = path.lstat()
        if stat.S_ISLNK(before.st_mode) or not stat.S_ISREG(before.st_mode):
            raise SystemExit(f"non-plain evidence file: {path}")
        flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags)
        try:
            opened = os.fstat(descriptor)
            if (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino):
                raise SystemExit(f"evidence file changed identity: {path}")
            digest = hashlib.sha256()
            with os.fdopen(descriptor, "rb", closefd=False) as handle:
                for block in iter(lambda: handle.read(8 * 1024 * 1024), b""):
                    digest.update(block)
            after = os.fstat(descriptor)
            if ((opened.st_dev, opened.st_ino, opened.st_size, opened.st_mtime_ns)
                    != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)):
                raise SystemExit(f"evidence file changed during inventory: {path}")
        finally:
            os.close(descriptor)
        relative = path.relative_to(root).as_posix()
        encoded = base64.b64encode(relative.encode("utf-8")).decode("ascii")
        rows.append((relative, encoded, opened.st_size, digest.hexdigest()))
rows.sort(key=lambda row: row[0])
with output.open("x", encoding="ascii", newline="") as handle:
    handle.write("path_base64,size,sha256\n")
    for _, encoded, size, digest in rows:
        handle.write(f"{encoded},{size},{digest}\n")
    handle.flush()
    os.fsync(handle.fileno())
PY
}

run_registered_matrix() {
    local schedule="$OUT_DIR/schedule.csv" sequence cell direction fixture initiator pair row_count
    [[ ! -e "$schedule" && ! -L "$schedule" ]] || session_void 'schedule evidence already exists'
    (set -o noclobber; emit_schedule > "$schedule") || session_void 'cannot write registered schedule'
    exclusive_line "$OUT_DIR/runtime-gates.csv" \
        'sequence,cell,pair,q_free_bytes,windows_free_bytes,q_quiet,windows_quiet'
    while IFS=',' read -r sequence cell direction fixture initiator; do
        pair=$(( (10#$sequence - 1) / 2 % 8 + 1 ))
        if (( 10#$sequence % 2 == 1 )); then
            runtime_boundary_gate "$sequence" "$cell" "$pair"
        fi
        run_arm "$((10#$sequence))" "$cell" "$direction" "$fixture" "$initiator" "$pair"
    done < "$schedule"
    row_count=$(awk 'END {print NR - 1}' "$RUNS_CSV")
    [[ "$row_count" -eq "$ARM_COUNT" ]] \
        || session_void "runs.csv has $row_count valid rows, expected $ARM_COUNT"
}

on_exit() {
    local rc=$?
    trap - EXIT INT TERM
    set +e
    if ! stop_current_client; then rc=1; fi
    if ! stop_current_daemon; then rc=1; fi
    if ! restore_windows_runtime >/dev/null; then
        mark_void 'Windows active daemon state could not be restored non-destructively'
        rc=1
    fi
    if [[ "$SESSION_STARTED" == 1 && "$SESSION_COMPLETE" != 1 ]]; then
        mark_void "harness exited before complete acceptance (rc=$rc)"
        [[ "$rc" -ne 0 ]] || rc=1
    fi
    exit "$rc"
}

run_selftest() {
    local old_tag=$SESSION_TAG sample direction fixture source parent remote_host guard_text
    SESSION_TAG='selftest-ldt4'
    assert_schedule
    [[ "$(fixture_source q_to_windows large)" == '/Users/michael/blit-ldt4-staging/fixtures/src_large' ]] \
        || die 'q large fixture mapping selftest failed'
    [[ "$(fixture_source q_to_windows mixed)" == '/Users/michael/blit-ldt4-staging/fixtures/src_mixed' ]] \
        || die 'q mixed fixture mapping selftest failed'
    [[ "$(fixture_source windows_to_q small)" == 'D:/blit-test/ldt4-staging/fixtures/src_small' ]] \
        || die 'Windows small fixture mapping selftest failed'
    [[ "$(active_destination q_to_windows mixed)" == 'D:/blit-test/ldt4-sessions/selftest-ldt4/active/mixed' ]] \
        || die 'Windows active destination mapping selftest failed'
    sample=$(retained_destination windows_to_q ldt4-096)
    [[ "$sample" == '/Users/michael/blit-ldt4-sessions/selftest-ldt4/retained/ldt4-096' ]] \
        || die 'q retained destination mapping selftest failed'
    for direction in q_to_windows windows_to_q; do
        case "$direction" in
            q_to_windows) remote_host=$Q_IP ;;
            windows_to_q) remote_host=$WIN_IP ;;
        esac
        for fixture in large small mixed; do
            source=$(fixture_source "$direction" "$fixture")
            parent=$(dirname "$source")
            [[ "$(responder_module_path "$direction" "$fixture" destination_init)" == "$parent" ]] \
                || die "$direction/$fixture source-responder module mapping selftest failed"
            [[ "$(responder_module_path "$direction" "$fixture" source_init)" == "$(destination_root "$direction")/selftest-ldt4" ]] \
                || die "$direction/$fixture destination-responder module mapping selftest failed"
            [[ "$(client_source_argument "$direction" "$fixture" source_init)" == "$source" ]] \
                || die "$direction/$fixture local-source argument selftest failed"
            [[ "$(client_source_argument "$direction" "$fixture" destination_init)" == "$remote_host:$DAEMON_PORT:/ldt4/src_$fixture" ]] \
                || die "$direction/$fixture remote-source argument selftest failed"
            [[ "$(client_destination_argument "$direction" "$fixture" source_init)" == "$(remote_destination_argument "$direction" "$fixture")" ]] \
                || die "$direction/$fixture remote-destination argument selftest failed"
            [[ "$(client_destination_argument "$direction" "$fixture" destination_init)" == "$(active_destination "$direction" "$fixture")/" ]] \
                || die "$direction/$fixture local-destination argument selftest failed"
        done
    done
    q_responder_for q_to_windows destination_init \
        || die 'q responder ownership mapping selftest failed for q-to-Windows'
    q_responder_for windows_to_q source_init \
        || die 'q responder ownership mapping selftest failed for Windows-to-q'
    if q_responder_for q_to_windows source_init; then
        die 'Windows responder ownership mapping selftest failed for q-to-Windows'
    fi
    if q_responder_for windows_to_q destination_init; then
        die 'Windows responder ownership mapping selftest failed for Windows-to-q'
    fi
    local stopped=''
    stop_q_daemon() { stopped="${stopped}q"; }
    stop_windows_daemon() { stopped="${stopped}w"; }
    CURRENT_DAEMON_ENDPOINT=q
    stop_current_daemon || die 'q daemon ownership dispatch returned failure'
    [[ "$stopped" == q && -z "$CURRENT_DAEMON_ENDPOINT" ]] \
        || die 'q daemon ownership dispatch selftest failed'
    CURRENT_DAEMON_ENDPOINT=windows
    stop_current_daemon || die 'Windows daemon ownership dispatch returned failure'
    [[ "$stopped" == qw && -z "$CURRENT_DAEMON_ENDPOINT" ]] \
        || die 'Windows daemon ownership dispatch selftest failed'
    stop_current_daemon || die 'empty daemon ownership dispatch returned failure'
    [[ "$stopped" == qw ]] || die 'empty daemon ownership dispatch selftest failed'
    declare -F run_client start_q_daemon start_windows_daemon extract_session_id \
        restore_windows_runtime run_registered_matrix assert_q_registered_path \
        assert_windows_registered_paths stop_current_client \
        write_final_evidence_inventory >/dev/null \
        || die 'registered harness function inventory selftest failed'
    guard_text=$(windows_path_guard_script)
    [[ "$guard_text" == *'function Assert-Ldt4PlainPath'* \
        && "$guard_text" == *'ReparsePoint'* \
        && "$guard_text" == *'function ConvertTo-Ldt4CommandLine'* ]] \
        || die 'Windows path/command guard selftest failed'
    LC_ALL=C grep -Fq 'Get-Process cargo,rustc,blit,blit-daemon' "$SCRIPT_PATH" \
        || die 'Windows quiet process inventory selftest failed'
    LC_ALL=C grep -Fq "[version]'7.4'" "$SCRIPT_PATH" \
        || die 'PowerShell version floor selftest failed'
    LC_ALL=C grep -Fq 'windows_powershell=$win_ps' "$SCRIPT_PATH" \
        || die 'PowerShell evidence selftest failed'
    LC_ALL=C grep -Fq "FINAL-SHA256.csv" "$SCRIPT_PATH" \
        || die 'final inventory selftest failed'
    LC_ALL=C grep -Fq '[IO.FileMode]::Append' "$SCRIPT_PATH" \
        || die 'exclusive Windows log append selftest failed'
    LC_ALL=C grep -Fq 'reaped=true' "$SCRIPT_PATH" \
        || die 'client reap evidence selftest failed'
    LC_ALL=C grep -Fq 'ambiguous exact launcher recovery' "$SCRIPT_PATH" \
        || die 'exact Windows launcher recovery selftest failed'
    sample=$(printf 'profile banner\r\nLDT4-FILE-B64|aGVsbG8=\r\n' \
        | decode_windows_file_payload) \
        || die 'tagged Windows fetch decode selftest failed'
    [[ "$sample" == hello ]] || die 'tagged Windows fetch payload changed'
    if printf 'profile banner\r\n' | decode_windows_file_payload >/dev/null 2>&1; then
        die 'missing Windows fetch payload tag passed selftest'
    fi
    if printf 'LDT4-FILE-B64|aGVsbG8=\nLDT4-FILE-B64|d29ybGQ=\n' \
        | decode_windows_file_payload >/dev/null 2>&1; then
        die 'duplicate Windows fetch payload tags passed selftest'
    fi
    if printf '%s\n' 'LDT4-FILE-B64|%%%' \
        | decode_windows_file_payload >/dev/null 2>&1; then
        die 'malformed Windows fetch payload passed selftest'
    fi
    sample=$(printf '    1\n  265\n12345\nnoise\n' | normalize_q_client_pid_list) \
        || die 'q client PID normalization selftest failed'
    [[ "$sample" == $'1\n265\n12345' ]] \
        || die 'q client PID normalization dropped padded low PIDs'
    python3 - "$SCRIPT_PATH" <<'PY' || die 'static harness safety selftest failed'
import pathlib
import re
import sys

text = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
fetch = text[text.index("fetch_windows_file() {"):text.index("manifest_shape() {")]
required_fetch_line = r'''[Console]::Out.Write(\"\`nLDT4-FILE-B64|\" + [Convert]::ToBase64String([IO.File]::ReadAllBytes('$remote')) + \"\`n\")'''
if required_fetch_line not in fetch:
    raise SystemExit("Windows fetch framing is not escaped through Bash")
forbidden = (
    "Stop-Process " + "-Name",
    "task" + "kill /IM",
    "Remove-" + "Item",
    "[IO.File]::" + "Delete",
)
if any(token in text for token in forbidden):
    raise SystemExit("destructive or name-wide process operation appeared")
prepare = text[text.index("prepare_windows_runtime() {"):text.index("restore_windows_runtime() {")]
restore = text[text.index("restore_windows_runtime() {"):text.index("q_responder_for() {")]
q_paths = text[text.index("assert_q_registered_paths() {"):text.index("assert_windows_registered_paths() {")]
windows_paths = text[text.index("assert_windows_registered_paths() {"):text.index("mark_void() {")]
for staged in ('src_large', 'src_small', 'src_mixed'):
    if f'$Q_STAGE_ROOT/fixtures/{staged}' not in q_paths:
        raise SystemExit(f"q {staged} fixture disappeared from boundary path guards")
if '$WIN_FIXTURE_STAGE/fixtures/src_small' not in windows_paths:
    raise SystemExit("Windows small fixture disappeared from boundary path guards")
stage = text[text.index("stage_fixtures() {"):text.index("build_fixture_manifests() {")]
required_stage_copy = 'scp -r "${SSH_MUX[@]}" "$WIN_SSH:$remote_source" "$incoming_root/"'
if required_stage_copy not in stage:
    raise SystemExit("canonical Windows fixture staging disappeared")
copy = stage.index(required_stage_copy)
source_manifest = stage.index('write_windows_manifest "$remote_source" "$remote_manifest"', 0, copy)
validate = stage.index('cmp -s "$q_manifest" "$win_manifest"', copy)
required_promotion = 'rename_q_directory_exclusive "$incoming" "$local_destination"'
if required_promotion not in stage:
    raise SystemExit("canonical fixture promotion is not an exclusive atomic rename")
if 'mv -n "$incoming"' in stage:
    raise SystemExit("canonical fixture promotion fell back to mv")
promote = stage.index(required_promotion, validate)
if not source_manifest < copy < validate < promote or 'incoming-fixtures' not in stage:
    raise SystemExit("canonical fixture copy is not validated before stable-path promotion")
environment = text[text.index("environment_gate() {"):text.index("prepare_windows_runtime() {")]
evidence_start = environment.index('exclusive_line "$OUT_DIR/environment-$phase.txt"')
environment_evidence = environment[evidence_start:environment.index("\n}", evidence_start)]
if environment_evidence.count("$quiet_q") != 1:
    raise SystemExit("environment evidence does not include exactly one canonical q quiet record")
if "time_machine_auto=$auto" in environment_evidence or "time_machine_running=$tm_running" in environment_evidence:
    raise SystemExit("environment evidence duplicates Time Machine fields outside q quiet record")
record_flush = prepare.index(r"\$recordStream.Flush(\$true)")
record_barrier = prepare.index("Write-VolumeCache D", record_flush)
retained_guard = prepare.index("unresolved retained-before daemon")
active_guard = prepare.index(r"if (\$hadPrior -and \$priorHash -ceq \$stagedHash)")
intent_create = prepare.index(r"\$recordStream =", active_guard)
if not retained_guard < intent_create or not active_guard < intent_create:
    raise SystemExit("stale runtime baseline can reach intent creation")
runtime_mutation = min(
    prepare.index("[IO.File]::Move", record_barrier),
    prepare.index(r"\$activeStream =", record_barrier),
)
if not record_flush < record_barrier < runtime_mutation:
    raise SystemExit("durable swap intent is not sealed before runtime mutation")
no_intent = restore.index("if (-not (Test-Path -LiteralPath \\$record))")
no_intent_return = restore.index("return", no_intent)
active_classification = restore.index(r"\$activeExists =", no_intent_return)
if no_intent_return >= active_classification:
    raise SystemExit("missing swap intent can reach active-path mutation")
for required in (
    r"\$recordMatch = [regex]::Match",
    r"\$priorHash -cne \$expectedPrior",
    r"\$originalAlreadyActive = \$hadPrior -and -not \$priorExists",
    "restored daemon differs from durable prior SHA",
    "recovery recreated an originally absent active daemon",
    "normal restoration did not retain the exact tested daemon",
):
    if required not in restore:
        raise SystemExit(f"runtime recovery binding disappeared: {required}")
main = text[text.rindex("\nmain() {\n") + 1:]
analyzer_result = main.index('exclusive_line "$OUT_DIR/analyzer-result.txt"')
inventory = main.index("write_final_evidence_inventory", analyzer_result)
complete = main.index("SESSION_COMPLETE=1", inventory)
if not analyzer_result < inventory < complete:
    raise SystemExit("final evidence ordering changed")
tail = main[inventory + len("write_final_evidence_inventory"):]
if re.search(r'\b(exclusive_line|append_line|write_measurements_complete)\b', tail):
    raise SystemExit("evidence mutation appears after final inventory")
if text.count("q_to_windows_large") < 2:
    raise SystemExit("registered fixed matrix changed")
PY
    SESSION_TAG=$old_tag
    printf 'ldt-4 harness selftest: PASS (%s arms, no SSH)\n' "$ARM_COUNT"
}

validate_invocation() {
    [[ "$SELFTEST" == 0 || "$SELFTEST" == 1 ]] || die 'SELFTEST must be exactly 0 or 1'
    require_full_sha "$EXPECTED_HARNESS_SHA" || die 'EXPECTED_HARNESS_SHA must be full lowercase 40-hex'
    [[ "$EXPECTED_HARNESS_SHA" != "$ARTIFACT_SHA" ]] || die 'artifact and harness SHAs must differ'
    SESSION_TAG="ldt4-$(date -u '+%Y%m%dT%H%M%SZ')-${EXPECTED_HARNESS_SHA:0:12}"
    OUT_DIR="$EVIDENCE_ROOT/$SESSION_TAG"
    require_safe_tag "$SESSION_TAG" || die 'derived SESSION_TAG is unsafe'
    [[ ! -e "$OUT_DIR" && ! -L "$OUT_DIR" ]] || die "OUT_DIR already exists: $OUT_DIR"
    [[ -x "$ANALYZER" ]] || die "analyzer is not executable: $ANALYZER"
    assert_schedule
}

main() {
    local artifact_hashes restore_record analysis_record
    if [[ "$SELFTEST" == 1 ]]; then
        wssh() { die 'selftest attempted an SSH call'; }
        run_selftest
        return
    fi
    [[ $# -eq 0 ]] || die 'the registered harness accepts no positional arguments'
    validate_invocation
    for command in git python3 ssh scp shasum lsof nc awk cmp tar; do
        command -v "$command" >/dev/null || die "required command absent: $command"
    done
    assert_q_registered_paths preflight
    assert_windows_registered_paths preflight
    verify_harness_identity
    artifact_hashes=$(verify_artifacts)
    reserve_evidence
    trap on_exit EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    reserve_endpoint_sessions
    initialize_evidence_files "$artifact_hashes"
    stage_fixtures
    build_fixture_manifests
    environment_gate start
    prepare_windows_runtime
    run_registered_matrix
    stop_current_client || session_void 'client survived registered matrix'
    stop_current_daemon || session_void 'responder survived registered matrix'
    restore_windows_runtime normal >/dev/null \
        || session_void 'Windows active daemon restoration failed after matrix'
    restore_record=$WIN_RESTORE_RECORD
    exclusive_line "$OUT_DIR/windows-runtime-restoration.txt" "$restore_record"
    environment_gate end
    write_measurements_complete
    analysis_record=$(python3 "$ANALYZER" --session-dir "$OUT_DIR" \
        --expected-harness-sha "$EXPECTED_HARNESS_SHA") \
        || session_void 'reviewed analyzer refused the complete measurement set'
    exclusive_line "$OUT_DIR/analyzer-result.txt" "$analysis_record"
    write_final_evidence_inventory \
        || session_void 'cannot write final evidence SHA-256 inventory'
    SESSION_COMPLETE=1
    trap - EXIT INT TERM
    note "$analysis_record"
    note "registered evidence retained at $OUT_DIR"
}

main "$@"
