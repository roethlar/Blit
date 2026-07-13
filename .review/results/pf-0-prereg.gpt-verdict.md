# pf-0-prereg — adjudication of the codex review of `35b9620`

reviewer: gpt-5.5 (codex exec, read-only)
subject: `docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md`
codex verdict: **NOT READY** — 4 BLOCKER, 3 HIGH, 1 NOTE
adjudicated by: Claude (author), against source/CSVs
outcome: **8 raised → 7 Accepted, 1 Accepted-as-NOTE, 0 Rejected**

The review was worth its cost: it found a **factual error in the premise**
(F6) that the STATE queue entry itself repeats, and it broke the masking
guards (F4) that existed precisely to prevent a false PASS.

---

## F1 — "The run does not isolate MTU; the control is required after EVERY outcome" — **ACCEPTED** (BLOCKER)

Codex agrees `Aquantia x 1500 x f35702a` is the right control (not "the NIC
changed"), but says it is needed on **every** outcome, not just a PASS, and
must be run at the **same scope** as the jumbo session.

Verified and correct. My design let a FAIL license "MTU is not the cause".
It does not: a FAIL proves only that jumbo is *insufficient to dissolve* P1.
MTU could still be a **CONFIRMED CONTRIBUTING** cause under the parent's own
20-50% band (`OTP12_PERF_FINDINGS.md:522`) while P1 still fails its 1.10 bar.
Without a matched 1500 arm there is no `Δ_1500` to grade any recovery against.

Fix: both MTUs are now measured, at identical scope and RUNS, back to back on
the same NIC and sha. The experiment's measurand is the **change in the
invariance gap between the two MTU conditions**, not a single ratio.

## F2 — "The `r >= 1.20` causal rejection is invalid" — **ACCEPTED** (BLOCKER)

At `win_init` = 939, `r` = 1.20 implies `mac_init` ≈ 1127, i.e. a
(1221−1127)/282 = **33% recovery** of `Δ_P1` — which the parent plan grades as
**CONFIRMED CONTRIBUTING**, not as a kill. My band would have rejected a cause
the governing plan calls real. Arithmetic verified.

Fix: the ad-hoc ratio bands are deleted. MTU is graded on the parent's
**uniform pre-registered scale** (`OTP12_PERF_FINDINGS.md:516-527`), against a
`Δ` that is now actually measured.

## F3 — "RUNS=4 cannot support definitive calls; the 5% drift is not a noise estimate" — **ACCEPTED** (HIGH)

Correct on both counts. The 1.237 → 1.300 movement I called "session drift"
came from sessions that differed in **NIC and sha**, so it is not a noise
estimate at all — citing it as one was the same class of error as the rest of
this session's retractions. And the parent defines P1's bar at **RUNS=8**
(`OTP12_PERF_FINDINGS.md:548`).

Fix: **RUNS=8** in both MTU conditions. Band arithmetic is specified as the
harness's exact integer form (`10*hi <= 11*lo`, `bench_otp12_win.sh:668`), not
the 3-decimal printed ratio.

## F4 — "The masking guards admit the masking artifact" — **ACCEPTED** (BLOCKER)

Devastating and correct. A shared 1000 ms floor passes **all three** of my
guards: ratio 1.000 ≤ 1.10, fast arm 1000 ≤ 1033, slow arm 1000 ≤ 1024. The
guards were porous exactly where they were supposed to be strongest. My 70%
slow-arm threshold was also not the parent's closure definition — the parent's
`Δ` is the **arm difference** (`OTP12_PERF_FINDINGS.md:501`), not a slow-arm
absolute.

Fix: with the matched 1500 control (F1), masking becomes **directly
observable** rather than inferred — if both arms are slower at 9000 while the
ratio improves, that is degradation, and it is reported as degradation. The
fast-arm guard is now relative to its own 1500 measurement, not to a
hard-coded number.

## F5 — "MSS validates capability, not blit's treatment" — **ACCEPTED** (BLOCKER)

Correct, and the distinction matters. `getsockopt(TCP_MAXSEG)` = 8948 proves
the **path's negotiated ceiling**. It does **not** prove blit's data plane
*fills* those segments: application write boundaries, Nagle, and record
framing could leave segments short of the MSS regardless. I measured the
ceiling and wrote as if I had measured the fill.

Also correct that my prediction 3 was backwards: **MSS 8948 with unchanged
wall time is a legitimate null** ("per-packet cost is irrelevant to blit"), not
evidence the instrument lied. I had made a real possible result unfalsifiable.

Fix: the claim is downgraded to what was measured (ceiling, both directions,
6.18x *available* segment reduction). Segment **fill** is stated as unmeasured.
The "if nothing moves, the run is suspect" rule is **deleted** and replaced
with a positive control that can fail honestly (F6).

## F6 — "The packet-load premise is mis-specified" — **ACCEPTED** (HIGH) — *the factual error*

Correct, and this one invalidates the stated rationale. At MSS 1448 the
segment counts are:

| fixture | bytes | segments @1448 | segments @8948 |
|---|---|---|---|
| **large** | 1 073 741 824 | **~741 500** | ~120 000 |
| mixed | 547 110 912 | ~377 800 | ~61 100 |
| small | 40 960 000 | ~28 300 | ~4 600 |

**`large` is the packet-heaviest fixture, by ~2x over mixed** — not `mixed`.
My doc, and `docs/STATE.md:5` / the Queue 1a entry, both assert mixed is "the
most packet-heavy fixture we test". That is **false**. What is distinctive
about `mixed` is the *interleaving* of one bulk stream with 5000 small files,
not packet count.

Fix: premise restated (jumbo cuts per-packet overhead across all TCP cells;
`mixed` is P1's cell because that is where the failure was *observed*, not
because it is packet-heaviest). `wm_tcp_large` is added as the **bulk-packet
positive control** with a pre-registered, falsifiable threshold. STATE.md's
claim must also be corrected — filed below.

## F7 — "The void-row inventory is incomplete" — **ACCEPTED** (HIGH)

Correct. I listed `old_committed` and `cross ... min_old_committed` but missed
that every block-1 **`combined`** row also embeds the committed leg
(`bench_otp12_win.sh:697-702`: `combined` is PASS only if *both* `p1` and `p2`
hold). Codex's evidence is exact: 12b's P2 reads `FAIL-BOTH` while 12c's reads
`FAIL-SAME-SESSION` *solely* because the committed leg flipped.

Fix: `combined` rows added to the void list.

## F8 — "Running the experiment does not violate governance" — **ACCEPTED as NOTE**

Agreed, and it matches my reading: the run lands no fix, changes no wire
contract, and STATE sequences it before code. Recorded explicitly: **a PASS
licenses evidence for a plan amendment only** — it cannot reshape pf-1,
rebaseline, or close P1/P2 without a reviewed amendment.

---

## Deferred / filed elsewhere

- **`docs/STATE.md:5` and Queue 1a repeat the F6 factual error** ("TCP x mixed
  — the most packet-heavy fixture we test"). That is a STATE correction, not a
  pre-registration edit; it lands with the revision commit.

## Fix commit

`docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md` rewritten (F1-F7),
`docs/STATE.md` corrected (F6). Fix sha: recorded below on landing.

fix sha: `7921adc`
