# ls-5 — directory-sweep destination stat

**Status**: Evidence. Single runs, 2026-08-01.
Plan: `docs/plan/LOCAL_SMALL_FILE_PATH.md` (ls-5).

## Premise, measured before any code

One recursive directory-enumeration sweep of `H:\apps` (46,041 files,
7.36 GiB, single-threaded PowerShell) returned every entry's name, size,
mtime and attribute DWORD in **21.19 s** — against ~49 s of serial per-file
stats (ls-1 sub-attribution) spent learning the same facts one file at a
time. `FindFirstFile` carries the metadata in the enumeration itself; the
per-file round trip buys nothing the sweep did not already have.

Also probed first: whether the share can even store named streams, because
a volume that cannot would make the per-file stream check a
guaranteed-constant answer. **It can** — an ADS written to `H:\` came back
intact and enumerable, and `GetVolumeInformationW` advertises
`FILE_NAMED_STREAMS` (flags `0x0005006F`). The capability-gate shortcut is
dead; stream-check elision would be a real semantic change (owner gate,
below).

## What was run

    BLIT_TRACE_LOCAL_PHASES=1 BLIT_TRACE_RUN_ID=ls5-smb-converged-1 \
      blit copy 'D:\Apps\' 'H:\apps\' --dry-run

Release build of the ls-5 tree on netwatch-01, same protocol as ls-1 step
(0) and the checkers run. Log: `logs/ls5-smb-converged-1.log`.

## Result

    Up to date: 46041 files examined, 0 changed (dry run) (in 114.71s)

| run | wall |
|---|---:|
| ls-1 step (0), serial diff | 273.57 s |
| adaptive checkers (checkers.md) | 166.38 s |
| **ls-5 directory sweep** | **114.73 s** |

Phase report (phases overlap; checker threads sum past the wall clock):

| phase | total (summed) | samples | note |
|---|---:|---:|---|
| ENUMERATE | 2.39 s | 1 | |
| ENUMERATE_BACKPRESSURE | 111.87 s | 46,041 | waiting on the diff |
| COMPARE | 114.65 s | 360 | wall span — the whole run |
| COMPARE_STAT | 195.61 s | 46,041 | contains sweep + lock waits |
| COMPARE_SWEEP | 87.26 s | **6,191** | component of COMPARE_STAT |
| COMPARE_METADATA | **915.80 s** | 46,041 | named-stream enumeration |

6,191 sweeps against 6,709 directories under `D:\Apps` — **zero re-sweep
waste**; the 128-directory FIFO window never thrashed on manifest-order
locality (518 directories hold no compared files).

## Local neutrality (A/B, same rig)

Converged `copy --dry-run` of a synthetic 200-dir × 100-file NVMe tree,
baseline binary built from HEAD (`32486690`) in a detached worktree,
3 alternating rounds:

    baseline: 0.73 / 0.71 / 0.71 s      ls-5: 0.72 / 0.71 / 0.72 s

Neutral within noise. The win is round-trip-bound destinations; local
per-file stats were already ~8 µs.

## What it says

**The named-stream check is now the compare.** COMPARE_METADATA's summed
thread time (915.8 s over 46,041 files, ~19.9 ms of thread time per file
under concurrency) dwarfs everything else in the diff; the stat term ls-1
measured at ~49 s serial is gone as a round-trip cost. The remaining wall
is approximately `sweep floor (~21 s single-threaded) + stream checks
under checker concurrency` — which is the ~115 s observed.

**RESOLVED 2026-08-01**: the gate below closed as D-2026-08-01-4 — the
interrogation is deleted from the default compare outright, not elided
conditionally. Evidence: `stream-interrogation-deletion.md`. The paragraph
stands as the state of knowledge when ls-5 landed.

**Next lever, owner-gated:** eliding the per-file destination stream
enumeration when the manifest declares no source streams would drop the
converged compare toward its sweep floor (~25–35 s plausible from these
numbers). It is a fidelity question, not an engineering one: today a stray
ADS added out-of-band to an otherwise-unchanged destination file is
detected and repaired on every run (rel-4); under source-driven checking
it would go undetected until a `--checksum` run or a size/mtime change.
Note the default SizeMtime compare ALREADY accepts exactly this class of
out-of-band divergence for the file's main data stream.

## Caveats

- Single runs throughout; no variance estimate. One SMB server, one tree.
- The dry run performs no attribute repair; a repair-heavy first run is
  unmeasured on the sweep path (repairs ride the same verdicts, so the
  sweep changes what they cost to FIND, not to perform).
- The copying case over SMB is unmeasured for ls-5 (local copying case is
  the neutral A/B above).
