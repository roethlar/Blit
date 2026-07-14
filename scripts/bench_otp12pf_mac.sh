#!/usr/bin/env bash
# =============================================================================
# ⛔ NOT CLEARED TO RUN — REWORKED IN ROUND 3, REVIEW NOT YET PASSED ⛔
#
# The round-3 rework (this file) addresses all 15 findings from codex round 2 and
# grok's second opinion. It has NOT been reviewed. The review is the gate, not the
# rework: three rounds running, every revision of this instrument has shipped a
# defect capable of a false claim, and two of them were introduced BY THE REWORK
# THAT FIXED THE PREVIOUS ONE.
#
#   .review/results/macmac-harness-r2.gpt-verdict.md    (codex, 12 findings)
#   .review/results/macmac-harness-r2.grok-verdict.md   (grok, +3 findings)
#
# Clearing it: land the round-3 review, adjudicate, and delete this block plus the
# CLEARED_BY_REVIEW guard below. Until then `SELFTEST=1` and `PREFLIGHT_ONLY=1`
# work (they take NO data); a timed run refuses.
# =============================================================================
# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md (rev 4)
# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
#
# WHY THIS RIG EXISTS
# -------------------
# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
# cell. It answers ONE question, SCOPED TO THIS PAIR:
#
#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
#
#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
#     "platform residue" that can be waived; code-level hypotheses strengthen. It
#     leaves macOS/APFS and host x role explanations OPEN.
#   * null       -> P1 did NOT reproduce on this pair. That is CONSISTENT with
#     "Windows is required", but does NOT prove it: it could equally be a
#     property of these two machines, their disks, or this macOS version.
#
# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
# H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
# InitiatorReceivePlaneRun.add_dialed_stream, the dial-before-ACK at
# transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with H1,
# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
#
# THE INSTRUMENT IS THE RISK. Three claims in this project have been retracted to
# harness bugs, and this harness alone has now had 20 defects found across two
# reviews. What round 2 caught, and what is fixed here:
#
#   * THE TIMER WAS MEASURING FSYNC NOISE. It captured time.monotonic() in TWO
#     separate `python3 -c` processes and subtracted them. On macOS that clock is
#     PROCESS-RELATIVE: a 1000 ms sleep measured -1 ms on nagatha and 2 ms on q
#     (measured; yes, negative). Every `ms` row would have been ~= fsync_ms alone,
#     and the invariance ratio — THE ENTIRE MEASURAND — would have been computed on
#     fsync noise, which can manufacture or mask a one-directional effect at will.
#     The repo ALREADY documents this trap (bench_otp12_zoey.sh:116 uses time.time()
#     precisely because monotonic is wrong across processes) and I reintroduced it
#     anyway. Now: ONE process times itself and spawns the client (time_argv), and
#     PREFLIGHT PROVES IT on both hosts against a known sleep before any data.
#   * The preflight COULD NOT SUCCEED: `grep -c` exits 1 on no match, so a CLEAN
#     binary tripped the dirty-marker probe and died; and norm_mac used gawk's
#     strtonum(), absent from stock macOS awk. The round-1 "fixes" were never
#     executed — I ran `bash -n`, not the gates. Every gate below is now exercised
#     by SELFTEST=1, which runs them for real.
#   * Gates FAILED OPEN: pgrep errors read as "quiet"; a failed `top` read as 0%
#     CPU and a late idle sample could overwrite a busy one; non-numeric `iostat`
#     read as zero and CERTIFIED drainage; the drain watched a hardcoded `disk0`
#     that the data need never touch (grok); `die` inside $(...) exited only the
#     subshell, so an empty hash still landed. Every probe is now sentinel-framed,
#     rc-aware, and fails CLOSED.
#
# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
# one ssh), so dispatch is outside the window by construction.
#
# Usage:
#   SELFTEST=1       bash scripts/bench_otp12pf_mac.sh   # exercise every gate, no data
#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh # the run (needs review clearance)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SELF="${BASH_SOURCE[0]}"
VERDICT_PY="$SCRIPT_DIR/otp12pf_mac_verdict.py"
VERDICT_TEST="$SCRIPT_DIR/otp12pf_mac_verdict_test.py"

SELFTEST="${SELFTEST:-0}"
PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"

# The review is the gate. A timed run refuses until round 3 is adjudicated; the
# no-data modes stay available so the gates can be exercised.
if [[ "$SELFTEST" != 1 && "$PREFLIGHT_ONLY" != 1 && "${CLEARED_BY_REVIEW:-0}" != 1 ]]; then
  echo "REFUSING: this harness was reworked in round 3 and has NOT passed review." >&2
  echo "Every previous revision shipped a defect capable of a false claim, and two" >&2
  echo "were introduced by the rework that fixed the last one. Land the round-3" >&2
  echo "review first. SELFTEST=1 and PREFLIGHT_ONLY=1 take no data and still run." >&2
  exit 2
fi

# The pre-registered build. Not overridable by accident: a run against an
# unregistered build is not the registered experiment.
REGISTERED_BUILD="f35702a"

# --- nagatha: LOCAL end (driver) ---------------------------------------------
N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
N_NIC="${N_NIC:-en11}"
N_MAC="${N_MAC:-00:e0:4d:01:4c:a3}"              # nagatha's OWN en11 MAC (measured)
N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"
N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"

# --- q: REMOTE end ------------------------------------------------------------
Q_SSH="${Q_SSH:-michael@q}"
Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
Q_NIC="${Q_NIC:-en8}"
Q_MAC="${Q_MAC:-00:01:d2:19:04:a3}"              # q's OWN en8 MAC (measured)
Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"

PORT="${PORT:-9031}"
RUNS="${RUNS:-8}"
SETTLE_MS="${SETTLE_MS:-250}"     # equal pre-fsync window on BOTH arms
LOAD_MAX="${LOAD_MAX:-3.0}"
DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
DRAIN_MBPS="${DRAIN_MBPS:-2}"
DELTA_REF_MS="${DELTA_REF_MS:-230}"   # rig W's measured Delta_P1 (the reference effect)
TIMER_TOLERANCE_MS="${TIMER_TOLERANCE_MS:-120}"  # the timer self-test's allowed error

# The REGISTERED cell set. The verdict engine requires ALL of them present and
# complete: a partial set that is merely filtered lets a ONE-CELL run emit
# "VANISHES" while claiming both cells vanished (codex r2 BLOCKER 1).
REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"
CELLS="$REGISTERED_CELLS"

SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"

MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }

mkdir -p "$OUT_DIR/blit-logs"
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
die() { log "FATAL: $*"; exit 1; }
nocr() { tr -d '\r'; }

# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
# falls through to the wrong host (the trap the Linux harness documents).
# `bash -c` locally pins the inner shell so local and remote parse identically.
# pipefail is set in BOTH children: without it a failed probe at the head of a
# pipeline is masked by a successful `tail`/`awk` and the gate reads "fine".
hrun() {
  local h="$1"; shift
  local cmd="set -o pipefail
$*"
  if [[ "$h" == n ]]; then bash -c "$cmd"; else qssh "bash -c $(printf '%q' "$cmd")"; fi
}
hblit()   { if [[ "$1" == n ]]; then echo "$N_BLIT";   else echo "$Q_BLIT";   fi; }
hdaemon() { if [[ "$1" == n ]]; then echo "$N_DAEMON"; else echo "$Q_DAEMON"; fi; }
hmod()    { if [[ "$1" == n ]]; then echo "$N_MODULE"; else echo "$Q_MODULE"; fi; }
hip()     { if [[ "$1" == n ]]; then echo "$N_IP";     else echo "$Q_IP";     fi; }
hnic()    { if [[ "$1" == n ]]; then echo "$N_NIC";    else echo "$Q_NIC";    fi; }
hmac()    { if [[ "$1" == n ]]; then echo "$N_MAC";    else echo "$Q_MAC";    fi; }
hname()   { if [[ "$1" == n ]]; then echo nagatha;     else echo q;           fi; }
other()   { if [[ "$1" == n ]]; then echo q;           else echo n;           fi; }

# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
FIX_COUNT_small=10000; FIX_BYTES_small=40960000
FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
fix_count() { case "$1" in large) echo $FIX_COUNT_large;; mixed) echo $FIX_COUNT_mixed;; small) echo $FIX_COUNT_small;; esac; }
fix_bytes() { case "$1" in large) echo $FIX_BYTES_large;; mixed) echo $FIX_BYTES_mixed;; small) echo $FIX_BYTES_small;; esac; }

# =============================================================================
# THE TIMER. One process times itself AND spawns the client, so the interval is
# measured by a single clock and python's startup cost falls outside it.
#
# NEVER bracket a command with two separate `python3 -c 'time.monotonic()'` calls:
# on macOS that clock is PROCESS-RELATIVE and the difference is garbage (measured:
# -1 ms and 2 ms for a 1000 ms sleep). bench_otp12_zoey.sh:116 already said so.
# =============================================================================
time_argv() {   # $1 = host; rest = argv. Echoes "MS,RC" or "" on a broken probe.
  local h="$1"; shift
  local qa="" a
  for a in "$@"; do qa="$qa $(printf '%q' "$a")"; done
  hrun "$h" "python3 - $qa <<'PYEOF'
import subprocess, sys, time
argv = [a for a in sys.argv[1:] if a]          # an empty flag must not become argv
err = open('/tmp/mm-client.err', 'wb')
t = time.monotonic()
rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=err)
ms = int((time.monotonic() - t) * 1000)
err.close()
print('R:%d,%d:R' % (ms, rc))
PYEOF" | nocr | sed -n 's/.*R:\(-\{0,1\}[0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1
}

# The gate that makes the timer bug unshippable: prove the clock on the rig,
# against a known interval, before any data is taken.
timer_gate() {
  local h="$1" out ms rc lo hi
  out="$(time_argv "$h" /bin/sleep 1)"
  [[ "$out" == *,* ]] || die "$(hname "$h"): the timer probe returned nothing — refusing"
  ms="${out%%,*}"; rc="${out##*,}"
  [[ "$rc" == 0 ]] || die "$(hname "$h"): the timer probe's own child exited $rc"
  lo=$(( 1000 - TIMER_TOLERANCE_MS )); hi=$(( 1000 + TIMER_TOLERANCE_MS ))
  if (( ms < lo || ms > hi )); then
    die "$(hname "$h"): THE TIMER IS LYING — a 1000 ms sleep measured ${ms} ms (allowed ${lo}-${hi}).
This is the round-2 killer: cross-process time.monotonic() on macOS is PROCESS-RELATIVE and
read -1 ms / 2 ms for this exact sleep. Every row would be fsync noise. REFUSING to take data."
  fi
  log "  timer ok on $(hname "$h"): a 1000 ms sleep measures ${ms} ms"
}

# --- provenance ---------------------------------------------------------------
# `die` inside $(...) exits only the SUBSHELL, so the outer command substitution
# succeeds with an empty value. These return non-zero instead and the CALLER dies.
embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
  local h="$1" p="$2" raw hit dirty
  # `grep -c` exits 1 on NO MATCH, which is not an error. Only rc>=2 is. The old
  # `|| echo X` turned a clean binary's legitimate "0" into "0\nX" and DIED.
  raw="$(hrun "$h" "c=\$(grep -c -a -- '+$EXPECT_SHA' '$p'); rc=\$?
d=\$(grep -c -a -- '+$EXPECT_SHA.dirty' '$p'); rd=\$?
if [ \$rc -ge 2 ] || [ \$rd -ge 2 ]; then echo 'E:ERR:E'; else echo \"E:\$c:\$d:E\"; fi" \
    | nocr | sed -n 's/.*E:\([0-9]*\):\([0-9]*\):E.*/\1 \2/p' | head -1)" || return 1
  [[ -n "$raw" ]] || return 1
  read -r hit dirty <<<"$raw"
  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
}
sha256_of() {      # returns non-zero on a short/empty hash; the CALLER must `|| die`
  local h="$1" p="$2" v
  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')" || return 1
  [[ ${#v} -eq 64 ]] || return 1
  echo "$v"
}

# --- gates: every one fails CLOSED --------------------------------------------
# Stock macOS awk has no strtonum() (that is gawk). Hand-rolled hex, so the ARP
# comparison actually runs instead of erroring out.
norm_mac() {
  awk -F: '
    function hex(s,   i,c,d,v) {
      v = 0; s = tolower(s)
      for (i = 1; i <= length(s); i++) {
        c = substr(s, i, 1); d = index("0123456789abcdef", c) - 1
        if (d < 0) return -1
        v = v * 16 + d
      }
      return v
    }
    {
      if (NF != 6) { print ""; next }
      out = ""; ok = 1
      for (i = 1; i <= NF; i++) {
        v = hex($i)
        if (v < 0 || v > 255) { ok = 0; break }
        out = out sprintf("%s%02x", (i > 1 ? ":" : ""), v)
      }
      print (ok ? out : "")
    }'
}

quiescence_gate() {
  local h="$1" raw busy
  # pgrep: 0 = found, 1 = none, >=2 = ERROR. The old probe could not tell an error
  # from "quiet" — a gate that cannot answer must never answer "fine".
  raw="$(hrun "$h" 'busy=""
for p in codex cargo rustc; do
  pgrep -x "$p" >/dev/null 2>&1; rc=$?
  if [ $rc -eq 0 ]; then busy="$busy $p"
  elif [ $rc -ne 1 ]; then echo "Q:PROBE-ERROR($p=$rc):Q"; exit 0
  fi
done
echo "Q:OK:${busy# }:Q"' | nocr | sed -n 's/.*Q:OK:\(.*\):Q.*/BUSY=\1/p;s/.*Q:\(PROBE-ERROR[^:]*\):Q.*/ERR=\1/p' | head -1)" \
    || die "$(hname "$h"): quiescence probe FAILED to execute"
  [[ "$raw" == BUSY=* ]] || die "$(hname "$h"): quiescence probe did not answer ('$raw') — refusing"
  busy="${raw#BUSY=}"
  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
}

timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
  local h="$1" running auto
  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
  [[ "$running" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1" | nocr | tr -cd '0-9')" || auto=""
  [[ "$auto" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
}

spotlight_gate() {
  local h="$1" cpu
  # The MAX across samples, not the last: a late idle sample could overwrite an
  # earlier busy one. NR==0 (top produced nothing) is an ERROR, not 0% CPU.
  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null \
    | awk '/^mds_stores/{ if (\$2+0 > m) m = \$2+0 } END{ if (NR == 0) print \"ERR\"; else printf \"%d\", m+0 }'" | nocr)" || cpu="ERR"
  [[ "$cpu" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
}

load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
load_gate() {
  local h="$1" l ok
  l="$(load1 "$h")" || l=""
  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die "$(hname "$h"): cannot read load1 (got '$l') — refusing"
  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
}

link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
  local h="$1" o peer_ip want got route_nic nic
  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"; nic="$(hnic "$h")"
  [[ -n "$want" ]] || die "$(hname "$o"): its configured MAC does not parse — refusing"
  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
  # The ARP entry ON THE NIC THE TRAFFIC WILL EGRESS. `arp -n <ip>` prints one line
  # PER INTERFACE that has an entry — q holds entries for nagatha on en0, en1 AND
  # en8 — so an unfiltered $4 yields a MULTI-LINE string that can never equal a
  # single MAC. (Measured: this refused a perfectly good link. It is also the more
  # correct check: a stale entry on the 1GbE NIC is irrelevant to the 10GbE path.)
  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk -v nic='$nic' '\$5 == \"on\" && \$6 == nic {print \$4}' | head -1" | nocr | norm_mac)"
  [[ -n "$got" ]] || die "$(hname "$h"): no ARP entry for $peer_ip ON $nic — the 10GbE path has not resolved the peer"
  [[ "$got" == "$want" ]] \
    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
  [[ "$route_nic" == "$(hnic "$h")" ]] \
    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
}

# --- the drain device: RESOLVED, never hardcoded (grok) ------------------------
# `iostat disk0` can certify a disk the data never touched. Worse, on APFS the
# volume lives on a SYNTHESIZED disk whose stats may be empty while the physical
# store is saturated — a false "quiet". Resolve the module path to its PHYSICAL
# store and verify iostat actually reports it.
N_DISK=""; Q_DISK=""
hdisk() { if [[ "$1" == n ]]; then echo "$N_DISK"; else echo "$Q_DISK"; fi; }
resolve_disk() {
  local h="$1" p dev
  p="$(hmod "$h")"
  dev="$(hrun "$h" "d=\$(df '$p' 2>/dev/null | awk 'NR==2{print \$1}' | sed 's|^/dev/||')
[ -n \"\$d\" ] || { echo 'D:NONE:D'; exit 0; }
ps=\$(diskutil info \"\$d\" 2>/dev/null | awk -F: '/APFS Physical Store/{gsub(/[ \t]/, \"\", \$2); print \$2}' | head -1)
[ -n \"\$ps\" ] && d=\"\$ps\"
echo \"D:\$(echo \"\$d\" | sed -E 's/s[0-9]+\$//'):D\"" | nocr | sed -n 's/.*D:\([^:]*\):D.*/\1/p' | head -1)"
  [[ "$dev" =~ ^disk[0-9]+$ ]] || die "$(hname "$h"): cannot resolve the physical disk behind $p (got '$dev') — a drain that watches the wrong device certifies a disk the data never touched"
  # It must actually REPORT: an iostat that emits nothing for this device would
  # make every sample non-numeric, and the drain must never read that as quiet.
  local probe
  probe="$(hrun "$h" "iostat -d -w 1 -c 2 '$dev' 2>/dev/null | tail -1 | awk '{print \$3}'" | nocr)" || probe=""
  [[ "$probe" =~ ^[0-9]+\.?[0-9]*$ ]] \
    || die "$(hname "$h"): iostat does not report numeric throughput for $dev (got '$probe') — refusing"
  if [[ "$h" == n ]]; then N_DISK="$dev"; else Q_DISK="$dev"; fi
  log "  drain device on $(hname "$h"): $dev (backs $p, idle probe ${probe} MB/s)"
}

# --- the settle-gap bound (NOT a removal — a measured bound) -------------------
# Between the client exiting and the fsync starting, the OS writes back dirty pages
# FOR FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
# BY DIRECTION. SETTLE_MS makes the window EQUAL on both arms; the residual is the
# ssh return-path difference, which is bounded by the round-trip time measured here.
# It is NOT "removed by construction", and the pre-registration no longer says so.
#
# Timed in ONE process, for the same reason the transfer is. Bracketing each ssh
# with two `python3 -c time.time()` calls would have charged it TWO interpreter
# startups (~30 ms) and reported them as network latency — measured: it read 35 ms
# for a round trip that is actually ~5 ms. The instrument's own bound would have
# been wrong by 7x, in the direction that flatters nothing and confuses everything.
SSH_RTT_MS=0
measure_ssh_rtt() {
  SSH_RTT_MS="$(python3 -c '
import statistics, subprocess, sys, time
argv = sys.argv[1:]
ts = []
for _ in range(5):
    t = time.monotonic()
    subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    ts.append((time.monotonic() - t) * 1000.0)
print(int(statistics.median(ts)))
' ssh "${SSH_MUX[@]}" "$Q_SSH" true)"
  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die "cannot measure the ssh round trip — refusing"
  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms — this BOUNDS the residual settle-gap asymmetry (the settle itself is ${SETTLE_MS} ms, EQUAL on both arms)"
}

# =============================================================================
preflight() {
  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
  [[ "$EXPECT_SHA" == "$REGISTERED_BUILD" ]] \
    || die "EXPECT_SHA='$EXPECT_SHA' but the PRE-REGISTERED build is $REGISTERED_BUILD — a run against another build is not the registered experiment"
  # The instrument must be the REVIEWED instrument: a modified harness must not be
  # able to claim the reviewed commit.
  git -C "$REPO_ROOT" diff --quiet HEAD -- "$SELF" "$VERDICT_PY" "$VERDICT_TEST" \
    || die "the instrument has UNCOMMITTED changes (harness/verdict/test) — it cannot claim the reviewed commit. Commit or stash first."
  # The decision rule proves itself before it grades anything.
  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
    || die "the verdict engine's OWN guard test FAILS (see $OUT_DIR/verdict-guard-test.txt) — the decision rule is broken; refusing to take data"
  log "verdict-engine guard test passed ($(grep -c ' ok$' "$OUT_DIR/verdict-guard-test.txt" || true) cases)"

  local h p w want got wantb gotb
  for h in n q; do
    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
    timer_gate "$h"                       # THE measurand's clock, proved on the rig
    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
    done
    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
    if hrun "$h" "pgrep -x blit-daemon >/dev/null 2>&1"; then
      die "$(hname "$h"): a blit-daemon is already running — stop it first"
    fi
    for w in large mixed small; do
      want="$(fix_count "$w")"; wantb="$(fix_bytes "$w")"
      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
    done
    link_gate "$h"
    resolve_disk "$h"
  done
  measure_ssh_rtt
  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
}

write_manifest() {
  local f="$OUT_DIR/staging-manifest.txt" h nb nd qb qd vh th
  # Hashes computed FIRST, in the caller's shell: `die` inside $(...) exits only the
  # subshell, so the old code wrote an EMPTY hash and called it provenance.
  nb="$(sha256_of n "$N_BLIT")"   || die "nagatha: cannot hash $N_BLIT"
  nd="$(sha256_of n "$N_DAEMON")" || die "nagatha: cannot hash $N_DAEMON"
  qb="$(sha256_of q "$Q_BLIT")"   || die "q: cannot hash $Q_BLIT"
  qd="$(sha256_of q "$Q_DAEMON")" || die "q: cannot hash $Q_DAEMON"
  vh="$(shasum -a 256 "$VERDICT_PY" | cut -d' ' -f1)"
  th="$(shasum -a 256 "$VERDICT_TEST" | cut -d' ' -f1)"
  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
    echo "# verdict_sha256=$vh verdict_test_sha256=$th"   # the engine grades separately: hash it too
    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
    echo "# drain_disk_nagatha=$N_DISK drain_disk_q=$Q_DISK ssh_rtt_ms=$SSH_RTT_MS"
    echo "# cells=$CELLS"
    echo "host,role,sha,sha256,path"
    echo "nagatha,client,$EXPECT_SHA,$nb,$N_BLIT"
    echo "nagatha,daemon,$EXPECT_SHA,$nd,$N_DAEMON"
    echo "q,client,$EXPECT_SHA,$qb,$Q_BLIT"
    echo "q,daemon,$EXPECT_SHA,$qd,$Q_DAEMON"; } > "$f"
  log "staging manifest recorded (harness + verdict-engine + 4 binary hashes, every threshold)"
}

# --- daemons ------------------------------------------------------------------
N_PID=""; Q_PID=""; TEARDOWN_FAILED=0
daemon_start() {
  local h="$1" cfg mod bin pid
  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
  # The daemon's OWN pid, from $! — not `pgrep | head -1`, which picks the first of
  # whatever happens to be running.
  pid="$(hrun "$h" "mkdir -p '$mod' || exit 1
printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg' || exit 1
nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
echo \"P:\$!:P\"" | nocr | sed -n 's/.*P:\([0-9][0-9]*\):P.*/\1/p' | head -1)"
  [[ "$pid" =~ ^[0-9]+$ ]] || die "$(hname "$h"): daemon did not report a pid (see $mod/mm-daemon.log)"
  sleep 2
  hrun "$h" "ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon" \
    || die "$(hname "$h"): daemon pid $pid is not alive (see $mod/mm-daemon.log)"
  # ALIVE is not SERVING: it must hold the port we are about to measure through.
  hrun "$h" "lsof -nP -a -p $pid -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1" \
    || die "$(hname "$h"): daemon pid $pid is NOT LISTENING on $PORT (see $mod/mm-daemon.log)"
  if [[ "$h" == n ]]; then N_PID="$pid"; else Q_PID="$pid"; fi
  log "$(hname "$h") daemon up (pid $pid, listening) on $(hip "$h"):$PORT"
}
# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
# handshake reached some listener's backlog — not that the daemon speaks blit).
smoke() {
  local h="$1" o
  o="$(other "$h")"
  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 \
    || die "$(hname "$o"): cannot stage the smoke fixture"
  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
  log "smoke ok: $(hname "$h") daemon serves blit"
}
daemon_stop() {
  local h="$1" pid state
  if [[ "$h" == n ]]; then pid="$N_PID"; else pid="$Q_PID"; fi
  [[ -n "$pid" ]] || return 0
  hrun "$h" "kill $pid 2>/dev/null || true
for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done
if ps -p $pid >/dev/null 2>&1; then kill -9 $pid 2>/dev/null || true; sleep 1; fi" >/dev/null 2>&1 || true
  # A teardown that cannot be VERIFIED is a failure, not a success. The old probe
  # called a FAILED ssh "GONE".
  state="$(hrun "$h" "if ps -p $pid >/dev/null 2>&1; then echo 'S:ALIVE:S'; else echo 'S:GONE:S'; fi" \
    | nocr | sed -n 's/.*S:\([A-Z]*\):S.*/\1/p' | head -1)" || state=""
  if [[ "$state" != GONE ]]; then
    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown or could not be probed (got '$state') — port $PORT may still be held"
    TEARDOWN_FAILED=1
    touch "$OUT_DIR/TEARDOWN-FAILED"
    return 1
  fi
  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
}
cleanup() {
  daemon_stop n || true
  daemon_stop q || true
  rm -rf "$MUX" 2>/dev/null || true
  if [[ "$TEARDOWN_FAILED" == 1 ]]; then
    log "ERROR: a daemon survived teardown — see $OUT_DIR/TEARDOWN-FAILED. Clean it up before the next session."
  fi
}
trap cleanup EXIT

# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
RUN_DRAIN=""; RUN_COLD=""
drain_host() {   # $1 = host. Echoes drained_<n>x2s | DRAIN-TIMEOUT | DRAIN-ERROR
  local h="$1" dev
  dev="$(hdisk "$h")"
  [[ -n "$dev" ]] || { echo DRAIN-ERROR; return 0; }
  hrun "$h" "quiet=0
for i in \$(seq 1 $DRAIN_ITERS); do
  w=\$(iostat -d -w 2 -c 2 '$dev' 2>/dev/null | tail -1 | awk '{print \$3}')
  case \"\$w\" in
    ''|*[!0-9.]*) echo DRAIN-ERROR; exit 0 ;;   # non-numeric must NEVER certify quiet
  esac
  ok=\$(awk -v w=\"\$w\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
done
echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
}
prep_run() {   # $1 = dest host
  local dh="$1" cn=ok cq=ok out
  # Purge BOTH ends first — the purge itself dirties the disk, so a drain certified
  # BEFORE it proves nothing.
  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
}

# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0
fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes" or "NA 0 0"
  local out
  out="$(hrun "$1" "sleep $(awk -v m="$SETTLE_MS" 'BEGIN{printf \"%.3f\", m/1000}')
python3 - '$2' <<'PYEOF'
import os, sys, time
p = sys.argv[1]
if not os.path.isdir(p):
    print('F:NA:0:0:F')          # a MISSING tree must never read as a fast flush
    raise SystemExit
t = time.monotonic()             # ONE process: this interval is measured by one clock
files = 0
nbytes = 0
for root, _d, fs in os.walk(p):
    for name in fs:
        fp = os.path.join(root, name)
        nbytes += os.path.getsize(fp)
        fd = os.open(fp, os.O_RDONLY)
        os.fsync(fd)
        os.close(fd)
        files += 1
print('F:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes))
PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3/p' | head -1)" || out=""
  echo "${out:-NA 0 0}"
}

# --- one timed run ------------------------------------------------------------
RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin wc wb
  bin="$(hblit "$ih")"
  prep_run "$dh"
  out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
  read -r RUN_FLUSH RUN_FILES RUN_BYTES <<<"$(fsync_tree "$dh" "$landed")"
  RUN_VALID=yes
  wc="$(fix_count "$w")"; wb="$(fix_bytes "$w")"
  if [[ "$RUN_FLUSH" == NA ]]; then
    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
    RUN_VALID=no; RUN_FLUSH=0
  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
    RUN_VALID=no
  fi
  # A negative or absurd transfer time means the CLOCK failed, not that the transfer
  # was fast. It must never enter the data.
  if [[ ! "$RUN_MS" =~ ^[0-9]+$ ]] || (( RUN_MS < 1 )); then
    log "  VOID: transfer timer returned '$RUN_MS' — the clock failed (round 2's killer). NOT a fast run."
    RUN_VALID=no; RUN_MS=0
  fi
  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
}

# --- arms ---------------------------------------------------------------------
# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
# above is what makes a wrong path fatal instead of silently free.
CUR_W=""; CUR_FLAG=""
arm_srcinit() {
  local sh="$1" dh="$2" run="$3"
  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
}
arm_destinit() {
  local sh="$1" dh="$2" run="$3"
  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
}

CSV="$OUT_DIR/runs.csv"
META="$OUT_DIR/meta.csv"

run_pair_loop() {
  local cell="$1" sh="$2" dh="$3"
  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
  while (( valid < RUNS && attempts < max )); do
    attempts=$(( attempts + 1 ))
    local order pair=yes rowA="" rowB="" arm aname init rid run
    if (( slot % 2 )); then order="A B"; else order="B A"; fi
    for arm in $order; do
      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
      rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
      if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"
      else arm_destinit "$sh" "$dh" "$run"; fi
      [[ "$RUN_VALID" == yes ]] || pair=no
      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
      if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
    done
    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
  done
  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
  else echo "$cell,$attempts,yes" >> "$META"; fi
}

compute_verdicts() {
  DELTA_REF_MS="$DELTA_REF_MS" VERDICT_CELLS="$VERDICT_CELLS" \
  CONTROL_CELLS="$CONTROL_CELLS" REGISTERED_CELLS="$REGISTERED_CELLS" \
  python3 "$VERDICT_PY" \
    "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/paired.csv" \
    "$OUT_DIR/verdicts.csv" "$OUT_DIR/session_verdict.txt"
}

# =============================================================================
# SELFTEST — exercise every gate for real, take NO data.
#
# This exists because round 1's "fixes" were never executed: I ran `bash -n` and
# shipped a preflight that COULD NOT SUCCEED (grep -c's exit 1, gawk's strtonum).
# A syntax check is not an execution.
# =============================================================================
SELFTEST_FIRED=0
# Each gate runs in a SUBSHELL so its `die` cannot abort the sweep. What the
# selftest proves is that a gate EXECUTES and ANSWERS — a gate that fires because
# the rig is genuinely dirty (a codex session running, Time Machine enabled) has
# passed this test, not failed it. No bypass is added to the real path: preflight
# still dies on the first refusal.
gate_probe() {
  local label="$1"; shift
  if ( "$@" ); then
    log "  [OK]    $label — answers, and the condition holds"
  else
    SELFTEST_FIRED=$(( SELFTEST_FIRED + 1 ))
    log "  [FIRED] $label — the gate REFUSED (reason in the FATAL line above). It executes and fails CLOSED, which is what this proves."
  fi
}
selftest() {
  local h
  log "SELFTEST — running every gate for real. No daemon, no transfer, no data."
  log "instrument: harness=$HARNESS_SHA256"
  log "--- the verdict engine's own guard test (incl. mutation proof) ---"
  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
    || die "the verdict guard test FAILS (see $OUT_DIR/verdict-guard-test.txt)"
  log "  $(grep -E '^[0-9]+/[0-9]+ cases passed' "$OUT_DIR/verdict-guard-test.txt")"
  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt)"
  log "  $(grep -E '^[0-9]+/[0-9]+ mutations killed' "$OUT_DIR/verdict-mutations.txt") — every reverted fix is caught"
  for h in n q; do
    log "--- $(hname "$h") ---"
    gate_probe "timer        (the measurand's clock)" timer_gate "$h"
    gate_probe "quiescence   (codex/cargo/rustc)"     quiescence_gate "$h"
    gate_probe "time machine (running OR enabled)"    timemachine_gate "$h"
    gate_probe "spotlight    (mds_stores CPU)"        spotlight_gate "$h"
    gate_probe "load         (load1 <= $LOAD_MAX)"      load_gate "$h"
    gate_probe "link         (ARP + 10GbE route)"     link_gate "$h"
    gate_probe "drain device (resolved, not disk0)"   resolve_disk "$h"
    log "  [--]    mac parse (no gawk strtonum): $(hmac "$h") -> $(hmac "$h" | norm_mac)"
    log "  [--]    load1=$(load1 "$h")"
  done
  measure_ssh_rtt
  log "SELFTEST COMPLETE — every gate executed. $SELFTEST_FIRED gate(s) refused (see above)."
  log "This is NOT clearance to take data: the round-3 review is."
}

main() {
  if [[ "$SELFTEST" == 1 ]]; then
    EXPECT_SHA="${EXPECT_SHA:-$REGISTERED_BUILD}"
    HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
    HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
    selftest
    exit 0
  fi
  preflight
  write_manifest
  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
    exit 0
  fi
  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
  echo "cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
  echo "cell,pairs_attempted,complete" > "$META"
  daemon_start n; daemon_start q
  smoke n; smoke q

  local carrier w flag cell
  for w in mixed large small; do
    for carrier in tcp grpc; do
      if [[ "$carrier" == grpc ]]; then flag="--force-grpc"; else flag=""; fi
      CUR_W="$w"; CUR_FLAG="$flag"
      cell="nq_${carrier}_${w}"; if [[ ",$CELLS," == *",$cell,"* ]]; then run_pair_loop "$cell" n q; fi
      cell="qn_${carrier}_${w}"; if [[ ",$CELLS," == *",$cell,"* ]]; then run_pair_loop "$cell" q n; fi
    done
  done

  # End-load BEFORE the verdict is computed: it is a condition OF the session, and
  # a session whose end-load is only known afterwards cannot void on it.
  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
  compute_verdicts
  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
  log "=== PAIRED STATS (the rule is graded on these) ==="
  column -t -s, "$OUT_DIR/paired.csv" | tee -a "$OUT_DIR/bench.log"
  log "=== SESSION VERDICT (computed by the harness from the PRE-REGISTERED rule) ==="
  cat "$OUT_DIR/session_verdict.txt" | tee -a "$OUT_DIR/bench.log"
  log "runs: $CSV"
}

# EXPECT_SHA is required for anything that touches the rig's binaries; SELFTEST
# supplies the registered default so the gates can be exercised without ceremony.
if [[ "$SELFTEST" != 1 ]]; then
  EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a)}"
  HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
  HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
fi
main "$@"
