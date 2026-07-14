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
SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
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

# =============================================================================
# THE REGISTERED CONSTANTS. **NOT OVERRIDABLE.**
#
# Round-5 (codex, BLOCKER): these were `${VAR:-default}`, so the pre-registered
# decision rule could be edited FROM THE COMMAND LINE — `DELTA_REF_MS=240` turned a
# RIG-VOID into a VANISHES. A pre-registration that the operator can retune, after
# the data exists, in the direction of the answer they want, IS NOT A
# PRE-REGISTRATION AT ALL.
#
# They are literals, and the harness REFUSES to start if one is merely PRESENT in the
# environment — a deviation must be loud, never silently ignored. The check reads the
# environment BEFORE the assignments below, or an override would be masked by the
# very line meant to pin it.
# =============================================================================
_overrides=""
for _v in SETTLE_MS LOAD_MAX DRAIN_ITERS DRAIN_QUIET DRAIN_MBPS DELTA_REF_MS TIMER_TOLERANCE_MS; do
  [[ -n "${!_v+set}" ]] && _overrides="$_overrides $_v=${!_v}"
done
if [[ -n "$_overrides" ]]; then
  echo "REFUSING: the pre-registered constants are NOT tunable, and these are set in the" >&2
  echo "environment:$_overrides" >&2
  echo "A rule the operator can retune after seeing the data is not a pre-registration." >&2
  echo "To change one, amend docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md and" >&2
  echo "put it back through review. That is the entire point of the document." >&2
  exit 2
fi

SETTLE_MS=250              # equal pre-fsync window on BOTH arms
# Computed ONCE, HERE, at top level — and this line is load-bearing history.
#
# It used to be computed inline as `sleep $(awk ... 'BEGIN{printf \"%.3f\", m/1000}')`
# INSIDE the double-quoted hrun string. A command substitution is parsed FRESH by
# bash, so those `\"` escapes — which are correct for hrun's two-level strings — were
# literal backslashes to awk. **The awk errored on EVERY call, `sleep` got an empty
# argument and FAILED, and the old code ignored its exit status because the python
# walk that followed supplied the status.**
#
# So THE SETTLE HAS NEVER RUN — not once, in any revision, since 24660ae introduced
# it. And 24660ae is the commit that added it TO FIX the free-writeback asymmetry
# that reverses sign with direction — the artifact judged capable of MANUFACTURING a
# one-directional P1 out of nothing. The pre-registration has claimed an equal settle
# on both arms through revisions 3, 4 and 5. It was never applied.
#
# Found only by EXECUTING it (round-5 codex flagged the ignored exit status; running
# it showed the status was ALWAYS failure). `bash -n` sees nothing here.
SETTLE_SEC="$(awk -v m="$SETTLE_MS" 'BEGIN{printf "%.3f", m/1000}')"
[[ "$SETTLE_SEC" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "FATAL: settle seconds did not compute ('$SETTLE_SEC')" >&2; exit 1; }
LOAD_MAX=3.0               # start AND end load1 bar on both Macs
DRAIN_ITERS=60
DRAIN_QUIET=3
DRAIN_MBPS=2               # destination disk must be below this to start a window
DELTA_REF_MS=230           # rig W's measured Delta_P1 — THE reference effect
TIMER_TOLERANCE_MS=120     # the timer self-test's allowed error on a 1000 ms sleep

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
# A gate that CANNOT ANSWER is BLIND, and blindness is what fails open on the night.
# It is marked EXPLICITLY here, never inferred from the wording of a message —
# inferring it from prose is how a blind timer came to be scored as a working gate.
die_blind() { log "FATAL[PROBE-BLIND]: $*"; exit 1; }
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
  hrun "$h" "$(hpy "$h") - $qa <<'PYEOF'
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
  [[ "$out" == *,* ]] || die_blind "$(hname "$h"): the timer probe returned nothing — refusing"
  ms="${out%%,*}"; rc="${out##*,}"
  [[ "$rc" == 0 ]] || die_blind "$(hname "$h"): the timer probe's own child exited $rc"
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

# THE ONLY process probe in this harness. pgrep: 0 = found, 1 = none, >=2 = ERROR.
# Echoes RUNNING | NONE | BROKEN. A probe that cannot answer must NEVER answer "fine",
# and there must be exactly ONE of these -- round 5 found the fail-open surviving in a
# duplicate site precisely because there were two.
pgrep_state() {
  local h="$1" pat="$2" raw
  raw="$(hrun "$h" "pgrep -x '$pat' >/dev/null 2>&1; rc=\$?
if [ \$rc -eq 0 ]; then echo 'G:RUNNING:G'
elif [ \$rc -eq 1 ]; then echo 'G:NONE:G'
else echo 'G:BROKEN:G'; fi" | nocr | sed -n 's/.*G:\([A-Z]*\):G.*/\1/p' | head -1)" || raw=""
  case "$raw" in
    RUNNING|NONE|BROKEN) echo "$raw" ;;
    *)                   echo BROKEN ;;   # no sentinel back == a broken probe
  esac
}

quiescence_gate() {
  local h="$1" p busy=""
  for p in codex cargo rustc; do
    case "$(pgrep_state "$h" "$p")" in
      RUNNING) busy="$busy $p" ;;
      NONE)    : ;;
      *)       die_blind "$(hname "$h"): the quiescence probe for '$p' BROKE — refusing (a gate that cannot answer must not answer 'fine')" ;;
    esac
  done
  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running:$busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
}

timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
  local h="$1" running auto
  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
  [[ "$running" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1" | nocr | tr -cd '0-9')" || auto=""
  [[ "$auto" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
}

spotlight_gate() {
  local h="$1" cpu
  # The MAX across samples, not the last: a late idle sample could overwrite an
  # earlier busy one. NR==0 (top produced nothing) is an ERROR, not 0% CPU.
  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null \
    | awk '/^mds_stores/{ if (\$2+0 > m) m = \$2+0 } END{ if (NR == 0) print \"ERR\"; else printf \"%d\", m+0 }'" | nocr)" || cpu="ERR"
  [[ "$cpu" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
}

load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
load_gate() {
  local h="$1" l ok
  l="$(load1 "$h")" || l=""
  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die_blind "$(hname "$h"): cannot read load1 (got '$l') — refusing"
  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
}

link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
  local h="$1" o peer_ip want got route_nic nic
  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"; nic="$(hnic "$h")"
  [[ -n "$want" ]] || die_blind "$(hname "$o"): its configured MAC does not parse — refusing"
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
  # A FAILED `diskutil` MUST NOT silently fall back to the synthesized disk (round-5
  # codex, HIGH). On APFS the volume lives on a synthesized container whose iostat
  # counters can read IDLE while the physical store is saturated — so falling back to
  # it is not a harmless default, it is a FALSE QUIET that certifies drainage on a
  # device the data never touched. If the volume is APFS, the physical-store lookup
  # must SUCCEED or the gate refuses.
  dev="$(hrun "$h" "d=\$(df '$p' 2>/dev/null | awk 'NR==2{print \$1}' | sed 's|^/dev/||')
[ -n \"\$d\" ] || { echo 'D:NO-DF:D'; exit 0; }
info=\$(diskutil info \"\$d\" 2>/dev/null) || { echo 'D:NO-DISKUTIL:D'; exit 0; }
[ -n \"\$info\" ] || { echo 'D:EMPTY-DISKUTIL:D'; exit 0; }
if echo \"\$info\" | grep -q 'APFS'; then
  ps=\$(echo \"\$info\" | awk -F: '/APFS Physical Store/{gsub(/[ \t]/, \"\", \$2); print \$2}' | head -1)
  [ -n \"\$ps\" ] || { echo 'D:APFS-NO-STORE:D'; exit 0; }
  d=\"\$ps\"
fi
echo \"D:\$(echo \"\$d\" | sed -E 's/s[0-9]+\$//'):D\"" | nocr | sed -n 's/.*D:\([^:]*\):D.*/\1/p' | head -1)"
  # Returns non-zero rather than dying, so the CALLER decides. (The self-test runs
  # each gate in a subshell to survive a refusal — and a `die` in there was invisible
  # while the global it sets was discarded, so the drain then had no device and
  # reported DRAIN-ERROR. The self-test was breaking its own next gate.)
  if [[ ! "$dev" =~ ^disk[0-9]+$ ]]; then
    log "$(hname "$h"): cannot resolve the PHYSICAL disk behind $p (got '$dev') — a drain that watches the wrong device certifies a disk the data never touched, and on APFS a synthesized disk can read idle while the physical store saturates"
    return 1
  fi
  # It must actually REPORT: an iostat that emits nothing for this device would
  # make every sample non-numeric, and the drain must never read that as quiet.
  local probe
  probe="$(hrun "$h" "iostat -d -w 1 -c 2 '$dev' 2>/dev/null | tail -1 | awk '{print \$3}'" | nocr)" || probe=""
  if [[ ! "$probe" =~ ^[0-9]+\.?[0-9]*$ ]]; then
    log "$(hname "$h"): iostat does not report numeric throughput for $dev (got '$probe') — cannot certify drainage"
    return 1
  fi
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
  # A FAILED ssh must not contribute a plausible number (round-5 codex, MEDIUM): a
  # fast-failing connection would report a small "bound" and flatter the settle claim.
  SSH_RTT_MS="$(python3 -c '
import statistics, subprocess, sys, time
argv = sys.argv[1:]
ts = []
for _ in range(5):
    t = time.monotonic()
    rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if rc != 0:
        print("SSH-FAILED")
        raise SystemExit
    ts.append((time.monotonic() - t) * 1000.0)
print(int(statistics.median(ts)))
' ssh "${SSH_MUX[@]}" "$Q_SSH" true)"
  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die_blind "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
  local rtt_max=$(( SETTLE_MS / 4 ))
  (( SSH_RTT_MS <= rtt_max )) \
    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
}

# =============================================================================
preflight() {
  # RUNS=8, and ONLY 8. The RUNS=16 escalation was removed (owner, 2026-07-14): a null is
  # judged on the FULL RANGE, which only WIDENS with n, so more pairs could never rescue an
  # UNCLEAR rig or certify a control -- and if you already have an EFFECT you do not need
  # it. Its p-hacking guard surface goes with it.
  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"

  # The instrument must be the REVIEWED instrument: a modified harness must not be
  # able to claim the reviewed commit.
  git -C "$REPO_ROOT" diff --quiet HEAD -- "$SELF" "$VERDICT_PY" "$VERDICT_TEST" \
    || die "the instrument has UNCOMMITTED changes (harness/verdict/test) — it cannot claim the reviewed commit. Commit or stash first."
  # The decision rule proves itself before it grades anything — AND proves the proof
  # is not vacuous. Running only the cases would let a silently-reverted fix pass
  # preflight if the cases still happen to pass for another reason (round-3 grok).
  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
    || die "the verdict engine's OWN guard test FAILS (see $OUT_DIR/verdict-guard-test.txt) — the decision rule is broken; refusing to take data"
  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt); the rule is not actually guarded, refusing to take data"
  log "verdict-engine guard test passed ($(grep -cE ' ok$' "$OUT_DIR/verdict-guard-test.txt" || true) cases, $(grep -cE 'KILLED' "$OUT_DIR/verdict-mutations.txt" || true) mutations killed)"

  local h p w want got wantb gotb
  for h in n q; do
    resolve_python "$h" || die_blind "$(hname "$h"): cannot establish an absolute python3 — refusing"
    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
    timer_gate "$h"                       # THE measurand's clock, proved on the rig
    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
    done
    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
    # THE SAME pgrep FAIL-OPEN AS THE QUIESCENCE GATE, IN A DUPLICATE SITE I DID NOT
    # TOUCH (round-5 codex, HIGH). `if hrun ... pgrep; then die; fi` reads rc>=2 (a
    # BROKEN probe, or a failed ssh) as "no daemon is running" and sails on. Every
    # process probe now goes through this one rc-aware helper -- there is no second
    # site left to forget.
    case "$(pgrep_state "$h" blit-daemon)" in
      RUNNING) die "$(hname "$h"): a blit-daemon is already running — stop it first" ;;
      NONE)    : ;;
      *)       die "$(hname "$h"): cannot probe for a stale blit-daemon — refusing (a gate that cannot answer must not answer 'fine')" ;;
    esac
    for w in large mixed small; do
      want="$(fix_count "$w")"; wantb="$(fix_bytes "$w")"
      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
    done
    link_gate "$h"
    resolve_disk "$h" || die "$(hname "$h"): cannot establish the drain device — refusing"
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
N_PY=""; Q_PY=""
hpy() { if [[ "$1" == n ]]; then echo "$N_PY"; else echo "$Q_PY"; fi; }
resolve_python() {
  local h="$1" p
  p="$(hrun "$h" "command -v python3" | nocr)" || p=""
  if [[ "$p" != /* ]]; then
    log "$(hname "$h"): cannot resolve an absolute python3 (got '$p')"; return 1
  fi
  if ! hrun "$h" "test -x '$p'"; then
    log "$(hname "$h"): python3 at '$p' is not executable"; return 1
  fi
  if [[ "$h" == n ]]; then N_PY="$p"; else Q_PY="$p"; fi
  log "  python3 on $(hname "$h"): $p (absolute — a PATH entry or shell function cannot stand in for the interpreter that MEASURES the settle)"
}

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
  # OWN THE PID BEFORE VALIDATING IT (round-5 codex, MEDIUM): the old code stored it
  # only AFTER the alive/listening checks, so a daemon that started but failed
  # validation was `die`d on while the EXIT trap did not yet know its pid — leaking a
  # live daemon holding the port for the next session to trip over.
  if [[ "$h" == n ]]; then N_PID="$pid"; else Q_PID="$pid"; fi
  sleep 2
  hrun "$h" "ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon" \
    || die "$(hname "$h"): daemon pid $pid is not alive (see $mod/mm-daemon.log)"
  # ALIVE is not SERVING: it must hold the port we are about to measure through.
  hrun "$h" "lsof -nP -a -p $pid -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1" \
    || die "$(hname "$h"): daemon pid $pid is NOT LISTENING on $PORT (see $mod/mm-daemon.log)"
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
  local h="$1" dev out
  dev="$(hdisk "$h")"
  [[ -n "$dev" ]] || { echo DRAIN-ERROR; return 0; }
  out="$(
  # A FAILED iostat must not certify quiet even when it printed a parseable line
  # (round-5 codex, HIGH: a numeric line followed by a NONZERO EXIT still accumulated
  # "quiet" samples). The exit code is now checked BEFORE the value is used.
  hrun "$h" "quiet=0
for i in \$(seq 1 $DRAIN_ITERS); do
  out=\$(iostat -d -w 2 -c 2 '$dev' 2>/dev/null); rc=\$?
  if [ \$rc -ne 0 ]; then echo DRAIN-ERROR; exit 0; fi
  w=\$(echo \"\$out\" | tail -1 | awk '{print \$3}')
  # A REAL number, not merely digits-and-dots: "." and ".." pass a shape test, read as 0,
  # and 0 < the threshold CERTIFIES QUIET (codex r9). awk decides, and it must see exactly
  # one numeric field.
  ok_num=\$(echo \"\$w\" | awk '{ print (NF == 1 && \$1 ~ /^[0-9]+(\\.[0-9]+)?\$/) ? 1 : 0 }')
  if [ \"\$ok_num\" != 1 ]; then echo DRAIN-ERROR; exit 0; fi
  ok=\$(awk -v w=\"\$w\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
done
echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1)" || out="DRAIN-ERROR"
  # ONE token, or it is an error -- AND the probe must have EXITED cleanly. A drain that
  # printed `drained_*` and THEN failed is not a drain (codex r8: I fixed the value and
  # left the status, which is the same defect one layer down).
  case "$out" in
    drained_[0-9]*x2s) echo "$out" ;;
    DRAIN-TIMEOUT)     echo DRAIN-TIMEOUT ;;
    *)                 echo DRAIN-ERROR ;;
  esac
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
RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
  local out
  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
  #
  # It used to be a shell `sleep` before the python. Round 5 found the awk computing
  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
  # RAN. Round 6 then found the repair was still not provable: `sleep` is
  # PATH/function-resolved, the walk's timer starts AFTER it, and the self-test only
  # counted files — so a no-op `sleep` would pass while the log narrated "settle
  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").
  #
  # A protection that cannot be OBSERVED is not a protection. The settle now happens
  # in python, is timed by the same monotonic clock as the walk, and is REPORTED. The
  # caller VOIDS the pair if it did not actually elapse. There is no shell sleep left
  # to shadow, no exit status left to discard, and no narration left to trust.
  out="$(hrun "$1" "$(hpy "$1") - '$SETTLE_SEC' '$2' <<'PYEOF'
import os, sys, time
settle = float(sys.argv[1])
p = sys.argv[2]
t0 = time.monotonic()
time.sleep(settle)
settled_ms = int((time.monotonic() - t0) * 1000)
if not os.path.isdir(p):
    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
    raise SystemExit
t = time.monotonic()
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
print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3 \4/p' | head -1)" || out=""
  echo "${out:-NA 0 0 0}"
}
# The settle actually elapsed, on the destination's own clock. Anything else voids.
settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }

# --- one timed run ------------------------------------------------------------
RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin wc wb
  bin="$(hblit "$ih")"
  prep_run "$dh"
  out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
  RUN_VALID=yes
  wc="$(fix_count "$w")"; wb="$(fix_bytes "$w")"
  # The equal settle is the ONLY thing standing between this rig and a free-writeback
  # artifact that REVERSES SIGN WITH DIRECTION — i.e. that can manufacture P1 out of
  # nothing. It has already been silently dead once. If it did not measurably elapse,
  # the row is not a fast row; it is a VOID row.
  if ! settle_ok "$RUN_SETTLED"; then
    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
    RUN_VALID=no
  fi
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

# THE CELLS ARE INTERLEAVED, NOT RUN BACK TO BACK.
#
# Round-8 (codex, HIGH): both measurand cells used to run first, then the controls. So the
# controls certified a window THEY NEVER SHARED -- a transient (a background process, a
# thermal excursion, a disk that woke up) could hit the measurand and be entirely gone by
# the time the gRPC/large controls ran, and they would certify the rig as clean. The
# controls are the ONLY thing standing between this rig and a rig-wide artifact, and they
# cannot vouch for a window they were not in.
#
# So the schedule is SLOT-MAJOR: within slot i, EVERY cell takes one ABBA pair, in a fixed
# registered order, before any cell takes slot i+1. All six cells therefore span the same
# wall-clock window and see the same transients.
#
#   cell           src dst fixture flag
CELL_TABLE=(
  "nq_tcp_mixed    n   q   mixed   "
  "qn_tcp_mixed    q   n   mixed   "
  "nq_grpc_mixed   n   q   mixed   --force-grpc"
  "qn_grpc_mixed   q   n   mixed   --force-grpc"
  "nq_tcp_large    n   q   large   "
  "qn_tcp_large    q   n   large   "
)

# macOS ships bash 3.2, which has NO associative arrays. Parallel indexed arrays, keyed by
# the cell's position in CELL_TABLE.
CELL_VALID=(); CELL_ATTEMPTS=()
run_one_pair() {   # $1=idx $2=cell $3=srchost $4=dsthost $5=fixture $6=flag $7=slot -> 0 if VALID
  local i="$1" cell="$2" sh="$3" dh="$4" w="$5" flag="$6" slot="$7"
  local attempts=$(( ${CELL_ATTEMPTS[$i]:-0} + 1 ))
  CELL_ATTEMPTS[$i]=$attempts
  CUR_W="$w"; CUR_FLAG="$flag"
  local order pair=yes rowA="" rowB="" arm aname init rid run
  # ABBA: the arm order alternates by slot, so a monotonic drift cannot favour one arm.
  if (( slot % 2 )); then order="A B"; else order="B A"; fi
  for arm in $order; do
    if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
    rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
    if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"; else arm_destinit "$sh" "$dh" "$run"; fi
    [[ "$RUN_VALID" == yes ]] || pair=no
    local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
    if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
    log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (fsync ${RUN_FLUSH}ms, settle ${RUN_SETTLED}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
  done
  echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
  if [[ "$pair" == yes ]]; then
    CELL_VALID[$i]=$(( ${CELL_VALID[$i]:-0} + 1 ))
    return 0
  fi
  log "  $cell: pair at slot $slot VOIDED"
  return 1
}

run_all_cells() {
  local slot i j cell sh dh w flag max=$(( 2 * RUNS )) n=${#CELL_TABLE[@]}
  for (( i = 0; i < n; i++ )); do CELL_VALID[$i]=0; CELL_ATTEMPTS[$i]=0; done
  for (( slot = 1; slot <= RUNS; slot++ )); do
    log "=== SLOT $slot / $RUNS (every cell takes one pair before any cell takes the next) ==="
    # ROTATE the cell order by slot. A FIXED order put both measurands ahead of every
    # control in every slot, so a PERIODIC transient could land on the measurands and never
    # on the controls that exist to catch it. Over 8 slots each cell occupies each position.
    for (( j = 0; j < n; j++ )); do
      i=$(( (j + slot - 1) % n ))
      read -r cell sh dh w flag <<<"${CELL_TABLE[$i]}"
      # a voided pair is retried IN PLACE, so the cell stays in step with its siblings
      while (( ${CELL_ATTEMPTS[$i]:-0} < max )); do
        if run_one_pair "$i" "$cell" "$sh" "$dh" "$w" "${flag:-}" "$slot"; then break; fi
      done
    done
  done
  for (( i = 0; i < n; i++ )); do
    read -r cell sh dh w flag <<<"${CELL_TABLE[$i]}"
    if (( ${CELL_VALID[$i]:-0} < RUNS )); then
      echo "$cell,${CELL_ATTEMPTS[$i]},no" >> "$META"
      log "  $cell INCOMPLETE: ${CELL_VALID[$i]}/$RUNS valid pairs"
    else
      echo "$cell,${CELL_ATTEMPTS[$i]},yes" >> "$META"
    fi
  done
}

SESSION_VOID_REASON=""
# The end-load is a CONDITION OF THE SESSION, not a log line. A mid-session load
# spike is exactly the contamination the start gate exists to prevent, and until now
# it could not void anything: the code logged `load1 (end)` and computed a verdict
# anyway, while the comment claimed a session "can void on it" (round-3 grok, HIGH —
# a doc claim the code did not honour, which is the defect class this whole review
# exists to kill).
end_load_gate() {
  local h l ok
  for h in n q; do
    l="$(load1 "$h")" || l=""
    if [[ ! "$l" =~ ^[0-9]+\.?[0-9]*$ ]]; then
      SESSION_VOID_REASON="end-load on $(hname "$h") could not be read (got '$l') — a session whose end conditions are unknown cannot be graded"
      return
    fi
    ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
    if [[ "$ok" != 1 ]]; then
      SESSION_VOID_REASON="end-load on $(hname "$h") is $l (> $LOAD_MAX) — the machine was NOT quiet at the end of the session, so a contaminant may have entered the timed windows"
      return
    fi
  done
}

compute_verdicts() {
  DELTA_REF_MS="$DELTA_REF_MS" VERDICT_CELLS="$VERDICT_CELLS" \
  CONTROL_CELLS="$CONTROL_CELLS" REGISTERED_CELLS="$REGISTERED_CELLS" \
  REQUIRED_PAIRS="$RUNS" SESSION_VOID_REASON="$SESSION_VOID_REASON" \
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
SELFTEST_FIRED=0; SELFTEST_BROKEN=0
# A gate can end in three states, and the old self-test collapsed two of them
# (round-5 codex, HIGH: "every nonzero result — including a BROKEN probe — is labeled
# [FIRED], and the self-test exits zero"). That is the same fail-open it exists to
# hunt, committed by the hunter:
#
#   [OK]     the probe answered and the condition holds.
#   [FIRED]  the probe answered and the condition is genuinely UNMET (codex is
#            running, Time Machine is on). The gate WORKS. Not a self-test failure.
#   [BROKEN] the probe could not answer at all. THE GATE IS BLIND, and the self-test
#            FAILS (exit 1) — a blind gate is exactly what fails open on the night.
#
# The two are told apart by the refusal text: every "cannot answer" die() in this file
# says so in the words below, and every genuine-condition die() does not.
# A REPORTER, never a gate: it must always return 0, or `set -e` aborts the sweep at
# the first refusal and the remaining gates go untested (which is exactly what it did
# the first time it ran — the self-test could not even test itself).
gate_probe() {
  local label="$1"; shift
  local err rc=0
  err="$( { "$@"; } 2>&1 )" || rc=1
  if (( rc == 0 )); then
    log "  [OK]     $label — answers, and the condition holds"
  elif grep -q 'PROBE-BLIND' <<<"$err"; then
    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 ))
    log "  [BROKEN] $label — THE PROBE COULD NOT ANSWER. A blind gate fails open on the night."
  else
    SELFTEST_FIRED=$(( SELFTEST_FIRED + 1 ))
    log "  [FIRED]  $label — the gate REFUSED a genuinely unmet condition. It works."
  fi
  # Never hide what the gate said — including its own evidence on success.
  [[ -n "$err" ]] && sed 's/^/           | /' <<<"$err" | tee -a "$OUT_DIR/bench.log" >&2
  return 0
}

# The fsync/settle path, exercised for real on a throwaway tree. It is the durability
# measurement AND the equal-settle window — the two things that once manufactured P1 —
# and the self-test never touched them.
selftest_fsync() {
  local h="$1" d ms files bytes settled
  d="$(hmod "$h")/selftest_${SESSION_TAG}"
  hrun "$h" "rm -rf '$d' && mkdir -p '$d' && printf 'aaaa' > '$d/a' && printf 'bb' > '$d/b'" \
    || { log "  [BROKEN] fsync/settle — cannot stage a probe tree"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); return 1; }
  read -r ms files bytes settled <<<"$(fsync_tree "$h" "$d")"
  hrun "$h" "rm -rf '$d'" >/dev/null 2>&1 || true
  if [[ "$ms" == NA || "$files" != 2 || "$bytes" != 6 ]]; then
    log "  [BROKEN] fsync/settle — walk returned ms=$ms files=$files bytes=$bytes, want 2 files / 6 bytes"
    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
  fi
  # THE SETTLE MUST BE PROVED, NOT NARRATED (round-6, both reviewers). The old check
  # counted files and then LOGGED "settle included" — which is a sentence, not an
  # assertion. It would have passed with the settle stone dead, which is precisely how
  # the settle stayed dead for three revisions.
  if ! settle_ok "$settled"; then
    log "  [BROKEN] fsync/settle — THE SETTLE DID NOT ELAPSE: measured ${settled}ms, want >= ${SETTLE_MS}ms"
    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
  fi
  log "  [OK]     fsync/settle — 2 files/6 bytes walked in ${ms}ms; settle MEASURED at ${settled}ms (>= ${SETTLE_MS}ms), counts VERIFIED"
}

selftest() {
  local h
  log "SELFTEST — exercising the gates for real. No transfer, NO DATA."
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
    # NOT through gate_probe: it runs its argument in a SUBSHELL, and this function's
    # whole job is to SET a global. (resolve_disk had the identical bug — the self-test
    # was breaking its own next gate. Same class, and it caught itself this time.)
    if resolve_python "$h"; then log "  [OK]     python3       (absolute, not PATH-resolved)"
    else log "  [BROKEN] python3       — could not resolve an absolute interpreter"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); fi
    gate_probe "timer         (the measurand's clock)" timer_gate "$h"
    gate_probe "quiescence    (codex/cargo/rustc)"     quiescence_gate "$h"
    gate_probe "time machine  (running OR enabled)"    timemachine_gate "$h"
    gate_probe "spotlight     (mds_stores CPU)"        spotlight_gate "$h"
    gate_probe "load  start   (load1 <= $LOAD_MAX)"      load_gate "$h"
    gate_probe "link          (ARP on the egress NIC + 10GbE route)" link_gate "$h"
    # NOT through gate_probe: it runs its argument in a SUBSHELL (so a `die` cannot
    # abort the sweep), and resolve_disk's whole job is to SET a global. Called there,
    # the assignment was discarded and the drain loop below then had no device and
    # reported DRAIN-ERROR — the self-test was breaking its own next gate and blaming
    # the harness.
    if resolve_disk "$h"; then log "  [OK]     drain device  (resolved via the APFS physical store)"
    else log "  [BROKEN] drain device  — could not resolve the physical disk"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); fi
    # The paths the old self-test claimed and did not run (round-5 codex, HIGH):
    gate_probe "purge         (sudo -n, or every run reads WARM)" hrun "$h" "sudo -n /usr/sbin/purge"
    case "$(pgrep_state "$h" blit-daemon)" in
      NONE)    log "  [OK]     stale daemon  (rc-aware probe: none running)" ;;
      RUNNING) log "  [FIRED]  stale daemon  (one IS running — the gate would refuse)"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
      *)       log "  [BROKEN] stale daemon  — the probe could not answer"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
    esac
    # DRAIN-TIMEOUT is a genuinely busy disk (the gate WORKING); DRAIN-ERROR is a blind
    # probe. Scoring them the same made the classification untrustworthy (grok r6, F7).
    local dr; dr="$(drain_host "$h")"
    case "$dr" in
      drained*)      log "  [OK]     drain loop    ($dr)" ;;
      DRAIN-TIMEOUT) log "  [FIRED]  drain loop    — the disk is genuinely busy; the gate would void the pair"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
      *)             log "  [BROKEN] drain loop    — the probe could not answer ('$dr')"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
    esac
    selftest_fsync "$h"
    log "  [--]     mac parse (no gawk strtonum): $(hmac "$h") -> $(hmac "$h" | norm_mac)"
  done
  SESSION_VOID_REASON=""; end_load_gate
  if [[ -z "$SESSION_VOID_REASON" ]]; then
    log "  [OK]     end-load gate (both Macs under $LOAD_MAX; it CAN void a session)"
  elif [[ "$SESSION_VOID_REASON" == *"could not be read"* ]]; then
    # An UNREADABLE end-load is a blind probe, not a busy machine (grok r6, F7).
    log "  [BROKEN] end-load gate — $SESSION_VOID_REASON"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1))
  else
    log "  [FIRED]  end-load gate — $SESSION_VOID_REASON"; SELFTEST_FIRED=$((SELFTEST_FIRED+1))
  fi
  measure_ssh_rtt
  log ""
  log "SELFTEST: $SELFTEST_FIRED gate(s) refused a genuinely unmet condition; $SELFTEST_BROKEN blind."
  log "NOT exercised here (they need a real transfer): daemon start/lsof/teardown, the"
  log "smoke transfer, the ABBA pair loop, pair-voiding, and the manifest. PREFLIGHT_ONLY=1"
  log "covers the manifest and the build-provenance gates. This self-test does NOT claim"
  log "to run every gate — the previous one did, and it was not true."
  log "THIS IS NOT CLEARANCE TO TAKE DATA. The review is."
  if (( SELFTEST_BROKEN > 0 )); then
    log "SELFTEST FAILED: $SELFTEST_BROKEN gate(s) are BLIND."
    exit 1
  fi
  log "SELFTEST PASSED: every gate exercised here can answer."
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
  echo "cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
  echo "cell,pairs_attempted,complete" > "$META"
  daemon_start n; daemon_start q
  smoke n; smoke q

  run_all_cells

  # End-load BEFORE the verdict is computed, and it can VOID the session.
  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
  end_load_gate
  if [[ -n "$SESSION_VOID_REASON" ]]; then
    log "ERROR: SESSION VOID — $SESSION_VOID_REASON"
    touch "$OUT_DIR/SESSION-VOID"
  fi
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
