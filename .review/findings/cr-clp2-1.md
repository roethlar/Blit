# cr-clp2-1: Redirected log lines bypass control-byte sanitization

**Severity**: MEDIUM — an unreadable unix filename carrying `\n`/ANSI in a
routed warn breaks the single-row display or executes the escape.
**Status**: Verified
**Branch**: — (default-branch mode)
**Commit**: (filled at landing)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `246acb54..9532ee20`
(`.review/results/clp-2-range.codex.json`).

## Evidence
`blit-cli/src/transfers/local.rs:382-383` — `row_line_sink` forwards each
backend line unchanged to `ProgressBar::println`, though
`sanitize_row_text` exists at :369-376;
`blit-core/src/remote/transfer/source.rs:556-557` interpolates a raw
relative filename into exactly such a warning.

## Predicted observable failure
TTY progress run over a tree with a control-byte filename in an
unreadable entry: the routed warn emits extra terminal lines or a live
escape sequence — the same broken-row class clp-2 closed for the row
message and `-v` lines.

## What
The sanitizer covers the row message and `-v` lines but not the third
rendered-text path, the log-redirect sink.

## Approach
Sanitize inside `row_line_sink` before handing the line to
`RowOutput::println` — the one chokepoint every routed backend line
passes through.

## Files changed
- `crates/blit-cli/src/transfers/local.rs` — sanitize inside `row_line_sink`; new guard test `a_backend_line_with_control_bytes_is_sanitized`.

## Guard proof
As built: `a_backend_line_with_control_bytes_is_sanitized` — FAILED red with the sanitize call reverted to pass-through (verified this session), green restored; the clean-line pass-through test still pins non-mangling.

## Coder dispute (if any)
None.

## Known gaps
None.

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / xhigh / standard; codex-cli 0.146.0 (detached, disposable worktree). Reviewed `a6cf7a8c5d8457ea91c31ce953f1744371200636` base `4179a144135531dcd71627878a39a663e0d0a705`. guard_confirmed: **true** (reviewer ran revert→red→restore→green itself). Verdict: **accepted**, no comments. 2026-07-31T04:59Z. Record: `.review/results/cr-clp2-1.codex.json`.
