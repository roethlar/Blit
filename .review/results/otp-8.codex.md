# codex review — otp-8 (commit 5ffc9be)

reviewer: gpt-5.6-sol via `codex exec -s read-only` (OpenAI Codex
v0.144.1, superpowers plugin disabled, reasoning effort ultra),
2026-07-10, session id 019f4c42-8dfa-7b02-9ffb-3e8359401e5a.

The raw capture was the full 1.3 MB session transcript (every file the
reviewer paged through); trimmed here to the reviewer's own output.
The adjudication in `otp-8.gpt-verdict.md` records what was verified.

Reviewer's interim note before the verdict: "The two added tests
themselves check out: both compile into the daemon harness, their
block math is unique to the asserted effective size, and the workspace
lister reports 1,552 runnable tests plus the pre-existing ignored
test. The remaining review question is scope closure …"

## Findings

- `crates/blit-core/src/transfer_session/mod.rs:1183` — **High** —
  In-stream file/tar sends—and resume at line 1224—do not poll queued
  peer faults. Cancellation can hang behind a stalled read or surface
  as `INTERNAL` instead of `CANCELLED`; therefore the finding doc's
  cancellation deferral is unsound.

- `crates/blit-core/src/transfer_session/mod.rs:1716` — **Medium** —
  `TarShardHeader` sends all member headers in one protobuf frame.
  Legal 2,048/4,096-entry shards with long paths can exceed tonic's
  4 MiB limit; only archive chunks are bounded. Thus resume was not
  the only real-wire residue.

VERDICT: FAIL — the new tests are otherwise sound; runnable test count
is confirmed at 1550 → 1552.

(tokens used: 280,082)
