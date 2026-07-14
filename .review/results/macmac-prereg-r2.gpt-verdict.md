# macmac-prereg — adjudication of codex round 2

**Slice**: `8375c0a` + `e1e351d` — pre-registration rev 2 + the new harness
`scripts/bench_otp12pf_mac.sh`.
**Reviewer**: `gpt-5.6-sol` @ `ultra` (per `~/.codex/config.toml`).
**Raw review**: `.review/results/macmac-prereg-r2.codex.md`
**Verdict**: **NOT READY — 3 BLOCKER, 6 HIGH, 2 LOW.**
**Adjudication: 11 findings, 11 ACCEPTED, 0 rejected.** → **revision 3 required
before the rig may run.**

**The rig is NOT cleared to run.** Round 1 killed the experiment's central
inference; round 2 shows the *replacement* inference is still overclaimed, the
statistics cannot support a null, and the harness does not implement its own
decision rule. No data has been taken, which is the point — but this is now two
consecutive rounds where the design, not the prose, was wrong.

---

## BLOCKER 1 — rev 2 substitutes ANOTHER false dichotomy → **ACCEPTED**

Rev 2 claims: *reproduces ⇒ "platform-general cost of the layout" (not platform
residue); vanishes ⇒ "P1 requires the Windows peer".* **Both halves overreach.**

- A reproduction on **these two Macs** is equally consistent with a **macOS/APFS
  or host×role residue**. "Not Windows-specific" does **not** imply
  "platform-general".
- A null licenses only **"P1 did not reproduce on this macOS↔macOS pair"** — not
  "Windows is required". The pair is two specific machines with specific disks.
- Rev 2 also asserts a reproduction "closes the platform-residue escape". **There
  is no such escape on the books**: the parent states plainly that D-2026-07-12-1
  does *not* cover P1 (P1 *is* the invariance failure its precondition requires),
  so P1 today has **no escape hatch** and none can be closed. That sentence
  invents a consequence.

**Required in rev 3**: state only what the rig licenses —
*reproduces* ⇒ **P1 does not require a Windows peer** (nothing more);
*vanishes* ⇒ **P1 does not reproduce on this pair** (nothing more) — and name
macOS-specific / host×role residue as live alternatives to a reproduction.

## BLOCKER 2 — the power gate is broken, and the counterexample is damning → **ACCEPTED**

`S = max(d_i) − min(d_i)` is a **range**, not an MDE, not a precision estimate,
and not an equivalence bound; at n=8 it grows with the sample. The registered
"powered" test also never compares the observed `D` to the reference effect, and
it takes the fast arm as `min(src,dest)` rather than the **source-initiated
baseline**.

Codex's counterexample, which my rule blesses:

> `srcinit = 2000×8`, `d = [0,180,180,190,190,200,200,200]`
> → ratio 1.095 **PASS**, `D = 190`, `S = 200`, `powered = yes` → **VANISHES**

Seven of eight pairs positive, an effect **83% of the 230 ms reference**, and the
rule reports *P1 is absent*. That is precisely the class of error pf-0 committed
(a null from an instrument that could not see the effect) — reproduced here in a
document written to prevent it.

**Required in rev 3**: a genuine paired **equivalence** procedure —
distribution-free CI on the median of `d_i` (at n=8 the order statistics
`[d₍₂₎, d₍₇₎]` give a ≈93% interval), with:
- **REPRODUCES** iff the cell FAILS the bar **and** the CI lower bound > 0;
- **VANISHES** iff the CI **upper** bound < the pre-registered equivalence margin
  `Δ_eq` (the bar in ms on this rig: `0.10 × median(srcinit)`), **and** that
  margin is itself below the reference effect being excluded;
- **UNDERPOWERED/INCONCLUSIVE** whenever the CI is too wide to do either.
`D` alone never decides anything.

## BLOCKER 3 — the harness implements none of the rule → **ACCEPTED**

`compute_verdicts` emits per-cell `PASS/FAIL/INCOMPLETE` only. It contains **no**
rig-validity (control) gate, **no** clustering/bistability statistic, **no** power
gate applied to a verdict, and **none** of the six outcomes. The registered rule
therefore lives only in prose — meaning a human applies it **after seeing the
numbers**, which is exactly what pre-registration exists to prevent. The six
outcomes are also still overlapping (MIXED-SIGN is shadowed by
REPRODUCES/INVERSION) and incomplete (FAIL with |D| ≤ S; sub-bar negative
asymmetry; incomplete cells are unmapped; UNDERPOWERED and UNSTABLE are extra
outcomes never listed among the six).

**Required in rev 3**: the harness **computes the session verdict itself** — one
exhaustive, mutually exclusive decision tree, emitted as a single machine-readable
line — and the prose merely describes what the code does.

## HIGH 1 — durable time is two disjoint intervals → **ACCEPTED**

The single-process monotonic rewrite fixed the clock bug, but the transfer window
**ends before the call returns**, and the fsync walk **begins after another
dispatch + interpreter startup** — so arm-dependent writeback can occur *free*, in
the gap. The destination is also declared **drained before** `sync; purge` runs,
with no re-drain, and non-numeric `iostat` output coerces to `0` and reads as
"quiet".

## HIGH 2 — landing/data-shape validation is fail-open → **ACCEPTED**

Fixtures are checked **by count only** (a truncated/wrong-size tree passes), and
`os.walk()` over a **missing or empty** landed path silently visits nothing and
prints `F:0:F` — i.e. **an unlanded transfer is accepted as a valid 0 ms flush.**
Must assert file count **and byte sum** at the landed path, and a zero-file walk
must VOID.

## HIGH 3 — the ARP gate is cosmetic → **ACCEPTED**

A failed ping is ignored; *any* complete ARP entry passes; the peer MAC is never
compared to the known value; interface/MTU/media are unchecked; and the
`q`→nagatha direction is never tested. The documented **own-MAC black hole** and
the **wrong-NIC route** both pass this gate as written.

## HIGH 4 — the environmental gates are not fail-closed → **ACCEPTED**

Every probe **fails open**: a `tmutil` error reads as "not running"; an AutoBackup
read failure is explicitly coerced to `0`; a Spotlight probe failure emits `0`,
and a trailing idle `mds` row can overwrite a hot `mds_stores` sample. A parse
failure must VOID, never pass.

## HIGH 5 — the registered protocol is unenforced → **ACCEPTED**

`RUNS=2` is accepted (the registration says 8); an arbitrary `CELLS` list can omit
**every control** yet still receive verdicts and `powered=yes`; the harness's own
documented default runs **12 cells, not the registered six**; drain thresholds are
overridable and unrecorded. The harness must refuse anything that is not the
registered protocol, or label the output NON-REGISTERED.

## HIGH 6 — instrument provenance is weaker than binary provenance → **ACCEPTED**

Binaries reject `+sha.dirty`, but the **harness itself** is labelled only with the
committed `HEAD` — an edited worktree is invisible. sha256 outputs are not
validated as 64 hex chars, so the manifest can claim "4 hashes" while holding
empty values.

## LOW 1 — median flooring + an unspecified IQR → **ACCEPTED**

Even-`n` medians are floored *before* the "integer-exact" bar, permitting a
half-millisecond boundary flip. The IQR is an ad-hoc `x₆−x₃` at n=8 and can report
zero where Tukey hinges report a large spread.

## LOW 2 — the sequencing conflict was only half-fixed → **ACCEPTED**

I changed `docs/STATE.md` only. The **active plan** still says pf-1 starts now, and
`docs/DECISIONS.md` still records the settled MTU→pf-1 sequence, while rev 2
inserts Mac↔Mac before pf-1. Durable guidance still disagrees with itself.

---

## Status

**Rev 3 is required before any timed run.** Nothing is lost — no rig time was
spent, and two independent instrument bugs (the cross-process `time.monotonic()`
subtraction, and the landed-path semantics) were caught by live validation before
this review even ran. But the honest summary is that **the Mac↔Mac design has now
failed review twice on substance**, and the next revision must make the *harness*
the authority (it computes the verdict) rather than the prose.
