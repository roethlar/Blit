# cr-c1-2 contested: verification dispatch invalid — reviewer transport had no execution capability

**Kind**: transport failure, not a disagreement over the fix. The reviewer
returned `invalid` with `guard_confirmed: false`, `capability_ok: false`
(record `.review/results/cr-c1-2.claude.json`, raw envelope
`logs/cr-c1-2-verify-claude-fable-5.json`).

## What happened

The cr-c1-2 verification dispatch (claude / claude-fable-5 / high, over
`824b8db3..22261bf5`, owner-approved 2026-08-04) hit a blanket execution
denial in the reviewer session: direct Bash (`git diff`, `cargo --version`)
returned "requires approval" — beyond the anticipated `rtk` rewrite — and
the prescribed child-subagent fallback also failed to complete the proof.
The reviewer did everything possible read-only (direct `.git` ref reads
confirming head state, `tokensave_changelog` range confirmation, full code
read of the fix and its tests) and reported a favorable *inspection*
assessment, but correctly refused to substitute reading for the guard
proof and declared the review invalid. Per D-2026-07-23-7 the attempt is
not auto-retried.

## Local evidence already in hand (not a reviewer verdict)

The coder's own mutation guard: reverting the `SelectEndpoint` arm to its
pre-fix body turns `select_resets_pane_and_replaces_in_flight_browse` and
`select_registered_daemon_clears_error` red; restored, the crate suite is
16/16 green; full workspace gate green on macOS (fmt, clippy native +
`x86_64-unknown-linux-gnu`, 1837 → 1838 tests). Recorded in
`.review/findings/cr-c1-2.md` (Guard proof).

## What the owner must decide

1. **Re-dispatch the verification** (explicit go required) — same or
   different harness/model/effort, e.g. `codereview claude claude-fable-5 high`
   again or name another reviewer.
2. **Local closure** — accept the local guard proof as the record (REVIEW.md
   legend permits `[x]` local closure when external review is not
   authorized); the finding doc carries the proof and this note.
3. **Leave open** — cr-c1-2 stays `[~]` until a future review pass covers it.

## Ruling (owner, 2026-08-04)

**Option 2 — local closure.** Owner: "no more reviews. those are very
expensive." Recorded as D-2026-08-04-4 (`docs/DECISIONS.md`): external/paid
review dispatches are off unless the owner explicitly orders one; local
guard proofs close findings. cr-c1-2 is Verified by local closure on the
mutation guard recorded in `.review/findings/cr-c1-2.md`.
