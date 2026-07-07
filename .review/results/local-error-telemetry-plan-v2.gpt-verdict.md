# LOCAL_ERROR_TELEMETRY.md v2 (plan draft) — GPT review adjudication

**Change**: `docs/plan/LOCAL_ERROR_TELEMETRY.md` Q1-Q4 update, commit `906524e`.
**Reviewer**: gpt-5.5 (codex-cli 0.142.5, read-only sandbox).
**Raw review**: `.review/results/local-error-telemetry-plan-v2.codex.md`.
**Codex verdict**: CHANGES REQUESTED — 2 Medium, 1 Low.

## F1 (Medium) — "no fire-and-forget/detached path" claim is false — ACCEPTED

**Codex**: `run_transfer` does await each `TransferRoute` arm inline, but the
delegated route can take `--detach`: `run_remote_to_remote_direct` returns after
`Started` and the daemon continues independently. Post-Started failures will not
reach the `main.rs` wrapper.

**Adjudication: ACCEPTED (real).** Verified by reading
`transfers/remote_remote_direct.rs:130-152`: `DelegatedPullExecution.detach:
args.detach`, and the `if args.detach { ... }` branch returns a synthesized
zero-summary `Ok` as soon as the daemon confirms the `Started` event, dropping the
receiver — the transfer continues on the daemon with no CLI process attached. A
failure after that point never produces an `Err` for `run_transfer` to return.

**Fix**: scoped the doc precisely — covers every route's failure up to the point
the CLI stops observing it, which is the whole transfer for four of the five
routes and "up to a successful `Started`" for a `--detach`ed
`RemoteToRemoteDelegated` transfer. Added an explicit non-goal/acceptance-criteria
exception and a slice-2 test asserting no record lands for a successfully detached
transfer.

## F2 (Medium) — `route` field derivation hand-waved + crate-layering bug — ACCEPTED

**Codex**: a `blit-core::error_history` recorder cannot rederive route via
`select_transfer_route` because that lives in `blit-app`; also `run_transfer` can
fail before route selection/parsing succeeds. "Return the route alongside its
Result" needs the API explicitly shaped to preserve `Option<Route>` through
`run_with_retries`; as written this was hand-waved.

**Adjudication: ACCEPTED (real).** Verified: `TransferRoute`/`select_transfer_route`
are defined in `blit-app/src/transfers/dispatch.rs:69,112`; `blit-core` cannot
depend on `blit-app` (wrong direction in this workspace's layering — `blit-core` is
the foundational library every other crate depends on, never the reverse). Also
confirmed `run_transfer` has pre-route-selection fallible calls (e.g. endpoint
parsing) before its `match select_transfer_route(...)`.

**Fix**: `route` is now `Option<String>` — a plain string label, `None` when the
failure predates route selection. `error_history` (in `blit-core`) never imports
or knows about `TransferRoute`; the stringification happens in `blit-cli`/
`blit-app`, which already see the enum. Slice 2 now explicitly says
`run_transfer` needs to thread `Option<TransferRoute>` out alongside its `Result`
(exact shape left to the slice, but the crate-boundary constraint is now stated,
not hand-waved), with tests for both the `None` case and the `--detach` exclusion.

## F3 (Low) — Q5 option (b) asserted a compatibility with D-2026-07-05-4 that doesn't hold — ACCEPTED

**Codex**: Q5 option (b) says to start before ONE_TRANSFER_PATH fully ships
"without formally reopening" D-2026-07-05-4, which conflicts with that decision's
hard "only work item until it ships" wording. Option (a) is coherent; option (b)
needs to be removed or reframed as an explicit recorded exception/supersession.

**Adjudication: ACCEPTED (real, on the doc-honesty point) — owner's choice stands.**
Codex is right that "without formally reopening" doesn't dissolve the substantive
conflict — starting slice 1 before ONE_TRANSFER_PATH ships is an exception to
D-2026-07-05-4's absolute wording whether or not a decision doc records it. This is
not something for the reviewing agent to overrule: the owner was presented this
exact tension via `AskUserQuestion` (both options stated, including the
"no recorded exception" framing) and explicitly chose option (b) with that tradeoff
in front of them. The fix is to the doc's honesty, not the decision: rewrote Q5 to
state the resolution plainly, acknowledge the tension instead of asserting it away,
and note that a lightweight `D-2026-07-06-n` recording the informal exception would
be the clean fix if/when this plan flips Active — without recording one now, since
the owner chose the informal route.

## Summary

All 3 findings accepted and fixed in `8ea2334`. No findings rejected or deferred.
F1/F2 are real technical corrections to the design (detach boundary, crate-layering
fix for `route`). F3 is a documentation-honesty fix; the underlying owner decision
(Q5 option b) is unchanged — codex's point was about how the doc *described* that
decision, not that the decision itself needed to change, and that isn't this
review loop's call to make.
