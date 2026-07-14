#!/usr/bin/env bash
# =============================================================================
# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
# =============================================================================
#
# WHY THIS RIG EXISTS
# -------------------
# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
# on macOS<->Windows. Linux<->Linux shows NO P1 (8/8 PASS). macOS<->macOS is the
# untested cell of the 2x2. It answers ONE question:
#
#     Does P1 REQUIRE the macOS<->Windows PAIRING, or is it a platform-general
#     cost of the destination-initiated layout?
#
#   * reproduces -> P1 needs no Windows peer: it is NOT platform residue, the
#     "accept it as platform residue" escape closes, and every code-level
#     hypothesis strengthens;
#   * vanishes   -> P1 is pairing-dependent: platform-agnostic code mechanisms
#     weaken and a Windows-specific cost (or an interaction) rises.
#
# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
# Revision 1 of this script and of docs/STATE.md claimed "reproduces => H1 DIES,
# because H1 accuses the Windows accept branch". That is FALSE and is retracted:
# H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
# InitiatorReceivePlaneRun.add_dialed_stream, the synchronous dial-before-ACK at
# transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with
# H1, not fatal to it. (The parent plan itself warns: "'consistent with H1' is
# not confirmation.") Caught by codex review of the pre-registration, BEFORE any
# rig time was spent.
#
# WHAT IT MEASURES
#   cell = <nq|qn>_<carrier>_<fixture>
#     nq_* : data nagatha -> q        qn_* : data q -> nagatha
#   arms per cell (the ONLY variable):
#     srcinit  : the SOURCE host's CLI pushes      (source-initiated)
#     destinit : the DEST   host's CLI pulls       (destination-initiated)
#   BOTH data directions are measured, but a reproduction is NOT required in
#   both: P1's recorded signature on rig W is ONE-DIRECTIONAL (wm_tcp_mixed FAILS
#   while mw_tcp_mixed PASSES), so demanding failure in both would rewrite the
#   finding. A reproduction in EITHER direction demonstrates the layout cost
#   without a Windows peer.
#
#   Endpoint asymmetry does NOT simply cancel: switching the initiator also
#   reassigns which Mac runs the CLI and which runs the daemon, and q is the
#   faster machine. Only arm-independent costs cancel; host x role interactions
#   do not. Hence both directions are reported SEPARATELY and no conclusion may
#   lean on perfect cancellation.
#
# VERDICT: invariance bar, max(srcinit,destinit)/min <= 1.10, integer-exact
# (10*hi <= 11*lo). This script COMPUTES; it DECLARES nothing.
#
# METHODOLOGY (otp-12 shape + the two gates pf-0 proved were missing)
#   * QUIESCENCE gate on BOTH Macs (codex/cargo/rustc) — here nagatha is a bench
#     END, not merely the driver; load on either end contaminates ASYMMETRICALLY.
#   * TIME MACHINE gate on BOTH Macs — the hole pf-0 found: the old quiet-gate
#     watched only codex/cargo/rustc and would have sailed straight through the
#     backup that fired 1 minute before pf-0's run (hourly cadence; one
#     destination is a network share on skippy = the same 10GbE fabric).
#   * cold caches BOTH ends every run via `sudo -n /usr/sbin/purge` (a failed
#     purge VOIDS the pair — a warm row is worse than no row);
#   * destination disk drained to quiet (iostat) before each timed window;
#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER BY THE INITIATOR/VERB:
#     the macOS per-file fsync walk runs on the destination for BOTH arms. (The
#     otp-2w rule, re-learned the hard way: a sync inside the initiator's bracket
#     charges the pull arm for writeback the push arm gets free and MANUFACTURES
#     invariance failures — including on the gRPC control that must stay clean.)
#   * fresh never-seen destination per run; ABBA counterbalance; pair-void with a
#     2*RUNS cap then INCOMPLETE; nonzero exit or undrained window voids the pair;
#   * same-build gate: every binary embeds a CLEAN +EXPECT_SHA (never +sha.dirty).
#
# TOPOLOGY NOTE (why one end is local): the driver runs on nagatha, so the nagatha
# end is LOCAL and the q end is over ssh. This is the proven rig-W shape: each
# timed window is self-timed ON the initiating host — locally for nagatha, and
# INSIDE a single ssh for q — so the ssh round trip is outside the window by
# construction and neither arm is charged for dispatch. The driver is blocked
# waiting during every timed window, so its own load is idle and identical across
# arms.
#
# Usage:
#   EXPECT_SHA=f35702a RUNS=8 bash scripts/bench_otp12pf_mac.sh
#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
#   CELLS=nq_tcp_mixed,qn_tcp_mixed RUNS=8 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a) — the binaries are gated on it}"

# --- nagatha: LOCAL end (driver runs here) -----------------------------------
N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"    # the pinned clone
N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"

# --- q: REMOTE end ------------------------------------------------------------
Q_SSH="${Q_SSH:-michael@q}"
Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"

PORT="${PORT:-9031}"
RUNS="${RUNS:-8}"
PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
CELLS="${CELLS:-}"
SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
DRAIN_MBPS="${DRAIN_MBPS:-2}"     # dest disk considered quiet below this MB/s

# /tmp, not $TMPDIR: macOS TMPDIR busts ssh's 104-byte ControlPath cap (otp-12c).
MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"
SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }

mkdir -p "$OUT_DIR/blit-logs"
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
die() { log "FATAL: $*"; exit 1; }
nocr() { tr -d '\r'; }
want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }

# --- host abstraction: $1 = n (local nagatha) | q (remote) --------------------
# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
# falls through to the wrong host (the exact trap the Linux harness documents).
hrun() {
  local h="$1"; shift
  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "$*"; fi
}
hblit()   { [[ "$1" == n ]] && echo "$N_BLIT"   || echo "$Q_BLIT"; }
hdaemon() { [[ "$1" == n ]] && echo "$N_DAEMON" || echo "$Q_DAEMON"; }
hmod()    { [[ "$1" == n ]] && echo "$N_MODULE" || echo "$Q_MODULE"; }
hip()     { [[ "$1" == n ]] && echo "$N_IP"     || echo "$Q_IP"; }
hname()   { [[ "$1" == n ]] && echo nagatha     || echo q; }

# --- fixtures (otp-2 shapes; verified by count, never trusted) ----------------
FIX_COUNT_large=1;     FIX_COUNT_small=10000;  FIX_COUNT_mixed=5001

# --- provenance: embed +sha AND reject +sha.dirty -----------------------------
embeds_clean() {   # $1=host $2=path
  hrun "$1" "grep -qa -- '+$EXPECT_SHA' '$2' && ! grep -qa -- '+$EXPECT_SHA.dirty' '$2'"
}
sha256_of() {      # $1=host $2=path
  hrun "$1" "shasum -a 256 '$2' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f'
}

# --- the two gates pf-0 proved were missing -----------------------------------
quiescence_gate() {   # $1 = host. Bench ENDS must be quiet; load contaminates ASYMMETRICALLY.
  local h="$1" busy
  busy="$(hrun "$h" "pgrep -x codex >/dev/null && echo codex; pgrep -x cargo >/dev/null && echo cargo; pgrep -x rustc >/dev/null && echo rustc; true" | nocr | tr '\n' ' ')"
  busy="$(echo "$busy" | xargs || true)"
  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). Both Macs are bench ENDS — a busy end inflates one arm and MANUFACTURES P1 (.agents/machines.md). Stop them (do NOT blanket-kill the owner's sessions) and re-run."
}
timemachine_gate() {   # $1 = host. FAIL-CLOSED — the hole pf-0 found.
  local h="$1" running auto
  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';'" | nocr | tr -cd '0-9')"
  [[ "${running:-0}" == 1 ]] && die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench END (one destination is on skippy, the same 10GbE fabric)."
  # AUTOBACKUP ENABLED is itself disqualifying, not a warning: macOS repeats
  # HOURLY, so a backup can begin *inside* the window. pf-0's fired 1 minute
  # before its run and the old gate never looked. A warning here would let the
  # session start and be silently contaminated mid-flight.
  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null || echo 0" | nocr | tr -cd '0-9')"
  [[ "${auto:-0}" == 1 ]] && die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED — macOS repeats hourly, so a backup can start MID-SESSION. Disable it for the window (\`sudo tmutil disable\`) and re-enable after."
  true
}
spotlight_gate() {   # $1 = host. mds_stores is a recorded contaminant (.agents/machines.md).
  # Instantaneous sample: `ps` %CPU is a DECAYING AVERAGE and reads a finished
  # backup as 255% (learned in pf-0) — top -l 2 is the honest instrument.
  local h="$1" cpu
  cpu="$(hrun "$h" "top -l 2 -n 20 -o cpu -stats command,cpu 2>/dev/null | awk '/mds_stores|^mds /{c=\$NF} END{print c+0}'" | nocr | tr -cd '0-9.')"
  awk -v c="${cpu:-0}" 'BEGIN{exit !(c+0 > 20)}' \
    && die "$(hname "$h"): Spotlight (mds_stores) is actively indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
  true
}
load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
load_gate() {   # $1 = host. The Macs idle at ~1.5-2.0; above 3.0 something is running.
  local h="$1" l; l="$(load1 "$h")"
  awk -v l="${l:-0}" 'BEGIN{exit !(l+0 > 3.0)}' \
    && die "$(hname "$h"): load1 is $l (> 3.0) — a bench END must be quiet. Find what is running before starting a timed session."
  true
}

preflight() {
  [[ "$RUNS" -ge 2 ]] || die "RUNS must be >= 2"
  local h p
  for h in n q; do
    quiescence_gate "$h"
    timemachine_gate "$h"
    spotlight_gate "$h"
    load_gate "$h"
    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
      embeds_clean "$h" "$p" \
        || die "$(hname "$h"): $p does not embed a CLEAN +$EXPECT_SHA (same-build rule, D-2026-07-05-2)"
    done
    # Cold-cache capability is METHODOLOGY, not a nicety — hard gate, fail closed.
    hrun "$h" "sudo -n /usr/sbin/purge" \
      || die "$(hname "$h") cannot purge without a password (need the NOPASSWD /usr/sbin/purge sudoers rule) — every run would read WARM"
    hrun "$h" "pgrep -x blit-daemon >/dev/null" \
      && die "$(hname "$h"): a blit-daemon is already running — stop it first (stale-daemon refusal)"
    # Fixtures.
    local w want got
    for w in large mixed small; do
      want="$(eval echo "\$FIX_COUNT_$w")"
      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
      [[ "${got:-0}" == "$want" ]] \
        || die "$(hname "$h"): src_$w has ${got:-0}/$want files — stage the fixtures before a timed run"
    done
  done
  # Link validity, MEASURED not assumed (.agents/machines.md): the peer's ARP entry
  # must be the PEER's MAC, never our own — a host route on a directly-connected
  # subnet installs a BLACK HOLE that still reports the right interface.
  local pmac
  ping -c1 -W1 "$Q_IP" >/dev/null 2>&1 || true
  pmac="$(arp -n "$Q_IP" 2>/dev/null | awk '{print $4}')"
  [[ -n "$pmac" && "$pmac" != "(incomplete)" ]] || die "no ARP entry for q ($Q_IP) — the link is not up"
  log "preflight OK  build=$EXPECT_SHA (harness HEAD=$HARNESS_HEAD)  runs/arm=$RUNS  q_mac=$pmac"
  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
}

write_manifest() {
  local f="$OUT_DIR/staging-manifest.txt" h
  { echo "# harness_head=$HARNESS_HEAD binary_identity=$EXPECT_SHA"
    echo "host,role,sha,sha256,path"
    for h in n q; do
      echo "$(hname "$h"),client,$EXPECT_SHA,$(sha256_of "$h" "$(hblit "$h")"),$(hblit "$h")"
      echo "$(hname "$h"),daemon,$EXPECT_SHA,$(sha256_of "$h" "$(hdaemon "$h")"),$(hdaemon "$h")"
    done; } > "$f"
  log "staging manifest recorded (4 hashes)"
}

# --- daemons (both ends serve: the source daemon serves pulls, the dest daemon
#     serves pushes) --------------------------------------------------------
N_PID=""; Q_PID=""
daemon_start() {   # $1 = host
  local h="$1" cfg mod bin pid
  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"
  cfg="$mod/mm-bench.toml"
  hrun "$h" "mkdir -p '$mod'
printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg'
nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
sleep 2
pgrep -x blit-daemon | head -1" >/dev/null 2>&1 || true
  pid="$(hrun "$h" "pgrep -x blit-daemon | head -1" | nocr | tr -cd '0-9')"
  [[ -n "$pid" ]] || die "$(hname "$h"): daemon failed to start (see $(hmod "$h")/mm-daemon.log)"
  # Listening, not merely alive (the rig-W lesson: the process check passed while
  # the socket was not accepting, and the smoke died on a transport error).
  hrun "$h" "nc -z -G 3 127.0.0.1 $PORT" \
    || die "$(hname "$h"): daemon pid $pid is up but NOT listening on $PORT"
  [[ "$h" == n ]] && N_PID="$pid" || Q_PID="$pid"
  log "$(hname "$h") daemon up (pid $pid) on $(hip "$h"):$PORT"
}
daemon_stop() {   # $1 = host; PID-scoped, comm-verified, and the death is VERIFIED
  local h="$1" pid; pid="$([[ "$h" == n ]] && echo "$N_PID" || echo "$Q_PID")"
  [[ -n "$pid" ]] || return 0
  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; true" >/dev/null 2>&1 || true
  if hrun "$h" "ps -p $pid >/dev/null 2>&1"; then
    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown — port $PORT may still be held"
    return 1
  fi
  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
}
cleanup() {
  daemon_stop n || true
  daemon_stop q || true
  rm -rf "$MUX" 2>/dev/null || true
}
trap cleanup EXIT

# --- cold + drain -------------------------------------------------------------
RUN_DRAIN=""; RUN_COLD=""
drain_host() {   # $1 = DESTINATION host; wait until its disk is quiet (macOS iostat)
  hrun "$1" "quiet=0
for i in \$(seq 1 $DRAIN_ITERS); do
  w=\$(iostat -d -w 2 -c 2 disk0 2>/dev/null | tail -1 | awk '{print \$3}')
  ok=\$(awk -v w=\"\${w:-99}\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained \${i}x2s\"; exit 0; fi
done
echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
}
prep_run() {   # $1 = dest host. Drain the DEST, then cold BOTH ends. A failed purge VOIDS.
  local dh="$1" out cn=ok cq=ok
  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"; RUN_DRAIN="${RUN_DRAIN// /_}"
  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
}

# --- durability: ALWAYS the DESTINATION host, identically for both arms --------
fsync_tree_ms() {   # $1 = DEST host, $2 = landed path. Prints ms, or NA (=> VOID).
  local out
  out="$(hrun "$1" "python3 - '$2' <<'PYEOF'
import os, sys, time
t = time.monotonic()
for root, dirs, files in os.walk(sys.argv[1]):
    for name in files:
        fd = os.open(os.path.join(root, name), os.O_RDONLY)
        os.fsync(fd)
        os.close(fd)
print('F:%d:F' % int((time.monotonic() - t) * 1000))
PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p' | head -1)"
  echo "${out:-NA}"   # a failed fsync must never read as a plausible flush
}

# --- one timed run ------------------------------------------------------------
RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
timed_run() {   # $1=initiating host $2=src spec $3=dst spec $4=DEST host $5=landed path $6=flag
  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" out bin
  bin="$(hblit "$ih")"
  prep_run "$dh"
  # The window is self-timed ON the initiating host (locally for nagatha; inside a
  # SINGLE ssh for q), so dispatch/round-trip is outside it by construction.
  # NO sync in here — durability is charged to the destination, below.
  out="$(hrun "$ih" "python3 -c 'import time;print(int(time.monotonic()*1000))' > /tmp/mm_t0
'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/mm-client.err; rc=\$?
t1=\$(python3 -c 'import time;print(int(time.monotonic()*1000))'); t0=\$(cat /tmp/mm_t0)
echo \"R:\$((t1-t0)),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
  RUN_FLUSH="$(fsync_tree_ms "$dh" "$landed")"
  RUN_VALID=yes
  if [[ "$RUN_FLUSH" == NA ]]; then RUN_VALID=no; RUN_FLUSH=0; fi
  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
}

# --- arms: the ONLY variable is which host's CLI initiates --------------------
CUR_W=""; CUR_FLAG=""
arm_srcinit() {    # the SOURCE host pushes into the DEST daemon
  local cell="$1" rid="$2" sh="$3" dh="$4" landed
  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}/src_$CUR_W"
  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" \
                  "$(hip "$dh"):$PORT:/bench/${SESSION_TAG}_${cell}_${rid}/" \
                  "$dh" "$landed" "$CUR_FLAG"
  hrun "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" >/dev/null 2>&1 || true
}
arm_destinit() {   # the DEST host pulls from the SOURCE daemon
  local cell="$1" rid="$2" sh="$3" dh="$4" landed
  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}"
  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" \
                  "$landed" \
                  "$dh" "$landed" "$CUR_FLAG"
  hrun "$dh" "rm -rf '$landed'" >/dev/null 2>&1 || true
}

CSV="$OUT_DIR/runs.csv"; echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,cold,valid" > "$CSV"
META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"

run_pair_loop() {   # $1=cell $2=src host $3=dest host
  local cell="$1" sh="$2" dh="$3"
  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
  while (( valid < RUNS && attempts < max )); do
    attempts=$(( attempts + 1 ))
    local order pair=yes rowA="" rowB="" arm rid aname init
    if (( slot % 2 )); then order="A B"; else order="B A"; fi
    for arm in $order; do
      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
      rid="${aname}_s${slot}a${attempts}"
      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh"
      else arm_destinit "$cell" "$rid" "$sh" "$dh"; fi
      [[ "$RUN_VALID" == yes ]] || pair=no
      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
      [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
    done
    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
  done
  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
  else echo "$cell,$attempts,yes" >> "$META"; fi
}

compute_verdicts() {
  python3 - "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" "$OUT_DIR/paired.csv" <<'PY'
import csv, sys
runs_p, meta_p, sum_p, ver_p, pair_p = sys.argv[1:6]
rows = list(csv.DictReader(open(runs_p)))
meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
by, void = {}, {}
# PAIRED slots: the pre-registered noise model. Each ABBA slot yields a matched
# (srcinit, destinit) pair under identical conditions, so d_i = destinit - srcinit
# is a WITHIN-slot difference — no between-session drift can enter it. pf-0's
# review established that an unpaired spread is NOT a noise floor.
slots = {}
for r in rows:
    k = (r["cell"], r["arm"])
    if r["valid"] == "yes":
        by.setdefault(k, []).append(int(r["ms"]))
        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = int(r["ms"])
    else:
        void[k] = void.get(k, 0) + 1

def med(v):
    v = sorted(v); n = len(v)
    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2

def complete(c):
    if c not in meta or meta[c]["complete"] != "yes":
        return False
    arms = [a for (cc, a) in by if cc == c]
    return "srcinit" in arms and "destinit" in arms

with open(sum_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,pairs_attempted,runs\n")
    for (c, a) in sorted(by):
        if not complete(c):
            continue
        v = by[(c, a)]
        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
        # every run is printed: pf-0 found the fast arm BIMODAL, and a median
        # alone hides a mode-mixture shift that would fake a recovery.
        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s,%s\n" % (
            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
            void.get((c, a), 0), meta[c]["pairs_attempted"],
            " ".join(str(x) for x in v)))

# The paired statistics the pre-registered rule is actually graded on.
#   D = median(d_i)  -> the effect (positive = destination-initiated is slower)
#   S = spread(d_i)  -> the PAIRED noise floor (max-min; IQR also reported)
#   MDE = S          -> conservatively, the smallest |D| this cell can resolve
# DELTA_REF = 230 ms: rig W's measured Delta_P1, the effect size this rig must be
# able to see before any "vanishes" claim is permitted (the POWER GATE).
DELTA_REF = 230
with open(pair_p, "w") as f:
    f.write("cell,n_pairs,D_median_ms,S_spread_ms,IQR_ms,MDE_ms,fast_arm_ms,"
            "delta_ref_ms,ref_ratio_on_fast_arm,powered_for_null,d_i\n")
    for c in sorted(meta):
        ds = sorted(v["destinit"] - v["srcinit"]
                    for (cc, _r), v in slots.items()
                    if cc == c and "srcinit" in v and "destinit" in v)
        if not ds:
            continue
        n = len(ds)
        D = med(ds)
        S = max(ds) - min(ds)
        q1, q3 = ds[n // 4], ds[(3 * n) // 4 - (1 if n % 4 == 0 else 0)]
        fast = min(med(by[(c, "srcinit")]), med(by[(c, "destinit")])) if complete(c) else 0
        # A 230 ms effect is only VISIBLE against a ratio bar if the fast arm is
        # fast enough: at a 2.3 s fast arm, 230 ms IS exactly 1.10 and sits ON the
        # bar. So the null branch needs BOTH: MDE <= DELTA_REF, and a ref-sized
        # effect that would actually breach 1.10 here.
        ref_ratio = (fast + DELTA_REF) / fast if fast else 0.0
        powered = "yes" if (S <= DELTA_REF and 10 * (fast + DELTA_REF) > 11 * fast) else "NO"
        f.write("%s,%d,%d,%d,%d,%d,%d,%d,%.3f,%s,%s\n" % (
            c, n, D, S, q3 - q1, S, fast, DELTA_REF, ref_ratio, powered,
            " ".join(str(x) for x in ds)))

with open(ver_p, "w") as f:
    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,D_ms,S_ms,bar,outcome,powered_for_null\n")
    for c in sorted(meta):
        if not complete(c):
            f.write("%s,invariance,srcinit,destinit,,,,,,1.10,INCOMPLETE,\n" % c)
            continue
        s, d = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
        hi, lo = max(s, d), min(s, d)
        # integer-exact bar (10*hi <= 11*lo) — never the printed 3-decimal ratio
        outcome = "PASS" if 10 * hi <= 11 * lo else "FAIL"
        ds = sorted(v["destinit"] - v["srcinit"]
                    for (cc, _r), v in slots.items()
                    if cc == c and "srcinit" in v and "destinit" in v)
        D = med(ds) if ds else 0
        S = (max(ds) - min(ds)) if ds else 0
        fast = lo
        powered = "yes" if (ds and S <= DELTA_REF and 10 * (fast + DELTA_REF) > 11 * fast) else "NO"
        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,%d,1.10,%s,%s\n" % (
            c, s, d, (hi / lo) if lo else 0.0, D, S, outcome, powered))
PY
}

main() {
  preflight
  write_manifest
  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
    exit 0
  fi
  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
  daemon_start n
  daemon_start q

  local carrier w flag cell
  for w in mixed large small; do
    for carrier in tcp grpc; do
      [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
      CUR_W="$w"; CUR_FLAG="$flag"
      cell="nq_${carrier}_${w}"                       # data nagatha -> q
      want_cell "$cell" && run_pair_loop "$cell" n q
      cell="qn_${carrier}_${w}"                       # data q -> nagatha
      want_cell "$cell" && run_pair_loop "$cell" q n
    done
  done

  compute_verdicts
  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
  log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/cell; ABBA) ==="
  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
  log "=== VERDICTS (computed, NOT declared — read the pre-registered rule) ==="
  column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
  log "runs: $CSV"
}
main "$@"
