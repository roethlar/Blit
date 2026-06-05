# Reopened: audit-h3b-pull-data-plane-write-stall

**Reviewer**: gpt (relayed via owner), 2026-06-05.
**Slice logic**: acceptable for the core path (DataPlaneSession::from_stream
wraps TcpStream in StallGuardWriter; targeted blit-core write-side tests + fmt
+ clippy pass clean).
**Blocker**: same repository reproducibility issue that blocked h11.
**Cleanup**: GPT also flagged a non-blocker doc/code mismatch:
`stall_guard.rs:162` reset the deadline on `Ok(0)` despite the docs saying
"any byte count > 0 counts as progress." Fixed bundled with the build-fix.

## Finding

Same build break as the h11 reopen: a clean checkout at round-1 SHA `c14db51`
cannot build because the dual-pane modules referenced by `main.rs` aren't
committed. See `audit-h11-f1-confirm-detail-err.reopened.md` for the full
diagnostic.

Additionally, on the h3b code itself:

> Nonblocking cleanup: `crates/blit-core/src/remote/transfer/stall_guard.rs:162`
> resets the deadline on Ok(0) even though the docs say > 0 bytes count as
> progress. With current write_all/TcpStream use this is not a blocker, but
> it should be tightened.

The doc-comment above the writer says:

> The deadline is re-armed on every successful `poll_write` (any byte count > 0
> counts as progress)

But the implementation reset on any `Ok(_)`, including `Ok(0)`. A peer that
accepts zero bytes per poll would never trip the guard — defeating the
hardening this slice is supposed to provide. Tightening the implementation
to match the doc is the right call even though it isn't reachable through
`write_all` on a TcpStream.

## Resolution

Build-fix commit at master `1b3cb39` bundles both fixes:

1. Commits the two missing dual-pane module files + `screens/mod.rs`
   declaration, unblocking clean-checkout build.
2. Tightens `StallGuardWriter::poll_write`: `Ok(0)` is forwarded without
   resetting the deadline; only `Ok(n)` where n > 0 resets. The doc-comment is
   unchanged because the implementation now matches it.

Clean-checkout validation passes:

- `cargo fmt --all -- --check`: clean
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo test --workspace`: 315 blit-core (including the 3 h3b write-side
  tests) + 646 blit-tui + all other suites green from a wiped target dir.

The existing 3 h3b unit tests continue to pass — they never exercised the
`Ok(0)` path (the duplex-based tests yield real byte counts).

Re-arming the h3b sentinel at master `1b3cb39` — that SHA includes the
original h3b implementation (`c14db51`) plus the build fix plus the
`Ok(0)` tightening.

## Required fixes

None for the original h3b implementation. The two fixes (build-fix + Ok(0)
tightening) are landed at `1b3cb39`. Re-verification at master `1b3cb39`
should pass.

## Known gaps (carried forward from round 1)

- No DataPlaneSession-level integration test directly drives a stall through
  `send_file_*` / `finish` and asserts the TimedOut surfaces. The wiring is
  one-line and the AsyncWrite composition is exercised by the three write-side
  unit tests; a future slice touching the session abstraction may want to
  add a virtual-time DataPlaneSession test.
- audit-h3c (gRPC fallback) still pending — separate mechanism.
