# cr-uv7-3: a source outside the crate could not hand its walk's record back

**Severity**: MEDIUM — a `TransferSource` written outside `blit-core` can record what its walk saw and give it to nobody: a move from it is given no account and removes nothing
**Status**: Verified — `combined-r19` (Codex openreview, 2026-11-22) reviewed `ca377805..17f4eb97` — the eight `combined-r18` fixes — with `guard_confirmed: true`: "The public per-scan record lets an external source participate in move accounting"; the reviewer's own guard was this fix's (the record discarded → the external-source test red)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `de358e10`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r18` over `7912da16..ca377805` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r18.codex.md`. It completes cr-uv6-4 (`4d18150d`).

## Evidence
"An external `TransferSource` can collect observations but cannot attach them to a newly constructed `SourceScan`. My consumer check failed with Rust error `E0624` because `seeing` is crate-private. Expose a supported constructor or pass the per-scan collector into the source."

## Intake
Admitted as the reviewer put it. `blit-core` is the platform crate third-party embedders consume (D-2026-08-17-2); a public trait method whose result such an embedder cannot build is no part of that platform.

## What
`SourceScan::seeing` is public. `moved::Sightings`, `sighting`, `name_key` and `Version` already were.

## Guard proof
- `crates/blit-core/tests/scan_seeing_outside.rs::a_source_outside_the_crate_hands_its_walks_record_back` — a test binary, so a crate of its own: a source of one file that looks at it itself, records what it saw, and hands the record back from `scan_seeing`; the account from it holds the file. A plain scan hands nothing back.
- Proved by hand (a build failure is not a red test to the mutation driver): with `seeing` the crate's alone, the guard does not compile — `error[E0624]: method 'seeing' is private`, the reviewer's own; restored byte for byte, it passes.

## Files changed
- `crates/blit-core/src/remote/transfer/source.rs` — `SourceScan::seeing`.
- `crates/blit-core/tests/scan_seeing_outside.rs` — guard (a new test binary: 57).

## Known gaps
- An outside source must still record what its walk saw itself, in the form `moved::sighting` gives; nothing checks that the record and the manifest agree.
