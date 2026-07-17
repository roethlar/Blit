# ldt-4-r1-f5a — make accepted-resize documentation match accept-or-fault behavior

**Severity**: LOW — a stale API comment described a successful partial
settlement that production deliberately makes impossible.
**Status**: Fixed and manually guard-proved; neutral whole-change re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `f67ef75`

## Evidence

`crates/blit-core/src/dial.rs:709-715` said a post-ACK local dial failure could
settle at the unchanged local count. The more specific active contract and
production disagree: source ACK handling propagates `add_stream`/retirement
failure, requires actual logical membership to equal the requested target, and
only then calls `resize_settled(..., target, true)`. The data-plane comment and
tests likewise require a post-accept failure to fault the session.

## Predicted observable failure

A future analyzer or runtime change following the stale generic comment could
accept or emit `accepted=true` with membership different from the target,
weakening the fail-fast same-build contract and making successful evidence
ambiguous. Fable's proposed analyzer relaxation was one concrete instance of
that misreading.

## What

State the actual production rule at the generic dial settlement API:
`accepted=true` is published only after local membership reaches the proposal
target; post-ACK membership failure faults rather than partially settling.
Refusal, stale-epoch, and cooldown behavior are unchanged.

## Approach

Only the stale Rust documentation comment changes. The strict analyzer,
runtime logic, wire behavior, and tests remain untouched.

## Files changed

- `crates/blit-core/src/dial.rs` — correct `resize_settled` contract comment.

## Guard proof

- `source_resize_ack_effective_count_matches_acceptance` passes and rejects an
  accepted ACK whose effective count differs from its target.
- `accepted_add_and_remove_settle_after_need_complete_in_both_layouts` passes
  exact ADD/REMOVE target settlement under both socket layouts.
- A comment-truth check requires the accept-or-fault sentence and rejects the
  stale `local count if a post-ack dial failed` wording. Temporarily restoring
  the old comment returns 1; exact restoration returns green.

## Coder dispute

Original `ldt-4-r1-f5` is declined: the analyzer is correct to reject partial
accepted settlement. This separate narrowed documentation defect is admitted.

## Known gaps

None. This changes no executable behavior.

## Reviewer comments

Claude Fable 5/max returned the analyzer candidate over exact
`e41b871..0e48721` with `guard_confirmed=true`. Intake proved its predicted
runtime path impossible but found this stale sentence was the misleading
evidence behind it. Final fixed-SHA whole-change re-review is pending.
