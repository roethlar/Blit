#!/usr/bin/env bash
# =============================================================================
# bench_baseline_tools.sh — external baselines on rig W (Mac <-> Windows)
#
# WHY (owner, 2026-07-13): "end goal is fastest option on each OS, not absolute
# OS parity. as long as we beat the other options, we can call that OS overhead
# until we get more hardware datapoints." Plus: "if robocopy beats blit win>mac,
# then we have a problem. if blit beats robocopy, that's a win."
#
# TWO DIFFERENT QUESTIONS — do not conflate them:
#
#  (A) THE ASYMMETRY CONTROL (the decisive one).  Compare a tool AGAINST ITSELF
#      in the two initiator layouts for the SAME data direction. Whatever that
#      tool's transport overhead is, it is identical in both runs, so it cancels
#      and the comparison is protocol-independent. blit fails this on rig W for
#      TCP+mixed (1.300). Does rclone fail it too?
#        * rclone symmetric  => blit TCP plane is the odd one out => OUR BUG.
#        * rclone asymmetric => the OS pair really is directionally lopsided
#                               => "OS overhead" is defensible.
#      NOTE the in-house control already points at us: blit gRPC (also TCP, via
#      HTTP/2) is SYMMETRIC on this same pair (1.021), and blit TCP is symmetric
#      on Linux<->Linux (1.003-1.092). Only blit-TCP-on-Mac/Windows is lopsided.
#
#  (B) THE SHIPPING BAR (what the owner actually ships against).  Compare
#      DIFFERENT tools' wall-clock time for the same bytes. These do NOT speak
#      the same protocol (blit = custom TCP; rclone = SSH/SFTP; robocopy = SMB),
#      so this is "what a user experiences with each tool", NOT a controlled
#      protocol comparison. Report it as such.
#
# METHODOLOGY (same as the blit harnesses; anything less is not comparable):
#   * cold caches BOTH ends every run (macOS purge; Windows standby purge);
#   * fresh, never-seen destination per run;
#   * durability keyed by the DESTINATION host, never the initiator/verb — the
#     bug that invalidated the first Linux session (see otp12-perf README);
#   * interleaved ABBA across arms; a nonzero exit voids the run;
#   * identical fixtures to otp-2/otp-12 (mixed = 512 MiB + 5000 x 2 KiB).
#
# Usage:  MAC_HOST=10.1.10.91 WIN_HOST=10.1.10.177 bash scripts/bench_baseline_tools.sh
#         TOOLS=rclone RUNS=3 ... (subset)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

MAC_HOST="${MAC_HOST:?set MAC_HOST to the Mac 10GbE IP}"
WIN_HOST="${WIN_HOST:-10.1.10.177}"
WIN_SSH="${WIN_SSH:-michael@$WIN_HOST}"
MAC_USER="${MAC_USER:-$(whoami)}"
RUNS="${RUNS:-3}"
TOOLS="${TOOLS:-blit,rclone}"          # robocopy needs SMB sharing on the Mac
FIXTURE="${FIXTURE:-mixed}"            # where P1 lives
MAC_WORK="${MAC_WORK:-$HOME/blit-bench-work}"
WIN_TEST="${WIN_TEST:-D:\\blit-test}"
BLIT="${BLIT:-$REPO_ROOT/target/release/blit}"
WIN_BLIT="${WIN_BLIT:-D:\\blit-test\\bins\\f35702a\\blit.exe}"
PORT="${PORT:-9031}"

SESSION="$(date +%Y%m%dT%H%M%S)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/baseline_tools_$SESSION}"
mkdir -p "$OUT_DIR"
CSV="$OUT_DIR/runs.csv"; echo "tool,data_dir,initiator,run,ms,exit,valid" > "$CSV"

MUX="$(mktemp -d /tmp/blit-base-mux.XXXXXX)"
SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ControlMaster=auto
         -o "ControlPath=$MUX/%C" -o ControlPersist=180)
wssh() { ssh "${SSH_MUX[@]}" "$WIN_SSH" "$@"; }
nocr() { tr -d '\r'; }
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
die() { log "FATAL: $*"; exit 1; }

trap 'ssh "${SSH_MUX[@]}" -O exit "$WIN_SSH" 2>/dev/null || true; rm -rf "$MUX"' EXIT

# --- cold caches, both ends -------------------------------------------------
cold_both() {
  sync; sudo -n /usr/sbin/purge 2>/dev/null || die "need NOPASSWD /usr/sbin/purge on the Mac"
  wssh "pwsh -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null 2>&1 \
    || wssh "powershell -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null 2>&1 \
    || die "Windows standby purge failed"
}
# --- durability, keyed by DESTINATION (never the initiator) ------------------
flush_mac_ms() { python3 - "$1" <<'PY'
import os, sys, time
t = time.monotonic()
for root, _, files in os.walk(sys.argv[1]):
    for n in files:
        fd = os.open(os.path.join(root, n), os.O_RDONLY); os.fsync(fd); os.close(fd)
print(int((time.monotonic() - t) * 1000))
PY
}
flush_win_ms() {
  wssh "\$a=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); try{ Write-VolumeCache -DriveLetter D -ErrorAction Stop }catch{ 'F:NA:F'; exit 0 }; \$b=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); \"F:\$(\$b-\$a):F\"" 2>/dev/null \
    | nocr | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p;s/.*F:NA:F.*/NA/p' | head -1
}
now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }

RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
# Time a command run ON THE MAC, then add the destination-side flush.
time_on_mac() {   # $1 = dest kind (mac|win) ; $2.. = command
  local dk="$1"; shift
  local a b rc=0 flush
  cold_both
  a="$(now_ms)"; "$@" >/dev/null 2>>"$OUT_DIR/err.log" || rc=$?; b="$(now_ms)"
  if [[ "$dk" == win ]]; then flush="$(flush_win_ms)"; else flush="$(flush_mac_ms "$MAC_DEST")"; fi
  RUN_VALID=yes; [[ -z "$flush" || "$flush" == NA ]] && { RUN_VALID=no; flush=0; }
  RUN_MS=$(( b - a + flush )); RUN_EXIT=$rc
  [[ $rc -eq 0 ]] || RUN_VALID=no
}
# Time a command run ON WINDOWS (Stopwatch inside ONE ssh; the round trip stays out).
time_on_win() {   # $1 = dest kind ; $2 = the pwsh command string
  local dk="$1" cmd="$2" out rc flush
  cold_both
  out="$(wssh "\$sw=[Diagnostics.Stopwatch]::StartNew(); $cmd > \$null 2>&1; \$rc=\$LASTEXITCODE; \$sw.Stop(); \"R:\$([int]\$sw.Elapsed.TotalMilliseconds),\${rc}:R\"" \
    | nocr | sed -n 's/.*R:\([0-9][0-9]*,-\{0,1\}[0-9][0-9]*\):R.*/\1/p' | head -1)"
  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; rc="${out##*,}"; else RUN_MS=0; rc=99; fi
  if [[ "$dk" == win ]]; then flush="$(flush_win_ms)"; else flush="$(flush_mac_ms "$MAC_DEST")"; fi
  RUN_VALID=yes; [[ -z "$flush" || "$flush" == NA ]] && { RUN_VALID=no; flush=0; }
  RUN_MS=$(( RUN_MS + flush )); RUN_EXIT="$rc"
  # robocopy exit codes 0-7 are SUCCESS (8+ is failure); every other tool uses 0.
  if [[ "$ROBO" == 1 ]]; then [[ "$rc" -lt 8 ]] || RUN_VALID=no; else [[ "$rc" == 0 ]] || RUN_VALID=no; fi
}

MAC_SRC="$MAC_WORK/src_$FIXTURE"
MAC_DEST=""; ROBO=0
row() { echo "$1,$2,$3,$4,$RUN_MS,$RUN_EXIT,$RUN_VALID" >> "$CSV"
        log "  $1 $2 init=$3 run $4: ${RUN_MS}ms (exit $RUN_EXIT, valid=$RUN_VALID)"; }

# =============================== the runs ====================================
# Data direction WIN -> MAC (where blit fails: 1.300 when the MAC initiates).
# Arm 1: initiated on WINDOWS (the fast arm for blit).
# Arm 2: initiated on the MAC  (the slow arm for blit).
main() {
  [[ -d "$MAC_SRC" ]] || die "fixture missing: $MAC_SRC"
  log "session $SESSION  fixture=$FIXTURE  runs=$RUNS  tools=$TOOLS"
  log "data direction WIN->MAC; arms = initiated-on-windows vs initiated-on-mac"

  local i tag
  for (( i=1; i<=RUNS; i++ )); do
    tag="${SESSION}_r${i}"

    if [[ ",$TOOLS," == *,blit,* ]]; then
      # blit, win-initiated (push from the Windows staged tree to the Mac daemon)
      MAC_DEST="$MAC_WORK/base_${tag}_blit_wi"; mkdir -p "$MAC_DEST"; ROBO=0
      time_on_win mac "& '$WIN_BLIT' copy '$WIN_TEST\\bench-module\\pull_src_$FIXTURE\\src_$FIXTURE' '$MAC_HOST:$PORT:/bench/base_${tag}_blit_wi/' --yes"
      row blit win_to_mac windows "$i"; rm -rf "$MAC_DEST"

      # blit, mac-initiated (pull from the Windows daemon) — the failing arm
      MAC_DEST="$MAC_WORK/base_${tag}_blit_mi"; mkdir -p "$MAC_DEST"; ROBO=0
      time_on_mac mac "$BLIT" copy "$WIN_HOST:$PORT:/bench/pull_src_$FIXTURE/src_$FIXTURE" "$MAC_DEST" --yes
      row blit win_to_mac mac "$i"; rm -rf "$MAC_DEST"
    fi

    if [[ ",$TOOLS," == *,rclone,* ]]; then
      # rclone over SFTP, win-initiated: Windows pushes to the Mac over SSH.
      MAC_DEST="$MAC_WORK/base_${tag}_rc_wi"; mkdir -p "$MAC_DEST"; ROBO=0
      time_on_win mac "rclone copy 'D:/blit-test/bench-module/pull_src_$FIXTURE/src_$FIXTURE' ':sftp,host=$MAC_HOST,user=$MAC_USER:$MAC_DEST' --sftp-key-file \$env:USERPROFILE\\.ssh\\id_ed25519 --transfers 8 --checkers 8"
      row rclone win_to_mac windows "$i"; rm -rf "$MAC_DEST"

      # rclone over SFTP, mac-initiated: the Mac pulls from Windows over SSH.
      MAC_DEST="$MAC_WORK/base_${tag}_rc_mi"; mkdir -p "$MAC_DEST"; ROBO=0
      time_on_mac mac rclone copy ":sftp,host=$WIN_HOST,user=michael:D:/blit-test/bench-module/pull_src_$FIXTURE/src_$FIXTURE" "$MAC_DEST" --transfers 8 --checkers 8
      row rclone win_to_mac mac "$i"; rm -rf "$MAC_DEST"
    fi

    if [[ ",$TOOLS," == *,robocopy,* ]]; then
      # robocopy can ONLY run on Windows and ONLY speaks SMB -> it cannot test
      # the initiator axis at all. It is a SPEED reference for win->mac.
      MAC_DEST="$MAC_WORK/base_${tag}_robo"; mkdir -p "$MAC_DEST"; ROBO=1
      time_on_win mac "robocopy 'D:\\blit-test\\bench-module\\pull_src_$FIXTURE\\src_$FIXTURE' '\\\\$MAC_HOST\\${SMB_SHARE:-blit-bench-work}\\base_${tag}_robo' /E /MT:8 /NFL /NDL /NJH /NJS"
      row robocopy win_to_mac windows "$i"; rm -rf "$MAC_DEST"
    fi
  done

  log ""; log "=== MEDIANS (ms; destination-keyed durability included) ==="
  python3 - "$CSV" <<'PY'
import csv, sys, statistics as st
rows = [r for r in csv.DictReader(open(sys.argv[1])) if r["valid"] == "yes"]
by = {}
for r in rows: by.setdefault((r["tool"], r["initiator"]), []).append(int(r["ms"]))
print(f"{'tool':10} {'initiated on':13} {'median':>8} {'runs':>5}")
for k in sorted(by): print(f"{k[0]:10} {k[1]:13} {st.median(by[k]):8.0f} {len(by[k]):5d}")
print()
print("=== ASYMMETRY (the decisive control): same tool, same direction, both initiators ===")
for tool in sorted({t for t, _ in by}):
    a, b = by.get((tool, "windows")), by.get((tool, "mac"))
    if a and b:
        hi, lo = max(st.median(a), st.median(b)), min(st.median(a), st.median(b))
        print(f"  {tool:10} win-init {st.median(a):6.0f}  mac-init {st.median(b):6.0f}  ratio {hi/lo:5.3f}  {'FAIL >1.10' if hi > 1.10*lo else 'PASS'}")
    else:
        print(f"  {tool:10} (single-initiator tool — cannot test the asymmetry axis)")
PY
  log "runs: $CSV"
}
main "$@"
