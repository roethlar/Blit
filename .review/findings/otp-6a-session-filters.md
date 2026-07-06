# otp-6a — filters on the session (source scan honors `open.filter`)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-6.
**Contract**: `docs/TRANSFER_SESSION.md`.
**Builds on**: otp-3 (session core), otp-4b/otp-5b (data plane, both directions).

## Staging

otp-6 is "mirror + filters on the session (one delete rule)". It splits into two
independently green, independently reviewable sub-slices (the otp-4b/otp-5b
precedent): **otp-6a = filters** (this slice); **otp-6b = mirror** (the delete
rule, which reuses the filter for scoping). Filters land first because the mirror
FilteredSubset scope is defined in terms of the filter.

## What

Until now the unified session refused any non-default `SessionOpen.filter` at OPEN
("filters are not implemented on the unified session yet (otp-6)"), so a filtered
`blit copy --include …` could not ride the session. This slice honors the filter
on the SOURCE scan: only matching files are manifested, diffed, and transferred —
whichever end initiates (push: source initiator; pull: source responder reads the
DESTINATION initiator's filter). No mirror/deletion behavior changes here.

## Predicted observable failure (closed by this slice)

A session opened with `filter.include = ["*.txt"]` over a mixed tree transfers ALL
files (filter ignored) instead of only the `.txt` files. Pinned by the new role-
suite test `source_filter_limits_manifest_under_both_initiators` (asserts
`files_transferred == 2`, the need list is exactly the two `.txt` paths, and the
`.log`/`.bin` files never land at the destination — under both initiator roles).

## Approach

The filter already has a wire type (`FilterSpec`), a validated runtime conversion
(`operation_spec::filter_from_spec`, which compiles each glob and rejects malformed
patterns — R5-F4), and a scan that accepts an optional `FileFilter`
(`TransferSource::scan`). The session simply had them disconnected. Three changes
reconnect them:

- `source_open_validator` no longer refuses a non-default filter; instead it
  **validates the globs** by running `filter_from_spec` and faulting
  (`protocol_violation`) on a malformed pattern. Because the validator runs on the
  responder's *received* open (via `responder_finish`), a hostile/buggy peer's bad
  glob is refused at OPEN and peer-notified, before any bytes move — rather than
  faulting mid-scan.
- `source_send_half` builds the `FileFilter` from `negotiated.open.filter` and
  passes it to `source.scan(…)` (was hardcoded `None`). Absent/default filter →
  `None` → scans everything (unchanged from otp-3). The conversion cannot fail on a
  validated open; any error is still mapped to a fault rather than unwrapped.
- `filter_from_spec` widened from private to `pub(crate)` so the session module can
  reach it (same crate).

The DESTINATION is untouched: it just receives a smaller manifest and diffs it as
before. Invariance holds by construction — the filter lives in the open, which both
ends share, and only the source consumes it.

## Files changed

- `crates/blit-core/src/transfer_session/mod.rs` — `source_open_validator` now
  validates globs instead of refusing; `source_send_half` builds and applies the
  scan filter.
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — `filter_from_spec`
  is now `pub(crate)`.
- `crates/blit-core/tests/transfer_session_roles.rs` — new test
  `source_filter_limits_manifest_under_both_initiators`; `run_session` refactored
  to delegate to a new `run_session_with_open` so a fixture can supply a custom
  open.

## Tests / guard proof

- `source_filter_limits_manifest_under_both_initiators` — reverting the
  `source.scan(scan_filter, …)` wiring to `source.scan(None, …)` makes it FAIL
  (all 4 files transfer, `files_transferred == 4 ≠ 2`); restoring makes it PASS.
  Guard proof run 2026-07-06.
- Full gate green: `cargo fmt --all -- --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace` → **1523 passed**
  (baseline 1522 + this test), 2 ignored. Count did not drop.

## Known gaps

- Mirror/deletion is still refused at OPEN (`destination_open_validator` on
  `mirror_enabled`) — that is otp-6b, deliberately out of this slice.
- The mirror-refusal test `mirror_request_is_refused_until_its_slice_lands`
  remains green and unchanged; it flips in otp-6b.
