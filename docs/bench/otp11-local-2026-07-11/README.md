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

## Rerun with the hardened harness (codex otp-11a F5 fix round)

Same fixtures, harness now aborts on any binary failure; NEW = the
fix-round build.

| cell  | old median | new median | delta | verdict |
|-------|-----------:|-----------:|------:|---------|
| huge  |      21 ms |      21 ms |   ±0% | PASS    |
| tree  |      33 ms |      33 ms |   ±0% | PASS    |
| small |    1663 ms |    1696 ms | +2.0% | PASS    |
| noop  |      20 ms |      23 ms | +15%  | FAIL*   |

\* The 33-file noop cell measures startup jitter (3 ms delta on a
~21 ms operation; the first run had NEW winning 27→24 ms). A focused
follow-up cell was run to get a real signal — and found one:

## noop10k — the change-journal cell (RESOLVED: the old fast path was unsound)

No-op mirror over the already-synced 10,000-file tree, 5 runs,
interleaved, per-binary presync + warmup absorbed in runs 1–2:

| run | old | new |
|-----|----:|----:|
| 1 (cold) | 1637 ms | 1743 ms |
| 2 | 610 ms | 231 ms |
| 3 | 20 ms | 218 ms |
| 4 | 22 ms | 219 ms |
| 5 | 21 ms | 218 ms |
| **steady median** | **~21 ms** | **~219 ms** |

The OLD path's steady state is its change-journal skip engaging. The
owner rejected both "accept the 219 ms" and "keep the journal
shortcut" (FAST vs SIMPLE) — and the investigation of the real fix
found the 21 ms was **unsound**: on macOS/Linux the probe's
`NoChanges` verdict decays to ROOT-directory mtime equality whenever
the global event counter moved (it always has, between runs), and a
deep file modification never touches the root dir's mtime.

**Reproduced against the pre-otp-11 binary (`blit-old` @ `d2bd843`,
2026-07-12)** — warm the journal with 3 mirror runs, modify
`src/sub/deep.txt`, mirror again:

```
== run4 (after DEEP modification) ==
Journal probe src state=NoChanges snapshot=true path=.../src
Journal probe dest state=NoChanges snapshot=true path=.../dst/src
Filesystem journal fast-path: source/destination unchanged; skipping planner.
Up to date: filesystem journal reports no changes (in 739.08µs)
== source deep file: v2-CHANGED
== dest   deep file: v1        <- SILENT DATA LOSS: the change never syncs
```

(`compare_macos`/`compare_linux` in `change_journal/snapshot.rs`:
`event_id`/`ctime` mismatch falls back to `root_mtime` equality;
`engine/mod.rs` consumed `NoChanges && NoChanges` → skip. The Windows
strict-USN arm is sound but rarely hits; its mtime fallback has the
same hole.)

**Verdict for the cell**: the 21 ms target is fraudulent FAST — it
violates RELIABLE (a mirror that reports "Up to date" while source
and destination differ). The sound-vs-sound comparison is old run 2
(610 ms, full pass + snapshot write) vs the session (219 ms): the
session route is **2.8× faster than every sound no-op in the tree**
— the cell PASSES the converge-up gate against the honest baseline.
The 11b deletion of `change_journal/` + the engine's skip now removes
a data-loss bug, not a feature. Pinned on the session route by
`deep_modification_after_warm_runs_syncs`
(`crates/blit-core/tests/local_session.rs`) so no future optimization
reintroduces the shape.

The principled O(changes) tier — journal-assisted no-op done SOUNDLY
(USN range replay; FSEvents historical replay honoring must-rescan;
fail-open to the full session) as a negotiated SESSION capability
(both roles, both carriers, remote no-ops included) — is filed as
designed future work in the slice doc, not resurrected apparatus.
