# ls-4: the apply pipeline shipped one worker, and it cost 2.7×

**Status**: Evidence. 2026-08-01, netwatch-01.
Plan: `docs/plan/LOCAL_SMALL_FILE_PATH.md`.

## Why this was invisible until now

Step (0) measured a converged mirror to an SMB share and found COMPARE at
100% of wall with the apply side at ZERO — which correctly falsified the
plan's L1–L4 apply hypotheses **for that workload**. It is easy to read that
as "apply is never the problem". It is not: the bottleneck follows the
destination.

The owner's screenshot of a local-to-local mirror is what re-opened it.

## Phase breakdown, local NVMe → NVMe, 46,041 files / 7.36 GiB

| phase | share of wall |
|---|---:|
| **APPLY_BACKPRESSURE** | **81.7%** (14.68 s, 1404 payloads, 10.5 ms each) |
| ENUMERATE | 13.6% |
| COMPARE | 12.1% |
| PLAN | 5.0% |
| APPLY (drain) | 0.8% |

The mirror image of the SMB result. 81.7% of the run was the diff loop
BLOCKED handing payloads to a single sink worker — the L3 hypothesis,
measured directly. The drain being ~0 confirms the pipeline was never behind
once fed; it was starved of parallelism, not of work.

## Worker sweep

Same tree, `--workers N` (the hidden diagnostic pin):

| workers | local NVMe→NVMe | SMB destination |
|---:|---:|---:|
| 1 | 17.68 s | 35.03 s |
| 2 | 9.50 s | — |
| 4 | 7.73 s | 35.22 s |
| 8 | 7.19 s | — |
| 16 | 6.58 s | 35.17 s |

Two findings, and the second decided the design:

1. **Local: up to 2.69×**, knee around 4–8.
2. **Network: completely flat, and NO PENALTY at 16.** The destination is the
   constraint there, so client-side apply concurrency neither helps nor
   hurts.

## Why a fixed default, not the `--checkers` treatment

D-2026-08-01-1 says tuning the program can work out at runtime must be worked
out at runtime. Here there is nothing to work out: no measured destination has
an optimum below 8, because the only non-improving case is also
non-degrading. Adding an adaptive throttle to the WRITE path — where per-file
containment (pfc-2/3) and failure classification live — would be real risk
bought for no measured gain, and the apply pipeline cannot resize its worker
set mid-session anyway.

`DEFAULT_SINK_WORKERS = 8`: captures 2.46× of the available 2.69× and sits
just past the knee.

## Result on the owner's tree

    before:  17.68 s   (• Workers used: 1)
    after:    8.70 s   (• Workers used: 8, 866.07 MiB/s)

**2.03× end to end**, measured on the shipped default rather than a pinned
sweep value.

## Caveats

- Two destination classes only: local NVMe and one SMB share. A spinning
  disk or a heavily contended share could plausibly prefer fewer, and there
  is no evidence either way. The hidden `--workers` pin remains the escape
  hatch if that turns up.
- Single run per configuration; no variance estimate.
- The SMB arm used a 3,018-file / 420 MiB subtree, not the full tree, to
  keep the write polite. It shows flatness, not a throughput number.
