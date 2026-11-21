# cr-uv5-6: the benchmark scripts removed a directory of a fixed name

**Severity**: MEDIUM — a directory of that name in the place the script is pointed at loses its contents; two runs at once remove each other's work
**Status**: Verified — `combined-r17` (Codex openreview, 2026-11-21) reviewed `13faafd2..7912da16` — the nine `combined-r16` fixes and local-copy-mode — with `guard_confirmed: true`: "the benchmarks now allocate their own scratch directories."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `aa63eae1`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. It completes cr-uv4-6 (`f40e83b2`).

## Evidence
"Both scripts recursively delete a fixed-name child at startup. An existing directory of that name loses its contents, and concurrent runs can delete each other's work. Create a fresh temporary child and clean up only the directory that invocation created."

cr-uv4-6 stopped `bench_uv4.py` removing the directory it was GIVEN, by having it work in — and remove, before it began — `<where>/blit-bench-uv4`; `bench_uv5.py` was written the same way. A name fixed in advance is not the script's own: it is whoever's made it first.

Shown before the fix, each script cut down in size and nothing else, run twice at once in a directory already holding a directory of its fixed name with a file in it:

```
bench_uv4.py   exit of the two runs: [1, 0] | somebody's file still there: False
bench_uv5.py   exit of the two runs: [1, 1] | somebody's file still there: False
```

(the failures: `FileExistsError` on the other run's `module`, and a listing of 1,700 lines where 2,000 files had been made — the other run had removed them).

## Intake
Admitted as the reviewer put it.

## What
Both scripts make their directory with `tempfile.mkdtemp(prefix="blit-bench-uv4-" | "blit-bench-uv5-", dir=<where>)` — a name no other directory has — and remove exactly that when they end, however they end (`atexit`). Nothing is removed before a run begins. `<where>` is made if it is not there, as before.

## Guard proof
A demonstration, not a test in the suite — these are scripts beside a benchmark's record, and nothing compiles or runs them. The same check after the fix:

```
bench_uv4.py   exit of the two runs: [0, 0] | somebody's file still there: True | left behind: []
bench_uv5.py   exit of the two runs: [0, 0] | somebody's file still there: True | left behind: []
```

Red before and green after, on the scripts as committed either side of this commit; sizes cut to 200 and 2,000 files and one run each for the check, against this tree's debug build.

## Files changed
- `docs/bench/uv4-find-2026-11-21/bench_uv4.py`, `docs/bench/uv5-ls-2026-11-21/bench_uv5.py`.

## Known gaps
- The figures in `result.txt` beside each script were taken with the scripts as they were; nothing measured has changed, and they were not taken again.
- A run killed outright leaves its directory behind (a name of its own, so nobody else's is touched).
- `bench_uv1.py`, `bench_uv2.py` and `bench_uv3.py` (slices 1–3) take a WORK directory that is handed to them whole, by their usage line, and make and remove named things inside it; they are as they were. If the same rule is wanted for them it is the same four lines.
- No test in the suite: the check is `check_bench_scratch.py`, in the session's scratch directory, and is not in the repository.
