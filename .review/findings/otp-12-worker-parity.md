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

- Before each payload batch enters the shared elastic send pipeline, drive
  the existing one-stream-per-epoch resize protocol until the currently known
  shape target is settled. Needs and resume hashes continue to be processed
  while acknowledgements are in flight, so the target incorporates all work
  learned during the ramp.
- Stop a refused ramp instead of retrying the same unattainable target under
  fresh epochs forever.
- Centralize receiver stream-ceiling resolution in `dial.rs` and use it on
  both the SOURCE dial and destination-initiator admission path. Wire value
  zero remains unknown/default, never one.
- Strengthen both role-orientation integration pins from merely `> 1` to the
  exact shared target `8`; the destination-initiator case explicitly carries
  the legal unknown-capacity value.

## Files

- `crates/blit-core/src/dial.rs`
- `crates/blit-core/src/transfer_session/mod.rs`
- `crates/blit-core/tests/transfer_session_roles.rs`

## Tests

- Guard proof before the implementation: the strengthened role pins failed
  at 3 streams (SOURCE initiator) and 2 streams (DESTINATION initiator).
- Separate zero-capacity guard proof: after the ramp fix but before the
  shared ceiling fix, the DESTINATION-initiator pin failed at 1 stream.
- After both fixes: the two exact-target pins pass at 8 and the complete
  `transfer_session_roles` integration target passes 39/39.
- Full workspace gate passes: `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` (1488 tests, 2 ignored; no failures).

## Known gaps

- Socket acquisition remains connection-role-specific by design: the network
  initiator dials the responder so a pull caller does not need an inbound
  listener through NAT/firewalls. Byte work is still one SOURCE send pipeline
  and one DESTINATION receive pipeline. This slice removes worker-count drift;
  it does not invert that network topology.
- No hardware benchmark is part of this code slice. The existing otp-12
  acceptance rigs remain the performance proof after review.

## Reviewer comments

(appended after the codex round)
