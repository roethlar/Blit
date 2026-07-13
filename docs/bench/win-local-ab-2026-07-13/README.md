# blit vs robocopy — LOCAL D: → E: on netwatch-01 (2026-07-13)

**Status**: Evidence (recorded). **This README declares nothing** — it records
what was measured. Adjudication belongs to the owner.
**Revision 2** — the original headline was WRONG (see §The correction). Codex
review of `4402987`: 3 HIGH + 5 MEDIUM, all accepted
(`.review/results/win-local-ab.gpt-verdict.md`).

**Why**: owner request — a local-only A/B on the Windows box. It strips the
network out entirely: no MTU, no MSS, no initiator layout, no daemon, no wire.
If blit trails robocopy *here*, the problem was never the wire.

**blit**: `D:\blit-test\bins\f35702a\blit.exe` (embed-verified `+f35702a`).
`f35702a` is the SHIPPING transfer code — the only delta `f35702a..HEAD` is
`bb28ddd` (cargo fmt), and otp-11 (local rides the unified session) is already
**in** `f35702a`.
**Rig**: D: = disk#0, E: = disk#3 — two **separate, identical** Crucial T705
4 TB NVMe drives.

## ⚠ The correction (owner, 2026-07-13)

Revision 1 reported "blit is ~2× slower than robocopy". **That was
8-thread robocopy against 1-worker blit.** The harness passed `/MT:8` while
blit's local apply runs **one** worker by default
(`transfer_session/local.rs:602` — `sink_workers` is 1 unless the hidden
`--workers` flag sets `debug_mode`; the CLI prints `Workers used: 1`). It was
also generous to robocopy versus its own default — plain `robocopy /E` with no
`/MT` is single-threaded.

Owner: *"robocopy with /mt:N beats our tar streaming? or was that
single-threaded robocopy?"* — it was `/MT:8`.

## The result (4-arm interleaved session — `summary-4arm.csv`)

All four arms in ONE session, rotating the start arm per slot, so every
comparison is internally controlled. RUNS=6, medians in ms (transfer +
destination flush):

| fixture | blit (ship, 1 worker) | blit `--workers 8` | robocopy `/MT:1` | robocopy `/MT:8` |
|---|---:|---:|---:|---:|
| `large` 1 × 1 GiB | 538 | 538 | 538 | 542 |
| `small` 10 000 × 4 KiB | 1402 | 1336 | 1540 | **697** |
| `mixed` 512 MiB + 5000 | 930 | 784 | 1052 | **487** |

**At EQUAL concurrency, blit WINS:**

| | blit | robocopy | ratio |
|---|---:|---:|---:|
| `small` @ 1 thread | **1402** | 1540 | **0.911** |
| `mixed` @ 1 thread | **930** | 1052 | **0.884** |
| `small` @ 8 threads | 1336 | **697** | 1.918 |
| `mixed` @ 8 threads | 784 | **487** | 1.610 |

**The actual defect — blit does not SCALE:**

| | 1 → 8 |
|---|---|
| blit, `small` | **1.05×** |
| blit, `mixed` | **1.19×** |
| robocopy, `small` | 2.21× |
| robocopy, `mixed` | 2.16× |

blit's per-file path is **not** slow — it beats robocopy at one thread. blit
fails to convert concurrency into throughput, **and it ships one worker**.
`large` is at parity in every arm (a ≥1 MiB file becomes a `File` payload →
one `CopyFileExW`, the same syscall robocopy makes; thread count is irrelevant
for a single file).

## ⚠ blit is doing LESS work, not more (codex F3 — and it is a BUG)

The obvious defence of a slow tool is "it does more". blit does **less**:
it silently drops **Windows attributes (ReadOnly/Hidden/System) and alternate
data streams** on the tar path, on both the local and the remote route
(exit 0, no warning). Robocopy's defaults (`/COPY:DAT`, `/DCOPY:DA`) preserve
them, and `/E` preserves empty directories.

**That is now its own finding** —
`docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md`, D-2026-07-13-3. It makes
this comparison **more** unfavourable to blit, not less. Any fidelity fix adds
per-file work to the tar path, so these numbers get worse before they get
better — which is why the fidelity fix and `LOCAL_SMALL_FILE_PATH.md` are to be
planned together.

blit is NOT paying a durability or integrity premium either: no content hashing
by default (BLAKE3 only under `--checksum`), no verification pass, no per-file
fsync (`sink.rs:368` — "Intentionally no sync_all").

## Instrument notes (all found live; all recorded rather than tuned away)

- **The rig is BI-STABLE for blit and stable for robocopy.** blit's shipped
  config measured **1388 ms** (`summary.csv`) and **2225 ms**
  (`summary-fairA-mt1.csv`) for `small` — identical binary, identical flags,
  flat within each session — while `robocopy /MT:8` read **697 ms** in every
  session. Cause: an 8-thread neighbour leaves the CPU boosted, and blit's
  single-worker, syscall-bound run inherits it; robocopy at `/MT:8` generates
  its own load and is immune. Both single-threaded arms move together
  (`robo_mt1` 2531 → 1540 alongside blit 2225 → 1402), and **the ratio holds**
  (0.879 → 0.911). **CONSEQUENCE: absolute times on this rig are only
  meaningful WITHIN a session.** Cross-session comparison of blit arms is
  invalid. The 4-arm design exists for exactly this.
- **Ordering bias, found and fixed.** With no warm-up, ABBA fixes the first arm
  of slot 1, so the previous cell's teardown (a 10k-file delete still settling
  past the drain) was charged to it every time. An untimed discarded warm-up
  per cell absorbs it (`mixed` spread 43.2% → 4.2%). `summary-runs4-biased.csv`
  is the biased session, kept as the record.
- **The cold-allocation story is NOT established** (codex F7). Warm-up and the
  first timed arm were both blit, so robocopy never saw the same cell-start
  state — that confounds tool with order. The honest statement is "an
  unexplained first-blit outlier"; medians are robust to it.
- **An unpinned blit-only cost sits inside the timed window** (codex F5):
  perf-history is enabled by default and written before the process returns
  (`blit-cli/src/context.rs:8`, `transfer_session/local.rs:694`), with a
  read/rewrite rotation past a 1 MB cap (`perf_history.rs:443`). Robocopy pays
  no equivalent. `large` parity makes it unlikely to explain the shape-dependent
  gap, but it is a real confound and future runs should disable or record it.
- **Validity proves COUNTS, not bytes** (codex F6). Each run verifies the landed
  file count, not content. A correct-count-but-truncated tree would pass. Read
  `large` as *throughput* parity, not byte-for-byte verification.
- Codex found **no timed-window, tree-shape, or flush defect**: both arms land
  the same precreated `$DestRoot\$leaf`, and the E: flush is symmetric and
  properly charged. Note removing the flush would **enlarge** the gaps
  (≈2.37×/2.35×), so charging it is conservative toward blit.

## What this does NOT say

- **Nothing about P1.** A local copy has no initiator axis — there is no "who
  dials" to vary. P1 is an initiator-invariance failure and cannot appear here.
- **It is not an H7 reproduction** (codex F4). The local route sends no
  `NeedBatch` — it plans and applies directly, emitting only `NeedComplete`
  (`transfer_session/mod.rs:3353`). It therefore exercises H7's
  mutex-protected **manifest-map insertion** but **not** its cited per-need
  channel hop. A supplemental lead, not a pf-1 substitute.
- **"Not an otp-11 regression" was too categorical** (codex F8). otp-11's gate
  was RUNS=3, **macOS/APFS**, same-volume, old-vs-new blit
  (`docs/bench/otp11-local-2026-07-11/`). It shows no material otp-11
  regression **on the Mac**. Windows old-vs-new is **UNMEASURED** — the old blit
  *client* was never staged on netwatch-01 (only the old daemon). The claim
  "old blit carries this Windows cost too" is **not established**.
- **Cross-tool wall clock**, so this is the SHIPPING bar, not a controlled
  protocol comparison.

## Files

`summary-4arm.csv` + `runs-4arm.csv` — **the authoritative session** (4 arms,
one session, RUNS=6). `summary.csv` + `runs.csv` — the RUNS=8 two-arm session
(the 8-vs-1 comparison; superseded as a headline).
`summary-fairA-mt1.csv` / `summary-fairB-mt8.csv` — the equal-concurrency pair
that first exposed the inversion. `summary-runs4-biased.csv` — the
pre-warm-up session, kept as the record of the ordering bias.
