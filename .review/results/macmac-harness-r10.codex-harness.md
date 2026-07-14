Reading additional input from stdin...
OpenAI Codex v0.144.3
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f6192-fbf2-74f1-b61b-073c33ac516d
--------
user
Shell correctness review of one bash script: scripts/bench_otp12pf_mac.sh (read only that file, plus docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md for what it is supposed to do).

It is a benchmark harness on the owner's own two Macs. It runs file transfers between them, times them, and records the numbers. It runs entirely on hardware the owner controls, and no data has ever been taken with it. It uses ssh to the second machine, sudo to drop the page cache between runs, and pgrep/iostat/top to check the machines are idle before it measures anything. Those are the measurement mechanics; the review is about whether they are CORRECT, not about what they are.

The thing at stake: this harness has been reviewed ten times and every round found a defect that would have produced a WRONG MEASUREMENT. Two defect classes keep coming back, and you should assume both are still present:

(a) THE FIX WAS APPLIED WHERE IT WAS SHOWN, NOT WHERE IT LIVED. A fail-open `pgrep` was fixed
    in one gate and left identical in a second. The drain was fixed by validating its VALUE
    and left failing by discarding its EXIT STATUS. Find the next duplicate.

(b) A PROTECTION THAT NEVER EXECUTES, OR CANNOT FAIL. The equal pre-fsync settle -- the one
    thing neutralising an artifact that could otherwise manufacture the result -- HAD NEVER
    RUN FOR THREE REVISIONS: an awk inside a command substitution had the wrong quoting, so
    `sleep` got an empty argument and failed, and its exit status was discarded by the python
    that followed. `bash -n` sees nothing. It is now performed and MEASURED inside the same
    python process as the fsync walk, reported as settled_ms, and the pair VOIDS if it did
    not elapse. Find the next protection that does not actually execute, or that executes but
    can never fail.

Check specifically:
1. Every exit status that a following command can discard: pipelines (a pipeline's status is
   the LAST command's), command substitutions, `|| true`, `&&` chains, and any value that is
   validated without checking that the probe producing it succeeded.
2. Every gate that reads a number: can an error, an empty string, or a non-number read as a
   passing value? (`.` once passed a digits-and-dots test, read as 0, and certified an idle
   disk.)
3. The slot-major scheduler (run_all_cells / run_one_pair): the void/retry accounting, the
   ABBA arm alternation, the per-slot cell rotation, the CSV rows, and whether a cell that
   exhausts its attempts can corrupt another cell. Note macOS ships bash 3.2: no associative
   arrays, no mapfile, no ${x^^}.
4. Quoting through ssh: the script sends command strings through `bash -c` twice (once
   locally, once remotely), with heredocs inside them. Where else could an escape be wrong in
   the way the settle's awk was?
5. `set -euo pipefail` interactions, subshell state loss (a function that sets a global,
   called inside `$(...)`, loses it -- this has bitten twice), and the EXIT trap.
6. Does the harness do what the pre-registration says? Any claim in the doc the script does
   not implement, or behaviour it has that the doc does not register?

You may run the script's own self-test (`SELFTEST=1 bash scripts/bench_otp12pf_mac.sh`), which exercises the gates and takes no data, and you may read its output.

Output a concise markdown findings list (file:line, severity BLOCKER/HIGH/MEDIUM/LOW, rationale), then a final VERDICT line: READY or NOT READY. Be concise; do not invoke skills.
codex
I’ll trace the script against the preregistration, then run only its `SELFTEST=1` path. I’ll focus the report on measurement-corrupting defects and avoid commenting on the intended mechanics themselves.
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
codex
The static trace has already exposed one preregistration mismatch in the build gate. I’m running the permitted self-test now to distinguish live gate failures from defects that only appear in the timed scheduler.
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_job started
mcp: ptk/ptk_job (completed)
codex
- `scripts/bench_otp12pf_mac.sh:102,488-510,1088` — **BLOCKER** — `REGISTERED_BUILD` is never enforced. Any `EXPECT_SHA` is accepted; `EXPECT_SHA=f35702a.dirty` can even certify a `.dirty` binary because the dirty probe then searches for `.dirty.dirty`. Contradicts `PREREGISTRATION.md:135-138,466-470`.

- `scripts/bench_otp12pf_mac.sh:459-479` — **BLOCKER** — Five preflight RTT samples’ median is not a bound on later per-run SSH return/dispatch gaps. A delayed timed-run SSH can provide asymmetric free writeback without being measured or voided, preserving the artifact this protection claims to bound.

- `scripts/bench_otp12pf_mac.sh:742,859` — **HIGH** — `run_one_pair` is called as an `if` condition, disabling `errexit` throughout its call tree. A failing `time_argv` that emitted a parseable sentinel is therefore accepted; its exit status is discarded by the following value check.

- `scripts/bench_otp12pf_mac.sh:365-368` — **HIGH** — Spotlight’s raw CPU field is coerced with `$2+0`. A malformed/non-numeric `mds_stores` value becomes zero, is reformatted as numeric `0`, and certifies idle.

- `scripts/bench_otp12pf_mac.sh:682-685` — **HIGH** — `sync; sudo ... purge` reports only `purge`’s status. A failed `sync` followed by successful `purge` marks the run `cold`.

- `scripts/bench_otp12pf_mac.sh:782,788` — **HIGH** — Timed-tree deletion failures are discarded with `|| true`. A large/mixed landed tree can survive and contaminate later cells without voiding or stopping the session.

- `scripts/bench_otp12pf_mac.sh:685,833-837` — **MEDIUM** — `RUN_COLD` contains a comma on purge failure, but is emitted unescaped into CSV. Every such void row has an extra column and can corrupt downstream parsing.

- `scripts/bench_otp12pf_mac.sh:624-629` — **MEDIUM** — Teardown treats every nonzero `ps` result as `GONE`; a broken `ps` probe is indistinguishable from an absent PID, contradicting the registered fail-closed teardown claim.

`SELFTEST=1` exited 0, but it explicitly does not exercise the timed path, scheduler, CSV handling, or teardown.

**VERDICT: NOT READY**
tokens used
116,034
- `scripts/bench_otp12pf_mac.sh:102,488-510,1088` — **BLOCKER** — `REGISTERED_BUILD` is never enforced. Any `EXPECT_SHA` is accepted; `EXPECT_SHA=f35702a.dirty` can even certify a `.dirty` binary because the dirty probe then searches for `.dirty.dirty`. Contradicts `PREREGISTRATION.md:135-138,466-470`.

- `scripts/bench_otp12pf_mac.sh:459-479` — **BLOCKER** — Five preflight RTT samples’ median is not a bound on later per-run SSH return/dispatch gaps. A delayed timed-run SSH can provide asymmetric free writeback without being measured or voided, preserving the artifact this protection claims to bound.

- `scripts/bench_otp12pf_mac.sh:742,859` — **HIGH** — `run_one_pair` is called as an `if` condition, disabling `errexit` throughout its call tree. A failing `time_argv` that emitted a parseable sentinel is therefore accepted; its exit status is discarded by the following value check.

- `scripts/bench_otp12pf_mac.sh:365-368` — **HIGH** — Spotlight’s raw CPU field is coerced with `$2+0`. A malformed/non-numeric `mds_stores` value becomes zero, is reformatted as numeric `0`, and certifies idle.

- `scripts/bench_otp12pf_mac.sh:682-685` — **HIGH** — `sync; sudo ... purge` reports only `purge`’s status. A failed `sync` followed by successful `purge` marks the run `cold`.

- `scripts/bench_otp12pf_mac.sh:782,788` — **HIGH** — Timed-tree deletion failures are discarded with `|| true`. A large/mixed landed tree can survive and contaminate later cells without voiding or stopping the session.

- `scripts/bench_otp12pf_mac.sh:685,833-837` — **MEDIUM** — `RUN_COLD` contains a comma on purge failure, but is emitted unescaped into CSV. Every such void row has an extra column and can corrupt downstream parsing.

- `scripts/bench_otp12pf_mac.sh:624-629` — **MEDIUM** — Teardown treats every nonzero `ps` result as `GONE`; a broken `ps` probe is indistinguishable from an absent PID, contradicting the registered fail-closed teardown claim.

`SELFTEST=1` exited 0, but it explicitly does not exercise the timed path, scheduler, CSV handling, or teardown.

**VERDICT: NOT READY**
