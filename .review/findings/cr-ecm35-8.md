# cr-ecm35-8: the scoped scan's diagnostics were found by a prefix of their text

**Severity**: MEDIUM — a breach of D-2026-11-24-1 that misreports a reason. A retry pass's scoped scan reports every requested path it did not enumerate. When the scan had recorded an error for that path, the reason is that error. The scan kept its errors as messages, `"{path} ({reason})"`, and the report found a path's error by the prefix `"{path} ("`. So `a (backup)`'s entry answered for `a`, and its tail became the reason: "backup) (not found".
**Status**: Reviewed, found incomplete — `combined-r37` (Codex openreview, 2026-11-24) reviewed `d2494965..2c55cb3a` — the `combined-r36` fixes — with `guard_confirmed: true`: "Exclusive creation, applying permissions through the open handle, and sharing failure accounting between the log and saved record are sound choices. The unretried count also works through the CLI. With one requested name over the wire budget, it reported one failure and one file not retried." "With 70 Unix filenames sharing one display spelling, one copied and 69 failed. The saved record named only 65, so `jobs retry` refused. The retry scan also assigned one unreadable entry's reason to two distinct raw names. Use exact identities in both producers and in the summary join. Its growing-vector search took 0.17, 0.68 and 2.72 seconds for 8,000, 16,000 and 32,000 unique names in the test build." The remaining work is cr-ecm36-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `efc08c79`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r36` over `dc14afc4..d2494965` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r36.codex.md`.

## Evidence
"When `a` was missing and the scan recorded an error for `a (backup)`, the prefix lookup attributed `backup) (not found` to `a`. Carry the path and reason separately from enumeration."

## Intake
Admitted as the reviewer put it. It is the owner's rule: a decision is never read from words. The working agent's own scan under that rule passed over this line as a path prefix; it was a message's.

## What
- **A typed entry.** A scan's unreadable entry is `remote::transfer::source::Unreadable { path, reason }`, kept apart from enumeration on. Its `Display` is the former message, `path (reason)`. Every scan signature carries `Vec<Unreadable>`; `record_unreadable_entry` is the one maker.
- **Found by path.** `requested_but_unscanned` finds a requested path's entry by `entry.path == rel`.
- **For people.** The one surface that lists them — the refusal naming the first five, and `LocalMirrorSummary.unreadable_paths` — shows them as `path (reason)`. Nothing reads them back.

## Guard proof
- `transfer_session::tests::a_scan_entry_is_found_by_its_path_not_a_prefix`:
  - with an entry for `a (backup)` only, `a` is reported missing, not unreadable;
  - `b`'s own entry gives its reason.
- Mutation, restored byte for byte: found by a prefix → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,084 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/remote/transfer/source.rs` — `Unreadable`; `record_unreadable_entry`; the scan signatures; a test reads the fields.
- `crates/blit-core/src/transfer_session/mod.rs` — the lookup by path; guard; signatures.
- `crates/blit-core/src/transfer_session/local.rs`, `crates/blit-core/src/remote/transfer/session_client.rs` — the entries shown to people; signatures.
- `crates/blit-core/tests/{source_side_containment,transfer_session_roles,scan_seeing_outside}.rs`, `crates/blit-daemon/src/service/transfer_session_e2e.rs` — test signatures.

## Known gaps
- None known.
