# audit-h1 — `mirror --relay-via-cli` bypasses `require_complete_scan` data-loss guard

**Source**: 2026-06-04 audit chain, R2/R3 finding **H1** (top-priority data-loss-class).
GPT R1-review caught the gap; my workflow audit missed it.
**Parent**: cross-references the same `require_complete_scan` regime closed for `move` in
R50-F1 / R51-F2 (see `crates/blit-cli/src/transfers/mod.rs:568-590`,
`crates/blit-cli/tests/local_move_semantics.rs:370-402`).

## What

`blit mirror --relay-via-cli REMOTE_A REMOTE_B` could silently destroy data on the
destination when the source daemon had unreadable subtrees.

The relay path drives:
- `crates/blit-cli/src/transfers/mod.rs:230` `TransferRoute::RemoteToRemoteRelay` →
  `run_remote_push_transfer(args, Endpoint::Remote(src), dst, mirror)` (R2-H1 evidence
  point 1).
- `crates/blit-core/src/remote/transfer/source.rs:233` `RemoteTransferSource::scan`
  takes `_unreadable_paths: Arc<Mutex<Vec<String>>>` with a leading underscore — the
  channel is **ignored**. The scanner pulls header metadata via the legacy Pull RPC
  (`scan_remote_files`) and forwards headers only; daemon-side enumeration errors are
  not propagated back as unreadable entries (R2-H1 evidence point 2).
- `crates/blit-core/src/remote/push/client/mod.rs:842-852` derives
  `scan_complete = unreadable_paths.lock().map(|g| g.is_empty()).unwrap_or(false)`,
  which for the relay path is **always `true`** because the source scanner never
  populates that list (R2-H1 evidence point 3).
- Destination then runs mirror's destination-purge step under the assumption the
  source view was complete, deleting destination-only files that may correspond to
  the missing source entries.

The `move --relay-via-cli` reject-gate at `transfers/mod.rs:579-589` (R50-F1 / R51-F2,
2026-05) already closed this whole data-loss class for `move`. `mirror` was never
gated. `copy --relay-via-cli` has the same incomplete-scan exposure but no
destination-purge step, so an incomplete source scan loses **no data** on the
destination side; this slice does not touch copy.

## Approach

Symmetric reject-gate with `move`'s pattern. Placed inside the `RemoteToRemoteRelay`
match arm in `run_transfer`, gated on `mirror=true`, before
`ensure_remote_push_supported` so we bail as early as the other data-loss-class gates.
Error message points the user at the safe escape (drop `--relay-via-cli` to use the
delegated path, which enforces `require_complete_scan` end-to-end).

The narrower / more thorough alternative — plumb `unreadable_paths` through the relay
source scanner so the existing safety guard fires correctly — restores the feature
but needs proto + daemon + CLI work and is tracked as a follow-up (see Known gaps).

## Files changed

- `crates/blit-cli/src/transfers/mod.rs`:
  - `+18` lines in `run_transfer`'s `RemoteToRemoteRelay` arm: gate-comment +
    `if mirror { bail!(...) }`.
  - `+47` lines in `mod tests`: two regression tests
    `mirror_rejected_with_relay_via_cli_for_remote_to_remote` (positive, asserts
    bail message) and `copy_relay_via_cli_does_not_trip_mirror_gate` (negative,
    asserts copy does NOT see the new message).

## Tests added

- `mirror_rejected_with_relay_via_cli_for_remote_to_remote` — pins the data-loss
  rejection contract. Calls `run_transfer` with mirror + relay + remote→remote
  endpoints (`host-a:/m/`, `host-b:/m/`); the bail fires before any RPC so no daemon
  is needed.
- `copy_relay_via_cli_does_not_trip_mirror_gate` — asserts `copy --relay-via-cli`
  between two remote endpoints does NOT pick up the new mirror-only message. The
  copy path still bails (no listener) but with a different downstream error.
- Existing `remote_to_remote_explicit_relay_uses_legacy_cli_byte_path`
  (`crates/blit-cli/tests/remote_remote.rs:242`) continues to pass — it tests
  `copy --relay-via-cli` end-to-end and was deliberately untouched.

Workspace validation suite green: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`.

## Known gaps

- **Restoring relay-mirror as a working feature.** The full fix is to plumb
  `unreadable_paths` from the source daemon through `scan_remote_files` /
  `RemoteTransferSource::scan` so `send_manifest_complete` can report the real
  signal. That needs a proto change (or a side-channel) and is a multi-slice
  follow-up. This slice closes the data-loss window by refusing the combination,
  matching move's posture.
- **Source-daemon-side enumeration errors.** Even on the delegated path,
  `require_complete_scan` only catches what the source daemon's scanner classifies
  as unreadable. Anything below that (kernel-level FS hangs, partial directory
  reads that don't surface as errors) is out of scope.

## Cross-references

- R3 finding H1 (top-priority data-loss item), see
  `docs/audit/AUDIT_REPORT_2026-06-04_R2.md` H1 and R3 file `_R3.md`.
- Move's symmetric gate at `crates/blit-cli/src/transfers/mod.rs:568-590`
  (R50-F1 / R51-F2).
- Move's symmetric regression test at
  `crates/blit-cli/tests/local_move_semantics.rs:380-402`.
- `RemoteTransferSource::scan` ignored-`_unreadable_paths` parameter:
  `crates/blit-core/src/remote/transfer/source.rs:233`.
- `send_manifest_complete` scan-complete derivation:
  `crates/blit-core/src/remote/push/client/mod.rs:842-852`.
