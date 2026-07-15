# otp-12-worker-parity — initiator-independent stream target

**Slice**: ONE_TRANSFER_PATH otp-12 acceptance repair. The active plan
requires one sender-owned, receiver-bounded stream policy for both role
assignments; initiator/verb may not change the realized worker count.

## What

The unified session computed the same shape target in both orientations but
did not guarantee reaching it. Resize advances one stream per epoch. Once
`NeedComplete` arrived, the SOURCE resolved only the one epoch already in
flight and stopped proposing. On the same 10,000-file fixture (shape target
8), the source-initiator test settled at 3 streams and the
destination-initiator test at 2.

The destination-initiator admission side also interpreted an advertised
`max_streams = 0` as a one-stream ceiling, while the SOURCE dial correctly
interpreted the wire value as unknown/default. That was a role-specific cap.

## Approach

- Queue each planned payload onto the existing bounded elastic pipeline while
  selecting that send against SOURCE control events. Queue readiness is biased
  first, so the floor worker starts useful work immediately; resize ACKs then
  add workers to that same work-stealing queue under backpressure.
- After `NeedComplete` and the final planned payload, close the payload input
  before settling the residual one-stream-per-epoch ramp. Existing workers
  drain and emit END; late workers either join remaining work or see the closed
  queue and emit END immediately. Final worker count reaches the full stable
  shape target without putting serialized resize RTTs before first byte or
  holding idle receive workers under StallGuard.
- Make a refused resize terminal for that transfer: it consumes the wire epoch,
  preserves the live worker count, and blocks both shape and tuner proposals.
  A later plain or resume batch therefore cannot retry it with another token.
- Centralize receiver stream-ceiling resolution in `dial.rs` and use it on
  both the SOURCE dial and destination-initiator admission path. Wire value
  zero remains unknown/default, never one.
- Strengthen both role-orientation integration pins from merely `> 1` to the
  exact shared target `8`; the destination-initiator case explicitly carries
  the legal unknown-capacity value.

## Files

- `crates/blit-core/src/dial.rs`
- `crates/blit-core/src/transfer_session/data_plane.rs`
- `crates/blit-core/src/transfer_session/mod.rs`
- `crates/blit-core/tests/transfer_session_roles.rs`
- `docs/TRANSFER_SESSION.md`

## Tests

- Guard proof before the implementation: the strengthened role pins failed
  at 3 streams (SOURCE initiator) and 2 streams (DESTINATION initiator).
- Separate zero-capacity guard proof: after the ramp fix but before the
  shared ceiling fix, the DESTINATION-initiator pin failed at 1 stream.
- After both fixes: the two exact-target pins pass at 8 and the complete
  `transfer_session_roles` integration target passed 39/39 before review.
- Review guard for nonblocking convergence: with resize ACK #2 held, both
  initiator layouts send all 2,000 one-byte files before the ACK reaches the
  SOURCE, then settle at the same exact target of 4 with identical trees.
  Mutation proof: restoring the pre-dispatch settle makes the test fail after
  10 seconds with payload stalled behind ACK #2; restoring the fix passes in
  about 1.5 seconds.
- Review guard for terminal refusal: the dial consumes the refused epoch,
  leaves live workers unchanged, and every later shape/tuner proposal is
  `None`. Mutation proof: omitting the terminal record immediately returns a
  new shape proposal and fails the pin.
- Re-review guard for resize arbitration: a deterministic hook pauses the
  tuner while it owns the epoch lock. The accepted-settlement hook reports only
  after a real `try_lock` observes contention, and a guard-owned acquisition
  token must be identical at tuner entry and epoch claim. Dropping/reacquiring
  arbitration after the hook therefore fails deterministically on token 11 vs
  12, independent of which waiter the scheduler runs first. The same test
  crosses refused settlement; omitting terminal refusal lets epoch 2 escape.
- Release guard: the same arbitration test passes under `cargo test --release`.
  Mutation proof: moving the state mutation inside `debug_assert!` makes the
  optimized test fail because release compilation removes the assertion and
  its side effect; evaluating the mutation unconditionally restores green.
- Full workspace gate passes: `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` (1,490 passed, 2 ignored; 1,492 test functions,
  no failures).

## Known gaps

- Socket acquisition remains connection-role-specific by design: the network
  initiator dials the responder so a pull caller does not need an inbound
  listener through NAT/firewalls. Byte work is still one SOURCE send pipeline
  and one DESTINATION receive pipeline. This slice removes worker-count drift;
  it does not invert that network topology.
- No hardware benchmark is part of this code slice. The existing otp-12
  acceptance rigs remain the performance proof after review.
- The singular-token wire contract still grows one socket per epoch. On a very
  short/high-latency transfer, tail epochs may open workers only to close them;
  a bulk jump would require a separately reviewed multi-token wire change.

## Reviewer comments

Codex (`gpt-5.6-sol`, xhigh) returned **FAIL** with two findings, both accepted:

- **HIGH**: settling every resize epoch before payload dispatch serialized up
  to 31 control RTTs ahead of first byte while receive StallGuards were already
  active. Fixed by bounded queue/control concurrency plus close-before-tail
  convergence; deterministic gated-ACK guard added.
- **MEDIUM**: refusal was only locally terminal; a later batch could repropose
  the same target/epoch with a fresh token. Fixed by consuming the refused epoch
  and recording terminal refusal in the shared `TransferDial`; mutation-guarded.

The independent fix review returned **FAIL** with one additional MEDIUM,
accepted: the split `resize_refused` check and `pending_epoch` CAS left a
multi-producer race in which a waiter could claim the slot after refusal.
Epoch, pending, and refusal arbitration now share one short mutex-protected
state, which also prevents epoch ABA across an intervening accepted settle.

The independent review of that repair returned **FAIL** with one MEDIUM and
one LOW, both accepted. Tuner cooldown, sustain, and direction were still
computed outside the epoch lock, so a decision could survive an accepted
shape settlement and immediately claim the next epoch. The stress guard also
depended on scheduler luck and did not cross accepted settlement. The tuner
decision and claim now share the settlement lock, and the replacement guard
forces both accepted and refused interleavings deterministically.

The review of the arbitration repair returned **FAIL** with one LOW, accepted:
the first replacement guard observed the lock at the hook but joined the tuner
before accepted settlement, so it inferred rather than forced the exact race.
The accepted settler was moved before tuner release.

The review of that test repair returned **FAIL** with one LOW, accepted: its
channel signal still fired immediately before `resize_settled`, so scheduler
order could let a guard-drop mutation pass. The signal now comes from inside
`resize_settled` after its real `try_lock` result, and the tuner guard carries a
test-only acquisition token recorded at entry and claim. Dropping/reacquiring
the guard changes that token and fails deterministically; restoring one
continuous acquisition passes in debug and release.
