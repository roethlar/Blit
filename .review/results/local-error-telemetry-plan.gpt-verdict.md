# LOCAL_ERROR_TELEMETRY.md (plan draft) — GPT review adjudication

**Change**: `docs/plan/LOCAL_ERROR_TELEMETRY.md` (new Draft plan doc), commit `284f7f9`.
**Reviewer**: gpt-5.5 (codex-cli 0.142.5, read-only sandbox).
**Raw review**: `.review/results/local-error-telemetry-plan.codex.md`.
**Codex verdict**: CHANGES REQUESTED — 1 High, 1 Medium, 1 Low.

## F1 (High) — proposed chokepoint records every route, not just local — ACCEPTED

**Codex**: the doc's first draft wired the failure recorder at
`crates/blit-cli/src/main.rs`'s `Commands::Copy`/`Commands::Mirror` arms, but
`run_transfer` (called from both) dispatches `LocalToLocal`/`LocalToRemote`/
`RemoteToLocal`/`RemoteToRemoteRelay`/`RemoteToRemoteDelegated` from one function
via `select_transfer_route` (`transfers/mod.rs:101-287`). Wiring at `main.rs` would
record every route's failures, including remote ones and pre-dispatch argument
bails, contradicting the doc's stated local-only scope.

**Adjudication: ACCEPTED (real).** Verified by reading `transfers/mod.rs:101-287`:
`run_transfer` is a single function whose `match select_transfer_route(...)` covers
five route variants, each with its own `!src.exists()` bail before calling its
route-specific runner. `main.rs`'s `Commands::Copy`/`Mirror` arms call
`run_with_retries(..., || run_transfer(...)).await?` — there is no route
information left by the time an `Err` reaches that point. The proposed chokepoint
was too high in the call stack for a local-only feature.

**Fix**: relocated the chokepoint into `run_transfer`'s `TransferRoute::LocalToLocal`
arm specifically (`transfers/mod.rs:235-241`) — wraps that arm's
`run_local_transfer(...)` call (and its local `!src.exists()` bail, in-scope since
it's still a local-route failure), leaving every other route arm untouched. Added a
slice-2 test requirement pinning that a non-`LocalToLocal` route failure does
**not** produce a record.

## F2 (Medium) — recorder-failure handling contradicts byte-identical stderr — ACCEPTED

**Codex**: the doc said a recorder failure logs via `log::warn!`, but `blit`
installs a real stderr backend for the `log` facade (`stderr_log.rs`, wired
`main.rs:35`) that prints `blit: warn: <msg>` to stderr on every `log::warn!`. That
would itself change stderr output when recording fails, contradicting the doc's own
"process exit code and stderr output... byte-identical to today's" acceptance
criterion.

**Adjudication: ACCEPTED (real).** Verified `stderr_log.rs:24-32`: the installed
`Log` impl unconditionally `eprintln!`s any enabled record. Found the existing
precedent for exactly this problem: `engine/history.rs::record_performance_history`
(lines 36-40) already solves it for `perf_local.jsonl` — a failed history write is
silently dropped unless `options.verbose`, via a direct `eprintln!` gated on the
verbose flag, not the `log` facade.

**Fix**: switched the design to match that existing convention — silent by default,
`--verbose`-gated `eprintln!`, so default-mode stderr is unaffected by a recorder
failure either way.

## F3 (Low) — dangling "(see D1)" references — ACCEPTED

**Codex**: lines 97 and 112 referenced "(see D1)" but the doc defines no `D1`
section (unlike `OTP7_RESUME.md`'s D1-D6 convention it was modeled after).

**Adjudication: ACCEPTED (real).** Verified: the doc has an "Open questions" section
(Q1-Q5) but no separate "Design decisions" section with D-numbered entries.

**Fix**: retargeted both references to "(see Q1 below)", the actual open question
covering that fork.

## Summary

All 3 findings accepted and fixed in `ebb668f`. No findings rejected or deferred.
This is a Draft plan doc, held out of `docs/STATE.md`'s Queue per the owner's
explicit choice this session (D-2026-07-05-4 pins the Queue to ONE_TRANSFER_PATH
exclusively) — no code lands from this doc until the owner lifts that gate and
flips `**Status**: Draft` → `Active`.
