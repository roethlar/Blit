# otp-3 — TransferSession core (role-parameterized, in-process)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-3.
**Status**: implemented — awaiting codex review.
**Contract**: `docs/TRANSFER_SESSION.md` (post-review, `f861579`).

## What

The unified session state machine exists in blit-core and moves real
bytes in-process, with the roles swappable over the same fixtures.
`run_source` / `run_destination` implement the contract's HELLO →
OPEN/ACCEPT → role-lane phases over a `FrameTransport`; the byte
carrier is the in-stream frame grammar (file records + tar-shard
records, strictly serialized, fail-fast). The owner's invariance
property is now in the test suite: every fixture runs under both
initiator layouts and must produce the identical need-list set,
identical summary, and byte-identical destination tree.

## Approach (as implemented)

- `crates/blit-core/src/transfer_session/transport.rs` —
  `FrameTx`/`FrameRx` halves + `FrameTransport` (splittable) +
  `in_process_pair()` on bounded mpsc (64 frames/direction).
- `crates/blit-core/src/transfer_session/mod.rs` —
  - `session_build_id()` = `CARGO_PKG_VERSION+BLIT_GIT_SHA[.dirty]`
    (build.rs emits the sha; rerun-if-changed on `.git/HEAD` +
    `.git/refs`; dirty flag sampled at build-script time, best-effort
    by nature). `CONTRACT_VERSION = 1`. Exact-match hello both ways;
    mismatch → `SessionError{BUILD_MISMATCH}` naming both ids
    (D-2026-07-05-2).
  - `establish()` — ONE hello/open/accept implementation both role
    drivers call. Responder-side capability validation refuses what
    later slices implement (mirror → otp-6, filters → otp-6, resume →
    otp-7) with a `SessionError` instead of accepting — fail-fast, no
    silent ignoring. Receiver capacity travels DEST→SOURCE at setup
    (open when initiator is DEST, accept when responder is), consumed
    from otp-4 when the dial attaches.
  - SOURCE driver: split into a send half and a dedicated receive
    half (deadlock-freedom: the transport is bounded both ways, so a
    single loop that blocks on send while the peer blocks on its own
    send would deadlock — the recv half always drains). Needs are
    validated against the sent-manifest map (unknown / duplicate /
    resume-flagged → PROTOCOL_VIOLATION), which bounds the internal
    event queue by the source's own manifest size. Payloads plan per
    accumulated need batch via `diff_planner::plan_push_payloads`,
    emit as file records (`file_begin` + `file_data`×N, completion at
    exactly `header.size`) and tar records (existing tar builder via
    `prepare_payload`), only after `ManifestComplete` (in-stream
    carrier rule). `SourceDone` only after `NeedComplete` + queue
    drained; then awaits the destination's summary.
  - DESTINATION driver: sequential frame loop (its sends can't
    deadlock because the source's recv half always drains). Manifest
    entries buffer into 128-entry chunks (w4-4 rationale) and
    stat+compare on the blocking pool; need batches stream back
    mid-manifest; `NeedComplete` only after ManifestComplete + all
    entries diffed. File records write through
    `FsTransferSink::write_file_stream` fed by a bounded
    `tokio::io::duplex` pipe (256 KiB); tar records buffer to exactly
    `archive_size` (≤ `MAX_TAR_SHARD_BYTES`, `try_reserve_exact`) and
    unpack through the existing tar-safety path
    (`write_payload(TarShard)`). Grammar violations (payload before
    ManifestComplete, record interleave/overrun/short-complete,
    payload not on the need list, `SourceDone` with outstanding
    needs, resume/resize frames in an otp-3 session) →
    `SessionError{PROTOCOL_VIOLATION}` + abort. Diff stats go through
    the same canonical-containment chokepoint as sink writes
    (R46-F3): an escaping manifest path is a violation, not a stat.
  - Faults are `SessionFault` (wire code + message + both build ids +
    peer_notified), carried in `eyre::Report` — tests downcast and
    assert codes. An end that aborts sends the error frame first
    unless the peer already knows.

### Deviations from the scoped approach (2026-07-05 survey)

1. **Destination diff predicate**: the scoping note named
   `diff_planner::filter_unchanged`, but that predicate stats BOTH
   sides locally — impossible for a wire destination and a role-
   separation leak in-process (otp-4 must be transport substitution,
   not new choreography). The mode-aware header-vs-target owner that
   already exists is `manifest::compare_manifests`; its per-entry
   body is now extracted as `manifest::header_transfer_status`
   (public), `compare_manifests` is refactored onto it, and the
   session destination feeds it from a live stat. Single-owner intent
   preserved; `From<ComparisonMode> for CompareMode` added alongside.
2. **`DestinationOutcome`**: `run_destination` returns
   `{summary, needed_paths}` rather than bare summary — the role
   suite pins need-set equality across role assignments, which the
   scoping called for but the driver didn't expose.
3. `SessionEndpoint::Initiator` boxes its `SessionOpen`
   (clippy large-enum-variant); `SessionEndpoint::initiator()`
   constructor provided.

## Files

- `crates/blit-core/src/transfer_session/{mod.rs,transport.rs}` (new)
- `crates/blit-core/src/lib.rs` (module export)
- `crates/blit-core/build.rs` (BLIT_GIT_SHA emission)
- `crates/blit-core/src/manifest.rs` (`header_transfer_status`
  extraction + `From<ComparisonMode>`; `compare_manifests` behavior
  unchanged)
- `crates/blit-core/Cargo.toml` (filetime added to dev-deps for the
  fixture suite)
- `crates/blit-core/tests/transfer_session_roles.rs` (new, the
  role-parameterized suite)

## Tests

Suite 1484 → 1500 (+16; count never dropped). New:

- `transfer_session_roles.rs` (12): small mixed tree (multi-chunk
  3 MiB file, empty file, spaced/nested names) byte-identical under
  both initiators with identical need sets + summaries; 200-file
  force-tar tree likewise (tar record grammar both layouts);
  incremental pre-seeded destination needs exactly {changed,
  missing}; identical pre-seeded tree yields empty need list and
  0/0 summary; mtime preservation on streamed files; build-id
  mismatch refused both ends under both initiator layouts (message
  names both ids, no bytes move); contract-version mismatch refused;
  mirror-enabled open refused with the otp-6 pointer; scripted-peer
  violations fail fast (payload record before ManifestComplete, need
  for never-manifested path, resume-flagged need, manifest entry
  after ManifestComplete) with the error frame observed on the wire.
- `transport.rs` (2): pair delivery both directions; closed-peer
  semantics.
- `mod.rs` (2): build-id shape; fault wire round-trip (perspective
  swap included).

Gate: `cargo fmt --check` ✓, `clippy --workspace --all-targets
-D warnings` ✓, `cargo test --workspace` 1500/0 ✓.

## Known gaps (carried forward)

- **SizeMtime semantic divergence, decided at otp-4/5 parity**: the
  session inherits `manifest::compare_file`'s Default arm (transfer
  when src NEWER; skip when target same-age-or-newer) — today's
  pull_sync semantic. Today's push daemon uses exact size+mtime
  equality instead, so a destination file with newer mtime but
  different content re-transfers under old push and is skipped by
  the session. The otp-4 A/B parity pins against old push will
  surface this; picking the unified semantic (and whether
  `compare_file`'s Default arm changes) is that slice's recorded
  decision. otp-3 deliberately did not change live pull_sync
  behavior by editing the shared arm.
- Checksum compare mode transfers everything when headers carry no
  checksum (manifest enumeration never populates it today) — the
  conservative arm of `compare_file`, parity with today's push.
  Whether the session grows source-side checksum population is a
  parity-slice call (otp-4/5).
- Strict `SourceDone`: a needed file that vanishes source-side
  mid-transfer faults the session (`INTERNAL` on read failure /
  EOF-short). Old push tolerates and skips (`check_availability`,
  unreadable list). The contract has no "source skipped these"
  notification yet; if parity requires tolerance, that's a contract
  addendum at otp-4/5, not silent skipping.
- Need-batch cadence knobs (the FileListBatcher's 5 ms/64 KiB early
  flush + 25 ms max delay) are not replicated in-process; batches
  flush per 128-entry diff chunk and at ManifestComplete. Cadence
  matters when a real wire + mid-manifest data-plane spin-up exist —
  otp-4.
- Single-file source roots (`relative_path = ""` wire form) are
  untested against the session; parity slices own that edge.
- `require_complete_scan` + `ManifestComplete.scan_complete` travel
  the wire but gate nothing until mirror (otp-6).
- Resize frames on the in-stream carrier are treated as
  PROTOCOL_VIOLATION (no data plane exists to resize in otp-3); the
  frame table marks them any-phase for sessions WITH a plane —
  otp-4 wires the real semantics.
- In-process transport caps frames, not bytes (64 × ≤1 MiB payload
  frames ≈ 64 MiB/direction worst case). Fine for tests and local
  use; the wire carrier has HTTP/2 byte-level flow control.
