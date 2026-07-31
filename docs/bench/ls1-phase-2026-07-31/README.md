# ls-1 step (0) — where the wall clock goes on the owner's workload

**Status**: Evidence. One run, 2026-07-31.
Plan: `docs/plan/LOCAL_SMALL_FILE_PATH.md`.

## What was run

    BLIT_TRACE_LOCAL_PHASES=1 BLIT_TRACE_RUN_ID=ls1-step0-dapps \
      blit copy 'D:\Apps\' 'H:\apps\' --dry-run

Release build at `906f1bdc`, on netwatch-01. Source `D:\Apps` = 46,041 files
/ 7,902,792,937 bytes on local NVMe. Destination `H:\apps` = the owner's SMB
share, already converged by their 2026-07-31 field runs.

`--dry-run`, not `mirror`, deliberately: the question is where a converged
run spends its time, and a mirror against a live backup volume is
destructive if anything on the destination has since diverged. Dry run plans
without writing or deleting. It reproduces the shape of the owner's second
field run — which copied 0 files, 0 B in **283.92 s** — at **273.57 s**.

## Result

    Up to date: 46041 files examined, 0 changed (dry run) (in 273.57s)

| phase | total | samples | % of wall |
|---|---:|---:|---:|
| ENUMERATE | 2.99 s | 1 | 1.1% |
| ENUMERATE_BACKPRESSURE | 269.40 s | 46,041 | 98.5% |
| **COMPARE** | **273.51 s** | **360** | **100.0%** |
| ATTRIBUTE_REPAIR | 0 | 0 | 0% |
| PLAN | 0 | 0 | 0% |
| APPLY_BACKPRESSURE | 0 | 0 | 0% |
| APPLY | 0 | 1 | 0% |
| DELETE | 0 | 0 | 0% |

Phases overlap by design (the SOURCE and DESTINATION drivers are joined
concurrently), so these do not sum to the wall clock — see the module docs
on `phase_probe.rs`. `session_failed: false`.

## What it says

**The destination diff owns the entire wall clock.** COMPARE is 273.51 s of
a 273.57 s run. 360 samples is 360 manifest chunks (46,041 / 128), so each
chunk costs ~760 ms, or **~5.9 ms per file** — the cost of stat-ing one file
across SMB, paid serially, 46,041 times.

**The source walk is not the problem.** ENUMERATE is 2.99 s. The walk reads
the whole 46k-file tree off local NVMe in three seconds.

**The enumerate/backpressure split earned its place immediately.** The
source task's own wall span was ~272 s, of which 269.40 s was spent BLOCKED
on the bounded manifest channel waiting for the destination to catch up.
Without the split (cr-ls1-1's sibling problem, fixed before this run),
ENUMERATE would have read as ~272 s and this run would have indicted the
source walk — which is 1.1% of the cost.

**It falsifies the plan's hypothesis set for this workload.** L1–L4 in
`LOCAL_SMALL_FILE_PATH.md` are all apply-path: tar framing, per-file
syscalls, one sink worker, per-file `create_dir_all`. On this run the entire
apply side — PLAN, APPLY_BACKPRESSURE, APPLY — is zero. None of L1–L4 can
explain a second of the owner's 273 seconds.

**`Workers used: 1` is not the cause of THIS complaint.** Single-worker apply
is a real defect and remains priority-1, but it is not what makes the owner's
converged mirror take four and a half minutes. Raising the worker count
would not have moved this number.

## What it does not say

- One run, one workload, one destination. No repetition, no variance
  estimate, no comparison against another tool. This selects a phase to
  attribute; it does not grade a fix.
- A dry run performs no attribute repair (pfc-6 disables it when the
  destination writes nothing), so ATTRIBUTE_REPAIR: 0 here is a property of
  the run mode, not evidence that repair is free. The owner's first field
  run repaired 5,445 files and that cost is unmeasured.
- The copying case is unmeasured. This is the converged case only.
- Whether the ~5.9 ms per file is SMB round-trip latency, Windows metadata
  enumeration (ADS/attributes), or blit's own per-file work inside the diff
  is NOT resolved here. That is the next question, and it is a sub-phase
  question inside COMPARE.
