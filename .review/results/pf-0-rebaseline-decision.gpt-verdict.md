# pf-0-rebaseline-decision — adjudication of the codex review

**Slice**: `d71c0ed` — D-2026-07-14-1 (owner: re-record the committed baselines
at MTU 9000) + propagation into `OTP12_PERF_FINDINGS.md` §pf-0,
`OTP12_ACCEPTANCE_RUN.md` D5, `docs/STATE.md`.
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = ultra` (read from
`~/.codex/config.toml`).
**Raw review**: `.review/results/pf-0-rebaseline-decision.codex.md`
**Verdict**: NOT READY — 3 HIGH, 2 MEDIUM, 1 LOW.
**Adjudication: 6 findings, 6 ACCEPTED, 0 rejected.**
**Fix sha**: recorded below.

The owner's *decision* is not in question and is not reopened. What the review
established is that **the decision as I first wrote it was not executable**, and
that its stated rationale was over-applied. Two findings were serious enough
that following the entry literally would have produced a broken baseline.

---

## HIGH 1 — an unguarded re-record can LOOSEN the very bar it amends → **ACCEPTED**

`OTP12_ACCEPTANCE_RUN.md` D2 exists, in its own words, so that *"the fixed
pre-cutover bar must not be loosened by a slower old rerun"* (design finding F2).
A re-record re-rolls **hardware, OS/disk state and day** as well as MTU — rig W's
Mac end is now **`q`**, not the nagatha that recorded 2026-07-10 — so an
unguarded re-record does exactly what F2 forbids. I proposed re-recording with no
non-loosening guard at all; old-file immutability would have survived while the
*functional* freeze quietly died.

**Fix — applying F2, not inventing a rule**: the acceptance reference for each
cell is the **per-cell MINIMUM of {2026-07-10 median, re-recorded 9000 median}**.
It can only tighten. A cell whose re-record is *slower* is **flagged for
investigation, never silently adopted** — the old build getting slower on faster
hardware would mean the rig or the method drifted, and that must be explained
before anything is graded against it. Recorded in D-2026-07-14-1 and in the D2
amendment note.

## HIGH 2 — D2 still mandated the 2026-07-10 median while D5 called it superseded → **ACCEPTED**

I amended D5 and missed D2, leaving the acceptance contract self-contradictory,
with only one `BASELINE_SUMMARY` per harness to satisfy both. The historical
baseline READMEs also still read as live acceptance references.

**Fix**: D2 carries the amendment note; both baseline READMEs
(`otp2w-baseline-2026-07-10`, `otp2-baseline-2026-07-10`) are re-labelled
**"SUPERSEDED AS THE ACCEPTANCE REFERENCE — retained as a HISTORICAL MTU-1500
record; data unmodified"**, with an explicit *do not cite this as the live
ceiling*.

## HIGH 3 — rig Z has no clean "original OLD build" to re-record with → **ACCEPTED**

My entry demanded "the same OLD build as its original baseline". For rig Z that
build does not exist in clean form: the otp-2 baseline's *client* was a clean
`e757dcc`, but the *daemon that actually ran* was a **dirty `731023b`** — which
D1/D6's clean-matched-pair discipline forbids reusing, while using `e757dcc`
would (by my own wording) "change the reference build". The entry was
unexecutable as written.

**Fix**: rig Z re-records on a **clean `e757dcc` pair**. This is sound and is not
a new reference build: `git diff 731023b e757dcc -- crates proto Cargo.toml
Cargo.lock` is **empty** (the committed daemon code is identical — the otp-2
README's own correction note), and **otp-12a already staged a clean `e757dcc`
rebuild** for its old arm. Precedent, not novelty.

## MEDIUM 1 — the "3–4% faster" rationale is over-applied → **ACCEPTED**

The figure is `wm_tcp_large`, **rig W only**, and **both arms are the NEW build**
(`f35702a`) — it is not a measured old-vs-new leniency. pf-0 measured no small
cells, no rig-Z cells, and no OLD-build MTU response, and its own
committed-reference rows were VOID at jumbo. So "the ceiling is loose by 3–4%"
cannot be generalized across the acceptance matrices. Same failure mode as the
pf-0 review's BLOCKER 1: a real result stretched past its domain.

**Fix**: the justification is restated as **methodological** — *a reference and
the sessions graded against it must share the MTU of the fabric under test*. pf-0
proves the mismatch is real and that MTU moves wall time on at least one cell,
which makes a mismatched ceiling unsound in an **unknown** direction (and lenient
in the one direction actually measured). That is a stronger footing than the
number was.

## MEDIUM 2 — STATE contradicted itself about what is next → **ACCEPTED**

I set STATE's NEXT ACTION to `pf-1` while its own queue still requires the
**Mac↔Mac** experiment before any pf code, and the newest handoff entry still
said the owner baseline decision was next.

**Fix**: NEXT ACTION is now the **Mac↔Mac rig** (Queue 1(ii) — the last
experiment before code; it discriminates H1 outright), then pf-1. Queue item
1(i) is marked `[x]` DONE. The handoff entry's NEXT line is corrected, and the
baseline re-record is labelled a `pf-final` prerequisite, **not** a pf-1 blocker.

## LOW — "P1 is the one finding between blit and shipping" understates P2 → **ACCEPTED**

P2 remains a committed hard both-rigs converge-up bar.

**Fix**: reworded to "the one **class** of finding (P1/P2)".

---

## Independently verified before the review landed (not a codex finding)

The review confirmed my scope claim (rig W and rig Z mismatched, rig D
unaffected). I had verified rig Z directly rather than assuming it: zoey's
pre-jumbo `systemd-networkd` configs, backed up as `*.premtu` and dated
**2026-04-30**, carry **no `MTUBytes` stanza** (i.e. the default 1500), and the
MTU-9000 configs were written **2026-07-13**. Evidence is now cited in the
decision entry and the otp-2 baseline README.
