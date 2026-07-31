# cr-ls1-3 — The phase artifact bypasses the progress row's sole-writer contract

**Severity**: MEDIUM
**Status**: FIXED — awaiting reviewer verification
**Source**: `ls-1-range` round-2 codex dispatch over `a0b5d83d..35948e70`
Reviewer: codex / gpt-5.6-sol / xhigh / workspace-write (detached, disposable
worktree); codex-cli 0.146.0. Record:
`.review/results/ls-1-range.codex.r2.json`.

## The finding, as returned

> `phase_probe.rs:243` writes and flushes raw stderr synchronously from
> `run_local_session`, while `blit-cli/src/transfers/local.rs:571` declares
> the live progress writer the sole owner of transfer-time stderr.
> Interactive CLI runs enable that row automatically, and the phase report is
> emitted before the caller clears it, so progress control writes can
> interleave with or scroll away the sole JSON artifact; the blocking write
> also runs on the async worker. Route the report through the row-aware
> output path or defer it until the row is closed.

## Why it is admitted

Correct, and it defeats the probe's only purpose in the exact configuration
an operator would use. clp-2 made the row the sole stderr writer precisely
because unrouted writes get destroyed by the row; this probe then wrote
unrouted. Worse, `effective_progress()` engages the row on any interactive
TTY without `-p`, so the default interactive run is the broken one.

## Fix

The report goes through `blit_core::stderr_log::route_line` — the shared
line sink the row installs (`redirect_lines`), falling back to plain stderr
when no row exists, so non-interactive and piped runs are byte-identical.
`route_line` is promoted to `pub` with a doc comment recording why.

Deliberately NOT `log::info!`, which would also be row-aware: that path
prefixes `binary: LEVEL:` and is gated by `BLIT_LOG`'s max level, so an
operator running with default logging would receive no artifact at all — a
silent diagnostic is worse than a mangled one.

**Guard**: `the_report_serializes_to_one_parseable_line` pins the artifact
as a single line of valid JSON with stable `SCREAMING_SNAKE_CASE` phase
names — the properties that make routing it worthwhile rather than dumping
it. Not a guard on the routing itself; see Known gap.

## Known gap

No test proves the report reaches the row's sink rather than raw stderr,
because the row lives in `blit-cli` and the probe in `blit-core`, and
neither crate can observe the other's sink in a unit test. The routing is
one call to a function whose fallback behaviour IS tested by the existing
`stderr_log` tests. Recorded rather than papered over.
