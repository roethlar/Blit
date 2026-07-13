# win-local-ab — adjudication of the codex review of `4402987`

reviewer: **gpt-5.6-sol**, reasoning effort **ultra** (codex exec, read-only;
codex-cli 0.144.3 — model read from `~/.codex/config.toml`, not assumed)
subject: `scripts/bench_win_local_ab.sh`, `scripts/windows/local-ab-run.ps1`,
`docs/bench/win-local-ab-2026-07-13/`
codex verdict: **"Valid user-level shipping comparison, but not equal work"** —
3 HIGH, 5 MEDIUM
adjudicated by: Claude (author), against source + the rig
outcome: **8 raised → 8 Accepted, 0 Rejected**

This review paid for itself twice: it asked the one question that could have
invalidated the whole finding ("is blit simply doing MORE work?"), and the
answer — **blit does LESS** — became a bug of its own (D-2026-07-13-3),
materially more important than the perf number that started it.

---

## F1 — "The README under-reports execution asymmetries" — **ACCEPTED** (HIGH)

Correct. Default blit hashes nothing (BLAKE3 only under `--checksum`) and does
no verification pass and no per-file fsync — but it *does* do real per-file
bookkeeping (readability probe, manifest-map insert under a mutex, in-process
channel hop, canonical-containment check, availability re-open). The README
described the copy as if the two tools did the same work.
**Fix**: README rev 2 states what blit does and does not pay for, explicitly.

## F2 — "Small-file execution is radically different, including parallelism" — **ACCEPTED** (HIGH)

Correct, and it was the headline error. `/MT:8` vs one apply worker
(`local.rs:602`) is not equal parallelism. The owner caught this
independently and in the same words.
**Fix**: measured. A 4-arm interleaved session (blit ship / blit `--workers 8`
/ robocopy `/MT:1` / robocopy `/MT:8`) now carries the finding. The result
**inverts** at equal concurrency — blit WINS at one thread (0.911 / 0.884) and
loses at eight (1.918 / 1.610) — and the real defect is that **blit does not
scale** (1.05× from 8× the workers, vs robocopy's 2.21×).

## F3 — "Blit is not a semantic superset of robocopy" — **ACCEPTED** (HIGH) — *became its own bug*

Correct, and the most valuable finding in the review. Verified directly on the
rig rather than taken on faith: blit silently drops **Windows attributes and
alternate data streams** on the tar path, and the loss is **conditional on file
count** (`transfer_plan.rs:103-109`). Identical 200 KiB files: 40 → LOST, 3 →
PRESERVED. Proven on the **remote** route too (loopback daemon, exit 0).
**Fix**: `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md`, D-2026-07-13-3,
queued behind otp-12 by the owner. The README now says blit does **less** work,
which makes the comparison *more* unfavourable to blit.
Codex was also right that **empty directories are a documented non-goal**
(`blit check` help, `cli.rs:20-35`) and that **ACLs** are not a gap (robocopy
needs `/COPY:S`). Both recorded as NOT-this-bug.

## F4 — "The H7 reproduction claim is inaccurate" — **ACCEPTED** (MEDIUM)

Correct. The local route sends no `NeedBatch` — it plans and applies directly,
emitting only `NeedComplete` (`mod.rs:3353`). So it exercises H7's
mutex-protected manifest-map insert but **not** the per-need channel hop H7
actually cites. My README called it an H7 reproduction; it is a supplemental
lead and not a pf-1 substitute.
**Fix**: README rev 2 says exactly that.

## F5 — "Blit has an unpinned, timed side effect" — **ACCEPTED** (MEDIUM)

Correct and I had missed it entirely. Perf-history is on by default and written
before the process returns (`context.rs:8`, `local.rs:694`), with a
read/rewrite rotation past a 1 MB cap (`perf_history.rs:443`). That is
**blit-only work inside the Stopwatch** that robocopy has no equivalent of.
`large` parity argues it does not explain the shape-dependent gap, but it is a
real confound.
**Fix**: recorded in the README as a known confound; future runs must disable or
record it. Not retro-fitted to the committed numbers — the recorded evidence
stands on the harness as it ran.

## F6 — "Validity proves counts, not bytes" — **ACCEPTED** (MEDIUM)

Correct. Each run verifies the landed file **count**, not content. A
correct-count, zero-length or truncated tree would pass the gate. My README said
"byte-for-byte", which the harness does not establish.
**Fix**: reworded to *throughput* parity. (Given F3, the trees are in fact
provably NOT equivalent — blit's lacks ADS and attributes — which makes the
original wording doubly wrong.)

## F7 — "The cold-NTFS attribution is unsupported" — **ACCEPTED** (MEDIUM)

Correct. The warm-up and the first timed arm were both blit, so robocopy never
observed the same cell-start state — that confounds tool with order/allocation
state. I had written it up as "blit is disproportionately sensitive to cold NTFS
allocation state", which the data does not support.
**Fix**: downgraded to "an unexplained first-blit outlier"; medians are robust
to it. The 4-arm session's rotating start arm removes the ordering confound.

## F8 — "'Not an otp-11 regression' is too categorical" — **ACCEPTED** (HIGH)

Correct. The otp-11 gate was RUNS=3 on **macOS/APFS**, same-volume. It proves
no material otp-11 regression **on the Mac**; it says nothing about
Windows/NTFS. I stated the conclusion categorically in the body while admitting
"UNMEASURED" in Known gaps — a contradiction inside one document.
**Fix**: the body now says Windows old-vs-new is UNMEASURED and that "old blit
carries this Windows cost" is **not established**. Closing it needs a native
`0f922de` client build on netwatch-01 (`ls-1(b)` in
`docs/plan/LOCAL_SMALL_FILE_PATH.md`).

---

## Codex findings that CLEARED the harness (recorded, since they matter)

- **No timed-window, tree-shape, or flush defect.** Both arms target the same
  precreated `$DestRoot\$leaf`; the E: flush is symmetric and properly charged
  to the destination.
- Removing the flush would **enlarge** the gaps (≈2.37× / 2.35×), so charging
  it is conservative *toward blit*.
- `_Total` in the drain counter is conservative — unrelated disks can cause a
  false timeout, but they cannot conceal E: writing above the threshold.

## Fix commit

README rev 2 + the 4-arm evidence + this verdict.
fix sha: recorded on landing (this file is committed WITH the fix, so the sha
is the commit that contains it — do not invent one here; a fabricated sha was
caught in `pf-0-prereg.gpt-verdict.md` earlier in the same session).
