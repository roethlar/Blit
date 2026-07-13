# otp-12pf — the same-OS invariance rig (magneto↔skippy, 2026-07-13)

**Status**: Recorded. **This README declares nothing** — acceptance is the
owner's at otp-13. **Result: P1 does NOT reproduce with Linux on both
ends.**

**⚠ SUPERSEDES an earlier version of this file** which claimed the exact
opposite ("P1 reproduces at 1.78 → the confound breaks toward CODE"). That
claim was **WRONG and is RETRACTED**: it rested on a scratch probe whose
durability accounting was broken (below). The corrected, methodology-clean
session refutes it. The retraction is kept visible rather than quietly
overwritten — the wrong number was reported to the owner and briefly drove
the plan.

## Why this rig exists

P1 (destination-initiated TCP mixed pays ~25–30%) was only ever measured on
rig W (Mac↔Windows). On a **two-host** rig, host identity IS role: in the
slow arm the destination is the Mac (which dials) AND the source is Windows
(which accepts) — inseparable. So P1 was equally consistent with:

1. **code** — the destination-initiator layout is genuinely slow (H1/H5/H6), or
2. **platform residue** — a macOS/Windows write-path or TCP-stack artifact,
   which D-2026-07-12-1 lets the owner accept as satisfied.

This rig removes every platform term: **Linux on both ends**. Endpoints need
not match each other — an invariance comparison holds both endpoints fixed
and varies only the initiator, so endpoint asymmetry cancels *within* each
pair (`ONE_TRANSFER_PATH.md` criterion 1). What zoey failed was the
absolute-speed floor; magneto clears it (owner: "fast enough to saturate
10 GbE where zoey is definitely not").

## Rig

| host | CPU | disk | link |
|---|---|---|---|
| **skippy** | AMD EPYC (32 threads) | ZFS `generic-pool` | 10 GbE (10.1.10.143) |
| **magneto** | Intel, power-efficient (4c) | WD SN850 NVMe (Gen4) | 10 GbE (10.1.10.10) |

Harness `scripts/bench_otp12pf_linux.sh` @ `2c0af86`; binaries `+f35702a`
x86_64-musl both ends (same-build handshake, D-2026-07-05-2). Session
`20260713T134444`: cold caches on BOTH ends every run (`drop_caches` via the
exact NOPASSWD grant — a failed drop voids the pair), destination drained to
quiet, fresh never-seen destination per run, ABBA interleave, pair-void,
RUNS=4, exit codes gated. **64 timed runs, 8/8 cells complete, ZERO voided
pairs.**

## Result — P1 does NOT reproduce

Invariance bar `max/min ≤ 1.10`. `srcinit` = the SOURCE host's CLI pushes;
`destinit` = the DESTINATION host's CLI pulls. **8 / 8 PASS** (7 at RUNS=4;
`ms_grpc_mixed` via its pre-registered RUNS=8 escalation, below).

| cell | srcinit | destinit | ratio | outcome |
|---|---|---|---|---|
| **`sm_tcp_mixed`** — P1's cell | 1745 | 1905 | **1.092** | **PASS** |
| **`ms_tcp_mixed`** — P1's cell | 2085 | 2079 | **1.003** | **PASS** |
| `sm_tcp_large` | 2595 | 2530 | 1.026 | PASS |
| `ms_tcp_large` | 4584 | 5029 | 1.097 | PASS |
| `sm_tcp_small` | 820 | 870 | 1.061 | PASS |
| `ms_tcp_small` | 2135 | 2114 | 1.010 | PASS |
| `sm_grpc_mixed` (carrier control) | 2390 | 2325 | 1.028 | PASS |
| `ms_grpc_mixed` (carrier control) | 4139 | 2974 | 1.392 | **FAIL** → escalated |

**TCP × mixed × destination-initiated — the exact P1 cell — passes at 1.092
and 1.003.** There is no 25–30% destination-initiator penalty with Linux on
both ends.

`ms_grpc_mixed` failed at 1.392 with the **source**-initiated arm slow (the
opposite direction from P1) on spreads of 25.1% / 36.9% — which trips D2's
pre-registered escalation trigger (straddle + >25% arm spread). It reran at
RUNS=8 (`rerun-8pair/`, 16 runs, 0 voided) and, per D2's supersession
amendment, **the RUNS=8 medians govern**:

```
ms_grpc_mixed,invariance,srcinit,destinit,3435,3230,1.063,1.10,PASS
```

So the cell **PASSES at 1.063**; the 1.392 was low-n noise. Spreads stay
high (48.2% / 61.2%) — that cell is simply noisy on this rig, and the
RUNS=4 rows remain committed and visible.

**Governing outcome: 8/8 PASS.**

## Reading (numbers only; no adjudication)

- **P1 requires the Mac↔Windows pairing.** It does not appear when the
  platform terms are removed. The confound is broken — **toward platform,
  not toward code** (the reverse of this file's retracted first version).
  D-2026-07-12-1's platform-residue discriminator is therefore the relevant
  frame for P1 at the otp-13 walk.
- **This does NOT fully exonerate the code.** It rules out a *pure layout*
  property (which would have shown up here), but a code path whose cost only
  becomes material under a specific platform — e.g. a slow accept path on the
  Windows side, which H1 accuses — would look exactly like this. It narrows
  the hypothesis space; it does not close it. H1's dial/accept inversion
  counterfactual on rig W remains the way to finish the job.
- **P2 is NOT tested here.** P2 is a converge bar (new vs OLD push), and this
  rig has no `0f922de` build staged. Nothing in this session speaks to it.

## The bug that produced the retracted claim

The first revision of the harness (and the scratch probe before it) ran the
durability `sync` **inside the initiating host's timed bracket**:

- **push arm**: initiator = the SOURCE, which only READ. Its `sync` was a
  no-op; the destination's writeback was **never paid**.
- **pull arm**: initiator = the DESTINATION, which had just written the
  bytes. It paid the **full writeback**.

One arm was charged for durability the other got for free — multi-second on
skippy's ZFS at 1 GiB. That manufactured invariance "failures" across every
carrier and fixture, worst on the largest files (`ms_tcp_large` 3.285), and
crucially **including the gRPC carrier control** (`sm_grpc_mixed` 1.400)
which is supposed to be clean. **The carrier-independence is what exposed
it**: a real code effect is carrier-specific; an accounting artifact is not.

Fix (`2c0af86`): the transfer window is bracketed on the initiator with **no
sync**; the destination-side sync is then self-timed **on the destination
host** and added to **both** arms identically. A failed sync yields `NA` and
voids the run. `flush_ms` is its own `runs.csv` column so the accounting is
auditable. This is the otp-2w rule — *durability keyed by DESTINATION, never
by verb* — which this harness broke and has re-learned.

Buggy-session numbers are NOT committed as evidence; they exist only in the
retraction note above and in `logs/otp12pf_linux_20260713T133110/`
(untracked).

## Files

`runs.csv` (incl. `flush_ms`), `summary.csv`, `verdicts.csv`, `meta.csv`,
`staging-manifest.txt`, `drain-outcomes.txt`; the RUNS=8 `ms_grpc_mixed`
escalation under `rerun-8pair/`.
