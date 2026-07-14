# macmac-prereg — adjudication of the codex review (round 1)

**Slice**: `f0343f4` — pre-registration of the Mac↔Mac rig
(`docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`, rev 1).
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = ultra` (read from
`~/.codex/config.toml`).
**Raw review**: `.review/results/macmac-prereg.codex.md`
**Verdict**: NOT READY — 1 BLOCKER, 7 HIGH, 1 LOW.
**Adjudication: 9 findings, 9 ACCEPTED, 0 rejected.** → **revision 2**.

**No data existed when this review ran. That is the entire point** — the review
killed an invalid inference *before* it cost rig time, which is what
pre-registration is for.

---

## BLOCKER — the experiment cannot discriminate H1, and rev 1's premise was false → **ACCEPTED**

Rev 1's headline: *"P1 reproduces macOS↔macOS ⇒ **H1 DIES**, because H1 accuses
the Windows accept branch."*

**H1 does not accuse Windows.** Verbatim in the parent, H1 accuses **blit's own
code paths** — the `SourceSockets` Dial/Accept branches,
`InitiatorReceivePlaneRun.add_dialed_stream`, and the destination's synchronous
dial-before-ACK (`transfer_session/mod.rs:3113`). **The word "Windows" appears
nowhere in it.** Windows is merely *who happens to be the accepting source* in
P1's slow arm on rig W; the accused code runs on macOS too. So a Mac↔Mac
reproduction is **consistent with H1**, not fatal to it — and the parent already
warns that *"'consistent with H1' is not confirmation."*

**Provenance of my error, stated plainly**: I took the framing from
`docs/STATE.md` ("H1 accuses the *Windows* accept branch") and never opened H1.
That is the **second** time this session I propagated a wrong claim about the
hypotheses instead of reading them (the first: "H1/H5/H6/H7" — H5/H6/H7 are P2).
**`docs/STATE.md` is corrected in both places.**

**Fix**: rev 2 reframes the experiment around the question it can actually
answer — **does P1 require the macOS↔Windows PAIRING, or is it a
platform-general cost of the destination-initiated layout?** That is still
decision-relevant (a reproduction closes the "accept P1 as platform residue"
escape and strengthens *every* code hypothesis; a null makes it
pairing-dependent), but it is **not** an H1 kill/confirm, and rev 2 says so in
its first section and again under "What this does NOT establish".

## HIGH — endpoint asymmetry does NOT cancel → **ACCEPTED**

Rev 1 asserted machine asymmetry "cancels within each cell". It does not:
switching the initiator also **reassigns which Mac runs the CLI and which runs
the daemon**, and `q` is the faster machine. Only arm-independent costs cancel;
**host×role interactions do not**, and a shared disk/fsync bottleneck can mask
the effect.

**Fix**: the cancellation claim is withdrawn. Both data directions are measured
and reported separately, and outcome 6 (**MIXED-SIGN → host×role interaction,
inconclusive**) exists precisely because the rig cannot decompose that case.

## HIGH — "requiring both directions" rewrites P1 → **ACCEPTED**

P1's recorded signature on rig W is **one-directional**: `wm_tcp_mixed` FAILS
while `mw_tcp_mixed` PASSES. Rev 1 required *both* Mac↔Mac directions to fail
before calling it a reproduction, and mapped a one-directional result to
"machine asymmetry / inconclusive" — which would have let a **real reproduction
be waved away**.

**Fix**: **a reproduction in EITHER direction suffices** and is reported per
direction. Direction-symmetry is a descriptive fact, not a gate.

## HIGH — "VANISHES" had no power gate (pf-0's exact error, about to repeat) → **ACCEPTED**

The sharpest finding. Eight runs and passing controls do **not** prove the rig
can resolve a ~230 ms effect: the 1.10 bar is a **ratio**, so at a 2.3 s fast arm
a 230 ms effect sits *exactly on* the bar and a "PASS" would mean nothing.
pf-0 reported a KILL with an instrument that could not see the effect it killed;
rev 1 was set up to do it again.

**Fix**: a **POWER GATE evaluated before any null claim** — compute the paired
`MDE` and the ratio a rig-W-sized `Δ_ref = 230 ms` would produce **on this rig's
own fast arm**; if `MDE > Δ_ref` or a ref-sized effect would not breach 1.10
here, the cell is **UNDERPOWERED** and a PASS is **INCONCLUSIVE**, never
"P1 vanishes". A reproduction needs no such gate (an effect that is seen is
seen); a null does. The harness now emits `paired.csv` with
`powered_for_null` per cell so this cannot be skipped.

## HIGH — `N` was not a noise floor → **ACCEPTED**

Rev 1's `N` = max |ratio−1| across four control cells: four point estimates from
**different carriers, fixtures and destinations**, conflating genuine
control-specific initiator effects with sampling noise, ignoring ABBA pairing.
It could equally mask a real effect or bless a fake one.

**Fix**: replaced with the **paired within-cell** statistic — per ABBA slot,
`d_i = destinit_i − srcinit_i`; `D = median(d_i)` is the effect and
`S = spread(d_i)` is the noise. Same slots, same conditions, so no
between-session drift can enter. This is exactly the paired construction pf-0's
own review demanded of pf-1, applied here in advance.

## HIGH — the outcome set was neither exhaustive nor unique → **ACCEPTED**

Codex's counterexamples: `+11%/+9%` fell into "one-direction-only" despite both
exceeding `N`; opposite-sign failures had no mapping; a passing positive plus a
passing inversion could satisfy PARTIAL before INVERSION. And "verdict flips when
inspected" defined **no statistic**, leaving the bistability override post-hoc.

**Fix**: six ordered, mutually exclusive outcomes (RIG-VOID, REPRODUCES,
INVERSION, VANISHES, PARTIAL, MIXED-SIGN), each with a numeric condition on `D`,
`S` and the integer-exact bar; "no outcome may be reported that is not one of
these". The **bistability override is now a statistic**: two clusters separated
by more than `S` **and** a verdict that flips when graded on pooled runs rather
than medians → the cell is reported **UNSTABLE**.

## HIGH — the gates were not fail-closed → **ACCEPTED**

Rev 1 only **warned** on Time Machine autobackup — the very hole pf-0 exposed
(its backup fired 1 minute before the run, and macOS repeats hourly, so one can
start *inside* the window). Spotlight was named as a contaminant but ungated, and
`load1` was recorded with no threshold.

**Fix**: **fail-closed**. Time Machine **running OR autobackup merely enabled** →
refuse to start. Spotlight (`mds_stores`) actively indexing → refuse. `load1 >
3.0` on either Mac → refuse. All implemented in `bench_otp12pf_mac.sh` and
sampled with `top -l 2` (instantaneous), **not** `ps` — whose %CPU is a decaying
average that read a *finished* backup as 255% during pf-0.

## HIGH — "initiator is the only variable" was not instrumented → **ACCEPTED**

No fixture gate, no fail-closed fsync semantics, no macOS drain metric, and a
link check that neither validated both routes nor matched nagatha's recorded
speed.

**Fix**: rev 2 states each as a gate and the harness enforces them — fixtures
verified **by count on both ends** before any timed run; the destination-keyed
`fsync` walk is **timed and fail-closed** (`NA` → pair VOIDs); the drain is a
**named macOS metric** (`iostat`, `<2 MB/s` for 3 consecutive 2 s samples,
`DRAIN-TIMEOUT` voids); peer **ARP must resolve to the peer's MAC** (the
black-hole trap); and an ssh throughput test is explicitly **rejected** as an
instrument (it caps ~79 MB/s either way regardless of link health).

## LOW — the Mac↔Mac-before-pf-1 sequence conflicts with durable guidance → **ACCEPTED**

**Fix**: `docs/STATE.md`'s queue entry rewritten (it carried the retracted "H1
DIES" framing too). The two-experiments-before-code sequence is stated once, in
STATE, and the parent's staging is not restated.

---

**Fix sha**: recorded on the follow-up commit.
**Validation**: `bash scripts/agent/check-docs.sh` → OK; `bash -n` and an AST
parse of the embedded python → OK. (Docs + a new bench script; no `crates/`
change, so the docs gate is the applicable gate per D-2026-07-04-1.)
