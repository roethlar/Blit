# otp-4b-1 — codex adjudication

reviewer: gpt-5.5 (codex v0.142.5, xhigh, read-only)
commit reviewed: `881d412`
raw review: `.review/results/otp-4b1-data-plane.codex.md`

Codex VERDICT: **FAIL** — 2 High findings. Both adjudicated **Accepted**.

## F1 — data-plane completion is a weak count proxy (mod.rs:1267) — ACCEPTED (real)

Codex: the data-plane completion check `files_written == needed_paths.len()`
is count-only. Because `execute_receive_pipeline` writes socket-provided
paths directly (`pipeline.rs:445/465/498`), a peer can (a) send a path not
on the need list, (b) duplicate one needed path while omitting another, or
(c) send non-resume BLOCK records, and still pass if the count matches.
Sink containment stops root escape, but the need-list/mode contract is not
enforced.

Verified against source: correct. The session's **in-stream** carrier
DOES enforce membership — `destination_session`'s FileBegin/TarShardHeader
arms do `outstanding.remove(path)` (violation if absent) and the payload
grammar rejects block/resume frames in a non-resume session. So the
data-plane carrier was looser than the session's own other carrier — an
internal inconsistency, not merely a parity gap with old push. (Old push
also trusts the authenticated peer's paths, but the unified session's
fail-fast contract and its own in-stream strictness set the bar here.)

Fix: unify both carriers on ONE shared `outstanding` set. The control
loop inserts each granted path BEFORE sending its `NeedBatch` (insert
happens-before the source can send that payload, so no race), the
in-stream arms claim from it as today, and the data-plane receive claims
from it via a `NeedListSink` decorator that (i) requires each written path
to be present-and-unclaimed, (ii) rejects `FileBlock`/`FileBlockComplete`
in a non-resume session. Completion in BOTH carriers becomes
`outstanding.is_empty()`, replacing the count proxy.

## F2 — no read-side StallGuard on the data-plane receive (data_plane.rs:153) — ACCEPTED (real)

Codex: accepted sockets go raw into `execute_receive_pipeline`, without
the read-side `StallGuard` the existing push receive uses
(`blit-daemon .../push/data_plane.rs` `receive_push_data_plane` →
`StallGuard::new(socket, TRANSFER_STALL_TIMEOUT)`). A peer that auths then
stalls pins the DEST at `recv.join()` (SourceDone) instead of faulting
after the REV4 stall timeout. This is a carried REV4 RELIABLE invariant.

Verified: correct — independently spotted before the review landed. Fix:
wrap each accepted socket in `StallGuard::new(socket, TRANSFER_STALL_TIMEOUT)`
before `execute_receive_pipeline`, matching old push.

## Non-findings codex confirmed
- Token order/size (session_token ‖ epoch0_sub_token, 16+16) correct.
- No dependency on `remote::push` or the daemon push service (the
  otp-10-deleted drivers) — boundary clean.

## Fix commit
`e1aafcc` — otp-4b-1: address review (2 findings). Both F1 + F2 fixed;
gate green (fmt/clippy/test **1512/0**); guard proof on the F1 test
(`need_list_sink_enforces_membership_and_rejects_blocks` fails with
`claim()` neutered). Re-review of `e1aafcc` requested (the fix added
shared-set concurrency + a sink decorator — non-trivial).

## Re-review of `e1aafcc` — 1 High — ACCEPTED (real)

raw: `.review/results/otp-4b1-data-plane.fix-review.codex.md`.

Codex: `outstanding` now serves double duty — ever-granted DEDUP (the
`insert` filter in `diff_chunk_and_send_needs`) AND not-yet-delivered
COMPLETION (claimed by `NeedListSink`). On the data plane the source
sends payloads for earlier NeedBatches while the destination is still
diffing later manifest chunks, so a `claim` (remove) races an `insert`
(grant): for a DUPLICATED manifest path, the claim can remove the first
grant before the second chunk's `insert` runs, letting it re-grant the
same path — breaking "needed at most once" (duplicate delivery / false
unfulfilled need, timing-dependent).

Verified: real. The in-stream carrier is safe only because its phase
ordering sends every need before any payload (grant and claim never
overlap); the data plane's immediate-start payloads break that, which my
shared-set fix did not account for.

Fix (codex option a): split the concerns. A monotonic, control-loop-LOCAL
`granted` set does dedup (insert-only, never removed → a concurrent claim
cannot re-open a grant); the shared `outstanding` set is purely
completion (inserted for freshly-granted paths before the NeedBatch,
claimed by both carriers, `is_empty()` at SourceDone). `granted` is
touched only by the single control-loop task, so it needs no lock.

## Fix-of-fix commit
`777dfc5` — otp-4b-1: fix the dedup/claim race. Two-set split (local
monotonic `granted` for dedup + shared `outstanding` for completion).
Gate green, suite 1512/0, no regression.

## Confirming re-review of `777dfc5` — PASS (no findings)
raw: `.review/results/otp-4b1-data-plane.race-fix-review.codex.md`.
Codex confirmed the split is correct and complete: `granted` is
control-loop-local, insert-only, touched only via
`diff_chunk_and_send_needs`; `outstanding` is populated only from freshly
deduped grants before the NeedBatch and claimed by both carriers; no
lock-across-await, deadlock, poisoning, or in-stream regression.

## otp-4b-1 CLOSED
3 review passes: `881d412` (2 High → fixed `e1aafcc`), fix-review
(1 High race → fixed `777dfc5`), confirming re-review PASS. Suite
1509 → **1512/0**. otp-4b-2 (resize + sf-2) and otp-4b-3 (cancel e2e)
remain.
