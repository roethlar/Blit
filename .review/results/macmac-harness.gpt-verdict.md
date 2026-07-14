# macmac-harness — adjudication of the codex review (round 1)

**Slice**: `e1e351d` — `scripts/bench_otp12pf_mac.sh`, the Mac↔Mac harness
(+ pre-registration rev 2).
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = ultra`.
**Raw review**: `.review/results/macmac-harness.codex.md`
**Verdict**: NOT READY — 3 BLOCKER, 6 HIGH, 1 MEDIUM, 1 LOW.
**Adjudication: 11 findings, 11 ACCEPTED, 0 rejected.**

**No data has been taken. The instrument was reviewed before it measured
anything** — which is the only reason none of this became a retraction. Three of
these would have produced a *confidently wrong result*.

---

## BLOCKER 1 — the harness does not compute its own registered rule → **ACCEPTED**

The pre-registration defines six ordered outcomes, a RIG-VOID gate, a power gate
and an UNSTABLE override. The harness emits only per-cell `PASS/FAIL` plus paired
stats. **The session verdict would therefore have been applied BY HAND, after
seeing the numbers** — which is exactly what pre-registration exists to prevent.
Codex also notes the prose tree is itself still overlapping/incomplete.

**Fix**: the harness must mechanize the rule end-to-end and emit a
`session_verdict.txt` (RIG-VOID / REPRODUCES / INVERSION / VANISHES / PARTIAL /
MIXED-SIGN / INCONCLUSIVE-UNDERPOWERED / UNSTABLE), with the prose tightened to
match exactly.

## BLOCKER 2 — the noise statistic would have declared a REAL effect "vanished" → **ACCEPTED**

`S = max(d) − min(d)` is a **range**, not an MDE or an equivalence bound: it grows
with n and is dominated by outliers, so a *large, consistent* effect can hide
under it. Codex's counterexample, which my code accepts:

    srcinit = 2000 ms (×8);  d = [0, 180, 180, 190, 190, 200, 200, 200]
    -> D = 190, S = 200, bar = PASS, powered = yes
    -> |D| <= S  =>  "VANISHES"

…despite **7/8 pairs positive** and `D` at **83% of rig W's Δ_P1**. Repeated in
both directions it would have declared "P1 requires the Windows peer" off an
effect nearly the size of P1 itself. This is pf-0's underpowered-null error
wearing a power gate.

**Fix**: replace the range with a real paired inference —
- **bootstrap 95% CI on median(d_i)** (n=8, resampled in-process, no scipy);
- **exact sign test** (k of 8 positive, two-sided binomial);
- **REPRODUCES** requires bar FAIL **and** CI lower bound > 0;
- **VANISHES** requires bar PASS **and** the CI **upper** bound below the
  bar-breaching effect for that cell (`0.10 × srcinit_median` — the effect that
  would push the ratio to 1.10), i.e. a genuine **equivalence** result;
- otherwise **INCONCLUSIVE** (and **UNDERPOWERED** when the CI is too wide to
  exclude a bar-breaching effect).

## BLOCKER 3 — the registered inference still overreaches → **ACCEPTED**

Rev 2 narrowed rev 1's "H1 dies" to "platform-general cost of the layout". Still
too strong. A reproduction proves only that **P1 can occur without a Windows peer
on THIS pair**; a null proves only **non-reproduction on this pair** — not that
Windows is *required* (it could be a property of these two specific machines,
disks, or macOS versions).

**Fix**: rev 3 scopes every claim to *this pair*, and states the residual
alternatives explicitly. (This is the third tightening of the same claim; the
lesson is that each round I stated a conclusion one step broader than the design
could carry.)

## HIGH — the fsync walk is fail-open, and nothing checks that bytes landed → **ACCEPTED** *(found independently by the author before the review returned)*

`os.walk()` on a missing, unreadable or empty path emits a perfectly valid
`F:0:F` — **a missing tree reads as a fast, successful flush**. The push and pull
landed paths are *currently* correct (verified empirically: a push to
`/bench/RUNDIR/` lands `RUNDIR/src_<W>`; a pull into `RUNDIR` lands the files
directly in `RUNDIR`), but that is **luck, not a guard** — and there is **no
destination count or byte-sum check**, so an exit-0 zero-byte or partial transfer
becomes a valid *fast* row. This is the otp-2w bug's exact shape.

**Fix**: the fsync walk returns `F:<ms>:<files>:F`; the harness **VOIDs the pair**
unless the landed file count equals the fixture count **and** the landed byte sum
matches. Source fixtures get a byte-sum check too, not just a count.

## HIGH — transfer and fsync are disjoint intervals, and the free-writeback gap REVERSES BY DIRECTION → **ACCEPTED**

The sharpest finding, and the one that could have *manufactured the result*.
Between the client exiting and the fsync starting, the OS writes back dirty pages
**for free** (charged to neither interval). That gap is **longer for whichever arm
ran over ssh**, because the ssh return trip happens first:

    cell nq (src=nagatha, dest=q):  srcinit = LOCAL client,  destinit = REMOTE client
    cell qn (src=q, dest=nagatha):  srcinit = REMOTE client, destinit = LOCAL client

So the favoured arm **flips sign with the data direction**. P1's whole signature is
*one-directional* — meaning this artifact is capable of **producing a
one-directional "reproduction" out of nothing**. Codex also notes `prep_run`
certifies the drain *before* `sync; purge` and never re-checks it.

**Fix (needs an owner decision — see below)**: make the client launch **symmetric**
so neither arm carries an ssh return the other lacks. Also re-order `prep_run` so
the drain is certified *after* the purge, and re-checked.

## HIGH — environmental gates fail OPEN → **ACCEPTED**

`pgrep` errors read as "quiet"; `tmutil` errors/empty parse to zero; an AutoBackup
**read error explicitly becomes "disabled"**; `top` failures become zero and a
trailing idle `mds` sample can overwrite a busy one; malformed/empty `load1`
becomes 0. Every one of these fails toward "go".

**Fix**: each gate must fail **closed** — an unreadable gate is a VOID, never a
pass. (This is the same class as pf-0's `ps` decaying-average trap: an instrument
that cannot answer must not answer "fine".)

## HIGH — the ARP/link gate does not prove the link → **ACCEPTED**

It ignores ping failure, accepts *any* complete MAC without comparing it to `q`'s
**known** MAC (so the documented **own-MAC black hole** passes), and never checks
the q→nagatha direction or that the route uses the 10GbE NIC rather than falling
back to 1GbE.

**Fix**: compare against the recorded peer MAC, check **both** directions, and
assert the route egresses the 10GbE interface — plus the existing rule that an ssh
throughput test is **not** a valid link check.

## HIGH — the registered protocol is unenforced → **ACCEPTED**

`RUNS>=2` is accepted (the design says 8); a misspelled `CELLS` can silently drop
every control or measure nothing; blank `CELLS` runs **12** cells, not the six
registered. Overridable drain thresholds are not recorded in the evidence.

**Fix**: validate `CELLS` against the registered set, require the registered
`RUNS`, and record every threshold in the manifest.

## HIGH — instrument provenance is weak → **ACCEPTED**

The manifest records `HEAD`, so a **modified** harness still claims the reviewed
commit; `sha256_of` accepts empty/malformed hashes; and `! grep` turns a
*read error* on the dirty-marker check into "clean".

**Fix**: hash the harness file itself into the manifest, refuse a dirty harness,
and make hash/provenance failures fatal.

## MEDIUM — daemon liveness and teardown → **ACCEPTED**

`nc -z` proves only that a handshake reached *some* listener's backlog — not that
the captured PID accepts or speaks blit. Teardown logs "verified gone" when the
ssh/`ps` probe *itself* failed, and cleanup discards a positively detected
survivor.

**Fix**: probe with a real blit call (the smoke), and treat a survivor or an
unverifiable teardown as fatal.

## LOW — median/IQR conventions → **ACCEPTED**

Even-sample medians are floored before the "exact" bar and the `D > S`
comparisons; the n=8 IQR convention is unstated. Codex confirms the ABBA void
retry, slot pairing and the `destinit − srcinit` sign are otherwise **correct**.

**Fix**: state the convention and apply it consistently.

---

## The one finding that needs the owner: symmetric client launch

Fixing the free-writeback asymmetry requires the two arms to be launched
identically. The options are an infrastructure choice, not a code choice:

- **(A)** drive the harness from a **third host** (skippy/magneto) so **both**
  Macs are remote and symmetric — needs ssh keys from that host to both Macs;
- **(B)** keep the driver on nagatha but launch **both** clients over ssh,
  including nagatha→itself — needs a host key + `authorized_keys` entry on
  nagatha;
- **(C)** equalize with a fixed settle window before the fsync on both arms —
  no infra change, but it lets writeback complete "for free" for both arms and so
  weakens what destination-keyed durability is meant to charge.

Recorded for the owner; **no rig time until it is resolved**, because this
artifact is capable of manufacturing exactly the one-directional result the
experiment is looking for.
