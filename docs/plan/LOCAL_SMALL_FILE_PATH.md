# LOCAL_SMALL_FILE_PATH — the local carrier frames small files for a wire that isn't there

**Status**: Draft
**Created**: 2026-07-13
**Owner decisions required**: see §Decisions (none assumed; no code until Active)
**Parent**: none. This is **not** P1 and **not** P2
(`docs/plan/OTP12_PERF_FINDINGS.md`) — see §Not-that-finding.
**Contract**: `docs/TRANSFER_SESSION.md`. **No wire change is in scope.** The
tar shard is the *wire* payload format and must remain exactly as it is for
remote sessions; this plan touches only the **local apply carrier**, which
crosses no transport. Any slice that would alter a frame stops and amends the
contract through the loop first.

## The observation

Local `D: -> E:` copy on netwatch-01, two separate identical NVMe drives, no
network, no daemon, no initiator, no carrier negotiation
(`docs/bench/win-local-ab-2026-07-13/`, RUNS=8 medians):

| fixture | blit | robocopy `/MT:8` | ratio |
|---|---:|---:|---:|
| `large` 1 × 1 GiB | 539 ms | 541 ms | **0.996 parity** |
| `mixed` 512 MiB + 5000 files | 934 ms | 501 ms | 1.863 |
| `small` 10 000 × 4 KiB | 1388 ms | 697 ms | 1.991 |

**⚠ THAT COMPARISON IS NOT CONCURRENCY-FAIR** (owner, 2026-07-13): robocopy
ran 8 threads; blit's local apply runs **one** worker by default
(`transfer_session/local.rs:602` — `sink_workers` is 1 unless the hidden
`--workers` flag sets `debug_mode`). It is also generous to robocopy relative
to its own default (plain `robocopy /E` with no `/MT` is single-threaded).
**The equal-concurrency sessions are the ones that attribute anything**, and
this plan does not proceed past ls-1 without them.

## The mechanism (code-read; execution on the local route confirmed)

`blit copy` → `run_local_session` (`transfer_session/local.rs:511`) →
`diff_chunk_and_apply_local` (`transfer_session/mod.rs:3362`). The byte path
**forks on size** (`transfer_plan.rs:72`, `:106`):

- **≥ 1 MiB** → `PreparedPayload::File` → `copy_resolved_file_payload`
  (`sink.rs:473`) → `windows_copyfile` → **`CopyFileExW`**
  (`copy/windows.rs:363`) / `clonefile` on APFS (`copy/file_copy/clone.rs:15`).
  One syscall, kernel-side. **The same call robocopy makes** — hence exact
  parity on `large`.
- **< 1 MiB** → **tar shard**. Per file, on a copy where **no bytes cross any
  transport**:
  1. `build_tar_shard` (`payload.rs:254-293`) opens the file and streams it
     into an **in-memory tar `Vec<u8>`** — 512 B header + data + 512 B padding
     (a 4 KiB file becomes ~5 KiB of framing);
  2. `safe_extract_tar_shard` (`tar_safety.rs:173-183`) **parses that tar and
     memcpys every file's bytes into a second `Vec<u8>`**;
  3. `create_dir_all(parent)` **per file** (`sink.rs:600`), even though all 128
     files in a shard usually share one parent;
  4. `fs::write` (`sink.rs:603`), then a **separate destination open** —
     `set_file_mtime` (`sink.rs:606`) = `CreateFileW` + `SetFileTime` + close —
     to stamp a timestamp `CopyFileEx` sets inline;
  5. plus **two probe opens per source file before the real read**: a
     readability probe in the scan (`remote/transfer/source.rs:142`) and an
     availability probe at plan time (`source.rs:260`).

So a 4 KiB file costs roughly **3 source opens + 2 destination opens + a
`create_dir_all` + 2 full memcpys**, against robocopy's **1 `CopyFileEx`**. On
NTFS `CreateFile` is the dominant per-file cost. Reads are additionally
**serial**: one pipeline worker, and `prepare_payload` does not overlap
`write_payload` (`pipeline.rs:196-206`).

**Why the framing exists at all**: the tar shard is the *wire* format for
small files — it amortizes per-file round trips across a network. otp-11's D1
adopted it for the local carrier by **behavior preservation**, not by design:
the plan says the local apply uses "`PreparedPayload::File`/`TarShard`
**exactly as the old local pipeline**, same parallelism, same zero-copy
primitives" (`docs/plan/OTP11_LOCAL_SESSION.md` D1). D1's zero-copy analysis
is entirely about **large** files (protecting clonefile/block-clone). **Nobody
ever asked whether a local copy should frame small files at all.**

## Not-that-finding (what this is NOT)

- **Not P1.** P1 is an initiator/verb *invariance* failure. A local copy has
  no initiator axis — there is nothing to vary. This cannot speak to it.
- **Not P2.** P2 is a *new-vs-old* regression on the TCP push path. This cost
  is **not** new: otp-11's own local gate measured old-vs-new at `small`
  1684 ms → 1750 ms, **+3.9% PASS** (`docs/bench/otp11-local-2026-07-11/`), and
  D1 says the local carrier reproduces the old pipeline's payload shapes. The
  old local path tar-sharded small files too.
- **Not a regression at all** — it is a long-standing, unexamined inheritance.
- **⚠ UNMEASURED**: the old blit *client* was never staged on Windows (only the
  old daemon), so "old blit is ~2× robocopy **on Windows** too" is an
  inference from the macOS otp-11 gate. `ls-1` closes that gap or states it.

## Hypotheses (each names the mechanism it accuses)

- **L1 — tar framing on a wireless path.** Encode + parse + 2 memcpys + ~1 KB
  framing per 4 KiB file, buying nothing locally
  (`payload.rs:254-293`; `tar_safety.rs:173-183`).
- **L2 — per-file syscall count.** ~5 handle opens per file vs robocopy's ~1
  (`source.rs:142`, `:260`; `sink.rs:600`, `:603`, `:606`).
- **L3 — single apply worker.** `sink_workers = 1` by default
  (`local.rs:602`); reads serial, no read/write overlap (`pipeline.rs:196-206`).
  Robocopy was given 8 threads.
- **L4 — `create_dir_all` per file** rather than once per shard
  (`sink.rs:600`).

L1/L2/L4 are per-file path costs; **L3 is a concurrency default**. They are
independent and may all contribute. **L3 is measured first** because if it
dominates, the others are a much smaller prize than the headline 2× suggests.

## Attribution rule — pre-registered, uniform (no post-hoc bands)

Borrowed deliberately from `OTP12_PERF_FINDINGS.md` §pf-1 decision rule, so an
environmental/config cause is held to the same bar as a code cause.

- **`Δ_local(fixture)`** = `median(blit) − median(robocopy)` **at equal
  concurrency**, per fixture, on this rig. It is NOT the 8-vs-1 number.
- Every hypothesis needs a **wall-time counterfactual**: a variant that removes
  exactly the accused mechanism, run interleaved against the unmodified build,
  same rig, same fixture. A hypothesis with no counterfactual **cannot be
  confirmed** — it is carried as UNTESTED and this plan does not close.
- Recovery `r` = share of `Δ_local` removed, graded:
  - `r ≥ 50%` → **CONFIRMED DOMINANT**
  - `20% ≤ r < 50%` → **CONFIRMED CONTRIBUTING**
  - `r < 20%` → **KILLED** as a material cause
- **Overlapping causes are attributed SEQUENTIALLY, never summed** (L1 and L2
  both remove handle opens; summing would double-count). Grade each solo, then
  measure the **incremental** recovery of adding it to the already-applied set.
- Closes only when confirmed contributions account for **≥ 70% of `Δ_local`**.
  Otherwise the residue is stated in the probe record — never "several
  hypotheses were consistent, moving on".

## Fix criteria (pre-registered)

- **Bar**: at **equal concurrency**, blit ≤ robocopy on `small` and `mixed`
  on this rig. (A separate question — whether blit's *default* worker count
  should change — is a Decision below, not a bar.)
- **`large` must stay at parity.** The ≥1 MiB `File` payload path
  (`CopyFileExW` / `clonefile`) is the thing otp-11 D1 fought to protect. Any
  slice that costs the zero-copy primitive **fails**, regardless of small-file
  wins.
- **No remote regression, and no wire change.** The tar shard remains the wire
  format. A local-carrier fix must be scoped to the local apply path; if a
  slice would touch code shared with the wire sink, it must prove the remote
  cells unmoved.
- **No suite regressions**; floor = the current count. New pins carry guard
  proofs (temporary revert) per the loop.

## Slices (each through the codex loop; NO code before ls-1 lands)

- **ls-1 (HARD GATE) — attribution, no behavior change.**
  (a) the equal-concurrency A/B (`ROBO_MT=1`/`BLIT_WORKERS=0` and
  `ROBO_MT=8`/`BLIT_WORKERS=8`) — establishes `Δ_local` and grades **L3**;
  (b) build the `0f922de` client natively on netwatch-01 and A/B old-vs-new
  locally **on Windows**, closing the UNMEASURED gap above;
  (c) instrument the local apply (behind the existing debug flag): per-file
  open counts, tar encode/decode time, `create_dir_all` time, so L1/L2/L4 have
  numbers before anyone edits them.
  Probe record committed and codex-reviewed **before any fix slice exists**.
- **ls-2..n** — one fix slice per CONFIRMED cause, smallest change first,
  A/B'd against the unmodified build on the same rig.
- **ls-final** — re-run the full local matrix (`large`/`small`/`mixed`) at both
  concurrencies, plus the remote no-regression check.

## Decisions

1. **Priority — SETTLED (D-2026-07-13-2): BEHIND.** This plan is sequenced
   behind the ACTIVE `OTP12_PERF_FINDINGS.md` (MTU experiment → pf-1 → its fix
   slices). The finding is recorded now; only the *fix* waits. Rationale in the
   decision entry: the local cost is very unlikely to explain P1 (no initiator
   axis; a per-file/worker cost cancels between two arms of the same code) or
   P2 (P2 is *new*, this cost is *old*), while fixing it first would touch code
   shared with the wire sink and void otp-12's pre-fix baselines.
   **Carried into pf-1 as a cheap check** — the one way the two could touch: if
   the unified session changed the **remote receive** worker count the way the
   local side sits at one (`local.rs:602`), that WOULD be new, per-file, and a
   live P2 candidate. Read the executed old path; do not assume.
2. **OPEN — the core design question.** Should the local carrier skip tar
   framing for small files and copy each one directly (the same primitive
   `large` already uses), accepting that local and remote payload shapes then
   differ for small files? **Do not answer this from the current evidence** —
   see the hypothesis re-ranking below; L1 may not be worth touching at all.
3. **OPEN — the default worker count.** Local apply ships at one worker
   (`--workers` is a hidden debug flag). Should the shipped default change?

## ⚠ HYPOTHESIS RE-RANKING (2026-07-13, after the equal-concurrency runs)

The headline "blit is 2× robocopy" was **8-thread robocopy vs 1-worker blit**
(owner caught it). At **equal concurrency** the picture inverts:

| fixture | blit | robocopy | ratio |
|---|---:|---:|---:|
| `small` @ 1 thread | 2225 | 2531 | **0.88 — blit WINS** |
| `mixed` @ 1 thread | 942 | 1045 | **0.90 — blit WINS** |
| `small` @ 8 threads | 1331 | 697 | 1.91 — blit loses |
| `mixed` @ 8 threads | 790 | 502 | 1.57 — blit loses |

Scaling from 8× the concurrency: **blit 1.67×** (small) / 1.19× (mixed);
**robocopy 3.63×** / 2.08×.

- **L3 (single worker / no scaling) is now the PRIME suspect.** blit does not
  convert concurrency into throughput.
- **L1 (tar framing) is DEMOTED.** It cannot be the dominant cost of a tool
  that *wins* at one thread. It may still be why blit scales badly (a serial
  tar-build that cannot overlap writes — `pipeline.rs:196-206`), which is an
  L1×L3 interaction, not L1 alone. `ls-1` must separate them.
- **⚠ THESE NUMBERS ARE NOT YET TRUSTED.** blit's absolute time is **bi-stable
  across sessions** — the identical binary and flags measured 1388 ms and
  2225 ms for `small` in two sessions, flat within each, while `robocopy /MT:8`
  read 697 ms in both. Working hypothesis: CPU boost/core-parking residency —
  an 8-thread neighbour leaves the CPU hot, and blit's single-worker,
  syscall-bound run inherits it, while robocopy at `/MT:8` generates its own
  load and is immune. **If true, blit's honest single-worker cost is ~2225 ms
  and the 1388 ms figure was an artifact of what it was benchmarked next to.**
  A 4-arm single-session run (every arm sharing identical neighbours) is the
  control; no attribution is made until it lands. Cross-session comparison of
  blit arms is INVALID until this is explained.
