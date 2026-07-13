#!/usr/bin/env bash
# =============================================================================
# bench_otp12_delegated.sh  —  otp-12c "rig D" delegated-vs-direct parity
# ONE_TRANSFER_PATH slice otp-12, sub-slice 12c; design:
# docs/plan/OTP12_ACCEPTANCE_RUN.md  D1 / D2 / D4 / D5 / D6 / D7.
# =============================================================================
#
# WHAT THIS MEASURES (plan D4, rig D — delegated-vs-direct parity)
# ----------------------------------------------------------------
# For one logical remote<->remote transfer (skippy daemon <-> Windows daemon,
# over 10 GbE) we compare two ways of moving the SAME bytes over the SAME data
# plane to the SAME destination disk. The ONLY difference is who spawns the
# initiator and the trigger/progress relay:
#
#   delegated : Mac runs `blit copy SRC_DAEMON DST_DAEMON --yes`. Remote<->remote
#               is delegated-only (D-2026-07-11-1): this ALWAYS calls DelegatedPull
#               on the DESTINATION daemon, which initiates the one session against
#               the source daemon in the DESTINATION role. The Mac only relays
#               control + progress (no payload through the Mac). Timed ON THE MAC
#               around the CLI (it blocks until the relayed Summary), PLUS the
#               destination's self-timed flush — deliberately INCLUDING the
#               trigger RPC + relay overhead (the honest end-to-end delegation cost).
#   direct    : the DESTINATION host runs the pull itself — `blit copy SRC_DAEMON
#               LOCAL_DIR --yes` (a normal remote->local pull, NOT delegated). Timed
#               on that host (self-timed), PLUS the same flush.
#
# Data plane, destination disk, and flush are identical across arms; only the
# initiator (Mac-relayed daemon vs local CLI) differs. That is the parity axis.
#
# DIRECTIONS / CELLS (plan D5 label grammar, extended to rig D)
#   sw_<carrier>_<fixture> : source = skippy, dest = Windows
#   ws_<carrier>_<fixture> : source = Windows, dest = skippy
# 6 TCP verdict cells (3 fixtures x 2 dirs) + 1 secondary gRPC smoke cell
# (sw_grpc_large). 2 arms x RUNS(4) x (6+1) = 56 timed runs (plan D7).
#
# VERDICT (plan D2): per cell, delegated-parity bar = max(delegated,direct)/min
# <= 1.10. TCP cells are the verdict rows; the grpc cell is computed identically
# and labeled secondary (its cell name carries the carrier). The script COMPUTES
# and WRITES the matrix; it never flips a plan checkbox (checkpoints are owner-only).
#
# ------------------------------------------------------------------------------
# BUILD IDENTITY — READ BEFORE RUNNING (sharp edge; plan: same-build handshake)
# ------------------------------------------------------------------------------
# The verdict is meaningful only if every binary on all three hosts is the SAME
# build. NEW_SHA is computed from `git rev-parse --short HEAD`; the harness refuses
# to run unless `blit --version` on the Mac, skippy AND Windows all embed
# EXPECT_SHA (default = NEW_SHA), and the staged Windows daemon == the launched
# (active) daemon byte-for-byte.
#
#   * At authoring, HEAD = dcbd6ea ("governance refresh: toolkit ...") sits ONE
#     docs/tooling-only commit above f35702a (the sha in the rig-W staging paths).
#     dcbd6ea does NOT touch crates/, so a release build there SHOULD be identical
#     to one at f35702a — but this harness does not assume it.
#   * OPERATOR ACTION: rebuild release binaries at CURRENT HEAD on all three hosts
#     and stage them under the $NEW_SHA-derived paths (…/bins/$NEW_SHA/), OR, if you
#     have independently confirmed the f35702a binaries are byte-identical to HEAD,
#     run with EXPECT_SHA=f35702a (and point SKIPPY_BLIT/…/WIN_BLIT at those paths).
#     Do not silence this gate.
#   * The clean-tree gate ignores docs/ churn but fails on any dirt under crates/
#     or Cargo.{toml,lock} — those affect binary identity; docs do not.
#
# OTHER SHARP EDGES (each guarded below)
#   * Daemon kills are PID-scoped + comm/name-verified — NEVER a blunt `pkill blit`.
#   * Stale-listener refusal on $PORT on both daemon hosts before launch.
#   * ABBA counterbalanced interleave (A,B,B,A,A,B,B,A; A=delegated, B=direct) with
#     the D2 valid-run rule: a run with nonzero exit OR an undrained pre-run window
#     VOIDS its whole pair; the pair reruns at the same slot until RUNS valid pairs
#     exist, capped at 2*RUNS attempts; at the cap the cell is INCOMPLETE.
#   * Cold caches on BOTH data-plane ends every run (skippy drop_caches via sudo -n;
#     Windows standby purge) + drain-gate the destination disk (Windows Get-Counter
#     loop; skippy /proc/diskstats quiet-window loop with a device-regex knob).
#   * Delegation authorization is IP/CIDR, not hostname (production SSRF rule):
#     MAC_HOST / SKIPPY_HOST / WIN_HOST MUST be numeric IPs.
#
# SCOPE: writes fixtures/config/logs locally + on the two rig hosts, drives the
# matrix, emits CSVs + verdicts. Does not commit; does not touch git remotes.
# PREFLIGHT_ONLY=1 runs every static gate and exits before fixtures/daemons.
#
# NOTE: this harness cannot be end-to-end tested from the authoring host (no rig
# access). It follows the rig-W/rig-Z template shapes verbatim where possible;
# treat the first live run as a shakeout and prefer PREFLIGHT_ONLY=1 first.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ------------------------------------------------------------------ config ----
NEW_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
EXPECT_SHA="${EXPECT_SHA:-$NEW_SHA}"          # binary-embed gate; override only with proof

# Mac — initiator of the delegated arm (NOT a data endpoint)
MAC_HOST="${MAC_HOST:?set MAC_HOST to the Mac's 10GbE IP (numeric — used in [delegation] allowlists)}"
MAC_BLIT="${MAC_BLIT:-$REPO_ROOT/target/release/blit}"
MAC_WORK="${MAC_WORK:-$HOME/blit-bench-work}"

# skippy — Linux daemon host (source for sw_*, dest for ws_*)
SKIPPY_SSH="${SKIPPY_SSH:-admin@skippy}"
SKIPPY_HOST="${SKIPPY_HOST:?set SKIPPY_HOST to skippy's 10GbE IP (numeric)}"
SKIPPY_BIN="${SKIPPY_BIN:-/mnt/generic-pool/video/blit-bin}"
SKIPPY_BLIT="${SKIPPY_BLIT:-$SKIPPY_BIN/bins/$NEW_SHA/blit}"
SKIPPY_DAEMON="${SKIPPY_DAEMON:-$SKIPPY_BIN/bins/$NEW_SHA/blit-daemon}"
SKIPPY_MODULE="${SKIPPY_MODULE:-/mnt/generic-pool/video/bench-data}"   # module 'bench' data root
SKIPPY_TEMP="${SKIPPY_TEMP:-/mnt/generic-pool/video/blit-bin}"         # config/log dir (exec-friendly pool)
SKIPPY_DISK_REGEX="${SKIPPY_DISK_REGEX:-^sd[a-z]$|^nvme[0-9]+n1$|^dm-[0-9]+$}"  # /proc/diskstats field-3 match

# Windows — daemon host (dest for sw_*, source for ws_*)
WIN_SSH="${WIN_SSH:-michael@netwatch-01}"
WIN_HOST="${WIN_HOST:-10.1.10.177}"
WIN_DRIVE="${WIN_DRIVE:-D}"
WIN_TEST="${WIN_TEST:-D:\\blit-test}"
WIN_BINS="${WIN_BINS:-$WIN_TEST\\bins\\$NEW_SHA}"
WIN_BLIT="${WIN_BLIT:-$WIN_BINS\\blit.exe}"
NEW_WIN_DAEMON="${NEW_WIN_DAEMON:-$WIN_BINS\\blit-daemon.exe}"
ACTIVE_WIN_DAEMON="${ACTIVE_WIN_DAEMON:-$WIN_TEST\\bins\\active\\blit-daemon.exe}"
WIN_MODULE="${WIN_MODULE:-$WIN_TEST\\bench-module}"

# common
PORT="${PORT:-9031}"
RUNS="${RUNS:-4}"
PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
CELLS="${CELLS:-}"                            # empty = full matrix; else comma-list of cell names
SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12_delegated_$SESSION_TAG}"

# drain gate (2s quiet windows, matching the zoey/win loops)
DRAIN_ITERS="${DRAIN_ITERS:-60}"              # up to 60x2s = 120s
DRAIN_QUIET="${DRAIN_QUIET:-3}"               # consecutive quiet windows
WIN_DRAIN_THRESH="${WIN_DRAIN_THRESH:-1048576}"   # bytes/sec on D: considered idle
SKIPPY_DRAIN_SECTORS="${SKIPPY_DRAIN_SECTORS:-4096}"  # sectors written / 2s considered idle

# ssh multiplexing
MUX_DIR="$(mktemp -d "${TMPDIR:-/tmp}/blit-deleg-mux.XXXXXX")"
SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
         -o ControlMaster=auto -o "ControlPath=$MUX_DIR/%C" -o ControlPersist=180)

mkdir -p "$OUT_DIR" "$OUT_DIR/blit-logs"

# ------------------------------------------------------------------ helpers ---
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
die() { log "FATAL: $*"; exit 1; }
now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
sssh() { ssh "${SSH_MUX[@]}" "$SKIPPY_SSH" "$@"; }
wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }             # remote default shell assumed PowerShell
nocr() { tr -d '\r'; }

want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }

# ---- self-timed durability (destination-OS keyed, never verb keyed) ----------
flush_win_ms() {   # Windows volume flush, self-timed; prints ms or NA
  wssh "\$a=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); try{ Write-VolumeCache -DriveLetter '$WIN_DRIVE' -ErrorAction Stop }catch{ 'F:NA:F'; exit 0 }; \$b=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); \"F:\$(\$b-\$a):F\"" 2>/dev/null \
    | nocr | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p;s/.*F:NA:F.*/NA/p' | head -1
}
sync_skippy_ms() {   # skippy sync bracketed by /proc/uptime in one shell
  sssh "a=\$(awk '{print int(\$1*1000)}' /proc/uptime); sync; b=\$(awk '{print int(\$1*1000)}' /proc/uptime); echo \$((b-a))" 2>/dev/null | nocr | tr -dc '0-9' || echo NA
}

# ---- sha256 + version-embed provenance ---------------------------------------
sha256_local() { local h; h=$(shasum -a 256 "$1" | cut -d' ' -f1) || die "sha256 failed for $1"; [[ ${#h} -eq 64 ]] || die "bad sha256 '$h' for $1"; echo "$h"; }
sha256_skippy() { local h; h=$(sssh "sha256sum '$1' 2>/dev/null | cut -d' ' -f1" | nocr) || die "remote sha256 failed $1"; [[ ${#h} -eq 64 ]] || die "bad remote sha256 '$h' for $1"; echo "$h"; }
sha256_win()   { local h; h=$(wssh "(Get-FileHash -Algorithm SHA256 '$1' -ErrorAction SilentlyContinue).Hash" | nocr | tr 'A-F' 'a-f' | tr -cd '0-9a-f'); [[ ${#h} -eq 64 ]] || die "bad win sha256 '$h' for $1"; echo "$h"; }
# Build identity is checked by grepping the BUILD-ID form "+<sha>" out of the
# binary itself (a compile-time literal; otp-12a-run F1). There is NO usable
# `blit --version` — the CLI rejects the flag — so grep the exe, never run it.
# LC_ALL=C + -a are load-bearing on BSD grep (macOS) for matches inside binaries.

# ------------------------------------------------------------------ preflight -
preflight() {
  log "== preflight (HEAD=$NEW_SHA  EXPECT_SHA=$EXPECT_SHA  RUNS=$RUNS) =="
  [[ "$RUNS" -ge 2 ]] || die "RUNS must be >= 2 (got $RUNS)"
  command -v python3 >/dev/null || die "python3 required on the Mac"
  command -v shasum  >/dev/null || die "shasum required on the Mac"

  # clean SOURCE tree (docs churn allowed; crates/Cargo dirt is not)
  local dirty; dirty="$(git -C "$REPO_ROOT" status --porcelain -- crates Cargo.toml Cargo.lock)"
  [[ -z "$dirty" ]] || die "source tree DIRTY (crates/Cargo.*) — binary identity unclear:
$dirty"

  # Mac client
  [[ -x "$MAC_BLIT" ]] || die "MAC_BLIT not executable: $MAC_BLIT (cargo build --release)"
  LC_ALL=C grep -qa -- "+$EXPECT_SHA" "$MAC_BLIT" \
    || die "MAC_BLIT does not embed +$EXPECT_SHA — rebuild at HEAD or set EXPECT_SHA"

  # skippy client + daemon
  sssh "test -x '$SKIPPY_BLIT'"   || die "skippy blit missing/not exec: $SKIPPY_BLIT"
  sssh "test -x '$SKIPPY_DAEMON'" || die "skippy blit-daemon missing/not exec: $SKIPPY_DAEMON"
  sssh "grep -qa -- '+$EXPECT_SHA' '$SKIPPY_BLIT'"   || die "skippy blit does not embed +$EXPECT_SHA"
  sssh "grep -qa -- '+$EXPECT_SHA' '$SKIPPY_DAEMON'" || die "skippy blit-daemon does not embed +$EXPECT_SHA"
  sssh "sudo -n true" 2>/dev/null \
    || log "  WARNING: 'sudo -n' on skippy not permitted — drop_caches will be skipped and runs may read warm"

  # windows client + staged daemon
  wssh "if(-not(Test-Path '$WIN_BLIT')){exit 1}"        || die "windows client missing: $WIN_BLIT"
  wssh "if(-not(Test-Path '$NEW_WIN_DAEMON')){exit 1}"  || die "windows staged daemon missing: $NEW_WIN_DAEMON"
  wssh "if(Select-String -Path '$WIN_BLIT' -SimpleMatch -Quiet -Pattern '+$EXPECT_SHA'){exit 0}else{exit 1}" \
    || die "windows blit does not embed +$EXPECT_SHA — restage the native build"
  wssh "if(Select-String -Path '$NEW_WIN_DAEMON' -SimpleMatch -Quiet -Pattern '+$EXPECT_SHA'){exit 0}else{exit 1}" \
    || die "windows daemon does not embed +$EXPECT_SHA — restage the native build"
  wssh "if(-not(Test-Path '$WIN_TEST\\purge-standby.ps1')){exit 1}" \
    || log "  WARNING: $WIN_TEST\\purge-standby.ps1 missing — Windows standby purge will be skipped (stage scripts/windows/purge-standby.ps1)"

  # stale-listener refusal (also proves ssh reachability to both daemon hosts)
  if sssh "ss -ltn 2>/dev/null | awk '{print \$4}' | grep -q ':$PORT\$'"; then die "skippy: port $PORT already in LISTEN — stop the stale daemon first"; fi
  if wssh "if(Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue){exit 0}else{exit 1}" 2>/dev/null; then die "windows: port $PORT already in LISTEN — stop the stale daemon first"; fi

  log "preflight OK"
}

# staging manifest — sha256 of every blit binary on all three hosts (plan D6)
write_manifest() {
  local f="$OUT_DIR/staging-manifest.txt"
  local h_mc h_sc h_sd h_wc h_wd
  h_mc="$(sha256_local "$MAC_BLIT")"
  h_sc="$(sha256_skippy "$SKIPPY_BLIT")"
  h_sd="$(sha256_skippy "$SKIPPY_DAEMON")"
  h_wc="$(sha256_win "$WIN_BLIT")"
  h_wd="$(sha256_win "$NEW_WIN_DAEMON")"
  {
    echo "host,role,sha,sha256,path"
    echo "mac,client,$NEW_SHA,$h_mc,$MAC_BLIT"
    echo "skippy,client,$NEW_SHA,$h_sc,$SKIPPY_BLIT"
    echo "skippy,daemon,$NEW_SHA,$h_sd,$SKIPPY_DAEMON"
    echo "windows,client,$NEW_SHA,$h_wc,$WIN_BLIT"
    echo "windows,daemon,$NEW_SHA,$h_wd,$NEW_WIN_DAEMON"
  } > "$f"
  log "staging manifest recorded (5 hashes)"
}

# ------------------------------------------------------------ daemon config ----
WIN_DAEMON_HASH=""
write_configs() {
  local hosts="\"$MAC_HOST/32\", \"$SKIPPY_HOST/32\", \"$WIN_HOST/32\""
  # skippy config (Linux path -> TOML basic string)
  cat > "$OUT_DIR/skippy-bench.toml" <<EOF
[daemon]
bind = "0.0.0.0"
port = $PORT
no_mdns = true

[delegation]
allow_delegated_pull = true
allowed_source_hosts = [$hosts]

[[module]]
name = "bench"
path = "$SKIPPY_MODULE"
EOF
  # windows config (backslash path -> TOML *literal* string, single quotes)
  cat > "$OUT_DIR/win-bench.toml" <<EOF
[daemon]
bind = "0.0.0.0"
port = $PORT
no_mdns = true

[delegation]
allow_delegated_pull = true
allowed_source_hosts = [$hosts]

[[module]]
name = "bench"
path = '$WIN_MODULE'
EOF
  SKIPPY_CFG="$SKIPPY_TEMP/blit-bench-deleg.toml"
  WIN_CFG="$WIN_TEST\\blit-bench-deleg.toml"
  scp "${SSH_MUX[@]}" -q "$OUT_DIR/skippy-bench.toml" "$SKIPPY_SSH:$SKIPPY_CFG" || die "scp skippy config failed"
  scp "${SSH_MUX[@]}" -q "$OUT_DIR/win-bench.toml" "$WIN_SSH:$(printf '%s' "$WIN_CFG" | tr '\\' '/')" || die "scp windows config failed"
}

# ------------------------------------------------------------ daemon lifecycle -
SKIPPY_DAEMON_STARTED=0 ; SKIPPY_PID=""
WIN_DAEMON_STARTED=0    ; WIN_PID=""

skippy_daemon_start() {
  sssh "ss -ltn 2>/dev/null | awk '{print \$4}' | grep -q ':$PORT\$'" && die "skippy: port $PORT already in LISTEN — refusing"
  sssh "mkdir -p '$SKIPPY_MODULE'"
  SKIPPY_PID="$(sssh "nohup '$SKIPPY_DAEMON' --config '$SKIPPY_CFG' > '$SKIPPY_TEMP/blit-bench-daemon.log' 2>&1 & echo \$!" | nocr | tr -dc '0-9')"
  [[ -n "$SKIPPY_PID" ]] || die "skippy: failed to launch daemon"
  SKIPPY_DAEMON_STARTED=1
  local i
  for i in $(seq 1 40); do
    sssh "ss -ltn 2>/dev/null | awk '{print \$4}' | grep -q ':$PORT\$'" && { log "skippy daemon up (pid $SKIPPY_PID) on $SKIPPY_HOST:$PORT"; return 0; }
    sleep 0.5
  done
  sssh "tail -20 '$SKIPPY_TEMP/blit-bench-daemon.log'" >&2 || true
  die "skippy: daemon pid $SKIPPY_PID never listened on $PORT"
}
skippy_daemon_stop() {
  [[ "$SKIPPY_DAEMON_STARTED" == 1 && -n "$SKIPPY_PID" ]] || return 0
  # PID-scoped, comm-verified: only kill if THAT pid is still a blit process.
  sssh "if [ -r /proc/$SKIPPY_PID/comm ] && grep -qi blit /proc/$SKIPPY_PID/comm; then kill $SKIPPY_PID 2>/dev/null; for i in 1 2 3 4 5 6; do [ -d /proc/$SKIPPY_PID ] || break; sleep 0.5; done; [ -d /proc/$SKIPPY_PID ] && kill -9 $SKIPPY_PID 2>/dev/null; fi; true" 2>/dev/null || true
  SKIPPY_DAEMON_STARTED=0
  log "skippy daemon stopped (pid $SKIPPY_PID)"
}

win_daemon_start() {
  wssh "if(Get-Process blit-daemon -ErrorAction SilentlyContinue){'STALE'; exit 1}" || die "windows: a blit-daemon is already running — stop it first"
  [[ -n "$WIN_DAEMON_HASH" ]] || die "WIN_DAEMON_HASH not captured (write_manifest must run first)"
  # copy staged daemon into the fixed active path (one firewall rule) and verify
  # the landed bytes ARE the staged build before launch (no handshake covers this).
  wssh "\$ErrorActionPreference='Stop'
\$d=Split-Path '$ACTIVE_WIN_DAEMON'; if(!(Test-Path \$d)){New-Item -ItemType Directory -Force \$d | Out-Null}
Copy-Item '$NEW_WIN_DAEMON' '$ACTIVE_WIN_DAEMON' -Force
if((Get-FileHash -Algorithm SHA256 '$ACTIVE_WIN_DAEMON').Hash.ToLower() -ne '$WIN_DAEMON_HASH'){ 'active exe hash mismatch after copy'; exit 1 }
if(!(Test-Path '$WIN_MODULE')){New-Item -ItemType Directory -Force '$WIN_MODULE' | Out-Null}" \
    || die "windows: staging active daemon failed (BUILD_MISMATCH?)"
  # WMI launch: Win32-OpenSSH reaps Start-Process children when the ssh session
  # closes; Win32_Process.Create detaches the daemon from the session (STATE.md).
  wssh "\$ErrorActionPreference='Stop'
\$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = 'cmd /c \"\"$ACTIVE_WIN_DAEMON\" --config \"$WIN_CFG\" > \"$WIN_TEST\\daemon-out.log\" 2> \"$WIN_TEST\\daemon-err.log\"\"' }
if(\$r.ReturnValue -ne 0){ \"wmi create failed: \$(\$r.ReturnValue)\"; exit 1 }
Set-Content -Path '$WIN_TEST\\daemon-wmi.pid' -Value \$r.ProcessId" \
    || die "windows: WMI daemon launch failed"
  WIN_DAEMON_STARTED=1
  sleep 2
  # resolve the daemon pid as the blit-daemon whose PARENT is our cmd (this launch)
  WIN_PID="$(wssh "\$c=Get-Content '$WIN_TEST\\daemon-wmi.pid' -ErrorAction SilentlyContinue
\$d=Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object { \$_.ParentProcessId -eq \$c } | Select-Object -First 1
if(\$d){ \$d.ProcessId }" | nocr | tr -dc '0-9')"
  local i
  for i in $(seq 1 40); do
    [[ -n "$WIN_PID" ]] && wssh "if(Get-NetTCPConnection -State Listen -LocalPort $PORT -ErrorAction SilentlyContinue){exit 0}else{exit 1}" \
      && { log "windows daemon up (pid $WIN_PID) on $WIN_HOST:$PORT"; return 0; }
    sleep 0.5
    [[ -z "$WIN_PID" ]] && WIN_PID="$(wssh "\$c=Get-Content '$WIN_TEST\\daemon-wmi.pid' -ErrorAction SilentlyContinue; \$d=Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object { \$_.ParentProcessId -eq \$c } | Select-Object -First 1; if(\$d){\$d.ProcessId}" | nocr | tr -dc '0-9')"
  done
  wssh "Get-Content '$WIN_TEST\\daemon-err.log' -ErrorAction SilentlyContinue | Select-Object -First 10" >&2 || true
  die "windows: daemon never listened on $PORT"
}
win_daemon_stop() {
  [[ "$WIN_DAEMON_STARTED" == 1 ]] || return 0
  wssh "\$p='$WIN_PID'
if(\$p){ \$proc=Get-Process -Id \$p -ErrorAction SilentlyContinue; if(\$proc -and \$proc.ProcessName -eq 'blit-daemon'){ Stop-Process -Id \$p -Force } }
else { \$c=Get-Content '$WIN_TEST\\daemon-wmi.pid' -ErrorAction SilentlyContinue; if(\$c){ \$d=Get-CimInstance Win32_Process -Filter \"Name='blit-daemon.exe'\" | Where-Object { \$_.ParentProcessId -eq \$c } | Select-Object -First 1; if(\$d){ Stop-Process -Id \$d.ProcessId -Force } } }
Remove-Item '$WIN_TEST\\daemon-wmi.pid' -ErrorAction SilentlyContinue" 2>/dev/null || true
  WIN_DAEMON_STARTED=0
  log "windows daemon stopped (pid $WIN_PID)"
}

daemons_start() { write_configs; skippy_daemon_start; win_daemon_start; }
daemons_stop()  { win_daemon_stop; skippy_daemon_stop; }

# ------------------------------------------------------------ drain + cold ----
drain_win() {   # Write-VolumeCache then poll D: write rate until quiet
  wssh "\$ErrorActionPreference='Stop'
Write-VolumeCache -DriveLetter '$WIN_DRIVE'
\$quiet=0
for(\$i=0; \$i -lt $DRAIN_ITERS; \$i++){
  try{ \$w=(Get-Counter '\\PhysicalDisk(_Total)\\Disk Write Bytes/sec' -SampleInterval 2 -MaxSamples 1).CounterSamples[0].CookedValue }catch{ 'DRAIN-ERROR'; exit 0 }
  if(\$null -ne \$w -and [double]\$w -lt $WIN_DRAIN_THRESH){ \$quiet++ } else { \$quiet=0 }
  if(\$quiet -ge $DRAIN_QUIET){ \"drained \$((\$i+1)*2)s\"; exit 0 }
}
'DRAIN-TIMEOUT'" 2>/dev/null | nocr || echo DRAIN-ERROR
}
drain_skippy() {   # sync then poll /proc/diskstats sectors-written until quiet
  sssh "sync
quiet=0
for i in \$(seq 1 $DRAIN_ITERS); do
  a=\$(awk '\$3 ~ /$SKIPPY_DISK_REGEX/ {s+=\$10} END{printf \"%.0f\", s}' /proc/diskstats)
  sleep 2
  b=\$(awk '\$3 ~ /$SKIPPY_DISK_REGEX/ {s+=\$10} END{printf \"%.0f\", s}' /proc/diskstats)
  if [ \$((b-a)) -lt $SKIPPY_DRAIN_SECTORS ]; then quiet=\$((quiet+1)); else quiet=0; fi
  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained \${i}x2s\"; exit 0; fi
done
echo DRAIN-TIMEOUT" 2>/dev/null | nocr || echo DRAIN-ERROR
}

RUN_DRAIN=""
prep_run() {   # $1 = dest kind (win|skippy). Drain the dest, then cold BOTH ends.
  local dest_kind="$1" outcome
  if [[ "$dest_kind" == win ]]; then outcome="$(drain_win)"; else outcome="$(drain_skippy)"; fi
  RUN_DRAIN="${outcome:-DRAIN-ERROR}"
  RUN_DRAIN="${RUN_DRAIN// /_}"
  echo "$RUN_DRAIN" >> "$OUT_DIR/drain.log"
  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($dest_kind) window UNDRAINED ($RUN_DRAIN) — pair will void, rerun"
  # cold BOTH data-plane ends every run (plan D4)
  sssh "sudo -n sh -c 'sync; echo 3 > /proc/sys/vm/drop_caches'" 2>/dev/null || true
  wssh "pwsh -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null 2>&1 \
    || wssh "powershell -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null 2>&1 || true
}

# ------------------------------------------------------------ fixtures --------
# Identical shapes to otp-2 (plan D5): 1 GiB large / 10k x 4 KiB small /
# 512 MiB + 5000 x 2 KiB mixed. Existence alone is NOT trusted — verify count+bytes.
FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
FIX_COUNT_small=10000; FIX_BYTES_small=40960000
FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
fixture_shape() { find "$1" -type f -exec stat -f%z {} + 2>/dev/null | awk '{s+=$1} END{printf "%d,%d\n", NR, s}'; }
verify_fixture() {
  local w="$1" wc wb got; eval "wc=\$FIX_COUNT_$w; wb=\$FIX_BYTES_$w"
  got="$(fixture_shape "$MAC_WORK/src_$w")"
  [[ "$got" == "$wc,$wb" ]] || die "fixture src_$w shape $got, want $wc,$wb — remove $MAC_WORK/src_$w and re-run"
}
gen_fixtures() {
  mkdir -p "$MAC_WORK"
  if [[ ! -d "$MAC_WORK/src_large" ]]; then
    mkdir -p "$MAC_WORK/src_large"
    dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
  fi
  if [[ ! -d "$MAC_WORK/src_small" ]]; then
    mkdir -p "$MAC_WORK/src_small"; local i d
    for i in $(seq 1 10000); do d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"; dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null; done
  fi
  if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
    mkdir -p "$MAC_WORK/src_mixed"; local i d
    dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
    for i in $(seq 1 5000); do d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"; dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null; done
  fi
  local w; for w in large small mixed; do verify_fixture "$w"; done
  log "fixtures verified (count + byte sum)"
}

# Stage pull sources onto BOTH daemon hosts (untimed; shared across arms — bytes
# are bytes). Land pull_src_<w>/src_<w>/ by copying the DIR src_<w>. Verify count.
skippy_module_count() { sssh "find '$SKIPPY_MODULE/$1' -type f 2>/dev/null | wc -l" | nocr | tr -dc '0-9'; }
win_module_count() { wssh "(Get-ChildItem -Path '$WIN_MODULE\\$1' -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count" | nocr | tr -dc '0-9'; }
stage_pull_sources() {
  log "staging pull sources on skippy + windows (untimed)"
  local w want got
  for w in large small mixed; do
    eval "want=\$FIX_COUNT_$w"
    got="$(skippy_module_count "pull_src_$w/src_$w")"; got="${got:-0}"
    if [[ "$got" != "$want" ]]; then
      "$MAC_BLIT" copy "$MAC_WORK/src_$w" "$SKIPPY_HOST:$PORT:/bench/pull_src_$w/" --yes > /dev/null 2> "$OUT_DIR/blit-logs/stage_skippy_$w.err" || die "staging pull_src_$w -> skippy failed"
      got="$(skippy_module_count "pull_src_$w/src_$w")"; got="${got:-0}"
      [[ "$got" == "$want" ]] || die "pull_src_$w on skippy still $got/$want after staging"
    fi
    got="$(win_module_count "pull_src_$w\\src_$w")"; got="${got:-0}"
    if [[ "$got" != "$want" ]]; then
      "$MAC_BLIT" copy "$MAC_WORK/src_$w" "$WIN_HOST:$PORT:/bench/pull_src_$w/" --yes > /dev/null 2> "$OUT_DIR/blit-logs/stage_win_$w.err" || die "staging pull_src_$w -> windows failed"
      got="$(win_module_count "pull_src_$w\\src_$w")"; got="${got:-0}"
      [[ "$got" == "$want" ]] || die "pull_src_$w on windows still $got/$want after staging"
    fi
    log "  pull_src_$w staged/verified ($want files, both hosts)"
  done
}

# ------------------------------------------------------------ timed runs ------
RUNS_CSV="$OUT_DIR/runs.csv"
echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$RUNS_CSV"
META="$OUT_DIR/meta.csv"
echo "cell,pairs_attempted,complete" > "$META"
RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
CUR_W=""; CUR_FLAG=""; CUR_CARRIER=""

# deleg_run CELL RID DEST_KIND SRC_REMOTE  — Mac-timed delegated copy
deleg_run() {
  local cell="$1" rid="$2" dest_kind="$3" src="$4"
  local fresh="${SESSION_TAG}_${cell}_${rid}" dst dphys start end rc=0
  if [[ "$dest_kind" == win ]]; then dst="$WIN_HOST:$PORT:/bench/$fresh/"; dphys="$WIN_MODULE\\$fresh"
  else                               dst="$SKIPPY_HOST:$PORT:/bench/$fresh/"; dphys="$SKIPPY_MODULE/$fresh"; fi
  prep_run "$dest_kind"
  start="$(now_ms)"
  "$MAC_BLIT" copy "$src" "$dst" --yes $CUR_FLAG > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
  end="$(now_ms)"
  if [[ "$dest_kind" == win ]]; then RUN_FLUSH="$(flush_win_ms)"; wssh "Remove-Item -Recurse -Force '$dphys' -ErrorAction SilentlyContinue" >/dev/null 2>&1 || true
  else                               RUN_FLUSH="$(sync_skippy_ms)"; sssh "rm -rf '$dphys'" 2>/dev/null || true; fi
  RUN_VALID=yes
  [[ -z "$RUN_FLUSH" || "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
  RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc
  [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
}

# direct_run CELL RID DEST_KIND SRC_REMOTE  — dest-host self-timed local pull
T_MS=0; T_RC=0
win_client_run() {   # src dst_physical -> sets T_MS/T_RC (Stopwatch on Windows)
  local out
  out="$(wssh "\$sw=[Diagnostics.Stopwatch]::StartNew(); & '$WIN_BLIT' copy '$1' '$2' --yes $CUR_FLAG > \$null 2> '$WIN_TEST\\client-err.log'; \$rc=\$LASTEXITCODE; \$sw.Stop(); \"R:\$([int]\$sw.Elapsed.TotalMilliseconds),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
  if [[ "$out" == *,* ]]; then T_MS="${out%%,*}"; T_RC="${out##*,}"; else T_MS=0; T_RC=99; fi
}
skippy_client_run() {   # src dst_physical -> sets T_MS/T_RC (/proc/uptime bracket)
  local out
  out="$(sssh "a=\$(awk '{print int(\$1*1000)}' /proc/uptime); '$SKIPPY_BLIT' copy '$1' '$2' --yes $CUR_FLAG >/dev/null 2>/tmp/blit-client-err.log; rc=\$?; b=\$(awk '{print int(\$1*1000)}' /proc/uptime); echo \"R:\$((b-a)),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
  if [[ "$out" == *,* ]]; then T_MS="${out%%,*}"; T_RC="${out##*,}"; else T_MS=0; T_RC=99; fi
}
direct_run() {
  local cell="$1" rid="$2" dest_kind="$3" src="$4"
  local fresh="${SESSION_TAG}_${cell}_${rid}" dphys
  prep_run "$dest_kind"
  if [[ "$dest_kind" == win ]]; then
    dphys="$WIN_MODULE\\$fresh"
    win_client_run "$src" "$dphys"
    RUN_FLUSH="$(flush_win_ms)"; wssh "Remove-Item -Recurse -Force '$dphys' -ErrorAction SilentlyContinue" >/dev/null 2>&1 || true
  else
    dphys="$SKIPPY_MODULE/$fresh"
    skippy_client_run "$src" "$dphys"
    RUN_FLUSH="$(sync_skippy_ms)"; sssh "rm -rf '$dphys'" 2>/dev/null || true
  fi
  RUN_VALID=yes
  [[ -z "$RUN_FLUSH" || "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
  RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC
  [[ "$T_RC" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
}

# arm wrappers (CUR_W bound by the matrix driver)
sw_deleg()  { deleg_run  "$1" "$2" win    "$SKIPPY_HOST:$PORT:/bench/pull_src_${CUR_W}/src_${CUR_W}/"; }
sw_direct() { direct_run "$1" "$2" win    "$SKIPPY_HOST:$PORT:/bench/pull_src_${CUR_W}/src_${CUR_W}/"; }
ws_deleg()  { deleg_run  "$1" "$2" skippy "$WIN_HOST:$PORT:/bench/pull_src_${CUR_W}/src_${CUR_W}/"; }
ws_direct() { direct_run "$1" "$2" skippy "$WIN_HOST:$PORT:/bench/pull_src_${CUR_W}/src_${CUR_W}/"; }

# ABBA counterbalanced interleave with the D2 pair-void valid-run rule.
# run_pair_loop CELL ARM_A ARM_B FN_A FN_B DIRECT_HOST
run_pair_loop() {
  local cell="$1" armA="$2" armB="$3" fnA="$4" fnB="$5" directHost="$6"
  local slot=1 attempts=0 valid=0 max_attempts=$(( 2 * RUNS ))
  log "=== $cell ($armA vs $armB, ABBA, $RUNS pairs, carrier=$CUR_CARRIER) ==="
  while (( valid < RUNS && attempts < max_attempts )); do
    attempts=$(( attempts + 1 ))
    local order pair_valid=yes arm fn aname init rid rowA="" rowB=""
    if (( slot % 2 )); then order="A B"; else order="B A"; fi
    for arm in $order; do
      if [[ "$arm" == A ]]; then fn="$fnA"; aname="$armA"; else fn="$fnB"; aname="$armB"; fi
      case "$aname" in delegated) init=mac;; direct) init="$directHost";; *) init="?";; esac
      rid="${aname}_s${slot}a${attempts}"   # arm in every path -> no cross-arm collision
      "$fn" "$cell" "$rid"
      [[ "$RUN_VALID" == yes ]] || pair_valid=no
      local row="$cell,$aname,$NEW_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN"
      if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
      log "  $cell/$aname slot $slot (attempt $attempts): ${RUN_MS}ms (flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN)"
    done
    echo "$rowA,$pair_valid" >> "$RUNS_CSV"
    echo "$rowB,$pair_valid" >> "$RUNS_CSV"
    if [[ "$pair_valid" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 )); else log "  $cell: pair slot $slot VOIDED — re-running the slot"; fi
  done
  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS valid pairs after $attempts attempts"
  else echo "$cell,$attempts,yes" >> "$META"; fi
}

# ------------------------------------------------------------ smoke -----------
# Reachability + delegation-gate + firewall smoke: a tiny probe both directions,
# both arms. The data plane binds ephemeral ports, so the transfer IS the firewall
# test. Any failure aborts. Runs live before the matrix (needs daemons up).
smoke() {
  log "== smoke: reachability + delegation + firewall (both directions/arms) =="
  mkdir -p "$MAC_WORK/src_probe"; echo "otp12c-smoke" > "$MAC_WORK/src_probe/probe.txt"
  "$MAC_BLIT" copy "$MAC_WORK/src_probe" "$SKIPPY_HOST:$PORT:/bench/pull_src_probe/" --yes > /dev/null 2>&1 || die "smoke: stage probe -> skippy failed (Mac->skippy reachability)"
  "$MAC_BLIT" copy "$MAC_WORK/src_probe" "$WIN_HOST:$PORT:/bench/pull_src_probe/" --yes > /dev/null 2>&1 || die "smoke: stage probe -> windows failed (Mac->windows reachability)"
  local CUR_W=probe CUR_FLAG="" CUR_CARRIER=tcp ok=1 spec fn dk
  for spec in "sw_deleg:win" "sw_direct:win" "ws_deleg:skippy" "ws_direct:skippy"; do
    fn="${spec%%:*}"; dk="${spec##*:}"
    RUN_MS=0 RUN_FLUSH=0 RUN_EXIT=1 RUN_VALID=no RUN_DRAIN=DRAIN-ERROR
    "$fn" smoke probe
    if [[ "$RUN_EXIT" == 0 ]]; then log "  smoke $fn OK (${RUN_MS}ms)"; else log "  smoke $fn FAILED (exit=$RUN_EXIT)"; ok=0; fi
  done
  # cleanup probe sources
  sssh "rm -rf '$SKIPPY_MODULE/pull_src_probe'" 2>/dev/null || true
  wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\pull_src_probe' -ErrorAction SilentlyContinue" >/dev/null 2>&1 || true
  [[ "$ok" == 1 ]] || die "smoke FAILED — check daemon reachability, [delegation] allowed_source_hosts (IP/CIDR!), and the data-plane firewall on both hosts"
  log "smoke OK"
}

# ------------------------------------------------------------ verdicts --------
compute_verdicts() {
  python3 - "$RUNS_CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" "$OUT_DIR/drain-outcomes.txt" <<'PYEOF'
import csv, sys
runs_p, meta_p, summary_p, verdicts_p, drain_p = sys.argv[1:6]
rows = list(csv.DictReader(open(runs_p)))
meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}

by_arm, voided = {}, {}
for r in rows:
    key = (r["cell"], r["arm"])
    if r["valid"] == "yes":
        by_arm.setdefault(key, []).append(int(r["ms"]))
    else:
        voided[key] = voided.get(key, 0) + 1

def median(v):
    v = sorted(v); n = len(v)
    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2

def complete(cell):
    if cell not in meta or meta[cell]["complete"] != "yes":
        return False
    arms = [a for (c, a) in by_arm if c == cell]
    return "delegated" in arms and "direct" in arms

def m(cell, arm):
    return median(by_arm[(cell, arm)]) if (cell, arm) in by_arm else None

def bar(hi, lo):   # max/min <= 1.10, integer-exact
    return 10 * hi <= 11 * lo

# summary.csv (plan D5 schema) — complete cells only, medians over valid runs
with open(summary_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
    for (cell, arm) in sorted(by_arm):
        if not complete(cell):
            continue
        v = by_arm[(cell, arm)]
        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")

# verdicts.csv (plan D5 schema) — delegated parity, bar max/min <= 1.10
with open(verdicts_p, "w") as f:
    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
    for cell in sorted(meta):
        if not complete(cell):
            f.write(f"{cell},delegated,delegated,direct,,,,1.10,INCOMPLETE\n")
            continue
        d, x = m(cell, "delegated"), m(cell, "direct")
        hi, lo = max(d, x), min(d, x)
        outcome = "PASS" if bar(hi, lo) else "FAIL"
        # lhs_ms=delegated median, rhs_ms=direct median; ratio is the symmetric
        # parity ratio (max/min) that the bar tests. Directional detail is fully
        # recoverable from the two medians. grpc cell is computed identically;
        # its carrier (in the cell name) marks it secondary.
        f.write(f"{cell},delegated,delegated,direct,{d},{x},{hi/lo:.3f},1.10,{outcome}\n")

# drain-outcomes.txt — audit of drain quality per cell/arm (recorded)
agg = {}
for r in rows:
    k = (r["cell"], r["arm"], r["drain"])
    agg[k] = agg.get(k, 0) + 1
with open(drain_p, "w") as f:
    f.write("# drain outcome counts per cell/arm (recorded; undrained voids the pair per D2)\n")
    for (cell, arm, drain), n in sorted(agg.items()):
        f.write(f"{cell:22s} {arm:10s} {drain:16s} {n}\n")
PYEOF
}

# ------------------------------------------------------------ cleanup ---------
on_exit() {
  local rc=$?
  daemons_stop || true
  # best-effort sweep of any leftover fresh dirs from THIS session
  sssh "rm -rf '$SKIPPY_MODULE'/${SESSION_TAG}_* 2>/dev/null; true" 2>/dev/null || true
  wssh "Get-ChildItem '$WIN_MODULE' -Directory -ErrorAction SilentlyContinue | Where-Object { \$_.Name -like '${SESSION_TAG}_*' } | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue" >/dev/null 2>&1 || true
  ssh "${SSH_MUX[@]}" -O exit "$SKIPPY_SSH" 2>/dev/null || true
  ssh "${SSH_MUX[@]}" -O exit "$WIN_SSH" 2>/dev/null || true
  rm -rf "$MUX_DIR" 2>/dev/null || true
  exit $rc
}
trap on_exit EXIT

# ------------------------------------------------------------ main ------------
main() {
  log "OUT_DIR=$OUT_DIR"
  preflight
  write_manifest
  # capture the staged Windows daemon hash for the active-exe launch verify
  WIN_DAEMON_HASH="$(sha256_win "$NEW_WIN_DAEMON")"

  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
    log "PREFLIGHT_ONLY: static gates + manifest passed; no daemon started, nothing timed."
    log "  (the live reachability/delegation/firewall smoke runs at the start of a real session)"
    exit 0
  fi

  log "session $SESSION_TAG  sha=$NEW_SHA  skippy=$SKIPPY_HOST  windows=$WIN_HOST  mac=$MAC_HOST"
  gen_fixtures
  daemons_start
  smoke
  stage_pull_sources

  # ---- matrix: 6 TCP cells (3 fixtures x 2 dirs) + 1 grpc smoke cell ----
  local w
  for w in large small mixed; do
    CUR_W="$w"; CUR_FLAG=""; CUR_CARRIER=tcp
    want_cell "sw_tcp_$w" && run_pair_loop "sw_tcp_$w" delegated direct sw_deleg sw_direct win
    want_cell "ws_tcp_$w" && run_pair_loop "ws_tcp_$w" delegated direct ws_deleg ws_direct skippy
  done
  CUR_W=large; CUR_FLAG="--force-grpc"; CUR_CARRIER=grpc   # secondary carrier smoke (skippy->win, large)
  want_cell "sw_grpc_large" && run_pair_loop "sw_grpc_large" delegated direct sw_deleg sw_direct win

  # a mistyped CELLS entry must not exit 0 with empty evidence
  if [[ -n "$CELLS" ]]; then
    local c
    for c in ${CELLS//,/ }; do
      tail -n +2 "$META" | grep -q "^$c," || die "CELLS entry '$c' matched no comparison — nothing measured"
    done
  fi

  daemons_stop
  compute_verdicts
  log ""
  log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
  log ""
  log "=== VERDICTS (delegated parity, bar max/min <= 1.10; grpc = secondary) ==="
  column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
  log "runs: $RUNS_CSV   manifest: $OUT_DIR/staging-manifest.txt   drain: $OUT_DIR/drain-outcomes.txt"
}

main "$@"
