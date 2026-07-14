Reading additional input from stdin...
OpenAI Codex v0.144.3
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f615e-5df4-7522-8055-ed8ac528c855
--------
user
Review the Mac<->Mac benchmark instrument at HEAD (commit 8830fda). Run: git show 8830fda, and read the four files at HEAD.

NO DATA HAS EVER BEEN TAKEN. Eight prior review rounds, 90+ defects, all accepted. The decision rule was REWRITTEN and simplified (rev 8), then round 8 found a hole in the rewrite itself, which rev 9 closes.

Files:
- scripts/otp12pf_mac_verdict.py      the decision rule
- scripts/otp12pf_mac_verdict_test.py 29 cases + 10 mutations + direct rule tests
- scripts/bench_otp12pf_mac.sh        the harness (bash 3.2 — macOS has no associative arrays)
- docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md   rev 9, the spec

THE RULE, whole:
  per cell: paired ABBA differences d_i = destinit_i - srcinit_i; the median; one EXACT
  order-statistic CI (coverage >= 95%); and the full RANGE [min, max].
  T_pos = min(src_median/10, 230ms); T_neg = -min(src_median/11, 230ms).
  B = the arm bias the CLEAN controls could not exclude (max |CI bound| over clean controls).
    EFFECT    CI_lo >= T_pos + B          (a positive claim may use the CI)
    INVERTED  CI_hi <= T_neg - B
    NONE      the FULL RANGE lies inside (T_neg + B, T_pos - B)   <-- every pair, not the median
    UNCLEAR   otherwise
  Every control must be NONE at T/2 (full-range too) or NO measurand verdict is read at all.
  RUNS = 8, and only 8. There is NO escalation.
  The cells are INTERLEAVED slot-major, so the controls share the measurands' window.
  The 1.10 bar is reported and takes no part in inference. The sign test is reported, not decided on.

WHY A NULL USES THE RANGE AND AN EFFECT USES THE CI: the >=95% CI is the NARROWEST valid
interval, so at n>8 it TRIMS outliers -- a bimodal arm then gives a narrow median CI and a
FALSE NULL (round 8: codex drove CI=[1,1] from modes at +-110). An equivalence claim must not
be reachable by trimming away the pairs that contradict it. A positive claim may tolerate a
few stragglers.

The question is not "is this nice code". It is: can this instrument produce a confident, WRONG ANSWER? This is a scientific-instrument correctness review.

REVIEW, in priority order:
1. THE ASYMMETRY (range for a null, CI for an effect). Is it sound, or does it break something? At n=8 the CI IS the range, so the two coincide -- does anything in the code or the spec depend on them differing? Is the EFFECT side now the weak one: can a bimodal arm, or a few outliers, manufacture a false EFFECT through the CI?
2. B, the control-bias carry. Is it computed from the right quantity? Can B behave perversely, or can it swing the verdict perversely (e.g. a MORE biased control making an EFFECT easier)? Is `max |CI bound|` right, or should it be the median, or the range? What if the controls' bias points the OPPOSITE way to the measurand's effect?
3. THE INTERLEAVE. run_all_cells / run_one_pair: check the void/retry accounting, the ABBA order, the CSV rows, the meta completeness, and whether a cell that exhausts its attempts corrupts the others. Bash 3.2: no associative arrays, no `${x^^}`, no `mapfile`.
4. Removing the escalation: does anything still reference it? Can RUNS be anything but 8?
5. THE TWO RECURRING CLASSES, which have appeared in EVERY round -- assume both are present:
   (a) "fixed the branch I was shown, not the class" (e.g. a fail-open pgrep fixed in one gate, left in its duplicate; the drain fixed by VALUE and left failing by STATUS);
   (b) "a fix that never executed" (SETTLE_MS had never run for three revisions; the settle is now performed and MEASURED inside the same python process as the fsync walk, reported as settled_ms, and the pair VOIDS if it did not elapse).
   FIND THE NEXT ONE. Which claimed protection does not execute, or executes but cannot fail? Prefer RUNNING things to reading them.
6. Is the guard non-vacuous? 10 mutations + 4 direct rule tests. Which fixes have NO mutation? Note one mutation was REPLACED by a direct test because it could not be killed through a session at n=8 (CI == RANGE there) -- is that reasoning right?

If you have shell access, use it: run the engine on synthetic sessions; construct synthetic sessions that would expose a wrong verdict, and check whether any gate can pass while unable to answer.

Output a concise markdown findings list -- file:line, severity (BLOCKER/HIGH/MEDIUM/LOW), rationale -- then reproductions with actual commands/output, then a final VERDICT line: READY TO RUN or NOT READY TO RUN. Be concise; do not invoke skills.
codex
I’ll trace the decision rule, harness execution paths, and guard coverage against commit `8830fda`, then run focused synthetic sessions and shell-path checks. I’ll report only correctness findings with reproducible evidence.
exec
/bin/zsh -lc 'wc -l scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md && rg -n "RUNS|ESCALAT|range|CI|bias|interleav|settled|SETTLE|run_all_cells|run_one_pair|pgrep|drain" scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     396 scripts/otp12pf_mac_verdict.py
     420 scripts/otp12pf_mac_verdict_test.py
    1086 scripts/bench_otp12pf_mac.sh
     484 docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
    2386 total
scripts/otp12pf_mac_verdict.py:21:    CI  = exact distribution-free order-statistic interval on the population median, the
scripts/otp12pf_mac_verdict.py:24:    RANGE = [min(d), max(d)], and a NULL is judged on the RANGE, never on a trimmed CI.
scripts/otp12pf_mac_verdict.py:27:    outliers, and a bimodal arm then yields a narrow median CI and a FALSE verdict. grok
scripts/otp12pf_mac_verdict.py:38:new case to walk past, because they partition the CI's position relative to +-T)
scripts/otp12pf_mac_verdict.py:39:    EFFECT    CI_lo >= +T                 destination-initiated is slower, by >= T
scripts/otp12pf_mac_verdict.py:40:    INVERTED  CI_hi <= -T                 source-initiated is slower, by >= T
scripts/otp12pf_mac_verdict.py:41:    NONE      -T < CI_lo and CI_hi < +T   an effect of size T is EXCLUDED (equivalence)
scripts/otp12pf_mac_verdict.py:42:    UNCLEAR   anything else               the CI spans the threshold: no answer
scripts/otp12pf_mac_verdict.py:55:    * The sign test is REPORTED, not decided on. At n=8 the CI already implies it
scripts/otp12pf_mac_verdict.py:56:      (CI_lo >= T > 0 means every pair is >= T), so making it a second gate only added
scripts/otp12pf_mac_verdict.py:59:      precedence stack. A bimodal arm widens the CI, and a wide CI lands in UNCLEAR --
scripts/otp12pf_mac_verdict.py:143:    for k in range(1, n // 2 + 1):
scripts/otp12pf_mac_verdict.py:144:        cov = 1.0 - 2.0 * sum(comb(n, i) for i in range(k)) / (2.0 ** n)
scripts/otp12pf_mac_verdict.py:157:    tail = sum(comb(n, i) for i in range(min(k, n - k) + 1))
scripts/otp12pf_mac_verdict.py:173:    EFFECT/INVERTED use the >=95% CI on the median; NONE uses the FULL RANGE. At the
scripts/otp12pf_mac_verdict.py:174:    registered n=8 these coincide (the CI IS the range), so nothing can be trimmed either
scripts/otp12pf_mac_verdict.py:179:    a >=95% CI at n>8 TRIMS outliers, so a BIMODAL arm produces a NARROW median CI and a
scripts/otp12pf_mac_verdict.py:180:    FALSE NULL (driven: CI = [1,1] from modes at +-110). An equivalence claim must never be
scripts/otp12pf_mac_verdict.py:182:    bimodality needs no special branch: it cannot hide from the range.
scripts/otp12pf_mac_verdict.py:213:# ---- pass 2: the controls certify the rig, and BOUND its residual bias ---------------
scripts/otp12pf_mac_verdict.py:217:# codex, BLOCKER). The bias the controls FAIL TO EXCLUDE is therefore carried into the
scripts/otp12pf_mac_verdict.py:222:#         The RANGE, not the CI: the CI is an interval for the MEDIAN and it TRIMS, and a
scripts/otp12pf_mac_verdict.py:224:#     an EFFECT must clear  T + B     (bias could be INFLATING it)
scripts/otp12pf_mac_verdict.py:225:#     a NULL   must fit in  T - B     (bias could be MASKING an effect)
scripts/otp12pf_mac_verdict.py:243:# ---- pass 3: grade the measurands, against thresholds widened by the control bias -----
scripts/otp12pf_mac_verdict.py:254:        # ...and a NULL must survive the TIGHTER bound: bias could be masking an effect.
scripts/otp12pf_mac_verdict.py:280:    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,range_lo,range_hi,"
scripts/otp12pf_mac_verdict.py:309:    why = ("cells short of their %d pairs, or with a CI below the registered %.0f%% "
scripts/otp12pf_mac_verdict.py:321:           % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d], T/2=%d)"
scripts/otp12pf_mac_verdict.py:350:    why = ("the CI spans the threshold in: %s. The rig could not resolve an effect of "
scripts/otp12pf_mac_verdict.py:357:       "the arm bias they could NOT exclude: an EFFECT must clear T+B, a NULL must fit in T-B."]
scripts/otp12pf_mac_verdict.py:363:    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] range=[%+5d,%+5d] "
scripts/otp12pf_mac_verdict.py:385:        "the median CI, which at n>8 would TRIM the outliers that contradict it. An EFFECT",
scripts/otp12pf_mac_verdict.py:386:        "uses the CI. That is why bimodality needs no special branch: it cannot hide from",
scripts/otp12pf_mac_verdict.py:387:        "the range.",
scripts/otp12pf_mac_verdict.py:390:        "and take NO part in this verdict, which is decided only by the paired CI against",
scripts/otp12pf_mac_verdict_test.py:34:    medians and the CI on the PAIRED differences, and the two only disagree when the
scripts/otp12pf_mac_verdict_test.py:44:        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,"
scripts/otp12pf_mac_verdict_test.py:45:                   "exit,drain,cold,valid".split(","))
scripts/otp12pf_mac_verdict_test.py:56:                            "drained_1x2s", "cold", "yes"])
scripts/otp12pf_mac_verdict_test.py:58:                            "drained_1x2s", "cold", "yes"])
scripts/otp12pf_mac_verdict_test.py:63:            # believe it (a 1-pair CSV once graded as a full cell at 0% CI coverage).
scripts/otp12pf_mac_verdict_test.py:96:    ("codex r2: the inverting threshold is -src/11, not -src/10 (CI [-190,0] @ 2000)",
scripts/otp12pf_mac_verdict_test.py:116:    ("one huge outlier must not manufacture a reproduction (the CI's LOWER bound decides)",
scripts/otp12pf_mac_verdict_test.py:120:    ("grok r9: a LONG cell (16 pairs) is INCOMPLETE — a CI at n>8 TRIMS the pairs that contradict it",
scripts/otp12pf_mac_verdict_test.py:129:    ("grok r2: a bar-FAIL control whose CI crosses zero blocks every verdict",
scripts/otp12pf_mac_verdict_test.py:139:    ("codex r5: ...and so does one with a single zero pair (CI [0,230])",
scripts/otp12pf_mac_verdict_test.py:144:    ("grok r5: ...and a non-directional one (CI [-10,230])",
scripts/otp12pf_mac_verdict_test.py:171:    ("an exact 10% effect is reportable on a bias-free rig (it was once unreachable)",
scripts/otp12pf_mac_verdict_test.py:175:    # codex r8, BLOCKER: a control at +5 is "clean", but that 5ms of arm bias may be
scripts/otp12pf_mac_verdict_test.py:177:    # 5 rig. It must not be banked as a reproduction. B carries the bias the controls
scripts/otp12pf_mac_verdict_test.py:179:    ("codex r8: an effect of exactly T is NOT a reproduction when the controls carry bias",
scripts/otp12pf_mac_verdict_test.py:183:    ("grok r9: a null must also survive the TIGHTER bound (bias could be MASKING an effect)",
scripts/otp12pf_mac_verdict_test.py:187:    ("codex r8: ...and the same effect IS one once the rig is bias-free",
scripts/otp12pf_mac_verdict_test.py:222:    ("grok r3: n=1 with complete=yes must not grade at 0% CI coverage",
scripts/otp12pf_mac_verdict_test.py:258:    ("EFFECT is decided on the CI's MIDPOINT, not its lower bound (an outlier reproduces)",
scripts/otp12pf_mac_verdict_test.py:262:    ("the NULL is not tightened by the control bias -- a masked effect reads as a null (grok r9)",
scripts/otp12pf_mac_verdict_test.py:267:    ("the control's residual bias is not carried into the measurand (codex r8)",
scripts/otp12pf_mac_verdict_test.py:292:    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
scripts/otp12pf_mac_verdict_test.py:307:        # ci narrow (outliers trimmed), range wide: a bimodal arm. MUST NOT be NONE.
scripts/otp12pf_mac_verdict_test.py:308:        ("bimodal: CI=[1,1] but range=[-110,110], T=73", (1, 1, -110, 110, 73, -66), "UNCLEAR"),
scripts/otp12pf_mac_verdict_test.py:309:        ("clean: CI and range both inside T",            (2, 3, -4, 3, 73, -66),      "NONE"),
scripts/otp12pf_mac_verdict_test.py:322:    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
scripts/otp12pf_mac_verdict_test.py:331:    for _ in range(200):
scripts/otp12pf_mac_verdict_test.py:332:        d = [rr.randint(-500, 500) for _ in range(8)]
scripts/otp12pf_mac_verdict_test.py:335:            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
scripts/otp12pf_mac_verdict_test.py:339:        print("  %-46s -> %-8s ok" % ("n=8: the >=95% CI IS [min,max] (200 draws)", "identity"))
scripts/otp12pf_mac_verdict_test.py:361:    for _ in range(n):
scripts/otp12pf_mac_verdict_test.py:362:        got = session(measurand_d=[rng.randint(-600, 600) for _ in range(8)],
scripts/otp12pf_mac_verdict_test.py:364:                      control_d=[rng.randint(-300, 300) for _ in range(8)],
scripts/otp12pf_mac_verdict_test.py:379:        for i in range(0, len(subs), 2):
scripts/otp12pf_mac_verdict_test.py:413:    print("The RULE, called directly (a session at n=8 cannot separate CI from RANGE):")
scripts/bench_otp12pf_mac.sh:37:# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
scripts/bench_otp12pf_mac.sh:63:#   * Gates FAILED OPEN: pgrep errors read as "quiet"; a failed `top` read as 0%
scripts/bench_otp12pf_mac.sh:65:#     read as zero and CERTIFIED drainage; the drain watched a hardcoded `disk0`
scripts/bench_otp12pf_mac.sh:124:RUNS="${RUNS:-8}"
scripts/bench_otp12pf_mac.sh:141:for _v in SETTLE_MS LOAD_MAX DRAIN_ITERS DRAIN_QUIET DRAIN_MBPS DELTA_REF_MS TIMER_TOLERANCE_MS; do
scripts/bench_otp12pf_mac.sh:153:SETTLE_MS=250              # equal pre-fsync window on BOTH arms
scripts/bench_otp12pf_mac.sh:163:# So THE SETTLE HAS NEVER RUN — not once, in any revision, since 24660ae introduced
scripts/bench_otp12pf_mac.sh:171:SETTLE_SEC="$(awk -v m="$SETTLE_MS" 'BEGIN{printf "%.3f", m/1000}')"
scripts/bench_otp12pf_mac.sh:172:[[ "$SETTLE_SEC" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "FATAL: settle seconds did not compute ('$SETTLE_SEC')" >&2; exit 1; }
scripts/bench_otp12pf_mac.sh:200:# It is marked EXPLICITLY here, never inferred from the wording of a message —
scripts/bench_otp12pf_mac.sh:323:# THE ONLY process probe in this harness. pgrep: 0 = found, 1 = none, >=2 = ERROR.
scripts/bench_otp12pf_mac.sh:327:pgrep_state() {
scripts/bench_otp12pf_mac.sh:329:  raw="$(hrun "$h" "pgrep -x '$pat' >/dev/null 2>&1; rc=\$?
scripts/bench_otp12pf_mac.sh:342:    case "$(pgrep_state "$h" "$p")" in
scripts/bench_otp12pf_mac.sh:400:# --- the drain device: RESOLVED, never hardcoded (grok) ------------------------
scripts/bench_otp12pf_mac.sh:413:  # it is not a harmless default, it is a FALSE QUIET that certifies drainage on a
scripts/bench_otp12pf_mac.sh:428:  # while the global it sets was discarded, so the drain then had no device and
scripts/bench_otp12pf_mac.sh:431:    log "$(hname "$h"): cannot resolve the PHYSICAL disk behind $p (got '$dev') — a drain that watches the wrong device certifies a disk the data never touched, and on APFS a synthesized disk can read idle while the physical store saturates"
scripts/bench_otp12pf_mac.sh:435:  # make every sample non-numeric, and the drain must never read that as quiet.
scripts/bench_otp12pf_mac.sh:439:    log "$(hname "$h"): iostat does not report numeric throughput for $dev (got '$probe') — cannot certify drainage"
scripts/bench_otp12pf_mac.sh:443:  log "  drain device on $(hname "$h"): $dev (backs $p, idle probe ${probe} MB/s)"
scripts/bench_otp12pf_mac.sh:449:# BY DIRECTION. SETTLE_MS makes the window EQUAL on both arms; the residual is the
scripts/bench_otp12pf_mac.sh:466:for _ in range(5):
scripts/bench_otp12pf_mac.sh:476:  local rtt_max=$(( SETTLE_MS / 4 ))
scripts/bench_otp12pf_mac.sh:478:    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
scripts/bench_otp12pf_mac.sh:479:  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
scripts/bench_otp12pf_mac.sh:484:  # RUNS=8, and ONLY 8. The RUNS=16 escalation was removed (owner, 2026-07-14): a null is
scripts/bench_otp12pf_mac.sh:488:  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
scripts/bench_otp12pf_mac.sh:513:    # THE SAME pgrep FAIL-OPEN AS THE QUIESCENCE GATE, IN A DUPLICATE SITE I DID NOT
scripts/bench_otp12pf_mac.sh:514:    # TOUCH (round-5 codex, HIGH). `if hrun ... pgrep; then die; fi` reads rc>=2 (a
scripts/bench_otp12pf_mac.sh:518:    case "$(pgrep_state "$h" blit-daemon)" in
scripts/bench_otp12pf_mac.sh:531:    resolve_disk "$h" || die "$(hname "$h"): cannot establish the drain device — refusing"
scripts/bench_otp12pf_mac.sh:534:  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
scripts/bench_otp12pf_mac.sh:550:    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
scripts/bench_otp12pf_mac.sh:551:    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
scripts/bench_otp12pf_mac.sh:552:    echo "# drain_disk_nagatha=$N_DISK drain_disk_q=$Q_DISK ssh_rtt_ms=$SSH_RTT_MS"
scripts/bench_otp12pf_mac.sh:582:  # The daemon's OWN pid, from $! — not `pgrep | head -1`, which picks the first of
scripts/bench_otp12pf_mac.sh:644:# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
scripts/bench_otp12pf_mac.sh:646:drain_host() {   # $1 = host. Echoes drained_<n>x2s | DRAIN-TIMEOUT | DRAIN-ERROR
scripts/bench_otp12pf_mac.sh:664:  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
scripts/bench_otp12pf_mac.sh:667:  # ONE token, or it is an error -- AND the probe must have EXITED cleanly. A drain that
scripts/bench_otp12pf_mac.sh:668:  # printed `drained_*` and THEN failed is not a drain (codex r8: I fixed the value and
scripts/bench_otp12pf_mac.sh:671:    drained_[0-9]*x2s) echo "$out" ;;
scripts/bench_otp12pf_mac.sh:678:  # Purge BOTH ends first — the purge itself dirties the disk, so a drain certified
scripts/bench_otp12pf_mac.sh:684:  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
scripts/bench_otp12pf_mac.sh:685:  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
scripts/bench_otp12pf_mac.sh:686:  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
scripts/bench_otp12pf_mac.sh:690:RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
scripts/bench_otp12pf_mac.sh:691:fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
scripts/bench_otp12pf_mac.sh:693:  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
scripts/bench_otp12pf_mac.sh:696:  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
scripts/bench_otp12pf_mac.sh:706:  out="$(hrun "$1" "$(hpy "$1") - '$SETTLE_SEC' '$2' <<'PYEOF'
scripts/bench_otp12pf_mac.sh:712:settled_ms = int((time.monotonic() - t0) * 1000)
scripts/bench_otp12pf_mac.sh:714:    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
scripts/bench_otp12pf_mac.sh:727:print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
scripts/bench_otp12pf_mac.sh:732:settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
scripts/bench_otp12pf_mac.sh:742:  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
scripts/bench_otp12pf_mac.sh:749:  if ! settle_ok "$RUN_SETTLED"; then
scripts/bench_otp12pf_mac.sh:750:    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
scripts/bench_otp12pf_mac.sh:767:  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
scripts/bench_otp12pf_mac.sh:818:run_one_pair() {   # $1=idx $2=cell $3=srchost $4=dsthost $5=fixture $6=flag $7=slot -> 0 if VALID
scripts/bench_otp12pf_mac.sh:831:    local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
scripts/bench_otp12pf_mac.sh:833:    log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (fsync ${RUN_FLUSH}ms, settle ${RUN_SETTLED}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
scripts/bench_otp12pf_mac.sh:844:run_all_cells() {
scripts/bench_otp12pf_mac.sh:845:  local slot i cell sh dh w flag max=$(( 2 * RUNS )) n=${#CELL_TABLE[@]}
scripts/bench_otp12pf_mac.sh:847:  for (( slot = 1; slot <= RUNS; slot++ )); do
scripts/bench_otp12pf_mac.sh:848:    log "=== SLOT $slot / $RUNS (every cell takes one pair before any cell takes the next) ==="
scripts/bench_otp12pf_mac.sh:853:        if run_one_pair "$i" "$cell" "$sh" "$dh" "$w" "${flag:-}" "$slot"; then break; fi
scripts/bench_otp12pf_mac.sh:859:    if (( ${CELL_VALID[$i]:-0} < RUNS )); then
scripts/bench_otp12pf_mac.sh:861:      log "  $cell INCOMPLETE: ${CELL_VALID[$i]}/$RUNS valid pairs"
scripts/bench_otp12pf_mac.sh:894:  REQUIRED_PAIRS="$RUNS" SESSION_VOID_REASON="$SESSION_VOID_REASON" \
scripts/bench_otp12pf_mac.sh:946:  local h="$1" d ms files bytes settled
scripts/bench_otp12pf_mac.sh:950:  read -r ms files bytes settled <<<"$(fsync_tree "$h" "$d")"
scripts/bench_otp12pf_mac.sh:956:  # THE SETTLE MUST BE PROVED, NOT NARRATED (round-6, both reviewers). The old check
scripts/bench_otp12pf_mac.sh:960:  if ! settle_ok "$settled"; then
scripts/bench_otp12pf_mac.sh:961:    log "  [BROKEN] fsync/settle — THE SETTLE DID NOT ELAPSE: measured ${settled}ms, want >= ${SETTLE_MS}ms"
scripts/bench_otp12pf_mac.sh:964:  log "  [OK]     fsync/settle — 2 files/6 bytes walked in ${ms}ms; settle MEASURED at ${settled}ms (>= ${SETTLE_MS}ms), counts VERIFIED"
scripts/bench_otp12pf_mac.sh:993:    # the assignment was discarded and the drain loop below then had no device and
scripts/bench_otp12pf_mac.sh:996:    if resolve_disk "$h"; then log "  [OK]     drain device  (resolved via the APFS physical store)"
scripts/bench_otp12pf_mac.sh:997:    else log "  [BROKEN] drain device  — could not resolve the physical disk"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); fi
scripts/bench_otp12pf_mac.sh:1000:    case "$(pgrep_state "$h" blit-daemon)" in
scripts/bench_otp12pf_mac.sh:1007:    local dr; dr="$(drain_host "$h")"
scripts/bench_otp12pf_mac.sh:1009:      drained*)      log "  [OK]     drain loop    ($dr)" ;;
scripts/bench_otp12pf_mac.sh:1010:      DRAIN-TIMEOUT) log "  [FIRED]  drain loop    — the disk is genuinely busy; the gate would void the pair"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
scripts/bench_otp12pf_mac.sh:1011:      *)             log "  [BROKEN] drain loop    — the probe could not answer ('$dr')"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
scripts/bench_otp12pf_mac.sh:1055:  echo "cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
scripts/bench_otp12pf_mac.sh:1060:  run_all_cells
scripts/bench_otp12pf_mac.sh:1070:  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:7:> Per cell, take the **paired** ABBA differences, their median, and one **exact CI**.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:8:> Compare that CI against **one threshold** `T = min(10% of the source arm, 230 ms)`.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:9:> Four states, exhaustive by construction: **EFFECT** (CI clears +T), **INVERTED** (CI
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:10:> clears −T), **NONE** (CI lies inside ±T — an effect of size T is *excluded*), **UNCLEAR**
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:11:> (the CI spans a threshold). **Every control must be NONE at T/2, or no verdict about the
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:22:> **Revisions 3, 4 and 5 of this document asserted that a fixed, equal `SETTLE_MS`
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:69:- Round 7 (`1e03063`): **NOT READY** from both again — the drain fails open (a
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:70:  `drained_*` value followed by a non-zero exit), rev 7's text contradicted itself, and
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:121:i.e. if the CI excludes one. Otherwise the verdict is `UNCLEAR`, which is **not** a null.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:160:`RUNS=8`, ABBA-counterbalanced, pair-void. **All six cells must be present and
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:177:DECISION RULE, not in the measurement**: a 1 ms effect reported as a reproduction; a
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:191:      CI = EXACT distribution-free order-statistic interval on the population median —
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:210:They partition the CI's position relative to the thresholds. **There is no label here for
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:215:| **EFFECT** | `CI_lo >= T_pos + B` — destination-initiated is slower, by at least T |
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:216:| **INVERTED** | `CI_hi <= T_neg − B` — source-initiated is slower, by at least T |
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:218:| **UNCLEAR** | anything else — the CI spans a threshold; the rig cannot answer |
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:220:**A NULL IS JUDGED ON THE RANGE, AN EFFECT ON THE CI — and that asymmetry is the point
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:221:(round-8, codex, BLOCKER).** The ≥95% CI is the *narrowest* valid interval, so at n>8 it
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:222:**trims outliers**; a **bimodal** arm then yields a *narrow median CI* and a **false null**
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:223:(codex drove `CI = [1,1]` from modes at ±110). **An equivalence claim must never be
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:225:the CI: pairs clearing T is evidence, and a few stragglers do not undo it.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:227:*This is also why bimodality needs no special branch — it cannot hide from the range. The
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:241:`T/2 = 50` certifies — but *that 49 ms of arm bias may be riding in the measurand too*, so a
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:242:measurand effect of exactly `T` could be half real and half rig. The bias the controls **fail
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:245:    B = max over clean controls of the largest |CI bound|
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:246:    an EFFECT must clear   T + B     (the bias could be INFLATING it)
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:247:    a NULL must fit inside T − B     (the bias could be MASKING an effect)
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:264:1. **INCOMPLETE** — any registered cell short of its `RUNS` pairs, or with a CI below 95%
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:284:- **The sign test is reported, not decided on.** At n=8 the CI already implies it
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:285:  (`CI_lo >= T > 0` means *every* pair clears T), so making it a second gate only added
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:288:  precedence stack.** A bimodal arm **widens the CI**, and a wide CI lands in `UNCLEAR` —
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:299:The old `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:300:**full range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR`
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:306:trimming outliers, and a bimodal arm then yields a narrow median CI and a false verdict:
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:318:`DELTA_REF_MS`, `SETTLE_MS`, `LOAD_MAX`, `DRAIN_MBPS` and the rest are **literals** in
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:371:   **⛔ AND UNTIL REV 6, THE SETTLE NEVER RAN AT ALL (see the correction at the top).**
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:404:gate executes" while never touching drain, purge, daemon, fsync/settle, stale-daemon or
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:414:function exists to set, so the drain then had no device and blamed the harness).
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:417:  Mac. `pgrep` rc≥2 is an **error**, not "quiet" (rev 3 could not tell them apart).
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:438:  (rev 3 read it as zero and **certified drainage**).
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:449:- **DAEMON LIFECYCLE** — the pid comes from `$!` (not `pgrep | head -1`, which picks
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:457:- **THE VERDICT ENGINE'S OWN GUARD TEST RUNS AT PREFLIGHT — cases AND mutations.**

exec
/bin/zsh -lc "sed -n '1,260p' AGENTS.md && sed -n '1,320p' .agents/repo-guidance.md && sed -n '1,260p' .agents/state.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# Agent Guidance

## Prime Invariants
<!-- prime:begin — keep terse; re-grounded after compaction -->
These outrank everything below. After a context compaction, re-read this block from AGENTS.md before continuing.

- Words first. Answer questions and musings in words; act only on an explicit instruction or go. A handed-over report, plan, or spec is evidence to assess, not a decision to implement.
- No code change without an approved plan; docs and other non-code edits don't need one (e.g. a README). When unsure, treat it as code. Do not expand scope without approval.
- Commit each slice as it lands; never leave finished work uncommitted. History-rewrite and destructive or outward-facing actions always need an explicit go. Push policy: see `.agents/push-policy.md`.
- Repo is memory. Durable truth lives in the repo, not chat or working memory. Under context pressure, re-ground from AGENTS.md; prefer a fresh session when degraded.
<!-- prime:end -->

## Repo-Specific Guidance

@.agents/repo-guidance.md

Repo-specific rules live in `.agents/repo-guidance.md`, imported above (read it directly if your harness does not process `@` imports). It extends this file and never overrides it — flag any genuine conflict.

## Universal Invariants

- The Prime Invariants above are the hardest-to-reverse rules; this section adds the rest.
- Agent-local or harness-local memory stores kept outside the repo are not durable memory, on any harness. Persist project-specific durable knowledge into the repo's `.agents/` files; reserve out-of-repo stores for genuinely cross-project facts (owner identity, preferences).
- Record important repo facts, decisions, invariants, verification rules, non-goals, and open questions in repo files, or explicitly report them as unrecorded. Write them generalized, tied to repo evidence or explicit human intent, so they make sense without the conversation that produced them — never as transient chat wording. Label inferred-but-unverified facts as assumptions until repo evidence or explicit human approval supports them.
- Keep one canonical location for each durable truth. Prefer pointers over duplicating the same rule; never keep a second copy of a count or enumeration another doc owns.
- Establish one immediately discoverable current-state entry point (`.agents/state.md`). Do not reconstruct current state from chat, long journals, or tool-local memory.
- When repo documents disagree, flag the conflict instead of silently choosing whichever source is convenient. Code and tests are evidence for behavior; approved plans and guidance are evidence for intent.
- Specific over generic: an explicit authority or scope boundary, or a rule or decision whose wording removes discretion for the case it names ("unconditional", "no per-run choice", "deterministic"), outranks every generic default for that case — flag-conflicts, one-canonical-location, smallest-guidance-set included. Apply it as written; do not reopen the case it settles as a conflict or approval question against surrounding repo state such as git history. Generic defaults govern only questions no more specific rule has already resolved.
- Prefer the smallest durable guidance set that fits the repo.
- Do not circumvent a roadblock whose provenance you have not established — a failing test, a guard or assertion, a lint or type error, a `.gitignore` rule, a refusal or permission denial, a config prohibition, a CI gate. Before removing or bypassing one, inspect its origin thoroughly enough to confirm it is not load-bearing; if you cannot, treat it as legitimate and stop or ask.
- Escalate an iterative process on stalled progress, never on duration. Each cycle must bank a verifiable delta — a test moving red→green, a finding closed with its guard proof, a build or type error resolved, a committed slice; a cycle that produces none is a stall. After a few consecutive stalled cycles (state the threshold you are using; default ~2-3), stop and surface to a human. A long run that banks a delta each cycle is healthy and must not be capped on duration or turn count.
- `AGENTS.md` is governance only — it must be portable. The test: would this line still be true and useful if copied unchanged into an unrelated repo? Process, invariants, and operator definitions pass. Anything true only of *this* repo — a concrete source path, the repo's own name as a fact, its verification commands, a restatement of current state or the decisions queue — fails and lives in `.agents/`, with `AGENTS.md` pointing to it, never restating it. References to the toolkit's own standard layout — `.agents/state.md`, operator names — are portable and allowed.
- `AGENTS.md` is the toolkit template, installed and replaced whole by governance refresh; no agent hand-edits it. Durable repo-specific rules go to `.agents/repo-guidance.md` and facts to the other `.agents/` files; a proposed `AGENTS.md` edit is out of bounds — question it, do not perform it.

## Session Startup

1. Read `AGENTS.md`, `.agents/repo-guidance.md`, and `.agents/state.md` if present, plus relevant `.agents/` files, before making changes; note any untracked or ignored agent-control files that affect the task.
2. Clone freshness: before trusting `.agents/state.md`, compare this clone against its canonical remote with a read-only check (`git ls-remote <remote> HEAD` against the local ref). Behind or diverged — say so and treat recorded state as possibly stale; unreachable — proceed with a one-line caveat, never block.
3. This repo ships a compaction re-ground hook (Claude Code; other harnesses only as listed in the toolkit's harness-capabilities record); if your harness gates hooks until the workspace is trusted, say what the hook does and run the trust step only on an explicit go — never bypass the gate.

## Source Of Truth

1. Human request.
2. `AGENTS.md`, extended by `.agents/repo-guidance.md` (extends, never overrides).
3. `.agents/state.md` for current work; `.agents/decisions.md` for settled decisions; approved `.agents/playbooks/*`.
4. Current code, tests, and CI as evidence for behavior.
5. Existing docs, only when consistent with current repo evidence.

When sources disagree, apply the flag-conflicts invariant (Universal Invariants): surface the conflict and fix the lower-authority source, or ask which should win.

## Operator Requests

Treat these owner words as process requests:

- `catchup`: re-read `AGENTS.md` (the Prime Invariants in full), `.agents/state.md`, and active repo docs; summarize current state, next action, blockers, and one proposed first action. Make no changes until the human responds.
- `handoff`: a fast save-my-place snapshot — seconds, not minutes. Update `.agents/state.md` `## Now` / `## Next` (and `## Blockers` if something is live) so the next session resumes without chat context: in-flight work, next action, stop. No archive rotation, no re-verification sweep, no mandatory re-anchoring of volatile facts — that hygiene belongs to `drift`. Machine-specific facts (CLI paths, local tool versions, host layout) go to the tracked `.agents/machines.md` under a heading for the current machine, dated, created on first use — never into `.agents/state.md`, which stays portable and may at most point there.
- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves — `AGENTS.md` and `.agents/*` — are in scope as drift targets, not just sources of truth. `drift` also owns the deliberate state-hygiene pass: rotate landed or superseded `## Now` entries verbatim to `docs/history/state-archive.md` (create on first use); re-verify the recorded basis of every parked or blocked item and move anything falsified into `## Blockers` with the new evidence; volatile facts (CI state, counts) carry `as of <commit>` and are re-verified or dropped; push status is never recorded in state files — git owns it, sessions check it live, and unpushed work is mentioned only in the moment it matters — so any recorded push-state line is deleted on sight, not refreshed; a count or enumeration another file owns is pointed to, never copied; machine-specific facts relocate to `.agents/machines.md`, and stale entries there are pruned.
- `decision`: record a settled durable decision in `.agents/decisions.md` and update affected guidance.
- `plan`: draft or update a durable plan before broad implementation work. Plan documents are written for agents, never the owner: self-contained and technical, implementable by a completely cold, less-capable agent — no human-facing summary prose, no chat or session references that need the originating conversation to make sense. The owner does not read plan documents; present every decision a plan needs in chat as roughly 25-50 plain-English words — the problem, the change, the cost or risk — one decision at a time, never a batch, no jargon. Record the owner's approved wording durably (the decisions log, the plan's status line) so the approval survives the chat.
- `playbook <name>`: read `.agents/playbooks/<name>.md` and follow it. Playbooks are approved durable workflows; this operator is how a session invokes one by name. If the named playbook does not exist, say so rather than guessing.

## Verification

Use the repo's current automated verification entry point recorded in `.agents/repo-guidance.md` (Verification).

- For code changes, run the current automated verification before claiming completion.
- When a change ships with a new test, prove the test guards it: temporarily revert the change, confirm the test fails, restore it, confirm everything passes. A test that passes with its fix reverted is vacuous and must be replaced.
- For docs-only changes, code verification is not required unless the docs affect setup, commands, runtime behavior, generated files, or user-visible behavior.
- For behavior that automation does not cover, run the relevant manual check, smoke test, or playtest, or state clearly that it was not run.
- If no verification entry point is recorded yet, identify the likely command from repo evidence, record it, and label uncertainty. Ask the human only when evidence conflicts, no plausible command exists, or the command appears destructive, expensive, credentialed, or otherwise unsafe to run automatically.

## Git Safety

- Never conclude a branch is merged from ancestry alone: `git branch --merged` can lie after an `-s ours` or octopus merge records ancestry without content. Verify the content actually arrived (`git diff <branch> <main>`) before deleting anything or treating work as landed.
- When working through a list of findings or fixes, address exactly one item per commit and commit each before starting the next. Batch sweeps spanning many findings happen only on the owner's explicit request. Whether work happens on a branch is this repo's policy, not this rule's.
- Do not rewrite history or restructure existing commits without explicit owner approval: no `git commit --amend`, `rebase`, `squash`, or force-push, and no reordering or collapsing commits already made. The owner's approval authorizes the scoped commit as announced — it does not authorize a later rewrite of it. Default to a new commit per fix; if history genuinely needs reshaping, stop and ask.

## Final Response

Explain what changed, what was validated, and any remaining risk in plain English.
# Repo-Specific Guidance
<!-- Extends AGENTS.md; never overrides it. Rules and pointers only — state
     lives in .agents/state.md. -->

## Mission Detail

Blit is a high-performance, extensible file enumeration, planning, transfer,
and orchestration platform for local and remote backups, migration, and
cross-platform syncing, with CLI and daemon interfaces (`crates/blit-cli`,
`crates/blit-daemon`), async-aware planning, and Windows/Linux/macOS support.

## Reading Order

This repo predates the toolkit's `.agents/state.md` / `.agents/decisions.md`
convention and keeps its own canonical files at different paths; the
`.agents/` files below are pointer stubs, not duplicates. Read in this order:

1. `docs/STATE.md` — single entry point for current active work, queue, and
   blockers (the canonical equivalent of `.agents/state.md`; see
   `.agents/state.md` for why the path differs).
2. The active plan doc(s) `docs/STATE.md` names (under `docs/plan/`).
3. `REVIEW.md` + `.review/` — review-loop status for in-flight findings.
4. `docs/DECISIONS.md` — settled decisions and supersessions (the canonical
   equivalent of `.agents/decisions.md`).
5. `docs/agent/PROTOCOL.md` — the executable procedures behind the trigger
   vocabulary (`catchup`, `plan`, `decision`, `handoff`, `drift`, plus the
   repo-specific `slice` operator below).
6. Everything else in `docs/` — reference or historical; check its
   `**Status**:` header.
7. Code and tests are ground truth for behavior; plans are ground truth for
   intent. A mismatch is a drift finding, not permission to pick whichever is
   convenient.

`DEVLOG.md` is append-only history — write to it, never read it for current
state. `TODO.md` is the long-horizon backlog; the actionable queue lives in
`docs/STATE.md` and `REVIEW.md`. `.serena/memories/` and any tool-local
memory are scratch, never authoritative.

## Operator Vocabulary (repo-specific extension)

`AGENTS.md`'s Operator Requests section defines the toolkit's generic
vocabulary (`catchup`, `handoff`, `drift`, `decision`, `plan`, `playbook`).
In this repo every one of those words resolves to a procedure in
`docs/agent/PROTOCOL.md`, not to the generic `.agents/state.md`/
`.agents/decisions.md` files directly — read the matching section there and
execute it exactly:

- `catchup` → re-ground from `docs/STATE.md` + active docs; summarize
  now/next/blockers.
- `plan <topic>` → interview the owner, write `docs/plan/<NAME>.md`; no code
  until `**Status**: Active`.
- `decision <topic>` → record in `docs/DECISIONS.md`, propagate
  supersessions.
- `handoff` → update `docs/STATE.md` for the next session; prune to caps.
- `drift [scope]` → audit a doc against code; fix docs, file findings, raise
  questions.
- `slice` (repo-specific, no generic-template equivalent) → pick up the next
  review finding and run it through the codex review loop
  (`docs/agent/GPT_REVIEW_LOOP.md`).

**Review policy (D-2026-07-04-1): every code change and every plan change
goes through the codex review loop in `docs/agent/GPT_REVIEW_LOOP.md` — no
exceptions.** The `.review/README.md` async sentinel hand-off is retired;
its `findings/`/`results/` records and `REVIEW.md` remain the record store.

Claude Code exposes these as `/catchup`, `/plan`, … via `.claude/commands/`;
Antigravity exposes `catchup`/`handoff` as workspace skills in
`.agents/skills/`. This repo drafts `.agents/playbooks/reviewloop.md` as a template, but the codex review loop and `docs/agent/PROTOCOL.md` already cover that role for review-loop work.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Test count may grow but never drop versus the prior baseline unless the
  removal is called out in the finding doc's Known gaps.
- Windows parity: after touching platform-specific code (`win_fs`, planners),
  run `scripts/windows/run-blit-tests.ps1`.
- Docs gate (CI): a push touching `crates/**` or `proto/**` must also touch
  `docs/STATE.md`, unless the commit message contains `[state: skip]`
  (reserved for mechanical changes). `scripts/agent/check-docs.sh` must pass;
  run it locally before pushing docs changes.
- This section is the canonical home for the verification commands (the
  `.agents/repo-map.json` mirror was retired 2026-07-08 with the toolkit's
  JSON layer).

## Remotes & Sync

- `origin` — `http://q:3000/michael/blit_v2.git` (**LAN gitea**). This is
  what a bare `git push` / `git ls-remote origin` talks to.
- `github` — `https://github.com/roethlar/Blit.git` (**GitHub**).
- **`git push` does NOT update GitHub.** The two remotes are independent
  and nothing auto-syncs them: pushing `origin` moves the LAN gitea only,
  and GitHub needs its own explicit `git push github`. Either can lag the
  other by many commits. A ref-listing before a push must therefore name
  the remote's **URL**, not just "origin" — the name alone does not say
  which host is being published to.
- **CORRECTED 2026-07-13** against `git remote -v` (evidence in the
  session's push output: a bare `git push` printed
  `To http://q:3000/michael/blit_v2.git`, while `git push github` printed
  `To https://github.com/roethlar/Blit.git` and fast-forwarded GitHub
  `6d37a22..be1989d` — i.e. GitHub had been dozens of commits stale while
  every agent push went to the mirror). The prior text had the two names
  **inverted** (`origin` = GitHub, plus a `gitea` remote that does not
  exist) and asserted it had been "verified against `git remote -v`
  2026-07-04" while disparaging an earlier revision that was in fact
  correct. Do not re-invert this without re-reading `git remote -v`.
- Push policy: `.agents/push-policy.md` (ask). This repo's git-safety rules
  go well beyond a simple push policy — see Earned Practices below.

## Earned Practices

These are absolute; they exist because an unapproved `git merge -s ours`
octopus (commit `c793df2`) was pushed to `origin/master` without the owner's
consent (`docs/DECISIONS.md` D-2026-06-07-1).

- **No agent-created branches.** Agents never create git branches on their
  own decision. All work happens on `master` or the branch the owner already
  checked out.
- **Owner is the sole gate for git operations that publish, rewrite, or
  destroy.** No `push`, `push --force`/`--force-with-lease`,
  `reset --hard`, rebase or other history rewrite, `commit --amend` on
  pushed commits, or deletion of any branch/tag/ref (local or remote)
  without the owner approving that exact action in the current session.
  Working-tree edits, local commits, and read-only inspection
  (`status`/`log`/`diff`/`show`) need no special approval.
- **Branch deletion is by explicit name only** — the owner names the branch,
  the agent deletes that branch.
- **Before any push:** list the exact local refs, remote refs, and
  destination remotes, then stop and wait for approval.
- **`--merged`/`--no-merged` are unreliable in this repo.** The `-s ours`
  octopus made two now-abandoned branch tips ancestors of `master`, so
  `git branch --merged master` falsely lists them as merged and a plain
  `git merge` of those branches no-ops without landing any code
  (`docs/DECISIONS.md` D-2026-06-07-2). Verify content actually arrived
  (`git diff <branch> master`) before treating anything as landed or
  deleting it.
- **Checkpoints are owner-only.** Only an explicit owner message satisfies a
  checkpoint or verification step. Agents report observations; the owner
  declares pass/fail. Never self-certify a gate or continue a plan past one
  because the condition appears met. Approvals are single-use, step-specific,
  never carried across sessions. When the owner asks a question or thinks out
  loud, answer in plain English and stop — act only on an explicit decision.

## Style

- Rust edition 2021; format with rustfmt. Modules snake_case, types
  PascalCase, constants SHOUT_CASE; match existing names
  (`transfer_session`, `TransferSession`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
  tests deterministic; capture long logs under `logs/`.

## Project Map

- `crates/blit-core/` — core library (enumeration, planner, the unified
  `transfer_session` — every transfer, local included, since otp-11);
  most logic and unit tests live here. New modules get re-exported in
  `crates/blit-core/src/lib.rs`.
- `crates/blit-cli/`, `crates/blit-daemon/` — CLI and daemon binaries; admin
  verbs (scan, ls, find, du, df, rm, completions, profile, list-modules) live
  in `blit-cli` alongside transfer commands.
- `crates/blit-app/`, `crates/blit-tui/` — TUI application layers.
- `crates/blit-prometheus-bridge/` — metrics bridge.
- `proto/blit.proto` — gRPC definitions; `blit-core`'s build script vendors
  protoc.
- Integration tests live per-crate (`crates/blit-cli/tests/`,
  `crates/blit-core/tests/`); the root `Cargo.toml` is a virtual workspace,
  so a root-level `tests/` dir would never be compiled. `scripts/` — helper
  tooling.
- `blit-utils` was intentionally removed; its admin verbs now live in
  `blit-cli`. Do not re-add it as a workspace member.

## Commit and Docs Hygiene

- Commit subject: short imperative ("Add streaming planner heartbeat").
  Review-loop commits: `Fix <id>: <one-line summary>` per `.review/README.md`.
- After meaningful work: append a `DEVLOG.md` entry (newest-first, ISO
  timestamp) and update `docs/STATE.md` — the `handoff` procedure does both.
- Every doc in `docs/plan/` carries a `**Status**:` header, one of:
  `Draft | Active | Shipped | Superseded | Historical`. Superseding a doc
  requires a `docs/DECISIONS.md` entry naming winner and loser, and an edit
  to the superseded text.
- `docs/STATE.md` stays ≤ 200 lines with ≤ 3 handoff entries; prune the
  overflow into `DEVLOG.md`.
# Agent State (pointer)

Current state for this repo lives canonically in **`docs/STATE.md`** — read that
file, not this one. This stub exists only so tools expecting the standard
`.agents/` layout find their way.

Do not record state here. The path `docs/STATE.md` is load-bearing: the CI docs
gate (`.github/workflows/docs-gate.yml`), the doc lint
(`scripts/agent/check-docs.sh`), the session hooks (`scripts/agent/context.sh`),
and the `handoff` procedure in `docs/agent/PROTOCOL.md` all read or enforce it.

- Now / Next / Blockers: `docs/STATE.md`
- Verification commands: `.agents/repo-guidance.md` (Verification)
- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`

exec
/bin/zsh -lc 'git show --stat --oneline 8830fda && git show --format=fuller --find-renames 8830fda -- scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-0vDnKUEq' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-hOYxcQSd' (errno=Operation not permitted)
8830fda macmac rev 9: a null is judged on the RANGE; the escalation is gone; controls interleaved
 .../otp12-macmac-2026-07-14/PREREGISTRATION.md     |  57 +++++--
 scripts/bench_otp12pf_mac.sh                       | 179 ++++++++++-----------
 scripts/otp12pf_mac_verdict.py                     | 124 ++++++++++----
 scripts/otp12pf_mac_verdict_test.py                |  74 +++++++--
 4 files changed, 286 insertions(+), 148 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-sPyoh9Ht' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-gkli9fBb' (errno=Operation not permitted)
commit 8830fda7bb535accd919e294b00bfbd389911930
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Tue Jul 14 11:36:27 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Tue Jul 14 11:36:27 2026 -0400

    macmac rev 9: a null is judged on the RANGE; the escalation is gone; controls interleaved
    
    Round 8's blockers, all four. NO DATA TAKEN.
    
    1. DELETING `UNSTABLE` WAS UNSOUND AT n=16 (codex, BLOCKER — a hole in the rewrite's own
       reasoning, not in a branch). I argued "a bimodal arm widens the CI, so it lands in
       UNCLEAR". True at n=8, where the CI IS the full range. FALSE at n=16, where the interval
       is [d(4),d(13)] and therefore TRIMS three outliers per side — so a bimodal arm yields a
       NARROW median CI and a FALSE NULL. codex drove CI=[1,1] from modes at +-110.
    
       FIX: a NULL is judged on the FULL RANGE (every pair inside +-T), an EFFECT on the CI.
       An equivalence claim must never be reachable by trimming away the very pairs that
       contradict it; a positive claim may tolerate a few stragglers. That asymmetry is also
       why bimodality needs no special branch — it cannot hide from the range.
    
    2. THE RUNS=16 ESCALATION IS REMOVED (owner). The full range only WIDENS with n, so more
       pairs could never rescue an UNCLEAR rig nor certify a marginal control — and with an
       EFFECT you do not need them. A noisy rig is fixed by a quieter rig, not more pairs, and
       UNCLEAR says exactly that. Its whole p-hacking guard surface (a "once" marker, a verdict
       check, a data-hash burn) goes with it: none of that now has to be right.
    
    3. "CLEAN" IS NOT "ZERO" (codex, BLOCKER). A control at +49 with T/2=50 certifies — but that
       49ms of arm bias may be riding in the measurand too, so an effect of exactly T could be
       half real and half rig. B = the bias the controls FAIL to exclude is now carried into the
       measurand: an EFFECT must clear T+B, a NULL must fit inside T-B.
    
    4. THE CONTROLS WERE NOT CONTEMPORANEOUS (codex, HIGH). Both measurand cells ran first, then
       the controls — so THE CONTROLS CERTIFIED A WINDOW THEY WERE NEVER IN. A transient could
       hit the measurand and be gone before they ran. The schedule is now SLOT-MAJOR: within a
       slot, every cell takes one ABBA pair before any cell takes the next, so all six span the
       same window.
    
    Also: the drain STILL failed open — I had fixed the VALUE and left the STATUS, so a drain
    that printed `drained_*` and then exited non-zero was accepted. pipefail inside the
    substitution now carries hrun's status. (Same defect, one layer down. Again.)
    
    Guard: 29 cases, 10/10 mutations killed, plus the RULE tested DIRECTLY — because at n=8 a
    session cannot distinguish the CI from the RANGE (they are the same numbers), so the range
    semantics can only be proven by calling classify() with them differing. A mutation that
    cannot be killed is not a proof, and that one is now a direct test instead.
    
    macOS ships bash 3.2: no associative arrays. The scheduler uses parallel indexed arrays.
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
    Claude-Session: https://claude.ai/code/session_01BAcgnhwAsA3eN86n597PqB

diff --git a/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md b/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
index f662b98..3bffc1a 100644
--- a/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
+++ b/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
@@ -1,6 +1,6 @@
 # otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
 
-**Status**: Pre-registered, **revision 8**. **NO DATA EXISTS YET.**
+**Status**: Pre-registered, **revision 9**. **NO DATA EXISTS YET.**
 
 > ## THE RULE IN ONE PARAGRAPH (rev 8 — D-2026-07-14-3, owner: "simplify")
 >
@@ -212,11 +212,21 @@ a new case to walk past**, which is precisely what went wrong seven rounds runni
 
 | state | condition |
 |---|---|
-| **EFFECT** | `CI_lo >= T_pos` — destination-initiated is slower, by at least T |
-| **INVERTED** | `CI_hi <= T_neg` — source-initiated is slower, by at least T |
-| **NONE** | `T_neg < CI_lo` and `CI_hi < T_pos` — an effect of size T is **EXCLUDED** (equivalence) |
+| **EFFECT** | `CI_lo >= T_pos + B` — destination-initiated is slower, by at least T |
+| **INVERTED** | `CI_hi <= T_neg − B` — source-initiated is slower, by at least T |
+| **NONE** | **the FULL RANGE** lies inside `(T_neg, T_pos)` — *every* pair, not just the median. An effect of size T is **EXCLUDED** (equivalence) |
 | **UNCLEAR** | anything else — the CI spans a threshold; the rig cannot answer |
 
+**A NULL IS JUDGED ON THE RANGE, AN EFFECT ON THE CI — and that asymmetry is the point
+(round-8, codex, BLOCKER).** The ≥95% CI is the *narrowest* valid interval, so at n>8 it
+**trims outliers**; a **bimodal** arm then yields a *narrow median CI* and a **false null**
+(codex drove `CI = [1,1]` from modes at ±110). **An equivalence claim must never be
+reachable by trimming away the very pairs that contradict it.** A *positive* claim may use
+the CI: pairs clearing T is evidence, and a few stragglers do not undo it.
+
+*This is also why bimodality needs no special branch — it cannot hide from the range. The
+previous rule hand-coded an `UNSTABLE` override for exactly this, and got it wrong.*
+
 ### The controls are a PRECONDITION, at HALF the threshold
 
 **Every control must be `NONE` at `T/2`.** Half, because certifying a control with the
@@ -227,6 +237,28 @@ all but 1 ms of P1 while we call the rig clean (round 6 drove exactly that).
 not a null.** Uncertainty about a rig-wide confound is not evidence that the confound is
 absent, and P1's whole claim is that the effect is *specific* to TCP × mixed.
 
+**And "clean" is not "zero" (round-8, codex, BLOCKER).** A control sitting at `+49` with
+`T/2 = 50` certifies — but *that 49 ms of arm bias may be riding in the measurand too*, so a
+measurand effect of exactly `T` could be half real and half rig. The bias the controls **fail
+to exclude** is therefore carried into the measurand's thresholds:
+
+    B = max over clean controls of the largest |CI bound|
+    an EFFECT must clear   T + B     (the bias could be INFLATING it)
+    a NULL must fit inside T − B     (the bias could be MASKING an effect)
+
+If the controls are genuinely clean, `B` is a few ms and this barely moves. If they are
+marginal, it bites — which is the point.
+
+### The controls are CONTEMPORANEOUS with the measurands
+
+The schedule is **slot-major**: within slot *i*, **every** cell takes one ABBA pair, in a
+fixed registered order, before any cell takes slot *i+1*. All six cells therefore span the
+same wall-clock window.
+
+*(Round-8, codex, HIGH: both measurand cells used to run first and the controls afterwards
+— so **the controls certified a window they were never in**. A transient could hit the
+measurand and be gone before the controls ran, and they would pronounce the rig clean.)*
+
 ### The session verdict
 
 1. **INCOMPLETE** — any registered cell short of its `RUNS` pairs, or with a CI below 95%
@@ -262,18 +294,15 @@ absent, and P1's whole claim is that the effect is *specific* to TCP × mixed.
   registered in advance — but it is **not nothing**, and it does not hide inside the word
   "none".
 
-### The escalation, registered in advance
+### There is NO escalation. `RUNS = 8`, and only 8.
 
-At n=8 the ≥95% interval **is the full range**, so one noisy pair can leave the rig
-`UNCLEAR`. A session returning **`UNCLEAR` or `CONTROLS-NOT-CLEAN`** — and *nothing else*
-— may be re-run **once** at `RUNS=16` (interval `[d(4), d(13)]`, coverage 97.9%, which
-tolerates three outliers per side).
+The `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the **full
+range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR` rig,
+nor certify a marginal control; and if you already have an `EFFECT`, you do not need them.
 
-It is triggered by a **power failure and by nothing else**, and that is **enforced**: the
-harness reads the prior session's `session_verdict.txt`, requires its data and manifest on
-the registered build, and **burns the escalation against the prior `runs.csv` hash** — so
-copying the session elsewhere cannot buy a second re-roll. **A result you merely dislike
-is not a trigger.**
+**A noisy rig is fixed by a quieter rig, not by more pairs — and `UNCLEAR` says exactly
+that.** Removing it also removes its entire p-hacking guard surface (a "once" marker, a
+verdict check, a data-hash burn), none of which now has to be right.
 
 ### The registered constants are PINNED IN CODE
 
diff --git a/scripts/bench_otp12pf_mac.sh b/scripts/bench_otp12pf_mac.sh
index caca7da..765fd0a 100755
--- a/scripts/bench_otp12pf_mac.sh
+++ b/scripts/bench_otp12pf_mac.sh
@@ -187,8 +187,6 @@ CELLS="$REGISTERED_CELLS"
 
 SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
 OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
-ESCALATED_FROM=""          # set only by the verified RUNS=16 escalation
-PRIOR_RUNS_SHA=""          # the data hash the escalation is bound to
 
 MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
 SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
@@ -483,57 +481,12 @@ print(int(statistics.median(ts)))
 
 # =============================================================================
 preflight() {
-  # RUNS=8 is the registered value. RUNS=16 is the ONLY registered escalation, and
-  # it may be used for exactly ONE reason: a prior session returned
-  # INCONCLUSIVE-UNDERPOWERED. It must NEVER be used to chase a result someone
-  # dislikes -- that is the p-hacking this pre-registration exists to prevent.
-  #
-  # Why it exists (round-3 grok, MEDIUM): at n=8 the >=95% order-statistic interval
-  # is the FULL RANGE [min,max], so ONE noisy pair with |d| >= margin blocks a null
-  # forever and the rig can only ever say UNDERPOWERED -- a null-incapable
-  # instrument is broken too, just less dangerously. At n=16 the interval is
-  # [d(4), d(13)] (coverage 97.9%), which tolerates three outliers per side.
-  [[ "$RUNS" == 8 || "$RUNS" == 16 ]] \
-    || die "RUNS must be 8 (registered) or 16 (the registered escalation, valid ONLY after an INCONCLUSIVE-UNDERPOWERED session) — got '$RUNS'"
-  if [[ "$RUNS" == 16 ]]; then
-    # A FLAG IS NOT A JUSTIFICATION (round-5 codex, HIGH). `UNDERPOWERED_ESCALATION=1`
-    # was sufficient on its own: no prior session named, none verified, "once"
-    # unenforced. That is a re-roll button with a serious-sounding name. The
-    # escalation must now POINT AT the underpowered session and the harness READS ITS
-    # VERDICT — the trigger is evidence on disk, not an operator's assertion.
-    local prior="${UNDERPOWERED_ESCALATION:-}" v
-    [[ -n "$prior" ]] \
-      || die "RUNS=16 is the escalation arm. Set UNDERPOWERED_ESCALATION=<path to the prior session dir> that returned INCONCLUSIVE-UNDERPOWERED. It buys POWER; it is NOT a re-roll."
-    # The trigger must be a REAL SESSION, not a directory that merely contains the right
-    # words (round-6, codex HIGH + grok F5: "any directory containing the expected first
-    # verdict line authorizes escalation; provenance, hashes, build and prior runs=8 are
-    # never checked"). So the prior session must carry its own DATA and MANIFEST, and
-    # the escalation is bound to the CONTENT of that data, not to its path.
-    for _f in session_verdict.txt runs.csv meta.csv staging-manifest.txt; do
-      [[ -f "$prior/$_f" ]] \
-        || die "UNDERPOWERED_ESCALATION='$prior' has no $_f — the escalation must name a REAL prior session, not a directory with the right words in it"
-    done
-    v="$(head -1 "$prior/session_verdict.txt" | sed -n 's/^SESSION VERDICT: *//p')"
-    # The two outcomes that mean "not enough power", and NOTHING else. A result you
-    # merely dislike (REPRODUCES, INVERTED, MIXED, DOES-NOT-REPRODUCE) is not a trigger.
-    case "$v" in
-      UNCLEAR|CONTROLS-NOT-CLEAN) : ;;
-      *) die "the prior session '$prior' returned '$v'. RUNS=16 is triggered ONLY by a POWER failure (UNCLEAR or CONTROLS-NOT-CLEAN) — re-running any other result at higher n is p-hacking, and this gate exists to stop it." ;;
-    esac
-    grep -q "binary_identity=$REGISTERED_BUILD" "$prior/staging-manifest.txt" \
-      || die "the prior session '$prior' was not run on the registered build $REGISTERED_BUILD — it cannot authorise an escalation"
-    # "Once" is bound to the DATA, not the directory: copying the session elsewhere does
-    # not buy a second re-roll, because the burn records the runs.csv hash.
-    PRIOR_RUNS_SHA="$(shasum -a 256 "$prior/runs.csv" | cut -d' ' -f1)"
-    if [[ -f "$REPO_ROOT/logs/ESCALATED-SESSIONS" ]] \
-       && grep -q "$PRIOR_RUNS_SHA" "$REPO_ROOT/logs/ESCALATED-SESSIONS"; then
-      die "this exact session's data (runs.csv $PRIOR_RUNS_SHA) has ALREADY authorised an escalation — see logs/ESCALATED-SESSIONS. 'Once' means once, and it is bound to the DATA, not the path."
-    fi
-    ESCALATED_FROM="$prior"
-    log "  escalation: RUNS=16, triggered by $prior (verified INCONCLUSIVE-UNDERPOWERED, build $REGISTERED_BUILD, runs.csv $PRIOR_RUNS_SHA)"
-  fi
-  [[ "$EXPECT_SHA" == "$REGISTERED_BUILD" ]] \
-    || die "EXPECT_SHA='$EXPECT_SHA' but the PRE-REGISTERED build is $REGISTERED_BUILD — a run against another build is not the registered experiment"
+  # RUNS=8, and ONLY 8. The RUNS=16 escalation was removed (owner, 2026-07-14): a null is
+  # judged on the FULL RANGE, which only WIDENS with n, so more pairs could never rescue an
+  # UNCLEAR rig or certify a control -- and if you already have an EFFECT you do not need
+  # it. Its p-hacking guard surface goes with it.
+  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
+
   # The instrument must be the REVIEWED instrument: a modified harness must not be
   # able to claim the reviewed commit.
   git -C "$REPO_ROOT" diff --quiet HEAD -- "$SELF" "$VERDICT_PY" "$VERDICT_TEST" \
@@ -597,7 +550,6 @@ write_manifest() {
     echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
     echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
     echo "# drain_disk_nagatha=$N_DISK drain_disk_q=$Q_DISK ssh_rtt_ms=$SSH_RTT_MS"
-    echo "# escalated_from=${ESCALATED_FROM:-none}"   # a RUNS=16 run must carry its trigger
     echo "# cells=$CELLS"
     echo "host,role,sha,sha256,path"
     echo "nagatha,client,$EXPECT_SHA,$nb,$N_BLIT"
@@ -711,9 +663,10 @@ for i in \$(seq 1 $DRAIN_ITERS); do
   if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
   if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
 done
-echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1)"
-  # ONE token, or it is an error. A multi-line value whose FIRST line says "drained"
-  # must never satisfy the caller's `== drained*` test.
+echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1)" || out="DRAIN-ERROR"
+  # ONE token, or it is an error -- AND the probe must have EXITED cleanly. A drain that
+  # printed `drained_*` and THEN failed is not a drain (codex r8: I fixed the value and
+  # left the status, which is the same defect one layer down).
   case "$out" in
     drained_[0-9]*x2s) echo "$out" ;;
     DRAIN-TIMEOUT)     echo DRAIN-TIMEOUT ;;
@@ -836,30 +789,80 @@ arm_destinit() {
 CSV="$OUT_DIR/runs.csv"
 META="$OUT_DIR/meta.csv"
 
-run_pair_loop() {
-  local cell="$1" sh="$2" dh="$3"
-  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
-  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
-  while (( valid < RUNS && attempts < max )); do
-    attempts=$(( attempts + 1 ))
-    local order pair=yes rowA="" rowB="" arm aname init rid run
-    if (( slot % 2 )); then order="A B"; else order="B A"; fi
-    for arm in $order; do
-      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
-      rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
-      if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"
-      else arm_destinit "$sh" "$dh" "$run"; fi
-      [[ "$RUN_VALID" == yes ]] || pair=no
-      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
-      if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
-      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
+# THE CELLS ARE INTERLEAVED, NOT RUN BACK TO BACK.
+#
+# Round-8 (codex, HIGH): both measurand cells used to run first, then the controls. So the
+# controls certified a window THEY NEVER SHARED -- a transient (a background process, a
+# thermal excursion, a disk that woke up) could hit the measurand and be entirely gone by
+# the time the gRPC/large controls ran, and they would certify the rig as clean. The
+# controls are the ONLY thing standing between this rig and a rig-wide artifact, and they
+# cannot vouch for a window they were not in.
+#
+# So the schedule is SLOT-MAJOR: within slot i, EVERY cell takes one ABBA pair, in a fixed
+# registered order, before any cell takes slot i+1. All six cells therefore span the same
+# wall-clock window and see the same transients.
+#
+#   cell           src dst fixture flag
+CELL_TABLE=(
+  "nq_tcp_mixed    n   q   mixed   "
+  "qn_tcp_mixed    q   n   mixed   "
+  "nq_grpc_mixed   n   q   mixed   --force-grpc"
+  "qn_grpc_mixed   q   n   mixed   --force-grpc"
+  "nq_tcp_large    n   q   large   "
+  "qn_tcp_large    q   n   large   "
+)
+
+# macOS ships bash 3.2, which has NO associative arrays. Parallel indexed arrays, keyed by
+# the cell's position in CELL_TABLE.
+CELL_VALID=(); CELL_ATTEMPTS=()
+run_one_pair() {   # $1=idx $2=cell $3=srchost $4=dsthost $5=fixture $6=flag $7=slot -> 0 if VALID
+  local i="$1" cell="$2" sh="$3" dh="$4" w="$5" flag="$6" slot="$7"
+  local attempts=$(( ${CELL_ATTEMPTS[$i]:-0} + 1 ))
+  CELL_ATTEMPTS[$i]=$attempts
+  CUR_W="$w"; CUR_FLAG="$flag"
+  local order pair=yes rowA="" rowB="" arm aname init rid run
+  # ABBA: the arm order alternates by slot, so a monotonic drift cannot favour one arm.
+  if (( slot % 2 )); then order="A B"; else order="B A"; fi
+  for arm in $order; do
+    if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
+    rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
+    if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"; else arm_destinit "$sh" "$dh" "$run"; fi
+    [[ "$RUN_VALID" == yes ]] || pair=no
+    local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
+    if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
+    log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (fsync ${RUN_FLUSH}ms, settle ${RUN_SETTLED}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
+  done
+  echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
+  if [[ "$pair" == yes ]]; then
+    CELL_VALID[$i]=$(( ${CELL_VALID[$i]:-0} + 1 ))
+    return 0
+  fi
+  log "  $cell: pair at slot $slot VOIDED"
+  return 1
+}
+
+run_all_cells() {
+  local slot i cell sh dh w flag max=$(( 2 * RUNS )) n=${#CELL_TABLE[@]}
+  for (( i = 0; i < n; i++ )); do CELL_VALID[$i]=0; CELL_ATTEMPTS[$i]=0; done
+  for (( slot = 1; slot <= RUNS; slot++ )); do
+    log "=== SLOT $slot / $RUNS (every cell takes one pair before any cell takes the next) ==="
+    for (( i = 0; i < n; i++ )); do
+      read -r cell sh dh w flag <<<"${CELL_TABLE[$i]}"
+      # a voided pair is retried IN PLACE, so the cell stays in step with its siblings
+      while (( ${CELL_ATTEMPTS[$i]:-0} < max )); do
+        if run_one_pair "$i" "$cell" "$sh" "$dh" "$w" "${flag:-}" "$slot"; then break; fi
+      done
     done
-    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
-    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
-    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
   done
-  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
-  else echo "$cell,$attempts,yes" >> "$META"; fi
+  for (( i = 0; i < n; i++ )); do
+    read -r cell sh dh w flag <<<"${CELL_TABLE[$i]}"
+    if (( ${CELL_VALID[$i]:-0} < RUNS )); then
+      echo "$cell,${CELL_ATTEMPTS[$i]},no" >> "$META"
+      log "  $cell INCOMPLETE: ${CELL_VALID[$i]}/$RUNS valid pairs"
+    else
+      echo "$cell,${CELL_ATTEMPTS[$i]},yes" >> "$META"
+    fi
+  done
 }
 
 SESSION_VOID_REASON=""
@@ -1048,29 +1051,13 @@ main() {
     log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
     exit 0
   fi
-  # "Once" means once: burn the escalation the moment it is used, so the same
-  # underpowered session cannot authorise a second, third, nth re-roll.
-  if [[ -n "$ESCALATED_FROM" ]]; then
-    echo "escalated to $SESSION_TAG (RUNS=$RUNS) on $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
-      >> "$ESCALATED_FROM/ESCALATED"
-    # Bound to the DATA, so a copy of the session cannot buy a second re-roll.
-    echo "$PRIOR_RUNS_SHA $ESCALATED_FROM -> $SESSION_TAG" >> "$REPO_ROOT/logs/ESCALATED-SESSIONS"
-  fi
   log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
   echo "cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
   echo "cell,pairs_attempted,complete" > "$META"
   daemon_start n; daemon_start q
   smoke n; smoke q
 
-  local carrier w flag cell
-  for w in mixed large small; do
-    for carrier in tcp grpc; do
-      if [[ "$carrier" == grpc ]]; then flag="--force-grpc"; else flag=""; fi
-      CUR_W="$w"; CUR_FLAG="$flag"
-      cell="nq_${carrier}_${w}"; if [[ ",$CELLS," == *",$cell,"* ]]; then run_pair_loop "$cell" n q; fi
-      cell="qn_${carrier}_${w}"; if [[ ",$CELLS," == *",$cell,"* ]]; then run_pair_loop "$cell" q n; fi
-    done
-  done
+  run_all_cells
 
   # End-load BEFORE the verdict is computed, and it can VOID the session.
   log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
diff --git a/scripts/otp12pf_mac_verdict.py b/scripts/otp12pf_mac_verdict.py
index 48ab219..b84653f 100644
--- a/scripts/otp12pf_mac_verdict.py
+++ b/scripts/otp12pf_mac_verdict.py
@@ -65,7 +65,7 @@ runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]
 # direction of the answer you want. That is the one thing pre-registration exists to
 # make impossible.
 DELTA_REF = 230          # ms; rig W's measured Delta_P1
-REGISTERED_PAIRS = (8, 16)
+REGISTERED_PAIRS = (8,)
 MIN_COVERAGE = 0.95
 
 _env = os.environ.get("DELTA_REF_MS")
@@ -161,43 +161,102 @@ def thresholds(s_med, scale=1.0):
             -min(s_med / 11.0, float(DELTA_REF)) * scale)
 
 
-def classify(ci_lo, ci_hi, t_pos, t_neg):
-    """THE RULE. Four states partitioning the CI's position relative to the thresholds.
-    They are mutually exclusive and exhaustive BY CONSTRUCTION -- there is no label here
-    for a new case to walk past, which is what went wrong seven rounds in a row."""
+def classify(ci_lo, ci_hi, rng_lo, rng_hi, t_pos, t_neg):
+    """THE RULE. Four states, mutually exclusive and exhaustive BY CONSTRUCTION.
+
+    EFFECT/INVERTED use the >=95% CI on the median: a POSITIVE claim can tolerate a few
+    outliers (13 of 16 pairs clearing T is evidence, and 3 stragglers do not undo it).
+
+    NONE uses the FULL RANGE -- EVERY pair must lie inside +-T. Round 8 (codex, BLOCKER):
+    at n=16 the CI is [d(4), d(13)], which TRIMS three outliers per side, so a BIMODAL arm
+    produces a NARROW median CI and a FALSE NULL (driven: CI = [1,1] from modes at +-110).
+    An equivalence claim must never be reachable by trimming away the very pairs that
+    contradict it. This is also why bimodality needs no special branch: it cannot hide
+    from the range.
+    """
     if ci_lo >= t_pos:
         return "EFFECT"
     if ci_hi <= t_neg:
         return "INVERTED"
-    if t_neg < ci_lo and ci_hi < t_pos:
+    if t_neg < rng_lo and rng_hi < t_pos:
         return "NONE"
     return "UNCLEAR"
 
 
-# ---- grade every registered cell ---------------------------------------------------
+# ---- pass 1: measure every cell -----------------------------------------------------
 cell = {}
 for c in sorted(set(REGISTERED) | set(meta)):
     d = paired(c)
     ci = median_ci(d) if d else None
     # COMPLETE is checked against the DATA, never against meta's say-so: a one-pair CSV
     # with a lying meta once graded as a full cell and emitted a null at 0% coverage.
-    if (meta.get(c, {}).get("complete") != "yes" or len(d) < PAIRS or ci is None):
+    if meta.get(c, {}).get("complete") != "yes" or len(d) < PAIRS or ci is None:
         cell[c] = dict(state="INCOMPLETE", n=len(d))
         continue
     s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
     hi, lo = max(s_med, d_med), min(s_med, d_med)
     ci_lo, ci_hi, cov = ci
-    t_pos, t_neg = thresholds(s_med)
-    c_pos, c_neg = thresholds(s_med, 0.5)                      # controls: HALF
     p, k, n = sign_p(d)
-    cell[c] = dict(
-        state=classify(ci_lo, ci_hi, t_pos, t_neg),            # measurand rule
-        ctrl_state=classify(ci_lo, ci_hi, c_pos, c_neg),       # control rule
-        n=len(d), d=d, D=med(d), ci=(ci_lo, ci_hi), cov=cov, T=t_pos, Tneg=t_neg,
-        src=s_med, dst=d_med, p=p, k=k,
-        # The acceptance bar: integer-exact, `<= 1.10` PASSES. REPORTED, never used.
-        bar="PASS" if 10 * hi <= 11 * lo else "FAIL",
-        ratio=hi / lo if lo else 0.0)
+    cell[c] = dict(n=len(d), d=d, D=med(d), ci=(ci_lo, ci_hi), rng=(min(d), max(d)),
+                   cov=cov, src=s_med, dst=d_med, p=p, k=k,
+                   # The acceptance bar: integer-exact, `<= 1.10` PASSES. REPORTED, never used.
+                   bar="PASS" if 10 * hi <= 11 * lo else "FAIL",
+                   ratio=hi / lo if lo else 0.0)
+
+# ---- pass 2: the controls certify the rig, and BOUND its residual bias ---------------
+# A control certifies clean at T/2 -- but "clean" is not "zero". A control sitting at +49
+# with T/2 = 50 is accepted, and THAT 49 ms OF ARM BIAS MAY BE RIDING IN THE MEASURAND
+# TOO, so a measurand "EFFECT" at exactly T could be half real and half rig (round-8
+# codex, BLOCKER). The bias the controls FAIL TO EXCLUDE is therefore carried into the
+# measurand's thresholds:
+#
+#     B = max over clean controls of the largest |CI bound|   -- the arm asymmetry that
+#                                                                could not be ruled out
+#     an EFFECT must clear  T + B     (bias could be INFLATING it)
+#     a NULL   must fit in  T - B     (bias could be MASKING an effect)
+#
+# If the controls are genuinely clean, B is a few ms and this barely moves. If they are
+# marginal, it bites -- which is the point.
+dirty = []
+B = 0.0
+for c in CONTROLS:
+    x = cell.get(c, {})
+    if x.get("state") == "INCOMPLETE":
+        continue
+    c_pos, c_neg = thresholds(x["src"], 0.5)
+    x["ctrl_state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], c_pos, c_neg)
+    x["ctrl_T"] = c_pos
+    if x["ctrl_state"] != "NONE":
+        dirty.append(c)
+    else:
+        B = max(B, abs(x["ci"][0]), abs(x["ci"][1]))
+
+# ---- pass 3: grade the measurands, against thresholds widened by the control bias -----
+for c in MEASURANDS:
+    x = cell.get(c, {})
+    if x.get("state") == "INCOMPLETE":
+        continue
+    t_pos, t_neg = thresholds(x["src"])
+    x["T"] = t_pos
+    x["B"] = B
+    x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1],
+                          t_pos + B, t_neg - B)          # an EFFECT must clear T + B
+    if x["state"] == "NONE":
+        # ...and a NULL must survive the TIGHTER bound: bias could be masking an effect.
+        if not (t_neg + B < x["rng"][0] and x["rng"][1] < t_pos - B):
+            x["state"] = "UNCLEAR"
+
+# Controls also carry a state for the report; measurands carry a ctrl_state for symmetry.
+for c in cell:
+    x = cell[c]
+    if x.get("state") == "INCOMPLETE":
+        continue
+    if "state" not in x:                                  # a control: report its own state
+        t_pos, t_neg = thresholds(x["src"])
+        x["T"] = t_pos
+        x["B"] = 0.0
+        x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], t_pos, t_neg)
+    x.setdefault("ctrl_state", "-")
 
 # ---- outputs -----------------------------------------------------------------------
 with open(sum_p, "w") as f:
@@ -209,16 +268,17 @@ with open(sum_p, "w") as f:
                                                " ".join(map(str, v))))
 
 with open(pair_p, "w") as f:
-    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,coverage,"
-            "T_ms,sign_p,k_pos,state,control_state\n")
+    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,range_lo,range_hi,"
+            "coverage,T_ms,B_ms,sign_p,k_pos,state,control_state\n")
     for c in sorted(cell):
         x = cell[c]
         if x["state"] == "INCOMPLETE":
-            f.write("%s,%d,,,,,,,,,,,,INCOMPLETE,\n" % (c, x["n"]))
+            f.write("%s,%d,,,,,,,,,,,,,,,INCOMPLETE,\n" % (c, x["n"]))
             continue
-        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%.4f,%d,%.4f,%d/%d,%s,%s\n" % (
+        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%d,%d,%.4f,%d,%d,%.4f,%d/%d,%s,%s\n" % (
             c, x["n"], x["src"], x["dst"], x["ratio"], x["bar"], x["D"],
-            x["ci"][0], x["ci"][1], x["cov"], round(x["T"]), x["p"], x["k"], x["n"],
+            x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], x["cov"],
+            round(x["T"]), round(x.get("B", 0)), x["p"], x["k"], x["n"],
             x["state"], x["ctrl_state"]))
 
 with open(ver_p, "w") as f:
@@ -233,8 +293,6 @@ with open(ver_p, "w") as f:
 
 # ---- THE SESSION VERDICT -----------------------------------------------------------
 incomplete = [c for c in REGISTERED if cell.get(c, {}).get("state") == "INCOMPLETE"]
-# A control is clean only at HALF the threshold.
-dirty = [c for c in CONTROLS if not incomplete and cell[c]["ctrl_state"] != "NONE"]
 m = {c: cell[c]["state"] for c in MEASURANDS if not incomplete}
 
 if incomplete:
@@ -285,16 +343,19 @@ else:
            % ", ".join(c for c, s in m.items() if s == "UNCLEAR"))
 
 out = ["SESSION VERDICT: %s" % verdict, "", why, "",
-       "Per cell (T = min(srcinit_median/10, %d ms); controls must be NONE at T/2):" % DELTA_REF]
+       "Per cell. T = min(srcinit_median/10, %d ms). Controls must be NONE at T/2, and B is"
+       % DELTA_REF,
+       "the arm bias they could NOT exclude: an EFFECT must clear T+B, a NULL must fit in T-B."]
 for c in sorted(cell):
     x = cell[c]
     if x["state"] == "INCOMPLETE":
         out.append("  %-14s INCOMPLETE (%d pairs)" % (c, x["n"]))
         continue
-    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] (%.1f%%) T=%3dms  "
-               "ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
+    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] range=[%+5d,%+5d] "
+               "T=%3dms B=%3dms  ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
                % (c, x["state"], x["ctrl_state"], x["D"], x["ci"][0], x["ci"][1],
-                  100 * x["cov"], round(x["T"]), x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
+                  x["rng"][0], x["rng"][1], round(x["T"]), round(x.get("B", 0)),
+                  x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
 # A cell can be NONE (an effect of size T is excluded) and STILL carry a real, consistent
 # effect BELOW T -- e.g. 99 ms on a 1000 ms arm, one millisecond under the threshold, on
 # 7 of 8 pairs. That is not a contradiction and it does not change the verdict, but it
@@ -311,6 +372,11 @@ if subthreshold:
             "threshold, so they are not a reproduction of P1. They are NOT nothing."]
 
 out += ["",
+        "A NULL (NONE) is judged on the full RANGE -- EVERY pair inside the bound -- not on",
+        "the median CI, which at n>8 would TRIM the outliers that contradict it. An EFFECT",
+        "uses the CI. That is why bimodality needs no special branch: it cannot hide from",
+        "the range.",
+        "",
         "The bar/ratio columns are the project's ACCEPTANCE criterion. They are reported",
         "and take NO part in this verdict, which is decided only by the paired CI against",
         "T. sign_p is reported, not decided on. All runs are in summary.csv -- read them.",
diff --git a/scripts/otp12pf_mac_verdict_test.py b/scripts/otp12pf_mac_verdict_test.py
index f2aecd5..0d4c9e1 100644
--- a/scripts/otp12pf_mac_verdict_test.py
+++ b/scripts/otp12pf_mac_verdict_test.py
@@ -64,7 +64,7 @@ def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=
             f.write("%s,%d,yes\n" % (cell, pairs))
     env = dict(os.environ, VERDICT_CELLS=",".join(MEASURANDS),
                CONTROL_CELLS=",".join(CONTROLS), REGISTERED_CELLS=",".join(REGISTERED),
-               REQUIRED_PAIRS=str(pairs), SESSION_VOID_REASON=void_reason)
+               REQUIRED_PAIRS="8", SESSION_VOID_REASON=void_reason)
     env.pop("DELTA_REF_MS", None)                      # PINNED in the engine
     env.update(env_extra or {})
     out = subprocess.run([sys.executable, engine(), runs, meta,
@@ -86,7 +86,7 @@ CASES = [
      "UNCLEAR", "DOES-NOT-REPRODUCE"),
 
     ("codex r2: a rig-W-sized effect (230ms) in EVERY pair, on a slow 2500ms arm",
-     dict(measurand_d=[230] * 8, src=2500),
+     dict(measurand_d=[230] * 8, src=2500, control_d=[0] * 8),
      "REPRODUCES", "DOES-NOT-REPRODUCE"),
 
     ("codex r2: an effect the 10% bar alone would forgive (240ms @ 2500)",
@@ -164,10 +164,22 @@ CASES = [
      dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000),
      "REPRODUCES", None),
 
-    ("an exact 10% effect is reportable (it was once unreachable by construction)",
-     dict(measurand_d=[100] * 8, src=1000),
+    ("an exact 10% effect is reportable on a bias-free rig (it was once unreachable)",
+     dict(measurand_d=[100] * 8, src=1000, control_d=[0] * 8),
      "REPRODUCES", None),
 
+    # codex r8, BLOCKER: a control at +5 is "clean", but that 5ms of arm bias may be
+    # riding in the measurand too -- so an effect of EXACTLY T could be (T-5) real plus
+    # 5 rig. It must not be banked as a reproduction. B carries the bias the controls
+    # could not exclude into the measurand's threshold.
+    ("codex r8: an effect of exactly T is NOT a reproduction when the controls carry bias",
+     dict(measurand_d=[100] * 8, src=1000, control_d=[5] * 8),
+     "UNCLEAR", "REPRODUCES"),
+
+    ("codex r8: ...and the same effect IS one once the rig is bias-free",
+     dict(measurand_d=[105] * 8, src=1000, control_d=[5] * 8),
+     "REPRODUCES", "UNCLEAR"),
+
     ("source-initiated slower is INVERTED, never 'P1 absent'",
      dict(measurand_d=[-300, -310, -320, -330, -340, -350, -360, -370], src=1000),
      "INVERTED", None),
@@ -184,6 +196,11 @@ CASES = [
                     "qn_tcp_mixed": ([-20, 300, 310, 320, 330, 340, 350, 360], 1000)}),
      "REPRODUCES", "UNCLEAR"),
 
+    ("codex r8: a bimodal arm cannot hide from the RANGE (a null is judged on every pair)",
+     dict(measurand_d=[-110, 0, -110, 110, 110, 0, -110, 0], src=730,
+          control_d=[0] * 8),
+     "UNCLEAR", "DOES-NOT-REPRODUCE"),
+
     ("a null the rig could not have SEEN is UNCLEAR, not a null",
      dict(measurand_d=[-400, -300, -100, 0, 0, 100, 300, 400], src=2000),
      "UNCLEAR", "DOES-NOT-REPRODUCE"),
@@ -211,8 +228,8 @@ CASES = [
 
 MUTATIONS = [
     ("the control threshold is the SAME as the measurand's, not half (grok r6)",
-     ["    c_pos, c_neg = thresholds(s_med, 0.5)                      # controls: HALF",
-      "    c_pos, c_neg = thresholds(s_med, 1.0)"],
+     ['    c_pos, c_neg = thresholds(x["src"], 0.5)',
+      '    c_pos, c_neg = thresholds(x["src"], 1.0)'],
      "D=+229, ONE MS under"),
 
     ("dirty controls block only the null, not a reproduction (codex r6)",
@@ -234,9 +251,13 @@ MUTATIONS = [
      ["    if ci_lo >= t_pos:", "    if (ci_lo + ci_hi) / 2.0 >= t_pos:"],
      "one huge outlier"),
 
+    ("the control's residual bias is not carried into the measurand (codex r8)",
+     ["        B = max(B, abs(x[\"ci\"][0]), abs(x[\"ci\"][1]))", "        B = max(B, 0.0)"],
+     "exactly T is NOT a reproduction"),
+
     ("the engine trusts meta.complete and never counts the pairs (grok r3)",
-     ['    if (meta.get(c, {}).get("complete") != "yes" or len(d) < PAIRS or ci is None):',
-      '    if (meta.get(c, {}).get("complete") != "yes" or ci is None):'],
+     ['    if meta.get(c, {}).get("complete") != "yes" or len(d) < PAIRS or ci is None:',
+      '    if meta.get(c, {}).get("complete") != "yes" or ci is None:'],
      "SHORT cell (6 of 8 pairs)"),
 
     ("a missing registered cell is filtered away (codex r2)",
@@ -253,6 +274,38 @@ MUTATIONS = [
 ]
 
 
+def rule_unit_tests():
+    """The RULE itself, called directly -- because a session at n=8 cannot distinguish the
+    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
+    closed codex's round-8 blocker; judging a NULL on the RANGE is the SEMANTICS that keeps
+    it closed if a larger n is ever registered again, and it can only be tested here."""
+    import importlib.util
+    spec = importlib.util.spec_from_file_location("eng", DEFAULT_VERDICT)
+    # the engine runs on import (it is a script), so exercise classify() via a subprocess-free
+    # re-implementation guard: read the function out of the source and exec it in isolation.
+    src = open(DEFAULT_VERDICT).read()
+    start = src.index("def classify(")
+    end = src.index("\n\n", src.index("return \"UNCLEAR\"", start))
+    ns = {}
+    exec(src[start:end], ns)
+    classify = ns["classify"]
+    bad = 0
+    checks = [
+        # ci narrow (outliers trimmed), range wide: a bimodal arm. MUST NOT be NONE.
+        ("bimodal: CI=[1,1] but range=[-110,110], T=73", (1, 1, -110, 110, 73, -66), "UNCLEAR"),
+        ("clean: CI and range both inside T",            (2, 3, -4, 3, 73, -66),      "NONE"),
+        ("a real effect clears T",                       (80, 90, 75, 95, 73, -66),   "EFFECT"),
+        ("an inverted effect clears -T",                 (-90, -80, -95, -75, 73, -66), "INVERTED"),
+    ]
+    for name, args, want in checks:
+        got = classify(*args)
+        ok = got == want
+        print("  %-46s -> %-8s %s" % (name, got, "ok" if ok else "*** FAIL (want %s) ***" % want))
+        if not ok:
+            bad += 1
+    return bad
+
+
 def run_cases():
     bad = []
     for name, kw, must_be, must_not in CASES:
@@ -323,8 +376,11 @@ if __name__ == "__main__":
         n = mutate()
         print("\n%d/%d mutations killed" % (len(MUTATIONS) - n, len(MUTATIONS)))
         sys.exit(1 if n else 0)
+    print("The RULE, called directly (a session at n=8 cannot separate CI from RANGE):")
+    unit = rule_unit_tests()
+    print()
     fails = run_cases()
     print()
     z = fuzz()
     print("\n%d/%d cases passed" % (len(CASES) - len(fails), len(CASES)))
-    sys.exit(1 if (fails or z) else 0)
+    sys.exit(1 if (fails or z or unit) else 0)

exec
/bin/zsh -lc 'git show --stat --oneline 8830fda && git log --oneline --decorate -10 && git diff --stat 8830fda..HEAD -- scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md && git diff 8830fda..HEAD -- scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-44G5u3EN' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1rOaDRfz' (errno=Operation not permitted)
8830fda macmac rev 9: a null is judged on the RANGE; the escalation is gone; controls interleaved
 .../otp12-macmac-2026-07-14/PREREGISTRATION.md     |  57 +++++--
 scripts/bench_otp12pf_mac.sh                       | 179 ++++++++++-----------
 scripts/otp12pf_mac_verdict.py                     | 124 ++++++++++----
 scripts/otp12pf_mac_verdict_test.py                |  74 +++++++--
 4 files changed, 286 insertions(+), 148 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-n5dZSRu7' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-gL8OMQuW' (errno=Operation not permitted)
7d72f77 (HEAD -> master) round 9 (grok): B is the control's RANGE, not its CI; and the null's B-tighten is now guarded
2264ae2 prereg: the UNCLEAR remedy still pointed at the removed escalation
446549e round 9: n must be EXACTLY 8 — a 16-pair CSV was graded, and the CI TRIMMED the pairs that contradicted it
8830fda macmac rev 9: a null is judged on the RANGE; the escalation is gone; controls interleaved
08570b5 round 8: the simplification has a REAL hole — deleting UNSTABLE was unsound at n=16
79c1f2d macmac rev 8: rewrite the decision rule — one statistic, one threshold, four states
30d4374 D-2026-07-14-3: simplify the Mac<->Mac decision rule (owner: 'simplify')
f7f6e17 (origin/master, origin/HEAD, github/master) round 7: both reviewers again NOT READY — and the decision RULE is now where the bugs live
325d06e STATE: prune to the 200-line cap, gated on the EXIT STATUS this time
cb6d134 STATE: prune to the 200-line cap — I committed dd778ae while check-docs was FAILING
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-RG2fPOAa' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1nIg7H7D' (errno=Operation not permitted)
 .../otp12-macmac-2026-07-14/PREREGISTRATION.md     | 19 +++++++---
 scripts/otp12pf_mac_verdict.py                     | 41 +++++++++++++---------
 scripts/otp12pf_mac_verdict_test.py                | 38 ++++++++++++++++++--
 3 files changed, 75 insertions(+), 23 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-C07sLrZw' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rebaY3oP' (errno=Operation not permitted)
diff --git a/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md b/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
index 3bffc1a..06abf3d 100644
--- a/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
+++ b/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
@@ -273,7 +273,7 @@ measurand and be gone before the controls ran, and they would pronounce the rig
 6. **INVERTED** — a new finding; never banked as "P1 absent".
 7. **DOES-NOT-REPRODUCE** — **both** measurand cells `NONE`, with clean controls. A
    genuine equivalence result.
-8. **UNCLEAR** — otherwise. **This is not a null.** The registered remedy is `RUNS=16`.
+8. **UNCLEAR** — otherwise. **This is not a null.** There is no remedy in more pairs: fix the rig.
 
 ### What is deliberately ABSENT, and why that is safe
 
@@ -294,11 +294,20 @@ measurand and be gone before the controls ran, and they would pronounce the rig
   registered in advance — but it is **not nothing**, and it does not hide inside the word
   "none".
 
-### There is NO escalation. `RUNS = 8`, and only 8.
+### There is NO escalation. `n` is EXACTLY 8.
 
-The `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the **full
-range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR` rig,
-nor certify a marginal control; and if you already have an `EFFECT`, you do not need them.
+The old `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the
+**full range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR`
+rig, nor certify a marginal control; and if you already have an `EFFECT`, you do not need
+them.
+
+**And `n` must be EXACTLY 8, not "at least 8" (round-9, grok, BLOCKER).** At the registered
+n=8 the ≥95% interval **is** `[min, max]` — it *cannot* trim. At any larger n it starts
+trimming outliers, and a bimodal arm then yields a narrow median CI and a false verdict:
+grok drove a 16-pair CSV (three pairs at −500 trimmed away, thirteen at +200 left) straight
+to **`REPRODUCES`**. A cell carrying any count but the registered one is **`INCOMPLETE`**.
+*(I removed 16 from the registered list and left the completeness check saying `>=`. Fixed
+where I looked, not where it lived — again.)*
 
 **A noisy rig is fixed by a quieter rig, not by more pairs — and `UNCLEAR` says exactly
 that.** Removing it also removes its entire p-hacking guard surface (a "once" marker, a
diff --git a/scripts/otp12pf_mac_verdict.py b/scripts/otp12pf_mac_verdict.py
index b84653f..4c59e0f 100644
--- a/scripts/otp12pf_mac_verdict.py
+++ b/scripts/otp12pf_mac_verdict.py
@@ -18,9 +18,15 @@ WHY IT IS THIS SMALL
 THE STATISTIC (paired, because the design is paired)
     d_i = destinit_i - srcinit_i          per ABBA slot (positive = destination slower)
     D   = median(d_i)                     low median, even n
-    CI  = exact distribution-free order-statistic interval on the population median,
-          the narrowest whose coverage is >= 95%. At n=8 that is [min(d), max(d)]
-          (99.22%); at n=16, [d_(4), d_(13)] (97.87%). No bootstrap, no approximation.
+    CI  = exact distribution-free order-statistic interval on the population median, the
+          narrowest whose coverage is >= 95%. AT THE REGISTERED n=8 THAT IS [min(d), max(d)]
+          (99.22%) -- i.e. it CANNOT trim. No bootstrap, no approximation.
+    RANGE = [min(d), max(d)], and a NULL is judged on the RANGE, never on a trimmed CI.
+
+    n IS EXACTLY 8. Not "at least": at any larger n the >=95% interval starts TRIMMING
+    outliers, and a bimodal arm then yields a narrow median CI and a FALSE verdict. grok
+    drove exactly that with a 16-pair CSV (3 pairs at -500 trimmed away, 13 at +200 left)
+    -> REPRODUCES. The cell must carry EXACTLY the registered pair count or it is INCOMPLETE.
 
 THE THRESHOLD (one)
     T = min(srcinit_median / 10, DELTA_REF)
@@ -164,15 +170,16 @@ def thresholds(s_med, scale=1.0):
 def classify(ci_lo, ci_hi, rng_lo, rng_hi, t_pos, t_neg):
     """THE RULE. Four states, mutually exclusive and exhaustive BY CONSTRUCTION.
 
-    EFFECT/INVERTED use the >=95% CI on the median: a POSITIVE claim can tolerate a few
-    outliers (13 of 16 pairs clearing T is evidence, and 3 stragglers do not undo it).
+    EFFECT/INVERTED use the >=95% CI on the median; NONE uses the FULL RANGE. At the
+    registered n=8 these coincide (the CI IS the range), so nothing can be trimmed either
+    way -- the distinction is the SEMANTICS that keeps the rule sound if a larger n is ever
+    registered, and the engine REFUSES any n but 8.
 
     NONE uses the FULL RANGE -- EVERY pair must lie inside +-T. Round 8 (codex, BLOCKER):
-    at n=16 the CI is [d(4), d(13)], which TRIMS three outliers per side, so a BIMODAL arm
-    produces a NARROW median CI and a FALSE NULL (driven: CI = [1,1] from modes at +-110).
-    An equivalence claim must never be reachable by trimming away the very pairs that
-    contradict it. This is also why bimodality needs no special branch: it cannot hide
-    from the range.
+    a >=95% CI at n>8 TRIMS outliers, so a BIMODAL arm produces a NARROW median CI and a
+    FALSE NULL (driven: CI = [1,1] from modes at +-110). An equivalence claim must never be
+    reachable by trimming away the very pairs that contradict it. This is also why
+    bimodality needs no special branch: it cannot hide from the range.
     """
     if ci_lo >= t_pos:
         return "EFFECT"
@@ -190,7 +197,7 @@ for c in sorted(set(REGISTERED) | set(meta)):
     ci = median_ci(d) if d else None
     # COMPLETE is checked against the DATA, never against meta's say-so: a one-pair CSV
     # with a lying meta once graded as a full cell and emitted a null at 0% coverage.
-    if meta.get(c, {}).get("complete") != "yes" or len(d) < PAIRS or ci is None:
+    if meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None:
         cell[c] = dict(state="INCOMPLETE", n=len(d))
         continue
     s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
@@ -210,8 +217,10 @@ for c in sorted(set(REGISTERED) | set(meta)):
 # codex, BLOCKER). The bias the controls FAIL TO EXCLUDE is therefore carried into the
 # measurand's thresholds:
 #
-#     B = max over clean controls of the largest |CI bound|   -- the arm asymmetry that
-#                                                                could not be ruled out
+#     B = max over clean controls of the largest |RANGE bound| -- the arm asymmetry the
+#                                                                 controls could not rule out.
+#         The RANGE, not the CI: the CI is an interval for the MEDIAN and it TRIMS, and a
+#         bound on what the rig might be carrying must not be computed by trimming.
 #     an EFFECT must clear  T + B     (bias could be INFLATING it)
 #     a NULL   must fit in  T - B     (bias could be MASKING an effect)
 #
@@ -229,7 +238,7 @@ for c in CONTROLS:
     if x["ctrl_state"] != "NONE":
         dirty.append(c)
     else:
-        B = max(B, abs(x["ci"][0]), abs(x["ci"][1]))
+        B = max(B, abs(x["rng"][0]), abs(x["rng"][1]))
 
 # ---- pass 3: grade the measurands, against thresholds widened by the control bias -----
 for c in MEASURANDS:
@@ -308,7 +317,7 @@ elif dirty:
     why = ("control cell(s) are not free of an arm asymmetry at T/2: %s. P1 is claimed "
            "TCP-only and mixed-only; if the gRPC/large controls may be carrying the same "
            "asymmetry, then NEITHER a reproduction NOR a null is readable off this rig. "
-           "Re-run at RUNS=16 to buy the power to certify them."
+           "There is no escalation: a noisy rig is fixed by a QUIETER RIG, not more pairs."
            % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d], T/2=%d)"
                        % (c, cell[c]["ctrl_state"], cell[c]["D"], cell[c]["ci"][0],
                           cell[c]["ci"][1], round(cell[c]["T"] / 2))
@@ -339,7 +348,7 @@ elif all(s == "NONE" for s in m.values()):
 else:
     verdict = "UNCLEAR"
     why = ("the CI spans the threshold in: %s. The rig could not resolve an effect of "
-           "size T either way -- this is NOT 'P1 vanishes'. Re-run at RUNS=16."
+           "size T either way -- this is NOT 'P1 vanishes'. Fix the rig, do not add pairs."
            % ", ".join(c for c, s in m.items() if s == "UNCLEAR"))
 
 out = ["SESSION VERDICT: %s" % verdict, "", why, "",
diff --git a/scripts/otp12pf_mac_verdict_test.py b/scripts/otp12pf_mac_verdict_test.py
index 0d4c9e1..8bf1b95 100644
--- a/scripts/otp12pf_mac_verdict_test.py
+++ b/scripts/otp12pf_mac_verdict_test.py
@@ -117,6 +117,10 @@ CASES = [
      dict(measurand_d=[10, 10, 10, 10, 10, 10, 10, 800], src=1000),
      "UNCLEAR", "REPRODUCES"),
 
+    ("grok r9: a LONG cell (16 pairs) is INCOMPLETE — a CI at n>8 TRIMS the pairs that contradict it",
+     dict(measurand_d=[-500] * 3 + [200] * 13, src=1000, control_d=[0] * 16),
+     "INCOMPLETE", "REPRODUCES"),
+
     ("a SHORT cell (6 of 8 pairs) claiming complete=yes is INCOMPLETE",
      dict(measurand_d=[-4, -2, -1, 0, 1, 2], src=2000),
      "INCOMPLETE", "DOES-NOT-REPRODUCE"),
@@ -176,6 +180,10 @@ CASES = [
      dict(measurand_d=[100] * 8, src=1000, control_d=[5] * 8),
      "UNCLEAR", "REPRODUCES"),
 
+    ("grok r9: a null must also survive the TIGHTER bound (bias could be MASKING an effect)",
+     dict(measurand_d=[60] * 8, src=1000, control_d=[49] * 8),
+     "UNCLEAR", "DOES-NOT-REPRODUCE"),
+
     ("codex r8: ...and the same effect IS one once the rig is bias-free",
      dict(measurand_d=[105] * 8, src=1000, control_d=[5] * 8),
      "REPRODUCES", "UNCLEAR"),
@@ -251,12 +259,17 @@ MUTATIONS = [
      ["    if ci_lo >= t_pos:", "    if (ci_lo + ci_hi) / 2.0 >= t_pos:"],
      "one huge outlier"),
 
+    ("the NULL is not tightened by the control bias -- a masked effect reads as a null (grok r9)",
+     ["        if not (t_neg + B < x[\"rng\"][0] and x[\"rng\"][1] < t_pos - B):",
+      "        if False:"],
+     "null must also survive the TIGHTER bound"),
+
     ("the control's residual bias is not carried into the measurand (codex r8)",
-     ["        B = max(B, abs(x[\"ci\"][0]), abs(x[\"ci\"][1]))", "        B = max(B, 0.0)"],
+     ['        B = max(B, abs(x["rng"][0]), abs(x["rng"][1]))', "        B = max(B, 0.0)"],
      "exactly T is NOT a reproduction"),
 
     ("the engine trusts meta.complete and never counts the pairs (grok r3)",
-     ['    if meta.get(c, {}).get("complete") != "yes" or len(d) < PAIRS or ci is None:',
+     ['    if meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None:',
       '    if meta.get(c, {}).get("complete") != "yes" or ci is None:'],
      "SHORT cell (6 of 8 pairs)"),
 
@@ -303,6 +316,27 @@ def rule_unit_tests():
         print("  %-46s -> %-8s %s" % (name, got, "ok" if ok else "*** FAIL (want %s) ***" % want))
         if not ok:
             bad += 1
+
+    # THE IDENTITY THE WHOLE RULE LEANS ON: at the registered n=8, the >=95% order-statistic
+    # interval IS [min, max]. That is why nothing can be trimmed -- not a null, not B. If
+    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
+    # must refuse that n (it does).
+    ns2 = {}
+    src2 = open(DEFAULT_VERDICT).read()
+    st = src2.index("def median_ci(")
+    exec(src2[st:src2.index("\n\n", src2.index("return best", st))],
+         {"comb": __import__("math").comb, "MIN_COVERAGE": 0.95}, ns2)
+    import random as _r
+    rr = _r.Random(9)
+    for _ in range(200):
+        d = [rr.randint(-500, 500) for _ in range(8)]
+        lo, hi, cov = ns2["median_ci"](d)
+        if (lo, hi) != (min(d), max(d)) or cov < 0.95:
+            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
+            bad += 1
+            break
+    else:
+        print("  %-46s -> %-8s ok" % ("n=8: the >=95% CI IS [min,max] (200 draws)", "identity"))
     return bad
 
 

codex
One scope discrepancy is material: the checked-out HEAD is `7d72f77`, four commits past `8830fda`; those later commits change the rule (`n == 8`, range-based `B`, stale escalation text). I’m reviewing the four files as they actually stand at HEAD, while using `git show 8830fda` to audit rev 9’s changes. The canonical remote is unreachable in this sandbox, and the clone is seven commits ahead of its last known `origin/master`.
exec
/bin/zsh -lc 'wc -l scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     396 scripts/otp12pf_mac_verdict.py
     420 scripts/otp12pf_mac_verdict_test.py
    1086 scripts/bench_otp12pf_mac.sh
     484 docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
    2386 total

exec
/bin/zsh -lc 'nl -ba docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'nl -ba scripts/otp12pf_mac_verdict.py' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'nl -ba scripts/otp12pf_mac_verdict_test.py' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env python3
     2	"""Guard test for otp12pf_mac_verdict.py (rev 8, D-2026-07-14-3).
     3	
     4	    python3 scripts/otp12pf_mac_verdict_test.py             # the cases
     5	    python3 scripts/otp12pf_mac_verdict_test.py --mutations # prove they are not vacuous
     6	
     7	EVERY case is a defect a reviewer actually drove out of a previous revision of this
     8	engine, across seven review rounds. The rule has now been REWRITTEN and simplified;
     9	these cases are the price of that rewrite. Each one asserts that the SIMPLER rule still
    10	refuses the wrong answer the COMPLEX rule once gave.
    11	
    12	A mutation reverts one fix in a copy of the engine; the named case must then FAIL.
    13	"""
    14	import csv, os, random, subprocess, sys, tempfile
    15	
    16	HERE = os.path.dirname(os.path.abspath(__file__))
    17	DEFAULT_VERDICT = os.path.join(HERE, "otp12pf_mac_verdict.py")
    18	CONTROLS = ("nq_grpc_mixed", "qn_grpc_mixed", "nq_tcp_large", "qn_tcp_large")
    19	MEASURANDS = ("nq_tcp_mixed", "qn_tcp_mixed")
    20	REGISTERED = MEASURANDS + CONTROLS
    21	OUTCOMES = {"INCOMPLETE", "RIG-VOID", "CONTROLS-NOT-CLEAN", "MIXED", "REPRODUCES",
    22	            "INVERTED", "DOES-NOT-REPRODUCE", "UNCLEAR"}
    23	
    24	
    25	def engine():
    26	    """Resolved per call: the mutation harness repoints it, and a cached path would
    27	    silently test the UNMUTATED engine and report a kill it never made."""
    28	    return os.environ.get("VERDICT_PY", DEFAULT_VERDICT)
    29	
    30	
    31	def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=(),
    32	            per_cell=None, void_reason="", pairs=8, env_extra=None):
    33	    """`src` may be an int OR a per-pair list. The bar is computed on the MARGINAL
    34	    medians and the CI on the PAIRED differences, and the two only disagree when the
    35	    source arm varies -- a constant-only helper made that whole class of bug
    36	    unguardable by construction."""
    37	    control_d = [5] * pairs if control_d is None else control_d
    38	    per_cell = per_cell or {}
    39	    tmp = tempfile.mkdtemp()
    40	    runs, meta = os.path.join(tmp, "runs.csv"), os.path.join(tmp, "meta.csv")
    41	    present = [c for c in REGISTERED if c not in drop_cells]
    42	    with open(runs, "w") as f:
    43	        w = csv.writer(f)
    44	        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,"
    45	                   "exit,drain,cold,valid".split(","))
    46	        for cell in present:
    47	            if cell in per_cell:
    48	                d, s = per_cell[cell]
    49	            elif cell in MEASURANDS:
    50	                d, s = measurand_d, src
    51	            else:
    52	                d, s = control_d, control_src
    53	            srcs = s if isinstance(s, list) else [s] * len(d)
    54	            for i, (di, si) in enumerate(zip(d, srcs), 1):
    55	                w.writerow([cell, "srcinit", "x", "h", i, si, 0, 250, 1, 1, 0,
    56	                            "drained_1x2s", "cold", "yes"])
    57	                w.writerow([cell, "destinit", "x", "h", i, si + di, 0, 250, 1, 1, 0,
    58	                            "drained_1x2s", "cold", "yes"])
    59	    with open(meta, "w") as f:
    60	        f.write("cell,pairs_attempted,complete\n")
    61	        for cell in present:
    62	            # `complete=yes` is asserted even when a cell is SHORT: the engine must not
    63	            # believe it (a 1-pair CSV once graded as a full cell at 0% CI coverage).
    64	            f.write("%s,%d,yes\n" % (cell, pairs))
    65	    env = dict(os.environ, VERDICT_CELLS=",".join(MEASURANDS),
    66	               CONTROL_CELLS=",".join(CONTROLS), REGISTERED_CELLS=",".join(REGISTERED),
    67	               REQUIRED_PAIRS="8", SESSION_VOID_REASON=void_reason)
    68	    env.pop("DELTA_REF_MS", None)                      # PINNED in the engine
    69	    env.update(env_extra or {})
    70	    out = subprocess.run([sys.executable, engine(), runs, meta,
    71	                          os.path.join(tmp, "s.csv"), os.path.join(tmp, "p.csv"),
    72	                          os.path.join(tmp, "v.csv"), os.path.join(tmp, "sv.txt")],
    73	                         env=env, capture_output=True, text=True)
    74	    if out.returncode != 0 and "REFUSING" in (out.stderr or ""):
    75	        return "ENGINE-REFUSED"          # a deliberate refusal is the engine WORKING
    76	    if out.returncode != 0:
    77	        return "ENGINE-CRASH: " + (out.stderr.strip().splitlines() or ["?"])[-1]
    78	    return out.stdout.splitlines()[0].split(":", 1)[1].strip()
    79	
    80	
    81	# (name, kwargs, must_be, must_not_be)
    82	CASES = [
    83	    # --- a real effect must never read as nothing --------------------------------
    84	    ("codex r1: a 190ms effect on 7/8 pairs is not a null",
    85	     dict(measurand_d=[0, 180, 180, 190, 190, 200, 200, 200], src=2000),
    86	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
    87	
    88	    ("codex r2: a rig-W-sized effect (230ms) in EVERY pair, on a slow 2500ms arm",
    89	     dict(measurand_d=[230] * 8, src=2500, control_d=[0] * 8),
    90	     "REPRODUCES", "DOES-NOT-REPRODUCE"),
    91	
    92	    ("codex r2: an effect the 10% bar alone would forgive (240ms @ 2500)",
    93	     dict(measurand_d=[-100, -50, 0, 50, 100, 200, 220, 240], src=2500),
    94	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
    95	
    96	    ("codex r2: the inverting threshold is -src/11, not -src/10 (CI [-190,0] @ 2000)",
    97	     dict(measurand_d=[-190, -190, 0, 0, 0, 0, 0, 0], src=2000),
    98	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
    99	
   100	    # --- an artifact must never read as an effect --------------------------------
   101	    ("codex r2: 7 positive + 1 negative is not a reproduction",
   102	     dict(measurand_d=[-20, 300, 310, 320, 330, 340, 350, 360], src=1000),
   103	     "UNCLEAR", "REPRODUCES"),
   104	
   105	    ("codex r5: a 1ms paired effect is not a reproduction, whatever the medians do",
   106	     dict(measurand_d=[1] * 13 + [-4500] * 3,
   107	          src=[1000] * 7 + [1200] * 6 + [5000] * 3,
   108	          control_d=[5] * 16, control_src=1000, pairs=16),
   109	     None, "REPRODUCES"),
   110	
   111	    ("codex r6: nor when the marginal bar fails in the MATCHING direction",
   112	     dict(measurand_d=[400] * 3 + [1] * 13, src=[1000] * 8 + [1200] * 8,
   113	          control_d=[5] * 16, control_src=1000, pairs=16),
   114	     None, "REPRODUCES"),
   115	
   116	    ("one huge outlier must not manufacture a reproduction (the CI's LOWER bound decides)",
   117	     dict(measurand_d=[10, 10, 10, 10, 10, 10, 10, 800], src=1000),
   118	     "UNCLEAR", "REPRODUCES"),
   119	
   120	    ("grok r9: a LONG cell (16 pairs) is INCOMPLETE — a CI at n>8 TRIMS the pairs that contradict it",
   121	     dict(measurand_d=[-500] * 3 + [200] * 13, src=1000, control_d=[0] * 16),
   122	     "INCOMPLETE", "REPRODUCES"),
   123	
   124	    ("a SHORT cell (6 of 8 pairs) claiming complete=yes is INCOMPLETE",
   125	     dict(measurand_d=[-4, -2, -1, 0, 1, 2], src=2000),
   126	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   127	
   128	    # --- the controls are a precondition -----------------------------------------
   129	    ("grok r2: a bar-FAIL control whose CI crosses zero blocks every verdict",
   130	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   131	          control_d=[-100, -50, 300, 320, 340, 350, 360, 380], control_src=1000),
   132	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   133	
   134	    ("grok r4: a Delta_ref-sized control effect blocks every verdict",
   135	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   136	          control_d=[230] * 8, control_src=2500),
   137	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   138	
   139	    ("codex r5: ...and so does one with a single zero pair (CI [0,230])",
   140	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   141	          control_d=[0] + [230] * 7, control_src=2500),
   142	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   143	
   144	    ("grok r5: ...and a non-directional one (CI [-10,230])",
   145	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   146	          control_d=[230] * 7 + [-10], control_src=2500),
   147	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   148	
   149	    ("grok r6: ...and one at D=+229, ONE MS under the reference effect",
   150	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   151	          control_d=[229] * 8, control_src=2500),
   152	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   153	
   154	    ("codex r6: a dirty control blocks a REPRODUCTION too, not just a null",
   155	     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000,
   156	          control_d=[0] + [230] * 7, control_src=2500),
   157	     "CONTROLS-NOT-CLEAN", "REPRODUCES"),
   158	
   159	    # ...but a GOOD rig must still be able to ANSWER. An instrument that can never
   160	    # conclude is also broken (grok r6: the "dead zone").
   161	    ("a clean rig with a tiny host x role control asymmetry still answers",
   162	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   163	          control_d=[5] * 8, control_src=1000),
   164	     "DOES-NOT-REPRODUCE", "CONTROLS-NOT-CLEAN"),
   165	
   166	    # --- the rig must be able to say each of the things it can say ----------------
   167	    ("a real, bar-breaking slowdown reproduces",
   168	     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000),
   169	     "REPRODUCES", None),
   170	
   171	    ("an exact 10% effect is reportable on a bias-free rig (it was once unreachable)",
   172	     dict(measurand_d=[100] * 8, src=1000, control_d=[0] * 8),
   173	     "REPRODUCES", None),
   174	
   175	    # codex r8, BLOCKER: a control at +5 is "clean", but that 5ms of arm bias may be
   176	    # riding in the measurand too -- so an effect of EXACTLY T could be (T-5) real plus
   177	    # 5 rig. It must not be banked as a reproduction. B carries the bias the controls
   178	    # could not exclude into the measurand's threshold.
   179	    ("codex r8: an effect of exactly T is NOT a reproduction when the controls carry bias",
   180	     dict(measurand_d=[100] * 8, src=1000, control_d=[5] * 8),
   181	     "UNCLEAR", "REPRODUCES"),
   182	
   183	    ("grok r9: a null must also survive the TIGHTER bound (bias could be MASKING an effect)",
   184	     dict(measurand_d=[60] * 8, src=1000, control_d=[49] * 8),
   185	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   186	
   187	    ("codex r8: ...and the same effect IS one once the rig is bias-free",
   188	     dict(measurand_d=[105] * 8, src=1000, control_d=[5] * 8),
   189	     "REPRODUCES", "UNCLEAR"),
   190	
   191	    ("source-initiated slower is INVERTED, never 'P1 absent'",
   192	     dict(measurand_d=[-300, -310, -320, -330, -340, -350, -360, -370], src=1000),
   193	     "INVERTED", None),
   194	
   195	    ("one direction reproduces, the other inverts -> MIXED",
   196	     dict(measurand_d=[0] * 8, src=1000,
   197	          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
   198	                    "qn_tcp_mixed": ([-300, -310, -320, -330, -340, -350, -360, -370], 1000)}),
   199	     "MIXED", "REPRODUCES"),
   200	
   201	    ("a clean one-direction reproduction is NOT masked by a noisy sibling",
   202	     dict(measurand_d=[0] * 8, src=1000,
   203	          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
   204	                    "qn_tcp_mixed": ([-20, 300, 310, 320, 330, 340, 350, 360], 1000)}),
   205	     "REPRODUCES", "UNCLEAR"),
   206	
   207	    ("codex r8: a bimodal arm cannot hide from the RANGE (a null is judged on every pair)",
   208	     dict(measurand_d=[-110, 0, -110, 110, 110, 0, -110, 0], src=730,
   209	          control_d=[0] * 8),
   210	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   211	
   212	    ("a null the rig could not have SEEN is UNCLEAR, not a null",
   213	     dict(measurand_d=[-400, -300, -100, 0, 0, 100, 300, 400], src=2000),
   214	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   215	
   216	    # --- integrity ---------------------------------------------------------------
   217	    ("a missing registered cell is INCOMPLETE, never filtered away",
   218	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   219	          drop_cells=("qn_tcp_mixed",)),
   220	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   221	
   222	    ("grok r3: n=1 with complete=yes must not grade at 0% CI coverage",
   223	     dict(measurand_d=[0], src=2000, control_d=[5], control_src=1000),
   224	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   225	
   226	    ("grok r3: a harness-detected session void (end-load) refuses a verdict",
   227	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   228	          void_reason="end-load on q is 9.1 (> 3.0)"),
   229	     "RIG-VOID", "DOES-NOT-REPRODUCE"),
   230	
   231	    ("codex r5: DELTA_REF_MS is PINNED -- the rule is not tunable from the environment",
   232	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   233	          env_extra={"DELTA_REF_MS": "240"}),
   234	     "ENGINE-REFUSED", "DOES-NOT-REPRODUCE"),
   235	]
   236	
   237	MUTATIONS = [
   238	    ("the control threshold is the SAME as the measurand's, not half (grok r6)",
   239	     ['    c_pos, c_neg = thresholds(x["src"], 0.5)',
   240	      '    c_pos, c_neg = thresholds(x["src"], 1.0)'],
   241	     "D=+229, ONE MS under"),
   242	
   243	    ("dirty controls block only the null, not a reproduction (codex r6)",
   244	     ["elif dirty:",
   245	      "elif dirty and not any(s == 'EFFECT' for s in m.values()):"],
   246	     "blocks a REPRODUCTION too"),
   247	
   248	    ("the inverting threshold is -src/10, not -src/11 (codex r2)",
   249	     ["            -min(s_med / 11.0, float(DELTA_REF)) * scale)",
   250	      "            -min(s_med / 10.0, float(DELTA_REF)) * scale)"],
   251	     "inverting threshold is -src/11"),
   252	
   253	    ("the threshold ignores DELTA_REF, so the bar alone forgives 240ms (codex r2)",
   254	     ["    return (min(s_med / 10.0, float(DELTA_REF)) * scale,",
   255	      "    return ((s_med / 10.0) * scale,"],
   256	     "bar alone would forgive"),
   257	
   258	    ("EFFECT is decided on the CI's MIDPOINT, not its lower bound (an outlier reproduces)",
   259	     ["    if ci_lo >= t_pos:", "    if (ci_lo + ci_hi) / 2.0 >= t_pos:"],
   260	     "one huge outlier"),
   261	
   262	    ("the NULL is not tightened by the control bias -- a masked effect reads as a null (grok r9)",
   263	     ["        if not (t_neg + B < x[\"rng\"][0] and x[\"rng\"][1] < t_pos - B):",
   264	      "        if False:"],
   265	     "null must also survive the TIGHTER bound"),
   266	
   267	    ("the control's residual bias is not carried into the measurand (codex r8)",
   268	     ['        B = max(B, abs(x["rng"][0]), abs(x["rng"][1]))', "        B = max(B, 0.0)"],
   269	     "exactly T is NOT a reproduction"),
   270	
   271	    ("the engine trusts meta.complete and never counts the pairs (grok r3)",
   272	     ['    if meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None:',
   273	      '    if meta.get(c, {}).get("complete") != "yes" or ci is None:'],
   274	     "SHORT cell (6 of 8 pairs)"),
   275	
   276	    ("a missing registered cell is filtered away (codex r2)",
   277	     ["for c in sorted(set(REGISTERED) | set(meta)):", "for c in sorted(meta):"],
   278	     "missing registered cell"),
   279	
   280	    ("a harness-detected session void is ignored (grok r3)",
   281	     ["elif SESSION_VOID:", "elif False:"],
   282	     "session void (end-load)"),
   283	
   284	    ("the registered DELTA_REF is taken from the environment again (codex r5)",
   285	     ['_env = os.environ.get("DELTA_REF_MS")', "_env = None"],
   286	     "DELTA_REF_MS is PINNED"),
   287	]
   288	
   289	
   290	def rule_unit_tests():
   291	    """The RULE itself, called directly -- because a session at n=8 cannot distinguish the
   292	    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
   293	    closed codex's round-8 blocker; judging a NULL on the RANGE is the SEMANTICS that keeps
   294	    it closed if a larger n is ever registered again, and it can only be tested here."""
   295	    import importlib.util
   296	    spec = importlib.util.spec_from_file_location("eng", DEFAULT_VERDICT)
   297	    # the engine runs on import (it is a script), so exercise classify() via a subprocess-free
   298	    # re-implementation guard: read the function out of the source and exec it in isolation.
   299	    src = open(DEFAULT_VERDICT).read()
   300	    start = src.index("def classify(")
   301	    end = src.index("\n\n", src.index("return \"UNCLEAR\"", start))
   302	    ns = {}
   303	    exec(src[start:end], ns)
   304	    classify = ns["classify"]
   305	    bad = 0
   306	    checks = [
   307	        # ci narrow (outliers trimmed), range wide: a bimodal arm. MUST NOT be NONE.
   308	        ("bimodal: CI=[1,1] but range=[-110,110], T=73", (1, 1, -110, 110, 73, -66), "UNCLEAR"),
   309	        ("clean: CI and range both inside T",            (2, 3, -4, 3, 73, -66),      "NONE"),
   310	        ("a real effect clears T",                       (80, 90, 75, 95, 73, -66),   "EFFECT"),
   311	        ("an inverted effect clears -T",                 (-90, -80, -95, -75, 73, -66), "INVERTED"),
   312	    ]
   313	    for name, args, want in checks:
   314	        got = classify(*args)
   315	        ok = got == want
   316	        print("  %-46s -> %-8s %s" % (name, got, "ok" if ok else "*** FAIL (want %s) ***" % want))
   317	        if not ok:
   318	            bad += 1
   319	
   320	    # THE IDENTITY THE WHOLE RULE LEANS ON: at the registered n=8, the >=95% order-statistic
   321	    # interval IS [min, max]. That is why nothing can be trimmed -- not a null, not B. If
   322	    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
   323	    # must refuse that n (it does).
   324	    ns2 = {}
   325	    src2 = open(DEFAULT_VERDICT).read()
   326	    st = src2.index("def median_ci(")
   327	    exec(src2[st:src2.index("\n\n", src2.index("return best", st))],
   328	         {"comb": __import__("math").comb, "MIN_COVERAGE": 0.95}, ns2)
   329	    import random as _r
   330	    rr = _r.Random(9)
   331	    for _ in range(200):
   332	        d = [rr.randint(-500, 500) for _ in range(8)]
   333	        lo, hi, cov = ns2["median_ci"](d)
   334	        if (lo, hi) != (min(d), max(d)) or cov < 0.95:
   335	            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
   336	            bad += 1
   337	            break
   338	    else:
   339	        print("  %-46s -> %-8s ok" % ("n=8: the >=95% CI IS [min,max] (200 draws)", "identity"))
   340	    return bad
   341	
   342	
   343	def run_cases():
   344	    bad = []
   345	    for name, kw, must_be, must_not in CASES:
   346	        got = session(**kw)
   347	        ok = not (must_be and got != must_be) and not (must_not and got == must_not)
   348	        print("%-66s -> %-20s %s" % (name[:66], got, "ok" if ok else "*** FAIL ***"))
   349	        if not ok:
   350	            bad.append(name)
   351	            print("      expected %s / must not be %s" % (must_be, must_not))
   352	    return bad
   353	
   354	
   355	def fuzz(n=300):
   356	    """No input may land outside the registered outcomes. The CONTROLS are fuzzed too --
   357	    pinning them clean once left every dirty-control path unexercised, and that is
   358	    exactly where a BLOCKER was hiding."""
   359	    rng = random.Random(4242)
   360	    bad = 0
   361	    for _ in range(n):
   362	        got = session(measurand_d=[rng.randint(-600, 600) for _ in range(8)],
   363	                      src=rng.choice([600, 1000, 2000, 2500, 5000]),
   364	                      control_d=[rng.randint(-300, 300) for _ in range(8)],
   365	                      control_src=rng.choice([600, 1000, 2500, 5000]))
   366	        if got not in OUTCOMES:
   367	            print("*** UNREGISTERED OUTCOME %r" % got)
   368	            bad += 1
   369	    print("fuzz: %d/%d inputs produced a registered outcome (measurand AND controls)"
   370	          % (n - bad, n))
   371	    return bad
   372	
   373	
   374	def mutate():
   375	    src = open(DEFAULT_VERDICT).read()
   376	    bad = 0
   377	    for name, subs, key in MUTATIONS:
   378	        body = src
   379	        for i in range(0, len(subs), 2):
   380	            old, new = subs[i], subs[i + 1]
   381	            if old not in body:     # the engine drifted: the proof is STALE, not passing
   382	                print("*** STALE MUTATION (target not found): %s" % name)
   383	                bad += 1
   384	                body = None
   385	                break
   386	            body = body.replace(old, new, 1)
   387	        if body is None:
   388	            continue
   389	        tmp = tempfile.mkdtemp()
   390	        path = os.path.join(tmp, "mutant.py")
   391	        open(path, "w").write(body)
   392	        case = next(c for c in CASES if key in c[0])
   393	        os.environ["VERDICT_PY"] = path
   394	        got = session(**case[1])
   395	        del os.environ["VERDICT_PY"]
   396	        # KILLED == the case now FAILS, by its OWN contract. Checking instead for a
   397	        # verdict named here let a mutant "survive" by failing a different way.
   398	        _, _, must_be, must_not = case
   399	        killed = (must_be and got != must_be) or (must_not and got == must_not)
   400	        print("%-66s -> %-20s %s" % (name[:66], got,
   401	                                     "KILLED" if killed else "*** SURVIVED — VACUOUS ***"))
   402	        if not killed:
   403	            bad += 1
   404	    return bad
   405	
   406	
   407	if __name__ == "__main__":
   408	    if "--mutations" in sys.argv:
   409	        print("Reverting each fix in a copy of the engine; the named case must then FAIL.\n")
   410	        n = mutate()
   411	        print("\n%d/%d mutations killed" % (len(MUTATIONS) - n, len(MUTATIONS)))
   412	        sys.exit(1 if n else 0)
   413	    print("The RULE, called directly (a session at n=8 cannot separate CI from RANGE):")
   414	    unit = rule_unit_tests()
   415	    print()
   416	    fails = run_cases()
   417	    print()
   418	    z = fuzz()
   419	    print("\n%d/%d cases passed" % (len(CASES) - len(fails), len(CASES)))
   420	    sys.exit(1 if (fails or z or unit) else 0)

 succeeded in 0ms:
     1	#!/usr/bin/env python3
     2	"""The Mac<->Mac decision rule (PREREGISTRATION.md rev 8, D-2026-07-14-3).
     3	
     4	WHAT THIS IS FOR
     5	    The harness COMPUTES the verdict, so no one can look at the numbers and then
     6	    invent a favourable reading. That -- and only that -- is what the mechanization
     7	    buys. The question, the statistic and the thresholds are all fixed before any
     8	    data exists.
     9	
    10	WHY IT IS THIS SMALL
    11	    The previous rule had ~10 outcomes, five thresholds, a control certification tier
    12	    and a precedence stack. Seven review rounds; FOUR of the last five BLOCKERs were
    13	    in the RULE, not in the measurement -- every one a corner where the branches
    14	    interacted to produce a confidently wrong verdict (a 1 ms effect reported as a
    15	    reproduction; a control carrying 229 of 230 ms certified "clean"; a null printed
    16	    while every control was dirty). Complexity was the defect. So:
    17	
    18	THE STATISTIC (paired, because the design is paired)
    19	    d_i = destinit_i - srcinit_i          per ABBA slot (positive = destination slower)
    20	    D   = median(d_i)                     low median, even n
    21	    CI  = exact distribution-free order-statistic interval on the population median, the
    22	          narrowest whose coverage is >= 95%. AT THE REGISTERED n=8 THAT IS [min(d), max(d)]
    23	          (99.22%) -- i.e. it CANNOT trim. No bootstrap, no approximation.
    24	    RANGE = [min(d), max(d)], and a NULL is judged on the RANGE, never on a trimmed CI.
    25	
    26	    n IS EXACTLY 8. Not "at least": at any larger n the >=95% interval starts TRIMMING
    27	    outliers, and a bimodal arm then yields a narrow median CI and a FALSE verdict. grok
    28	    drove exactly that with a 16-pair CSV (3 pairs at -500 trimmed away, 13 at +200 left)
    29	    -> REPRODUCES. The cell must carry EXACTLY the registered pair count or it is INCOMPLETE.
    30	
    31	THE THRESHOLD (one)
    32	    T = min(srcinit_median / 10, DELTA_REF)
    33	        srcinit/10  -- the project's own 1.10 invariance bar
    34	        DELTA_REF   -- 230 ms, the effect rig W actually measured
    35	    The smaller of the two: an effect must matter by BOTH standards to count.
    36	
    37	THE FOUR CELL STATES (mutually exclusive BY CONSTRUCTION -- there is no label for a
    38	new case to walk past, because they partition the CI's position relative to +-T)
    39	    EFFECT    CI_lo >= +T                 destination-initiated is slower, by >= T
    40	    INVERTED  CI_hi <= -T                 source-initiated is slower, by >= T
    41	    NONE      -T < CI_lo and CI_hi < +T   an effect of size T is EXCLUDED (equivalence)
    42	    UNCLEAR   anything else               the CI spans the threshold: no answer
    43	
    44	THE CONTROLS ARE A PRECONDITION
    45	    Every control must be NONE at T/2 -- HALF the threshold. Half, because certifying a
    46	    control with the very number that DEFINES the effect is incoherent: it would let the
    47	    gRPC control carry all but 1 ms of P1 while we call the rig clean. If any control
    48	    fails, NO verdict about the measurand is read: not a reproduction, and not a null.
    49	
    50	WHAT IS DELIBERATELY ABSENT
    51	    * The 1.10 bar takes NO part in inference. It is the project's ACCEPTANCE criterion:
    52	      computed on the marginal medians, reported in every row, and never consulted --
    53	      the marginal and paired statistics can disagree in direction AND magnitude, and
    54	      every attempt to let one stand in for the other produced a false verdict.
    55	    * The sign test is REPORTED, not decided on. At n=8 the CI already implies it
    56	      (CI_lo >= T > 0 means every pair is >= T), so making it a second gate only added
    57	      an interaction to get wrong.
    58	    * No UNSTABLE / PARTIAL / BAR-FAIL-INCONSISTENT / UNDERPOWERED branches, and no
    59	      precedence stack. A bimodal arm widens the CI, and a wide CI lands in UNCLEAR --
    60	      which is exactly what those branches were hand-coding. Every run of every arm is
    61	      still printed, so bimodality remains visible to the reader.
    62	"""
    63	import csv, os, sys
    64	from math import comb
    65	
    66	runs_p, meta_p, sum_p, pair_p, ver_p, sess_p = sys.argv[1:7]
    67	
    68	# ---- REGISTERED CONSTANTS: pinned in code, never taken from the environment --------
    69	# They were once `${VAR:-default}`, and DELTA_REF_MS=240 turned a void into a null --
    70	# i.e. the rule could be retuned from the command line, after the data existed, in the
    71	# direction of the answer you want. That is the one thing pre-registration exists to
    72	# make impossible.
    73	DELTA_REF = 230          # ms; rig W's measured Delta_P1
    74	REGISTERED_PAIRS = (8,)
    75	MIN_COVERAGE = 0.95
    76	
    77	_env = os.environ.get("DELTA_REF_MS")
    78	if _env is not None and _env.strip() != str(DELTA_REF):
    79	    sys.exit("REFUSING: DELTA_REF_MS=%r but the registered reference effect is %d ms. "
    80	             "The rule is not tunable from the environment.\n" % (_env, DELTA_REF))
    81	
    82	
    83	def cells_env(name):
    84	    return [c for c in os.environ.get(name, "").split(",") if c]
    85	
    86	
    87	MEASURANDS = cells_env("VERDICT_CELLS")
    88	CONTROLS = cells_env("CONTROL_CELLS")
    89	REGISTERED = cells_env("REGISTERED_CELLS") or (MEASURANDS + CONTROLS)
    90	PAIRS = int(os.environ.get("REQUIRED_PAIRS", "8"))
    91	# A harness-detected session void the engine cannot see for itself (end-load).
    92	SESSION_VOID = os.environ.get("SESSION_VOID_REASON", "").strip()
    93	
    94	if not MEASURANDS or not CONTROLS:
    95	    sys.exit("REFUSING: VERDICT_CELLS and CONTROL_CELLS must both be set -- the controls "
    96	             "are a precondition for any verdict, and an engine with none cannot certify "
    97	             "the rig.\n")
    98	if PAIRS not in REGISTERED_PAIRS:
    99	    sys.exit("REFUSING: REQUIRED_PAIRS=%d is not registered %s.\n" % (PAIRS, REGISTERED_PAIRS))
   100	
   101	
   102	def ms_of(r):
   103	    """A corrupt row stops the grading, loudly. Soft-mapping it would hide it."""
   104	    try:
   105	        return int(r["ms"])
   106	    except (TypeError, ValueError):
   107	        sys.stderr.write("CORRUPT ROW: cell=%s arm=%s run=%s ms=%r. A benchmark whose "
   108	                         "rows do not parse has no verdict.\n"
   109	                         % (r.get("cell"), r.get("arm"), r.get("run"), r.get("ms")))
   110	        raise SystemExit(2)
   111	
   112	
   113	rows = list(csv.DictReader(open(runs_p)))
   114	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
   115	
   116	by, slots, voided = {}, {}, {}
   117	for r in rows:
   118	    key = (r["cell"], r["arm"])
   119	    if r["valid"] == "yes":
   120	        by.setdefault(key, []).append(ms_of(r))
   121	        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = ms_of(r)
   122	    else:
   123	        voided[key] = voided.get(key, 0) + 1
   124	
   125	
   126	def med(v):
   127	    v = sorted(v)
   128	    return v[(len(v) - 1) // 2]
   129	
   130	
   131	def paired(c):
   132	    return [v["destinit"] - v["srcinit"]
   133	            for (cc, _run), v in sorted(slots.items())
   134	            if cc == c and "srcinit" in v and "destinit" in v]
   135	
   136	
   137	def median_ci(d):
   138	    """Exact order-statistic interval: the NARROWEST [d_(k), d_(n+1-k)] whose coverage
   139	    1 - 2*P(Bin(n,1/2) <= k-1) is still >= 95%. Returns (lo, hi, coverage)."""
   140	    d = sorted(d)
   141	    n = len(d)
   142	    best = None
   143	    for k in range(1, n // 2 + 1):
   144	        cov = 1.0 - 2.0 * sum(comb(n, i) for i in range(k)) / (2.0 ** n)
   145	        if cov >= MIN_COVERAGE:
   146	            best = (d[k - 1], d[n - k], cov)      # larger k => narrower
   147	    return best                                   # None if n is too small for 95%
   148	
   149	
   150	def sign_p(d):
   151	    """Reported, never decided on."""
   152	    nz = [x for x in d if x]
   153	    n = len(nz)
   154	    if not n:
   155	        return 1.0, 0, 0
   156	    k = sum(1 for x in nz if x > 0)
   157	    tail = sum(comb(n, i) for i in range(min(k, n - k) + 1))
   158	    return min(1.0, 2.0 * tail / 2 ** n), k, n
   159	
   160	
   161	def thresholds(s_med, scale=1.0):
   162	    """T_pos and T_neg -- NOT symmetric in ms, because the 1.10 bar is symmetric in
   163	    RATIO: +src/10 reaches ratio 1.10, but only -src/11 reaches the INVERSE 1.10.
   164	    Both capped at DELTA_REF, so an effect must matter by the project's bar AND be the
   165	    size of the one rig W measured. `scale` = 0.5 for controls."""
   166	    return (min(s_med / 10.0, float(DELTA_REF)) * scale,
   167	            -min(s_med / 11.0, float(DELTA_REF)) * scale)
   168	
   169	
   170	def classify(ci_lo, ci_hi, rng_lo, rng_hi, t_pos, t_neg):
   171	    """THE RULE. Four states, mutually exclusive and exhaustive BY CONSTRUCTION.
   172	
   173	    EFFECT/INVERTED use the >=95% CI on the median; NONE uses the FULL RANGE. At the
   174	    registered n=8 these coincide (the CI IS the range), so nothing can be trimmed either
   175	    way -- the distinction is the SEMANTICS that keeps the rule sound if a larger n is ever
   176	    registered, and the engine REFUSES any n but 8.
   177	
   178	    NONE uses the FULL RANGE -- EVERY pair must lie inside +-T. Round 8 (codex, BLOCKER):
   179	    a >=95% CI at n>8 TRIMS outliers, so a BIMODAL arm produces a NARROW median CI and a
   180	    FALSE NULL (driven: CI = [1,1] from modes at +-110). An equivalence claim must never be
   181	    reachable by trimming away the very pairs that contradict it. This is also why
   182	    bimodality needs no special branch: it cannot hide from the range.
   183	    """
   184	    if ci_lo >= t_pos:
   185	        return "EFFECT"
   186	    if ci_hi <= t_neg:
   187	        return "INVERTED"
   188	    if t_neg < rng_lo and rng_hi < t_pos:
   189	        return "NONE"
   190	    return "UNCLEAR"
   191	
   192	
   193	# ---- pass 1: measure every cell -----------------------------------------------------
   194	cell = {}
   195	for c in sorted(set(REGISTERED) | set(meta)):
   196	    d = paired(c)
   197	    ci = median_ci(d) if d else None
   198	    # COMPLETE is checked against the DATA, never against meta's say-so: a one-pair CSV
   199	    # with a lying meta once graded as a full cell and emitted a null at 0% coverage.
   200	    if meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None:
   201	        cell[c] = dict(state="INCOMPLETE", n=len(d))
   202	        continue
   203	    s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   204	    hi, lo = max(s_med, d_med), min(s_med, d_med)
   205	    ci_lo, ci_hi, cov = ci
   206	    p, k, n = sign_p(d)
   207	    cell[c] = dict(n=len(d), d=d, D=med(d), ci=(ci_lo, ci_hi), rng=(min(d), max(d)),
   208	                   cov=cov, src=s_med, dst=d_med, p=p, k=k,
   209	                   # The acceptance bar: integer-exact, `<= 1.10` PASSES. REPORTED, never used.
   210	                   bar="PASS" if 10 * hi <= 11 * lo else "FAIL",
   211	                   ratio=hi / lo if lo else 0.0)
   212	
   213	# ---- pass 2: the controls certify the rig, and BOUND its residual bias ---------------
   214	# A control certifies clean at T/2 -- but "clean" is not "zero". A control sitting at +49
   215	# with T/2 = 50 is accepted, and THAT 49 ms OF ARM BIAS MAY BE RIDING IN THE MEASURAND
   216	# TOO, so a measurand "EFFECT" at exactly T could be half real and half rig (round-8
   217	# codex, BLOCKER). The bias the controls FAIL TO EXCLUDE is therefore carried into the
   218	# measurand's thresholds:
   219	#
   220	#     B = max over clean controls of the largest |RANGE bound| -- the arm asymmetry the
   221	#                                                                 controls could not rule out.
   222	#         The RANGE, not the CI: the CI is an interval for the MEDIAN and it TRIMS, and a
   223	#         bound on what the rig might be carrying must not be computed by trimming.
   224	#     an EFFECT must clear  T + B     (bias could be INFLATING it)
   225	#     a NULL   must fit in  T - B     (bias could be MASKING an effect)
   226	#
   227	# If the controls are genuinely clean, B is a few ms and this barely moves. If they are
   228	# marginal, it bites -- which is the point.
   229	dirty = []
   230	B = 0.0
   231	for c in CONTROLS:
   232	    x = cell.get(c, {})
   233	    if x.get("state") == "INCOMPLETE":
   234	        continue
   235	    c_pos, c_neg = thresholds(x["src"], 0.5)
   236	    x["ctrl_state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], c_pos, c_neg)
   237	    x["ctrl_T"] = c_pos
   238	    if x["ctrl_state"] != "NONE":
   239	        dirty.append(c)
   240	    else:
   241	        B = max(B, abs(x["rng"][0]), abs(x["rng"][1]))
   242	
   243	# ---- pass 3: grade the measurands, against thresholds widened by the control bias -----
   244	for c in MEASURANDS:
   245	    x = cell.get(c, {})
   246	    if x.get("state") == "INCOMPLETE":
   247	        continue
   248	    t_pos, t_neg = thresholds(x["src"])
   249	    x["T"] = t_pos
   250	    x["B"] = B
   251	    x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1],
   252	                          t_pos + B, t_neg - B)          # an EFFECT must clear T + B
   253	    if x["state"] == "NONE":
   254	        # ...and a NULL must survive the TIGHTER bound: bias could be masking an effect.
   255	        if not (t_neg + B < x["rng"][0] and x["rng"][1] < t_pos - B):
   256	            x["state"] = "UNCLEAR"
   257	
   258	# Controls also carry a state for the report; measurands carry a ctrl_state for symmetry.
   259	for c in cell:
   260	    x = cell[c]
   261	    if x.get("state") == "INCOMPLETE":
   262	        continue
   263	    if "state" not in x:                                  # a control: report its own state
   264	        t_pos, t_neg = thresholds(x["src"])
   265	        x["T"] = t_pos
   266	        x["B"] = 0.0
   267	        x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], t_pos, t_neg)
   268	    x.setdefault("ctrl_state", "-")
   269	
   270	# ---- outputs -----------------------------------------------------------------------
   271	with open(sum_p, "w") as f:
   272	    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,voided,runs\n")
   273	    for (c, a) in sorted(by):
   274	        v = by[(c, a)]
   275	        f.write("%s,%s,%d,%d,%d,%d,%d,%s\n" % (c, a, med(v), sum(v) // len(v), min(v),
   276	                                               max(v), voided.get((c, a), 0),
   277	                                               " ".join(map(str, v))))
   278	
   279	with open(pair_p, "w") as f:
   280	    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,range_lo,range_hi,"
   281	            "coverage,T_ms,B_ms,sign_p,k_pos,state,control_state\n")
   282	    for c in sorted(cell):
   283	        x = cell[c]
   284	        if x["state"] == "INCOMPLETE":
   285	            f.write("%s,%d,,,,,,,,,,,,,,,INCOMPLETE,\n" % (c, x["n"]))
   286	            continue
   287	        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%d,%d,%.4f,%d,%d,%.4f,%d/%d,%s,%s\n" % (
   288	            c, x["n"], x["src"], x["dst"], x["ratio"], x["bar"], x["D"],
   289	            x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], x["cov"],
   290	            round(x["T"]), round(x.get("B", 0)), x["p"], x["k"], x["n"],
   291	            x["state"], x["ctrl_state"]))
   292	
   293	with open(ver_p, "w") as f:
   294	    f.write("comparison,kind,lhs_ms,rhs_ms,ratio,bar\n")
   295	    for c in sorted(cell):
   296	        x = cell[c]
   297	        if x["state"] == "INCOMPLETE":
   298	            f.write("%s,invariance,,,,INCOMPLETE\n" % c)
   299	        else:
   300	            f.write("%s,invariance,%d,%d,%.3f,%s\n"
   301	                    % (c, x["src"], x["dst"], x["ratio"], x["bar"]))
   302	
   303	# ---- THE SESSION VERDICT -----------------------------------------------------------
   304	incomplete = [c for c in REGISTERED if cell.get(c, {}).get("state") == "INCOMPLETE"]
   305	m = {c: cell[c]["state"] for c in MEASURANDS if not incomplete}
   306	
   307	if incomplete:
   308	    verdict = "INCOMPLETE"
   309	    why = ("cells short of their %d pairs, or with a CI below the registered %.0f%% "
   310	           "coverage: %s. No verdict is read." % (PAIRS, 100 * MIN_COVERAGE,
   311	                                                  ", ".join(incomplete)))
   312	elif SESSION_VOID:
   313	    verdict = "RIG-VOID"
   314	    why = "the harness voided this session: %s. No verdict is read." % SESSION_VOID
   315	elif dirty:
   316	    verdict = "CONTROLS-NOT-CLEAN"
   317	    why = ("control cell(s) are not free of an arm asymmetry at T/2: %s. P1 is claimed "
   318	           "TCP-only and mixed-only; if the gRPC/large controls may be carrying the same "
   319	           "asymmetry, then NEITHER a reproduction NOR a null is readable off this rig. "
   320	           "There is no escalation: a noisy rig is fixed by a QUIETER RIG, not more pairs."
   321	           % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d], T/2=%d)"
   322	                       % (c, cell[c]["ctrl_state"], cell[c]["D"], cell[c]["ci"][0],
   323	                          cell[c]["ci"][1], round(cell[c]["T"] / 2))
   324	                       for c in dirty))
   325	elif "EFFECT" in m.values() and "INVERTED" in m.values():
   326	    verdict = "MIXED"
   327	    why = ("one direction shows the effect and the other INVERTS it -- a host x role "
   328	           "interaction this rig cannot decompose. Inconclusive for the question.")
   329	elif "EFFECT" in m.values():
   330	    verdict = "REPRODUCES"
   331	    why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: it shows "
   332	           "P1 CAN occur macOS<->macOS, so it is not waivable as 'Windows residue'. It "
   333	           "does NOT establish a platform-general cost, does NOT name the mechanism, "
   334	           "does NOT kill H1 (the code H1 accuses runs here too), and leaves macOS/APFS "
   335	           "and host x role explanations OPEN."
   336	           % ", ".join(c for c, s in m.items() if s == "EFFECT"))
   337	elif "INVERTED" in m.values():
   338	    verdict = "INVERTED"
   339	    why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank it as "
   340	           "'P1 absent'." % ", ".join(c for c, s in m.items() if s == "INVERTED"))
   341	elif all(s == "NONE" for s in m.values()):
   342	    verdict = "DOES-NOT-REPRODUCE"
   343	    why = ("both TCP-mixed cells EXCLUDE an effect of size T, and every control is clean "
   344	           "at T/2 -- a genuine equivalence result. Scoped to THIS pair: P1 did not "
   345	           "reproduce macOS<->macOS. That is CONSISTENT with 'the Windows peer is "
   346	           "required' but does NOT prove it -- it could equally be a property of these "
   347	           "two machines, their disks, or this macOS version.")
   348	else:
   349	    verdict = "UNCLEAR"
   350	    why = ("the CI spans the threshold in: %s. The rig could not resolve an effect of "
   351	           "size T either way -- this is NOT 'P1 vanishes'. Fix the rig, do not add pairs."
   352	           % ", ".join(c for c, s in m.items() if s == "UNCLEAR"))
   353	
   354	out = ["SESSION VERDICT: %s" % verdict, "", why, "",
   355	       "Per cell. T = min(srcinit_median/10, %d ms). Controls must be NONE at T/2, and B is"
   356	       % DELTA_REF,
   357	       "the arm bias they could NOT exclude: an EFFECT must clear T+B, a NULL must fit in T-B."]
   358	for c in sorted(cell):
   359	    x = cell[c]
   360	    if x["state"] == "INCOMPLETE":
   361	        out.append("  %-14s INCOMPLETE (%d pairs)" % (c, x["n"]))
   362	        continue
   363	    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] range=[%+5d,%+5d] "
   364	               "T=%3dms B=%3dms  ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
   365	               % (c, x["state"], x["ctrl_state"], x["D"], x["ci"][0], x["ci"][1],
   366	                  x["rng"][0], x["rng"][1], round(x["T"]), round(x.get("B", 0)),
   367	                  x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
   368	# A cell can be NONE (an effect of size T is excluded) and STILL carry a real, consistent
   369	# effect BELOW T -- e.g. 99 ms on a 1000 ms arm, one millisecond under the threshold, on
   370	# 7 of 8 pairs. That is not a contradiction and it does not change the verdict, but it
   371	# must not hide inside the word "none". Reported, never decided on.
   372	subthreshold = [c for c in sorted(cell)
   373	                if cell[c]["state"] == "NONE" and cell[c]["p"] < 0.05 and cell[c]["D"]]
   374	if subthreshold:
   375	    out += ["",
   376	            "NOTE -- a real but SUB-THRESHOLD effect is present in: %s."
   377	            % ", ".join("%s (D=%+dms, T=%dms, sign_p=%.3f)"
   378	                        % (c, cell[c]["D"], round(cell[c]["T"]), cell[c]["p"])
   379	                        for c in subthreshold),
   380	            "These cells are consistent in direction but smaller than the registered",
   381	            "threshold, so they are not a reproduction of P1. They are NOT nothing."]
   382	
   383	out += ["",
   384	        "A NULL (NONE) is judged on the full RANGE -- EVERY pair inside the bound -- not on",
   385	        "the median CI, which at n>8 would TRIM the outliers that contradict it. An EFFECT",
   386	        "uses the CI. That is why bimodality needs no special branch: it cannot hide from",
   387	        "the range.",
   388	        "",
   389	        "The bar/ratio columns are the project's ACCEPTANCE criterion. They are reported",
   390	        "and take NO part in this verdict, which is decided only by the paired CI against",
   391	        "T. sign_p is reported, not decided on. All runs are in summary.csv -- read them.",
   392	        "",
   393	        "Computed from the pre-registered rule. It declares nothing beyond it."]
   394	
   395	open(sess_p, "w").write("\n".join(out) + "\n")
   396	print("\n".join(out))

 succeeded in 0ms:
     1	# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
     2	
     3	**Status**: Pre-registered, **revision 9**. **NO DATA EXISTS YET.**
     4	
     5	> ## THE RULE IN ONE PARAGRAPH (rev 8 — D-2026-07-14-3, owner: "simplify")
     6	>
     7	> Per cell, take the **paired** ABBA differences, their median, and one **exact CI**.
     8	> Compare that CI against **one threshold** `T = min(10% of the source arm, 230 ms)`.
     9	> Four states, exhaustive by construction: **EFFECT** (CI clears +T), **INVERTED** (CI
    10	> clears −T), **NONE** (CI lies inside ±T — an effect of size T is *excluded*), **UNCLEAR**
    11	> (the CI spans a threshold). **Every control must be NONE at T/2, or no verdict about the
    12	> measurand is read at all** — not a reproduction, and not a null. The 1.10 bar is
    13	> reported and takes **no part** in this; the sign test is reported, not decided on.
    14	>
    15	> That is the whole rule. Seven review rounds found 80+ defects and **four of the last five
    16	> BLOCKERs were in the decision rule, not the measurement** — the complexity *was* the
    17	> defect. What pre-registration is actually for is kept: the question, the statistic and the
    18	> thresholds are fixed **before any data exists**, and the harness **computes** the verdict.
    19	
    20	> ## ⛔ CORRECTION THAT THIS DOCUMENT OWES ITS READER
    21	>
    22	> **Revisions 3, 4 and 5 of this document asserted that a fixed, equal `SETTLE_MS`
    23	> window precedes the fsync on both arms. THAT WAS NEVER TRUE.** The settle was
    24	> computed by an `awk` inside a command substitution whose quoting was wrong, so the
    25	> awk errored, `sleep` received an empty argument and failed, and the code discarded
    26	> its exit status. **The settle has never executed — not once, in any revision.**
    27	>
    28	> It was introduced in `24660ae` — **the commit that added it to fix the
    29	> free-writeback asymmetry that reverses sign with direction**, i.e. the artifact
    30	> judged capable of *manufacturing a one-directional P1 out of nothing*. **The fix for
    31	> that BLOCKER never ran.**
    32	>
    33	> Nothing is retracted, because **no data was ever taken**. It is fixed, it is
    34	> validated at preflight, and `SELFTEST=1` now proves it on a real tree. But this
    35	> document was wrong for three revisions, and it says so here rather than quietly
    36	> correcting the text below.
    37	
    38	Every revision of this document and its instrument has been reviewed before it
    39	measured anything, and **every review has found defects capable of a false claim**:
    40	
    41	- Round 1 (design, `f0343f4`): NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → **9/9
    42	  accepted** (`.review/results/macmac-prereg.gpt-verdict.md`).
    43	- Round 2 (instrument, `e1e351d`): NOT READY — 3 BLOCKER + 6 HIGH + 1 MEDIUM + 1
    44	  LOW → **11/11 accepted** (`.review/results/macmac-harness.gpt-verdict.md`).
    45	- Round 3 (reworked instrument, `24660ae`): **NOT READY** — codex: 5 BLOCKER + 6
    46	  HIGH + 1 MEDIUM → **12/12 accepted**; **grok** (second reviewer, D-2026-07-14-2)
    47	  independently **confirmed both blockers with its own measurements** and found **3
    48	  more** → **15/15 accepted**.
    49	  (`.review/results/macmac-harness-r2.{gpt,grok}-verdict.md`)
    50	- Round 4 (the round-3 rework, `cae2e0f`): **NOT SAFE TO RUN** — **grok**, which
    51	  **drove the engine to a clean `VANISHES` while every control carried the full
    52	  rig-W effect** → **9 findings, 9 accepted** (1 BLOCKER, 3 HIGH, 4 MEDIUM, 1 LOW).
    53	  (`.review/results/macmac-harness-r3.grok-verdict.md`)
    54	- Round 5 (the round-4 rework, `a9460ce`): **NOT READY / NOT SAFE TO RUN** — **codex**
    55	  (3 BLOCKER, 6 HIGH, 2 MEDIUM) **and grok**, which converged on the **same BLOCKER
    56	  independently**: the materiality bug, **for the third round running**, in a branch
    57	  neither had been shown. → **12 findings, 12 accepted.** Plus **the dead settle**
    58	  (above), which the review's finding exposed but did not itself find.
    59	  (`.review/results/macmac-harness-r5.verdict.md`)
    60	
    61	- Round 6 (the round-5 rework, `aebd50b`): **NOT READY** — **codex** (3 BLOCKER) **and
    62	  grok** (2 BLOCKER), converging *again* on both hunted classes: the **marginal bar still
    63	  substituted for paired magnitude** (a **1 ms** paired effect reported `REPRODUCES` at
    64	  n=16), a control at **D=+229** — *one millisecond* under the reference effect —
    65	  **certified as clean**, uncertified controls **blocked only the null and not a
    66	  reproduction**, and the settle repair was **still not provable** (a no-op `sleep` would
    67	  have passed while the log narrated "settle included"). → **13 findings, 13 accepted.**
    68	  (`.review/results/macmac-harness-r6.{codex,grok}.md`)
    69	- Round 7 (`1e03063`): **NOT READY** from both again — the drain fails open (a
    70	  `drained_*` value followed by a non-zero exit), rev 7's text contradicted itself, and
    71	  the settle could still be shadowed. → **the owner chose to SIMPLIFY the rule rather than
    72	  harden it again (D-2026-07-14-3).** This document is the result.
    73	  (`.review/results/macmac-harness-r7.{codex,grok}.md`)
    74	
    75	**Seven rounds. 80+ findings, all accepted, none rejected. Still no datum taken** — which is
    76	the only reason none of it became a retraction.
    77	
    78	**The rule below was rewritten in rev 8, and amended in 4–7 before that. That is
    79	legitimate only because NO DATA HAS EVER BEEN TAKEN** — before the first run is the only honest time
    80	to change a pre-registered rule, and every amendment is forced by a reviewer's
    81	finding, not by a number anyone has seen.
    82	
    83	**The pattern to distrust: every rework has introduced a defect of its own.** Round
    84	2's killer (the timer) was introduced by the round-1 rework. Round 4's BLOCKER (the
    85	control void) is the *same structural error* as round 3's — the equivalence margin
    86	was fixed for the **measurand** and left bar-tied for the **controls**, so a control
    87	carrying a full rig-W-sized effect was labelled "sub-bar" and escaped the void.
    88	**Fixing a bug in one place is not fixing its class.**
    89	
    90	**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).
    91	
    92	## What this experiment answers — and what it does NOT
    93	
    94	Revision 1 claimed this rig "discriminates H1 outright": *P1 reproduces
    95	macOS↔macOS ⇒ H1 dies, because H1 accuses the Windows accept branch.*
    96	
    97	**That inference is invalid, and the premise is false.** H1, verbatim in the
    98	parent, accuses **blit's own code paths** — the `SourceSockets` Dial/Accept
    99	branches, `InitiatorReceivePlaneRun.add_dialed_stream`, and the destination's
   100	synchronous dial-before-ACK at `transfer_session/mod.rs:3113`. **The word
   101	"Windows" appears nowhere in H1.** Windows is merely *who happens to be the
   102	accepting source* in P1's slow arm on rig W. The accused code runs on macOS too.
   103	So a Mac↔Mac reproduction is **consistent with H1**, not fatal to it — and the
   104	parent already warns that *"'consistent with H1' is not confirmation."*
   105	
   106	The bad framing was inherited from `docs/STATE.md` ("H1 accuses the *Windows*
   107	accept branch") and copied without checking H1's text. **That is a repo error and
   108	it is corrected wherever it appears.**
   109	
   110	The question, scoped to **this pair** (rev 2 said "a platform-general cost of the
   111	layout"; a rig with two machines cannot license that):
   112	
   113	> **Can P1 occur WITHOUT a Windows peer — on this pair of Macs?**
   114	
   115	| outcome | what it licenses — and its limit |
   116	|---|---|
   117	| **P1 REPRODUCES** | P1 **does not require a Windows peer** (on this pair), so it is **not** waivable as "Windows residue", and every code-level hypothesis strengthens. **Limits**: it does **not** establish a platform-*general* cost (two Macs are not "all platforms"); it does **not** name the mechanism; it does **not** kill H1 (the code H1 accuses runs here too); and it leaves **macOS/APFS** and **host×role** explanations fully **OPEN** — "not Windows-specific" is not "not platform-specific" (round-3 BLOCKER). |
   118	| **P1 does NOT reproduce (null)** | P1 **did not occur on this pair**. That is **consistent with** "the Windows peer is required" — but does **not prove it**: it could equally be a property of *these two machines*, their disks, or this macOS version. It does **not** confirm H1 either. |
   119	
   120	A null is only reportable at all if the rig could have **seen** an effect of size T —
   121	i.e. if the CI excludes one. Otherwise the verdict is `UNCLEAR`, which is **not** a null.
   122	
   123	**This run does NOT bear on an escape hatch for P1, because P1 HAS NONE**
   124	(round-3 BLOCKER; parent + codex r5 F1). D-2026-07-12-1 waives only a
   125	*cross-direction* miss for a cell that **already passes** invariance — P1 *is* the
   126	invariance failure. Rev 3 said this run bore on "whether P1 must be fixed in code
   127	**or could be accepted as platform residue**". The second half was never on the
   128	table: **P1 is fixed to ≤1.10, or the owner amends acceptance criterion 1.**
   129	What this rig changes is the *hypothesis space*, not the *obligation*.
   130	
   131	## Rig
   132	
   133	- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000.
   134	- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000.
   135	- **Build**: `f35702a`, **clean `+f35702a`** on all four binaries (the `.dirty`
   136	  form is rejected) — the cutover sha behind every P1 measurement (12b/12c,
   137	  q-baseline, pf-0). HEAD is **not** code-identical to it, so the pin is
   138	  deliberate, and the harness **refuses any other build**.
   139	- **Both Macs are bench ENDS.** The codex loop cannot run during a session; the
   140	  quiescence gate enforces it on **both** hosts and has fired correctly in
   141	  practice (it refuses while the owner's `codex` runs on nagatha).
   142	
   143	**Endpoint asymmetry does NOT simply "cancel" (round-1 HIGH).** Switching the
   144	initiator also **reassigns which machine runs the CLI and which runs the daemon**,
   145	and `q` is the faster Mac. Only arm-independent costs cancel; **host×role
   146	interactions do not.** Handled by *measuring both data directions and reporting
   147	them separately*, not by assertion — and no conclusion may lean on the
   148	cancellation being perfect.
   149	
   150	## Cells
   151	
   152	Grammar `<nq|qn>_<carrier>_<fixture>`: `nq_*` = data **nagatha→q**, `qn_*` = data
   153	**q→nagatha**. Arms — the only variable — are `srcinit` (source's CLI pushes) and
   154	`destinit` (dest's CLI pulls).
   155	
   156	    REGISTERED = nq_tcp_mixed,  qn_tcp_mixed     <- THE MEASURAND (P1's shape)
   157	                 nq_grpc_mixed, qn_grpc_mixed    <- carrier control (P1 is TCP-only)
   158	                 nq_tcp_large,  qn_tcp_large     <- fixture control (P1 is mixed-only)
   159	
   160	`RUNS=8`, ABBA-counterbalanced, pair-void. **All six cells must be present and
   161	complete.** A partial set that is merely *filtered* would let a one-cell run emit
   162	`VANISHES` while claiming both cells vanished (round-3 BLOCKER); missing cells are
   163	`INCOMPLETE` and no verdict is read.
   164	
   165	**Both directions are measured, but a reproduction is NOT required in both
   166	(round-1 HIGH).** P1's recorded signature on rig W is **one-directional**:
   167	`wm_tcp_mixed` FAILS while `mw_tcp_mixed` PASSES. Demanding failure in both
   168	directions here would rewrite the finding. So: **a reproduction in EITHER
   169	direction demonstrates the cost without a Windows peer.** Because the two
   170	directions differ in *which machine is the destination*, a one-directional result
   171	is explicitly **not** dismissible as "machine asymmetry" (rev 1 did exactly that,
   172	which would have let a real reproduction be waved away).
   173	
   174	## THE RULE (rev 8 — D-2026-07-14-3, owner: "simplify")
   175	
   176	Seven review rounds found 80+ defects, and **four of the last five BLOCKERs were in the
   177	DECISION RULE, not in the measurement**: a 1 ms effect reported as a reproduction; a
   178	control carrying 229 of 230 ms certified "clean"; a null printed while every control was
   179	dirty. The rule had ~10 outcomes, five thresholds, a certification tier and a precedence
   180	stack. **The complexity was the defect.** It is replaced by the smallest thing that still
   181	prevents post-hoc rationalization.
   182	
   183	**What pre-registration is actually for, and what is kept:** the question, the statistic
   184	and the thresholds are fixed **before any data exists**, and the **harness computes the
   185	verdict** — so no one can look at the numbers and then invent a favourable reading.
   186	
   187	### The statistic (paired, because the design is paired)
   188	
   189	    per ABBA slot i:  d_i = destinit_i − srcinit_i      (positive = destination slower)
   190	      D  = median(d_i)                                  low median, even n
   191	      CI = EXACT distribution-free order-statistic interval on the population median —
   192	           the narrowest whose coverage is >= 95%.
   193	           n=8  -> [min(d), max(d)]   coverage 99.22%
   194	           n=16 -> [d(4), d(13)]      coverage 97.87%
   195	
   196	No bootstrap (the old one claimed 95% and delivered 92.97%). No approximation.
   197	
   198	### The threshold (one)
   199	
   200	    T_pos = min(srcinit_med / 10,  Δ_ref)        Δ_ref = 230 ms, rig W's measured effect
   201	    T_neg = −min(srcinit_med / 11, Δ_ref)
   202	
   203	`src/10` is the project's own **1.10 invariance bar**; `Δ_ref` is the effect rig W
   204	actually measured. **The smaller of the two** — an effect must matter by *both* standards.
   205	The negative bound is `−src/11`, **not** `−src/10`, because the bar is symmetric in
   206	**ratio**, not in milliseconds.
   207	
   208	### The four cell states — mutually exclusive and exhaustive BY CONSTRUCTION
   209	
   210	They partition the CI's position relative to the thresholds. **There is no label here for
   211	a new case to walk past**, which is precisely what went wrong seven rounds running.
   212	
   213	| state | condition |
   214	|---|---|
   215	| **EFFECT** | `CI_lo >= T_pos + B` — destination-initiated is slower, by at least T |
   216	| **INVERTED** | `CI_hi <= T_neg − B` — source-initiated is slower, by at least T |
   217	| **NONE** | **the FULL RANGE** lies inside `(T_neg, T_pos)` — *every* pair, not just the median. An effect of size T is **EXCLUDED** (equivalence) |
   218	| **UNCLEAR** | anything else — the CI spans a threshold; the rig cannot answer |
   219	
   220	**A NULL IS JUDGED ON THE RANGE, AN EFFECT ON THE CI — and that asymmetry is the point
   221	(round-8, codex, BLOCKER).** The ≥95% CI is the *narrowest* valid interval, so at n>8 it
   222	**trims outliers**; a **bimodal** arm then yields a *narrow median CI* and a **false null**
   223	(codex drove `CI = [1,1]` from modes at ±110). **An equivalence claim must never be
   224	reachable by trimming away the very pairs that contradict it.** A *positive* claim may use
   225	the CI: pairs clearing T is evidence, and a few stragglers do not undo it.
   226	
   227	*This is also why bimodality needs no special branch — it cannot hide from the range. The
   228	previous rule hand-coded an `UNSTABLE` override for exactly this, and got it wrong.*
   229	
   230	### The controls are a PRECONDITION, at HALF the threshold
   231	
   232	**Every control must be `NONE` at `T/2`.** Half, because certifying a control with the
   233	very number that *defines* the effect is incoherent: it would let the gRPC control carry
   234	all but 1 ms of P1 while we call the rig clean (round 6 drove exactly that).
   235	
   236	**If any control fails, NO verdict about the measurand is read — not a reproduction, and
   237	not a null.** Uncertainty about a rig-wide confound is not evidence that the confound is
   238	absent, and P1's whole claim is that the effect is *specific* to TCP × mixed.
   239	
   240	**And "clean" is not "zero" (round-8, codex, BLOCKER).** A control sitting at `+49` with
   241	`T/2 = 50` certifies — but *that 49 ms of arm bias may be riding in the measurand too*, so a
   242	measurand effect of exactly `T` could be half real and half rig. The bias the controls **fail
   243	to exclude** is therefore carried into the measurand's thresholds:
   244	
   245	    B = max over clean controls of the largest |CI bound|
   246	    an EFFECT must clear   T + B     (the bias could be INFLATING it)
   247	    a NULL must fit inside T − B     (the bias could be MASKING an effect)
   248	
   249	If the controls are genuinely clean, `B` is a few ms and this barely moves. If they are
   250	marginal, it bites — which is the point.
   251	
   252	### The controls are CONTEMPORANEOUS with the measurands
   253	
   254	The schedule is **slot-major**: within slot *i*, **every** cell takes one ABBA pair, in a
   255	fixed registered order, before any cell takes slot *i+1*. All six cells therefore span the
   256	same wall-clock window.
   257	
   258	*(Round-8, codex, HIGH: both measurand cells used to run first and the controls afterwards
   259	— so **the controls certified a window they were never in**. A transient could hit the
   260	measurand and be gone before the controls ran, and they would pronounce the rig clean.)*
   261	
   262	### The session verdict
   263	
   264	1. **INCOMPLETE** — any registered cell short of its `RUNS` pairs, or with a CI below 95%
   265	   coverage. (Checked against the **data**; `meta.complete` is not believed.)
   266	2. **RIG-VOID** — the harness voided the session (end-load; see Gates).
   267	3. **CONTROLS-NOT-CLEAN** — any control is not `NONE` at `T/2`.
   268	4. **MIXED** — one direction `EFFECT`, the other `INVERTED`: a host×role interaction this
   269	   rig cannot decompose.
   270	5. **REPRODUCES** — `EFFECT` in **either** direction. *(P1's rig-W signature is
   271	   one-directional, so demanding both would rewrite the finding. A messy sibling is
   272	   reported, never substituted.)*
   273	6. **INVERTED** — a new finding; never banked as "P1 absent".
   274	7. **DOES-NOT-REPRODUCE** — **both** measurand cells `NONE`, with clean controls. A
   275	   genuine equivalence result.
   276	8. **UNCLEAR** — otherwise. **This is not a null.** There is no remedy in more pairs: fix the rig.
   277	
   278	### What is deliberately ABSENT, and why that is safe
   279	
   280	- **The 1.10 bar takes NO part in inference.** It is computed on the *marginal medians*,
   281	  reported in every row as the project's **acceptance** criterion, and never consulted.
   282	  The marginal and paired statistics can disagree in **direction and magnitude**, and
   283	  every attempt to let one stand in for the other produced a false verdict.
   284	- **The sign test is reported, not decided on.** At n=8 the CI already implies it
   285	  (`CI_lo >= T > 0` means *every* pair clears T), so making it a second gate only added
   286	  an interaction to get wrong. It is printed per cell.
   287	- **No `UNSTABLE` / `PARTIAL` / `BAR-FAIL-INCONSISTENT` / `UNDERPOWERED` branches, and no
   288	  precedence stack.** A bimodal arm **widens the CI**, and a wide CI lands in `UNCLEAR` —
   289	  which is exactly what those branches were hand-coding. Every run of every arm is still
   290	  printed in `summary.csv`, so bimodality remains visible to the reader.
   291	- **A real but SUB-THRESHOLD effect is reported, not buried.** A cell can be `NONE` and
   292	  still carry a consistent effect below T (e.g. 99 ms on a 1000 ms arm, on 7 of 8 pairs).
   293	  The verdict prints a NOTE naming it. It does not change the outcome — the threshold was
   294	  registered in advance — but it is **not nothing**, and it does not hide inside the word
   295	  "none".
   296	
   297	### There is NO escalation. `n` is EXACTLY 8.
   298	
   299	The old `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the
   300	**full range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR`
   301	rig, nor certify a marginal control; and if you already have an `EFFECT`, you do not need
   302	them.
   303	
   304	**And `n` must be EXACTLY 8, not "at least 8" (round-9, grok, BLOCKER).** At the registered
   305	n=8 the ≥95% interval **is** `[min, max]` — it *cannot* trim. At any larger n it starts
   306	trimming outliers, and a bimodal arm then yields a narrow median CI and a false verdict:
   307	grok drove a 16-pair CSV (three pairs at −500 trimmed away, thirteen at +200 left) straight
   308	to **`REPRODUCES`**. A cell carrying any count but the registered one is **`INCOMPLETE`**.
   309	*(I removed 16 from the registered list and left the completeness check saying `>=`. Fixed
   310	where I looked, not where it lived — again.)*
   311	
   312	**A noisy rig is fixed by a quieter rig, not by more pairs — and `UNCLEAR` says exactly
   313	that.** Removing it also removes its entire p-hacking guard surface (a "once" marker, a
   314	verdict check, a data-hash burn), none of which now has to be right.
   315	
   316	### The registered constants are PINNED IN CODE
   317	
   318	`DELTA_REF_MS`, `SETTLE_MS`, `LOAD_MAX`, `DRAIN_MBPS` and the rest are **literals** in
   319	both the harness and the engine. The harness **refuses to start** if one is merely
   320	*present* in the environment. *(They were once `${VAR:-default}`, and `DELTA_REF_MS=240`
   321	turned a void into a null — i.e. the rule could be retuned from the command line, after
   322	the data existed, in the direction of the answer you want. **That is not a
   323	pre-registration.**)* To change one: amend this document and put it back through review.
   324	
   325	### The guard test
   326	
   327	`scripts/otp12pf_mac_verdict_test.py`: **26 cases — every one a defect a reviewer actually
   328	drove out of a previous revision** — each **mutation-proven** (reverting that fix in a copy
   329	of the engine makes exactly that case fail: **9/9 mutations killed**), plus a 300-input
   330	fuzz over the measurand **and** the controls. It runs at preflight, cases *and* mutations;
   331	a vacuous guard refuses the run.
   332	
   333	## The instrument — what round 3 found, and what now guards it
   334	
   335	**THE TIMER WAS MEASURING FSYNC NOISE (round-3 BLOCKER; I introduced it in the
   336	rework that fixed round 2).** The transfer timer captured `time.monotonic()` in
   337	**two separate `python3 -c` processes** and subtracted them. On macOS that clock is
   338	**process-relative**. Measured on this rig: a **1000 ms sleep read as −1 ms on
   339	nagatha and 2 ms on q** — *negative*. Every `ms` row would have been ≈ `fsync_ms`
   340	alone, and the invariance ratio — **the entire measurand** — would have been
   341	computed on fsync noise, which can manufacture or mask a one-directional effect at
   342	will. The rig would have produced a clean session, 0 voided pairs, and a confident,
   343	meaningless verdict. **Grok measured the same defect independently** (a 500 ms sleep
   344	reading ~3 ms) before being shown codex's finding.
   345	
   346	The repo **already documented this trap** — `bench_otp12_zoey.sh:116` uses
   347	`time.time()` and says why — and I reintroduced it anyway. **The lesson is not "add
   348	a reviewer"; it is READ THE EXISTING HARNESSES BEFORE WRITING A NEW ONE.**
   349	
   350	Now: **one process times itself and spawns the client**, and — the structural fix —
   351	**preflight PROVES THE CLOCK on both hosts against a known 1000 ms sleep before any
   352	data is taken**, and a run whose timer returns a non-positive value **VOIDS** rather
   353	than entering the data as a "fast" row. The timing bug class cannot ship again
   354	without the instrument catching it on the rig.
   355	
   356	**Two defects that could have MANUFACTURED the result (round-2, still guarded):**
   357	
   358	1. **The durability check was fail-open.** `os.walk()` on a missing path returns
   359	   **0 files in 0 ms** — a missing tree reads as a *fast, successful flush*. The two
   360	   arms need **different** landed paths (blit uses rsync-style slash semantics: a
   361	   push to `/bench/RUNDIR/` lands at `RUNDIR/src_<W>`; a pull into `RUNDIR` lands
   362	   **directly in** `RUNDIR`). A wrong path would charge one arm **zero** durability
   363	   while the other paid full — the otp-2w bug that once manufactured P1.
   364	   **Guarded**: the fsync walk returns its **file count and byte sum**, and the pair
   365	   **VOIDs** unless both match the fixture exactly.
   366	2. **The free-writeback gap REVERSES SIGN WITH DIRECTION.** Between a client exiting
   367	   and the fsync starting, the OS writes back dirty pages **for free**, and that gap
   368	   is longer for whichever arm ran over ssh — and *which arm that is flips with the
   369	   data direction*. Since P1's signature is one-directional, this artifact could
   370	   produce a one-directional "reproduction" **out of nothing**.
   371	   **⛔ AND UNTIL REV 6, THE SETTLE NEVER RAN AT ALL (see the correction at the top).**
   372	   The `awk` computing its duration sat in a command substitution with the wrong
   373	   quoting, so it errored, `sleep` got an empty argument and failed, and the exit
   374	   status was discarded. Revisions 3–5 asserted this fix while it was dead — including
   375	   the revision that *introduced* it to close this very BLOCKER.
   376	
   377	   **Now, and only now: equalized, and BOUNDED — not "removed" (round-3 HIGH).** The
   378	   settle window is **equal on both arms** (250 ms, computed once at top level,
   379	   validated at startup, and its failure **VOIDS the pair**). The residual is the ssh
   380	   dispatch difference, **measured at ~15 ms** (median of 5, warm mux, recorded in the
   381	   manifest every session; a failed ssh now refuses rather than contributing a
   382	   flattering number). A pre-fsync delay of 10/20/200 ms produced **no measurable
   383	   change** in fsync time here (72–94 ms, no trend) — APFS fsync on this rig is
   384	   per-file-metadata bound, not writeback bound — so a 15 ms residual cannot plausibly
   385	   move it. **That is a bound from measurement, not a removal by construction, and this
   386	   document no longer claims otherwise.** `SELFTEST=1` walks a real tree and proves the
   387	   settle actually executed.
   388	
   389	## Gates — every one fails CLOSED, and every one is EXECUTED
   390	
   391	Round 2 found the round-1 "fixes" **had never been run** (`bash -n` is not an
   392	execution): the preflight **could not succeed at all** — `grep -c` exits 1 on no
   393	match, so a **clean** binary tripped the dirty-marker probe and died, and `norm_mac`
   394	used gawk's `strtonum()`, absent from stock macOS awk.
   395	
   396	`SELFTEST=1` **exercises the gates for real and takes no data.** It reports three
   397	states — `[OK]`, `[FIRED]` (a genuinely unmet condition: the gate *works*), and
   398	`[BROKEN]` (**the probe cannot answer at all**) — and **exits non-zero on any BROKEN**,
   399	because *a blind gate is precisely what fails open on the night*. It also **prints what
   400	it does NOT cover**.
   401	
   402	*(Round-5 codex, HIGH: the previous self-test labelled **every** nonzero result
   403	`[FIRED]` — including a probe that could not answer — exited zero, and claimed "every
   404	gate executes" while never touching drain, purge, daemon, fsync/settle, stale-daemon or
   405	end-load. **A self-test that overstates itself is the very fail-open it exists to
   406	hunt.**)*
   407	
   408	It has now earned itself three times: it caught `link_gate` **refusing a perfectly good
   409	link** (`arp -n <ip>` prints **one line per interface** — `q` holds entries for nagatha
   410	on en0, en1 *and* en8 — so the unfiltered MAC was a three-line string that could never
   411	equal one MAC; the gate now checks the entry **on the egress NIC**, the more correct
   412	question anyway); it caught **the dead settle**; and it caught **itself** breaking its
   413	own next gate (it ran `resolve_disk` in a subshell, which discarded the global that
   414	function exists to set, so the drain then had no device and blamed the harness).
   415	
   416	- **QUIESCENCE, BOTH MACS** — refuse if `codex`/`cargo`/`rustc` runs on **either**
   417	  Mac. `pgrep` rc≥2 is an **error**, not "quiet" (rev 3 could not tell them apart).
   418	- **TIME MACHINE, BOTH MACS** — refuse if a backup is running **or if autobackup is
   419	  merely ENABLED** (macOS repeats hourly; pf-0's fired 1 minute before its run). A
   420	  read error refuses.
   421	- **SPOTLIGHT, BOTH MACS** — `mds_stores` CPU, taken as the **MAX across samples**
   422	  (rev 3 took the last, so a late idle sample could overwrite an earlier busy one);
   423	  a failed `top` is an **error**, not 0%.
   424	- **LOAD** — `load1` on both Macs at start **and end**. A start `load1` above 3.0
   425	  refuses; an **end** `load1` above 3.0 **VOIDS THE SESSION** (`RIG-VOID`), because a
   426	  mid-session load spike is exactly the contamination the start gate exists to stop.
   427	  *(Round-4, grok: rev 4 moved the end-load logging before the verdict and its
   428	  comment claimed a session "can void on it" — but the code only **logged** it and
   429	  graded anyway. A doc claim the code did not honour: the very defect class this
   430	  review exists to kill.)*
   431	- **COLD CACHES** both ends every run (`sudo -n /usr/sbin/purge`); a failed purge
   432	  **VOIDS the pair**.
   433	- **DRAIN** — destination disk quiet before each window (`< 2 MB/s`, 3 consecutive
   434	  2 s samples). The device is **RESOLVED from the module path** through its APFS
   435	  physical store (grok: rev 3 hardcoded `disk0` and could certify a disk the data
   436	  never touched — and on APFS a *synthesized* disk can read idle while the physical
   437	  store saturates). A **non-numeric** `iostat` sample is an **error**, never "quiet"
   438	  (rev 3 read it as zero and **certified drainage**).
   439	- **DURABILITY** — the per-file `fsync` walk runs **on the destination host for both
   440	  arms**, is timed, and returns `NA` on a missing tree → the pair **VOIDS**.
   441	- **FIXTURES** verified by **count AND byte sum** on both ends before any timed run.
   442	- **PROVENANCE** — clean `+f35702a` on all four binaries (never `.dirty`); the
   443	  harness, the **verdict engine** and its **guard test** are all hashed into the
   444	  manifest; the instrument must be **committed and clean** in git (a modified
   445	  harness must not be able to claim the reviewed commit); `EXPECT_SHA` must equal
   446	  the **registered** build. `die` inside `$(...)` exits only the subshell, so the
   447	  hash functions now **return non-zero** and the caller dies (rev 3 wrote an **empty
   448	  hash** and called it provenance).
   449	- **DAEMON LIFECYCLE** — the pid comes from `$!` (not `pgrep | head -1`, which picks
   450	  the first of whatever is running); it must be **alive AND LISTENING** on the port;
   451	  teardown is **verified** (a failed probe is a failure, not "GONE") and a survivor
   452	  is recorded, not discarded.
   453	- **LINK** — peer ARP **on the egress NIC** resolves to the **peer's** MAC (a host
   454	  route on a directly-connected subnet installs a black hole that still reports the
   455	  right interface), and the route egresses the 10GbE NIC (macOS routes by service
   456	  order, so a 1GbE NIC can win and every run would go over gigabit).
   457	- **THE VERDICT ENGINE'S OWN GUARD TEST RUNS AT PREFLIGHT — cases AND mutations.**
   458	  If the decision rule fails its own cases, or if the proof that guards it turns out
   459	  to be **vacuous** (a mutation survives), **no data is taken**. *(Round-4, grok: rev
   460	  4's preflight ran only the cases, so a silently-reverted fix could still pass if
   461	  the cases happened to pass for another reason.)*
   462	
   463	## What this does NOT establish
   464	
   465	- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
   466	- **A reproduction does not exonerate macOS/APFS or host×role** — "not
   467	  Windows-specific" is **not** "not platform-specific" (round-3 BLOCKER).
   468	- **It does not bear on an escape hatch for P1 — P1 has none.**
   469	- **It cannot measure P2** — P2 is a converge bar against the OLD build and no old
   470	  pair is staged on the Macs.
   471	- **A null here cannot retract rig W.** P1 is measured and reproduced four times
   472	  there; this rig can only speak about the *pairing*.
   473	
   474	## The runs
   475	
   476	```sh
   477	SELFTEST=1 bash scripts/bench_otp12pf_mac.sh                 # exercise the gates, no data
   478	PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
   479	CLEARED_BY_REVIEW=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh   # the run
   480	```
   481	
   482	The harness **refuses a timed run** until the round-3 review is adjudicated
   483	(`CLEARED_BY_REVIEW`); the no-data modes always work. It **computes; it declares
   484	nothing** — the verdict is read off the rule above.

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_mac_verdict_test.py | sed -n '231,460p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/otp12pf_mac_verdict.py | sed -n '340,420p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   340	           "'P1 absent'." % ", ".join(c for c, s in m.items() if s == "INVERTED"))
   341	elif all(s == "NONE" for s in m.values()):
   342	    verdict = "DOES-NOT-REPRODUCE"
   343	    why = ("both TCP-mixed cells EXCLUDE an effect of size T, and every control is clean "
   344	           "at T/2 -- a genuine equivalence result. Scoped to THIS pair: P1 did not "
   345	           "reproduce macOS<->macOS. That is CONSISTENT with 'the Windows peer is "
   346	           "required' but does NOT prove it -- it could equally be a property of these "
   347	           "two machines, their disks, or this macOS version.")
   348	else:
   349	    verdict = "UNCLEAR"
   350	    why = ("the CI spans the threshold in: %s. The rig could not resolve an effect of "
   351	           "size T either way -- this is NOT 'P1 vanishes'. Fix the rig, do not add pairs."
   352	           % ", ".join(c for c, s in m.items() if s == "UNCLEAR"))
   353	
   354	out = ["SESSION VERDICT: %s" % verdict, "", why, "",
   355	       "Per cell. T = min(srcinit_median/10, %d ms). Controls must be NONE at T/2, and B is"
   356	       % DELTA_REF,
   357	       "the arm bias they could NOT exclude: an EFFECT must clear T+B, a NULL must fit in T-B."]
   358	for c in sorted(cell):
   359	    x = cell[c]
   360	    if x["state"] == "INCOMPLETE":
   361	        out.append("  %-14s INCOMPLETE (%d pairs)" % (c, x["n"]))
   362	        continue
   363	    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] range=[%+5d,%+5d] "
   364	               "T=%3dms B=%3dms  ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
   365	               % (c, x["state"], x["ctrl_state"], x["D"], x["ci"][0], x["ci"][1],
   366	                  x["rng"][0], x["rng"][1], round(x["T"]), round(x.get("B", 0)),
   367	                  x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
   368	# A cell can be NONE (an effect of size T is excluded) and STILL carry a real, consistent
   369	# effect BELOW T -- e.g. 99 ms on a 1000 ms arm, one millisecond under the threshold, on
   370	# 7 of 8 pairs. That is not a contradiction and it does not change the verdict, but it
   371	# must not hide inside the word "none". Reported, never decided on.
   372	subthreshold = [c for c in sorted(cell)
   373	                if cell[c]["state"] == "NONE" and cell[c]["p"] < 0.05 and cell[c]["D"]]
   374	if subthreshold:
   375	    out += ["",
   376	            "NOTE -- a real but SUB-THRESHOLD effect is present in: %s."
   377	            % ", ".join("%s (D=%+dms, T=%dms, sign_p=%.3f)"
   378	                        % (c, cell[c]["D"], round(cell[c]["T"]), cell[c]["p"])
   379	                        for c in subthreshold),
   380	            "These cells are consistent in direction but smaller than the registered",
   381	            "threshold, so they are not a reproduction of P1. They are NOT nothing."]
   382	
   383	out += ["",
   384	        "A NULL (NONE) is judged on the full RANGE -- EVERY pair inside the bound -- not on",
   385	        "the median CI, which at n>8 would TRIM the outliers that contradict it. An EFFECT",
   386	        "uses the CI. That is why bimodality needs no special branch: it cannot hide from",
   387	        "the range.",
   388	        "",
   389	        "The bar/ratio columns are the project's ACCEPTANCE criterion. They are reported",
   390	        "and take NO part in this verdict, which is decided only by the paired CI against",
   391	        "T. sign_p is reported, not decided on. All runs are in summary.csv -- read them.",
   392	        "",
   393	        "Computed from the pre-registered rule. It declares nothing beyond it."]
   394	
   395	open(sess_p, "w").write("\n".join(out) + "\n")
   396	print("\n".join(out))

 succeeded in 0ms:
   231	    ("codex r5: DELTA_REF_MS is PINNED -- the rule is not tunable from the environment",
   232	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   233	          env_extra={"DELTA_REF_MS": "240"}),
   234	     "ENGINE-REFUSED", "DOES-NOT-REPRODUCE"),
   235	]
   236	
   237	MUTATIONS = [
   238	    ("the control threshold is the SAME as the measurand's, not half (grok r6)",
   239	     ['    c_pos, c_neg = thresholds(x["src"], 0.5)',
   240	      '    c_pos, c_neg = thresholds(x["src"], 1.0)'],
   241	     "D=+229, ONE MS under"),
   242	
   243	    ("dirty controls block only the null, not a reproduction (codex r6)",
   244	     ["elif dirty:",
   245	      "elif dirty and not any(s == 'EFFECT' for s in m.values()):"],
   246	     "blocks a REPRODUCTION too"),
   247	
   248	    ("the inverting threshold is -src/10, not -src/11 (codex r2)",
   249	     ["            -min(s_med / 11.0, float(DELTA_REF)) * scale)",
   250	      "            -min(s_med / 10.0, float(DELTA_REF)) * scale)"],
   251	     "inverting threshold is -src/11"),
   252	
   253	    ("the threshold ignores DELTA_REF, so the bar alone forgives 240ms (codex r2)",
   254	     ["    return (min(s_med / 10.0, float(DELTA_REF)) * scale,",
   255	      "    return ((s_med / 10.0) * scale,"],
   256	     "bar alone would forgive"),
   257	
   258	    ("EFFECT is decided on the CI's MIDPOINT, not its lower bound (an outlier reproduces)",
   259	     ["    if ci_lo >= t_pos:", "    if (ci_lo + ci_hi) / 2.0 >= t_pos:"],
   260	     "one huge outlier"),
   261	
   262	    ("the NULL is not tightened by the control bias -- a masked effect reads as a null (grok r9)",
   263	     ["        if not (t_neg + B < x[\"rng\"][0] and x[\"rng\"][1] < t_pos - B):",
   264	      "        if False:"],
   265	     "null must also survive the TIGHTER bound"),
   266	
   267	    ("the control's residual bias is not carried into the measurand (codex r8)",
   268	     ['        B = max(B, abs(x["rng"][0]), abs(x["rng"][1]))', "        B = max(B, 0.0)"],
   269	     "exactly T is NOT a reproduction"),
   270	
   271	    ("the engine trusts meta.complete and never counts the pairs (grok r3)",
   272	     ['    if meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None:',
   273	      '    if meta.get(c, {}).get("complete") != "yes" or ci is None:'],
   274	     "SHORT cell (6 of 8 pairs)"),
   275	
   276	    ("a missing registered cell is filtered away (codex r2)",
   277	     ["for c in sorted(set(REGISTERED) | set(meta)):", "for c in sorted(meta):"],
   278	     "missing registered cell"),
   279	
   280	    ("a harness-detected session void is ignored (grok r3)",
   281	     ["elif SESSION_VOID:", "elif False:"],
   282	     "session void (end-load)"),
   283	
   284	    ("the registered DELTA_REF is taken from the environment again (codex r5)",
   285	     ['_env = os.environ.get("DELTA_REF_MS")', "_env = None"],
   286	     "DELTA_REF_MS is PINNED"),
   287	]
   288	
   289	
   290	def rule_unit_tests():
   291	    """The RULE itself, called directly -- because a session at n=8 cannot distinguish the
   292	    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
   293	    closed codex's round-8 blocker; judging a NULL on the RANGE is the SEMANTICS that keeps
   294	    it closed if a larger n is ever registered again, and it can only be tested here."""
   295	    import importlib.util
   296	    spec = importlib.util.spec_from_file_location("eng", DEFAULT_VERDICT)
   297	    # the engine runs on import (it is a script), so exercise classify() via a subprocess-free
   298	    # re-implementation guard: read the function out of the source and exec it in isolation.
   299	    src = open(DEFAULT_VERDICT).read()
   300	    start = src.index("def classify(")
   301	    end = src.index("\n\n", src.index("return \"UNCLEAR\"", start))
   302	    ns = {}
   303	    exec(src[start:end], ns)
   304	    classify = ns["classify"]
   305	    bad = 0
   306	    checks = [
   307	        # ci narrow (outliers trimmed), range wide: a bimodal arm. MUST NOT be NONE.
   308	        ("bimodal: CI=[1,1] but range=[-110,110], T=73", (1, 1, -110, 110, 73, -66), "UNCLEAR"),
   309	        ("clean: CI and range both inside T",            (2, 3, -4, 3, 73, -66),      "NONE"),
   310	        ("a real effect clears T",                       (80, 90, 75, 95, 73, -66),   "EFFECT"),
   311	        ("an inverted effect clears -T",                 (-90, -80, -95, -75, 73, -66), "INVERTED"),
   312	    ]
   313	    for name, args, want in checks:
   314	        got = classify(*args)
   315	        ok = got == want
   316	        print("  %-46s -> %-8s %s" % (name, got, "ok" if ok else "*** FAIL (want %s) ***" % want))
   317	        if not ok:
   318	            bad += 1
   319	
   320	    # THE IDENTITY THE WHOLE RULE LEANS ON: at the registered n=8, the >=95% order-statistic
   321	    # interval IS [min, max]. That is why nothing can be trimmed -- not a null, not B. If
   322	    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
   323	    # must refuse that n (it does).
   324	    ns2 = {}
   325	    src2 = open(DEFAULT_VERDICT).read()
   326	    st = src2.index("def median_ci(")
   327	    exec(src2[st:src2.index("\n\n", src2.index("return best", st))],
   328	         {"comb": __import__("math").comb, "MIN_COVERAGE": 0.95}, ns2)
   329	    import random as _r
   330	    rr = _r.Random(9)
   331	    for _ in range(200):
   332	        d = [rr.randint(-500, 500) for _ in range(8)]
   333	        lo, hi, cov = ns2["median_ci"](d)
   334	        if (lo, hi) != (min(d), max(d)) or cov < 0.95:
   335	            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
   336	            bad += 1
   337	            break
   338	    else:
   339	        print("  %-46s -> %-8s ok" % ("n=8: the >=95% CI IS [min,max] (200 draws)", "identity"))
   340	    return bad
   341	
   342	
   343	def run_cases():
   344	    bad = []
   345	    for name, kw, must_be, must_not in CASES:
   346	        got = session(**kw)
   347	        ok = not (must_be and got != must_be) and not (must_not and got == must_not)
   348	        print("%-66s -> %-20s %s" % (name[:66], got, "ok" if ok else "*** FAIL ***"))
   349	        if not ok:
   350	            bad.append(name)
   351	            print("      expected %s / must not be %s" % (must_be, must_not))
   352	    return bad
   353	
   354	
   355	def fuzz(n=300):
   356	    """No input may land outside the registered outcomes. The CONTROLS are fuzzed too --
   357	    pinning them clean once left every dirty-control path unexercised, and that is
   358	    exactly where a BLOCKER was hiding."""
   359	    rng = random.Random(4242)
   360	    bad = 0
   361	    for _ in range(n):
   362	        got = session(measurand_d=[rng.randint(-600, 600) for _ in range(8)],
   363	                      src=rng.choice([600, 1000, 2000, 2500, 5000]),
   364	                      control_d=[rng.randint(-300, 300) for _ in range(8)],
   365	                      control_src=rng.choice([600, 1000, 2500, 5000]))
   366	        if got not in OUTCOMES:
   367	            print("*** UNREGISTERED OUTCOME %r" % got)
   368	            bad += 1
   369	    print("fuzz: %d/%d inputs produced a registered outcome (measurand AND controls)"
   370	          % (n - bad, n))
   371	    return bad
   372	
   373	
   374	def mutate():
   375	    src = open(DEFAULT_VERDICT).read()
   376	    bad = 0
   377	    for name, subs, key in MUTATIONS:
   378	        body = src
   379	        for i in range(0, len(subs), 2):
   380	            old, new = subs[i], subs[i + 1]
   381	            if old not in body:     # the engine drifted: the proof is STALE, not passing
   382	                print("*** STALE MUTATION (target not found): %s" % name)
   383	                bad += 1
   384	                body = None
   385	                break
   386	            body = body.replace(old, new, 1)
   387	        if body is None:
   388	            continue
   389	        tmp = tempfile.mkdtemp()
   390	        path = os.path.join(tmp, "mutant.py")
   391	        open(path, "w").write(body)
   392	        case = next(c for c in CASES if key in c[0])
   393	        os.environ["VERDICT_PY"] = path
   394	        got = session(**case[1])
   395	        del os.environ["VERDICT_PY"]
   396	        # KILLED == the case now FAILS, by its OWN contract. Checking instead for a
   397	        # verdict named here let a mutant "survive" by failing a different way.
   398	        _, _, must_be, must_not = case
   399	        killed = (must_be and got != must_be) or (must_not and got == must_not)
   400	        print("%-66s -> %-20s %s" % (name[:66], got,
   401	                                     "KILLED" if killed else "*** SURVIVED — VACUOUS ***"))
   402	        if not killed:
   403	            bad += 1
   404	    return bad
   405	
   406	
   407	if __name__ == "__main__":
   408	    if "--mutations" in sys.argv:
   409	        print("Reverting each fix in a copy of the engine; the named case must then FAIL.\n")
   410	        n = mutate()
   411	        print("\n%d/%d mutations killed" % (len(MUTATIONS) - n, len(MUTATIONS)))
   412	        sys.exit(1 if n else 0)
   413	    print("The RULE, called directly (a session at n=8 cannot separate CI from RANGE):")
   414	    unit = rule_unit_tests()
   415	    print()
   416	    fails = run_cases()
   417	    print()
   418	    z = fuzz()
   419	    print("\n%d/%d cases passed" % (len(CASES) - len(fails), len(CASES)))
   420	    sys.exit(1 if (fails or z or unit) else 0)

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_mac_verdict_test.py | sed -n '1,230p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env python3
     2	"""Guard test for otp12pf_mac_verdict.py (rev 8, D-2026-07-14-3).
     3	
     4	    python3 scripts/otp12pf_mac_verdict_test.py             # the cases
     5	    python3 scripts/otp12pf_mac_verdict_test.py --mutations # prove they are not vacuous
     6	
     7	EVERY case is a defect a reviewer actually drove out of a previous revision of this
     8	engine, across seven review rounds. The rule has now been REWRITTEN and simplified;
     9	these cases are the price of that rewrite. Each one asserts that the SIMPLER rule still
    10	refuses the wrong answer the COMPLEX rule once gave.
    11	
    12	A mutation reverts one fix in a copy of the engine; the named case must then FAIL.
    13	"""
    14	import csv, os, random, subprocess, sys, tempfile
    15	
    16	HERE = os.path.dirname(os.path.abspath(__file__))
    17	DEFAULT_VERDICT = os.path.join(HERE, "otp12pf_mac_verdict.py")
    18	CONTROLS = ("nq_grpc_mixed", "qn_grpc_mixed", "nq_tcp_large", "qn_tcp_large")
    19	MEASURANDS = ("nq_tcp_mixed", "qn_tcp_mixed")
    20	REGISTERED = MEASURANDS + CONTROLS
    21	OUTCOMES = {"INCOMPLETE", "RIG-VOID", "CONTROLS-NOT-CLEAN", "MIXED", "REPRODUCES",
    22	            "INVERTED", "DOES-NOT-REPRODUCE", "UNCLEAR"}
    23	
    24	
    25	def engine():
    26	    """Resolved per call: the mutation harness repoints it, and a cached path would
    27	    silently test the UNMUTATED engine and report a kill it never made."""
    28	    return os.environ.get("VERDICT_PY", DEFAULT_VERDICT)
    29	
    30	
    31	def session(measurand_d, src=2000, control_d=None, control_src=1000, drop_cells=(),
    32	            per_cell=None, void_reason="", pairs=8, env_extra=None):
    33	    """`src` may be an int OR a per-pair list. The bar is computed on the MARGINAL
    34	    medians and the CI on the PAIRED differences, and the two only disagree when the
    35	    source arm varies -- a constant-only helper made that whole class of bug
    36	    unguardable by construction."""
    37	    control_d = [5] * pairs if control_d is None else control_d
    38	    per_cell = per_cell or {}
    39	    tmp = tempfile.mkdtemp()
    40	    runs, meta = os.path.join(tmp, "runs.csv"), os.path.join(tmp, "meta.csv")
    41	    present = [c for c in REGISTERED if c not in drop_cells]
    42	    with open(runs, "w") as f:
    43	        w = csv.writer(f)
    44	        w.writerow("cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,"
    45	                   "exit,drain,cold,valid".split(","))
    46	        for cell in present:
    47	            if cell in per_cell:
    48	                d, s = per_cell[cell]
    49	            elif cell in MEASURANDS:
    50	                d, s = measurand_d, src
    51	            else:
    52	                d, s = control_d, control_src
    53	            srcs = s if isinstance(s, list) else [s] * len(d)
    54	            for i, (di, si) in enumerate(zip(d, srcs), 1):
    55	                w.writerow([cell, "srcinit", "x", "h", i, si, 0, 250, 1, 1, 0,
    56	                            "drained_1x2s", "cold", "yes"])
    57	                w.writerow([cell, "destinit", "x", "h", i, si + di, 0, 250, 1, 1, 0,
    58	                            "drained_1x2s", "cold", "yes"])
    59	    with open(meta, "w") as f:
    60	        f.write("cell,pairs_attempted,complete\n")
    61	        for cell in present:
    62	            # `complete=yes` is asserted even when a cell is SHORT: the engine must not
    63	            # believe it (a 1-pair CSV once graded as a full cell at 0% CI coverage).
    64	            f.write("%s,%d,yes\n" % (cell, pairs))
    65	    env = dict(os.environ, VERDICT_CELLS=",".join(MEASURANDS),
    66	               CONTROL_CELLS=",".join(CONTROLS), REGISTERED_CELLS=",".join(REGISTERED),
    67	               REQUIRED_PAIRS="8", SESSION_VOID_REASON=void_reason)
    68	    env.pop("DELTA_REF_MS", None)                      # PINNED in the engine
    69	    env.update(env_extra or {})
    70	    out = subprocess.run([sys.executable, engine(), runs, meta,
    71	                          os.path.join(tmp, "s.csv"), os.path.join(tmp, "p.csv"),
    72	                          os.path.join(tmp, "v.csv"), os.path.join(tmp, "sv.txt")],
    73	                         env=env, capture_output=True, text=True)
    74	    if out.returncode != 0 and "REFUSING" in (out.stderr or ""):
    75	        return "ENGINE-REFUSED"          # a deliberate refusal is the engine WORKING
    76	    if out.returncode != 0:
    77	        return "ENGINE-CRASH: " + (out.stderr.strip().splitlines() or ["?"])[-1]
    78	    return out.stdout.splitlines()[0].split(":", 1)[1].strip()
    79	
    80	
    81	# (name, kwargs, must_be, must_not_be)
    82	CASES = [
    83	    # --- a real effect must never read as nothing --------------------------------
    84	    ("codex r1: a 190ms effect on 7/8 pairs is not a null",
    85	     dict(measurand_d=[0, 180, 180, 190, 190, 200, 200, 200], src=2000),
    86	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
    87	
    88	    ("codex r2: a rig-W-sized effect (230ms) in EVERY pair, on a slow 2500ms arm",
    89	     dict(measurand_d=[230] * 8, src=2500, control_d=[0] * 8),
    90	     "REPRODUCES", "DOES-NOT-REPRODUCE"),
    91	
    92	    ("codex r2: an effect the 10% bar alone would forgive (240ms @ 2500)",
    93	     dict(measurand_d=[-100, -50, 0, 50, 100, 200, 220, 240], src=2500),
    94	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
    95	
    96	    ("codex r2: the inverting threshold is -src/11, not -src/10 (CI [-190,0] @ 2000)",
    97	     dict(measurand_d=[-190, -190, 0, 0, 0, 0, 0, 0], src=2000),
    98	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
    99	
   100	    # --- an artifact must never read as an effect --------------------------------
   101	    ("codex r2: 7 positive + 1 negative is not a reproduction",
   102	     dict(measurand_d=[-20, 300, 310, 320, 330, 340, 350, 360], src=1000),
   103	     "UNCLEAR", "REPRODUCES"),
   104	
   105	    ("codex r5: a 1ms paired effect is not a reproduction, whatever the medians do",
   106	     dict(measurand_d=[1] * 13 + [-4500] * 3,
   107	          src=[1000] * 7 + [1200] * 6 + [5000] * 3,
   108	          control_d=[5] * 16, control_src=1000, pairs=16),
   109	     None, "REPRODUCES"),
   110	
   111	    ("codex r6: nor when the marginal bar fails in the MATCHING direction",
   112	     dict(measurand_d=[400] * 3 + [1] * 13, src=[1000] * 8 + [1200] * 8,
   113	          control_d=[5] * 16, control_src=1000, pairs=16),
   114	     None, "REPRODUCES"),
   115	
   116	    ("one huge outlier must not manufacture a reproduction (the CI's LOWER bound decides)",
   117	     dict(measurand_d=[10, 10, 10, 10, 10, 10, 10, 800], src=1000),
   118	     "UNCLEAR", "REPRODUCES"),
   119	
   120	    ("grok r9: a LONG cell (16 pairs) is INCOMPLETE — a CI at n>8 TRIMS the pairs that contradict it",
   121	     dict(measurand_d=[-500] * 3 + [200] * 13, src=1000, control_d=[0] * 16),
   122	     "INCOMPLETE", "REPRODUCES"),
   123	
   124	    ("a SHORT cell (6 of 8 pairs) claiming complete=yes is INCOMPLETE",
   125	     dict(measurand_d=[-4, -2, -1, 0, 1, 2], src=2000),
   126	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   127	
   128	    # --- the controls are a precondition -----------------------------------------
   129	    ("grok r2: a bar-FAIL control whose CI crosses zero blocks every verdict",
   130	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   131	          control_d=[-100, -50, 300, 320, 340, 350, 360, 380], control_src=1000),
   132	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   133	
   134	    ("grok r4: a Delta_ref-sized control effect blocks every verdict",
   135	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   136	          control_d=[230] * 8, control_src=2500),
   137	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   138	
   139	    ("codex r5: ...and so does one with a single zero pair (CI [0,230])",
   140	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   141	          control_d=[0] + [230] * 7, control_src=2500),
   142	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   143	
   144	    ("grok r5: ...and a non-directional one (CI [-10,230])",
   145	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   146	          control_d=[230] * 7 + [-10], control_src=2500),
   147	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   148	
   149	    ("grok r6: ...and one at D=+229, ONE MS under the reference effect",
   150	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   151	          control_d=[229] * 8, control_src=2500),
   152	     "CONTROLS-NOT-CLEAN", "DOES-NOT-REPRODUCE"),
   153	
   154	    ("codex r6: a dirty control blocks a REPRODUCTION too, not just a null",
   155	     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000,
   156	          control_d=[0] + [230] * 7, control_src=2500),
   157	     "CONTROLS-NOT-CLEAN", "REPRODUCES"),
   158	
   159	    # ...but a GOOD rig must still be able to ANSWER. An instrument that can never
   160	    # conclude is also broken (grok r6: the "dead zone").
   161	    ("a clean rig with a tiny host x role control asymmetry still answers",
   162	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   163	          control_d=[5] * 8, control_src=1000),
   164	     "DOES-NOT-REPRODUCE", "CONTROLS-NOT-CLEAN"),
   165	
   166	    # --- the rig must be able to say each of the things it can say ----------------
   167	    ("a real, bar-breaking slowdown reproduces",
   168	     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000),
   169	     "REPRODUCES", None),
   170	
   171	    ("an exact 10% effect is reportable on a bias-free rig (it was once unreachable)",
   172	     dict(measurand_d=[100] * 8, src=1000, control_d=[0] * 8),
   173	     "REPRODUCES", None),
   174	
   175	    # codex r8, BLOCKER: a control at +5 is "clean", but that 5ms of arm bias may be
   176	    # riding in the measurand too -- so an effect of EXACTLY T could be (T-5) real plus
   177	    # 5 rig. It must not be banked as a reproduction. B carries the bias the controls
   178	    # could not exclude into the measurand's threshold.
   179	    ("codex r8: an effect of exactly T is NOT a reproduction when the controls carry bias",
   180	     dict(measurand_d=[100] * 8, src=1000, control_d=[5] * 8),
   181	     "UNCLEAR", "REPRODUCES"),
   182	
   183	    ("grok r9: a null must also survive the TIGHTER bound (bias could be MASKING an effect)",
   184	     dict(measurand_d=[60] * 8, src=1000, control_d=[49] * 8),
   185	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   186	
   187	    ("codex r8: ...and the same effect IS one once the rig is bias-free",
   188	     dict(measurand_d=[105] * 8, src=1000, control_d=[5] * 8),
   189	     "REPRODUCES", "UNCLEAR"),
   190	
   191	    ("source-initiated slower is INVERTED, never 'P1 absent'",
   192	     dict(measurand_d=[-300, -310, -320, -330, -340, -350, -360, -370], src=1000),
   193	     "INVERTED", None),
   194	
   195	    ("one direction reproduces, the other inverts -> MIXED",
   196	     dict(measurand_d=[0] * 8, src=1000,
   197	          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
   198	                    "qn_tcp_mixed": ([-300, -310, -320, -330, -340, -350, -360, -370], 1000)}),
   199	     "MIXED", "REPRODUCES"),
   200	
   201	    ("a clean one-direction reproduction is NOT masked by a noisy sibling",
   202	     dict(measurand_d=[0] * 8, src=1000,
   203	          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
   204	                    "qn_tcp_mixed": ([-20, 300, 310, 320, 330, 340, 350, 360], 1000)}),
   205	     "REPRODUCES", "UNCLEAR"),
   206	
   207	    ("codex r8: a bimodal arm cannot hide from the RANGE (a null is judged on every pair)",
   208	     dict(measurand_d=[-110, 0, -110, 110, 110, 0, -110, 0], src=730,
   209	          control_d=[0] * 8),
   210	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   211	
   212	    ("a null the rig could not have SEEN is UNCLEAR, not a null",
   213	     dict(measurand_d=[-400, -300, -100, 0, 0, 100, 300, 400], src=2000),
   214	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   215	
   216	    # --- integrity ---------------------------------------------------------------
   217	    ("a missing registered cell is INCOMPLETE, never filtered away",
   218	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   219	          drop_cells=("qn_tcp_mixed",)),
   220	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   221	
   222	    ("grok r3: n=1 with complete=yes must not grade at 0% CI coverage",
   223	     dict(measurand_d=[0], src=2000, control_d=[5], control_src=1000),
   224	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   225	
   226	    ("grok r3: a harness-detected session void (end-load) refuses a verdict",
   227	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   228	          void_reason="end-load on q is 9.1 (> 3.0)"),
   229	     "RIG-VOID", "DOES-NOT-REPRODUCE"),
   230	

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md | sed -n '1,210p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
     2	
     3	**Status**: Pre-registered, **revision 9**. **NO DATA EXISTS YET.**
     4	
     5	> ## THE RULE IN ONE PARAGRAPH (rev 8 — D-2026-07-14-3, owner: "simplify")
     6	>
     7	> Per cell, take the **paired** ABBA differences, their median, and one **exact CI**.
     8	> Compare that CI against **one threshold** `T = min(10% of the source arm, 230 ms)`.
     9	> Four states, exhaustive by construction: **EFFECT** (CI clears +T), **INVERTED** (CI
    10	> clears −T), **NONE** (CI lies inside ±T — an effect of size T is *excluded*), **UNCLEAR**
    11	> (the CI spans a threshold). **Every control must be NONE at T/2, or no verdict about the
    12	> measurand is read at all** — not a reproduction, and not a null. The 1.10 bar is
    13	> reported and takes **no part** in this; the sign test is reported, not decided on.
    14	>
    15	> That is the whole rule. Seven review rounds found 80+ defects and **four of the last five
    16	> BLOCKERs were in the decision rule, not the measurement** — the complexity *was* the
    17	> defect. What pre-registration is actually for is kept: the question, the statistic and the
    18	> thresholds are fixed **before any data exists**, and the harness **computes** the verdict.
    19	
    20	> ## ⛔ CORRECTION THAT THIS DOCUMENT OWES ITS READER
    21	>
    22	> **Revisions 3, 4 and 5 of this document asserted that a fixed, equal `SETTLE_MS`
    23	> window precedes the fsync on both arms. THAT WAS NEVER TRUE.** The settle was
    24	> computed by an `awk` inside a command substitution whose quoting was wrong, so the
    25	> awk errored, `sleep` received an empty argument and failed, and the code discarded
    26	> its exit status. **The settle has never executed — not once, in any revision.**
    27	>
    28	> It was introduced in `24660ae` — **the commit that added it to fix the
    29	> free-writeback asymmetry that reverses sign with direction**, i.e. the artifact
    30	> judged capable of *manufacturing a one-directional P1 out of nothing*. **The fix for
    31	> that BLOCKER never ran.**
    32	>
    33	> Nothing is retracted, because **no data was ever taken**. It is fixed, it is
    34	> validated at preflight, and `SELFTEST=1` now proves it on a real tree. But this
    35	> document was wrong for three revisions, and it says so here rather than quietly
    36	> correcting the text below.
    37	
    38	Every revision of this document and its instrument has been reviewed before it
    39	measured anything, and **every review has found defects capable of a false claim**:
    40	
    41	- Round 1 (design, `f0343f4`): NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → **9/9
    42	  accepted** (`.review/results/macmac-prereg.gpt-verdict.md`).
    43	- Round 2 (instrument, `e1e351d`): NOT READY — 3 BLOCKER + 6 HIGH + 1 MEDIUM + 1
    44	  LOW → **11/11 accepted** (`.review/results/macmac-harness.gpt-verdict.md`).
    45	- Round 3 (reworked instrument, `24660ae`): **NOT READY** — codex: 5 BLOCKER + 6
    46	  HIGH + 1 MEDIUM → **12/12 accepted**; **grok** (second reviewer, D-2026-07-14-2)
    47	  independently **confirmed both blockers with its own measurements** and found **3
    48	  more** → **15/15 accepted**.
    49	  (`.review/results/macmac-harness-r2.{gpt,grok}-verdict.md`)
    50	- Round 4 (the round-3 rework, `cae2e0f`): **NOT SAFE TO RUN** — **grok**, which
    51	  **drove the engine to a clean `VANISHES` while every control carried the full
    52	  rig-W effect** → **9 findings, 9 accepted** (1 BLOCKER, 3 HIGH, 4 MEDIUM, 1 LOW).
    53	  (`.review/results/macmac-harness-r3.grok-verdict.md`)
    54	- Round 5 (the round-4 rework, `a9460ce`): **NOT READY / NOT SAFE TO RUN** — **codex**
    55	  (3 BLOCKER, 6 HIGH, 2 MEDIUM) **and grok**, which converged on the **same BLOCKER
    56	  independently**: the materiality bug, **for the third round running**, in a branch
    57	  neither had been shown. → **12 findings, 12 accepted.** Plus **the dead settle**
    58	  (above), which the review's finding exposed but did not itself find.
    59	  (`.review/results/macmac-harness-r5.verdict.md`)
    60	
    61	- Round 6 (the round-5 rework, `aebd50b`): **NOT READY** — **codex** (3 BLOCKER) **and
    62	  grok** (2 BLOCKER), converging *again* on both hunted classes: the **marginal bar still
    63	  substituted for paired magnitude** (a **1 ms** paired effect reported `REPRODUCES` at
    64	  n=16), a control at **D=+229** — *one millisecond* under the reference effect —
    65	  **certified as clean**, uncertified controls **blocked only the null and not a
    66	  reproduction**, and the settle repair was **still not provable** (a no-op `sleep` would
    67	  have passed while the log narrated "settle included"). → **13 findings, 13 accepted.**
    68	  (`.review/results/macmac-harness-r6.{codex,grok}.md`)
    69	- Round 7 (`1e03063`): **NOT READY** from both again — the drain fails open (a
    70	  `drained_*` value followed by a non-zero exit), rev 7's text contradicted itself, and
    71	  the settle could still be shadowed. → **the owner chose to SIMPLIFY the rule rather than
    72	  harden it again (D-2026-07-14-3).** This document is the result.
    73	  (`.review/results/macmac-harness-r7.{codex,grok}.md`)
    74	
    75	**Seven rounds. 80+ findings, all accepted, none rejected. Still no datum taken** — which is
    76	the only reason none of it became a retraction.
    77	
    78	**The rule below was rewritten in rev 8, and amended in 4–7 before that. That is
    79	legitimate only because NO DATA HAS EVER BEEN TAKEN** — before the first run is the only honest time
    80	to change a pre-registered rule, and every amendment is forced by a reviewer's
    81	finding, not by a number anyone has seen.
    82	
    83	**The pattern to distrust: every rework has introduced a defect of its own.** Round
    84	2's killer (the timer) was introduced by the round-1 rework. Round 4's BLOCKER (the
    85	control void) is the *same structural error* as round 3's — the equivalence margin
    86	was fixed for the **measurand** and left bar-tied for the **controls**, so a control
    87	carrying a full rig-W-sized effect was labelled "sub-bar" and escaped the void.
    88	**Fixing a bug in one place is not fixing its class.**
    89	
    90	**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).
    91	
    92	## What this experiment answers — and what it does NOT
    93	
    94	Revision 1 claimed this rig "discriminates H1 outright": *P1 reproduces
    95	macOS↔macOS ⇒ H1 dies, because H1 accuses the Windows accept branch.*
    96	
    97	**That inference is invalid, and the premise is false.** H1, verbatim in the
    98	parent, accuses **blit's own code paths** — the `SourceSockets` Dial/Accept
    99	branches, `InitiatorReceivePlaneRun.add_dialed_stream`, and the destination's
   100	synchronous dial-before-ACK at `transfer_session/mod.rs:3113`. **The word
   101	"Windows" appears nowhere in H1.** Windows is merely *who happens to be the
   102	accepting source* in P1's slow arm on rig W. The accused code runs on macOS too.
   103	So a Mac↔Mac reproduction is **consistent with H1**, not fatal to it — and the
   104	parent already warns that *"'consistent with H1' is not confirmation."*
   105	
   106	The bad framing was inherited from `docs/STATE.md` ("H1 accuses the *Windows*
   107	accept branch") and copied without checking H1's text. **That is a repo error and
   108	it is corrected wherever it appears.**
   109	
   110	The question, scoped to **this pair** (rev 2 said "a platform-general cost of the
   111	layout"; a rig with two machines cannot license that):
   112	
   113	> **Can P1 occur WITHOUT a Windows peer — on this pair of Macs?**
   114	
   115	| outcome | what it licenses — and its limit |
   116	|---|---|
   117	| **P1 REPRODUCES** | P1 **does not require a Windows peer** (on this pair), so it is **not** waivable as "Windows residue", and every code-level hypothesis strengthens. **Limits**: it does **not** establish a platform-*general* cost (two Macs are not "all platforms"); it does **not** name the mechanism; it does **not** kill H1 (the code H1 accuses runs here too); and it leaves **macOS/APFS** and **host×role** explanations fully **OPEN** — "not Windows-specific" is not "not platform-specific" (round-3 BLOCKER). |
   118	| **P1 does NOT reproduce (null)** | P1 **did not occur on this pair**. That is **consistent with** "the Windows peer is required" — but does **not prove it**: it could equally be a property of *these two machines*, their disks, or this macOS version. It does **not** confirm H1 either. |
   119	
   120	A null is only reportable at all if the rig could have **seen** an effect of size T —
   121	i.e. if the CI excludes one. Otherwise the verdict is `UNCLEAR`, which is **not** a null.
   122	
   123	**This run does NOT bear on an escape hatch for P1, because P1 HAS NONE**
   124	(round-3 BLOCKER; parent + codex r5 F1). D-2026-07-12-1 waives only a
   125	*cross-direction* miss for a cell that **already passes** invariance — P1 *is* the
   126	invariance failure. Rev 3 said this run bore on "whether P1 must be fixed in code
   127	**or could be accepted as platform residue**". The second half was never on the
   128	table: **P1 is fixed to ≤1.10, or the owner amends acceptance criterion 1.**
   129	What this rig changes is the *hypothesis space*, not the *obligation*.
   130	
   131	## Rig
   132	
   133	- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000.
   134	- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000.
   135	- **Build**: `f35702a`, **clean `+f35702a`** on all four binaries (the `.dirty`
   136	  form is rejected) — the cutover sha behind every P1 measurement (12b/12c,
   137	  q-baseline, pf-0). HEAD is **not** code-identical to it, so the pin is
   138	  deliberate, and the harness **refuses any other build**.
   139	- **Both Macs are bench ENDS.** The codex loop cannot run during a session; the
   140	  quiescence gate enforces it on **both** hosts and has fired correctly in
   141	  practice (it refuses while the owner's `codex` runs on nagatha).
   142	
   143	**Endpoint asymmetry does NOT simply "cancel" (round-1 HIGH).** Switching the
   144	initiator also **reassigns which machine runs the CLI and which runs the daemon**,
   145	and `q` is the faster Mac. Only arm-independent costs cancel; **host×role
   146	interactions do not.** Handled by *measuring both data directions and reporting
   147	them separately*, not by assertion — and no conclusion may lean on the
   148	cancellation being perfect.
   149	
   150	## Cells
   151	
   152	Grammar `<nq|qn>_<carrier>_<fixture>`: `nq_*` = data **nagatha→q**, `qn_*` = data
   153	**q→nagatha**. Arms — the only variable — are `srcinit` (source's CLI pushes) and
   154	`destinit` (dest's CLI pulls).
   155	
   156	    REGISTERED = nq_tcp_mixed,  qn_tcp_mixed     <- THE MEASURAND (P1's shape)
   157	                 nq_grpc_mixed, qn_grpc_mixed    <- carrier control (P1 is TCP-only)
   158	                 nq_tcp_large,  qn_tcp_large     <- fixture control (P1 is mixed-only)
   159	
   160	`RUNS=8`, ABBA-counterbalanced, pair-void. **All six cells must be present and
   161	complete.** A partial set that is merely *filtered* would let a one-cell run emit
   162	`VANISHES` while claiming both cells vanished (round-3 BLOCKER); missing cells are
   163	`INCOMPLETE` and no verdict is read.
   164	
   165	**Both directions are measured, but a reproduction is NOT required in both
   166	(round-1 HIGH).** P1's recorded signature on rig W is **one-directional**:
   167	`wm_tcp_mixed` FAILS while `mw_tcp_mixed` PASSES. Demanding failure in both
   168	directions here would rewrite the finding. So: **a reproduction in EITHER
   169	direction demonstrates the cost without a Windows peer.** Because the two
   170	directions differ in *which machine is the destination*, a one-directional result
   171	is explicitly **not** dismissible as "machine asymmetry" (rev 1 did exactly that,
   172	which would have let a real reproduction be waved away).
   173	
   174	## THE RULE (rev 8 — D-2026-07-14-3, owner: "simplify")
   175	
   176	Seven review rounds found 80+ defects, and **four of the last five BLOCKERs were in the
   177	DECISION RULE, not in the measurement**: a 1 ms effect reported as a reproduction; a
   178	control carrying 229 of 230 ms certified "clean"; a null printed while every control was
   179	dirty. The rule had ~10 outcomes, five thresholds, a certification tier and a precedence
   180	stack. **The complexity was the defect.** It is replaced by the smallest thing that still
   181	prevents post-hoc rationalization.
   182	
   183	**What pre-registration is actually for, and what is kept:** the question, the statistic
   184	and the thresholds are fixed **before any data exists**, and the **harness computes the
   185	verdict** — so no one can look at the numbers and then invent a favourable reading.
   186	
   187	### The statistic (paired, because the design is paired)
   188	
   189	    per ABBA slot i:  d_i = destinit_i − srcinit_i      (positive = destination slower)
   190	      D  = median(d_i)                                  low median, even n
   191	      CI = EXACT distribution-free order-statistic interval on the population median —
   192	           the narrowest whose coverage is >= 95%.
   193	           n=8  -> [min(d), max(d)]   coverage 99.22%
   194	           n=16 -> [d(4), d(13)]      coverage 97.87%
   195	
   196	No bootstrap (the old one claimed 95% and delivered 92.97%). No approximation.
   197	
   198	### The threshold (one)
   199	
   200	    T_pos = min(srcinit_med / 10,  Δ_ref)        Δ_ref = 230 ms, rig W's measured effect
   201	    T_neg = −min(srcinit_med / 11, Δ_ref)
   202	
   203	`src/10` is the project's own **1.10 invariance bar**; `Δ_ref` is the effect rig W
   204	actually measured. **The smaller of the two** — an effect must matter by *both* standards.
   205	The negative bound is `−src/11`, **not** `−src/10`, because the bar is symmetric in
   206	**ratio**, not in milliseconds.
   207	
   208	### The four cell states — mutually exclusive and exhaustive BY CONSTRUCTION
   209	
   210	They partition the CI's position relative to the thresholds. **There is no label here for

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '281,560p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '841,1120p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   841	  return 1
   842	}
   843	
   844	run_all_cells() {
   845	  local slot i cell sh dh w flag max=$(( 2 * RUNS )) n=${#CELL_TABLE[@]}
   846	  for (( i = 0; i < n; i++ )); do CELL_VALID[$i]=0; CELL_ATTEMPTS[$i]=0; done
   847	  for (( slot = 1; slot <= RUNS; slot++ )); do
   848	    log "=== SLOT $slot / $RUNS (every cell takes one pair before any cell takes the next) ==="
   849	    for (( i = 0; i < n; i++ )); do
   850	      read -r cell sh dh w flag <<<"${CELL_TABLE[$i]}"
   851	      # a voided pair is retried IN PLACE, so the cell stays in step with its siblings
   852	      while (( ${CELL_ATTEMPTS[$i]:-0} < max )); do
   853	        if run_one_pair "$i" "$cell" "$sh" "$dh" "$w" "${flag:-}" "$slot"; then break; fi
   854	      done
   855	    done
   856	  done
   857	  for (( i = 0; i < n; i++ )); do
   858	    read -r cell sh dh w flag <<<"${CELL_TABLE[$i]}"
   859	    if (( ${CELL_VALID[$i]:-0} < RUNS )); then
   860	      echo "$cell,${CELL_ATTEMPTS[$i]},no" >> "$META"
   861	      log "  $cell INCOMPLETE: ${CELL_VALID[$i]}/$RUNS valid pairs"
   862	    else
   863	      echo "$cell,${CELL_ATTEMPTS[$i]},yes" >> "$META"
   864	    fi
   865	  done
   866	}
   867	
   868	SESSION_VOID_REASON=""
   869	# The end-load is a CONDITION OF THE SESSION, not a log line. A mid-session load
   870	# spike is exactly the contamination the start gate exists to prevent, and until now
   871	# it could not void anything: the code logged `load1 (end)` and computed a verdict
   872	# anyway, while the comment claimed a session "can void on it" (round-3 grok, HIGH —
   873	# a doc claim the code did not honour, which is the defect class this whole review
   874	# exists to kill).
   875	end_load_gate() {
   876	  local h l ok
   877	  for h in n q; do
   878	    l="$(load1 "$h")" || l=""
   879	    if [[ ! "$l" =~ ^[0-9]+\.?[0-9]*$ ]]; then
   880	      SESSION_VOID_REASON="end-load on $(hname "$h") could not be read (got '$l') — a session whose end conditions are unknown cannot be graded"
   881	      return
   882	    fi
   883	    ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   884	    if [[ "$ok" != 1 ]]; then
   885	      SESSION_VOID_REASON="end-load on $(hname "$h") is $l (> $LOAD_MAX) — the machine was NOT quiet at the end of the session, so a contaminant may have entered the timed windows"
   886	      return
   887	    fi
   888	  done
   889	}
   890	
   891	compute_verdicts() {
   892	  DELTA_REF_MS="$DELTA_REF_MS" VERDICT_CELLS="$VERDICT_CELLS" \
   893	  CONTROL_CELLS="$CONTROL_CELLS" REGISTERED_CELLS="$REGISTERED_CELLS" \
   894	  REQUIRED_PAIRS="$RUNS" SESSION_VOID_REASON="$SESSION_VOID_REASON" \
   895	  python3 "$VERDICT_PY" \
   896	    "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/paired.csv" \
   897	    "$OUT_DIR/verdicts.csv" "$OUT_DIR/session_verdict.txt"
   898	}
   899	
   900	# =============================================================================
   901	# SELFTEST — exercise every gate for real, take NO data.
   902	#
   903	# This exists because round 1's "fixes" were never executed: I ran `bash -n` and
   904	# shipped a preflight that COULD NOT SUCCEED (grep -c's exit 1, gawk's strtonum).
   905	# A syntax check is not an execution.
   906	# =============================================================================
   907	SELFTEST_FIRED=0; SELFTEST_BROKEN=0
   908	# A gate can end in three states, and the old self-test collapsed two of them
   909	# (round-5 codex, HIGH: "every nonzero result — including a BROKEN probe — is labeled
   910	# [FIRED], and the self-test exits zero"). That is the same fail-open it exists to
   911	# hunt, committed by the hunter:
   912	#
   913	#   [OK]     the probe answered and the condition holds.
   914	#   [FIRED]  the probe answered and the condition is genuinely UNMET (codex is
   915	#            running, Time Machine is on). The gate WORKS. Not a self-test failure.
   916	#   [BROKEN] the probe could not answer at all. THE GATE IS BLIND, and the self-test
   917	#            FAILS (exit 1) — a blind gate is exactly what fails open on the night.
   918	#
   919	# The two are told apart by the refusal text: every "cannot answer" die() in this file
   920	# says so in the words below, and every genuine-condition die() does not.
   921	# A REPORTER, never a gate: it must always return 0, or `set -e` aborts the sweep at
   922	# the first refusal and the remaining gates go untested (which is exactly what it did
   923	# the first time it ran — the self-test could not even test itself).
   924	gate_probe() {
   925	  local label="$1"; shift
   926	  local err rc=0
   927	  err="$( { "$@"; } 2>&1 )" || rc=1
   928	  if (( rc == 0 )); then
   929	    log "  [OK]     $label — answers, and the condition holds"
   930	  elif grep -q 'PROBE-BLIND' <<<"$err"; then
   931	    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 ))
   932	    log "  [BROKEN] $label — THE PROBE COULD NOT ANSWER. A blind gate fails open on the night."
   933	  else
   934	    SELFTEST_FIRED=$(( SELFTEST_FIRED + 1 ))
   935	    log "  [FIRED]  $label — the gate REFUSED a genuinely unmet condition. It works."
   936	  fi
   937	  # Never hide what the gate said — including its own evidence on success.
   938	  [[ -n "$err" ]] && sed 's/^/           | /' <<<"$err" | tee -a "$OUT_DIR/bench.log" >&2
   939	  return 0
   940	}
   941	
   942	# The fsync/settle path, exercised for real on a throwaway tree. It is the durability
   943	# measurement AND the equal-settle window — the two things that once manufactured P1 —
   944	# and the self-test never touched them.
   945	selftest_fsync() {
   946	  local h="$1" d ms files bytes settled
   947	  d="$(hmod "$h")/selftest_${SESSION_TAG}"
   948	  hrun "$h" "rm -rf '$d' && mkdir -p '$d' && printf 'aaaa' > '$d/a' && printf 'bb' > '$d/b'" \
   949	    || { log "  [BROKEN] fsync/settle — cannot stage a probe tree"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); return 1; }
   950	  read -r ms files bytes settled <<<"$(fsync_tree "$h" "$d")"
   951	  hrun "$h" "rm -rf '$d'" >/dev/null 2>&1 || true
   952	  if [[ "$ms" == NA || "$files" != 2 || "$bytes" != 6 ]]; then
   953	    log "  [BROKEN] fsync/settle — walk returned ms=$ms files=$files bytes=$bytes, want 2 files / 6 bytes"
   954	    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
   955	  fi
   956	  # THE SETTLE MUST BE PROVED, NOT NARRATED (round-6, both reviewers). The old check
   957	  # counted files and then LOGGED "settle included" — which is a sentence, not an
   958	  # assertion. It would have passed with the settle stone dead, which is precisely how
   959	  # the settle stayed dead for three revisions.
   960	  if ! settle_ok "$settled"; then
   961	    log "  [BROKEN] fsync/settle — THE SETTLE DID NOT ELAPSE: measured ${settled}ms, want >= ${SETTLE_MS}ms"
   962	    SELFTEST_BROKEN=$(( SELFTEST_BROKEN + 1 )); return 1
   963	  fi
   964	  log "  [OK]     fsync/settle — 2 files/6 bytes walked in ${ms}ms; settle MEASURED at ${settled}ms (>= ${SETTLE_MS}ms), counts VERIFIED"
   965	}
   966	
   967	selftest() {
   968	  local h
   969	  log "SELFTEST — exercising the gates for real. No transfer, NO DATA."
   970	  log "instrument: harness=$HARNESS_SHA256"
   971	  log "--- the verdict engine's own guard test (incl. mutation proof) ---"
   972	  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
   973	    || die "the verdict guard test FAILS (see $OUT_DIR/verdict-guard-test.txt)"
   974	  log "  $(grep -E '^[0-9]+/[0-9]+ cases passed' "$OUT_DIR/verdict-guard-test.txt")"
   975	  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
   976	    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt)"
   977	  log "  $(grep -E '^[0-9]+/[0-9]+ mutations killed' "$OUT_DIR/verdict-mutations.txt") — every reverted fix is caught"
   978	  for h in n q; do
   979	    log "--- $(hname "$h") ---"
   980	    # NOT through gate_probe: it runs its argument in a SUBSHELL, and this function's
   981	    # whole job is to SET a global. (resolve_disk had the identical bug — the self-test
   982	    # was breaking its own next gate. Same class, and it caught itself this time.)
   983	    if resolve_python "$h"; then log "  [OK]     python3       (absolute, not PATH-resolved)"
   984	    else log "  [BROKEN] python3       — could not resolve an absolute interpreter"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); fi
   985	    gate_probe "timer         (the measurand's clock)" timer_gate "$h"
   986	    gate_probe "quiescence    (codex/cargo/rustc)"     quiescence_gate "$h"
   987	    gate_probe "time machine  (running OR enabled)"    timemachine_gate "$h"
   988	    gate_probe "spotlight     (mds_stores CPU)"        spotlight_gate "$h"
   989	    gate_probe "load  start   (load1 <= $LOAD_MAX)"      load_gate "$h"
   990	    gate_probe "link          (ARP on the egress NIC + 10GbE route)" link_gate "$h"
   991	    # NOT through gate_probe: it runs its argument in a SUBSHELL (so a `die` cannot
   992	    # abort the sweep), and resolve_disk's whole job is to SET a global. Called there,
   993	    # the assignment was discarded and the drain loop below then had no device and
   994	    # reported DRAIN-ERROR — the self-test was breaking its own next gate and blaming
   995	    # the harness.
   996	    if resolve_disk "$h"; then log "  [OK]     drain device  (resolved via the APFS physical store)"
   997	    else log "  [BROKEN] drain device  — could not resolve the physical disk"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)); fi
   998	    # The paths the old self-test claimed and did not run (round-5 codex, HIGH):
   999	    gate_probe "purge         (sudo -n, or every run reads WARM)" hrun "$h" "sudo -n /usr/sbin/purge"
  1000	    case "$(pgrep_state "$h" blit-daemon)" in
  1001	      NONE)    log "  [OK]     stale daemon  (rc-aware probe: none running)" ;;
  1002	      RUNNING) log "  [FIRED]  stale daemon  (one IS running — the gate would refuse)"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
  1003	      *)       log "  [BROKEN] stale daemon  — the probe could not answer"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
  1004	    esac
  1005	    # DRAIN-TIMEOUT is a genuinely busy disk (the gate WORKING); DRAIN-ERROR is a blind
  1006	    # probe. Scoring them the same made the classification untrustworthy (grok r6, F7).
  1007	    local dr; dr="$(drain_host "$h")"
  1008	    case "$dr" in
  1009	      drained*)      log "  [OK]     drain loop    ($dr)" ;;
  1010	      DRAIN-TIMEOUT) log "  [FIRED]  drain loop    — the disk is genuinely busy; the gate would void the pair"; SELFTEST_FIRED=$((SELFTEST_FIRED+1)) ;;
  1011	      *)             log "  [BROKEN] drain loop    — the probe could not answer ('$dr')"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1)) ;;
  1012	    esac
  1013	    selftest_fsync "$h"
  1014	    log "  [--]     mac parse (no gawk strtonum): $(hmac "$h") -> $(hmac "$h" | norm_mac)"
  1015	  done
  1016	  SESSION_VOID_REASON=""; end_load_gate
  1017	  if [[ -z "$SESSION_VOID_REASON" ]]; then
  1018	    log "  [OK]     end-load gate (both Macs under $LOAD_MAX; it CAN void a session)"
  1019	  elif [[ "$SESSION_VOID_REASON" == *"could not be read"* ]]; then
  1020	    # An UNREADABLE end-load is a blind probe, not a busy machine (grok r6, F7).
  1021	    log "  [BROKEN] end-load gate — $SESSION_VOID_REASON"; SELFTEST_BROKEN=$((SELFTEST_BROKEN+1))
  1022	  else
  1023	    log "  [FIRED]  end-load gate — $SESSION_VOID_REASON"; SELFTEST_FIRED=$((SELFTEST_FIRED+1))
  1024	  fi
  1025	  measure_ssh_rtt
  1026	  log ""
  1027	  log "SELFTEST: $SELFTEST_FIRED gate(s) refused a genuinely unmet condition; $SELFTEST_BROKEN blind."
  1028	  log "NOT exercised here (they need a real transfer): daemon start/lsof/teardown, the"
  1029	  log "smoke transfer, the ABBA pair loop, pair-voiding, and the manifest. PREFLIGHT_ONLY=1"
  1030	  log "covers the manifest and the build-provenance gates. This self-test does NOT claim"
  1031	  log "to run every gate — the previous one did, and it was not true."
  1032	  log "THIS IS NOT CLEARANCE TO TAKE DATA. The review is."
  1033	  if (( SELFTEST_BROKEN > 0 )); then
  1034	    log "SELFTEST FAILED: $SELFTEST_BROKEN gate(s) are BLIND."
  1035	    exit 1
  1036	  fi
  1037	  log "SELFTEST PASSED: every gate exercised here can answer."
  1038	}
  1039	
  1040	main() {
  1041	  if [[ "$SELFTEST" == 1 ]]; then
  1042	    EXPECT_SHA="${EXPECT_SHA:-$REGISTERED_BUILD}"
  1043	    HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
  1044	    HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
  1045	    selftest
  1046	    exit 0
  1047	  fi
  1048	  preflight
  1049	  write_manifest
  1050	  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
  1051	    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
  1052	    exit 0
  1053	  fi
  1054	  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
  1055	  echo "cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid" > "$CSV"
  1056	  echo "cell,pairs_attempted,complete" > "$META"
  1057	  daemon_start n; daemon_start q
  1058	  smoke n; smoke q
  1059	
  1060	  run_all_cells
  1061	
  1062	  # End-load BEFORE the verdict is computed, and it can VOID the session.
  1063	  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
  1064	  end_load_gate
  1065	  if [[ -n "$SESSION_VOID_REASON" ]]; then
  1066	    log "ERROR: SESSION VOID — $SESSION_VOID_REASON"
  1067	    touch "$OUT_DIR/SESSION-VOID"
  1068	  fi
  1069	  compute_verdicts
  1070	  log "=== SUMMARY (cold, drained, durable; ABBA) ==="
  1071	  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
  1072	  log "=== PAIRED STATS (the rule is graded on these) ==="
  1073	  column -t -s, "$OUT_DIR/paired.csv" | tee -a "$OUT_DIR/bench.log"
  1074	  log "=== SESSION VERDICT (computed by the harness from the PRE-REGISTERED rule) ==="
  1075	  cat "$OUT_DIR/session_verdict.txt" | tee -a "$OUT_DIR/bench.log"
  1076	  log "runs: $CSV"
  1077	}
  1078	
  1079	# EXPECT_SHA is required for anything that touches the rig's binaries; SELFTEST
  1080	# supplies the registered default so the gates can be exercised without ceremony.
  1081	if [[ "$SELFTEST" != 1 ]]; then
  1082	  EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a)}"
  1083	  HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
  1084	  HARNESS_SHA256="$(shasum -a 256 "$SELF" | cut -d' ' -f1)"
  1085	fi
  1086	main "$@"

 succeeded in 0ms:
   281	  raw="$(hrun "$h" "c=\$(grep -c -a -- '+$EXPECT_SHA' '$p'); rc=\$?
   282	d=\$(grep -c -a -- '+$EXPECT_SHA.dirty' '$p'); rd=\$?
   283	if [ \$rc -ge 2 ] || [ \$rd -ge 2 ]; then echo 'E:ERR:E'; else echo \"E:\$c:\$d:E\"; fi" \
   284	    | nocr | sed -n 's/.*E:\([0-9]*\):\([0-9]*\):E.*/\1 \2/p' | head -1)" || return 1
   285	  [[ -n "$raw" ]] || return 1
   286	  read -r hit dirty <<<"$raw"
   287	  [[ "$hit" =~ ^[0-9]+$ && "$dirty" =~ ^[0-9]+$ ]] || return 1
   288	  [[ "$hit" -gt 0 && "$dirty" -eq 0 ]]
   289	}
   290	sha256_of() {      # returns non-zero on a short/empty hash; the CALLER must `|| die`
   291	  local h="$1" p="$2" v
   292	  v="$(hrun "$h" "shasum -a 256 '$p' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f')" || return 1
   293	  [[ ${#v} -eq 64 ]] || return 1
   294	  echo "$v"
   295	}
   296	
   297	# --- gates: every one fails CLOSED --------------------------------------------
   298	# Stock macOS awk has no strtonum() (that is gawk). Hand-rolled hex, so the ARP
   299	# comparison actually runs instead of erroring out.
   300	norm_mac() {
   301	  awk -F: '
   302	    function hex(s,   i,c,d,v) {
   303	      v = 0; s = tolower(s)
   304	      for (i = 1; i <= length(s); i++) {
   305	        c = substr(s, i, 1); d = index("0123456789abcdef", c) - 1
   306	        if (d < 0) return -1
   307	        v = v * 16 + d
   308	      }
   309	      return v
   310	    }
   311	    {
   312	      if (NF != 6) { print ""; next }
   313	      out = ""; ok = 1
   314	      for (i = 1; i <= NF; i++) {
   315	        v = hex($i)
   316	        if (v < 0 || v > 255) { ok = 0; break }
   317	        out = out sprintf("%s%02x", (i > 1 ? ":" : ""), v)
   318	      }
   319	      print (ok ? out : "")
   320	    }'
   321	}
   322	
   323	# THE ONLY process probe in this harness. pgrep: 0 = found, 1 = none, >=2 = ERROR.
   324	# Echoes RUNNING | NONE | BROKEN. A probe that cannot answer must NEVER answer "fine",
   325	# and there must be exactly ONE of these -- round 5 found the fail-open surviving in a
   326	# duplicate site precisely because there were two.
   327	pgrep_state() {
   328	  local h="$1" pat="$2" raw
   329	  raw="$(hrun "$h" "pgrep -x '$pat' >/dev/null 2>&1; rc=\$?
   330	if [ \$rc -eq 0 ]; then echo 'G:RUNNING:G'
   331	elif [ \$rc -eq 1 ]; then echo 'G:NONE:G'
   332	else echo 'G:BROKEN:G'; fi" | nocr | sed -n 's/.*G:\([A-Z]*\):G.*/\1/p' | head -1)" || raw=""
   333	  case "$raw" in
   334	    RUNNING|NONE|BROKEN) echo "$raw" ;;
   335	    *)                   echo BROKEN ;;   # no sentinel back == a broken probe
   336	  esac
   337	}
   338	
   339	quiescence_gate() {
   340	  local h="$1" p busy=""
   341	  for p in codex cargo rustc; do
   342	    case "$(pgrep_state "$h" "$p")" in
   343	      RUNNING) busy="$busy $p" ;;
   344	      NONE)    : ;;
   345	      *)       die_blind "$(hname "$h"): the quiescence probe for '$p' BROKE — refusing (a gate that cannot answer must not answer 'fine')" ;;
   346	    esac
   347	  done
   348	  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running:$busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
   349	}
   350	
   351	timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   352	  local h="$1" running auto
   353	  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
   354	  [[ "$running" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
   355	  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
   356	  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1" | nocr | tr -cd '0-9')" || auto=""
   357	  [[ "$auto" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
   358	  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
   359	}
   360	
   361	spotlight_gate() {
   362	  local h="$1" cpu
   363	  # The MAX across samples, not the last: a late idle sample could overwrite an
   364	  # earlier busy one. NR==0 (top produced nothing) is an ERROR, not 0% CPU.
   365	  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null \
   366	    | awk '/^mds_stores/{ if (\$2+0 > m) m = \$2+0 } END{ if (NR == 0) print \"ERR\"; else printf \"%d\", m+0 }'" | nocr)" || cpu="ERR"
   367	  [[ "$cpu" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
   368	  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
   369	}
   370	
   371	load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
   372	load_gate() {
   373	  local h="$1" l ok
   374	  l="$(load1 "$h")" || l=""
   375	  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die_blind "$(hname "$h"): cannot read load1 (got '$l') — refusing"
   376	  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   377	  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
   378	}
   379	
   380	link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
   381	  local h="$1" o peer_ip want got route_nic nic
   382	  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"; nic="$(hnic "$h")"
   383	  [[ -n "$want" ]] || die_blind "$(hname "$o"): its configured MAC does not parse — refusing"
   384	  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
   385	    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
   386	  # The ARP entry ON THE NIC THE TRAFFIC WILL EGRESS. `arp -n <ip>` prints one line
   387	  # PER INTERFACE that has an entry — q holds entries for nagatha on en0, en1 AND
   388	  # en8 — so an unfiltered $4 yields a MULTI-LINE string that can never equal a
   389	  # single MAC. (Measured: this refused a perfectly good link. It is also the more
   390	  # correct check: a stale entry on the 1GbE NIC is irrelevant to the 10GbE path.)
   391	  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk -v nic='$nic' '\$5 == \"on\" && \$6 == nic {print \$4}' | head -1" | nocr | norm_mac)"
   392	  [[ -n "$got" ]] || die "$(hname "$h"): no ARP entry for $peer_ip ON $nic — the 10GbE path has not resolved the peer"
   393	  [[ "$got" == "$want" ]] \
   394	    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
   395	  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
   396	  [[ "$route_nic" == "$(hnic "$h")" ]] \
   397	    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
   398	}
   399	
   400	# --- the drain device: RESOLVED, never hardcoded (grok) ------------------------
   401	# `iostat disk0` can certify a disk the data never touched. Worse, on APFS the
   402	# volume lives on a SYNTHESIZED disk whose stats may be empty while the physical
   403	# store is saturated — a false "quiet". Resolve the module path to its PHYSICAL
   404	# store and verify iostat actually reports it.
   405	N_DISK=""; Q_DISK=""
   406	hdisk() { if [[ "$1" == n ]]; then echo "$N_DISK"; else echo "$Q_DISK"; fi; }
   407	resolve_disk() {
   408	  local h="$1" p dev
   409	  p="$(hmod "$h")"
   410	  # A FAILED `diskutil` MUST NOT silently fall back to the synthesized disk (round-5
   411	  # codex, HIGH). On APFS the volume lives on a synthesized container whose iostat
   412	  # counters can read IDLE while the physical store is saturated — so falling back to
   413	  # it is not a harmless default, it is a FALSE QUIET that certifies drainage on a
   414	  # device the data never touched. If the volume is APFS, the physical-store lookup
   415	  # must SUCCEED or the gate refuses.
   416	  dev="$(hrun "$h" "d=\$(df '$p' 2>/dev/null | awk 'NR==2{print \$1}' | sed 's|^/dev/||')
   417	[ -n \"\$d\" ] || { echo 'D:NO-DF:D'; exit 0; }
   418	info=\$(diskutil info \"\$d\" 2>/dev/null) || { echo 'D:NO-DISKUTIL:D'; exit 0; }
   419	[ -n \"\$info\" ] || { echo 'D:EMPTY-DISKUTIL:D'; exit 0; }
   420	if echo \"\$info\" | grep -q 'APFS'; then
   421	  ps=\$(echo \"\$info\" | awk -F: '/APFS Physical Store/{gsub(/[ \t]/, \"\", \$2); print \$2}' | head -1)
   422	  [ -n \"\$ps\" ] || { echo 'D:APFS-NO-STORE:D'; exit 0; }
   423	  d=\"\$ps\"
   424	fi
   425	echo \"D:\$(echo \"\$d\" | sed -E 's/s[0-9]+\$//'):D\"" | nocr | sed -n 's/.*D:\([^:]*\):D.*/\1/p' | head -1)"
   426	  # Returns non-zero rather than dying, so the CALLER decides. (The self-test runs
   427	  # each gate in a subshell to survive a refusal — and a `die` in there was invisible
   428	  # while the global it sets was discarded, so the drain then had no device and
   429	  # reported DRAIN-ERROR. The self-test was breaking its own next gate.)
   430	  if [[ ! "$dev" =~ ^disk[0-9]+$ ]]; then
   431	    log "$(hname "$h"): cannot resolve the PHYSICAL disk behind $p (got '$dev') — a drain that watches the wrong device certifies a disk the data never touched, and on APFS a synthesized disk can read idle while the physical store saturates"
   432	    return 1
   433	  fi
   434	  # It must actually REPORT: an iostat that emits nothing for this device would
   435	  # make every sample non-numeric, and the drain must never read that as quiet.
   436	  local probe
   437	  probe="$(hrun "$h" "iostat -d -w 1 -c 2 '$dev' 2>/dev/null | tail -1 | awk '{print \$3}'" | nocr)" || probe=""
   438	  if [[ ! "$probe" =~ ^[0-9]+\.?[0-9]*$ ]]; then
   439	    log "$(hname "$h"): iostat does not report numeric throughput for $dev (got '$probe') — cannot certify drainage"
   440	    return 1
   441	  fi
   442	  if [[ "$h" == n ]]; then N_DISK="$dev"; else Q_DISK="$dev"; fi
   443	  log "  drain device on $(hname "$h"): $dev (backs $p, idle probe ${probe} MB/s)"
   444	}
   445	
   446	# --- the settle-gap bound (NOT a removal — a measured bound) -------------------
   447	# Between the client exiting and the fsync starting, the OS writes back dirty pages
   448	# FOR FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
   449	# BY DIRECTION. SETTLE_MS makes the window EQUAL on both arms; the residual is the
   450	# ssh return-path difference, which is bounded by the round-trip time measured here.
   451	# It is NOT "removed by construction", and the pre-registration no longer says so.
   452	#
   453	# Timed in ONE process, for the same reason the transfer is. Bracketing each ssh
   454	# with two `python3 -c time.time()` calls would have charged it TWO interpreter
   455	# startups (~30 ms) and reported them as network latency — measured: it read 35 ms
   456	# for a round trip that is actually ~5 ms. The instrument's own bound would have
   457	# been wrong by 7x, in the direction that flatters nothing and confuses everything.
   458	SSH_RTT_MS=0
   459	measure_ssh_rtt() {
   460	  # A FAILED ssh must not contribute a plausible number (round-5 codex, MEDIUM): a
   461	  # fast-failing connection would report a small "bound" and flatter the settle claim.
   462	  SSH_RTT_MS="$(python3 -c '
   463	import statistics, subprocess, sys, time
   464	argv = sys.argv[1:]
   465	ts = []
   466	for _ in range(5):
   467	    t = time.monotonic()
   468	    rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
   469	    if rc != 0:
   470	        print("SSH-FAILED")
   471	        raise SystemExit
   472	    ts.append((time.monotonic() - t) * 1000.0)
   473	print(int(statistics.median(ts)))
   474	' ssh "${SSH_MUX[@]}" "$Q_SSH" true)"
   475	  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die_blind "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
   476	  local rtt_max=$(( SETTLE_MS / 4 ))
   477	  (( SSH_RTT_MS <= rtt_max )) \
   478	    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
   479	  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
   480	}
   481	
   482	# =============================================================================
   483	preflight() {
   484	  # RUNS=8, and ONLY 8. The RUNS=16 escalation was removed (owner, 2026-07-14): a null is
   485	  # judged on the FULL RANGE, which only WIDENS with n, so more pairs could never rescue an
   486	  # UNCLEAR rig or certify a control -- and if you already have an EFFECT you do not need
   487	  # it. Its p-hacking guard surface goes with it.
   488	  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
   489	
   490	  # The instrument must be the REVIEWED instrument: a modified harness must not be
   491	  # able to claim the reviewed commit.
   492	  git -C "$REPO_ROOT" diff --quiet HEAD -- "$SELF" "$VERDICT_PY" "$VERDICT_TEST" \
   493	    || die "the instrument has UNCOMMITTED changes (harness/verdict/test) — it cannot claim the reviewed commit. Commit or stash first."
   494	  # The decision rule proves itself before it grades anything — AND proves the proof
   495	  # is not vacuous. Running only the cases would let a silently-reverted fix pass
   496	  # preflight if the cases still happen to pass for another reason (round-3 grok).
   497	  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
   498	    || die "the verdict engine's OWN guard test FAILS (see $OUT_DIR/verdict-guard-test.txt) — the decision rule is broken; refusing to take data"
   499	  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
   500	    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt); the rule is not actually guarded, refusing to take data"
   501	  log "verdict-engine guard test passed ($(grep -cE ' ok$' "$OUT_DIR/verdict-guard-test.txt" || true) cases, $(grep -cE 'KILLED' "$OUT_DIR/verdict-mutations.txt" || true) mutations killed)"
   502	
   503	  local h p w want got wantb gotb
   504	  for h in n q; do
   505	    resolve_python "$h" || die_blind "$(hname "$h"): cannot establish an absolute python3 — refusing"
   506	    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
   507	    timer_gate "$h"                       # THE measurand's clock, proved on the rig
   508	    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
   509	      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
   510	      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
   511	    done
   512	    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
   513	    # THE SAME pgrep FAIL-OPEN AS THE QUIESCENCE GATE, IN A DUPLICATE SITE I DID NOT
   514	    # TOUCH (round-5 codex, HIGH). `if hrun ... pgrep; then die; fi` reads rc>=2 (a
   515	    # BROKEN probe, or a failed ssh) as "no daemon is running" and sails on. Every
   516	    # process probe now goes through this one rc-aware helper -- there is no second
   517	    # site left to forget.
   518	    case "$(pgrep_state "$h" blit-daemon)" in
   519	      RUNNING) die "$(hname "$h"): a blit-daemon is already running — stop it first" ;;
   520	      NONE)    : ;;
   521	      *)       die "$(hname "$h"): cannot probe for a stale blit-daemon — refusing (a gate that cannot answer must not answer 'fine')" ;;
   522	    esac
   523	    for w in large mixed small; do
   524	      want="$(fix_count "$w")"; wantb="$(fix_bytes "$w")"
   525	      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
   526	      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
   527	      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
   528	        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
   529	    done
   530	    link_gate "$h"
   531	    resolve_disk "$h" || die "$(hname "$h"): cannot establish the drain device — refusing"
   532	  done
   533	  measure_ssh_rtt
   534	  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
   535	  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
   536	}
   537	
   538	write_manifest() {
   539	  local f="$OUT_DIR/staging-manifest.txt" h nb nd qb qd vh th
   540	  # Hashes computed FIRST, in the caller's shell: `die` inside $(...) exits only the
   541	  # subshell, so the old code wrote an EMPTY hash and called it provenance.
   542	  nb="$(sha256_of n "$N_BLIT")"   || die "nagatha: cannot hash $N_BLIT"
   543	  nd="$(sha256_of n "$N_DAEMON")" || die "nagatha: cannot hash $N_DAEMON"
   544	  qb="$(sha256_of q "$Q_BLIT")"   || die "q: cannot hash $Q_BLIT"
   545	  qd="$(sha256_of q "$Q_DAEMON")" || die "q: cannot hash $Q_DAEMON"
   546	  vh="$(shasum -a 256 "$VERDICT_PY" | cut -d' ' -f1)"
   547	  th="$(shasum -a 256 "$VERDICT_TEST" | cut -d' ' -f1)"
   548	  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
   549	    echo "# verdict_sha256=$vh verdict_test_sha256=$th"   # the engine grades separately: hash it too
   550	    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
   551	    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
   552	    echo "# drain_disk_nagatha=$N_DISK drain_disk_q=$Q_DISK ssh_rtt_ms=$SSH_RTT_MS"
   553	    echo "# cells=$CELLS"
   554	    echo "host,role,sha,sha256,path"
   555	    echo "nagatha,client,$EXPECT_SHA,$nb,$N_BLIT"
   556	    echo "nagatha,daemon,$EXPECT_SHA,$nd,$N_DAEMON"
   557	    echo "q,client,$EXPECT_SHA,$qb,$Q_BLIT"
   558	    echo "q,daemon,$EXPECT_SHA,$qd,$Q_DAEMON"; } > "$f"
   559	  log "staging manifest recorded (harness + verdict-engine + 4 binary hashes, every threshold)"
   560	}

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '1,280p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '561,840p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	#!/usr/bin/env bash
     2	# =============================================================================
     3	# ⛔ NOT CLEARED TO RUN — REWORKED IN ROUND 3, REVIEW NOT YET PASSED ⛔
     4	#
     5	# The round-3 rework (this file) addresses all 15 findings from codex round 2 and
     6	# grok's second opinion. It has NOT been reviewed. The review is the gate, not the
     7	# rework: three rounds running, every revision of this instrument has shipped a
     8	# defect capable of a false claim, and two of them were introduced BY THE REWORK
     9	# THAT FIXED THE PREVIOUS ONE.
    10	#
    11	#   .review/results/macmac-harness-r2.gpt-verdict.md    (codex, 12 findings)
    12	#   .review/results/macmac-harness-r2.grok-verdict.md   (grok, +3 findings)
    13	#
    14	# Clearing it: land the round-3 review, adjudicate, and delete this block plus the
    15	# CLEARED_BY_REVIEW guard below. Until then `SELFTEST=1` and `PREFLIGHT_ONLY=1`
    16	# work (they take NO data); a timed run refuses.
    17	# =============================================================================
    18	# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
    19	# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md (rev 4)
    20	# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
    21	#
    22	# WHY THIS RIG EXISTS
    23	# -------------------
    24	# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
    25	# on macOS<->Windows. Linux<->Linux shows NO P1. macOS<->macOS is the untested
    26	# cell. It answers ONE question, SCOPED TO THIS PAIR:
    27	#
    28	#     Can P1 occur WITHOUT a Windows peer, on this pair of Macs?
    29	#
    30	#   * reproduces -> P1 does NOT require a Windows peer (on this pair). It is not
    31	#     "platform residue" that can be waived; code-level hypotheses strengthen. It
    32	#     leaves macOS/APFS and host x role explanations OPEN.
    33	#   * null       -> P1 did NOT reproduce on this pair. That is CONSISTENT with
    34	#     "Windows is required", but does NOT prove it: it could equally be a
    35	#     property of these two machines, their disks, or this macOS version.
    36	#
    37	# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
    38	# H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
    39	# InitiatorReceivePlaneRun.add_dialed_stream, the dial-before-ACK at
    40	# transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
    41	# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with H1,
    42	# not fatal to it. (The parent warns: "'consistent with H1' is not confirmation.")
    43	#
    44	# THE INSTRUMENT IS THE RISK. Three claims in this project have been retracted to
    45	# harness bugs, and this harness alone has now had 20 defects found across two
    46	# reviews. What round 2 caught, and what is fixed here:
    47	#
    48	#   * THE TIMER WAS MEASURING FSYNC NOISE. It captured time.monotonic() in TWO
    49	#     separate `python3 -c` processes and subtracted them. On macOS that clock is
    50	#     PROCESS-RELATIVE: a 1000 ms sleep measured -1 ms on nagatha and 2 ms on q
    51	#     (measured; yes, negative). Every `ms` row would have been ~= fsync_ms alone,
    52	#     and the invariance ratio — THE ENTIRE MEASURAND — would have been computed on
    53	#     fsync noise, which can manufacture or mask a one-directional effect at will.
    54	#     The repo ALREADY documents this trap (bench_otp12_zoey.sh:116 uses time.time()
    55	#     precisely because monotonic is wrong across processes) and I reintroduced it
    56	#     anyway. Now: ONE process times itself and spawns the client (time_argv), and
    57	#     PREFLIGHT PROVES IT on both hosts against a known sleep before any data.
    58	#   * The preflight COULD NOT SUCCEED: `grep -c` exits 1 on no match, so a CLEAN
    59	#     binary tripped the dirty-marker probe and died; and norm_mac used gawk's
    60	#     strtonum(), absent from stock macOS awk. The round-1 "fixes" were never
    61	#     executed — I ran `bash -n`, not the gates. Every gate below is now exercised
    62	#     by SELFTEST=1, which runs them for real.
    63	#   * Gates FAILED OPEN: pgrep errors read as "quiet"; a failed `top` read as 0%
    64	#     CPU and a late idle sample could overwrite a busy one; non-numeric `iostat`
    65	#     read as zero and CERTIFIED drainage; the drain watched a hardcoded `disk0`
    66	#     that the data need never touch (grok); `die` inside $(...) exited only the
    67	#     subshell, so an empty hash still landed. Every probe is now sentinel-framed,
    68	#     rc-aware, and fails CLOSED.
    69	#
    70	# TOPOLOGY: the driver runs on nagatha; the nagatha end is LOCAL and q is over
    71	# ssh. Each timed window is self-timed ON the initiating host (locally, or INSIDE
    72	# one ssh), so dispatch is outside the window by construction.
    73	#
    74	# Usage:
    75	#   SELFTEST=1       bash scripts/bench_otp12pf_mac.sh   # exercise every gate, no data
    76	#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    77	#   EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh # the run (needs review clearance)
    78	# =============================================================================
    79	set -euo pipefail
    80	
    81	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    82	REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    83	SELF="${BASH_SOURCE[0]}"
    84	VERDICT_PY="$SCRIPT_DIR/otp12pf_mac_verdict.py"
    85	VERDICT_TEST="$SCRIPT_DIR/otp12pf_mac_verdict_test.py"
    86	
    87	SELFTEST="${SELFTEST:-0}"
    88	PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
    89	
    90	# The review is the gate. A timed run refuses until round 3 is adjudicated; the
    91	# no-data modes stay available so the gates can be exercised.
    92	if [[ "$SELFTEST" != 1 && "$PREFLIGHT_ONLY" != 1 && "${CLEARED_BY_REVIEW:-0}" != 1 ]]; then
    93	  echo "REFUSING: this harness was reworked in round 3 and has NOT passed review." >&2
    94	  echo "Every previous revision shipped a defect capable of a false claim, and two" >&2
    95	  echo "were introduced by the rework that fixed the last one. Land the round-3" >&2
    96	  echo "review first. SELFTEST=1 and PREFLIGHT_ONLY=1 take no data and still run." >&2
    97	  exit 2
    98	fi
    99	
   100	# The pre-registered build. Not overridable by accident: a run against an
   101	# unregistered build is not the registered experiment.
   102	REGISTERED_BUILD="f35702a"
   103	
   104	# --- nagatha: LOCAL end (driver) ---------------------------------------------
   105	N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
   106	N_NIC="${N_NIC:-en11}"
   107	N_MAC="${N_MAC:-00:e0:4d:01:4c:a3}"              # nagatha's OWN en11 MAC (measured)
   108	N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"
   109	N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
   110	N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
   111	N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
   112	
   113	# --- q: REMOTE end ------------------------------------------------------------
   114	Q_SSH="${Q_SSH:-michael@q}"
   115	Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
   116	Q_NIC="${Q_NIC:-en8}"
   117	Q_MAC="${Q_MAC:-00:01:d2:19:04:a3}"              # q's OWN en8 MAC (measured)
   118	Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
   119	Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
   120	Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
   121	Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"
   122	
   123	PORT="${PORT:-9031}"
   124	RUNS="${RUNS:-8}"
   125	
   126	# =============================================================================
   127	# THE REGISTERED CONSTANTS. **NOT OVERRIDABLE.**
   128	#
   129	# Round-5 (codex, BLOCKER): these were `${VAR:-default}`, so the pre-registered
   130	# decision rule could be edited FROM THE COMMAND LINE — `DELTA_REF_MS=240` turned a
   131	# RIG-VOID into a VANISHES. A pre-registration that the operator can retune, after
   132	# the data exists, in the direction of the answer they want, IS NOT A
   133	# PRE-REGISTRATION AT ALL.
   134	#
   135	# They are literals, and the harness REFUSES to start if one is merely PRESENT in the
   136	# environment — a deviation must be loud, never silently ignored. The check reads the
   137	# environment BEFORE the assignments below, or an override would be masked by the
   138	# very line meant to pin it.
   139	# =============================================================================
   140	_overrides=""
   141	for _v in SETTLE_MS LOAD_MAX DRAIN_ITERS DRAIN_QUIET DRAIN_MBPS DELTA_REF_MS TIMER_TOLERANCE_MS; do
   142	  [[ -n "${!_v+set}" ]] && _overrides="$_overrides $_v=${!_v}"
   143	done
   144	if [[ -n "$_overrides" ]]; then
   145	  echo "REFUSING: the pre-registered constants are NOT tunable, and these are set in the" >&2
   146	  echo "environment:$_overrides" >&2
   147	  echo "A rule the operator can retune after seeing the data is not a pre-registration." >&2
   148	  echo "To change one, amend docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md and" >&2
   149	  echo "put it back through review. That is the entire point of the document." >&2
   150	  exit 2
   151	fi
   152	
   153	SETTLE_MS=250              # equal pre-fsync window on BOTH arms
   154	# Computed ONCE, HERE, at top level — and this line is load-bearing history.
   155	#
   156	# It used to be computed inline as `sleep $(awk ... 'BEGIN{printf \"%.3f\", m/1000}')`
   157	# INSIDE the double-quoted hrun string. A command substitution is parsed FRESH by
   158	# bash, so those `\"` escapes — which are correct for hrun's two-level strings — were
   159	# literal backslashes to awk. **The awk errored on EVERY call, `sleep` got an empty
   160	# argument and FAILED, and the old code ignored its exit status because the python
   161	# walk that followed supplied the status.**
   162	#
   163	# So THE SETTLE HAS NEVER RUN — not once, in any revision, since 24660ae introduced
   164	# it. And 24660ae is the commit that added it TO FIX the free-writeback asymmetry
   165	# that reverses sign with direction — the artifact judged capable of MANUFACTURING a
   166	# one-directional P1 out of nothing. The pre-registration has claimed an equal settle
   167	# on both arms through revisions 3, 4 and 5. It was never applied.
   168	#
   169	# Found only by EXECUTING it (round-5 codex flagged the ignored exit status; running
   170	# it showed the status was ALWAYS failure). `bash -n` sees nothing here.
   171	SETTLE_SEC="$(awk -v m="$SETTLE_MS" 'BEGIN{printf "%.3f", m/1000}')"
   172	[[ "$SETTLE_SEC" =~ ^[0-9]+\.[0-9]+$ ]] || { echo "FATAL: settle seconds did not compute ('$SETTLE_SEC')" >&2; exit 1; }
   173	LOAD_MAX=3.0               # start AND end load1 bar on both Macs
   174	DRAIN_ITERS=60
   175	DRAIN_QUIET=3
   176	DRAIN_MBPS=2               # destination disk must be below this to start a window
   177	DELTA_REF_MS=230           # rig W's measured Delta_P1 — THE reference effect
   178	TIMER_TOLERANCE_MS=120     # the timer self-test's allowed error on a 1000 ms sleep
   179	
   180	# The REGISTERED cell set. The verdict engine requires ALL of them present and
   181	# complete: a partial set that is merely filtered lets a ONE-CELL run emit
   182	# "VANISHES" while claiming both cells vanished (codex r2 BLOCKER 1).
   183	REGISTERED_CELLS="nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   184	CONTROL_CELLS="nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large"
   185	VERDICT_CELLS="nq_tcp_mixed,qn_tcp_mixed"
   186	CELLS="$REGISTERED_CELLS"
   187	
   188	SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
   189	OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
   190	
   191	MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"   # /tmp: macOS TMPDIR busts ssh's 104b ControlPath cap
   192	SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
   193	         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
   194	qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
   195	
   196	mkdir -p "$OUT_DIR/blit-logs"
   197	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
   198	die() { log "FATAL: $*"; exit 1; }
   199	# A gate that CANNOT ANSWER is BLIND, and blindness is what fails open on the night.
   200	# It is marked EXPLICITLY here, never inferred from the wording of a message —
   201	# inferring it from prose is how a blind timer came to be scored as a working gate.
   202	die_blind() { log "FATAL[PROBE-BLIND]: $*"; exit 1; }
   203	nocr() { tr -d '\r'; }
   204	
   205	# --- host abstraction: $1 = n (local) | q (remote) -----------------------------
   206	# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
   207	# falls through to the wrong host (the trap the Linux harness documents).
   208	# `bash -c` locally pins the inner shell so local and remote parse identically.
   209	# pipefail is set in BOTH children: without it a failed probe at the head of a
   210	# pipeline is masked by a successful `tail`/`awk` and the gate reads "fine".
   211	hrun() {
   212	  local h="$1"; shift
   213	  local cmd="set -o pipefail
   214	$*"
   215	  if [[ "$h" == n ]]; then bash -c "$cmd"; else qssh "bash -c $(printf '%q' "$cmd")"; fi
   216	}
   217	hblit()   { if [[ "$1" == n ]]; then echo "$N_BLIT";   else echo "$Q_BLIT";   fi; }
   218	hdaemon() { if [[ "$1" == n ]]; then echo "$N_DAEMON"; else echo "$Q_DAEMON"; fi; }
   219	hmod()    { if [[ "$1" == n ]]; then echo "$N_MODULE"; else echo "$Q_MODULE"; fi; }
   220	hip()     { if [[ "$1" == n ]]; then echo "$N_IP";     else echo "$Q_IP";     fi; }
   221	hnic()    { if [[ "$1" == n ]]; then echo "$N_NIC";    else echo "$Q_NIC";    fi; }
   222	hmac()    { if [[ "$1" == n ]]; then echo "$N_MAC";    else echo "$Q_MAC";    fi; }
   223	hname()   { if [[ "$1" == n ]]; then echo nagatha;     else echo q;           fi; }
   224	other()   { if [[ "$1" == n ]]; then echo q;           else echo n;           fi; }
   225	
   226	# --- fixtures (otp-2 shapes) — count AND byte sum, never trusted --------------
   227	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   228	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   229	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   230	fix_count() { case "$1" in large) echo $FIX_COUNT_large;; mixed) echo $FIX_COUNT_mixed;; small) echo $FIX_COUNT_small;; esac; }
   231	fix_bytes() { case "$1" in large) echo $FIX_BYTES_large;; mixed) echo $FIX_BYTES_mixed;; small) echo $FIX_BYTES_small;; esac; }
   232	
   233	# =============================================================================
   234	# THE TIMER. One process times itself AND spawns the client, so the interval is
   235	# measured by a single clock and python's startup cost falls outside it.
   236	#
   237	# NEVER bracket a command with two separate `python3 -c 'time.monotonic()'` calls:
   238	# on macOS that clock is PROCESS-RELATIVE and the difference is garbage (measured:
   239	# -1 ms and 2 ms for a 1000 ms sleep). bench_otp12_zoey.sh:116 already said so.
   240	# =============================================================================
   241	time_argv() {   # $1 = host; rest = argv. Echoes "MS,RC" or "" on a broken probe.
   242	  local h="$1"; shift
   243	  local qa="" a
   244	  for a in "$@"; do qa="$qa $(printf '%q' "$a")"; done
   245	  hrun "$h" "$(hpy "$h") - $qa <<'PYEOF'
   246	import subprocess, sys, time
   247	argv = [a for a in sys.argv[1:] if a]          # an empty flag must not become argv
   248	err = open('/tmp/mm-client.err', 'wb')
   249	t = time.monotonic()
   250	rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=err)
   251	ms = int((time.monotonic() - t) * 1000)
   252	err.close()
   253	print('R:%d,%d:R' % (ms, rc))
   254	PYEOF" | nocr | sed -n 's/.*R:\(-\{0,1\}[0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1
   255	}
   256	
   257	# The gate that makes the timer bug unshippable: prove the clock on the rig,
   258	# against a known interval, before any data is taken.
   259	timer_gate() {
   260	  local h="$1" out ms rc lo hi
   261	  out="$(time_argv "$h" /bin/sleep 1)"
   262	  [[ "$out" == *,* ]] || die_blind "$(hname "$h"): the timer probe returned nothing — refusing"
   263	  ms="${out%%,*}"; rc="${out##*,}"
   264	  [[ "$rc" == 0 ]] || die_blind "$(hname "$h"): the timer probe's own child exited $rc"
   265	  lo=$(( 1000 - TIMER_TOLERANCE_MS )); hi=$(( 1000 + TIMER_TOLERANCE_MS ))
   266	  if (( ms < lo || ms > hi )); then
   267	    die "$(hname "$h"): THE TIMER IS LYING — a 1000 ms sleep measured ${ms} ms (allowed ${lo}-${hi}).
   268	This is the round-2 killer: cross-process time.monotonic() on macOS is PROCESS-RELATIVE and
   269	read -1 ms / 2 ms for this exact sleep. Every row would be fsync noise. REFUSING to take data."
   270	  fi
   271	  log "  timer ok on $(hname "$h"): a 1000 ms sleep measures ${ms} ms"
   272	}
   273	
   274	# --- provenance ---------------------------------------------------------------
   275	# `die` inside $(...) exits only the SUBSHELL, so the outer command substitution
   276	# succeeds with an empty value. These return non-zero instead and the CALLER dies.
   277	embeds_clean() {   # fail CLOSED: a read error must never read as "clean"
   278	  local h="$1" p="$2" raw hit dirty
   279	  # `grep -c` exits 1 on NO MATCH, which is not an error. Only rc>=2 is. The old
   280	  # `|| echo X` turned a clean binary's legitimate "0" into "0\nX" and DIED.

 succeeded in 0ms:
   561	
   562	# --- daemons ------------------------------------------------------------------
   563	N_PY=""; Q_PY=""
   564	hpy() { if [[ "$1" == n ]]; then echo "$N_PY"; else echo "$Q_PY"; fi; }
   565	resolve_python() {
   566	  local h="$1" p
   567	  p="$(hrun "$h" "command -v python3" | nocr)" || p=""
   568	  if [[ "$p" != /* ]]; then
   569	    log "$(hname "$h"): cannot resolve an absolute python3 (got '$p')"; return 1
   570	  fi
   571	  if ! hrun "$h" "test -x '$p'"; then
   572	    log "$(hname "$h"): python3 at '$p' is not executable"; return 1
   573	  fi
   574	  if [[ "$h" == n ]]; then N_PY="$p"; else Q_PY="$p"; fi
   575	  log "  python3 on $(hname "$h"): $p (absolute — a PATH entry or shell function cannot stand in for the interpreter that MEASURES the settle)"
   576	}
   577	
   578	N_PID=""; Q_PID=""; TEARDOWN_FAILED=0
   579	daemon_start() {
   580	  local h="$1" cfg mod bin pid
   581	  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
   582	  # The daemon's OWN pid, from $! — not `pgrep | head -1`, which picks the first of
   583	  # whatever happens to be running.
   584	  pid="$(hrun "$h" "mkdir -p '$mod' || exit 1
   585	printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg' || exit 1
   586	nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
   587	echo \"P:\$!:P\"" | nocr | sed -n 's/.*P:\([0-9][0-9]*\):P.*/\1/p' | head -1)"
   588	  [[ "$pid" =~ ^[0-9]+$ ]] || die "$(hname "$h"): daemon did not report a pid (see $mod/mm-daemon.log)"
   589	  # OWN THE PID BEFORE VALIDATING IT (round-5 codex, MEDIUM): the old code stored it
   590	  # only AFTER the alive/listening checks, so a daemon that started but failed
   591	  # validation was `die`d on while the EXIT trap did not yet know its pid — leaking a
   592	  # live daemon holding the port for the next session to trip over.
   593	  if [[ "$h" == n ]]; then N_PID="$pid"; else Q_PID="$pid"; fi
   594	  sleep 2
   595	  hrun "$h" "ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon" \
   596	    || die "$(hname "$h"): daemon pid $pid is not alive (see $mod/mm-daemon.log)"
   597	  # ALIVE is not SERVING: it must hold the port we are about to measure through.
   598	  hrun "$h" "lsof -nP -a -p $pid -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1" \
   599	    || die "$(hname "$h"): daemon pid $pid is NOT LISTENING on $PORT (see $mod/mm-daemon.log)"
   600	  log "$(hname "$h") daemon up (pid $pid, listening) on $(hip "$h"):$PORT"
   601	}
   602	# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
   603	# handshake reached some listener's backlog — not that the daemon speaks blit).
   604	smoke() {
   605	  local h="$1" o
   606	  o="$(other "$h")"
   607	  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 \
   608	    || die "$(hname "$o"): cannot stage the smoke fixture"
   609	  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
   610	    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
   611	    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
   612	  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
   613	  log "smoke ok: $(hname "$h") daemon serves blit"
   614	}
   615	daemon_stop() {
   616	  local h="$1" pid state
   617	  if [[ "$h" == n ]]; then pid="$N_PID"; else pid="$Q_PID"; fi
   618	  [[ -n "$pid" ]] || return 0
   619	  hrun "$h" "kill $pid 2>/dev/null || true
   620	for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done
   621	if ps -p $pid >/dev/null 2>&1; then kill -9 $pid 2>/dev/null || true; sleep 1; fi" >/dev/null 2>&1 || true
   622	  # A teardown that cannot be VERIFIED is a failure, not a success. The old probe
   623	  # called a FAILED ssh "GONE".
   624	  state="$(hrun "$h" "if ps -p $pid >/dev/null 2>&1; then echo 'S:ALIVE:S'; else echo 'S:GONE:S'; fi" \
   625	    | nocr | sed -n 's/.*S:\([A-Z]*\):S.*/\1/p' | head -1)" || state=""
   626	  if [[ "$state" != GONE ]]; then
   627	    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown or could not be probed (got '$state') — port $PORT may still be held"
   628	    TEARDOWN_FAILED=1
   629	    touch "$OUT_DIR/TEARDOWN-FAILED"
   630	    return 1
   631	  fi
   632	  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
   633	}
   634	cleanup() {
   635	  daemon_stop n || true
   636	  daemon_stop q || true
   637	  rm -rf "$MUX" 2>/dev/null || true
   638	  if [[ "$TEARDOWN_FAILED" == 1 ]]; then
   639	    log "ERROR: a daemon survived teardown — see $OUT_DIR/TEARDOWN-FAILED. Clean it up before the next session."
   640	  fi
   641	}
   642	trap cleanup EXIT
   643	
   644	# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
   645	RUN_DRAIN=""; RUN_COLD=""
   646	drain_host() {   # $1 = host. Echoes drained_<n>x2s | DRAIN-TIMEOUT | DRAIN-ERROR
   647	  local h="$1" dev out
   648	  dev="$(hdisk "$h")"
   649	  [[ -n "$dev" ]] || { echo DRAIN-ERROR; return 0; }
   650	  out="$(
   651	  # A FAILED iostat must not certify quiet even when it printed a parseable line
   652	  # (round-5 codex, HIGH: a numeric line followed by a NONZERO EXIT still accumulated
   653	  # "quiet" samples). The exit code is now checked BEFORE the value is used.
   654	  hrun "$h" "quiet=0
   655	for i in \$(seq 1 $DRAIN_ITERS); do
   656	  out=\$(iostat -d -w 2 -c 2 '$dev' 2>/dev/null); rc=\$?
   657	  if [ \$rc -ne 0 ]; then echo DRAIN-ERROR; exit 0; fi
   658	  w=\$(echo \"\$out\" | tail -1 | awk '{print \$3}')
   659	  case \"\$w\" in
   660	    ''|*[!0-9.]*) echo DRAIN-ERROR; exit 0 ;;   # non-numeric must NEVER certify quiet
   661	  esac
   662	  ok=\$(awk -v w=\"\$w\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   663	  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
   664	  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
   665	done
   666	echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1)" || out="DRAIN-ERROR"
   667	  # ONE token, or it is an error -- AND the probe must have EXITED cleanly. A drain that
   668	  # printed `drained_*` and THEN failed is not a drain (codex r8: I fixed the value and
   669	  # left the status, which is the same defect one layer down).
   670	  case "$out" in
   671	    drained_[0-9]*x2s) echo "$out" ;;
   672	    DRAIN-TIMEOUT)     echo DRAIN-TIMEOUT ;;
   673	    *)                 echo DRAIN-ERROR ;;
   674	  esac
   675	}
   676	prep_run() {   # $1 = dest host
   677	  local dh="$1" cn=ok cq=ok out
   678	  # Purge BOTH ends first — the purge itself dirties the disk, so a drain certified
   679	  # BEFORE it proves nothing.
   680	  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   681	  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   682	  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   683	  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
   684	  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
   685	  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   686	  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
   687	}
   688	
   689	# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
   690	RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
   691	fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
   692	  local out
   693	  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
   694	  #
   695	  # It used to be a shell `sleep` before the python. Round 5 found the awk computing
   696	  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
   697	  # RAN. Round 6 then found the repair was still not provable: `sleep` is
   698	  # PATH/function-resolved, the walk's timer starts AFTER it, and the self-test only
   699	  # counted files — so a no-op `sleep` would pass while the log narrated "settle
   700	  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").
   701	  #
   702	  # A protection that cannot be OBSERVED is not a protection. The settle now happens
   703	  # in python, is timed by the same monotonic clock as the walk, and is REPORTED. The
   704	  # caller VOIDS the pair if it did not actually elapse. There is no shell sleep left
   705	  # to shadow, no exit status left to discard, and no narration left to trust.
   706	  out="$(hrun "$1" "$(hpy "$1") - '$SETTLE_SEC' '$2' <<'PYEOF'
   707	import os, sys, time
   708	settle = float(sys.argv[1])
   709	p = sys.argv[2]
   710	t0 = time.monotonic()
   711	time.sleep(settle)
   712	settled_ms = int((time.monotonic() - t0) * 1000)
   713	if not os.path.isdir(p):
   714	    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
   715	    raise SystemExit
   716	t = time.monotonic()
   717	files = 0
   718	nbytes = 0
   719	for root, _d, fs in os.walk(p):
   720	    for name in fs:
   721	        fp = os.path.join(root, name)
   722	        nbytes += os.path.getsize(fp)
   723	        fd = os.open(fp, os.O_RDONLY)
   724	        os.fsync(fd)
   725	        os.close(fd)
   726	        files += 1
   727	print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
   728	PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3 \4/p' | head -1)" || out=""
   729	  echo "${out:-NA 0 0 0}"
   730	}
   731	# The settle actually elapsed, on the destination's own clock. Anything else voids.
   732	settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
   733	
   734	# --- one timed run ------------------------------------------------------------
   735	RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
   736	timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
   737	  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin wc wb
   738	  bin="$(hblit "$ih")"
   739	  prep_run "$dh"
   740	  out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
   741	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   742	  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
   743	  RUN_VALID=yes
   744	  wc="$(fix_count "$w")"; wb="$(fix_bytes "$w")"
   745	  # The equal settle is the ONLY thing standing between this rig and a free-writeback
   746	  # artifact that REVERSES SIGN WITH DIRECTION — i.e. that can manufacture P1 out of
   747	  # nothing. It has already been silently dead once. If it did not measurably elapse,
   748	  # the row is not a fast row; it is a VOID row.
   749	  if ! settle_ok "$RUN_SETTLED"; then
   750	    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
   751	    RUN_VALID=no
   752	  fi
   753	  if [[ "$RUN_FLUSH" == NA ]]; then
   754	    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
   755	    RUN_VALID=no; RUN_FLUSH=0
   756	  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
   757	    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
   758	    RUN_VALID=no
   759	  fi
   760	  # A negative or absurd transfer time means the CLOCK failed, not that the transfer
   761	  # was fast. It must never enter the data.
   762	  if [[ ! "$RUN_MS" =~ ^[0-9]+$ ]] || (( RUN_MS < 1 )); then
   763	    log "  VOID: transfer timer returned '$RUN_MS' — the clock failed (round 2's killer). NOT a fast run."
   764	    RUN_VALID=no; RUN_MS=0
   765	  fi
   766	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   767	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
   768	}
   769	
   770	# --- arms ---------------------------------------------------------------------
   771	# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
   772	# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
   773	# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
   774	# above is what makes a wrong path fatal instead of silently free.
   775	CUR_W=""; CUR_FLAG=""
   776	arm_srcinit() {
   777	  local sh="$1" dh="$2" run="$3"
   778	  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
   779	            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
   780	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   781	}
   782	arm_destinit() {
   783	  local sh="$1" dh="$2" run="$3"
   784	  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
   785	            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
   786	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   787	}
   788	
   789	CSV="$OUT_DIR/runs.csv"
   790	META="$OUT_DIR/meta.csv"
   791	
   792	# THE CELLS ARE INTERLEAVED, NOT RUN BACK TO BACK.
   793	#
   794	# Round-8 (codex, HIGH): both measurand cells used to run first, then the controls. So the
   795	# controls certified a window THEY NEVER SHARED -- a transient (a background process, a
   796	# thermal excursion, a disk that woke up) could hit the measurand and be entirely gone by
   797	# the time the gRPC/large controls ran, and they would certify the rig as clean. The
   798	# controls are the ONLY thing standing between this rig and a rig-wide artifact, and they
   799	# cannot vouch for a window they were not in.
   800	#
   801	# So the schedule is SLOT-MAJOR: within slot i, EVERY cell takes one ABBA pair, in a fixed
   802	# registered order, before any cell takes slot i+1. All six cells therefore span the same
   803	# wall-clock window and see the same transients.
   804	#
   805	#   cell           src dst fixture flag
   806	CELL_TABLE=(
   807	  "nq_tcp_mixed    n   q   mixed   "
   808	  "qn_tcp_mixed    q   n   mixed   "
   809	  "nq_grpc_mixed   n   q   mixed   --force-grpc"
   810	  "qn_grpc_mixed   q   n   mixed   --force-grpc"
   811	  "nq_tcp_large    n   q   large   "
   812	  "qn_tcp_large    q   n   large   "
   813	)
   814	
   815	# macOS ships bash 3.2, which has NO associative arrays. Parallel indexed arrays, keyed by
   816	# the cell's position in CELL_TABLE.
   817	CELL_VALID=(); CELL_ATTEMPTS=()
   818	run_one_pair() {   # $1=idx $2=cell $3=srchost $4=dsthost $5=fixture $6=flag $7=slot -> 0 if VALID
   819	  local i="$1" cell="$2" sh="$3" dh="$4" w="$5" flag="$6" slot="$7"
   820	  local attempts=$(( ${CELL_ATTEMPTS[$i]:-0} + 1 ))
   821	  CELL_ATTEMPTS[$i]=$attempts
   822	  CUR_W="$w"; CUR_FLAG="$flag"
   823	  local order pair=yes rowA="" rowB="" arm aname init rid run
   824	  # ABBA: the arm order alternates by slot, so a monotonic drift cannot favour one arm.
   825	  if (( slot % 2 )); then order="A B"; else order="B A"; fi
   826	  for arm in $order; do
   827	    if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
   828	    rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
   829	    if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"; else arm_destinit "$sh" "$dh" "$run"; fi
   830	    [[ "$RUN_VALID" == yes ]] || pair=no
   831	    local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
   832	    if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
   833	    log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (fsync ${RUN_FLUSH}ms, settle ${RUN_SETTLED}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
   834	  done
   835	  echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
   836	  if [[ "$pair" == yes ]]; then
   837	    CELL_VALID[$i]=$(( ${CELL_VALID[$i]:-0} + 1 ))
   838	    return 0
   839	  fi
   840	  log "  $cell: pair at slot $slot VOIDED"

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '320,450p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '451,580p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   451	# It is NOT "removed by construction", and the pre-registration no longer says so.
   452	#
   453	# Timed in ONE process, for the same reason the transfer is. Bracketing each ssh
   454	# with two `python3 -c time.time()` calls would have charged it TWO interpreter
   455	# startups (~30 ms) and reported them as network latency — measured: it read 35 ms
   456	# for a round trip that is actually ~5 ms. The instrument's own bound would have
   457	# been wrong by 7x, in the direction that flatters nothing and confuses everything.
   458	SSH_RTT_MS=0
   459	measure_ssh_rtt() {
   460	  # A FAILED ssh must not contribute a plausible number (round-5 codex, MEDIUM): a
   461	  # fast-failing connection would report a small "bound" and flatter the settle claim.
   462	  SSH_RTT_MS="$(python3 -c '
   463	import statistics, subprocess, sys, time
   464	argv = sys.argv[1:]
   465	ts = []
   466	for _ in range(5):
   467	    t = time.monotonic()
   468	    rc = subprocess.call(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
   469	    if rc != 0:
   470	        print("SSH-FAILED")
   471	        raise SystemExit
   472	    ts.append((time.monotonic() - t) * 1000.0)
   473	print(int(statistics.median(ts)))
   474	' ssh "${SSH_MUX[@]}" "$Q_SSH" true)"
   475	  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die_blind "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
   476	  local rtt_max=$(( SETTLE_MS / 4 ))
   477	  (( SSH_RTT_MS <= rtt_max )) \
   478	    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
   479	  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
   480	}
   481	
   482	# =============================================================================
   483	preflight() {
   484	  # RUNS=8, and ONLY 8. The RUNS=16 escalation was removed (owner, 2026-07-14): a null is
   485	  # judged on the FULL RANGE, which only WIDENS with n, so more pairs could never rescue an
   486	  # UNCLEAR rig or certify a control -- and if you already have an EFFECT you do not need
   487	  # it. Its p-hacking guard surface goes with it.
   488	  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
   489	
   490	  # The instrument must be the REVIEWED instrument: a modified harness must not be
   491	  # able to claim the reviewed commit.
   492	  git -C "$REPO_ROOT" diff --quiet HEAD -- "$SELF" "$VERDICT_PY" "$VERDICT_TEST" \
   493	    || die "the instrument has UNCOMMITTED changes (harness/verdict/test) — it cannot claim the reviewed commit. Commit or stash first."
   494	  # The decision rule proves itself before it grades anything — AND proves the proof
   495	  # is not vacuous. Running only the cases would let a silently-reverted fix pass
   496	  # preflight if the cases still happen to pass for another reason (round-3 grok).
   497	  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
   498	    || die "the verdict engine's OWN guard test FAILS (see $OUT_DIR/verdict-guard-test.txt) — the decision rule is broken; refusing to take data"
   499	  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
   500	    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt); the rule is not actually guarded, refusing to take data"
   501	  log "verdict-engine guard test passed ($(grep -cE ' ok$' "$OUT_DIR/verdict-guard-test.txt" || true) cases, $(grep -cE 'KILLED' "$OUT_DIR/verdict-mutations.txt" || true) mutations killed)"
   502	
   503	  local h p w want got wantb gotb
   504	  for h in n q; do
   505	    resolve_python "$h" || die_blind "$(hname "$h"): cannot establish an absolute python3 — refusing"
   506	    quiescence_gate "$h"; timemachine_gate "$h"; spotlight_gate "$h"; load_gate "$h"
   507	    timer_gate "$h"                       # THE measurand's clock, proved on the rig
   508	    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
   509	      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
   510	      embeds_clean "$h" "$p" || die "$(hname "$h"): $p is not a CLEAN +$EXPECT_SHA (same-build rule D-2026-07-05-2; a read error also fails here, by design)"
   511	    done
   512	    hrun "$h" "sudo -n /usr/sbin/purge" || die "$(hname "$h") cannot purge without a password — every run would read WARM"
   513	    # THE SAME pgrep FAIL-OPEN AS THE QUIESCENCE GATE, IN A DUPLICATE SITE I DID NOT
   514	    # TOUCH (round-5 codex, HIGH). `if hrun ... pgrep; then die; fi` reads rc>=2 (a
   515	    # BROKEN probe, or a failed ssh) as "no daemon is running" and sails on. Every
   516	    # process probe now goes through this one rc-aware helper -- there is no second
   517	    # site left to forget.
   518	    case "$(pgrep_state "$h" blit-daemon)" in
   519	      RUNNING) die "$(hname "$h"): a blit-daemon is already running — stop it first" ;;
   520	      NONE)    : ;;
   521	      *)       die "$(hname "$h"): cannot probe for a stale blit-daemon — refusing (a gate that cannot answer must not answer 'fine')" ;;
   522	    esac
   523	    for w in large mixed small; do
   524	      want="$(fix_count "$w")"; wantb="$(fix_bytes "$w")"
   525	      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
   526	      gotb="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f -exec stat -f %z {} + 2>/dev/null | awk '{s+=\$1} END{printf \"%d\", s+0}'" | tr -cd '0-9')"
   527	      [[ "${got:-0}" == "$want" && "${gotb:-0}" == "$wantb" ]] \
   528	        || die "$(hname "$h"): src_$w is ${got:-0} files/${gotb:-0} bytes, want $want/$wantb — the arms must read identical trees"
   529	    done
   530	    link_gate "$h"
   531	    resolve_disk "$h" || die "$(hname "$h"): cannot establish the drain device — refusing"
   532	  done
   533	  measure_ssh_rtt
   534	  log "preflight OK  build=$EXPECT_SHA  harness=$HARNESS_HEAD  runs/arm=$RUNS  settle=${SETTLE_MS}ms"
   535	  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
   536	}
   537	
   538	write_manifest() {
   539	  local f="$OUT_DIR/staging-manifest.txt" h nb nd qb qd vh th
   540	  # Hashes computed FIRST, in the caller's shell: `die` inside $(...) exits only the
   541	  # subshell, so the old code wrote an EMPTY hash and called it provenance.
   542	  nb="$(sha256_of n "$N_BLIT")"   || die "nagatha: cannot hash $N_BLIT"
   543	  nd="$(sha256_of n "$N_DAEMON")" || die "nagatha: cannot hash $N_DAEMON"
   544	  qb="$(sha256_of q "$Q_BLIT")"   || die "q: cannot hash $Q_BLIT"
   545	  qd="$(sha256_of q "$Q_DAEMON")" || die "q: cannot hash $Q_DAEMON"
   546	  vh="$(shasum -a 256 "$VERDICT_PY" | cut -d' ' -f1)"
   547	  th="$(shasum -a 256 "$VERDICT_TEST" | cut -d' ' -f1)"
   548	  { echo "# harness_head=$HARNESS_HEAD harness_sha256=$HARNESS_SHA256"
   549	    echo "# verdict_sha256=$vh verdict_test_sha256=$th"   # the engine grades separately: hash it too
   550	    echo "# binary_identity=$EXPECT_SHA runs=$RUNS settle_ms=$SETTLE_MS load_max=$LOAD_MAX"
   551	    echo "# drain_mbps=$DRAIN_MBPS drain_quiet=$DRAIN_QUIET delta_ref_ms=$DELTA_REF_MS"
   552	    echo "# drain_disk_nagatha=$N_DISK drain_disk_q=$Q_DISK ssh_rtt_ms=$SSH_RTT_MS"
   553	    echo "# cells=$CELLS"
   554	    echo "host,role,sha,sha256,path"
   555	    echo "nagatha,client,$EXPECT_SHA,$nb,$N_BLIT"
   556	    echo "nagatha,daemon,$EXPECT_SHA,$nd,$N_DAEMON"
   557	    echo "q,client,$EXPECT_SHA,$qb,$Q_BLIT"
   558	    echo "q,daemon,$EXPECT_SHA,$qd,$Q_DAEMON"; } > "$f"
   559	  log "staging manifest recorded (harness + verdict-engine + 4 binary hashes, every threshold)"
   560	}
   561	
   562	# --- daemons ------------------------------------------------------------------
   563	N_PY=""; Q_PY=""
   564	hpy() { if [[ "$1" == n ]]; then echo "$N_PY"; else echo "$Q_PY"; fi; }
   565	resolve_python() {
   566	  local h="$1" p
   567	  p="$(hrun "$h" "command -v python3" | nocr)" || p=""
   568	  if [[ "$p" != /* ]]; then
   569	    log "$(hname "$h"): cannot resolve an absolute python3 (got '$p')"; return 1
   570	  fi
   571	  if ! hrun "$h" "test -x '$p'"; then
   572	    log "$(hname "$h"): python3 at '$p' is not executable"; return 1
   573	  fi
   574	  if [[ "$h" == n ]]; then N_PY="$p"; else Q_PY="$p"; fi
   575	  log "  python3 on $(hname "$h"): $p (absolute — a PATH entry or shell function cannot stand in for the interpreter that MEASURES the settle)"
   576	}
   577	
   578	N_PID=""; Q_PID=""; TEARDOWN_FAILED=0
   579	daemon_start() {
   580	  local h="$1" cfg mod bin pid

 succeeded in 0ms:
   320	    }'
   321	}
   322	
   323	# THE ONLY process probe in this harness. pgrep: 0 = found, 1 = none, >=2 = ERROR.
   324	# Echoes RUNNING | NONE | BROKEN. A probe that cannot answer must NEVER answer "fine",
   325	# and there must be exactly ONE of these -- round 5 found the fail-open surviving in a
   326	# duplicate site precisely because there were two.
   327	pgrep_state() {
   328	  local h="$1" pat="$2" raw
   329	  raw="$(hrun "$h" "pgrep -x '$pat' >/dev/null 2>&1; rc=\$?
   330	if [ \$rc -eq 0 ]; then echo 'G:RUNNING:G'
   331	elif [ \$rc -eq 1 ]; then echo 'G:NONE:G'
   332	else echo 'G:BROKEN:G'; fi" | nocr | sed -n 's/.*G:\([A-Z]*\):G.*/\1/p' | head -1)" || raw=""
   333	  case "$raw" in
   334	    RUNNING|NONE|BROKEN) echo "$raw" ;;
   335	    *)                   echo BROKEN ;;   # no sentinel back == a broken probe
   336	  esac
   337	}
   338	
   339	quiescence_gate() {
   340	  local h="$1" p busy=""
   341	  for p in codex cargo rustc; do
   342	    case "$(pgrep_state "$h" "$p")" in
   343	      RUNNING) busy="$busy $p" ;;
   344	      NONE)    : ;;
   345	      *)       die_blind "$(hname "$h"): the quiescence probe for '$p' BROKE — refusing (a gate that cannot answer must not answer 'fine')" ;;
   346	    esac
   347	  done
   348	  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running:$busy). BOTH Macs are bench ENDS — load inflates one arm and MANUFACTURES P1. Stop them (never blanket-kill the owner's sessions) and re-run."
   349	}
   350	
   351	timemachine_gate() {   # FAIL-CLOSED on running OR merely ENABLED (the hole pf-0 found)
   352	  local h="$1" running auto
   353	  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';' | head -1" | nocr | tr -cd '0-9')" || running=""
   354	  [[ "$running" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine status — refusing (a gate that cannot answer must not pass)"
   355	  [[ "$running" -eq 0 ]] || die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench end (one destination is on skippy, the same 10GbE fabric)."
   356	  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null | tr -cd '0-9' | head -1" | nocr | tr -cd '0-9')" || auto=""
   357	  [[ "$auto" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot read Time Machine AutoBackup — refusing (a READ ERROR must never read as 'disabled')"
   358	  [[ "$auto" -eq 0 ]] || die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED. macOS repeats hourly, so a backup can start INSIDE the window — pf-0's fired 1 minute before its run. Disable it for the session (\`sudo tmutil disable\`) and re-enable after."
   359	}
   360	
   361	spotlight_gate() {
   362	  local h="$1" cpu
   363	  # The MAX across samples, not the last: a late idle sample could overwrite an
   364	  # earlier busy one. NR==0 (top produced nothing) is an ERROR, not 0% CPU.
   365	  cpu="$(hrun "$h" "top -l 2 -n 30 -o cpu -stats command,cpu 2>/dev/null \
   366	    | awk '/^mds_stores/{ if (\$2+0 > m) m = \$2+0 } END{ if (NR == 0) print \"ERR\"; else printf \"%d\", m+0 }'" | nocr)" || cpu="ERR"
   367	  [[ "$cpu" =~ ^[0-9]+$ ]] || die_blind "$(hname "$h"): cannot sample Spotlight CPU (got '$cpu') — refusing"
   368	  [[ "$cpu" -lt 20 ]] || die "$(hname "$h"): Spotlight (mds_stores) is indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
   369	}
   370	
   371	load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
   372	load_gate() {
   373	  local h="$1" l ok
   374	  l="$(load1 "$h")" || l=""
   375	  [[ "$l" =~ ^[0-9]+\.?[0-9]*$ ]] || die_blind "$(hname "$h"): cannot read load1 (got '$l') — refusing"
   376	  ok="$(awk -v l="$l" -v m="$LOAD_MAX" 'BEGIN{print (l+0 <= m+0) ? 1 : 0}')"
   377	  [[ "$ok" == 1 ]] || die "$(hname "$h"): load1 is $l (> $LOAD_MAX) — a bench END must be quiet. Find what is running first."
   378	}
   379	
   380	link_gate() {   # both directions; the peer's ARP must be the PEER's MAC, never our own
   381	  local h="$1" o peer_ip want got route_nic nic
   382	  o="$(other "$h")"; peer_ip="$(hip "$o")"; want="$(hmac "$o" | norm_mac)"; nic="$(hnic "$h")"
   383	  [[ -n "$want" ]] || die_blind "$(hname "$o"): its configured MAC does not parse — refusing"
   384	  hrun "$h" "ping -c1 -W1 '$peer_ip' >/dev/null 2>&1" \
   385	    || die "$(hname "$h") cannot ping $peer_ip — the link is down"
   386	  # The ARP entry ON THE NIC THE TRAFFIC WILL EGRESS. `arp -n <ip>` prints one line
   387	  # PER INTERFACE that has an entry — q holds entries for nagatha on en0, en1 AND
   388	  # en8 — so an unfiltered $4 yields a MULTI-LINE string that can never equal a
   389	  # single MAC. (Measured: this refused a perfectly good link. It is also the more
   390	  # correct check: a stale entry on the 1GbE NIC is irrelevant to the 10GbE path.)
   391	  got="$(hrun "$h" "arp -n '$peer_ip' 2>/dev/null | awk -v nic='$nic' '\$5 == \"on\" && \$6 == nic {print \$4}' | head -1" | nocr | norm_mac)"
   392	  [[ -n "$got" ]] || die "$(hname "$h"): no ARP entry for $peer_ip ON $nic — the 10GbE path has not resolved the peer"
   393	  [[ "$got" == "$want" ]] \
   394	    || die "$(hname "$h"): ARP for $peer_ip is $got but the peer's real MAC is $want. If it equals OUR OWN NIC's MAC this is the documented BLACK HOLE (a host route on a directly-connected subnet) — 100% packet loss while \`route -n get\` still reports the right interface."
   395	  route_nic="$(hrun "$h" "route -n get '$peer_ip' 2>/dev/null | awk '/interface:/{print \$2}'" | nocr)"
   396	  [[ "$route_nic" == "$(hnic "$h")" ]] \
   397	    || die "$(hname "$h"): route to $peer_ip egresses '$route_nic', not the 10GbE NIC '$(hnic "$h")' — the multi-NIC trap (macOS routes by network SERVICE order, so a 1GbE NIC can win and every run would go over gigabit)."
   398	}
   399	
   400	# --- the drain device: RESOLVED, never hardcoded (grok) ------------------------
   401	# `iostat disk0` can certify a disk the data never touched. Worse, on APFS the
   402	# volume lives on a SYNTHESIZED disk whose stats may be empty while the physical
   403	# store is saturated — a false "quiet". Resolve the module path to its PHYSICAL
   404	# store and verify iostat actually reports it.
   405	N_DISK=""; Q_DISK=""
   406	hdisk() { if [[ "$1" == n ]]; then echo "$N_DISK"; else echo "$Q_DISK"; fi; }
   407	resolve_disk() {
   408	  local h="$1" p dev
   409	  p="$(hmod "$h")"
   410	  # A FAILED `diskutil` MUST NOT silently fall back to the synthesized disk (round-5
   411	  # codex, HIGH). On APFS the volume lives on a synthesized container whose iostat
   412	  # counters can read IDLE while the physical store is saturated — so falling back to
   413	  # it is not a harmless default, it is a FALSE QUIET that certifies drainage on a
   414	  # device the data never touched. If the volume is APFS, the physical-store lookup
   415	  # must SUCCEED or the gate refuses.
   416	  dev="$(hrun "$h" "d=\$(df '$p' 2>/dev/null | awk 'NR==2{print \$1}' | sed 's|^/dev/||')
   417	[ -n \"\$d\" ] || { echo 'D:NO-DF:D'; exit 0; }
   418	info=\$(diskutil info \"\$d\" 2>/dev/null) || { echo 'D:NO-DISKUTIL:D'; exit 0; }
   419	[ -n \"\$info\" ] || { echo 'D:EMPTY-DISKUTIL:D'; exit 0; }
   420	if echo \"\$info\" | grep -q 'APFS'; then
   421	  ps=\$(echo \"\$info\" | awk -F: '/APFS Physical Store/{gsub(/[ \t]/, \"\", \$2); print \$2}' | head -1)
   422	  [ -n \"\$ps\" ] || { echo 'D:APFS-NO-STORE:D'; exit 0; }
   423	  d=\"\$ps\"
   424	fi
   425	echo \"D:\$(echo \"\$d\" | sed -E 's/s[0-9]+\$//'):D\"" | nocr | sed -n 's/.*D:\([^:]*\):D.*/\1/p' | head -1)"
   426	  # Returns non-zero rather than dying, so the CALLER decides. (The self-test runs
   427	  # each gate in a subshell to survive a refusal — and a `die` in there was invisible
   428	  # while the global it sets was discarded, so the drain then had no device and
   429	  # reported DRAIN-ERROR. The self-test was breaking its own next gate.)
   430	  if [[ ! "$dev" =~ ^disk[0-9]+$ ]]; then
   431	    log "$(hname "$h"): cannot resolve the PHYSICAL disk behind $p (got '$dev') — a drain that watches the wrong device certifies a disk the data never touched, and on APFS a synthesized disk can read idle while the physical store saturates"
   432	    return 1
   433	  fi
   434	  # It must actually REPORT: an iostat that emits nothing for this device would
   435	  # make every sample non-numeric, and the drain must never read that as quiet.
   436	  local probe
   437	  probe="$(hrun "$h" "iostat -d -w 1 -c 2 '$dev' 2>/dev/null | tail -1 | awk '{print \$3}'" | nocr)" || probe=""
   438	  if [[ ! "$probe" =~ ^[0-9]+\.?[0-9]*$ ]]; then
   439	    log "$(hname "$h"): iostat does not report numeric throughput for $dev (got '$probe') — cannot certify drainage"
   440	    return 1
   441	  fi
   442	  if [[ "$h" == n ]]; then N_DISK="$dev"; else Q_DISK="$dev"; fi
   443	  log "  drain device on $(hname "$h"): $dev (backs $p, idle probe ${probe} MB/s)"
   444	}
   445	
   446	# --- the settle-gap bound (NOT a removal — a measured bound) -------------------
   447	# Between the client exiting and the fsync starting, the OS writes back dirty pages
   448	# FOR FREE, and that gap is longer for whichever arm ran over ssh — which REVERSES
   449	# BY DIRECTION. SETTLE_MS makes the window EQUAL on both arms; the residual is the
   450	# ssh return-path difference, which is bounded by the round-trip time measured here.

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '581,700p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   581	  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"; cfg="$mod/mm-bench.toml"
   582	  # The daemon's OWN pid, from $! — not `pgrep | head -1`, which picks the first of
   583	  # whatever happens to be running.
   584	  pid="$(hrun "$h" "mkdir -p '$mod' || exit 1
   585	printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg' || exit 1
   586	nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
   587	echo \"P:\$!:P\"" | nocr | sed -n 's/.*P:\([0-9][0-9]*\):P.*/\1/p' | head -1)"
   588	  [[ "$pid" =~ ^[0-9]+$ ]] || die "$(hname "$h"): daemon did not report a pid (see $mod/mm-daemon.log)"
   589	  # OWN THE PID BEFORE VALIDATING IT (round-5 codex, MEDIUM): the old code stored it
   590	  # only AFTER the alive/listening checks, so a daemon that started but failed
   591	  # validation was `die`d on while the EXIT trap did not yet know its pid — leaking a
   592	  # live daemon holding the port for the next session to trip over.
   593	  if [[ "$h" == n ]]; then N_PID="$pid"; else Q_PID="$pid"; fi
   594	  sleep 2
   595	  hrun "$h" "ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon" \
   596	    || die "$(hname "$h"): daemon pid $pid is not alive (see $mod/mm-daemon.log)"
   597	  # ALIVE is not SERVING: it must hold the port we are about to measure through.
   598	  hrun "$h" "lsof -nP -a -p $pid -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1" \
   599	    || die "$(hname "$h"): daemon pid $pid is NOT LISTENING on $PORT (see $mod/mm-daemon.log)"
   600	  log "$(hname "$h") daemon up (pid $pid, listening) on $(hip "$h"):$PORT"
   601	}
   602	# Liveness proved by a REAL blit transfer, not `nc -z` (which only proves a
   603	# handshake reached some listener's backlog — not that the daemon speaks blit).
   604	smoke() {
   605	  local h="$1" o
   606	  o="$(other "$h")"
   607	  hrun "$o" "mkdir -p '$(hmod "$o")/smoke_src' && echo mm-smoke > '$(hmod "$o")/smoke_src/probe.txt'" >/dev/null 2>&1 \
   608	    || die "$(hname "$o"): cannot stage the smoke fixture"
   609	  hrun "$o" "'$(hblit "$o")' copy '$(hmod "$o")/smoke_src' '$(hip "$h"):$PORT:/bench/mm_smoke_${SESSION_TAG}/' --yes" \
   610	    >/dev/null 2>"$OUT_DIR/blit-logs/smoke_$(hname "$h").err" \
   611	    || die "smoke to $(hname "$h") FAILED — the daemon is not serving blit (see blit-logs/smoke_$(hname "$h").err)"
   612	  hrun "$h" "rm -rf '$(hmod "$h")/mm_smoke_${SESSION_TAG}'" >/dev/null 2>&1 || true
   613	  log "smoke ok: $(hname "$h") daemon serves blit"
   614	}
   615	daemon_stop() {
   616	  local h="$1" pid state
   617	  if [[ "$h" == n ]]; then pid="$N_PID"; else pid="$Q_PID"; fi
   618	  [[ -n "$pid" ]] || return 0
   619	  hrun "$h" "kill $pid 2>/dev/null || true
   620	for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done
   621	if ps -p $pid >/dev/null 2>&1; then kill -9 $pid 2>/dev/null || true; sleep 1; fi" >/dev/null 2>&1 || true
   622	  # A teardown that cannot be VERIFIED is a failure, not a success. The old probe
   623	  # called a FAILED ssh "GONE".
   624	  state="$(hrun "$h" "if ps -p $pid >/dev/null 2>&1; then echo 'S:ALIVE:S'; else echo 'S:GONE:S'; fi" \
   625	    | nocr | sed -n 's/.*S:\([A-Z]*\):S.*/\1/p' | head -1)" || state=""
   626	  if [[ "$state" != GONE ]]; then
   627	    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown or could not be probed (got '$state') — port $PORT may still be held"
   628	    TEARDOWN_FAILED=1
   629	    touch "$OUT_DIR/TEARDOWN-FAILED"
   630	    return 1
   631	  fi
   632	  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
   633	}
   634	cleanup() {
   635	  daemon_stop n || true
   636	  daemon_stop q || true
   637	  rm -rf "$MUX" 2>/dev/null || true
   638	  if [[ "$TEARDOWN_FAILED" == 1 ]]; then
   639	    log "ERROR: a daemon survived teardown — see $OUT_DIR/TEARDOWN-FAILED. Clean it up before the next session."
   640	  fi
   641	}
   642	trap cleanup EXIT
   643	
   644	# --- cold + drain (purge FIRST, then drain, then RE-CHECK) --------------------
   645	RUN_DRAIN=""; RUN_COLD=""
   646	drain_host() {   # $1 = host. Echoes drained_<n>x2s | DRAIN-TIMEOUT | DRAIN-ERROR
   647	  local h="$1" dev out
   648	  dev="$(hdisk "$h")"
   649	  [[ -n "$dev" ]] || { echo DRAIN-ERROR; return 0; }
   650	  out="$(
   651	  # A FAILED iostat must not certify quiet even when it printed a parseable line
   652	  # (round-5 codex, HIGH: a numeric line followed by a NONZERO EXIT still accumulated
   653	  # "quiet" samples). The exit code is now checked BEFORE the value is used.
   654	  hrun "$h" "quiet=0
   655	for i in \$(seq 1 $DRAIN_ITERS); do
   656	  out=\$(iostat -d -w 2 -c 2 '$dev' 2>/dev/null); rc=\$?
   657	  if [ \$rc -ne 0 ]; then echo DRAIN-ERROR; exit 0; fi
   658	  w=\$(echo \"\$out\" | tail -1 | awk '{print \$3}')
   659	  case \"\$w\" in
   660	    ''|*[!0-9.]*) echo DRAIN-ERROR; exit 0 ;;   # non-numeric must NEVER certify quiet
   661	  esac
   662	  ok=\$(awk -v w=\"\$w\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   663	  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
   664	  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained_\${i}x2s\"; exit 0; fi
   665	done
   666	echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1)" || out="DRAIN-ERROR"
   667	  # ONE token, or it is an error -- AND the probe must have EXITED cleanly. A drain that
   668	  # printed `drained_*` and THEN failed is not a drain (codex r8: I fixed the value and
   669	  # left the status, which is the same defect one layer down).
   670	  case "$out" in
   671	    drained_[0-9]*x2s) echo "$out" ;;
   672	    DRAIN-TIMEOUT)     echo DRAIN-TIMEOUT ;;
   673	    *)                 echo DRAIN-ERROR ;;
   674	  esac
   675	}
   676	prep_run() {   # $1 = dest host
   677	  local dh="$1" cn=ok cq=ok out
   678	  # Purge BOTH ends first — the purge itself dirties the disk, so a drain certified
   679	  # BEFORE it proves nothing.
   680	  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   681	  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   682	  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   683	  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
   684	  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"
   685	  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   686	  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
   687	}
   688	
   689	# --- durability: DESTINATION host, both arms, and it VERIFIES WHAT IT FLUSHED --
   690	RUN_FLUSH=0; RUN_FILES=0; RUN_BYTES=0; RUN_SETTLED=0
   691	fsync_tree() {   # $1 = DEST host, $2 = landed path -> "ms files bytes settled_ms" | "NA 0 0 0"
   692	  local out
   693	  # THE SETTLE IS PERFORMED AND **MEASURED** INSIDE THE SAME PROCESS AS THE WALK.
   694	  #
   695	  # It used to be a shell `sleep` before the python. Round 5 found the awk computing
   696	  # its duration had ALWAYS errored, so the sleep ALWAYS failed and THE SETTLE NEVER
   697	  # RAN. Round 6 then found the repair was still not provable: `sleep` is
   698	  # PATH/function-resolved, the walk's timer starts AFTER it, and the self-test only
   699	  # counted files — so a no-op `sleep` would pass while the log narrated "settle
   700	  # included" (codex + grok, BLOCKER, and grok measured a 44 ms "250 ms settle").

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_mac.sh | sed -n '701,820p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   701	  #
   702	  # A protection that cannot be OBSERVED is not a protection. The settle now happens
   703	  # in python, is timed by the same monotonic clock as the walk, and is REPORTED. The
   704	  # caller VOIDS the pair if it did not actually elapse. There is no shell sleep left
   705	  # to shadow, no exit status left to discard, and no narration left to trust.
   706	  out="$(hrun "$1" "$(hpy "$1") - '$SETTLE_SEC' '$2' <<'PYEOF'
   707	import os, sys, time
   708	settle = float(sys.argv[1])
   709	p = sys.argv[2]
   710	t0 = time.monotonic()
   711	time.sleep(settle)
   712	settled_ms = int((time.monotonic() - t0) * 1000)
   713	if not os.path.isdir(p):
   714	    print('F:NA:0:0:%d:F' % settled_ms)   # a MISSING tree must never read as a fast flush
   715	    raise SystemExit
   716	t = time.monotonic()
   717	files = 0
   718	nbytes = 0
   719	for root, _d, fs in os.walk(p):
   720	    for name in fs:
   721	        fp = os.path.join(root, name)
   722	        nbytes += os.path.getsize(fp)
   723	        fd = os.open(fp, os.O_RDONLY)
   724	        os.fsync(fd)
   725	        os.close(fd)
   726	        files += 1
   727	print('F:%d:%d:%d:%d:F' % (int((time.monotonic() - t) * 1000), files, nbytes, settled_ms))
   728	PYEOF" | nocr | sed -n 's/.*F:\([^:]*\):\([0-9]*\):\([0-9]*\):\([0-9]*\):F.*/\1 \2 \3 \4/p' | head -1)" || out=""
   729	  echo "${out:-NA 0 0 0}"
   730	}
   731	# The settle actually elapsed, on the destination's own clock. Anything else voids.
   732	settle_ok() { [[ "$1" =~ ^[0-9]+$ ]] && (( $1 >= SETTLE_MS && $1 < SETTLE_MS * 4 )); }
   733	
   734	# --- one timed run ------------------------------------------------------------
   735	RUN_MS=0; RUN_EXIT=0; RUN_VALID=yes
   736	timed_run() {   # $1=init host $2=src $3=dst $4=DEST host $5=landed $6=flag $7=fixture
   737	  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" w="$7" out bin wc wb
   738	  bin="$(hblit "$ih")"
   739	  prep_run "$dh"
   740	  out="$(time_argv "$ih" "$bin" copy "$src" "$dst" --yes $flag)"
   741	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   742	  read -r RUN_FLUSH RUN_FILES RUN_BYTES RUN_SETTLED <<<"$(fsync_tree "$dh" "$landed")"
   743	  RUN_VALID=yes
   744	  wc="$(fix_count "$w")"; wb="$(fix_bytes "$w")"
   745	  # The equal settle is the ONLY thing standing between this rig and a free-writeback
   746	  # artifact that REVERSES SIGN WITH DIRECTION — i.e. that can manufacture P1 out of
   747	  # nothing. It has already been silently dead once. If it did not measurably elapse,
   748	  # the row is not a fast row; it is a VOID row.
   749	  if ! settle_ok "$RUN_SETTLED"; then
   750	    log "  VOID: the settle did not elapse (measured ${RUN_SETTLED}ms, want >= ${SETTLE_MS}ms) — the free-writeback gap is UNEQUALIZED and can manufacture a one-directional result"
   751	    RUN_VALID=no
   752	  fi
   753	  if [[ "$RUN_FLUSH" == NA ]]; then
   754	    log "  VOID: fsync found no tree at $landed (a missing tree must never read as a fast flush)"
   755	    RUN_VALID=no; RUN_FLUSH=0
   756	  elif [[ "$RUN_FILES" != "$wc" || "$RUN_BYTES" != "$wb" ]]; then
   757	    log "  VOID: destination has $RUN_FILES files/$RUN_BYTES bytes, want $wc/$wb — an exit-0 zero/partial transfer must not become a fast row"
   758	    RUN_VALID=no
   759	  fi
   760	  # A negative or absurd transfer time means the CLOCK failed, not that the transfer
   761	  # was fast. It must never enter the data.
   762	  if [[ ! "$RUN_MS" =~ ^[0-9]+$ ]] || (( RUN_MS < 1 )); then
   763	    log "  VOID: transfer timer returned '$RUN_MS' — the clock failed (round 2's killer). NOT a fast run."
   764	    RUN_VALID=no; RUN_MS=0
   765	  fi
   766	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   767	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
   768	}
   769	
   770	# --- arms ---------------------------------------------------------------------
   771	# The landed paths DIFFER by arm because blit uses rsync-style slash semantics:
   772	# a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>; a pull into RUNDIR
   773	# lands the files DIRECTLY in RUNDIR. Verified empirically. The count+byte gate
   774	# above is what makes a wrong path fatal instead of silently free.
   775	CUR_W=""; CUR_FLAG=""
   776	arm_srcinit() {
   777	  local sh="$1" dh="$2" run="$3"
   778	  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/$run/" \
   779	            "$dh" "$(hmod "$dh")/$run/src_$CUR_W" "$CUR_FLAG" "$CUR_W"
   780	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   781	}
   782	arm_destinit() {
   783	  local sh="$1" dh="$2" run="$3"
   784	  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/$run" \
   785	            "$dh" "$(hmod "$dh")/$run" "$CUR_FLAG" "$CUR_W"
   786	  hrun "$dh" "rm -rf '$(hmod "$dh")/$run'" >/dev/null 2>&1 || true
   787	}
   788	
   789	CSV="$OUT_DIR/runs.csv"
   790	META="$OUT_DIR/meta.csv"
   791	
   792	# THE CELLS ARE INTERLEAVED, NOT RUN BACK TO BACK.
   793	#
   794	# Round-8 (codex, HIGH): both measurand cells used to run first, then the controls. So the
   795	# controls certified a window THEY NEVER SHARED -- a transient (a background process, a
   796	# thermal excursion, a disk that woke up) could hit the measurand and be entirely gone by
   797	# the time the gRPC/large controls ran, and they would certify the rig as clean. The
   798	# controls are the ONLY thing standing between this rig and a rig-wide artifact, and they
   799	# cannot vouch for a window they were not in.
   800	#
   801	# So the schedule is SLOT-MAJOR: within slot i, EVERY cell takes one ABBA pair, in a fixed
   802	# registered order, before any cell takes slot i+1. All six cells therefore span the same
   803	# wall-clock window and see the same transients.
   804	#
   805	#   cell           src dst fixture flag
   806	CELL_TABLE=(
   807	  "nq_tcp_mixed    n   q   mixed   "
   808	  "qn_tcp_mixed    q   n   mixed   "
   809	  "nq_grpc_mixed   n   q   mixed   --force-grpc"
   810	  "qn_grpc_mixed   q   n   mixed   --force-grpc"
   811	  "nq_tcp_large    n   q   large   "
   812	  "qn_tcp_large    q   n   large   "
   813	)
   814	
   815	# macOS ships bash 3.2, which has NO associative arrays. Parallel indexed arrays, keyed by
   816	# the cell's position in CELL_TABLE.
   817	CELL_VALID=(); CELL_ATTEMPTS=()
   818	run_one_pair() {   # $1=idx $2=cell $3=srchost $4=dsthost $5=fixture $6=flag $7=slot -> 0 if VALID
   819	  local i="$1" cell="$2" sh="$3" dh="$4" w="$5" flag="$6" slot="$7"
   820	  local attempts=$(( ${CELL_ATTEMPTS[$i]:-0} + 1 ))

exec
/bin/zsh -lc "nl -ba scripts/otp12pf_mac_verdict_test.py | sed -n '286,345p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   286	     "DELTA_REF_MS is PINNED"),
   287	]
   288	
   289	
   290	def rule_unit_tests():
   291	    """The RULE itself, called directly -- because a session at n=8 cannot distinguish the
   292	    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
   293	    closed codex's round-8 blocker; judging a NULL on the RANGE is the SEMANTICS that keeps
   294	    it closed if a larger n is ever registered again, and it can only be tested here."""
   295	    import importlib.util
   296	    spec = importlib.util.spec_from_file_location("eng", DEFAULT_VERDICT)
   297	    # the engine runs on import (it is a script), so exercise classify() via a subprocess-free
   298	    # re-implementation guard: read the function out of the source and exec it in isolation.
   299	    src = open(DEFAULT_VERDICT).read()
   300	    start = src.index("def classify(")
   301	    end = src.index("\n\n", src.index("return \"UNCLEAR\"", start))
   302	    ns = {}
   303	    exec(src[start:end], ns)
   304	    classify = ns["classify"]
   305	    bad = 0
   306	    checks = [
   307	        # ci narrow (outliers trimmed), range wide: a bimodal arm. MUST NOT be NONE.
   308	        ("bimodal: CI=[1,1] but range=[-110,110], T=73", (1, 1, -110, 110, 73, -66), "UNCLEAR"),
   309	        ("clean: CI and range both inside T",            (2, 3, -4, 3, 73, -66),      "NONE"),
   310	        ("a real effect clears T",                       (80, 90, 75, 95, 73, -66),   "EFFECT"),
   311	        ("an inverted effect clears -T",                 (-90, -80, -95, -75, 73, -66), "INVERTED"),
   312	    ]
   313	    for name, args, want in checks:
   314	        got = classify(*args)
   315	        ok = got == want
   316	        print("  %-46s -> %-8s %s" % (name, got, "ok" if ok else "*** FAIL (want %s) ***" % want))
   317	        if not ok:
   318	            bad += 1
   319	
   320	    # THE IDENTITY THE WHOLE RULE LEANS ON: at the registered n=8, the >=95% order-statistic
   321	    # interval IS [min, max]. That is why nothing can be trimmed -- not a null, not B. If
   322	    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
   323	    # must refuse that n (it does).
   324	    ns2 = {}
   325	    src2 = open(DEFAULT_VERDICT).read()
   326	    st = src2.index("def median_ci(")
   327	    exec(src2[st:src2.index("\n\n", src2.index("return best", st))],
   328	         {"comb": __import__("math").comb, "MIN_COVERAGE": 0.95}, ns2)
   329	    import random as _r
   330	    rr = _r.Random(9)
   331	    for _ in range(200):
   332	        d = [rr.randint(-500, 500) for _ in range(8)]
   333	        lo, hi, cov = ns2["median_ci"](d)
   334	        if (lo, hi) != (min(d), max(d)) or cov < 0.95:
   335	            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
   336	            bad += 1
   337	            break
   338	    else:
   339	        print("  %-46s -> %-8s ok" % ("n=8: the >=95% CI IS [min,max] (200 draws)", "identity"))
   340	    return bad
   341	
   342	
   343	def run_cases():
   344	    bad = []
   345	    for name, kw, must_be, must_not in CASES:

codex
The registered `n=8` closes the alleged effect-side trimming hole: at eight pairs, both EFFECT and NONE are range decisions in fact, so a bimodal sample or even one contrary pair forces `UNCLEAR`. I’m now testing the live risks that remain: whether control bias can scale differently from raw milliseconds, whether fixed-order “interleaving” truly makes controls contemporaneous, and whether the guard exercises those properties.
exec
/bin/zsh -lc 'bash --version | head -1 && bash -n scripts/bench_otp12pf_mac.sh && echo BASH_SYNTAX_OK' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)
BASH_SYNTAX_OK

exec
/bin/zsh -lc "rg -n -i \"escalat|runs.?=.?(16|\\{16\\})|re-run at runs|run.*16|n=16|at least 8|26 cases|9/9|29 cases|10/10|11/11|CI bound|RANGE bound|CI lies inside|full range\" scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
scripts/otp12pf_mac_verdict_test.py:292:    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
scripts/otp12pf_mac_verdict_test.py:335:            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
scripts/otp12pf_mac_verdict.py:173:    EFFECT/INVERTED use the >=95% CI on the median; NONE uses the FULL RANGE. At the
scripts/otp12pf_mac_verdict.py:178:    NONE uses the FULL RANGE -- EVERY pair must lie inside +-T. Round 8 (codex, BLOCKER):
scripts/otp12pf_mac_verdict.py:220:#     B = max over clean controls of the largest |RANGE bound| -- the arm asymmetry the
scripts/otp12pf_mac_verdict.py:320:           "There is no escalation: a noisy rig is fixed by a QUIETER RIG, not more pairs."
scripts/otp12pf_mac_verdict.py:384:        "A NULL (NONE) is judged on the full RANGE -- EVERY pair inside the bound -- not on",
scripts/bench_otp12pf_mac.sh:484:  # RUNS=8, and ONLY 8. The RUNS=16 escalation was removed (owner, 2026-07-14): a null is
scripts/bench_otp12pf_mac.sh:485:  # judged on the FULL RANGE, which only WIDENS with n, so more pairs could never rescue an
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:10:> clears −T), **NONE** (CI lies inside ±T — an effect of size T is *excluded*), **UNCLEAR**
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:41:- Round 1 (design, `f0343f4`): NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → **9/9
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:44:  LOW → **11/11 accepted** (`.review/results/macmac-harness.gpt-verdict.md`).
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:64:  n=16), a control at **D=+229** — *one millisecond* under the reference effect —
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:194:           n=16 -> [d(4), d(13)]      coverage 97.87%
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:217:| **NONE** | **the FULL RANGE** lies inside `(T_neg, T_pos)` — *every* pair, not just the median. An effect of size T is **EXCLUDED** (equivalence) |
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:245:    B = max over clean controls of the largest |CI bound|
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:297:### There is NO escalation. `n` is EXACTLY 8.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:299:The old `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:300:**full range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR`
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:304:**And `n` must be EXACTLY 8, not "at least 8" (round-9, grok, BLOCKER).** At the registered
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:327:`scripts/otp12pf_mac_verdict_test.py`: **26 cases — every one a defect a reviewer actually
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:329:of the engine makes exactly that case fail: **9/9 mutations killed**), plus a 300-input

exec
/bin/zsh -lc "python3 -c \"import ast; t=ast.parse(open('scripts/otp12pf_mac_verdict_test.py').read()); print(*[(n.targets[0].id,len(n.value.elts)) for n in t.body if isinstance(n,ast.Assign) and isinstance(n.targets[0],ast.Name) and n.targets[0].id in ('CASES','MUTATIONS')],sep='\\\\n')\"" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-KHix9bqh' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-e4ke0DUu' (errno=Operation not permitted)
('CASES', 31)
('MUTATIONS', 11)

exec
/bin/zsh -lc "bash -c 'emit_runs(){ echo cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid; for c in nq_tcp_mixed qn_tcp_mixed nq_grpc_mixed qn_grpc_mixed nq_tcp_large qn_tcp_large; do if [[ \""'$c" == *_tcp_mixed ]]; then s=10000; d=490; else s=1000; d=49; fi; for i in 1 2 3 4 5 6 7 8; do echo "$c,srcinit,x,h,$i,$s,0,250,1,1,0,drained_1x2s,cold,yes"; echo "$c,destinit,x,h,$i,$((s+d)),0,250,1,1,0,drained_1x2s,cold,yes"; done; done; }; emit_meta(){ echo cell,pairs_attempted,complete; for c in nq_tcp_mixed qn_tcp_mixed nq_grpc_mixed qn_grpc_mixed nq_tcp_large qn_tcp_large; do echo "$c,8,yes"; done; }; VERDICT_CELLS=nq_tcp_mixed,qn_tcp_mixed CONTROL_CELLS=nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large REGISTERED_CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large REQUIRED_PAIRS=8 python3 scripts/otp12pf_mac_verdict.py <(emit_runs) <(emit_meta) /dev/null /dev/null /dev/null /dev/null | grep -E "''^(SESSION VERDICT|  nq_|  qn_)"'"' " in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-0ZiJ8yJc' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-JyEpRrMa' (errno=Operation not permitted)
SESSION VERDICT: REPRODUCES
  nq_grpc_mixed  NONE     ctrl=NONE     D=  +49ms CI=[  +49,  +49] range=[  +49,  +49] T=100ms B=  0ms  ratio=1.049 bar=PASS  sign_p=0.008 (8/8)
  nq_tcp_large   NONE     ctrl=NONE     D=  +49ms CI=[  +49,  +49] range=[  +49,  +49] T=100ms B=  0ms  ratio=1.049 bar=PASS  sign_p=0.008 (8/8)
  nq_tcp_mixed   EFFECT   ctrl=-        D= +490ms CI=[ +490, +490] range=[ +490, +490] T=230ms B= 49ms  ratio=1.049 bar=PASS  sign_p=0.008 (8/8)
  qn_grpc_mixed  NONE     ctrl=NONE     D=  +49ms CI=[  +49,  +49] range=[  +49,  +49] T=100ms B=  0ms  ratio=1.049 bar=PASS  sign_p=0.008 (8/8)
  qn_tcp_large   NONE     ctrl=NONE     D=  +49ms CI=[  +49,  +49] range=[  +49,  +49] T=100ms B=  0ms  ratio=1.049 bar=PASS  sign_p=0.008 (8/8)
  qn_tcp_mixed   EFFECT   ctrl=-        D= +490ms CI=[ +490, +490] range=[ +490, +490] T=230ms B= 49ms  ratio=1.049 bar=PASS  sign_p=0.008 (8/8)

exec
/bin/zsh -lc "bash -c 'emit_runs(){ echo cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid; ds=(-110 0 -110 110 110 0 -110 0); for c in nq_tcp_mixed qn_tcp_mixed nq_grpc_mixed qn_grpc_mixed nq_tcp_large qn_tcp_large; do i=0; for run in 1 2 3 4 5 6 7 8; do if [[ \""'$c" == nq_tcp_mixed || "$c" == qn_tcp_mixed ]]; then d=${ds[$i]}; s=730; else d=0; s=1000; fi; echo "$c,srcinit,x,h,$run,$s,0,250,1,1,0,drained_1x2s,cold,yes"; echo "$c,destinit,x,h,$run,$((s+d)),0,250,1,1,0,drained_1x2s,cold,yes"; i=$((i+1)); done; done; }; emit_meta(){ echo cell,pairs_attempted,complete; for c in nq_tcp_mixed qn_tcp_mixed nq_grpc_mixed qn_grpc_mixed nq_tcp_large qn_tcp_large; do echo "$c,8,yes"; done; }; VERDICT_CELLS=nq_tcp_mixed,qn_tcp_mixed CONTROL_CELLS=nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large REGISTERED_CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large REQUIRED_PAIRS=8 python3 scripts/otp12pf_mac_verdict.py <(emit_runs) <(emit_meta) /dev/null /dev/null /dev/null /dev/null | grep -E "''^(SESSION VERDICT|  nq_tcp_mixed|  qn_tcp_mixed)"'"' " in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-Gtx5yBcv' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-zE8TTSPL' (errno=Operation not permitted)
SESSION VERDICT: UNCLEAR
  nq_tcp_mixed   UNCLEAR  ctrl=-        D=   +0ms CI=[ -110, +110] range=[ -110, +110] T= 73ms B=  0ms  ratio=1.000 bar=PASS  sign_p=1.000 (2/8)
  qn_tcp_mixed   UNCLEAR  ctrl=-        D=   +0ms CI=[ -110, +110] range=[ -110, +110] T= 73ms B=  0ms  ratio=1.000 bar=PASS  sign_p=1.000 (2/8)

exec
/bin/zsh -lc "bash -c 'emit_runs(){ echo cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid; for c in nq_tcp_mixed qn_tcp_mixed nq_grpc_mixed qn_grpc_mixed nq_tcp_large qn_tcp_large; do for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16; do s=1000; d=0; [[ \""'$c" == nq_tcp_mixed || "$c" == qn_tcp_mixed ]] && { if (( i <= 3 )); then d=-500; else d=200; fi; }; echo "$c,srcinit,x,h,$i,$s,0,250,1,1,0,drained_1x2s,cold,yes"; echo "$c,destinit,x,h,$i,$((s+d)),0,250,1,1,0,drained_1x2s,cold,yes"; done; done; }; emit_meta(){ echo cell,pairs_attempted,complete; for c in nq_tcp_mixed qn_tcp_mixed nq_grpc_mixed qn_grpc_mixed nq_tcp_large qn_tcp_large; do echo "$c,16,yes"; done; }; VERDICT_CELLS=nq_tcp_mixed,qn_tcp_mixed CONTROL_CELLS=nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large REGISTERED_CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large REQUIRED_PAIRS=8 python3 scripts/otp12pf_mac_verdict.py <(emit_runs) <(emit_meta) /dev/null /dev/null /dev/null /dev/null | head -1; VERDICT_CELLS=nq_tcp_mixed,qn_tcp_mixed CONTROL_CELLS=nq_grpc_mixed,qn_grpc_mixed REGISTERED_CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed REQUIRED_PAIRS=16 python3 scripts/otp12pf_mac_verdict.py <(emit_runs) <(emit_meta) /dev/null /dev/null /dev/null /dev/null 2>&1 | tail -1'"' " in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-91L9KJaA' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-NpS4C81L' (errno=Operation not permitted)
SESSION VERDICT: INCOMPLETE


exec
/bin/zsh -lc 'VERDICT_CELLS=a CONTROL_CELLS=b REGISTERED_CELLS=a,b REQUIRED_PAIRS=16 python3 scripts/otp12pf_mac_verdict.py /dev/null /dev/null /dev/null /dev/null /dev/null /dev/null 2>&1 | grep REFUSING' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
REFUSING: REQUIRED_PAIRS=16 is not registered (8,).

exec
/bin/zsh -lc 'git diff --quiet 8830fda''^ 8830fda -- bench_otp12pf_mac.sh; echo "short_SELF_rc=$?"; git diff --quiet 8830fda''^ 8830fda -- scripts/bench_otp12pf_mac.sh; echo "canonical_path_rc=$?"; git ls-files --error-unmatch bench_otp12pf_mac.sh >/dev/null 2>&1; echo "short_SELF_tracked_rc=$?"; git ls-files --error-unmatch scripts/bench_otp12pf_mac.sh >/dev/null 2>&1; echo "canonical_tracked_rc=$?"' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-F9wQi4xj' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-L4SqdWzi' (errno=Operation not permitted)
short_SELF_rc=0
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-DaKeLAlx' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-q7QB6xa0' (errno=Operation not permitted)
canonical_path_rc=1
short_SELF_tracked_rc=1
canonical_tracked_rc=0

exec
/bin/zsh -lc "bash -c 'source <(sed -n \"805,866p\" scripts/bench_otp12pf_mac.sh); RUNS=2; META=/dev/null; CSV=/dev/null; log(){ :; }; run_one_pair(){ local i=\""'$1"; CELL_ATTEMPTS[$i]=$(( ${CELL_ATTEMPTS[$i]:-0} + 1 )); CELL_VALID[$i]=$(( ${CELL_VALID[$i]:-0} + 1 )); echo "slot=$7 cell=$2"; return 0; }; run_all_cells'"' " in /Users/michael/Dev/blit_v2
 exited 127 in 0ms:
bash: run_all_cells: command not found

exec
/bin/zsh -lc "bash -c 'source <(printf \"foo(){ echo ok; }\\n\"); type foo; foo' " in /Users/michael/Dev/blit_v2
 exited 127 in 0ms:
bash: line 0: type: foo: not found
bash: foo: command not found

exec
/bin/zsh -lc "bash -c 'eval \""'$(sed -n "805,866p" scripts/bench_otp12pf_mac.sh)"; RUNS=2; META=/dev/null; CSV=/dev/null; log(){ :; }; run_one_pair(){ local i="$1"; CELL_ATTEMPTS[$i]=$(( ${CELL_ATTEMPTS[$i]:-0} + 1 )); CELL_VALID[$i]=$(( ${CELL_VALID[$i]:-0} + 1 )); echo "slot=$7 cell=$2"; return 0; }; run_all_cells'"' " in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=1 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=1 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=1 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=1 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=1 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=1 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=2 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=2 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=2 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=2 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=2 cell=
bash: line 45: cannot create temp file for here document: Operation not permitted
slot=2 cell=
bash: line 53: cannot create temp file for here document: Operation not permitted
bash: line 53: cannot create temp file for here document: Operation not permitted
bash: line 53: cannot create temp file for here document: Operation not permitted
bash: line 53: cannot create temp file for here document: Operation not permitted
bash: line 53: cannot create temp file for here document: Operation not permitted
bash: line 53: cannot create temp file for here document: Operation not permitted

exec
/bin/zsh -lc "bash -c 'eval \""'$(sed -n "645,675p" scripts/bench_otp12pf_mac.sh)"; DRAIN_ITERS=1; DRAIN_QUIET=1; DRAIN_MBPS=2; hdisk(){ echo disk0; }; nocr(){ tr -d "'"\\\\r\"; }; iostat(){ printf \"disk KB/t MB/s\\\\ndisk0 1 .\\\\n\"; }; hrun(){ shift; eval \""'$*"; }; drain_host n'"' " in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
drained_1x2s

exec
/bin/zsh -lc "for slot in 1 2; do sed -n '807,812p' scripts/bench_otp12pf_mac.sh | awk -F\\\" -v s=\""'$slot" '"'{split("'$2,a,/ +/); print "slot=" s, a[1]}'"'; done" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
slot=1 nq_tcp_mixed
slot=1 qn_tcp_mixed
slot=1 nq_grpc_mixed
slot=1 qn_grpc_mixed
slot=1 nq_tcp_large
slot=1 qn_tcp_large
slot=2 nq_tcp_mixed
slot=2 qn_tcp_mixed
slot=2 nq_grpc_mixed
slot=2 qn_grpc_mixed
slot=2 nq_tcp_large
slot=2 qn_tcp_large

exec
/bin/zsh -lc "git show 8830fda:scripts/otp12pf_mac_verdict_test.py | python3 -c \"import ast,sys; t=ast.parse(sys.stdin.read()); print(*[(n.targets[0].id,len(n.value.elts)) for n in t.body if isinstance(n,ast.Assign) and isinstance(n.targets[0],ast.Name) and n.targets[0].id in ('CASES','MUTATIONS')],sep='\\\\n')\"" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp insteadconfstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead

git: error: couldn't create cache file '/tmp/xcrun_db-ITfx3sgJ' (errno=Operation not permitted)
python3: error: couldn't create cache file '/tmp/xcrun_db-u0NWe8GV' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-IkIfWwpd' (errno=Operation not permitted)
git: error: couldn't create cache file '/tmp/xcrun_db-oBkAuBIu' (errno=Operation not permitted)
('CASES', 29)
('MUTATIONS', 10)

exec
/bin/zsh -lc "git show 8830fda:scripts/otp12pf_mac_verdict.py | nl -ba | sed -n '185,270p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "git show 8830fda:docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md | nl -ba | sed -n '208,255p;280,330p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "git show 8830fda:scripts/bench_otp12pf_mac.sh | nl -ba | sed -n '475,500p;810,875p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-DDLjZZ8O' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LeAWqzw8' (errno=Operation not permitted)
   185	
   186	# ---- pass 1: measure every cell -----------------------------------------------------
   187	cell = {}
   188	for c in sorted(set(REGISTERED) | set(meta)):
   189	    d = paired(c)
   190	    ci = median_ci(d) if d else None
   191	    # COMPLETE is checked against the DATA, never against meta's say-so: a one-pair CSV
   192	    # with a lying meta once graded as a full cell and emitted a null at 0% coverage.
   193	    if meta.get(c, {}).get("complete") != "yes" or len(d) < PAIRS or ci is None:
   194	        cell[c] = dict(state="INCOMPLETE", n=len(d))
   195	        continue
   196	    s_med, d_med = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   197	    hi, lo = max(s_med, d_med), min(s_med, d_med)
   198	    ci_lo, ci_hi, cov = ci
   199	    p, k, n = sign_p(d)
   200	    cell[c] = dict(n=len(d), d=d, D=med(d), ci=(ci_lo, ci_hi), rng=(min(d), max(d)),
   201	                   cov=cov, src=s_med, dst=d_med, p=p, k=k,
   202	                   # The acceptance bar: integer-exact, `<= 1.10` PASSES. REPORTED, never used.
   203	                   bar="PASS" if 10 * hi <= 11 * lo else "FAIL",
   204	                   ratio=hi / lo if lo else 0.0)
   205	
   206	# ---- pass 2: the controls certify the rig, and BOUND its residual bias ---------------
   207	# A control certifies clean at T/2 -- but "clean" is not "zero". A control sitting at +49
   208	# with T/2 = 50 is accepted, and THAT 49 ms OF ARM BIAS MAY BE RIDING IN THE MEASURAND
   209	# TOO, so a measurand "EFFECT" at exactly T could be half real and half rig (round-8
   210	# codex, BLOCKER). The bias the controls FAIL TO EXCLUDE is therefore carried into the
   211	# measurand's thresholds:
   212	#
   213	#     B = max over clean controls of the largest |CI bound|   -- the arm asymmetry that
   214	#                                                                could not be ruled out
   215	#     an EFFECT must clear  T + B     (bias could be INFLATING it)
   216	#     a NULL   must fit in  T - B     (bias could be MASKING an effect)
   217	#
   218	# If the controls are genuinely clean, B is a few ms and this barely moves. If they are
   219	# marginal, it bites -- which is the point.
   220	dirty = []
   221	B = 0.0
   222	for c in CONTROLS:
   223	    x = cell.get(c, {})
   224	    if x.get("state") == "INCOMPLETE":
   225	        continue
   226	    c_pos, c_neg = thresholds(x["src"], 0.5)
   227	    x["ctrl_state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], c_pos, c_neg)
   228	    x["ctrl_T"] = c_pos
   229	    if x["ctrl_state"] != "NONE":
   230	        dirty.append(c)
   231	    else:
   232	        B = max(B, abs(x["ci"][0]), abs(x["ci"][1]))
   233	
   234	# ---- pass 3: grade the measurands, against thresholds widened by the control bias -----
   235	for c in MEASURANDS:
   236	    x = cell.get(c, {})
   237	    if x.get("state") == "INCOMPLETE":
   238	        continue
   239	    t_pos, t_neg = thresholds(x["src"])
   240	    x["T"] = t_pos
   241	    x["B"] = B
   242	    x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1],
   243	                          t_pos + B, t_neg - B)          # an EFFECT must clear T + B
   244	    if x["state"] == "NONE":
   245	        # ...and a NULL must survive the TIGHTER bound: bias could be masking an effect.
   246	        if not (t_neg + B < x["rng"][0] and x["rng"][1] < t_pos - B):
   247	            x["state"] = "UNCLEAR"
   248	
   249	# Controls also carry a state for the report; measurands carry a ctrl_state for symmetry.
   250	for c in cell:
   251	    x = cell[c]
   252	    if x.get("state") == "INCOMPLETE":
   253	        continue
   254	    if "state" not in x:                                  # a control: report its own state
   255	        t_pos, t_neg = thresholds(x["src"])
   256	        x["T"] = t_pos
   257	        x["B"] = 0.0
   258	        x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], t_pos, t_neg)
   259	    x.setdefault("ctrl_state", "-")
   260	
   261	# ---- outputs -----------------------------------------------------------------------
   262	with open(sum_p, "w") as f:
   263	    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,voided,runs\n")
   264	    for (c, a) in sorted(by):
   265	        v = by[(c, a)]
   266	        f.write("%s,%s,%d,%d,%d,%d,%d,%s\n" % (c, a, med(v), sum(v) // len(v), min(v),
   267	                                               max(v), voided.get((c, a), 0),
   268	                                               " ".join(map(str, v))))
   269	
   270	with open(pair_p, "w") as f:

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-VBSOfzeW' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HXYYK95U' (errno=Operation not permitted)
   475	  [[ "$SSH_RTT_MS" =~ ^[0-9]+$ ]] || die_blind "cannot measure the ssh round trip (got '$SSH_RTT_MS') — refusing"
   476	  local rtt_max=$(( SETTLE_MS / 4 ))
   477	  (( SSH_RTT_MS <= rtt_max )) \
   478	    || die "ssh dispatch is ${SSH_RTT_MS} ms (max ${rtt_max} ms) — the residual free-writeback asymmetry is bounded BY this number, and at that size it is no longer negligible against a ${SETTLE_MS} ms settle. A measured bound that is not ENFORCED is a note, not a protection."
   479	  log "  ssh dispatch (warm mux, median of 5): ${SSH_RTT_MS} ms (max ${rtt_max}) — ENFORCED; it bounds the residual settle-gap asymmetry (the settle is ${SETTLE_MS} ms, EQUAL on both arms)"
   480	}
   481	
   482	# =============================================================================
   483	preflight() {
   484	  # RUNS=8, and ONLY 8. The RUNS=16 escalation was removed (owner, 2026-07-14): a null is
   485	  # judged on the FULL RANGE, which only WIDENS with n, so more pairs could never rescue an
   486	  # UNCLEAR rig or certify a control -- and if you already have an EFFECT you do not need
   487	  # it. Its p-hacking guard surface goes with it.
   488	  [[ "$RUNS" == 8 ]] || die "RUNS must be 8 (the registered value) — got '$RUNS'"
   489	
   490	  # The instrument must be the REVIEWED instrument: a modified harness must not be
   491	  # able to claim the reviewed commit.
   492	  git -C "$REPO_ROOT" diff --quiet HEAD -- "$SELF" "$VERDICT_PY" "$VERDICT_TEST" \
   493	    || die "the instrument has UNCOMMITTED changes (harness/verdict/test) — it cannot claim the reviewed commit. Commit or stash first."
   494	  # The decision rule proves itself before it grades anything — AND proves the proof
   495	  # is not vacuous. Running only the cases would let a silently-reverted fix pass
   496	  # preflight if the cases still happen to pass for another reason (round-3 grok).
   497	  python3 "$VERDICT_TEST" >"$OUT_DIR/verdict-guard-test.txt" 2>&1 \
   498	    || die "the verdict engine's OWN guard test FAILS (see $OUT_DIR/verdict-guard-test.txt) — the decision rule is broken; refusing to take data"
   499	  python3 "$VERDICT_TEST" --mutations >"$OUT_DIR/verdict-mutations.txt" 2>&1 \
   500	    || die "the verdict guard test is VACUOUS — a mutation SURVIVED (see $OUT_DIR/verdict-mutations.txt); the rule is not actually guarded, refusing to take data"
   810	  "qn_grpc_mixed   q   n   mixed   --force-grpc"
   811	  "nq_tcp_large    n   q   large   "
   812	  "qn_tcp_large    q   n   large   "
   813	)
   814	
   815	# macOS ships bash 3.2, which has NO associative arrays. Parallel indexed arrays, keyed by
   816	# the cell's position in CELL_TABLE.
   817	CELL_VALID=(); CELL_ATTEMPTS=()
   818	run_one_pair() {   # $1=idx $2=cell $3=srchost $4=dsthost $5=fixture $6=flag $7=slot -> 0 if VALID
   819	  local i="$1" cell="$2" sh="$3" dh="$4" w="$5" flag="$6" slot="$7"
   820	  local attempts=$(( ${CELL_ATTEMPTS[$i]:-0} + 1 ))
   821	  CELL_ATTEMPTS[$i]=$attempts
   822	  CUR_W="$w"; CUR_FLAG="$flag"
   823	  local order pair=yes rowA="" rowB="" arm aname init rid run
   824	  # ABBA: the arm order alternates by slot, so a monotonic drift cannot favour one arm.
   825	  if (( slot % 2 )); then order="A B"; else order="B A"; fi
   826	  for arm in $order; do
   827	    if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
   828	    rid="${aname}_s${slot}a${attempts}"; run="${SESSION_TAG}_${cell}_${rid}"
   829	    if [[ "$aname" == srcinit ]]; then arm_srcinit "$sh" "$dh" "$run"; else arm_destinit "$sh" "$dh" "$run"; fi
   830	    [[ "$RUN_VALID" == yes ]] || pair=no
   831	    local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_SETTLED,$RUN_FILES,$RUN_BYTES,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
   832	    if [[ "$arm" == A ]]; then rowA="$row"; else rowB="$row"; fi
   833	    log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (fsync ${RUN_FLUSH}ms, settle ${RUN_SETTLED}ms, $RUN_FILES files, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
   834	  done
   835	  echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
   836	  if [[ "$pair" == yes ]]; then
   837	    CELL_VALID[$i]=$(( ${CELL_VALID[$i]:-0} + 1 ))
   838	    return 0
   839	  fi
   840	  log "  $cell: pair at slot $slot VOIDED"
   841	  return 1
   842	}
   843	
   844	run_all_cells() {
   845	  local slot i cell sh dh w flag max=$(( 2 * RUNS )) n=${#CELL_TABLE[@]}
   846	  for (( i = 0; i < n; i++ )); do CELL_VALID[$i]=0; CELL_ATTEMPTS[$i]=0; done
   847	  for (( slot = 1; slot <= RUNS; slot++ )); do
   848	    log "=== SLOT $slot / $RUNS (every cell takes one pair before any cell takes the next) ==="
   849	    for (( i = 0; i < n; i++ )); do
   850	      read -r cell sh dh w flag <<<"${CELL_TABLE[$i]}"
   851	      # a voided pair is retried IN PLACE, so the cell stays in step with its siblings
   852	      while (( ${CELL_ATTEMPTS[$i]:-0} < max )); do
   853	        if run_one_pair "$i" "$cell" "$sh" "$dh" "$w" "${flag:-}" "$slot"; then break; fi
   854	      done
   855	    done
   856	  done
   857	  for (( i = 0; i < n; i++ )); do
   858	    read -r cell sh dh w flag <<<"${CELL_TABLE[$i]}"
   859	    if (( ${CELL_VALID[$i]:-0} < RUNS )); then
   860	      echo "$cell,${CELL_ATTEMPTS[$i]},no" >> "$META"
   861	      log "  $cell INCOMPLETE: ${CELL_VALID[$i]}/$RUNS valid pairs"
   862	    else
   863	      echo "$cell,${CELL_ATTEMPTS[$i]},yes" >> "$META"
   864	    fi
   865	  done
   866	}
   867	
   868	SESSION_VOID_REASON=""
   869	# The end-load is a CONDITION OF THE SESSION, not a log line. A mid-session load
   870	# spike is exactly the contamination the start gate exists to prevent, and until now
   871	# it could not void anything: the code logged `load1 (end)` and computed a verdict
   872	# anyway, while the comment claimed a session "can void on it" (round-3 grok, HIGH —
   873	# a doc claim the code did not honour, which is the defect class this whole review
   874	# exists to kill).
   875	end_load_gate() {

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-p96h9B0d' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-BKNHyT1s' (errno=Operation not permitted)
   208	### The four cell states — mutually exclusive and exhaustive BY CONSTRUCTION
   209	
   210	They partition the CI's position relative to the thresholds. **There is no label here for
   211	a new case to walk past**, which is precisely what went wrong seven rounds running.
   212	
   213	| state | condition |
   214	|---|---|
   215	| **EFFECT** | `CI_lo >= T_pos + B` — destination-initiated is slower, by at least T |
   216	| **INVERTED** | `CI_hi <= T_neg − B` — source-initiated is slower, by at least T |
   217	| **NONE** | **the FULL RANGE** lies inside `(T_neg, T_pos)` — *every* pair, not just the median. An effect of size T is **EXCLUDED** (equivalence) |
   218	| **UNCLEAR** | anything else — the CI spans a threshold; the rig cannot answer |
   219	
   220	**A NULL IS JUDGED ON THE RANGE, AN EFFECT ON THE CI — and that asymmetry is the point
   221	(round-8, codex, BLOCKER).** The ≥95% CI is the *narrowest* valid interval, so at n>8 it
   222	**trims outliers**; a **bimodal** arm then yields a *narrow median CI* and a **false null**
   223	(codex drove `CI = [1,1]` from modes at ±110). **An equivalence claim must never be
   224	reachable by trimming away the very pairs that contradict it.** A *positive* claim may use
   225	the CI: pairs clearing T is evidence, and a few stragglers do not undo it.
   226	
   227	*This is also why bimodality needs no special branch — it cannot hide from the range. The
   228	previous rule hand-coded an `UNSTABLE` override for exactly this, and got it wrong.*
   229	
   230	### The controls are a PRECONDITION, at HALF the threshold
   231	
   232	**Every control must be `NONE` at `T/2`.** Half, because certifying a control with the
   233	very number that *defines* the effect is incoherent: it would let the gRPC control carry
   234	all but 1 ms of P1 while we call the rig clean (round 6 drove exactly that).
   235	
   236	**If any control fails, NO verdict about the measurand is read — not a reproduction, and
   237	not a null.** Uncertainty about a rig-wide confound is not evidence that the confound is
   238	absent, and P1's whole claim is that the effect is *specific* to TCP × mixed.
   239	
   240	**And "clean" is not "zero" (round-8, codex, BLOCKER).** A control sitting at `+49` with
   241	`T/2 = 50` certifies — but *that 49 ms of arm bias may be riding in the measurand too*, so a
   242	measurand effect of exactly `T` could be half real and half rig. The bias the controls **fail
   243	to exclude** is therefore carried into the measurand's thresholds:
   244	
   245	    B = max over clean controls of the largest |CI bound|
   246	    an EFFECT must clear   T + B     (the bias could be INFLATING it)
   247	    a NULL must fit inside T − B     (the bias could be MASKING an effect)
   248	
   249	If the controls are genuinely clean, `B` is a few ms and this barely moves. If they are
   250	marginal, it bites — which is the point.
   251	
   252	### The controls are CONTEMPORANEOUS with the measurands
   253	
   254	The schedule is **slot-major**: within slot *i*, **every** cell takes one ABBA pair, in a
   255	fixed registered order, before any cell takes slot *i+1*. All six cells therefore span the
   280	- **The 1.10 bar takes NO part in inference.** It is computed on the *marginal medians*,
   281	  reported in every row as the project's **acceptance** criterion, and never consulted.
   282	  The marginal and paired statistics can disagree in **direction and magnitude**, and
   283	  every attempt to let one stand in for the other produced a false verdict.
   284	- **The sign test is reported, not decided on.** At n=8 the CI already implies it
   285	  (`CI_lo >= T > 0` means *every* pair clears T), so making it a second gate only added
   286	  an interaction to get wrong. It is printed per cell.
   287	- **No `UNSTABLE` / `PARTIAL` / `BAR-FAIL-INCONSISTENT` / `UNDERPOWERED` branches, and no
   288	  precedence stack.** A bimodal arm **widens the CI**, and a wide CI lands in `UNCLEAR` —
   289	  which is exactly what those branches were hand-coding. Every run of every arm is still
   290	  printed in `summary.csv`, so bimodality remains visible to the reader.
   291	- **A real but SUB-THRESHOLD effect is reported, not buried.** A cell can be `NONE` and
   292	  still carry a consistent effect below T (e.g. 99 ms on a 1000 ms arm, on 7 of 8 pairs).
   293	  The verdict prints a NOTE naming it. It does not change the outcome — the threshold was
   294	  registered in advance — but it is **not nothing**, and it does not hide inside the word
   295	  "none".
   296	
   297	### There is NO escalation. `RUNS = 8`, and only 8.
   298	
   299	The `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the **full
   300	range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR` rig,
   301	nor certify a marginal control; and if you already have an `EFFECT`, you do not need them.
   302	
   303	**A noisy rig is fixed by a quieter rig, not by more pairs — and `UNCLEAR` says exactly
   304	that.** Removing it also removes its entire p-hacking guard surface (a "once" marker, a
   305	verdict check, a data-hash burn), none of which now has to be right.
   306	
   307	### The registered constants are PINNED IN CODE
   308	
   309	`DELTA_REF_MS`, `SETTLE_MS`, `LOAD_MAX`, `DRAIN_MBPS` and the rest are **literals** in
   310	both the harness and the engine. The harness **refuses to start** if one is merely
   311	*present* in the environment. *(They were once `${VAR:-default}`, and `DELTA_REF_MS=240`
   312	turned a void into a null — i.e. the rule could be retuned from the command line, after
   313	the data existed, in the direction of the answer you want. **That is not a
   314	pre-registration.**)* To change one: amend this document and put it back through review.
   315	
   316	### The guard test
   317	
   318	`scripts/otp12pf_mac_verdict_test.py`: **26 cases — every one a defect a reviewer actually
   319	drove out of a previous revision** — each **mutation-proven** (reverting that fix in a copy
   320	of the engine makes exactly that case fail: **9/9 mutations killed**), plus a 300-input
   321	fuzz over the measurand **and** the controls. It runs at preflight, cases *and* mutations;
   322	a vacuous guard refuses the run.
   323	
   324	## The instrument — what round 3 found, and what now guards it
   325	
   326	**THE TIMER WAS MEASURING FSYNC NOISE (round-3 BLOCKER; I introduced it in the
   327	rework that fixed round 2).** The transfer timer captured `time.monotonic()` in
   328	**two separate `python3 -c` processes** and subtracted them. On macOS that clock is
   329	**process-relative**. Measured on this rig: a **1000 ms sleep read as −1 ms on
   330	nagatha and 2 ms on q** — *negative*. Every `ms` row would have been ≈ `fsync_ms`

exec
/bin/zsh -lc "bash -c 'VERDICT_CELLS=a CONTROL_CELLS=b REGISTERED_CELLS=a,b REQUIRED_PAIRS=16 python3 <(git show 8830fda:scripts/otp12pf_mac_verdict.py) /dev/null /dev/null /dev/null /dev/null /dev/null /dev/null 2>&1 | grep REFUSING' " in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2NoBkcgQ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-EMcznERk' (errno=Operation not permitted)
REFUSING: REQUIRED_PAIRS=16 is not registered (8,).

exec
/bin/zsh -lc "bash -c 'emit_runs(){ echo cell,arm,build,initiator,run,ms,flush_ms,settled_ms,files,bytes,exit,drain,cold,valid; for c in nq_tcp_mixed qn_tcp_mixed nq_grpc_mixed qn_grpc_mixed nq_tcp_large qn_tcp_large; do for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16; do s=1000; d=0; [[ \""'$c" == nq_tcp_mixed || "$c" == qn_tcp_mixed ]] && { if (( i <= 3 )); then d=-500; else d=200; fi; }; echo "$c,srcinit,x,h,$i,$s,0,250,1,1,0,drained_1x2s,cold,yes"; echo "$c,destinit,x,h,$i,$((s+d)),0,250,1,1,0,drained_1x2s,cold,yes"; done; done; }; emit_meta(){ echo cell,pairs_attempted,complete; for c in nq_tcp_mixed qn_tcp_mixed nq_grpc_mixed qn_grpc_mixed nq_tcp_large qn_tcp_large; do echo "$c,16,yes"; done; }; VERDICT_CELLS=nq_tcp_mixed,qn_tcp_mixed CONTROL_CELLS=nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large REGISTERED_CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large REQUIRED_PAIRS=8 python3 <(git show 8830fda:scripts/otp12pf_mac_verdict.py) <(emit_runs) <(emit_meta) /dev/null /dev/null /dev/null /dev/null | grep -E "''^(SESSION VERDICT|  nq_tcp_mixed|  qn_tcp_mixed)"'"' " in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-0DuSmn3j' (errno=Operation not permitted)
python3: error: couldn't create cache file '/tmp/xcrun_db-eZOIszsS' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ko5LhcYu' (errno=Operation not permitted)
python3: error: couldn't create cache file '/tmp/xcrun_db-8fbpWXlO' (errno=Operation not permitted)
SESSION VERDICT: REPRODUCES
  nq_tcp_mixed   EFFECT   ctrl=-        D= +200ms CI=[ +200, +200] range=[ -500, +200] T=100ms B=  0ms  ratio=1.200 bar=FAIL  sign_p=0.021 (13/16)
  qn_tcp_mixed   EFFECT   ctrl=-        D= +200ms CI=[ +200, +200] range=[ -500, +200] T=100ms B=  0ms  ratio=1.200 bar=FAIL  sign_p=0.021 (13/16)

exec
/bin/zsh -lc "if rg -n '\\"'$'"\\{["'^}]+'"\\"'^'"\\"'^'"\\}|\\b(mapfile|readarray)\\b|declare[[:space:]]+-A|local[[:space:]]+-A' scripts/bench_otp12pf_mac.sh; then exit 1; else echo 'no bash>3.2 constructs requested by review'; fi" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
no bash>3.2 constructs requested by review

codex
Checked-out HEAD is `7d72f77`, not `8830fda`; I reviewed both. The first finding affects `8830fda` but is already fixed at current HEAD. The remaining findings still affect current HEAD.

## Findings

- `8830fda:scripts/otp12pf_mac_verdict.py:193` — **BLOCKER** — Completeness was `len(d) < 8`, so a 16-pair session was graded. Its CI trimmed three contradictory pairs and produced `REPRODUCES`. Current [line 200](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:200) correctly requires exactly eight.

- [scripts/otp12pf_mac_verdict.py:220](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:220) — **BLOCKER** — `B` transports raw milliseconds between controls with potentially different baselines. Identical 4.9% arm bias in every cell can certify every control yet produce `REPRODUCES` in slower measurands. Neither median nor CI-versus-range fixes this; the rule needs an explicitly justified additive-bias model or a normalized carry. Opposite-sign bias and increasing `B` are otherwise conservative.

- [scripts/bench_otp12pf_mac.sh:492](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:492) — **HIGH** — The clean-instrument gate checks `$SELF` relative to `REPO_ROOT`, but `$SELF` is not canonicalized. Running from `scripts/` makes it `bench_otp12pf_mac.sh`, an unmatched pathspec, so an edited harness passes the “reviewed instrument” check.

- [scripts/bench_otp12pf_mac.sh:659](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:659) — **HIGH** — The drain validator accepts any string containing only digits and periods. A non-number such as `.` becomes zero in `awk` and certifies the disk as drained. This is the next recurring “gate executes but cannot fail correctly” defect.

- [scripts/bench_otp12pf_mac.sh:806](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:806) — **HIGH** — Scheduling is slot-major, but cell order is fixed with both measurands always before every control. A recurring slot-position or predecessor/carry-over artifact is therefore perfectly confounded with cell type for all eight pairs. A one-off transient only causes `UNCLEAR`, but a harness-induced recurring artifact can manufacture a confident effect without reaching controls.

- [PREREGISTRATION.md:5](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:5) — **LOW** — The “whole rule” still defines NONE using the CI, line 245 says `B` uses CI bounds while current code uses range bounds, and lines 327–330 claim 26 cases/9 mutations. `8830fda` actually has 29/10; current HEAD has 31/11.

The CI/range asymmetry itself is sound at registered `n=8`: CI equals range, so one contrary pair blocks both EFFECT and NONE. The direct-rule replacement is therefore reasonable. ABBA row pairing, retry/void accounting, exhausted-cell metadata, and Bash 3.2 syntax checked out; an exhausted cell makes the session incomplete without corrupting sibling cells. Current HEAD also refuses every pair count except eight.

## Reproductions

Requested commit accepting a long, contradictory session:

```text
# measurands: d=[-500]*3+[200]*13; controls: d=[0]*16
$ REQUIRED_PAIRS=8 python3 <(git show 8830fda:scripts/otp12pf_mac_verdict.py) \
    <(emit_runs) <(emit_meta) /dev/null /dev/null /dev/null /dev/null
SESSION VERDICT: REPRODUCES
nq_tcp_mixed EFFECT ... CI=[+200,+200] range=[-500,+200] ... (13/16)
```

Current HEAD correctly returns `INCOMPLETE` for the same CSV.

Raw-millisecond `B` failing to recognize the same proportional bias:

```text
# controls: src=1000,d=49; measurands: src=10000,d=490; all eight pairs
SESSION VERDICT: REPRODUCES
nq_grpc_mixed NONE   D=+49ms  ratio=1.049 bar=PASS
nq_tcp_mixed  EFFECT D=+490ms T=230ms B=49ms ratio=1.049 bar=PASS
```

Provenance path bypass:

```text
$ git diff --quiet 8830fda^ 8830fda -- bench_otp12pf_mac.sh; echo $?
0
$ git diff --quiet 8830fda^ 8830fda -- scripts/bench_otp12pf_mac.sh; echo $?
1
```

Drain parser fault injection using the actual function:

```text
$ # mocked iostat emits: "disk0 1 ."
$ drain_host n
drained_1x2s
```

The four direct rule tests and the randomized `n=8 CI == range` identity passed. Full cases/mutations could not run in this read-only sandbox because `tempfile.mkdtemp()` had no writable directory. `bash -n` passed under GNU Bash 3.2.57, with no associative arrays, `${x^^}`, or `mapfile`.

**VERDICT: NOT READY TO RUN**
tokens used
159,549
Checked-out HEAD is `7d72f77`, not `8830fda`; I reviewed both. The first finding affects `8830fda` but is already fixed at current HEAD. The remaining findings still affect current HEAD.

## Findings

- `8830fda:scripts/otp12pf_mac_verdict.py:193` — **BLOCKER** — Completeness was `len(d) < 8`, so a 16-pair session was graded. Its CI trimmed three contradictory pairs and produced `REPRODUCES`. Current [line 200](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:200) correctly requires exactly eight.

- [scripts/otp12pf_mac_verdict.py:220](/Users/michael/Dev/blit_v2/scripts/otp12pf_mac_verdict.py:220) — **BLOCKER** — `B` transports raw milliseconds between controls with potentially different baselines. Identical 4.9% arm bias in every cell can certify every control yet produce `REPRODUCES` in slower measurands. Neither median nor CI-versus-range fixes this; the rule needs an explicitly justified additive-bias model or a normalized carry. Opposite-sign bias and increasing `B` are otherwise conservative.

- [scripts/bench_otp12pf_mac.sh:492](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:492) — **HIGH** — The clean-instrument gate checks `$SELF` relative to `REPO_ROOT`, but `$SELF` is not canonicalized. Running from `scripts/` makes it `bench_otp12pf_mac.sh`, an unmatched pathspec, so an edited harness passes the “reviewed instrument” check.

- [scripts/bench_otp12pf_mac.sh:659](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:659) — **HIGH** — The drain validator accepts any string containing only digits and periods. A non-number such as `.` becomes zero in `awk` and certifies the disk as drained. This is the next recurring “gate executes but cannot fail correctly” defect.

- [scripts/bench_otp12pf_mac.sh:806](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:806) — **HIGH** — Scheduling is slot-major, but cell order is fixed with both measurands always before every control. A recurring slot-position or predecessor/carry-over artifact is therefore perfectly confounded with cell type for all eight pairs. A one-off transient only causes `UNCLEAR`, but a harness-induced recurring artifact can manufacture a confident effect without reaching controls.

- [PREREGISTRATION.md:5](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:5) — **LOW** — The “whole rule” still defines NONE using the CI, line 245 says `B` uses CI bounds while current code uses range bounds, and lines 327–330 claim 26 cases/9 mutations. `8830fda` actually has 29/10; current HEAD has 31/11.

The CI/range asymmetry itself is sound at registered `n=8`: CI equals range, so one contrary pair blocks both EFFECT and NONE. The direct-rule replacement is therefore reasonable. ABBA row pairing, retry/void accounting, exhausted-cell metadata, and Bash 3.2 syntax checked out; an exhausted cell makes the session incomplete without corrupting sibling cells. Current HEAD also refuses every pair count except eight.

## Reproductions

Requested commit accepting a long, contradictory session:

```text
# measurands: d=[-500]*3+[200]*13; controls: d=[0]*16
$ REQUIRED_PAIRS=8 python3 <(git show 8830fda:scripts/otp12pf_mac_verdict.py) \
    <(emit_runs) <(emit_meta) /dev/null /dev/null /dev/null /dev/null
SESSION VERDICT: REPRODUCES
nq_tcp_mixed EFFECT ... CI=[+200,+200] range=[-500,+200] ... (13/16)
```

Current HEAD correctly returns `INCOMPLETE` for the same CSV.

Raw-millisecond `B` failing to recognize the same proportional bias:

```text
# controls: src=1000,d=49; measurands: src=10000,d=490; all eight pairs
SESSION VERDICT: REPRODUCES
nq_grpc_mixed NONE   D=+49ms  ratio=1.049 bar=PASS
nq_tcp_mixed  EFFECT D=+490ms T=230ms B=49ms ratio=1.049 bar=PASS
```

Provenance path bypass:

```text
$ git diff --quiet 8830fda^ 8830fda -- bench_otp12pf_mac.sh; echo $?
0
$ git diff --quiet 8830fda^ 8830fda -- scripts/bench_otp12pf_mac.sh; echo $?
1
```

Drain parser fault injection using the actual function:

```text
$ # mocked iostat emits: "disk0 1 ."
$ drain_host n
drained_1x2s
```

The four direct rule tests and the randomized `n=8 CI == range` identity passed. Full cases/mutations could not run in this read-only sandbox because `tempfile.mkdtemp()` had no writable directory. `bash -n` passed under GNU Bash 3.2.57, with no associative arrays, `${x^^}`, or `mapfile`.

**VERDICT: NOT READY TO RUN**
