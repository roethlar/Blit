# cr-pfc4-2: The count cap does not bound the encoded summary frame

**Severity**: MEDIUM — 64 near-maximum path+reason strings can push the
closing summary frame past the 4 MiB decode limit; the frame is
rejected and the contained transfer ends as a session fault WITHOUT its
failure report.
**Status**: Verified
**Branch**: — (default-branch mode)
**Commit**: `71e87e1b`

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `72079331..bdc3e4a4`
(`.review/results/pfc-4-range.codex.json`).

## Evidence
`sink.rs:193-198` (`wire_failures`) caps entry COUNT at 64 but bounds
neither per-string nor aggregate encoded bytes;
`transfer_session/mod.rs:4830-4832` sends the whole report in one frame
under the documented 4 MiB decode limit (`mod.rs:99-100`).

## Predicted observable failure
Deep paths + long error chains × 64 entries > 4 MiB: the SOURCE rejects
the closing Summary frame; the session that successfully contained its
failures faults at the very end and delivers no report.

## What
The bounded-report constraint was implemented as an entry-count bound;
the repo's frame discipline (D-2026-07-10-1 precedent) requires an
encoded-byte bound.

## Approach
(planned) `wire_failures` applies, alongside the 64-entry cap: per-entry
truncation (reason capped at ~1 KiB, path carried whole up to a sane
bound with tail-preserving truncation past it) and a conservative
aggregate encoded-byte budget (~256 KiB) that drops trailing entries
once exceeded. `files_failed` stays the exact total. Same budget for
the delegated copy (cr-pfc4-1).

## Files changed
(filled with the fix commit)

## Guard proof
(planned) A synthesized report with 64 huge entries encodes under the
budget (assert on encoded_len) and round-trips; red when the byte
budget is removed.

## Coder dispute (if any)
None.

## Known gaps
None.

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / xhigh; codex-cli 0.146.0 (detached, disposable worktree). Reviewed `71e87e1bc7c3a98c907a4b1fe8c08936524bfa6b` base `15c74f1624b2fdb4e0c510aaafb6570983413119`. guard_confirmed **true** — reviewer removed both bounds itself: frame-limit/UTF-8/delegated tests red at 6,293,804 encoded bytes (>4 MiB) while ordinary pass-through stayed green; restored blob matches HEAD. Verdict: **accepted**. 2026-07-31T08:27Z. Record: `.review/results/cr-pfc4-2.codex.json`.

As built: per-entry bounds (reason 1 KiB head-truncated, path 4 KiB TAIL-preserving, char-boundary-safe, marker '[truncated]') + 256 KiB aggregate encoded budget in `wire_failures` (the one producer; the delegated re-encode inherits by copying) + a compile-time const assert that one bounded entry fits the budget. files_failed stays exact; ordinary reports pass byte-identical; record_failure's warn still carries full strings. Guard proven in TWO red variants — full revert: 6,293,804 B > the 4 MiB decode limit (the finding's predicted failure verbatim); aggregate-only revert: 328,576 B > the 256 KiB budget — each half independently load-bearing. SHA-verified restore; blit-core lib 471/0; workspace 1668/0.
