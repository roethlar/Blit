# design-4-fallback-midmanifest-negotiation — forced-gRPC pushes died at the need-list flush boundary

**Branch**: `master` (owner ratified the fix 2026-06-12: "yes, fix that")
**Commit**: `ddfeb58`
**Filed**: during w4-2 (its regression net exposed this); REVIEW.md row added 2026-06-12

## What

Any forced-gRPC push big enough to trigger a mid-manifest need-list flush
failed: ≥128 files (FILE_LIST_EARLY_FLUSH_ENTRIES) deterministically,
~100 timing-flaky (the 5 ms delay flush), ≤80 reliable. Pre-existing
(stash-bisect verified before w4-2 landed); it made w4-2's 262k wedge
unreachable in practice.

## Mechanism (verified, upgraded from the filed hypothesis)

The daemon's manifest loop hard-rejects `FileData` ("data payload received
before negotiation"). Two paths sent FileData into that loop:

1. **Daemon-initiated**: `handle_push_stream`'s early-flush branch sent
   `Negotiation(tcp_fallback)` mid-manifest (and re-sent on every flush —
   it never set `data_plane_handle`). The client streams FileData
   immediately on that announcement. The bind-failure fallback path had
   the same mid-manifest announcement.
2. **Client-initiated**: with `--force-grpc`, `transfer_mode` initialized
   to `Fallback`, so the first need-list batch — and the client's own
   manifest-send loop — called `stream_fallback_from_queue` with **no
   negotiation at all**.

Verified empirically: daemon `--metrics` shows `push err` at ~13 ms on a
128-file repro (manifest phase, not data phase); fixing only the daemon
side left the client-side race (intermittent pass/fail at 128), which is
how the second path was found.

## Fix

Both sides converge on the sequence every working small push already used
(manifest → ManifestComplete → one fallback negotiation from
`execute_grpc_fallback` → data):

- `control.rs`: early-flush branch is now TCP-only (`!force_grpc_effective`);
  forced-gRPC stays silent mid-manifest; bind-failure flips the mode flag
  without announcing.
- push client: new `fallback_negotiated` flag set only by the
  `Negotiation(tcp_fallback)` arm; the need-list arm and manifest-send
  loop hold payloads in `pending_queue` until it's set (the negotiation
  arm already drains the queue, so nothing is lost).

## Files changed

- `crates/blit-daemon/src/service/push/control.rs`
- `crates/blit-core/src/remote/push/client/mod.rs`
- `crates/blit-cli/tests/remote_tcp_fallback.rs` (regression promoted)
- `docs/STATE.md` (ratification note)

## Tests added

The CI regression `forced_grpc_push_many_files_completes` promoted from
50 → 2000 files (was capped below the bug's cliff; now proves the fix).
Manual verification: 128/200/1000/2000 sweeps + 3× repeat at the old 128
cliff, all complete with every file landing. Suite count flat at 1369
(test upgraded in place).

## Known gaps

- The 270k joint wedge repro stays `#[ignore]` for runtime only — expected
  to pass now but not yet executed end-to-end (multi-minute). Worth one
  manual run before 0.1.0 (or during the 10 GbE session).
- The zero-files-need-upload forced-gRPC path sends no negotiation at all;
  the client's early-finish handles it (unchanged behavior, covered by
  existing tests).
- The wire contract ("no FileData before negotiation") is still implicit
  in two code bases; w6-1/W1-class work could make it a typed state.
