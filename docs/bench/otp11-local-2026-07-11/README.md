# otp-11a local perf gate — A/B evidence (2026-07-11)

**Status**: Evidence (gate PASSED)
**Gate** (docs/plan/OTP11_LOCAL_SESSION.md, 11a step 4): per cell, the
session route's median ≤ the old local orchestration's median + 10%.
**Harness**: `scripts/bench_otp11_local_ab.sh`, RUNS=3, interleaved
(old,new per round) to level cache state.

## Rig

- Owner's macOS machine (Darwin 25.5.0, APFS same-volume, `$TMPDIR`),
  the clonefile-sensitive environment that motivated D1's direct
  carrier.
- OLD binary: release build @ `d2bd843` (pre-otp-11: local rides
  `TransferOrchestrator`/`TransferEngine`), built in a detached
  worktree.
- NEW binary: release build @ `dfdddd6` (otp-11a: local rides
  `run_local_session`, in-process session + local apply carrier).

## Cells and medians

| cell  | shape                                   | old median | new median | delta | verdict |
|-------|-----------------------------------------|-----------:|-----------:|------:|---------|
| huge  | 1 GiB single file, copy, fresh dest     |      22 ms |      22 ms |   ±0% | PASS    |
| tree  | 256 MiB + 32 small dirs, copy, fresh dest |    31 ms |      33 ms | +6.5% | PASS    |
| small | 10,000 × 4 KiB files, copy, fresh dest  |    1684 ms |    1750 ms | +3.9% | PASS    |
| noop  | mirror over already-synced tree         |      27 ms |      24 ms | −11%  | PASS    |

Overall: **PASS** (harness exit 0; raw per-run lines in the harness
log, workspace `otp11_ab.1ODzqm`).

## Reading

- **huge at 22 ms both sides** is the load-bearing result: the local
  apply carrier goes through `FsTransferSink`'s File-payload path and
  keeps APFS clonefile — a byte-relay carrier would have copied
  1 GiB through channels here (the failure mode the slice doc's D1
  rejected).
- **noop** measures the session diff against the old `no_work` fast
  path (both are full enumerate+stat passes; the retired journal skip
  did not engage on this rig — see the slice doc's D3/F8 note). The
  session diff is slightly faster.
- **small** (+3.9%) and **tree** (+6.5%) are within the gate; the
  session adds hello/open/manifest-frame overhead per run
  (~1–2 ms) and per-chunk diff scheduling, amortized by the shared
  planner/pipeline.
