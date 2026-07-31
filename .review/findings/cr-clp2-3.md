# cr-clp2-3: The fixed drain deadline can discard requested verbose lines

**Severity**: LOW — a fast small-file `-v` run can exit successfully with
queued per-file lines silently dropped.
**Status**: In progress — fixed, awaiting per-finding reviewer verdict
**Branch**: — (default-branch mode)
**Commit**: (filled at landing)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `246acb54..9532ee20`
(`.review/results/clp-2-range.codex.json`).

## Evidence
`blit-cli/src/transfers/local.rs:449-453` — `join_drained` aborts the
consumer after the 500 ms grace regardless of whether the lane closed;
`:537-539` applies that deadline on every successful finish while
`:418-424` may still be printing queued completions.

## Predicted observable failure
`blit copy -v -p` over thousands of tiny files: session returns with
>500 ms of queued `-v` output; the consumer is aborted; the command
exits 0 having silently dropped requested output.

## What
The bounded grace exists for FAILED sessions (a blocking enumeration
task can survive holding a sink clone, keeping the lane open); applying
it to successful sessions trades away correctness the success path does
not need — on success every sink clone is dropped, the lane closes
deterministically, and the consumer ends on its own.

## Approach
(planned) `finish()` learns the session outcome: on success, await the
consumer without a deadline (the lane is provably closing); on failure,
keep the bounded grace + abort.

## Files changed
(filled with the fix commit)

## Guard proof
(planned) Success path: a closed lane with many queued `-v` events —
every line is printed before finish returns (red under the
deadline-always revert with an artificially slow output); failure path:
a lane held open still returns within the grace.

## Coder dispute (if any)
None.

## Known gaps
None.

## Reviewer comments
(pending per-finding verification)

As built: `finish(session_succeeded)` routes through `drain_for_outcome` — success awaits the provably-closing lane unbounded, failure keeps the bounded grace + abort. Guard `a_successful_session_drains_every_queued_line_past_the_grace` (SlowRecorder, 100 queued lines past a 50 ms grace) FAILED red under the always-bounded revert, green restored; `finish_gives_up_on_a_lane_that_never_closes` still pins the failure path.
