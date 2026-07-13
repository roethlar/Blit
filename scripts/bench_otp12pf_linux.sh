#!/usr/bin/env bash
# =============================================================================
# bench_otp12pf_linux.sh — the SAME-OS invariance rig (magneto <-> skippy)
# Plan: docs/plan/OTP12_PERF_FINDINGS.md. Probe that motivated it:
# docs/bench/otp12-perf-2026-07-13/ (P1 reproduced at 1.78 on Linux<->Linux).
# =============================================================================
#
# WHY THIS RIG EXISTS
# -------------------
# P1 (destination-initiated TCP mixed pays ~25-30%) was only ever measured on
# rig W (Mac<->Windows). On a TWO-HOST rig host identity IS role: in the slow
# arm the destination is the Mac (which dials) AND the source is Windows (which
# accepts) — inseparable. So P1 was consistent with both "our layout is slow"
# and "macOS/Windows platform residue" (which D-2026-07-12-1 would let the owner
# ACCEPT). This rig removes the platform terms entirely: Linux on BOTH ends.
# A reproduction here cannot be a macOS or a Windows artifact.
#
# Endpoints need not match each other. An invariance comparison holds BOTH
# endpoints fixed and varies only the initiator, so endpoint asymmetry cancels
# within each pair (ONE_TRANSFER_PATH acceptance criterion 1). What zoey failed
# was the absolute-speed floor (it would MASK the effect); magneto clears it
# (owner, 2026-07-13: "fast enough to saturate 10GbE where zoey is definitely
# not").
#
# WHAT IT MEASURES
#   cell = <sm|ms>_<carrier>_<fixture>
#     sm_* : data skippy -> magneto     ms_* : data magneto -> skippy
#   arms per cell (the ONLY variable):
#     srcinit  : the SOURCE host's CLI pushes      (source-initiated)
#     destinit : the DEST   host's CLI pulls       (destination-initiated)
#   Both arms move the same bytes over the same data plane onto the same
#   destination disk, and both pay the same destination-side sync. TCP cells are
#   the verdict rows; grpc_mixed is the carrier control (P1 is TCP-only, so it
#   must NOT reproduce there).
#
# VERDICT: invariance bar, per plan D2 — max(srcinit,destinit)/min <= 1.10,
# integer-exact. This script COMPUTES; it declares nothing (otp-13 owner walk).
#
# METHODOLOGY (otp-12 shape; the probe had none of this)
#   * cold caches on BOTH ends every run (drop_caches via the exact NOPASSWD
#     grant on each host — a FAILED drop VOIDS the pair; never a warm row);
#   * destination disk drained to quiet before each timed window;
#   * fresh, never-seen destination per run (SESSION_TAG + arm + attempt);
#   * ABBA counterbalanced interleave; a nonzero exit or an undrained window
#     VOIDS the whole pair, which reruns (cap 2*RUNS, then INCOMPLETE);
#   * self-timed on the initiating host (/proc/uptime bracket in ONE ssh) plus
#     the destination-side sync — the ssh round trip stays OUTSIDE the window;
#   * same-build gate: every binary must embed +EXPECT_SHA and NOT +<sha>.dirty.
#
# Usage:
#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_linux.sh
#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_linux.sh
#   CELLS=sm_tcp_mixed RUNS=8 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_linux.sh
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

NEW_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
EXPECT_SHA="${EXPECT_SHA:-$NEW_SHA}"      # binary identity actually gated + hashed

# skippy (EPYC, ZFS pool)
S_SSH="${S_SSH:-admin@skippy}";  S_IP="${S_IP:-10.1.10.143}"
S_BIN="${S_BIN:-/mnt/generic-pool/video/blit-bin}"
S_BLIT="${S_BLIT:-$S_BIN/blit}"; S_DAEMON="${S_DAEMON:-$S_BIN/blit-daemon}"
S_MODULE="${S_MODULE:-$S_BIN/cmp}"
S_DISK_RE="${S_DISK_RE:-^sd[a-z]$|^nvme[0-9]+n1$|^dm-[0-9]+$}"

# magneto (Intel, NVMe)
M_SSH="${M_SSH:-michael@magneto}"; M_IP="${M_IP:-10.1.10.10}"
M_DIR="${M_DIR:-/home/michael/blit-probe}"
M_BLIT="${M_BLIT:-$M_DIR/bin/blit}"; M_DAEMON="${M_DAEMON:-$M_DIR/bin/blit-daemon}"
M_MODULE="${M_MODULE:-$M_DIR/module}"
M_DISK_RE="${M_DISK_RE:-^nvme[0-9]+n1$}"

PORT="${PORT:-9031}"
RUNS="${RUNS:-4}"
PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
CELLS="${CELLS:-}"
SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_linux_$SESSION_TAG}"
DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
DRAIN_SECTORS="${DRAIN_SECTORS:-4096}"

MUX="$(mktemp -d /tmp/blit-pf-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts the 104b ControlPath cap
SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
sssh() { ssh "${SSH_MUX[@]}" "$S_SSH" "$@"; }
mssh() { ssh "${SSH_MUX[@]}" "$M_SSH" "$@"; }

mkdir -p "$OUT_DIR/blit-logs"
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
die() { log "FATAL: $*"; exit 1; }
nocr() { tr -d '\r'; }
want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }

# host abstraction: $1 = s|m
# NOTE: written as an if/else, NOT `[[ ]] && sssh || mssh`. The && || form is a
# trap: when the skippy command exits NON-ZERO (e.g. pgrep correctly finding no
# daemon) the && chain fails and control falls into the || branch, which re-runs
# the command ON MAGNETO — wrong host, wrong args, and the exit code is lost.
# Found live at the first preflight.
hssh() {
  local h="$1"; shift
  if [[ "$h" == s ]]; then sssh "$@"; else mssh "$@"; fi
}
hblit()  { [[ "$1" == s ]] && echo "$S_BLIT"   || echo "$M_BLIT"; }
hmod()   { [[ "$1" == s ]] && echo "$S_MODULE" || echo "$M_MODULE"; }
hip()    { [[ "$1" == s ]] && echo "$S_IP"     || echo "$M_IP"; }
hdisk()  { [[ "$1" == s ]] && echo "$S_DISK_RE" || echo "$M_DISK_RE"; }
hname()  { [[ "$1" == s ]] && echo skippy      || echo magneto; }

# ---- fixtures (otp-2 shapes; verified by count+bytes, never trusted) --------
FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
FIX_COUNT_small=10000; FIX_BYTES_small=40960000
FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912

# ---- provenance: embed +sha AND reject +sha.dirty ---------------------------
embeds_clean() {   # $1=host $2=path
  hssh "$1" "grep -qa -- '+$EXPECT_SHA' '$2' && ! grep -qa -- '+$EXPECT_SHA.dirty' '$2'"
}
sha256_of() { hssh "$1" "sha256sum '$2' | cut -d' ' -f1" | nocr; }

preflight() {
  [[ "$RUNS" -ge 2 ]] || die "RUNS must be >= 2"
  local h p
  for h in s m; do
    for p in "$(hblit "$h")" "$([[ $h == s ]] && echo "$S_DAEMON" || echo "$M_DAEMON")"; do
      hssh "$h" "test -x '$p'" || die "$(hname "$h"): missing/not exec: $p"
      embeds_clean "$h" "$p" || die "$(hname "$h"): $p does not embed a CLEAN +$EXPECT_SHA"
    done
    # cold-cache capability is METHODOLOGY, not a nicety — hard gate.
    hssh "$h" "echo 1 | sudo -n /usr/bin/tee /proc/sys/vm/drop_caches >/dev/null" \
      || die "$(hname "$h") cannot drop caches (need NOPASSWD /usr/bin/tee /proc/sys/vm/drop_caches) — runs would read WARM"
    hssh "$h" "pgrep -x blit-daemon >/dev/null" && die "$(hname "$h"): a blit-daemon is already running — stop it first"
  done
  log "preflight OK  binary=$EXPECT_SHA (harness HEAD=$NEW_SHA)  runs/arm=$RUNS"
}

write_manifest() {
  local f="$OUT_DIR/staging-manifest.txt" h
  { echo "# harness_head=$NEW_SHA binary_identity=$EXPECT_SHA"
    echo "host,role,sha,sha256,path"
    for h in s m; do
      echo "$(hname "$h"),client,$EXPECT_SHA,$(sha256_of "$h" "$(hblit "$h")"),$(hblit "$h")"
      local d; [[ $h == s ]] && d="$S_DAEMON" || d="$M_DAEMON"
      echo "$(hname "$h"),daemon,$EXPECT_SHA,$(sha256_of "$h" "$d"),$d"
    done; } > "$f"
  log "staging manifest recorded (4 hashes)"
}

# ---- daemons: Linux both ends (they survive ssh close, unlike Win32-OpenSSH) --
S_PID=""; M_PID=""
daemon_start() {   # $1 = host
  local h="$1" cfg mod
  mod="$(hmod "$h")"
  cfg="$([[ $h == s ]] && echo "$S_BIN/pf-bench.toml" || echo "$M_DIR/pf-bench.toml")"
  local bin; [[ $h == s ]] && bin="$S_DAEMON" || bin="$M_DAEMON"
  hssh "$h" "mkdir -p '$mod'
printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg'
setsid '$bin' --config '$cfg' > '$mod/../pf-daemon.log' 2>&1 < /dev/null &
sleep 2; pgrep -x blit-daemon | head -1" | nocr | tr -dc '0-9' > "$OUT_DIR/.$h.pid"
  local pid; pid="$(cat "$OUT_DIR/.$h.pid")"
  [[ -n "$pid" ]] || die "$(hname "$h"): daemon failed to start"
  hssh "$h" "ss -ltn | grep -q ':$PORT'" || die "$(hname "$h"): daemon not listening on $PORT"
  [[ $h == s ]] && S_PID="$pid" || M_PID="$pid"
  log "$(hname "$h") daemon up (pid $pid) on $(hip "$h"):$PORT"
}
daemon_stop() {   # $1 = host ; PID-scoped + comm-verified; verify it actually died
  local h="$1" pid; pid="$([[ $h == s ]] && echo "$S_PID" || echo "$M_PID")"
  [[ -n "$pid" ]] || return 0
  hssh "$h" "if [ -r /proc/$pid/comm ] && grep -qi blit /proc/$pid/comm; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do [ -d /proc/$pid ] || break; sleep 0.5; done; [ -d /proc/$pid ] && kill -9 $pid 2>/dev/null; fi; true" 2>/dev/null || true
  if hssh "$h" "[ -d /proc/$pid ]" 2>/dev/null; then
    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown — port $PORT may still be held"; return 1
  fi
  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
}

# ---- cold + drain ------------------------------------------------------------
RUN_DRAIN=""; RUN_COLD=""
drain_host() {   # $1 = dest host; assert the disk regex matches >=1 device first
  local h="$1" re; re="$(hdisk "$h")"
  hssh "$h" "n=\$(awk '\$3 ~ /$re/ {c++} END{print c+0}' /proc/diskstats)
if [ \"\$n\" -eq 0 ]; then echo DRAIN-NODEV; exit 0; fi
sync; quiet=0
for i in \$(seq 1 $DRAIN_ITERS); do
  a=\$(awk '\$3 ~ /$re/ {s+=\$10} END{printf \"%.0f\", s}' /proc/diskstats); sleep 2
  b=\$(awk '\$3 ~ /$re/ {s+=\$10} END{printf \"%.0f\", s}' /proc/diskstats)
  if [ \$((b-a)) -lt $DRAIN_SECTORS ]; then quiet=\$((quiet+1)); else quiet=0; fi
  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained \${i}x2s\"; exit 0; fi
done
echo DRAIN-TIMEOUT" 2>/dev/null | nocr || echo DRAIN-ERROR
}
prep_run() {   # $1 = dest host. Drain dest, then cold BOTH ends. A failed drop VOIDS.
  local dh="$1" out cs=ok cm=ok
  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"; RUN_DRAIN="${RUN_DRAIN// /_}"
  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
  sssh "sync; echo 3 | sudo -n /usr/bin/tee /proc/sys/vm/drop_caches >/dev/null" >/dev/null 2>&1 || cs=FAIL
  mssh "sync; echo 3 | sudo -n /usr/bin/tee /proc/sys/vm/drop_caches >/dev/null" >/dev/null 2>&1 || cm=FAIL
  if [[ "$cs" == ok && "$cm" == ok ]]; then RUN_COLD=cold
  else RUN_COLD="COLD-FAIL(skippy=$cs,magneto=$cm)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
}
cold_ok() { [[ "$RUN_COLD" == cold ]]; }

# ---- one timed run -----------------------------------------------------------
RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
timed_run() {   # $1=initiating host  $2=src(spec)  $3=dst(spec)  $4=dest host  $5=flag
  local ih="$1" src="$2" dst="$3" dh="$4" flag="${5:-}" out bin
  bin="$(hblit "$ih")"
  prep_run "$dh"
  # /proc/uptime bracket INSIDE one ssh (round trip outside the window); the
  # destination-side sync is inside it, so durability is paid by both arms.
  out="$(hssh "$ih" "a=\$(awk '{print int(\$1*1000)}' /proc/uptime)
'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/pf-client.err; rc=\$?
sync
b=\$(awk '{print int(\$1*1000)}' /proc/uptime); echo \"R:\$((b-a)),\${rc}:R\"" | nocr \
    | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
  RUN_VALID=yes
  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
  cold_ok || RUN_VALID=no
}

# ---- arms: the ONLY variable is which host's CLI runs -------------------------
CUR_W=""; CUR_FLAG=""
arm_srcinit()  {   # source host pushes to the dest daemon
  local cell="$1" rid="$2" sh="$3" dh="$4"
  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/${SESSION_TAG}_${cell}_${rid}/" "$dh" "$CUR_FLAG"
  hssh "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" 2>/dev/null || true
}
arm_destinit() {   # dest host pulls from the source daemon
  local cell="$1" rid="$2" sh="$3" dh="$4"
  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}" "$dh" "$CUR_FLAG"
  hssh "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" 2>/dev/null || true
}

CSV="$OUT_DIR/runs.csv"; echo "cell,arm,build,initiator,run,ms,exit,drain,cold,valid" > "$CSV"
META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"

run_pair_loop() {   # cell src_host dest_host
  local cell="$1" sh="$2" dh="$3"
  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
  while (( valid < RUNS && attempts < max )); do
    attempts=$(( attempts + 1 ))
    local order pair=yes rowA="" rowB="" arm rid
    if (( slot % 2 )); then order="A B"; else order="B A"; fi
    for arm in $order; do
      local aname init
      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
      rid="${aname}_s${slot}a${attempts}"
      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh"; else arm_destinit "$cell" "$rid" "$sh" "$dh"; fi
      [[ "$RUN_VALID" == yes ]] || pair=no
      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
      [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
    done
    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
  done
  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
  else echo "$cell,$attempts,yes" >> "$META"; fi
}

stage_fixtures() {
  local h w want got
  for h in s m; do
    for w in large small mixed; do
      want="$(eval echo "\$FIX_COUNT_$w")"
      got="$(hssh "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -dc '0-9')"; got="${got:-0}"
      if [[ "$got" == "$want" ]]; then log "  $(hname "$h"):src_$w verified ($got files)"; continue; fi
      log "  $(hname "$h"):src_$w has $got/$want — staging from skippy (untimed)"
      if [[ "$h" == s ]]; then
        sssh "mkdir -p '$S_MODULE'; cp -r '$S_MODULE/pull_src_$w/src_$w' '$S_MODULE/src_$w' 2>/dev/null || true"
      else
        sssh "'$S_BLIT' copy '$S_MODULE/src_$w' '$M_IP:$PORT:/bench/src_$w/' --yes" >/dev/null 2>&1 \
          || die "staging src_$w onto magneto failed"
      fi
      got="$(hssh "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -dc '0-9')"; got="${got:-0}"
      [[ "$got" == "$want" ]] || die "$(hname "$h"):src_$w still $got/$want after staging"
      log "  $(hname "$h"):src_$w staged ($got files)"
    done
  done
}

compute_verdicts() {
  python3 - "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PY'
import csv, sys
runs_p, meta_p, sum_p, ver_p = sys.argv[1:5]
rows = list(csv.DictReader(open(runs_p)))
meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
by, void = {}, {}
for r in rows:
    k = (r["cell"], r["arm"])
    if r["valid"] == "yes": by.setdefault(k, []).append(int(r["ms"]))
    else: void[k] = void.get(k, 0) + 1
def med(v):
    v = sorted(v); n = len(v)
    return v[n//2] if n % 2 else (v[n//2-1] + v[n//2]) // 2
def complete(c):
    if c not in meta or meta[c]["complete"] != "yes": return False
    arms = [a for (cc, a) in by if cc == c]
    return "srcinit" in arms and "destinit" in arms
with open(sum_p, "w") as f:
    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
    for (c, a) in sorted(by):
        if not complete(c): continue
        v = by[(c, a)]
        sp = round(100.0*(max(v)-min(v))/max(min(v), 1), 1)
        f.write(f"{c},{a},{med(v)},{sum(v)//len(v)},{min(v)},{sp},{void.get((c,a),0)},{meta[c]['pairs_attempted']}\n")
with open(ver_p, "w") as f:
    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
    for c in sorted(meta):
        if not complete(c):
            f.write(f"{c},invariance,srcinit,destinit,,,,1.10,INCOMPLETE\n"); continue
        s, d = med(by[(c,"srcinit")]), med(by[(c,"destinit")])
        hi, lo = max(s, d), min(s, d)
        ok = 10*hi <= 11*lo          # max/min <= 1.10, integer-exact
        f.write(f"{c},invariance,srcinit,destinit,{s},{d},{hi/lo:.3f},1.10,{'PASS' if ok else 'FAIL'}\n")
PY
}

on_exit() {
  local rc=$?
  daemon_stop s || true; daemon_stop m || true
  sssh "rm -rf '$S_MODULE'/${SESSION_TAG}_* 2>/dev/null; true" 2>/dev/null || true
  mssh "rm -rf '$M_MODULE'/${SESSION_TAG}_* 2>/dev/null; true" 2>/dev/null || true
  ssh "${SSH_MUX[@]}" -O exit "$S_SSH" 2>/dev/null || true
  ssh "${SSH_MUX[@]}" -O exit "$M_SSH" 2>/dev/null || true
  rm -rf "$MUX" 2>/dev/null || true
  exit $rc
}
trap on_exit EXIT

main() {
  log "OUT_DIR=$OUT_DIR"
  preflight
  write_manifest
  [[ "$PREFLIGHT_ONLY" == 1 ]] && { log "PREFLIGHT_ONLY: gates + manifest passed; nothing started, nothing timed."; exit 0; }
  daemon_start s; daemon_start m
  log "staging fixtures (untimed)"; stage_fixtures

  local w carrier flag
  for w in mixed small large; do          # mixed first: P1 lives there
    for carrier in tcp grpc; do
      [[ "$carrier" == grpc && "$w" != mixed ]] && continue   # grpc = carrier control on mixed only
      [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
      CUR_W="$w"; CUR_FLAG="$flag"
      want_cell "sm_${carrier}_${w}" && run_pair_loop "sm_${carrier}_${w}" s m
      want_cell "ms_${carrier}_${w}" && run_pair_loop "ms_${carrier}_${w}" m s
    done
  done

  daemon_stop s; daemon_stop m
  compute_verdicts
  log ""; log "=== SUMMARY (cold BOTH ends, drained, durable; $RUNS valid pairs/cell; ABBA) ==="
  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
  log ""; log "=== VERDICTS (invariance: max/min <= 1.10; grpc = carrier control) ==="
  column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
  log "runs: $CSV"
}
main "$@"
