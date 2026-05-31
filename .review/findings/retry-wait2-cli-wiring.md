# retry-wait2-cli-wiring: --retry/--wait flags + transfer-dispatch wiring

**Severity**: Feature (owner-approved follow-up)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `68b34ac`
**Parent**: owner-approved robocopy-style `--retry`/`--wait` (memory
`audit-owner-decisions`). Part 2 of 2 — completes the feature; builds on
`retry-wait1-classifier-loop` (`is_retryable` + `run_with_retries`).

## What

Surfaces the retry loop on the CLI and wires it into the transfer path.

- **`cli.rs`** — `--retry <N>` (u32, default 0 = no retries) and
  `--wait <SECS>` (u64, default 5) on `TransferArgs`, under the existing
  `Reliability` help heading. This re-fills the slot R58-F8 reserved when
  it removed the old dead `--retries` knob — now actually wired, so it's
  no longer a "lying knob."
- **`main.rs`** — wrap the `Copy`/`Mirror` `run_transfer` and `Move`
  `run_move` calls in `run_with_retries(args.retry,
  Duration::from_secs(args.wait), |_n| <transfer>)`. A retry re-runs route
  selection + reconnect against the *resumable* transfer (continues, not
  restarts); only `is_retryable` transport failures (incl. the audit-1c
  `StallGuard` `TimedOut`) are retried — fatal errors fail immediately.
- **`transfers/mod.rs`** — the three test `TransferArgs` literals get the
  two new fields.

## Tests

`cli::tests::retry_wait_flags_parse_and_default` — the flags parse,
default to `retry=0`/`wait=5`, and accept explicit values.

**Verification gap (flagged):** the end-to-end retried transfer is
integration-shaped (it needs a daemon that drops/stalls mid-transfer to
trigger a real retryable failure), so it isn't unit-tested here; the loop
logic (retry/skip/exhaust classification) is covered by retry-wait1's 6
unit tests, and this slice verifies the flag plumbing.

## Files changed

- `crates/blit-cli/src/cli.rs`, `main.rs`, `transfers/mod.rs`.

## Scope

Completes the owner-approved retry-wait feature. The only remaining
backlog item is audit-7d (the main.rs refactor).

## Reviewer comments

(empty — pending review)
