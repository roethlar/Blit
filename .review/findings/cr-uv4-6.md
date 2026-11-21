# cr-uv4-6: the benchmark script removed the directory it was given

**Severity**: MEDIUM — a script in the repository destroys the contents of an existing directory passed to it
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv4-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. The defect is in utilities slice 4 (`35adeacb`).

## Evidence
"The new script recursively deletes the supplied work directory at startup. Passing an existing directory destroys its contents before measurement begins. Create and clean a private child directory."

Cause. `docs/bench/uv4-find-2026-11-21/bench_uv4.py` began with `shutil.rmtree(work, ignore_errors=True)` on its first argument, to start from nothing. Its author always gave it a directory made for it; the script did not say that it had to be one.

## Intake
Admitted as the reviewer put it.

## What
`bench_uv4.py` makes a directory of its own — `blit-bench-uv4` — inside the one it is given, works there, and removes that directory when it starts and when it ends. The directory given, and whatever else is in it, is not touched. Its usage text says so. `bench_uv5.py` (slice 5) was written the same way before it landed, the review having arrived first.

## Guard proof
No test in the suite runs a benchmark script. Run by hand, 2026-11-21, on Linux (`continuum`), each with a directory holding one file, `keep-me.txt`:
- the script as it was (`git show 35adeacb:docs/bench/uv4-find-2026-11-21/bench_uv4.py`): the file is gone as soon as the script starts;
- the script as it is: the file is there when the script ends, and `blit-bench-uv4` is not.

## Files changed
- `docs/bench/uv4-find-2026-11-21/bench_uv4.py` — the private directory; the usage text.
- `docs/bench/uv4-find-2026-11-21/README.md` — a line saying so.

## Known gaps
- The numbers in that directory's README and `result.txt` were taken with the script as it was; they are not taken again.
- `bench_uv1.py` and `bench_uv2.py` remove directories BY NAME inside the one they are given (`module`, `system`, …), not the directory itself; an existing directory holding entries of those names would lose them. They are as they were. `bench_uv3.py` removes nothing.
- Nothing stops a later script from being written the old way; this is a rule in a record, not a check.
