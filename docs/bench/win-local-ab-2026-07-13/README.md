# blit vs robocopy — LOCAL D: → E: on netwatch-01 (2026-07-13)

**Status**: Evidence (recorded). **This README declares nothing** — it records
what was measured. Adjudication belongs to the owner.

**Why**: owner request — a local-only A/B on the Windows box. It strips the
network out entirely: no MTU, no MSS, no initiator layout, no daemon, no
carrier, no wire. If blit trails robocopy *here*, the problem was never the
wire.

**Harness**: `scripts/bench_win_local_ab.sh` + `scripts/windows/local-ab-run.ps1`.
**blit**: `D:\blit-test\bins\f35702a\blit.exe` (embed-verified `+f35702a`).
`f35702a` is the SHIPPING transfer code — the only delta `f35702a..HEAD` is
`bb28ddd` (cargo fmt on `blit-app/endpoints.rs` + a test), and otp-11 (local
rides the unified session) is already **in** `f35702a`.

**Rig**: D: = disk#0 and E: = disk#3 — two **separate, identical** Crucial
T705 4 TB NVMe drives. No read/write contention on one device; neither side of
the copy is a bottleneck the other lacks.

**Method** (identical to the blit rig harnesses; anything less is not
comparable): cold caches every run (standby purge), writeback **drained**
before the window opens, fresh never-seen destination per run, destination
container precreated **outside** the window on both arms, durability keyed by
the **destination volume** (`Write-VolumeCache -DriveLetter E`, self-timed,
added to wall time), ABBA interleave, pair-void, nonzero exit voids the run,
and the landed file count is verified per run (a tool that "succeeds" while
writing nothing cannot score a fast time).

## Result — two independent sessions agree

RUNS=8, medians in ms (transfer + destination flush):

| fixture | shape | blit | robocopy | ratio | reading |
|---|---|---|---:|---:|---|
| `large` | 1 × 1 GiB | **539** | 541 | **0.996** | parity |
| `mixed` | 512 MiB + 5000 × 2 KiB | **934** | 501 | **1.863** | blit ~1.9× slower |
| `small` | 10 000 × 4 KiB | **1388** | 697 | **1.991** | blit ~2× slower |

Replicated at RUNS=4 in a prior session (`summary-runs4-biased.csv`; see the
ordering-bias note below): 0.991 / 1.877 / 2.024.

**blit matches robocopy byte-for-byte on bulk throughput and costs ~2× the
moment there are many files.** With no network in the picture, that is a
per-file cost inside blit.

## What this does NOT say

- **It says nothing about P1.** A local copy has no initiator axis — there is
  no "who dials" to vary. P1 is an initiator-invariance failure and cannot be
  observed here.
- **It is not an otp-11 regression.** otp-11's own local gate already A/B'd
  old-vs-new blit locally (`docs/bench/otp11-local-2026-07-11/`): `small`
  (10 000 × 4 KiB) went **1684 ms → 1750 ms, +3.9% PASS**. The unified session
  did not introduce this cost; it predates otp-11.
- **It is not P2.** P2 is a *new-vs-old blit* regression on the TCP push path.
  Old blit carries this local cost too (per the otp-11 gate above).
- **It is cross-tool**, so it is NOT a controlled protocol comparison —
  robocopy is a plain Win32 copy loop; blit rides the unified
  `transfer_session` (local included, since otp-11). This is "what a user
  experiences with each tool", which is the SHIPPING bar, not a mechanism
  attribution.

## Known gaps (stated, not hidden)

- **The old blit client was never staged on Windows** — only
  `bins\0f922de\blit-daemon.exe` exists (the otp-12 sessions ran the old
  *client* from the Mac). So "old blit is also ~2× robocopy **on Windows**" is
  an **INFERENCE** from the Mac otp-11 gate, **not a measurement on this box**.
  Closing it requires a native `0f922de` client build on netwatch-01.
- **A cold-allocation outlier survives, and it is a lead, not noise.** blit's
  warm-up (2253 ms) *and* first timed run (2175 ms) in the `small` cell are
  slow; runs 2-8 settle hard at 1386-1430. Robocopy shows the same effect far
  more weakly (742 → ~690). **blit is disproportionately sensitive to cold
  NTFS destination-allocation state (+57% vs robocopy's +8%).** The outlier is
  KEPT in `runs.csv` (medians are robust to it; it inflates `small/blit`
  `spread_pct` to 57.3%). It may itself be informative and is recorded rather
  than tuned away.
- **Ordering bias, found and fixed mid-session.** The first harness revision
  had no warm-up, and ABBA fixes `blit` as slot 1's first arm — so the
  previous cell's teardown (a 10k-file delete still settling past the drain)
  was charged to blit in *every* cell. `summary-runs4-biased.csv` is that
  biased run, kept for the record: it inflated `mixed/blit` spread to 43.2%.
  An untimed discarded warm-up run per cell now absorbs it (`mixed` spread
  43.2% → 4.2%). The medians moved <1% and the conclusion was unaffected.

## Relevance to the ACTIVE plan (`docs/plan/OTP12_PERF_FINDINGS.md`)

Not adjudicated here — surfaced for the owner:

- **H7** accuses HEAD's per-entry bookkeeping (a mutex-protected sent-manifest
  map + a per-need event-channel hop, `transfer_session/mod.rs:1038,:1123,:1350`)
  — **per-file, carrier-independent, shared by both carriers**. Since otp-11,
  local copies ride that same unified session. A network-free local
  reproduction is what H7 predicts.
- pf-1's Method proposes reproducing on "two-daemon in-process rigs on the
  Mac". This local rig is cheaper (~10 min), has no network confounds, and no
  initiator axis to muddy attribution.
- **But note the tension**: the otp-11 gate says old ≈ new locally, while H7
  accuses code that is *new*. If both hold, the local cost is NOT H7 — it is
  older than the unified session, and H7 must be tested against P2's *network*
  gap, not against this. That contradiction is not resolved by this evidence.

## Files

`runs.csv` (48 timed runs, RUNS=8), `summary.csv`,
`summary-runs4-biased.csv` (the pre-warm-up session, kept as the record of the
ordering bias).
