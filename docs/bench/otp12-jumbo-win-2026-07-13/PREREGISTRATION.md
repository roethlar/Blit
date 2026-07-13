# otp-12 rig-W jumbo re-run — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered. **No data exists yet.** This file is committed
BEFORE the run so the decision rule cannot be authored after seeing the
numbers. Results land beside it in `README.md` + CSVs; this file is not
edited once data exists (corrections go in `README.md`, marked as such).

**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (**Active**, D-2026-07-13-1);
`docs/STATE.md` Queue 1a names this run as the next action, ahead of any
code. **This is an ENVIRONMENTAL experiment, not a code counterfactual** —
it is not one of H1–H7 and it changes nothing in the tree. It runs first
because it is the cheapest experiment available and it can invalidate the
premise of the expensive ones.

## The question

`netwatch-01` ran at **MTU 1500 for every benchmark ever recorded**
(otp-2w, otp-12a/b/c — `.agents/machines.md` §Network/MTU). Jumbo has
therefore **never once been exercised** by a blit benchmark. P1 — the
headline invariance failure — is the **TCP × mixed × destination-initiator**
cell, i.e. the packet-heaviest fixture we own. If per-packet cost is the
mechanism, ~6× fewer packets is exactly where it would show.

**Falsifiable premise**: MTU 1500 → 9000 is causally responsible for some
or all of P1's 282 ms invariance gap (`Δ_P1(rig W)`, plan §decision rule).

## Instrument validation — performed BEFORE the run

This session has retracted three claims, all from trusting an unvalidated
instrument. So the instrument is validated first, and the validation is
recorded whether or not it flatters the hypothesis.

| check | method | result |
|---|---|---|
| Mac interface MTU | `ifconfig en9` | 9000 (`en9` = **10.1.10.54**, Aquantia) |
| Windows interface MTU | `Get-NetIPInterface` | `NlMtu` **9000** |
| Windows NIC jumbo | `Get-NetAdapterAdvancedProperty` | `Jumbo Packet = 9014 Bytes` (adapter level, not just IP) |
| L2 path, Win→Mac | DF ping, 8972 B payload | **OK**, 0% loss |
| L2 path, Linux→Mac and →Win | skippy `ping -M do -s 8972` | **OK**, both |
| L2 path, Mac→Win | DF ping, 8000 B payload | **OK** (macOS raw-socket cap of 8192 forbids more — the known ping trap, not a network limit) |
| **negotiated TCP MSS, Mac→Win** | **`getsockopt(TCP_MAXSEG)`** on a live socket | **8948** |
| **Mac's advertised MSS** | Linux `ss -ti` toward the Mac | **8948** (so Windows sends 8948 to the Mac) |
| **Win's advertised MSS** | Linux `ss -ti` toward netwatch-01 | **8948** |

`8948 = 9000 − 40 (IP+TCP) − 12 (TCP timestamps)`. At MTU 1500 the MSS was
**1448**. **Segment-count reduction is therefore 6.18×, MEASURED, in BOTH
directions of P1's cell** — not assumed. `getsockopt`/`ss` read the TCP
control block, so this number cannot be faked by offload or coalescing.

**An instrument was tested and DISCARDED.** Windows
`Get-NetAdapterStatistics` reported **10 680 received bytes per "packet"**
during a 1 GiB TCP transfer — *larger than a 9014-byte frame*, so that NIC
coalesces on receive despite reporting `RSC IPv4Enabled: False`. NIC packet
counters **cannot** discriminate 1500 from 9000 on this rig and must not be
cited. (Recorded because it would have "confirmed" jumbo either way.)

## What is held constant, and what is not

| variable | 12b (2026-07-12) | 12c (2026-07-13) | THIS RUN |
|---|---|---|---|
| new-arm sha | `e21cf84` | `f35702a` | **`f35702a`** (same worktree, binaries verified to embed `+f35702a`) |
| old-arm sha | `0f922de` | `0f922de` | **`0f922de`** |
| harness | `bench_otp12_win.sh` | same | **same** |
| Mac NIC | Aquantia @ **10.1.10.54** | TB5 dock @ 10.1.10.91 | **Aquantia @ 10.1.10.54** |
| **Windows MTU** | **1500** | **1500** | **9000** |
| `wm_tcp_mixed` invariance | 1.237 FAIL | 1.300 FAIL | ? |

**Neither prior session is a single-variable control**, and the pre-existing
STATE note ("the Mac's NIC also changed") understates the situation — but
also mis-locates it. The confound is **not** the NIC in general: **12b ran
on this very Aquantia adapter at MTU 1500 and P1 FAILED at 1.237.** So an
Aquantia-vs-dock difference cannot by itself dissolve P1. The one
combination **no session has ever run** is `Aquantia × MTU 1500 × f35702a`.
That — not "the NIC" — is the control this experiment may need.

## Verdict rows that this run VOIDS (important, and not obvious)

The harness grades every cell against **two** references: the same-session
old arm, and the **committed baseline** `otp2w-baseline-2026-07-10/summary.csv`
(`bench_otp12_win.sh:105`, pre-registered and not overridable).

**That committed baseline was recorded at MTU 1500.** At jumbo:

- **Same-session rows stay sound.** Both arms run on today's network, so
  `converge … old_session` and every **invariance** row (new-vs-new, one
  session) are fair comparisons. **P1 is an invariance row — it is
  measured cleanly at jumbo.** So is P2's same-session row.
- **Every `old_committed` and `cross … min_old_committed` row is VOID.**
  They compare a jumbo new arm against a 1500-MTU reference: the network
  improved under the reference, so those rows are flattering by
  construction. A PASS there is not evidence of convergence.

**Consequence for the plan, stated up front:** P2's bar requires ≤1.10
against **BOTH** references (`OTP12_PERF_FINDINGS.md` §Fix criteria). At
jumbo the committed reference is stale, so **P2's committed-reference row
cannot be satisfied until the committed baseline is re-recorded at jumbo.**
This run does not close P2 regardless of outcome. If the fleet stays at
jumbo, `pf-final` needs a re-baselined committed reference — that is a plan
amendment, and it goes through the loop; it is not assumed here.

## Pre-registered predictions and decision rule

Reference values, `wm_tcp_mixed` (12c): dest-initiated arm (`mac_init`)
**1221 ms**, source-initiated arm (`win_init`) **939 ms**, ratio **1.300**;
`Δ_P1(rig W)` = **282 ms**.

**If the MTU premise is TRUE**, the run shows all of:
1. `wm_tcp_mixed` invariance ratio falls to **≤ 1.10**;
2. it falls because the **slow arm speeds up** — `mac_init` drops toward
   939 ms — **not** because the fast arm slows down;
3. **absolute times move somewhere**: packet-heavy cells (mixed, small) show
   real speedups against 12c. This is the run's built-in positive control.
   If literally nothing moves in absolute terms, the measured MSS of 8948
   is contradicted by the wall clock and the run is suspect, not a null.

**Bands (pre-registered, no post-hoc adjustment).** Ratio `r` = the
`wm_tcp_mixed` invariance ratio at RUNS=4:

- **`r ≤ 1.10`** → P1 does not reproduce at jumbo. **This is NOT yet a
  conclusion** — it triggers BOTH confirmations below before any claim is
  recorded.
- **`r ≥ 1.20`** → **MTU is not the cause.** P1 stands (12b 1.237, 12c 1.300
  both sit here). No control run needed: the asymmetry survived a 6.18×
  packet reduction on two different NICs. Proceed to pf-1 unchanged.
- **`1.10 < r < 1.20`** → **INDETERMINATE.** Session-to-session drift on this
  cell is already ~5% (1.237 → 1.300 on the same 1500 network), so a
  4-sample median cannot resolve this band. Escalate to **RUNS=8** on the P1
  cells (the plan's D2 escalation) before saying anything.

**THE MASKING TRAP** (the failure mode that disqualified zoey and altiera as
rigs — `.agents/machines.md`): a ratio can fall toward 1.0 because a *shared*
bottleneck compresses both arms, not because the defect was fixed. Two
concrete guards, both required for `r ≤ 1.10` to count:

- **Fast-arm guard**: `win_init` median must not regress — it must stay
  ≤ 939 × 1.10 ≈ **1033 ms**. If the ratio "passed" because the fast arm got
  slower, that is degradation wearing a PASS, and it is reported as such.
- **Slow-arm guard**: `mac_init` must fall by **≥ 70% of Δ_P1** (≥ 197 ms of
  the 282 ms), i.e. to **≤ 1024 ms**. This is the plan's own ≥70% closure
  threshold (§pf-1 decision rule), reused so the environmental cause is held
  to the same bar as a code cause. A ratio that passes while the slow arm
  barely moved means both arms drifted, not that P1 was fixed.

**Required confirmations before ANY "jumbo dissolves P1" claim is recorded:**
1. **RUNS=8 escalation** on `wm_tcp_mixed` + `pull_tcp_mixed` (P1's bar is
   defined at RUNS=8, plan §Fix criteria).
2. **The control run**: `Aquantia × MTU 1500 × f35702a`, `CELLS=wm_tcp_mixed,
   mw_tcp_mixed,pull_tcp_mixed` — the one combination never run. It
   discriminates the last live alternative: if the control **reproduces**
   ~1.24–1.30, MTU is confirmed causal; if the control **passes at 1500**,
   then MTU is exonerated and 12c's 1.300 was an artifact of the TB5 dock,
   which would retroactively void 12c's P1 row rather than confirm it.
   Note this control requires flipping the Windows MTU back to 1500 and back
   again — a rig change, and it needs the owner's go.

**What no outcome licenses.** Even a clean PASS does not by itself close P1
under the parent plan: `OTP12_PERF_FINDINGS.md` §Fix criteria defines P1's
bar on the netwatch-01 rig at RUNS=8 against both references, and the global
rule requires every other cell to hold too. And it would not close P2, whose
committed reference is void at jumbo (above). **A PASS here changes what the
next experiment is; it does not end the investigation.**

## The run

Full 24-cell matrix, RUNS=4, ABBA, pair-void — a straight replication of the
12c session with MTU as the intended difference. Full matrix rather than a
`CELLS` subset because the controls (`mw_tcp_mixed` opposite direction,
`wm_grpc_mixed` opposite carrier, `wm_tcp_{large,small}` opposite fixture)
are what make P1's cell interpretable, and because cell ordering / cache and
thermal history would otherwise differ from the session being replicated.

```sh
cd /Users/michael/Dev/blit_v2_f35702a          # clean detached worktree @ f35702a
MAC_HOST=10.1.10.54 OLD_CLIENT_PROVENANCE_BY_BUILD=1 \
  bash scripts/bench_otp12_win.sh
```

Staging verified before writing this file: worktree clean at `f35702a`;
`target/release/{blit,blit-daemon}` embed `+f35702a`; Windows has
`D:\blit-test\bins\{0f922de,f35702a}`; no stale daemon on either host; Mac
old client at `~/blit-bench-work/bins/blit-0f922de`.

**Known rig-state change made during validation** (recorded, not hidden):
netwatch-01's `known_hosts` gained the Mac's key at its new IP 10.1.10.54
(it only had the retired .91). No blit code, config, or fixture was touched.
