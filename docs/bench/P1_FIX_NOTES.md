# P1 resize-dial diagnosis

**Status:** Diagnosis refuted by code inspection; no fix applied.

## Finding

The destination-side socket-acquisition asymmetry is real, but it cannot
stall manifest diffing or need emission in the current session choreography.

- The destination processes manifest entries and emits needs in
  `crates/blit-core/src/transfer_session/mod.rs:2799-2923`. On
  `ManifestComplete`, it diffs the final pending chunk, sends every resulting
  need, and then sends `NeedComplete` before returning to the receive loop.
- The source sends `ManifestComplete` at
  `crates/blit-core/src/transfer_session/mod.rs:1379-1383`. It does not call
  `maybe_propose_resize` until the payload phase, at lines 1397-1419. The only
  `DataPlaneResize` send site is `maybe_propose_resize`, at lines 1794-1812.
- `ManifestComplete` and `DataPlaneResize` use the same ordered frame sender.
  Consequently, the destination cannot enter its `Frame::Resize` arm until
  after it has processed `ManifestComplete`, flushed the final diff chunk,
  and sent `NeedComplete`. There are no later valid `ManifestEntry` frames or
  needs for the resize dial to block.
- The pull destination does await
  `InitiatorReceivePlaneRun::add_dialed_stream` inline at
  `crates/blit-core/src/transfer_session/mod.rs:3124-3126`; that method awaits
  `dial_data_plane` at
  `crates/blit-core/src/transfer_session/data_plane.rs:553-575`. The push
  destination instead calls the non-blocking `ResponderDataPlaneRun::arm` at
  `mod.rs:3121-3123` / `data_plane.rs:330-337`, while the accept loop runs in
  its own task (`data_plane.rs:218-327`). This confirms the local control-loop
  asymmetry, but not the claimed need-flow stall.
- Existing data-plane receive workers are also independent tasks: the push
  accept loop is spawned at `data_plane.rs:218-233`, and pull receive workers
  are spawned by `add_dialed_stream` at `data_plane.rs:568-574`. An inline
  control-lane dial therefore does not pause payload receipt on already-live
  sockets.

The source may enter its payload phase before the destination has consumed
the already-sent `ManifestComplete`, but frame ordering still makes the
destination finish the final diff and send `NeedComplete` before it can see
the following `DataPlaneResize`. The source receive half can consume those
already-sent needs concurrently with the destination's later dial.

## Change decision and invariants

No Rust code, concurrency, trace output, or wire behavior was changed. Moving
the pull dial into a task while retaining the exact ack-after-completed-dial
guarantee would not shorten the resize handshake, and there is no legitimate
manifest/need work for the destination loop to perform while that task runs.
It would add task/error/state coordination without addressing the recorded
P1 mechanism.

Leaving the code unchanged preserves all three load-bearing properties:

- **Dial before ack:** the pull destination still completes the authenticated
  epoch-N dial before sending `DataPlaneResizeAck`
  (`mod.rs:3124-3146`).
- **Fatal dial failure:** `add_dialed_stream` still returns its mapped dial
  error directly to the destination session (`data_plane.rs:560-567`).
- **One resize in flight:** the source driver's `pending_resize` guard remains
  at `mod.rs:1301,1801-1812`, reinforced by `TransferDial::pending_epoch` and
  its compare-exchange at `crates/blit-core/src/dial.rs:349-367`.

The frame bytes and ordering are unchanged.

## Timing and rig confirmation

No new phase timing was added because there is no control-loop-unblocking
change whose recovery it could measure. `--trace-data-plane` continues to
enable the existing `[data-plane-client]` connect-start traces, but those
traces do not report dial duration or a distinct need-flow blockage.

A rig run can test the refutation directly by recording, per resize epoch:

1. the destination's `NeedComplete` send time;
2. the destination's `DataPlaneResize` receive time;
3. the pull dial start/completion time and matching ack send time; and
4. per-stream payload-byte progress during that interval.

The current code requires (1) to precede (2). If already-live streams keep
making payload progress during (3), the claimed starvation mechanism is
absent at runtime as it is in the state machine. If the approximately 300 ms
gap still correlates with resize, the remaining live suspects are the
platform-dependent dial/accept handshake and stream-ramp timing, not stalled
manifest diff or need emission. Instrumenting those phases is a separate
diagnostic change; it should not be presented as validation of this refuted
fix.

## Residual risk

This is a static state-machine refutation, not a rig measurement of the true
P1 cause. The pull path's inline authenticated dial remains an observable
push/pull asymmetry and could still matter through connection establishment
or resize-ramp behavior on a macOS/Windows pair. What is ruled out is the
specific claim that it blocks ongoing manifest diffing, need emission, and
thereby starves the data plane.
