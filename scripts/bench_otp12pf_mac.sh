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
# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
# cell. It answers ONE question, SCOPED TO THIS PAIR:
#
#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
#
#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
#     "platform residue" that can be waived; code-level hypotheses strengthen.
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
# WHAT IT MEASURES
#   cell = <nq|qn>_<carrier>_<fixture>;  nq_* = data nagatha->q, qn_* = q->nagatha
#   arms (the ONLY variable): srcinit (source's CLI pushes) / destinit (dest's CLI
#   pulls). BOTH directions are measured, but a reproduction is NOT required in
#   both — P1's rig-W signature is ONE-DIRECTIONAL (wm FAILS, mw PASSES), so
#   demanding both would rewrite the finding.
#
#   Endpoint asymmetry does NOT cancel: switching the initiator also reassigns
#   which Mac runs the CLI vs the daemon, and q is faster. Both directions are
#   therefore reported separately and no conclusion leans on cancellation.
#
# THE INSTRUMENT IS THE RISK (three claims have been retracted to harness bugs).
# Everything below fails CLOSED. Codex review of the first revision found 11
# defects (3 BLOCKER) in this file before it measured anything; they are fixed
# here and named at their site.
#
#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER THE INITIATOR/VERB, and
#     the fsync walk VERIFIES WHAT IT FLUSHED: it returns the file count and byte
#     sum, and the pair VOIDS unless they match the fixture exactly. (os.walk of a
#     missing/empty path returns 0 files in 0 ms and reads as a FAST SUCCESSFUL
#     FLUSH — the otp-2w bug's exact shape. Verified empirically: a push to
#     /bench/RUNDIR/ lands RUNDIR/src_<W>, a pull into RUNDIR lands files directly
#     in RUNDIR, so the two arms need DIFFERENT landed paths and a wrong one would
#     silently charge an arm nothing.)
#   * A FIXED, EQUAL SETTLE (SETTLE_MS) precedes the fsync on BOTH arms. Between
#     a client exiting and the fsync starting, the OS writes back dirty pages FOR
#     FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
#     BY DIRECTION (in nq the remote arm is destinit; in qn it is srcinit). Since
#     P1's signature is one-directional, that artifact could MANUFACTURE the
#     result. Measured on this rig before fixing: a 10/20/200 ms pre-fsync delay
#     produced NO measurable change in fsync time (72-94 ms, no trend) — APFS
#     fsync here is per-file-metadata bound, not writeback bound — so the fixed
#     settle removes the structural asymmetry without weakening what durability
#     charges.
#   * cold caches BOTH ends every run (purge), then the destination disk is
#     drained to quiet AND RE-CHECKED — the purge itself dirties the disk, so a
#     drain certified BEFORE it proves nothing.
#   * pair-void on: nonzero exit, undrained window, failed purge, fsync mismatch.
#   * same-build gate: clean +EXPECT_SHA, never +sha.dirty; hash failures FATAL.
#   * the HARNESS ITSELF is hashed into the manifest — a modified harness must not
#     be able to claim the reviewed commit.
#
# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
# one ssh), so dispatch is outside the window by construction.
#
# Usage:
#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SELF="${BASH_SOURCE[0]}"

HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a)}"

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
PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
SETTLE_MS="${SETTLE_MS:-250}"     # equal pre-fsync window on BOTH arms
LOAD_MAX="${LOAD_MAX:-3.0}"
DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
DRAIN_MBPS="${DRAIN_MBPS:-2}"
DELTA_REF_MS="${DELTA_REF_MS:-230}"   # rig W's measured Delta_P1 (the reference effect)

# The REGISTERED cell set. An unregistered or misspelled CELLS must not be able to
# drop every control, or silently measure nothing (codex HIGH).
REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
CELLS="${CELLS:-$REGISTERED_CELLS}"
CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"

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
want_cell() { [[ ",$CELLS," == *",$1,"* ]]; }

# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
# falls through to the wrong host (the trap the Linux harness documents).
# `bash -c` locally pins the inner shell so local and remote parse identically
# (q's login shell is not assumed).
hrun() {
  local h="$1"; shift
  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "bash -c $(printf '%q' "$*")"; fi
}
hblit()   { [[ "$1" == n ]] && echo "$N_BLIT"   || echo "$Q_BLIT"; }
hdaemon() { [[ "$1" == n ]] && echo "$N_DAEMON" || echo "$Q_DAEMON"; }
hmod()    { [[ "$1" == n ]] && echo "$N_MODULE" || echo "$Q_MODULE"; }
hip()     { [[ "$1" == n ]] && echo "$N_IP"     || echo "$Q_IP"; }
hnic()    { [[ "$1" == n ]] && echo "$N_NIC"    || echo "$Q_NIC"; }
hmac()    { [[ "$1" == n ]] && echo "$N_MAC"    || echo "$Q_MAC"; }
hname()   { [[ "$1" == n ]] && echo nagatha     || echo q; }
other()   { [[ "$1" == n ]] && echo q           || echo n; }

# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
FIX_COUNT_small=10000; FIX_BYTES_small=40960000
FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912

# --- provenance ---------------------------------------------------------------
embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
  local h="$1" p="$2" hit dirty
  hit="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA' '$p' 2>/dev/null || echo X" | nocr)"
  dirty="$(hrun "$h" "grep -c -a -- '+$EXPECT_SHA.dirty' '$p' 2>/dev/null || echo X" | nocr)"
  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
}
sha256_of() {      # fail CLOSED on an empty/short hash
  local h="$1" p="$2" v
  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')"
  [[ ${#v} -eq 64 ]] || die "$(hname "$h"): sha256 of $p returned '${v}' (not 64 hex) — refusing"
  echo "$v"
}

# --- gates: every one fails CLOSED (codex HIGH: they all failed OPEN) ----------
norm_mac() { tr 'A-F' 'a-f' | awk -F: '{for(i=1;i<=NF;i++){printf "%s%02x", (i>1?":":""), strtonum("0x" $i)}; print ""}'; }

quiescence_gate() {
  local h="$1" out
  out="$(hrun "$h" "pgrep -x codex >/dev/null 2>&1 && echo codex; pgrep -x cargo >/dev/null 2>&1 && echo cargo; pgrep -x rustc >/dev/null 2>&1 && echo rustc; echo __OK__" | nocr)" \
    || die "$(hname "$h"): quiescence probe FAILED — a gate that cannot answer must not answer 'fine'"
  [[ "$out" == *__OK__* ]] || die "$(hname "$h"): quiescence probe returned no sentinel — refusing"
  local busy; busy="$(echo "$out" | grep -v __OK__ | tr '\n' ' ' | xargs || true)"
  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
}
timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
  local h="$1" running auto
  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
  [[ "$running" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1; echo" | nocr | tr -cd '0-9')" || auto=""
  [[ "$auto" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
}
spotlight_gate() {
  local h="$1" cpu
  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null | awk '/^mds_stores/{c=\$2} END{printf \"%d\", c+0}'" | nocr | tr -cd '0-9')" || cpu=""
  [[ "$cpu" =~ ^[0-9]+$ ]] || die "$(hname "$h"): cannot sample Spotlight CPU — refusing"
  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
}
load_gate() {
  local h="$1" l ok
  l="$(hrun "$h" "sysctl -n vm.loadavg" | nocr | awk '{print $2}')" || l=""
  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die "$(hname "$h"): cannot read load1 (got '$l') — refusing"
  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
}
load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }

link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
  local h="$1" o peer_ip want got route_nic
  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"
  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk '{print \$4}'" | nocr | norm_mac)"
  [[ -n "$got" && "$got" != "(incomplete)" ]] || die "$(hname "$h"): no ARP entry for $peer_ip"
  [[ "$got" == "$want" ]] \
    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
  [[ "$route_nic" == "$(hnic "$h")" ]] \
    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
}

preflight() {
  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
  local c
  for c in ${CELLS//,/ }; do
    [[ ",$REGISTERED_CELLS," == *",$c,"* ]] \
      || die "cell '$c' is not in the REGISTERED set ($REGISTERED_CELLS) — a misspelled cell must not silently drop a control or measure nothing"
  done
  local h p w want got wantb gotb
  for h in n q; do
    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
    done
    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
    if hrun "$h" "pgrep -x blit-daemon >/dev/null 2>&1"; then die "$(hname "$h"): a blit-daemon is already running — stop it first"; fi
    for w in large mixed small; do
      want="$(eval echo "\$FIX_COUNT_$w")"; wantb="$(eval echo "\$FIX_BYTES_$w")"
      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
    done
    link_gate "$h"
  done
  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
}

write_manifest() {
  local f="$OUT_DIR/staging-manifest.txt" h
  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
    echo "# cells=$CELLS"
    echo "host,role,sha,sha256,path"
    for h in n q; do
      echo "$(hname "$h"),client,$EXPECT_SHA,$(sha256_of "$h" "$(hblit "$h")"),$(hblit "$h")"
      echo "$(hname "$h"),daemon,$EXPECT_SHA,$(sha256_of "$h" "$(hdaemon "$h")"),$(hdaemon "$h")"
    done; } > "$f"
  log "staging manifest recorded (harness sha256 + 4 binary hashes + every threshold)"
}

# --- daemons ------------------------------------------------------------------
N_PID=""; Q_PID=""
daemon_start() {
  local h="$1" cfg mod bin pid
  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
  hrun "$h" "mkdir -p '$mod'
printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg'
nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
sleep 2" >/dev/null 2>&1 || true
  pid="$(hrun "$h" "pgrep -x blit-daemon | head -1" | nocr | tr -cd '0-9')"
  [[ -n "$pid" ]] || die "$(hname "$h"): daemon failed to start (see $(hmod "$h")/mm-daemon.log)"
  [[ "$h" == n ]] && N_PID="$pid" || Q_PID="$pid"
  log "$(hname "$h") daemon up (pid $pid) on $(hip "$h"):$PORT"
}
# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
# handshake reached some listener's backlog — not that the daemon speaks blit).
smoke() {
  local h="$1" o probe
  o="$(other "$h")"
  probe="$(hmod "$o")/mm_smoke_${SESSION_TAG}"
  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 || true
  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
  log "smoke ok: $(hname "$h") daemon serves blit"
}
daemon_stop() {
  local h="$1" pid; pid="$([[ "$h" == n ]] && echo "$N_PID" || echo "$Q_PID")"
  [[ -n "$pid" ]] || return 0
  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; echo __DONE__" >/dev/null 2>&1 || true
  # A teardown that cannot be VERIFIED is a failure, not a success (codex MEDIUM).
  if hrun "$h" "ps -p $pid >/dev/null 2>&1 && echo ALIVE || echo GONE" | nocr | grep -q ALIVE; then
    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown — port $PORT may still be held"
    return 1
  fi
  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
}
cleanup() { daemon_stop n || true; daemon_stop q || true; rm -rf "$MUX" 2>/dev/null || true; }
trap cleanup EXIT

# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
RUN_DRAIN=""; RUN_COLD=""
drain_host() {
  hrun "$1" "quiet=0
for i in \$(seq 1 $DRAIN_ITERS); do
  w=\$(iostat -d -w 2 -c 2 disk0 2>/dev/null | tail -1 | awk '{print \$3}')
  ok=\$(awk -v w=\"\${w:-99}\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
done
echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
}
prep_run() {   # $1 = dest host
  local dh="$1" cn=ok cq=ok out
  # Purge BOTH ends first — the purge itself dirties the disk, so a drain
  # certified before it proves nothing (codex HIGH).
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
  out="$(hrun "$1" "sleep $(awk -v m=$SETTLE_MS 'BEGIN{printf \"%.3f\", m/1000}')
python3 - '$2' <<'PYEOF'
import os, sys, time
p = sys.argv[1]
if not os.path.isdir(p):
    print('F:NA:0:0:F')          # a MISSING tree must never read as a fast flush
    raise SystemExit
t = time.monotonic()
files = 0
nbytes = 0
for root, _d, fs in os.walk(p):
    for name in fs:
        fp = os.path.join(root, name)
        fd = os.open(fp, os.O_RDONLY)
        os.fsync(fd)
        os.close(fd)
        files += 1
        nbytes += os.fstat(os.open(fp, os.O_RDONLY)).st_size if False else os.path.getsize(fp)
print('F:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes))
PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3/p' | head -1)"
  echo "${out:-NA 0 0}"
}

# --- one timed run ------------------------------------------------------------
RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin r
  bin="$(hblit "$ih")"
  prep_run "$dh"
  out="$(hrun "$ih" "t0=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/mm-client.err; rc=\$?
t1=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
echo \"R:\$((t1-t0)),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
  read -r RUN_FLUSH RUN_FILES RUN_BYTES <<<"$(fsync_tree "$dh" "$landed")"
  RUN_VALID=yes
  local wc wb; wc="$(eval echo "\$FIX_COUNT_$w")"; wb="$(eval echo "\$FIX_BYTES_$w")"
  if [[ "$RUN_FLUSH" == NA ]]; then
    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
    RUN_VALID=no; RUN_FLUSH=0
  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
    RUN_VALID=no
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
  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
}
arm_destinit() {
  local cell="$1" rid="$2" sh="$3" dh="$4" run="$5"
  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
}

CSV="$OUT_DIR/runs.csv"
echo "cell,arm,build,initiator,run,ms,flush_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"

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
      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh" "$run"
      else arm_destinit "$cell" "$rid" "$sh" "$dh" "$run"; fi
      [[ "$RUN_VALID" == yes ]] || pair=no
      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
      [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
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
  DELTA_REF_MS="$DELTA_REF_MS" VERDICT_CELLS="$VERDICT_CELLS" CONTROL_CELLS="$CONTROL_CELLS" \
  python3 "$SCRIPT_DIR/otp12pf_mac_verdict.py" \
    "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/paired.csv" \
    "$OUT_DIR/verdicts.csv" "$OUT_DIR/session_verdict.txt"
}

main() {
  preflight
  write_manifest
  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
    exit 0
  fi
  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
  daemon_start n; daemon_start q
  smoke n; smoke q

  local carrier w flag cell
  for w in mixed large small; do
    for carrier in tcp grpc; do
      [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
      CUR_W="$w"; CUR_FLAG="$flag"
      cell="nq_${carrier}_${w}"; want_cell "$cell" && run_pair_loop "$cell" n q
      cell="qn_${carrier}_${w}"; want_cell "$cell" && run_pair_loop "$cell" q n
    done
  done

  compute_verdicts
  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
  log "=== PAIRED STATS (the rule is graded on these) ==="
  column -t -s, "$OUT_DIR/paired.csv" | tee -a "$OUT_DIR/bench.log"
  log "=== SESSION VERDICT (computed by the harness from the PRE-REGISTERED rule) ==="
  cat "$OUT_DIR/session_verdict.txt" | tee -a "$OUT_DIR/bench.log"
  log "runs: $CSV"
}
main "$@"
