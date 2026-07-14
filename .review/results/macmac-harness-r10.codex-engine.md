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
session id: 019f6186-2c46-7b00-abe8-77b059fc6e56
--------
user
Correctness review of a statistical decision rule for a performance benchmark. Read ONLY these three files (do not read the bash harness):

- scripts/otp12pf_mac_verdict.py       the decision rule
- scripts/otp12pf_mac_verdict_test.py  32 cases + 12 mutations + 5 direct rule checks
- docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md   rev 10, the spec it must implement

Context: the rule decides whether a performance finding (destination-initiated transfers pay ~25-38% more than source-initiated ones) reproduces on a pair of Macs. NO DATA HAS BEEN TAKEN. Nine review rounds have found ~100 defects, all accepted; four of the last five serious ones were in this rule rather than in the measurement, so it was rewritten and simplified.

THE RULE:
  per cell: paired ABBA differences d_i = destinit_i - srcinit_i (n is EXACTLY 8), their
  median, one exact order-statistic CI (coverage >= 95%; at n=8 that IS [min,max]), and the
  full RANGE.
  T_pos = min(src_median/10, 230ms); T_neg = -min(src_median/11, 230ms).
  B = the arm bias the CLEAN controls could not rule out: taken from each control's full
      RANGE, as a FRACTION of its arm, scaled to the cell it is applied to.
    EFFECT    CI_lo >= T_pos + B
    INVERTED  CI_hi <= T_neg - B
    NONE      the FULL RANGE lies inside (T_neg + B, T_pos - B)
    UNCLEAR   otherwise
  Every control must be NONE at T/2, or no measurand verdict is read at all.
  The 1.10 ratio bar is reported and takes no part in inference. The sign test is reported.

The question is: can this rule produce a confident but WRONG verdict? Specifically:

1. Are the four states mutually exclusive and exhaustive? Find an input landing outside them or in two at once.
2. Is B sound in BOTH directions? Can it license an effect that is mostly rig bias? Can it make any verdict EASIER (it should only ever harden)? Can B exceed T, making a null impossible? Can B be zero when it should not be?
3. The asymmetry: a NULL is judged on the full RANGE, an EFFECT on the CI. At n=8 these coincide. Does anything depend on them differing? Is the EFFECT side now the weaker one?
4. n must be EXACTLY 8 (at larger n the CI trims outliers and a bimodal arm yields a narrow CI and a false null). Is that enforced everywhere it matters? Does anything else silently assume n=8 without saying so?
5. Does the code implement the pre-registration exactly? Any claim in the doc the code does not do, or behaviour the code has that the doc does not register?
6. Is the guard non-vacuous? 12 mutations, judged by whether the named case FAILS. Which fixes have NO mutation? Two guards are asserted directly rather than by mutation, because at n=8 the CI and the RANGE are the same two numbers and no synthetic session can tell them apart -- is that reasoning correct?

You may run python to evaluate the rule on synthetic inputs.

Output a concise markdown findings list (file:line, severity BLOCKER/HIGH/MEDIUM/LOW, rationale), then a final VERDICT line: READY or NOT READY. Be concise; do not invoke skills.
codex
I’ll audit the specification, implementation, and guards side by side, then run only the named Python test module and targeted synthetic checks if needed. I’ll keep all reads confined to the three files you listed.
exec
/bin/zsh -lc 'nl -ba docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'nl -ba scripts/otp12pf_mac_verdict_test.py' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'nl -ba scripts/otp12pf_mac_verdict.py' in /Users/michael/Dev/blit_v2
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
   220	#     B = the arm asymmetry the controls could not rule out, as a FRACTION OF THE ARM,
   221	#         scaled to the cell it is applied to. Taken from each control's full RANGE (not its
   222	#         CI: the CI is an interval for the MEDIAN and it TRIMS, and a bound on what the rig
   223	#         might be carrying must never be computed by trimming). Relative, not raw ms:
   224	#         the controls run different fixtures at different speeds.
   225	#     an EFFECT must clear  T + B     (bias could be INFLATING it)
   226	#     a NULL   must fit in  T - B     (bias could be MASKING an effect)
   227	#
   228	# If the controls are genuinely clean, B is a few ms and this barely moves. If they are
   229	# marginal, it bites -- which is the point.
   230	dirty = []
   231	B_frac = 0.0          # RELATIVE, not raw milliseconds
   232	for c in CONTROLS:
   233	    x = cell.get(c, {})
   234	    if x.get("state") == "INCOMPLETE":
   235	        continue
   236	    c_pos, c_neg = thresholds(x["src"], 0.5)
   237	    x["ctrl_state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], c_pos, c_neg)
   238	    x["ctrl_T"] = c_pos
   239	    if x["ctrl_state"] != "NONE":
   240	        dirty.append(c)
   241	    elif x["src"]:
   242	        # B IS A FRACTION OF THE ARM, NOT A MILLISECOND COUNT (round-9 codex, BLOCKER).
   243	        # The controls run on DIFFERENT fixtures and therefore different arm speeds: the
   244	        # same 4.9% arm bias is 122 ms on a 2500 ms large-file control and 24 ms on a fast
   245	        # one. Carrying raw ms across them OVER-penalises a measurand slower than the
   246	        # control and UNDER-penalises one that is faster -- and the second direction is the
   247	        # dangerous one: a 4.9% bias measured on a fast control would license a measurand
   248	        # effect that is mostly rig. Take the bias as a FRACTION and scale it to whatever
   249	        # arm it is being applied to.
   250	        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])
   251	
   252	# ---- pass 3: grade the measurands, against thresholds widened by the control bias -----
   253	for c in MEASURANDS:
   254	    x = cell.get(c, {})
   255	    if x.get("state") == "INCOMPLETE":
   256	        continue
   257	    t_pos, t_neg = thresholds(x["src"])
   258	    B = B_frac * x["src"]                    # the control bias, on THIS cell's arm
   259	    x["T"] = t_pos
   260	    x["B"] = B
   261	    x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1],
   262	                          t_pos + B, t_neg - B)          # an EFFECT must clear T + B
   263	    if x["state"] == "NONE":
   264	        # ...and a NULL must survive the TIGHTER bound: bias could be masking an effect.
   265	        if not (t_neg + B < x["rng"][0] and x["rng"][1] < t_pos - B):
   266	            x["state"] = "UNCLEAR"
   267	
   268	# Controls also carry a state for the report; measurands carry a ctrl_state for symmetry.
   269	for c in cell:
   270	    x = cell[c]
   271	    if x.get("state") == "INCOMPLETE":
   272	        continue
   273	    if "state" not in x:                                  # a control: report its own state
   274	        t_pos, t_neg = thresholds(x["src"])
   275	        x["T"] = t_pos
   276	        x["B"] = 0.0
   277	        x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], t_pos, t_neg)
   278	    x.setdefault("ctrl_state", "-")
   279	
   280	# ---- outputs -----------------------------------------------------------------------
   281	with open(sum_p, "w") as f:
   282	    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,voided,runs\n")
   283	    for (c, a) in sorted(by):
   284	        v = by[(c, a)]
   285	        f.write("%s,%s,%d,%d,%d,%d,%d,%s\n" % (c, a, med(v), sum(v) // len(v), min(v),
   286	                                               max(v), voided.get((c, a), 0),
   287	                                               " ".join(map(str, v))))
   288	
   289	with open(pair_p, "w") as f:
   290	    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,range_lo,range_hi,"
   291	            "coverage,T_ms,B_ms,sign_p,k_pos,state,control_state\n")
   292	    for c in sorted(cell):
   293	        x = cell[c]
   294	        if x["state"] == "INCOMPLETE":
   295	            f.write("%s,%d,,,,,,,,,,,,,,,INCOMPLETE,\n" % (c, x["n"]))
   296	            continue
   297	        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%d,%d,%.4f,%d,%d,%.4f,%d/%d,%s,%s\n" % (
   298	            c, x["n"], x["src"], x["dst"], x["ratio"], x["bar"], x["D"],
   299	            x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], x["cov"],
   300	            round(x["T"]), round(x.get("B", 0)), x["p"], x["k"], x["n"],
   301	            x["state"], x["ctrl_state"]))
   302	
   303	with open(ver_p, "w") as f:
   304	    f.write("comparison,kind,lhs_ms,rhs_ms,ratio,bar\n")
   305	    for c in sorted(cell):
   306	        x = cell[c]
   307	        if x["state"] == "INCOMPLETE":
   308	            f.write("%s,invariance,,,,INCOMPLETE\n" % c)
   309	        else:
   310	            f.write("%s,invariance,%d,%d,%.3f,%s\n"
   311	                    % (c, x["src"], x["dst"], x["ratio"], x["bar"]))
   312	
   313	# ---- THE SESSION VERDICT -----------------------------------------------------------
   314	incomplete = [c for c in REGISTERED if cell.get(c, {}).get("state") == "INCOMPLETE"]
   315	m = {c: cell[c]["state"] for c in MEASURANDS if not incomplete}
   316	
   317	if incomplete:
   318	    verdict = "INCOMPLETE"
   319	    why = ("cells short of their %d pairs, or with a CI below the registered %.0f%% "
   320	           "coverage: %s. No verdict is read." % (PAIRS, 100 * MIN_COVERAGE,
   321	                                                  ", ".join(incomplete)))
   322	elif SESSION_VOID:
   323	    verdict = "RIG-VOID"
   324	    why = "the harness voided this session: %s. No verdict is read." % SESSION_VOID
   325	elif dirty:
   326	    verdict = "CONTROLS-NOT-CLEAN"
   327	    why = ("control cell(s) are not free of an arm asymmetry at T/2: %s. P1 is claimed "
   328	           "TCP-only and mixed-only; if the gRPC/large controls may be carrying the same "
   329	           "asymmetry, then NEITHER a reproduction NOR a null is readable off this rig. "
   330	           "There is no escalation: a noisy rig is fixed by a QUIETER RIG, not more pairs."
   331	           % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d], T/2=%d)"
   332	                       % (c, cell[c]["ctrl_state"], cell[c]["D"], cell[c]["ci"][0],
   333	                          cell[c]["ci"][1], round(cell[c]["T"] / 2))
   334	                       for c in dirty))
   335	elif "EFFECT" in m.values() and "INVERTED" in m.values():
   336	    verdict = "MIXED"
   337	    why = ("one direction shows the effect and the other INVERTS it -- a host x role "
   338	           "interaction this rig cannot decompose. Inconclusive for the question.")
   339	elif "EFFECT" in m.values():
   340	    verdict = "REPRODUCES"
   341	    why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: it shows "
   342	           "P1 CAN occur macOS<->macOS, so it is not waivable as 'Windows residue'. It "
   343	           "does NOT establish a platform-general cost, does NOT name the mechanism, "
   344	           "does NOT kill H1 (the code H1 accuses runs here too), and leaves macOS/APFS "
   345	           "and host x role explanations OPEN."
   346	           % ", ".join(c for c, s in m.items() if s == "EFFECT"))
   347	elif "INVERTED" in m.values():
   348	    verdict = "INVERTED"
   349	    why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank it as "
   350	           "'P1 absent'." % ", ".join(c for c, s in m.items() if s == "INVERTED"))
   351	elif all(s == "NONE" for s in m.values()):
   352	    verdict = "DOES-NOT-REPRODUCE"
   353	    why = ("both TCP-mixed cells EXCLUDE an effect of size T, and every control is clean "
   354	           "at T/2 -- a genuine equivalence result. Scoped to THIS pair: P1 did not "
   355	           "reproduce macOS<->macOS. That is CONSISTENT with 'the Windows peer is "
   356	           "required' but does NOT prove it -- it could equally be a property of these "
   357	           "two machines, their disks, or this macOS version.")
   358	else:
   359	    verdict = "UNCLEAR"
   360	    why = ("the CI spans the threshold in: %s. The rig could not resolve an effect of "
   361	           "size T either way -- this is NOT 'P1 vanishes'. Fix the rig, do not add pairs."
   362	           % ", ".join(c for c, s in m.items() if s == "UNCLEAR"))
   363	
   364	out = ["SESSION VERDICT: %s" % verdict, "", why, "",
   365	       "Per cell. T = min(srcinit_median/10, %d ms). Controls must be NONE at T/2, and B is"
   366	       % DELTA_REF,
   367	       "the arm bias they could NOT exclude: an EFFECT must clear T+B, a NULL must fit in T-B."]
   368	for c in sorted(cell):
   369	    x = cell[c]
   370	    if x["state"] == "INCOMPLETE":
   371	        out.append("  %-14s INCOMPLETE (%d pairs)" % (c, x["n"]))
   372	        continue
   373	    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] range=[%+5d,%+5d] "
   374	               "T=%3dms B=%3dms  ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
   375	               % (c, x["state"], x["ctrl_state"], x["D"], x["ci"][0], x["ci"][1],
   376	                  x["rng"][0], x["rng"][1], round(x["T"]), round(x.get("B", 0)),
   377	                  x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
   378	# A cell can be NONE (an effect of size T is excluded) and STILL carry a real, consistent
   379	# effect BELOW T -- e.g. 99 ms on a 1000 ms arm, one millisecond under the threshold, on
   380	# 7 of 8 pairs. That is not a contradiction and it does not change the verdict, but it
   381	# must not hide inside the word "none". Reported, never decided on.
   382	subthreshold = [c for c in sorted(cell)
   383	                if cell[c]["state"] == "NONE" and cell[c]["p"] < 0.05 and cell[c]["D"]]
   384	if subthreshold:
   385	    out += ["",
   386	            "NOTE -- a real but SUB-THRESHOLD effect is present in: %s."
   387	            % ", ".join("%s (D=%+dms, T=%dms, sign_p=%.3f)"
   388	                        % (c, cell[c]["D"], round(cell[c]["T"]), cell[c]["p"])
   389	                        for c in subthreshold),
   390	            "These cells are consistent in direction but smaller than the registered",
   391	            "threshold, so they are not a reproduction of P1. They are NOT nothing."]
   392	
   393	out += ["",
   394	        "A NULL (NONE) is judged on the full RANGE -- EVERY pair inside the bound -- not on",
   395	        "the median CI, which at n>8 would TRIM the outliers that contradict it. An EFFECT",
   396	        "uses the CI. That is why bimodality needs no special branch: it cannot hide from",
   397	        "the range.",
   398	        "",
   399	        "The bar/ratio columns are the project's ACCEPTANCE criterion. They are reported",
   400	        "and take NO part in this verdict, which is decided only by the paired CI against",
   401	        "T. sign_p is reported, not decided on. All runs are in summary.csv -- read them.",
   402	        "",
   403	        "Computed from the pre-registered rule. It declares nothing beyond it."]
   404	
   405	open(sess_p, "w").write("\n".join(out) + "\n")
   406	print("\n".join(out))

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
    93	     dict(measurand_d=[-100, -50, 0, 50, 100, 200, 220, 240], src=2500, control_d=[0] * 8),
    94	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
    95	
    96	    ("codex r2: the inverting threshold is -src/11, not -src/10 (CI [-190,0] @ 2000)",
    97	     dict(measurand_d=[-190, -190, 0, 0, 0, 0, 0, 0], src=2000, control_d=[0] * 8),
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
   183	    ("codex r9: B is RELATIVE — a 4.9% bias on a FAST control must not under-penalise a slower measurand",
   184	     dict(measurand_d=[130] * 8, src=1000,
   185	          control_d=[24] * 8, control_src=500),
   186	     "UNCLEAR", "REPRODUCES"),
   187	
   188	    ("grok r9: a null must also survive the TIGHTER bound (bias could be MASKING an effect)",
   189	     dict(measurand_d=[60] * 8, src=1000, control_d=[49] * 8),
   190	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   191	
   192	    ("codex r8: ...and the same effect IS one once the rig is bias-free",
   193	     dict(measurand_d=[105] * 8, src=1000, control_d=[5] * 8),
   194	     "REPRODUCES", "UNCLEAR"),
   195	
   196	    ("source-initiated slower is INVERTED, never 'P1 absent'",
   197	     dict(measurand_d=[-300, -310, -320, -330, -340, -350, -360, -370], src=1000),
   198	     "INVERTED", None),
   199	
   200	    ("one direction reproduces, the other inverts -> MIXED",
   201	     dict(measurand_d=[0] * 8, src=1000,
   202	          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
   203	                    "qn_tcp_mixed": ([-300, -310, -320, -330, -340, -350, -360, -370], 1000)}),
   204	     "MIXED", "REPRODUCES"),
   205	
   206	    ("a clean one-direction reproduction is NOT masked by a noisy sibling",
   207	     dict(measurand_d=[0] * 8, src=1000,
   208	          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
   209	                    "qn_tcp_mixed": ([-20, 300, 310, 320, 330, 340, 350, 360], 1000)}),
   210	     "REPRODUCES", "UNCLEAR"),
   211	
   212	    ("codex r8: a bimodal arm cannot hide from the RANGE (a null is judged on every pair)",
   213	     dict(measurand_d=[-110, 0, -110, 110, 110, 0, -110, 0], src=730,
   214	          control_d=[0] * 8),
   215	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   216	
   217	    ("a null the rig could not have SEEN is UNCLEAR, not a null",
   218	     dict(measurand_d=[-400, -300, -100, 0, 0, 100, 300, 400], src=2000),
   219	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   220	
   221	    # --- integrity ---------------------------------------------------------------
   222	    ("a missing registered cell is INCOMPLETE, never filtered away",
   223	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   224	          drop_cells=("qn_tcp_mixed",)),
   225	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   226	
   227	    ("grok r3: n=1 with complete=yes must not grade at 0% CI coverage",
   228	     dict(measurand_d=[0], src=2000, control_d=[5], control_src=1000),
   229	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   230	
   231	    ("grok r3: a harness-detected session void (end-load) refuses a verdict",
   232	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   233	          void_reason="end-load on q is 9.1 (> 3.0)"),
   234	     "RIG-VOID", "DOES-NOT-REPRODUCE"),
   235	
   236	    ("codex r5: DELTA_REF_MS is PINNED -- the rule is not tunable from the environment",
   237	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   238	          env_extra={"DELTA_REF_MS": "240"}),
   239	     "ENGINE-REFUSED", "DOES-NOT-REPRODUCE"),
   240	]
   241	
   242	MUTATIONS = [
   243	    ("the control threshold is the SAME as the measurand's, not half (grok r6)",
   244	     ['    c_pos, c_neg = thresholds(x["src"], 0.5)',
   245	      '    c_pos, c_neg = thresholds(x["src"], 1.0)'],
   246	     "D=+229, ONE MS under"),
   247	
   248	    ("dirty controls block only the null, not a reproduction (codex r6)",
   249	     ["elif dirty:",
   250	      "elif dirty and not any(s == 'EFFECT' for s in m.values()):"],
   251	     "blocks a REPRODUCTION too"),
   252	
   253	    ("the inverting threshold is -src/10, not -src/11 (codex r2)",
   254	     ["            -min(s_med / 11.0, float(DELTA_REF)) * scale)",
   255	      "            -min(s_med / 10.0, float(DELTA_REF)) * scale)"],
   256	     "inverting threshold is -src/11"),
   257	
   258	    ("the threshold ignores DELTA_REF, so the bar alone forgives 240ms (codex r2)",
   259	     ["    return (min(s_med / 10.0, float(DELTA_REF)) * scale,",
   260	      "    return ((s_med / 10.0) * scale,"],
   261	     "bar alone would forgive"),
   262	
   263	    ("EFFECT is decided on the CI's MIDPOINT, not its lower bound (an outlier reproduces)",
   264	     ["    if ci_lo >= t_pos:", "    if (ci_lo + ci_hi) / 2.0 >= t_pos:"],
   265	     "one huge outlier"),
   266	
   267	    ("the NULL is not tightened by the control bias -- a masked effect reads as a null (grok r9)",
   268	     ["        if not (t_neg + B < x[\"rng\"][0] and x[\"rng\"][1] < t_pos - B):",
   269	      "        if False:"],
   270	     "null must also survive the TIGHTER bound"),
   271	
   272	    ("B is carried as RAW MILLISECONDS across controls of different arm speeds (codex r9)",
   273	     ['        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])',
   274	      '        B_frac = max(B_frac, abs(x["rng"][0]), abs(x["rng"][1]))',
   275	      '    B = B_frac * x["src"]                    # the control bias, on THIS cell\'s arm',
   276	      "    B = B_frac"],
   277	     "B is RELATIVE"),
   278	
   279	    ("the control's residual bias is not carried into the measurand (codex r8)",
   280	     ['        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])',
   281	      "        B_frac = max(B_frac, 0.0)"],
   282	     "exactly T is NOT a reproduction"),
   283	
   284	    ("the engine trusts meta.complete and never counts the pairs (grok r3)",
   285	     ['    if meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None:',
   286	      '    if meta.get(c, {}).get("complete") != "yes" or ci is None:'],
   287	     "SHORT cell (6 of 8 pairs)"),
   288	
   289	    ("a missing registered cell is filtered away (codex r2)",
   290	     ["for c in sorted(set(REGISTERED) | set(meta)):", "for c in sorted(meta):"],
   291	     "missing registered cell"),
   292	
   293	    ("a harness-detected session void is ignored (grok r3)",
   294	     ["elif SESSION_VOID:", "elif False:"],
   295	     "session void (end-load)"),
   296	
   297	    ("the registered DELTA_REF is taken from the environment again (codex r5)",
   298	     ['_env = os.environ.get("DELTA_REF_MS")', "_env = None"],
   299	     "DELTA_REF_MS is PINNED"),
   300	]
   301	
   302	
   303	def rule_unit_tests():
   304	    """The RULE itself, called directly -- because a session at n=8 cannot distinguish the
   305	    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
   306	    closed codex's round-8 blocker; judging a NULL on the RANGE is the SEMANTICS that keeps
   307	    it closed if a larger n is ever registered again, and it can only be tested here."""
   308	    import importlib.util
   309	    spec = importlib.util.spec_from_file_location("eng", DEFAULT_VERDICT)
   310	    # the engine runs on import (it is a script), so exercise classify() via a subprocess-free
   311	    # re-implementation guard: read the function out of the source and exec it in isolation.
   312	    src = open(DEFAULT_VERDICT).read()
   313	    start = src.index("def classify(")
   314	    end = src.index("\n\n", src.index("return \"UNCLEAR\"", start))
   315	    ns = {}
   316	    exec(src[start:end], ns)
   317	    classify = ns["classify"]
   318	    bad = 0
   319	    checks = [
   320	        # ci narrow (outliers trimmed), range wide: a bimodal arm. MUST NOT be NONE.
   321	        ("bimodal: CI=[1,1] but range=[-110,110], T=73", (1, 1, -110, 110, 73, -66), "UNCLEAR"),
   322	        ("clean: CI and range both inside T",            (2, 3, -4, 3, 73, -66),      "NONE"),
   323	        ("a real effect clears T",                       (80, 90, 75, 95, 73, -66),   "EFFECT"),
   324	        ("an inverted effect clears -T",                 (-90, -80, -95, -75, 73, -66), "INVERTED"),
   325	    ]
   326	    for name, args, want in checks:
   327	        got = classify(*args)
   328	        ok = got == want
   329	        print("  %-46s -> %-8s %s" % (name, got, "ok" if ok else "*** FAIL (want %s) ***" % want))
   330	        if not ok:
   331	            bad += 1
   332	
   333	    # THE IDENTITY THE WHOLE RULE LEANS ON: at the registered n=8, the >=95% order-statistic
   334	    # interval IS [min, max]. That is why nothing can be trimmed -- not a null, not B. If
   335	    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
   336	    # must refuse that n (it does).
   337	    ns2 = {}
   338	    src2 = open(DEFAULT_VERDICT).read()
   339	    st = src2.index("def median_ci(")
   340	    exec(src2[st:src2.index("\n\n", src2.index("return best", st))],
   341	         {"comb": __import__("math").comb, "MIN_COVERAGE": 0.95}, ns2)
   342	    import random as _r
   343	    rr = _r.Random(9)
   344	    for _ in range(200):
   345	        d = [rr.randint(-500, 500) for _ in range(8)]
   346	        lo, hi, cov = ns2["median_ci"](d)
   347	        if (lo, hi) != (min(d), max(d)) or cov < 0.95:
   348	            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
   349	            bad += 1
   350	            break
   351	    else:
   352	        print("  %-46s -> %-8s ok" % ("n=8: the >=95% CI IS [min,max] (200 draws)", "identity"))
   353	    return bad
   354	
   355	
   356	def run_cases():
   357	    bad = []
   358	    for name, kw, must_be, must_not in CASES:
   359	        got = session(**kw)
   360	        ok = not (must_be and got != must_be) and not (must_not and got == must_not)
   361	        print("%-66s -> %-20s %s" % (name[:66], got, "ok" if ok else "*** FAIL ***"))
   362	        if not ok:
   363	            bad.append(name)
   364	            print("      expected %s / must not be %s" % (must_be, must_not))
   365	    return bad
   366	
   367	
   368	def fuzz(n=300):
   369	    """No input may land outside the registered outcomes. The CONTROLS are fuzzed too --
   370	    pinning them clean once left every dirty-control path unexercised, and that is
   371	    exactly where a BLOCKER was hiding."""
   372	    rng = random.Random(4242)
   373	    bad = 0
   374	    for _ in range(n):
   375	        got = session(measurand_d=[rng.randint(-600, 600) for _ in range(8)],
   376	                      src=rng.choice([600, 1000, 2000, 2500, 5000]),
   377	                      control_d=[rng.randint(-300, 300) for _ in range(8)],
   378	                      control_src=rng.choice([600, 1000, 2500, 5000]))
   379	        if got not in OUTCOMES:
   380	            print("*** UNREGISTERED OUTCOME %r" % got)
   381	            bad += 1
   382	    print("fuzz: %d/%d inputs produced a registered outcome (measurand AND controls)"
   383	          % (n - bad, n))
   384	    return bad
   385	
   386	
   387	def mutate():
   388	    src = open(DEFAULT_VERDICT).read()
   389	    bad = 0
   390	    for name, subs, key in MUTATIONS:
   391	        body = src
   392	        for i in range(0, len(subs), 2):
   393	            old, new = subs[i], subs[i + 1]
   394	            if old not in body:     # the engine drifted: the proof is STALE, not passing
   395	                print("*** STALE MUTATION (target not found): %s" % name)
   396	                bad += 1
   397	                body = None
   398	                break
   399	            body = body.replace(old, new, 1)
   400	        if body is None:
   401	            continue
   402	        tmp = tempfile.mkdtemp()
   403	        path = os.path.join(tmp, "mutant.py")
   404	        open(path, "w").write(body)
   405	        case = next(c for c in CASES if key in c[0])
   406	        os.environ["VERDICT_PY"] = path
   407	        got = session(**case[1])
   408	        del os.environ["VERDICT_PY"]
   409	        # KILLED == the case now FAILS, by its OWN contract. Checking instead for a
   410	        # verdict named here let a mutant "survive" by failing a different way.
   411	        _, _, must_be, must_not = case
   412	        killed = (must_be and got != must_be) or (must_not and got == must_not)
   413	        print("%-66s -> %-20s %s" % (name[:66], got,
   414	                                     "KILLED" if killed else "*** SURVIVED — VACUOUS ***"))
   415	        if not killed:
   416	            bad += 1
   417	    return bad
   418	
   419	
   420	if __name__ == "__main__":
   421	    if "--mutations" in sys.argv:
   422	        print("Reverting each fix in a copy of the engine; the named case must then FAIL.\n")
   423	        n = mutate()
   424	        print("\n%d/%d mutations killed" % (len(MUTATIONS) - n, len(MUTATIONS)))
   425	        sys.exit(1 if n else 0)
   426	    print("The RULE, called directly (a session at n=8 cannot separate CI from RANGE):")
   427	    unit = rule_unit_tests()
   428	    print()
   429	    fails = run_cases()
   430	    print()
   431	    z = fuzz()
   432	    print("\n%d/%d cases passed" % (len(CASES) - len(fails), len(CASES)))
   433	    sys.exit(1 if (fails or z or unit) else 0)

 succeeded in 0ms:
     1	# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
     2	
     3	**Status**: Pre-registered, **revision 10**. **NO DATA EXISTS YET.**
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
   245	    B = the arm bias the clean controls could NOT rule out, taken from each control's
   246	        full RANGE (not its CI — the CI trims, and a bound must never be computed by
   247	        trimming), as a FRACTION of the arm, then scaled to the cell it is applied to.
   248	
   249	    an EFFECT must clear   T + B     (the bias could be INFLATING it)
   250	    a NULL must fit inside T − B     (the bias could be MASKING an effect)
   251	
   252	**B is RELATIVE, not raw milliseconds (round-9, codex, BLOCKER).** The controls run
   253	different fixtures at different arm speeds: the *same* 4.9% bias is 122 ms on a 2500 ms
   254	large-file control and 24 ms on a fast one. Carrying raw ms across them **under-penalises a
   255	measurand faster than the control** — and that is the dangerous direction, because it would
   256	license a measurand effect that is mostly rig.
   257	
   258	If the controls are genuinely clean, `B` is a few ms and this barely moves. If they are
   259	marginal, it bites — which is the point.
   260	
   261	### The controls are CONTEMPORANEOUS with the measurands
   262	
   263	The schedule is **slot-major**: within slot *i*, **every** cell takes one ABBA pair before
   264	any cell takes slot *i+1*. All six cells therefore span the same wall-clock window.
   265	
   266	**And the order ROTATES by slot (round-9, codex, HIGH).** A *fixed* order put both measurand
   267	cells ahead of every control in every slot — so a **periodic** transient could land on the
   268	measurands and never on the controls that exist to catch it. Over 8 slots each cell occupies
   269	each position.
   270	
   271	*(Round-8, codex, HIGH: both measurand cells used to run first and the controls afterwards
   272	— so **the controls certified a window they were never in**. A transient could hit the
   273	measurand and be gone before the controls ran, and they would pronounce the rig clean.)*
   274	
   275	### The session verdict
   276	
   277	1. **INCOMPLETE** — any registered cell short of its `RUNS` pairs, or with a CI below 95%
   278	   coverage. (Checked against the **data**; `meta.complete` is not believed.)
   279	2. **RIG-VOID** — the harness voided the session (end-load; see Gates).
   280	3. **CONTROLS-NOT-CLEAN** — any control is not `NONE` at `T/2`.
   281	4. **MIXED** — one direction `EFFECT`, the other `INVERTED`: a host×role interaction this
   282	   rig cannot decompose.
   283	5. **REPRODUCES** — `EFFECT` in **either** direction. *(P1's rig-W signature is
   284	   one-directional, so demanding both would rewrite the finding. A messy sibling is
   285	   reported, never substituted.)*
   286	6. **INVERTED** — a new finding; never banked as "P1 absent".
   287	7. **DOES-NOT-REPRODUCE** — **both** measurand cells `NONE`, with clean controls. A
   288	   genuine equivalence result.
   289	8. **UNCLEAR** — otherwise. **This is not a null.** There is no remedy in more pairs: fix the rig.
   290	
   291	### What is deliberately ABSENT, and why that is safe
   292	
   293	- **The 1.10 bar takes NO part in inference.** It is computed on the *marginal medians*,
   294	  reported in every row as the project's **acceptance** criterion, and never consulted.
   295	  The marginal and paired statistics can disagree in **direction and magnitude**, and
   296	  every attempt to let one stand in for the other produced a false verdict.
   297	- **The sign test is reported, not decided on.** At n=8 the CI already implies it
   298	  (`CI_lo >= T > 0` means *every* pair clears T), so making it a second gate only added
   299	  an interaction to get wrong. It is printed per cell.
   300	- **No `UNSTABLE` / `PARTIAL` / `BAR-FAIL-INCONSISTENT` / `UNDERPOWERED` branches, and no
   301	  precedence stack.** A bimodal arm **widens the CI**, and a wide CI lands in `UNCLEAR` —
   302	  which is exactly what those branches were hand-coding. Every run of every arm is still
   303	  printed in `summary.csv`, so bimodality remains visible to the reader.
   304	- **A real but SUB-THRESHOLD effect is reported, not buried.** A cell can be `NONE` and
   305	  still carry a consistent effect below T (e.g. 99 ms on a 1000 ms arm, on 7 of 8 pairs).
   306	  The verdict prints a NOTE naming it. It does not change the outcome — the threshold was
   307	  registered in advance — but it is **not nothing**, and it does not hide inside the word
   308	  "none".
   309	
   310	### There is NO escalation. `n` is EXACTLY 8.
   311	
   312	The old `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the
   313	**full range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR`
   314	rig, nor certify a marginal control; and if you already have an `EFFECT`, you do not need
   315	them.
   316	
   317	**And `n` must be EXACTLY 8, not "at least 8" (round-9, grok, BLOCKER).** At the registered
   318	n=8 the ≥95% interval **is** `[min, max]` — it *cannot* trim. At any larger n it starts
   319	trimming outliers, and a bimodal arm then yields a narrow median CI and a false verdict:
   320	grok drove a 16-pair CSV (three pairs at −500 trimmed away, thirteen at +200 left) straight
   321	to **`REPRODUCES`**. A cell carrying any count but the registered one is **`INCOMPLETE`**.
   322	*(I removed 16 from the registered list and left the completeness check saying `>=`. Fixed
   323	where I looked, not where it lived — again.)*
   324	
   325	**A noisy rig is fixed by a quieter rig, not by more pairs — and `UNCLEAR` says exactly
   326	that.** Removing it also removes its entire p-hacking guard surface (a "once" marker, a
   327	verdict check, a data-hash burn), none of which now has to be right.
   328	
   329	### The registered constants are PINNED IN CODE
   330	
   331	`DELTA_REF_MS`, `SETTLE_MS`, `LOAD_MAX`, `DRAIN_MBPS` and the rest are **literals** in
   332	both the harness and the engine. The harness **refuses to start** if one is merely
   333	*present* in the environment. *(They were once `${VAR:-default}`, and `DELTA_REF_MS=240`
   334	turned a void into a null — i.e. the rule could be retuned from the command line, after
   335	the data existed, in the direction of the answer you want. **That is not a
   336	pre-registration.**)* To change one: amend this document and put it back through review.
   337	
   338	### The guard test
   339	
   340	`scripts/otp12pf_mac_verdict_test.py`: **26 cases — every one a defect a reviewer actually
   341	drove out of a previous revision** — each **mutation-proven** (reverting that fix in a copy
   342	of the engine makes exactly that case fail: **9/9 mutations killed**), plus a 300-input
   343	fuzz over the measurand **and** the controls. It runs at preflight, cases *and* mutations;
   344	a vacuous guard refuses the run.
   345	
   346	## The instrument — what round 3 found, and what now guards it
   347	
   348	**THE TIMER WAS MEASURING FSYNC NOISE (round-3 BLOCKER; I introduced it in the
   349	rework that fixed round 2).** The transfer timer captured `time.monotonic()` in
   350	**two separate `python3 -c` processes** and subtracted them. On macOS that clock is
   351	**process-relative**. Measured on this rig: a **1000 ms sleep read as −1 ms on
   352	nagatha and 2 ms on q** — *negative*. Every `ms` row would have been ≈ `fsync_ms`
   353	alone, and the invariance ratio — **the entire measurand** — would have been
   354	computed on fsync noise, which can manufacture or mask a one-directional effect at
   355	will. The rig would have produced a clean session, 0 voided pairs, and a confident,
   356	meaningless verdict. **Grok measured the same defect independently** (a 500 ms sleep
   357	reading ~3 ms) before being shown codex's finding.
   358	
   359	The repo **already documented this trap** — `bench_otp12_zoey.sh:116` uses
   360	`time.time()` and says why — and I reintroduced it anyway. **The lesson is not "add
   361	a reviewer"; it is READ THE EXISTING HARNESSES BEFORE WRITING A NEW ONE.**
   362	
   363	Now: **one process times itself and spawns the client**, and — the structural fix —
   364	**preflight PROVES THE CLOCK on both hosts against a known 1000 ms sleep before any
   365	data is taken**, and a run whose timer returns a non-positive value **VOIDS** rather
   366	than entering the data as a "fast" row. The timing bug class cannot ship again
   367	without the instrument catching it on the rig.
   368	
   369	**Two defects that could have MANUFACTURED the result (round-2, still guarded):**
   370	
   371	1. **The durability check was fail-open.** `os.walk()` on a missing path returns
   372	   **0 files in 0 ms** — a missing tree reads as a *fast, successful flush*. The two
   373	   arms need **different** landed paths (blit uses rsync-style slash semantics: a
   374	   push to `/bench/RUNDIR/` lands at `RUNDIR/src_<W>`; a pull into `RUNDIR` lands
   375	   **directly in** `RUNDIR`). A wrong path would charge one arm **zero** durability
   376	   while the other paid full — the otp-2w bug that once manufactured P1.
   377	   **Guarded**: the fsync walk returns its **file count and byte sum**, and the pair
   378	   **VOIDs** unless both match the fixture exactly.
   379	2. **The free-writeback gap REVERSES SIGN WITH DIRECTION.** Between a client exiting
   380	   and the fsync starting, the OS writes back dirty pages **for free**, and that gap
   381	   is longer for whichever arm ran over ssh — and *which arm that is flips with the
   382	   data direction*. Since P1's signature is one-directional, this artifact could
   383	   produce a one-directional "reproduction" **out of nothing**.
   384	   **⛔ AND UNTIL REV 6, THE SETTLE NEVER RAN AT ALL (see the correction at the top).**
   385	   The `awk` computing its duration sat in a command substitution with the wrong
   386	   quoting, so it errored, `sleep` got an empty argument and failed, and the exit
   387	   status was discarded. Revisions 3–5 asserted this fix while it was dead — including
   388	   the revision that *introduced* it to close this very BLOCKER.
   389	
   390	   **Now, and only now: equalized, and BOUNDED — not "removed" (round-3 HIGH).** The
   391	   settle window is **equal on both arms** (250 ms, computed once at top level,
   392	   validated at startup, and its failure **VOIDS the pair**). The residual is the ssh
   393	   dispatch difference, **measured at ~15 ms** (median of 5, warm mux, recorded in the
   394	   manifest every session; a failed ssh now refuses rather than contributing a
   395	   flattering number). A pre-fsync delay of 10/20/200 ms produced **no measurable
   396	   change** in fsync time here (72–94 ms, no trend) — APFS fsync on this rig is
   397	   per-file-metadata bound, not writeback bound — so a 15 ms residual cannot plausibly
   398	   move it. **That is a bound from measurement, not a removal by construction, and this
   399	   document no longer claims otherwise.** `SELFTEST=1` walks a real tree and proves the
   400	   settle actually executed.
   401	
   402	## Gates — every one fails CLOSED, and every one is EXECUTED
   403	
   404	Round 2 found the round-1 "fixes" **had never been run** (`bash -n` is not an
   405	execution): the preflight **could not succeed at all** — `grep -c` exits 1 on no
   406	match, so a **clean** binary tripped the dirty-marker probe and died, and `norm_mac`
   407	used gawk's `strtonum()`, absent from stock macOS awk.
   408	
   409	`SELFTEST=1` **exercises the gates for real and takes no data.** It reports three
   410	states — `[OK]`, `[FIRED]` (a genuinely unmet condition: the gate *works*), and
   411	`[BROKEN]` (**the probe cannot answer at all**) — and **exits non-zero on any BROKEN**,
   412	because *a blind gate is precisely what fails open on the night*. It also **prints what
   413	it does NOT cover**.
   414	
   415	*(Round-5 codex, HIGH: the previous self-test labelled **every** nonzero result
   416	`[FIRED]` — including a probe that could not answer — exited zero, and claimed "every
   417	gate executes" while never touching drain, purge, daemon, fsync/settle, stale-daemon or
   418	end-load. **A self-test that overstates itself is the very fail-open it exists to
   419	hunt.**)*
   420	
   421	It has now earned itself three times: it caught `link_gate` **refusing a perfectly good
   422	link** (`arp -n <ip>` prints **one line per interface** — `q` holds entries for nagatha
   423	on en0, en1 *and* en8 — so the unfiltered MAC was a three-line string that could never
   424	equal one MAC; the gate now checks the entry **on the egress NIC**, the more correct
   425	question anyway); it caught **the dead settle**; and it caught **itself** breaking its
   426	own next gate (it ran `resolve_disk` in a subshell, which discarded the global that
   427	function exists to set, so the drain then had no device and blamed the harness).
   428	
   429	- **QUIESCENCE, BOTH MACS** — refuse if `codex`/`cargo`/`rustc` runs on **either**
   430	  Mac. `pgrep` rc≥2 is an **error**, not "quiet" (rev 3 could not tell them apart).
   431	- **TIME MACHINE, BOTH MACS** — refuse if a backup is running **or if autobackup is
   432	  merely ENABLED** (macOS repeats hourly; pf-0's fired 1 minute before its run). A
   433	  read error refuses.
   434	- **SPOTLIGHT, BOTH MACS** — `mds_stores` CPU, taken as the **MAX across samples**
   435	  (rev 3 took the last, so a late idle sample could overwrite an earlier busy one);
   436	  a failed `top` is an **error**, not 0%.
   437	- **LOAD** — `load1` on both Macs at start **and end**. A start `load1` above 3.0
   438	  refuses; an **end** `load1` above 3.0 **VOIDS THE SESSION** (`RIG-VOID`), because a
   439	  mid-session load spike is exactly the contamination the start gate exists to stop.
   440	  *(Round-4, grok: rev 4 moved the end-load logging before the verdict and its
   441	  comment claimed a session "can void on it" — but the code only **logged** it and
   442	  graded anyway. A doc claim the code did not honour: the very defect class this
   443	  review exists to kill.)*
   444	- **COLD CACHES** both ends every run (`sudo -n /usr/sbin/purge`); a failed purge
   445	  **VOIDS the pair**.
   446	- **DRAIN** — destination disk quiet before each window (`< 2 MB/s`, 3 consecutive
   447	  2 s samples). The device is **RESOLVED from the module path** through its APFS
   448	  physical store (grok: rev 3 hardcoded `disk0` and could certify a disk the data
   449	  never touched — and on APFS a *synthesized* disk can read idle while the physical
   450	  store saturates). A **non-numeric** `iostat` sample is an **error**, never "quiet"
   451	  (rev 3 read it as zero and **certified drainage**).
   452	- **DURABILITY** — the per-file `fsync` walk runs **on the destination host for both
   453	  arms**, is timed, and returns `NA` on a missing tree → the pair **VOIDS**.
   454	- **FIXTURES** verified by **count AND byte sum** on both ends before any timed run.
   455	- **PROVENANCE** — clean `+f35702a` on all four binaries (never `.dirty`); the
   456	  harness, the **verdict engine** and its **guard test** are all hashed into the
   457	  manifest; the instrument must be **committed and clean** in git (a modified
   458	  harness must not be able to claim the reviewed commit); `EXPECT_SHA` must equal
   459	  the **registered** build. `die` inside `$(...)` exits only the subshell, so the
   460	  hash functions now **return non-zero** and the caller dies (rev 3 wrote an **empty
   461	  hash** and called it provenance).
   462	- **DAEMON LIFECYCLE** — the pid comes from `$!` (not `pgrep | head -1`, which picks
   463	  the first of whatever is running); it must be **alive AND LISTENING** on the port;
   464	  teardown is **verified** (a failed probe is a failure, not "GONE") and a survivor
   465	  is recorded, not discarded.
   466	- **LINK** — peer ARP **on the egress NIC** resolves to the **peer's** MAC (a host
   467	  route on a directly-connected subnet installs a black hole that still reports the
   468	  right interface), and the route egresses the 10GbE NIC (macOS routes by service
   469	  order, so a 1GbE NIC can win and every run would go over gigabit).
   470	- **THE VERDICT ENGINE'S OWN GUARD TEST RUNS AT PREFLIGHT — cases AND mutations.**
   471	  If the decision rule fails its own cases, or if the proof that guards it turns out
   472	  to be **vacuous** (a mutation survives), **no data is taken**. *(Round-4, grok: rev
   473	  4's preflight ran only the cases, so a silently-reverted fix could still pass if
   474	  the cases happened to pass for another reason.)*
   475	
   476	## What this does NOT establish
   477	
   478	- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
   479	- **A reproduction does not exonerate macOS/APFS or host×role** — "not
   480	  Windows-specific" is **not** "not platform-specific" (round-3 BLOCKER).
   481	- **It does not bear on an escape hatch for P1 — P1 has none.**
   482	- **It cannot measure P2** — P2 is a converge bar against the OLD build and no old
   483	  pair is staged on the Macs.
   484	- **A null here cannot retract rig W.** P1 is measured and reproduced four times
   485	  there; this rig can only speak about the *pairing*.
   486	
   487	## The runs
   488	
   489	```sh
   490	SELFTEST=1 bash scripts/bench_otp12pf_mac.sh                 # exercise the gates, no data
   491	PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
   492	CLEARED_BY_REVIEW=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh   # the run
   493	```
   494	
   495	The harness **refuses a timed run** until the round-3 review is adjudicated
   496	(`CLEARED_BY_REVIEW`); the no-data modes always work. It **computes; it declares
   497	nothing** — the verdict is read off the rule above.

exec
/bin/zsh -lc 'wc -l scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     406 scripts/otp12pf_mac_verdict.py
     433 scripts/otp12pf_mac_verdict_test.py
     497 docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
    1336 total

exec
/bin/zsh -lc "sed -n '1,220p' scripts/otp12pf_mac_verdict_test.py | nl -ba" in /Users/michael/Dev/blit_v2
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
    93	     dict(measurand_d=[-100, -50, 0, 50, 100, 200, 220, 240], src=2500, control_d=[0] * 8),
    94	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
    95	
    96	    ("codex r2: the inverting threshold is -src/11, not -src/10 (CI [-190,0] @ 2000)",
    97	     dict(measurand_d=[-190, -190, 0, 0, 0, 0, 0, 0], src=2000, control_d=[0] * 8),
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
   183	    ("codex r9: B is RELATIVE — a 4.9% bias on a FAST control must not under-penalise a slower measurand",
   184	     dict(measurand_d=[130] * 8, src=1000,
   185	          control_d=[24] * 8, control_src=500),
   186	     "UNCLEAR", "REPRODUCES"),
   187	
   188	    ("grok r9: a null must also survive the TIGHTER bound (bias could be MASKING an effect)",
   189	     dict(measurand_d=[60] * 8, src=1000, control_d=[49] * 8),
   190	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   191	
   192	    ("codex r8: ...and the same effect IS one once the rig is bias-free",
   193	     dict(measurand_d=[105] * 8, src=1000, control_d=[5] * 8),
   194	     "REPRODUCES", "UNCLEAR"),
   195	
   196	    ("source-initiated slower is INVERTED, never 'P1 absent'",
   197	     dict(measurand_d=[-300, -310, -320, -330, -340, -350, -360, -370], src=1000),
   198	     "INVERTED", None),
   199	
   200	    ("one direction reproduces, the other inverts -> MIXED",
   201	     dict(measurand_d=[0] * 8, src=1000,
   202	          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
   203	                    "qn_tcp_mixed": ([-300, -310, -320, -330, -340, -350, -360, -370], 1000)}),
   204	     "MIXED", "REPRODUCES"),
   205	
   206	    ("a clean one-direction reproduction is NOT masked by a noisy sibling",
   207	     dict(measurand_d=[0] * 8, src=1000,
   208	          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
   209	                    "qn_tcp_mixed": ([-20, 300, 310, 320, 330, 340, 350, 360], 1000)}),
   210	     "REPRODUCES", "UNCLEAR"),
   211	
   212	    ("codex r8: a bimodal arm cannot hide from the RANGE (a null is judged on every pair)",
   213	     dict(measurand_d=[-110, 0, -110, 110, 110, 0, -110, 0], src=730,
   214	          control_d=[0] * 8),
   215	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   216	
   217	    ("a null the rig could not have SEEN is UNCLEAR, not a null",
   218	     dict(measurand_d=[-400, -300, -100, 0, 0, 100, 300, 400], src=2000),
   219	     "UNCLEAR", "DOES-NOT-REPRODUCE"),
   220	

exec
/bin/zsh -lc "sed -n '221,433p' scripts/otp12pf_mac_verdict_test.py | nl -ba -v 221" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   221	    # --- integrity ---------------------------------------------------------------
   222	    ("a missing registered cell is INCOMPLETE, never filtered away",
   223	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   224	          drop_cells=("qn_tcp_mixed",)),
   225	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   226	
   227	    ("grok r3: n=1 with complete=yes must not grade at 0% CI coverage",
   228	     dict(measurand_d=[0], src=2000, control_d=[5], control_src=1000),
   229	     "INCOMPLETE", "DOES-NOT-REPRODUCE"),
   230	
   231	    ("grok r3: a harness-detected session void (end-load) refuses a verdict",
   232	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   233	          void_reason="end-load on q is 9.1 (> 3.0)"),
   234	     "RIG-VOID", "DOES-NOT-REPRODUCE"),
   235	
   236	    ("codex r5: DELTA_REF_MS is PINNED -- the rule is not tunable from the environment",
   237	     dict(measurand_d=[-4, -2, -1, 0, 0, 1, 2, 3], src=2000,
   238	          env_extra={"DELTA_REF_MS": "240"}),
   239	     "ENGINE-REFUSED", "DOES-NOT-REPRODUCE"),
   240	]
   241	
   242	MUTATIONS = [
   243	    ("the control threshold is the SAME as the measurand's, not half (grok r6)",
   244	     ['    c_pos, c_neg = thresholds(x["src"], 0.5)',
   245	      '    c_pos, c_neg = thresholds(x["src"], 1.0)'],
   246	     "D=+229, ONE MS under"),
   247	
   248	    ("dirty controls block only the null, not a reproduction (codex r6)",
   249	     ["elif dirty:",
   250	      "elif dirty and not any(s == 'EFFECT' for s in m.values()):"],
   251	     "blocks a REPRODUCTION too"),
   252	
   253	    ("the inverting threshold is -src/10, not -src/11 (codex r2)",
   254	     ["            -min(s_med / 11.0, float(DELTA_REF)) * scale)",
   255	      "            -min(s_med / 10.0, float(DELTA_REF)) * scale)"],
   256	     "inverting threshold is -src/11"),
   257	
   258	    ("the threshold ignores DELTA_REF, so the bar alone forgives 240ms (codex r2)",
   259	     ["    return (min(s_med / 10.0, float(DELTA_REF)) * scale,",
   260	      "    return ((s_med / 10.0) * scale,"],
   261	     "bar alone would forgive"),
   262	
   263	    ("EFFECT is decided on the CI's MIDPOINT, not its lower bound (an outlier reproduces)",
   264	     ["    if ci_lo >= t_pos:", "    if (ci_lo + ci_hi) / 2.0 >= t_pos:"],
   265	     "one huge outlier"),
   266	
   267	    ("the NULL is not tightened by the control bias -- a masked effect reads as a null (grok r9)",
   268	     ["        if not (t_neg + B < x[\"rng\"][0] and x[\"rng\"][1] < t_pos - B):",
   269	      "        if False:"],
   270	     "null must also survive the TIGHTER bound"),
   271	
   272	    ("B is carried as RAW MILLISECONDS across controls of different arm speeds (codex r9)",
   273	     ['        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])',
   274	      '        B_frac = max(B_frac, abs(x["rng"][0]), abs(x["rng"][1]))',
   275	      '    B = B_frac * x["src"]                    # the control bias, on THIS cell\'s arm',
   276	      "    B = B_frac"],
   277	     "B is RELATIVE"),
   278	
   279	    ("the control's residual bias is not carried into the measurand (codex r8)",
   280	     ['        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])',
   281	      "        B_frac = max(B_frac, 0.0)"],
   282	     "exactly T is NOT a reproduction"),
   283	
   284	    ("the engine trusts meta.complete and never counts the pairs (grok r3)",
   285	     ['    if meta.get(c, {}).get("complete") != "yes" or len(d) != PAIRS or ci is None:',
   286	      '    if meta.get(c, {}).get("complete") != "yes" or ci is None:'],
   287	     "SHORT cell (6 of 8 pairs)"),
   288	
   289	    ("a missing registered cell is filtered away (codex r2)",
   290	     ["for c in sorted(set(REGISTERED) | set(meta)):", "for c in sorted(meta):"],
   291	     "missing registered cell"),
   292	
   293	    ("a harness-detected session void is ignored (grok r3)",
   294	     ["elif SESSION_VOID:", "elif False:"],
   295	     "session void (end-load)"),
   296	
   297	    ("the registered DELTA_REF is taken from the environment again (codex r5)",
   298	     ['_env = os.environ.get("DELTA_REF_MS")', "_env = None"],
   299	     "DELTA_REF_MS is PINNED"),
   300	]
   301	
   302	
   303	def rule_unit_tests():
   304	    """The RULE itself, called directly -- because a session at n=8 cannot distinguish the
   305	    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
   306	    closed codex's round-8 blocker; judging a NULL on the RANGE is the SEMANTICS that keeps
   307	    it closed if a larger n is ever registered again, and it can only be tested here."""
   308	    import importlib.util
   309	    spec = importlib.util.spec_from_file_location("eng", DEFAULT_VERDICT)
   310	    # the engine runs on import (it is a script), so exercise classify() via a subprocess-free
   311	    # re-implementation guard: read the function out of the source and exec it in isolation.
   312	    src = open(DEFAULT_VERDICT).read()
   313	    start = src.index("def classify(")
   314	    end = src.index("\n\n", src.index("return \"UNCLEAR\"", start))
   315	    ns = {}
   316	    exec(src[start:end], ns)
   317	    classify = ns["classify"]
   318	    bad = 0
   319	    checks = [
   320	        # ci narrow (outliers trimmed), range wide: a bimodal arm. MUST NOT be NONE.
   321	        ("bimodal: CI=[1,1] but range=[-110,110], T=73", (1, 1, -110, 110, 73, -66), "UNCLEAR"),
   322	        ("clean: CI and range both inside T",            (2, 3, -4, 3, 73, -66),      "NONE"),
   323	        ("a real effect clears T",                       (80, 90, 75, 95, 73, -66),   "EFFECT"),
   324	        ("an inverted effect clears -T",                 (-90, -80, -95, -75, 73, -66), "INVERTED"),
   325	    ]
   326	    for name, args, want in checks:
   327	        got = classify(*args)
   328	        ok = got == want
   329	        print("  %-46s -> %-8s %s" % (name, got, "ok" if ok else "*** FAIL (want %s) ***" % want))
   330	        if not ok:
   331	            bad += 1
   332	
   333	    # THE IDENTITY THE WHOLE RULE LEANS ON: at the registered n=8, the >=95% order-statistic
   334	    # interval IS [min, max]. That is why nothing can be trimmed -- not a null, not B. If
   335	    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
   336	    # must refuse that n (it does).
   337	    ns2 = {}
   338	    src2 = open(DEFAULT_VERDICT).read()
   339	    st = src2.index("def median_ci(")
   340	    exec(src2[st:src2.index("\n\n", src2.index("return best", st))],
   341	         {"comb": __import__("math").comb, "MIN_COVERAGE": 0.95}, ns2)
   342	    import random as _r
   343	    rr = _r.Random(9)
   344	    for _ in range(200):
   345	        d = [rr.randint(-500, 500) for _ in range(8)]
   346	        lo, hi, cov = ns2["median_ci"](d)
   347	        if (lo, hi) != (min(d), max(d)) or cov < 0.95:
   348	            print("  *** FAIL: at n=8 the CI is NOT the full range: %s vs %s" % ((lo, hi), (min(d), max(d))))
   349	            bad += 1
   350	            break
   351	    else:
   352	        print("  %-46s -> %-8s ok" % ("n=8: the >=95% CI IS [min,max] (200 draws)", "identity"))
   353	    return bad
   354	
   355	
   356	def run_cases():
   357	    bad = []
   358	    for name, kw, must_be, must_not in CASES:
   359	        got = session(**kw)
   360	        ok = not (must_be and got != must_be) and not (must_not and got == must_not)
   361	        print("%-66s -> %-20s %s" % (name[:66], got, "ok" if ok else "*** FAIL ***"))
   362	        if not ok:
   363	            bad.append(name)
   364	            print("      expected %s / must not be %s" % (must_be, must_not))
   365	    return bad
   366	
   367	
   368	def fuzz(n=300):
   369	    """No input may land outside the registered outcomes. The CONTROLS are fuzzed too --
   370	    pinning them clean once left every dirty-control path unexercised, and that is
   371	    exactly where a BLOCKER was hiding."""
   372	    rng = random.Random(4242)
   373	    bad = 0
   374	    for _ in range(n):
   375	        got = session(measurand_d=[rng.randint(-600, 600) for _ in range(8)],
   376	                      src=rng.choice([600, 1000, 2000, 2500, 5000]),
   377	                      control_d=[rng.randint(-300, 300) for _ in range(8)],
   378	                      control_src=rng.choice([600, 1000, 2500, 5000]))
   379	        if got not in OUTCOMES:
   380	            print("*** UNREGISTERED OUTCOME %r" % got)
   381	            bad += 1
   382	    print("fuzz: %d/%d inputs produced a registered outcome (measurand AND controls)"
   383	          % (n - bad, n))
   384	    return bad
   385	
   386	
   387	def mutate():
   388	    src = open(DEFAULT_VERDICT).read()
   389	    bad = 0
   390	    for name, subs, key in MUTATIONS:
   391	        body = src
   392	        for i in range(0, len(subs), 2):
   393	            old, new = subs[i], subs[i + 1]
   394	            if old not in body:     # the engine drifted: the proof is STALE, not passing
   395	                print("*** STALE MUTATION (target not found): %s" % name)
   396	                bad += 1
   397	                body = None
   398	                break
   399	            body = body.replace(old, new, 1)
   400	        if body is None:
   401	            continue
   402	        tmp = tempfile.mkdtemp()
   403	        path = os.path.join(tmp, "mutant.py")
   404	        open(path, "w").write(body)
   405	        case = next(c for c in CASES if key in c[0])
   406	        os.environ["VERDICT_PY"] = path
   407	        got = session(**case[1])
   408	        del os.environ["VERDICT_PY"]
   409	        # KILLED == the case now FAILS, by its OWN contract. Checking instead for a
   410	        # verdict named here let a mutant "survive" by failing a different way.
   411	        _, _, must_be, must_not = case
   412	        killed = (must_be and got != must_be) or (must_not and got == must_not)
   413	        print("%-66s -> %-20s %s" % (name[:66], got,
   414	                                     "KILLED" if killed else "*** SURVIVED — VACUOUS ***"))
   415	        if not killed:
   416	            bad += 1
   417	    return bad
   418	
   419	
   420	if __name__ == "__main__":
   421	    if "--mutations" in sys.argv:
   422	        print("Reverting each fix in a copy of the engine; the named case must then FAIL.\n")
   423	        n = mutate()
   424	        print("\n%d/%d mutations killed" % (len(MUTATIONS) - n, len(MUTATIONS)))
   425	        sys.exit(1 if n else 0)
   426	    print("The RULE, called directly (a session at n=8 cannot separate CI from RANGE):")
   427	    unit = rule_unit_tests()
   428	    print()
   429	    fails = run_cases()
   430	    print()
   431	    z = fuzz()
   432	    print("\n%d/%d cases passed" % (len(CASES) - len(fails), len(CASES)))
   433	    sys.exit(1 if (fails or z or unit) else 0)

exec
/bin/zsh -lc "sed -n '180,406p' scripts/otp12pf_mac_verdict.py | nl -ba -v 180" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
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
   220	#     B = the arm asymmetry the controls could not rule out, as a FRACTION OF THE ARM,
   221	#         scaled to the cell it is applied to. Taken from each control's full RANGE (not its
   222	#         CI: the CI is an interval for the MEDIAN and it TRIMS, and a bound on what the rig
   223	#         might be carrying must never be computed by trimming). Relative, not raw ms:
   224	#         the controls run different fixtures at different speeds.
   225	#     an EFFECT must clear  T + B     (bias could be INFLATING it)
   226	#     a NULL   must fit in  T - B     (bias could be MASKING an effect)
   227	#
   228	# If the controls are genuinely clean, B is a few ms and this barely moves. If they are
   229	# marginal, it bites -- which is the point.
   230	dirty = []
   231	B_frac = 0.0          # RELATIVE, not raw milliseconds
   232	for c in CONTROLS:
   233	    x = cell.get(c, {})
   234	    if x.get("state") == "INCOMPLETE":
   235	        continue
   236	    c_pos, c_neg = thresholds(x["src"], 0.5)
   237	    x["ctrl_state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], c_pos, c_neg)
   238	    x["ctrl_T"] = c_pos
   239	    if x["ctrl_state"] != "NONE":
   240	        dirty.append(c)
   241	    elif x["src"]:
   242	        # B IS A FRACTION OF THE ARM, NOT A MILLISECOND COUNT (round-9 codex, BLOCKER).
   243	        # The controls run on DIFFERENT fixtures and therefore different arm speeds: the
   244	        # same 4.9% arm bias is 122 ms on a 2500 ms large-file control and 24 ms on a fast
   245	        # one. Carrying raw ms across them OVER-penalises a measurand slower than the
   246	        # control and UNDER-penalises one that is faster -- and the second direction is the
   247	        # dangerous one: a 4.9% bias measured on a fast control would license a measurand
   248	        # effect that is mostly rig. Take the bias as a FRACTION and scale it to whatever
   249	        # arm it is being applied to.
   250	        B_frac = max(B_frac, abs(x["rng"][0]) / x["src"], abs(x["rng"][1]) / x["src"])
   251	
   252	# ---- pass 3: grade the measurands, against thresholds widened by the control bias -----
   253	for c in MEASURANDS:
   254	    x = cell.get(c, {})
   255	    if x.get("state") == "INCOMPLETE":
   256	        continue
   257	    t_pos, t_neg = thresholds(x["src"])
   258	    B = B_frac * x["src"]                    # the control bias, on THIS cell's arm
   259	    x["T"] = t_pos
   260	    x["B"] = B
   261	    x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1],
   262	                          t_pos + B, t_neg - B)          # an EFFECT must clear T + B
   263	    if x["state"] == "NONE":
   264	        # ...and a NULL must survive the TIGHTER bound: bias could be masking an effect.
   265	        if not (t_neg + B < x["rng"][0] and x["rng"][1] < t_pos - B):
   266	            x["state"] = "UNCLEAR"
   267	
   268	# Controls also carry a state for the report; measurands carry a ctrl_state for symmetry.
   269	for c in cell:
   270	    x = cell[c]
   271	    if x.get("state") == "INCOMPLETE":
   272	        continue
   273	    if "state" not in x:                                  # a control: report its own state
   274	        t_pos, t_neg = thresholds(x["src"])
   275	        x["T"] = t_pos
   276	        x["B"] = 0.0
   277	        x["state"] = classify(x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], t_pos, t_neg)
   278	    x.setdefault("ctrl_state", "-")
   279	
   280	# ---- outputs -----------------------------------------------------------------------
   281	with open(sum_p, "w") as f:
   282	    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,voided,runs\n")
   283	    for (c, a) in sorted(by):
   284	        v = by[(c, a)]
   285	        f.write("%s,%s,%d,%d,%d,%d,%d,%s\n" % (c, a, med(v), sum(v) // len(v), min(v),
   286	                                               max(v), voided.get((c, a), 0),
   287	                                               " ".join(map(str, v))))
   288	
   289	with open(pair_p, "w") as f:
   290	    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,range_lo,range_hi,"
   291	            "coverage,T_ms,B_ms,sign_p,k_pos,state,control_state\n")
   292	    for c in sorted(cell):
   293	        x = cell[c]
   294	        if x["state"] == "INCOMPLETE":
   295	            f.write("%s,%d,,,,,,,,,,,,,,,INCOMPLETE,\n" % (c, x["n"]))
   296	            continue
   297	        f.write("%s,%d,%d,%d,%.3f,%s,%d,%d,%d,%d,%d,%.4f,%d,%d,%.4f,%d/%d,%s,%s\n" % (
   298	            c, x["n"], x["src"], x["dst"], x["ratio"], x["bar"], x["D"],
   299	            x["ci"][0], x["ci"][1], x["rng"][0], x["rng"][1], x["cov"],
   300	            round(x["T"]), round(x.get("B", 0)), x["p"], x["k"], x["n"],
   301	            x["state"], x["ctrl_state"]))
   302	
   303	with open(ver_p, "w") as f:
   304	    f.write("comparison,kind,lhs_ms,rhs_ms,ratio,bar\n")
   305	    for c in sorted(cell):
   306	        x = cell[c]
   307	        if x["state"] == "INCOMPLETE":
   308	            f.write("%s,invariance,,,,INCOMPLETE\n" % c)
   309	        else:
   310	            f.write("%s,invariance,%d,%d,%.3f,%s\n"
   311	                    % (c, x["src"], x["dst"], x["ratio"], x["bar"]))
   312	
   313	# ---- THE SESSION VERDICT -----------------------------------------------------------
   314	incomplete = [c for c in REGISTERED if cell.get(c, {}).get("state") == "INCOMPLETE"]
   315	m = {c: cell[c]["state"] for c in MEASURANDS if not incomplete}
   316	
   317	if incomplete:
   318	    verdict = "INCOMPLETE"
   319	    why = ("cells short of their %d pairs, or with a CI below the registered %.0f%% "
   320	           "coverage: %s. No verdict is read." % (PAIRS, 100 * MIN_COVERAGE,
   321	                                                  ", ".join(incomplete)))
   322	elif SESSION_VOID:
   323	    verdict = "RIG-VOID"
   324	    why = "the harness voided this session: %s. No verdict is read." % SESSION_VOID
   325	elif dirty:
   326	    verdict = "CONTROLS-NOT-CLEAN"
   327	    why = ("control cell(s) are not free of an arm asymmetry at T/2: %s. P1 is claimed "
   328	           "TCP-only and mixed-only; if the gRPC/large controls may be carrying the same "
   329	           "asymmetry, then NEITHER a reproduction NOR a null is readable off this rig. "
   330	           "There is no escalation: a noisy rig is fixed by a QUIETER RIG, not more pairs."
   331	           % ", ".join("%s(%s, D=%+dms, CI=[%+d,%+d], T/2=%d)"
   332	                       % (c, cell[c]["ctrl_state"], cell[c]["D"], cell[c]["ci"][0],
   333	                          cell[c]["ci"][1], round(cell[c]["T"] / 2))
   334	                       for c in dirty))
   335	elif "EFFECT" in m.values() and "INVERTED" in m.values():
   336	    verdict = "MIXED"
   337	    why = ("one direction shows the effect and the other INVERTS it -- a host x role "
   338	           "interaction this rig cannot decompose. Inconclusive for the question.")
   339	elif "EFFECT" in m.values():
   340	    verdict = "REPRODUCES"
   341	    why = ("P1 reproduces WITHOUT a Windows peer, in: %s. Scoped to THIS pair: it shows "
   342	           "P1 CAN occur macOS<->macOS, so it is not waivable as 'Windows residue'. It "
   343	           "does NOT establish a platform-general cost, does NOT name the mechanism, "
   344	           "does NOT kill H1 (the code H1 accuses runs here too), and leaves macOS/APFS "
   345	           "and host x role explanations OPEN."
   346	           % ", ".join(c for c, s in m.items() if s == "EFFECT"))
   347	elif "INVERTED" in m.values():
   348	    verdict = "INVERTED"
   349	    why = ("source-initiated is the SLOW arm in: %s. A NEW finding; never bank it as "
   350	           "'P1 absent'." % ", ".join(c for c, s in m.items() if s == "INVERTED"))
   351	elif all(s == "NONE" for s in m.values()):
   352	    verdict = "DOES-NOT-REPRODUCE"
   353	    why = ("both TCP-mixed cells EXCLUDE an effect of size T, and every control is clean "
   354	           "at T/2 -- a genuine equivalence result. Scoped to THIS pair: P1 did not "
   355	           "reproduce macOS<->macOS. That is CONSISTENT with 'the Windows peer is "
   356	           "required' but does NOT prove it -- it could equally be a property of these "
   357	           "two machines, their disks, or this macOS version.")
   358	else:
   359	    verdict = "UNCLEAR"
   360	    why = ("the CI spans the threshold in: %s. The rig could not resolve an effect of "
   361	           "size T either way -- this is NOT 'P1 vanishes'. Fix the rig, do not add pairs."
   362	           % ", ".join(c for c, s in m.items() if s == "UNCLEAR"))
   363	
   364	out = ["SESSION VERDICT: %s" % verdict, "", why, "",
   365	       "Per cell. T = min(srcinit_median/10, %d ms). Controls must be NONE at T/2, and B is"
   366	       % DELTA_REF,
   367	       "the arm bias they could NOT exclude: an EFFECT must clear T+B, a NULL must fit in T-B."]
   368	for c in sorted(cell):
   369	    x = cell[c]
   370	    if x["state"] == "INCOMPLETE":
   371	        out.append("  %-14s INCOMPLETE (%d pairs)" % (c, x["n"]))
   372	        continue
   373	    out.append("  %-14s %-8s ctrl=%-8s D=%+5dms CI=[%+5d,%+5d] range=[%+5d,%+5d] "
   374	               "T=%3dms B=%3dms  ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
   375	               % (c, x["state"], x["ctrl_state"], x["D"], x["ci"][0], x["ci"][1],
   376	                  x["rng"][0], x["rng"][1], round(x["T"]), round(x.get("B", 0)),
   377	                  x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
   378	# A cell can be NONE (an effect of size T is excluded) and STILL carry a real, consistent
   379	# effect BELOW T -- e.g. 99 ms on a 1000 ms arm, one millisecond under the threshold, on
   380	# 7 of 8 pairs. That is not a contradiction and it does not change the verdict, but it
   381	# must not hide inside the word "none". Reported, never decided on.
   382	subthreshold = [c for c in sorted(cell)
   383	                if cell[c]["state"] == "NONE" and cell[c]["p"] < 0.05 and cell[c]["D"]]
   384	if subthreshold:
   385	    out += ["",
   386	            "NOTE -- a real but SUB-THRESHOLD effect is present in: %s."
   387	            % ", ".join("%s (D=%+dms, T=%dms, sign_p=%.3f)"
   388	                        % (c, cell[c]["D"], round(cell[c]["T"]), cell[c]["p"])
   389	                        for c in subthreshold),
   390	            "These cells are consistent in direction but smaller than the registered",
   391	            "threshold, so they are not a reproduction of P1. They are NOT nothing."]
   392	
   393	out += ["",
   394	        "A NULL (NONE) is judged on the full RANGE -- EVERY pair inside the bound -- not on",
   395	        "the median CI, which at n>8 would TRIM the outliers that contradict it. An EFFECT",
   396	        "uses the CI. That is why bimodality needs no special branch: it cannot hide from",
   397	        "the range.",
   398	        "",
   399	        "The bar/ratio columns are the project's ACCEPTANCE criterion. They are reported",
   400	        "and take NO part in this verdict, which is decided only by the paired CI against",
   401	        "T. sign_p is reported, not decided on. All runs are in summary.csv -- read them.",
   402	        "",
   403	        "Computed from the pre-registered rule. It declares nothing beyond it."]
   404	
   405	open(sess_p, "w").write("\n".join(out) + "\n")
   406	print("\n".join(out))

exec
/bin/zsh -lc "sed -n '1,220p' docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md | nl -ba" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
     2	
     3	**Status**: Pre-registered, **revision 10**. **NO DATA EXISTS YET.**
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

codex
The core classifier is internally exhaustive for valid timing data, but I’m checking the boundaries around it now—especially whether the engine actually pins the registered input domain and whether the control range is converted to a fractional bias without losing pair-level information. Those are the two places most likely to admit a confident result that the four labels themselves would not reveal.
exec
/bin/zsh -lc 'rg -n "ratio|B_ms|B =|RANGE|mutation|guard|32|12 mutations|5 direct|pairs|RUNS|REGISTERED_CELLS|REQUIRED_PAIRS|median" scripts/otp12pf_mac_verdict.py scripts/otp12pf_mac_verdict_test.py docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
scripts/otp12pf_mac_verdict.py:20:    D   = median(d_i)                     low median, even n
scripts/otp12pf_mac_verdict.py:21:    CI  = exact distribution-free order-statistic interval on the population median, the
scripts/otp12pf_mac_verdict.py:24:    RANGE = [min(d), max(d)], and a NULL is judged on the RANGE, never on a trimmed CI.
scripts/otp12pf_mac_verdict.py:27:    outliers, and a bimodal arm then yields a narrow median CI and a FALSE verdict. grok
scripts/otp12pf_mac_verdict.py:28:    drove exactly that with a 16-pair CSV (3 pairs at -500 trimmed away, 13 at +200 left)
scripts/otp12pf_mac_verdict.py:32:    T = min(srcinit_median / 10, DELTA_REF)
scripts/otp12pf_mac_verdict.py:52:      computed on the marginal medians, reported in every row, and never consulted --
scripts/otp12pf_mac_verdict.py:71:# direction of the answer you want. That is the one thing pre-registration exists to
scripts/otp12pf_mac_verdict.py:89:REGISTERED = cells_env("REGISTERED_CELLS") or (MEASURANDS + CONTROLS)
scripts/otp12pf_mac_verdict.py:90:PAIRS = int(os.environ.get("REQUIRED_PAIRS", "8"))
scripts/otp12pf_mac_verdict.py:99:    sys.exit("REFUSING: REQUIRED_PAIRS=%d is not registered %s.\n" % (PAIRS, REGISTERED_PAIRS))
scripts/otp12pf_mac_verdict.py:137:def median_ci(d):
scripts/otp12pf_mac_verdict.py:163:    RATIO: +src/10 reaches ratio 1.10, but only -src/11 reaches the INVERSE 1.10.
scripts/otp12pf_mac_verdict.py:173:    EFFECT/INVERTED use the >=95% CI on the median; NONE uses the FULL RANGE. At the
scripts/otp12pf_mac_verdict.py:178:    NONE uses the FULL RANGE -- EVERY pair must lie inside +-T. Round 8 (codex, BLOCKER):
scripts/otp12pf_mac_verdict.py:179:    a >=95% CI at n>8 TRIMS outliers, so a BIMODAL arm produces a NARROW median CI and a
scripts/otp12pf_mac_verdict.py:181:    reachable by trimming away the very pairs that contradict it. This is also why
scripts/otp12pf_mac_verdict.py:197:    ci = median_ci(d) if d else None
scripts/otp12pf_mac_verdict.py:211:                   ratio=hi / lo if lo else 0.0)
scripts/otp12pf_mac_verdict.py:220:#     B = the arm asymmetry the controls could not rule out, as a FRACTION OF THE ARM,
scripts/otp12pf_mac_verdict.py:221:#         scaled to the cell it is applied to. Taken from each control's full RANGE (not its
scripts/otp12pf_mac_verdict.py:258:    B = B_frac * x["src"]                    # the control bias, on THIS cell's arm
scripts/otp12pf_mac_verdict.py:282:    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,voided,runs\n")
scripts/otp12pf_mac_verdict.py:290:    f.write("cell,n,srcinit_med,destinit_med,ratio,bar,D_ms,CI_lo,CI_hi,range_lo,range_hi,"
scripts/otp12pf_mac_verdict.py:291:            "coverage,T_ms,B_ms,sign_p,k_pos,state,control_state\n")
scripts/otp12pf_mac_verdict.py:298:            c, x["n"], x["src"], x["dst"], x["ratio"], x["bar"], x["D"],
scripts/otp12pf_mac_verdict.py:304:    f.write("comparison,kind,lhs_ms,rhs_ms,ratio,bar\n")
scripts/otp12pf_mac_verdict.py:311:                    % (c, x["src"], x["dst"], x["ratio"], x["bar"]))
scripts/otp12pf_mac_verdict.py:319:    why = ("cells short of their %d pairs, or with a CI below the registered %.0f%% "
scripts/otp12pf_mac_verdict.py:330:           "There is no escalation: a noisy rig is fixed by a QUIETER RIG, not more pairs."
scripts/otp12pf_mac_verdict.py:361:           "size T either way -- this is NOT 'P1 vanishes'. Fix the rig, do not add pairs."
scripts/otp12pf_mac_verdict.py:365:       "Per cell. T = min(srcinit_median/10, %d ms). Controls must be NONE at T/2, and B is"
scripts/otp12pf_mac_verdict.py:371:        out.append("  %-14s INCOMPLETE (%d pairs)" % (c, x["n"]))
scripts/otp12pf_mac_verdict.py:374:               "T=%3dms B=%3dms  ratio=%.3f bar=%s  sign_p=%.3f (%d/%d)"
scripts/otp12pf_mac_verdict.py:377:                  x["ratio"], x["bar"], x["p"], x["k"], x["n"]))
scripts/otp12pf_mac_verdict.py:380:# 7 of 8 pairs. That is not a contradiction and it does not change the verdict, but it
scripts/otp12pf_mac_verdict.py:394:        "A NULL (NONE) is judged on the full RANGE -- EVERY pair inside the bound -- not on",
scripts/otp12pf_mac_verdict.py:395:        "the median CI, which at n>8 would TRIM the outliers that contradict it. An EFFECT",
scripts/otp12pf_mac_verdict.py:399:        "The bar/ratio columns are the project's ACCEPTANCE criterion. They are reported",
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:7:> Per cell, take the **paired** ABBA differences, their median, and one **exact CI**.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:17:> defect. What pre-registration is actually for is kept: the question, the statistic and the
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:160:`RUNS=8`, ABBA-counterbalanced, pair-void. **All six cells must be present and
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:181:prevents post-hoc rationalization.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:183:**What pre-registration is actually for, and what is kept:** the question, the statistic
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:190:      D  = median(d_i)                                  low median, even n
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:191:      CI = EXACT distribution-free order-statistic interval on the population median —
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:206:**ratio**, not in milliseconds.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:217:| **NONE** | **the FULL RANGE** lies inside `(T_neg, T_pos)` — *every* pair, not just the median. An effect of size T is **EXCLUDED** (equivalence) |
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:220:**A NULL IS JUDGED ON THE RANGE, AN EFFECT ON THE CI — and that asymmetry is the point
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:222:**trims outliers**; a **bimodal** arm then yields a *narrow median CI* and a **false null**
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:224:reachable by trimming away the very pairs that contradict it.** A *positive* claim may use
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:225:the CI: pairs clearing T is evidence, and a few stragglers do not undo it.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:245:    B = the arm bias the clean controls could NOT rule out, taken from each control's
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:246:        full RANGE (not its CI — the CI trims, and a bound must never be computed by
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:277:1. **INCOMPLETE** — any registered cell short of its `RUNS` pairs, or with a CI below 95%
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:289:8. **UNCLEAR** — otherwise. **This is not a null.** There is no remedy in more pairs: fix the rig.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:293:- **The 1.10 bar takes NO part in inference.** It is computed on the *marginal medians*,
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:305:  still carry a consistent effect below T (e.g. 99 ms on a 1000 ms arm, on 7 of 8 pairs).
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:312:The old `RUNS=16` escalation is **removed** (owner, 2026-07-14). A null is judged on the
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:313:**full range**, which only **widens** with n — so more pairs could never rescue an `UNCLEAR`
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:319:trimming outliers, and a bimodal arm then yields a narrow median CI and a false verdict:
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:320:grok drove a 16-pair CSV (three pairs at −500 trimmed away, thirteen at +200 left) straight
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:325:**A noisy rig is fixed by a quieter rig, not by more pairs — and `UNCLEAR` says exactly
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:326:that.** Removing it also removes its entire p-hacking guard surface (a "once" marker, a
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:336:pre-registration.**)* To change one: amend this document and put it back through review.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:338:### The guard test
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:341:drove out of a previous revision** — each **mutation-proven** (reverting that fix in a copy
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:342:of the engine makes exactly that case fail: **9/9 mutations killed**), plus a 300-input
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:343:fuzz over the measurand **and** the controls. It runs at preflight, cases *and* mutations;
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:344:a vacuous guard refuses the run.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:346:## The instrument — what round 3 found, and what now guards it
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:353:alone, and the invariance ratio — **the entire measurand** — would have been
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:355:will. The rig would have produced a clean session, 0 voided pairs, and a confident,
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:369:**Two defects that could have MANUFACTURED the result (round-2, still guarded):**
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:385:   The `awk` computing its duration sat in a command substitution with the wrong
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:393:   dispatch difference, **measured at ~15 ms** (median of 5, warm mux, recorded in the
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:456:  harness, the **verdict engine** and its **guard test** are all hashed into the
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:470:- **THE VERDICT ENGINE'S OWN GUARD TEST RUNS AT PREFLIGHT — cases AND mutations.**
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:471:  If the decision rule fails its own cases, or if the proof that guards it turns out
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:472:  to be **vacuous** (a mutation survives), **no data is taken**. *(Round-4, grok: rev
scripts/otp12pf_mac_verdict_test.py:5:    python3 scripts/otp12pf_mac_verdict_test.py --mutations # prove they are not vacuous
scripts/otp12pf_mac_verdict_test.py:12:A mutation reverts one fix in a copy of the engine; the named case must then FAIL.
scripts/otp12pf_mac_verdict_test.py:26:    """Resolved per call: the mutation harness repoints it, and a cached path would
scripts/otp12pf_mac_verdict_test.py:32:            per_cell=None, void_reason="", pairs=8, env_extra=None):
scripts/otp12pf_mac_verdict_test.py:34:    medians and the CI on the PAIRED differences, and the two only disagree when the
scripts/otp12pf_mac_verdict_test.py:36:    unguardable by construction."""
scripts/otp12pf_mac_verdict_test.py:37:    control_d = [5] * pairs if control_d is None else control_d
scripts/otp12pf_mac_verdict_test.py:60:        f.write("cell,pairs_attempted,complete\n")
scripts/otp12pf_mac_verdict_test.py:64:            f.write("%s,%d,yes\n" % (cell, pairs))
scripts/otp12pf_mac_verdict_test.py:66:               CONTROL_CELLS=",".join(CONTROLS), REGISTERED_CELLS=",".join(REGISTERED),
scripts/otp12pf_mac_verdict_test.py:67:               REQUIRED_PAIRS="8", SESSION_VOID_REASON=void_reason)
scripts/otp12pf_mac_verdict_test.py:84:    ("codex r1: a 190ms effect on 7/8 pairs is not a null",
scripts/otp12pf_mac_verdict_test.py:102:     dict(measurand_d=[-20, 300, 310, 320, 330, 340, 350, 360], src=1000),
scripts/otp12pf_mac_verdict_test.py:105:    ("codex r5: a 1ms paired effect is not a reproduction, whatever the medians do",
scripts/otp12pf_mac_verdict_test.py:108:          control_d=[5] * 16, control_src=1000, pairs=16),
scripts/otp12pf_mac_verdict_test.py:113:          control_d=[5] * 16, control_src=1000, pairs=16),
scripts/otp12pf_mac_verdict_test.py:120:    ("grok r9: a LONG cell (16 pairs) is INCOMPLETE — a CI at n>8 TRIMS the pairs that contradict it",
scripts/otp12pf_mac_verdict_test.py:124:    ("a SHORT cell (6 of 8 pairs) claiming complete=yes is INCOMPLETE",
scripts/otp12pf_mac_verdict_test.py:131:          control_d=[-100, -50, 300, 320, 340, 350, 360, 380], control_src=1000),
scripts/otp12pf_mac_verdict_test.py:155:     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000,
scripts/otp12pf_mac_verdict_test.py:168:     dict(measurand_d=[300, 310, 320, 330, 340, 350, 360, 370], src=1000),
scripts/otp12pf_mac_verdict_test.py:197:     dict(measurand_d=[-300, -310, -320, -330, -340, -350, -360, -370], src=1000),
scripts/otp12pf_mac_verdict_test.py:202:          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
scripts/otp12pf_mac_verdict_test.py:203:                    "qn_tcp_mixed": ([-300, -310, -320, -330, -340, -350, -360, -370], 1000)}),
scripts/otp12pf_mac_verdict_test.py:208:          per_cell={"nq_tcp_mixed": ([300, 310, 320, 330, 340, 350, 360, 370], 1000),
scripts/otp12pf_mac_verdict_test.py:209:                    "qn_tcp_mixed": ([-20, 300, 310, 320, 330, 340, 350, 360], 1000)}),
scripts/otp12pf_mac_verdict_test.py:212:    ("codex r8: a bimodal arm cannot hide from the RANGE (a null is judged on every pair)",
scripts/otp12pf_mac_verdict_test.py:275:      '    B = B_frac * x["src"]                    # the control bias, on THIS cell\'s arm',
scripts/otp12pf_mac_verdict_test.py:276:      "    B = B_frac"],
scripts/otp12pf_mac_verdict_test.py:284:    ("the engine trusts meta.complete and never counts the pairs (grok r3)",
scripts/otp12pf_mac_verdict_test.py:287:     "SHORT cell (6 of 8 pairs)"),
scripts/otp12pf_mac_verdict_test.py:305:    CI from the RANGE (with 8 pairs the >=95% interval IS [min,max]). Removing n=16 is what
scripts/otp12pf_mac_verdict_test.py:306:    closed codex's round-8 blocker; judging a NULL on the RANGE is the SEMANTICS that keeps
scripts/otp12pf_mac_verdict_test.py:311:    # re-implementation guard: read the function out of the source and exec it in isolation.
scripts/otp12pf_mac_verdict_test.py:335:    # this ever stops holding, the CI/RANGE distinctions above become live and the engine
scripts/otp12pf_mac_verdict_test.py:339:    st = src2.index("def median_ci(")
scripts/otp12pf_mac_verdict_test.py:346:        lo, hi, cov = ns2["median_ci"](d)
scripts/otp12pf_mac_verdict_test.py:421:    if "--mutations" in sys.argv:
scripts/otp12pf_mac_verdict_test.py:424:        print("\n%d/%d mutations killed" % (len(MUTATIONS) - n, len(MUTATIONS)))
scripts/otp12pf_mac_verdict_test.py:426:    print("The RULE, called directly (a session at n=8 cannot separate CI from RANGE):")

exec
/bin/zsh -lc "python3 -c 'import random; r=random.Random(4242); clean=0; counts={};
for _ in range(300):
 m=[r.randint(-600,600) for _ in range(8)]; ms=r.choice([600,1000,2000,2500,5000]); c=[r.randint(-300,300) for _ in range(8)]; s=r.choice([600,1000,2500,5000]); tp=min(s/10,230)*.5; tn=-min(s/11,230)*.5; ok=tn<min(c) and max(c)<tp; clean+=ok; counts[s]=counts.get(s,0)+ok
print(clean, counts)'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-zCYhxOUl' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-wFwAfFvp' (errno=Operation not permitted)
0 {600: 0, 2500: 0, 5000: 0, 1000: 0}

exec
/bin/zsh -lc "python3 -c 'import importlib.util; p=\"scripts/otp12pf_mac_verdict_test.py\"; s=importlib.util.spec_from_file_location(\"t\",p); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); print(len(m.CASES),len(m.MUTATIONS))'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-V9vmNBM7' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-py5P0MQL' (errno=Operation not permitted)
32 12

codex
- `scripts/otp12pf_mac_verdict.py:83-99` — **BLOCKER** — Registered cell identities and roles come from mutable environment variables. Omitting a dirty control, or excluding one measurand from `VERDICT_CELLS`, can produce `REPRODUCES`/`DOES-NOT-REPRODUCE` despite the preregistration requiring all six fixed cells (`PREREGISTRATION.md:156-163`). No guard covers this.

- `scripts/otp12pf_mac_verdict.py:102-110,161-190` — **HIGH** — Parseable zero/negative timings are accepted. With `src_median=0`, `B=0`, and `CI=[0,0]`, both EFFECT and INVERTED predicates hold; precedence returns EFFECT. Thus the states are not mutually exclusive over inputs the engine itself accepts.

- `scripts/otp12pf_mac_verdict.py:116-134,195-211` — **HIGH** — “Exactly eight” applies only to eight unique paired run keys. Extra duplicate or unpaired valid rows remain in `by` and alter source medians, hence T, B, and the bar. Eight legitimate `d=90, src=1000` pairs plus nine unpaired `src=100` rows grades EFFECT using `T=10`, instead of NONE using `T=100`.

- `scripts/otp12pf_mac_verdict_test.py:105-114,242-300`; `PREREGISTRATION.md:338-344` — **HIGH** — The guard is overstated. There are 32 cases but only 12 mutation targets, not “26 cases, each mutation-proven; 9/9.” The two marginal-versus-paired cases use 16 pairs and therefore pass trivially as `INCOMPLETE`; they do not guard their named defects. The other 20 cases are case-only, including long-n exactness, control-shape cases, boundary/reachability, precedence, and n=1 coverage.

- `scripts/otp12pf_mac_verdict_test.py:303-353` — **MEDIUM** — The reason for direct CI/RANGE checks is correct: at n=8 no session can distinguish them. NULL-on-RANGE and the n=8 identity are checked directly, but B-on-full-RANGE is not. Replacing B’s range with its CI would pass every current check.

- `scripts/otp12pf_mac_verdict_test.py:368-384` — **MEDIUM** — With the fixed seed, all 300 fuzz sessions have dirty controls, so none reaches measurand grading. The assertion also checks only that the session outcome is a recognized word, not that classification is correct.

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:5-13,208-225,245-250` — **MEDIUM** — The preregistration contradicts itself: the summary and state table omit B from NONE and sometimes say CI rather than RANGE. It also says “a few stragglers do not undo” EFFECT, but at registered n=8 the CI is `[min,max]`, so one straggler does undo it.

- `scripts/otp12pf_mac_verdict.py:150-158,206-207,297-301` — **LOW** — `sign_p()` correctly removes zero ties and returns its effective n, but that n is discarded; reporting uses total pairs instead. A test on one positive and seven zero differences is reported as `1/8`, although the sign test used `1/1`.

For valid positive, well-shaped data, the four final states do partition the input, B hardens both directions and never makes a verdict easier, and EFFECT is not weaker because CI equals RANGE at n=8. B can exceed T—for example 4.9% scaled onto a 5000 ms cell gives `B=245 > T=230`—but this safely makes NONE impossible and yields UNCLEAR. Full tempfile-dependent tests could not run in the read-only sandbox; all five direct checks passed.

VERDICT: NOT READY
tokens used
80,129
- `scripts/otp12pf_mac_verdict.py:83-99` — **BLOCKER** — Registered cell identities and roles come from mutable environment variables. Omitting a dirty control, or excluding one measurand from `VERDICT_CELLS`, can produce `REPRODUCES`/`DOES-NOT-REPRODUCE` despite the preregistration requiring all six fixed cells (`PREREGISTRATION.md:156-163`). No guard covers this.

- `scripts/otp12pf_mac_verdict.py:102-110,161-190` — **HIGH** — Parseable zero/negative timings are accepted. With `src_median=0`, `B=0`, and `CI=[0,0]`, both EFFECT and INVERTED predicates hold; precedence returns EFFECT. Thus the states are not mutually exclusive over inputs the engine itself accepts.

- `scripts/otp12pf_mac_verdict.py:116-134,195-211` — **HIGH** — “Exactly eight” applies only to eight unique paired run keys. Extra duplicate or unpaired valid rows remain in `by` and alter source medians, hence T, B, and the bar. Eight legitimate `d=90, src=1000` pairs plus nine unpaired `src=100` rows grades EFFECT using `T=10`, instead of NONE using `T=100`.

- `scripts/otp12pf_mac_verdict_test.py:105-114,242-300`; `PREREGISTRATION.md:338-344` — **HIGH** — The guard is overstated. There are 32 cases but only 12 mutation targets, not “26 cases, each mutation-proven; 9/9.” The two marginal-versus-paired cases use 16 pairs and therefore pass trivially as `INCOMPLETE`; they do not guard their named defects. The other 20 cases are case-only, including long-n exactness, control-shape cases, boundary/reachability, precedence, and n=1 coverage.

- `scripts/otp12pf_mac_verdict_test.py:303-353` — **MEDIUM** — The reason for direct CI/RANGE checks is correct: at n=8 no session can distinguish them. NULL-on-RANGE and the n=8 identity are checked directly, but B-on-full-RANGE is not. Replacing B’s range with its CI would pass every current check.

- `scripts/otp12pf_mac_verdict_test.py:368-384` — **MEDIUM** — With the fixed seed, all 300 fuzz sessions have dirty controls, so none reaches measurand grading. The assertion also checks only that the session outcome is a recognized word, not that classification is correct.

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:5-13,208-225,245-250` — **MEDIUM** — The preregistration contradicts itself: the summary and state table omit B from NONE and sometimes say CI rather than RANGE. It also says “a few stragglers do not undo” EFFECT, but at registered n=8 the CI is `[min,max]`, so one straggler does undo it.

- `scripts/otp12pf_mac_verdict.py:150-158,206-207,297-301` — **LOW** — `sign_p()` correctly removes zero ties and returns its effective n, but that n is discarded; reporting uses total pairs instead. A test on one positive and seven zero differences is reported as `1/8`, although the sign test used `1/1`.

For valid positive, well-shaped data, the four final states do partition the input, B hardens both directions and never makes a verdict easier, and EFFECT is not weaker because CI equals RANGE at n=8. B can exceed T—for example 4.9% scaled onto a 5000 ms cell gives `B=245 > T=230`—but this safely makes NONE impossible and yields UNCLEAR. Full tempfile-dependent tests could not run in the read-only sandbox; all five direct checks passed.

VERDICT: NOT READY
