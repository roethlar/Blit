# cr-ecm36-4: failures were not indexed by their exact bytes at the producer

**Severity**: HIGH — a retry is refused, or a reason given to the wrong file. Two producers kept a failed file by its text where several names share one text:
- **The sink's exact set** (`SinkOutcome`) was keyed by text, with one name's bytes per text. Seventy failed names of one spelling went out as one entry; the other 69 were counted unnamed, the saved record named only 65, and `blit jobs retry` refused.
- **The retry scan's unreadable entries** were keyed by text, so one entry's reason went to two names of that text.

And the summary's join (`RunTotals::failed_files`) searched a growing list for each name: 0.17, 0.68 and 2.72 seconds for 8,000, 16,000 and 32,000 names.
**Status**: Reviewed, found incomplete — `combined-r38` (Codex openreview, 2026-11-24) reviewed `2c55cb3a..6ee7ed5e` — the `combined-r37` fixes — with `guard_confirmed: true`: "Passing the root into the stream and tar writers, marking failed mmap copies through the open file handle, and preserving uncertainty after a source-only failure are sound changes." "I retried `dup�` and a sibling named `dup` followed by byte `FF`. After the UTF-8 source vanished, Blit copied the sibling and assigned its bytes to the missing file's failure. The next `jobs retry` exited **0** while `dup�` was still missing. Distinguish a known UTF-8 identity from one that still needs a manifest lookup." "With every failed file leaving a partial, the summary join took 91 ms for 6,000 files, 356 ms for 12,000 and 1.41 s for 24,000 in the debug build. It still scans the leftover vector for each identity. Build text and raw-byte lookup sets once." The remaining work is cr-ecm37-2 and -7
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c8fe7147`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r37` over `d2494965..2c55cb3a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r37.codex.md`. It completes cr-ecm35-3 (`4146e1fa`) and cr-ecm35-8 (`efc08c79`) in part.

## Evidence
"With 70 Unix filenames sharing one display spelling, one copied and 69 failed. The saved record named only 65, so `jobs retry` refused. The retry scan also assigned one unreadable entry's reason to two distinct raw names. Use exact identities in both producers and in the summary join. Its growing-vector search took 0.17, 0.68 and 2.72 seconds for 8,000, 16,000 and 32,000 unique names in the test build."

## Intake
Admitted as the reviewer put it.

## What
- **The sink.** `SinkOutcome` keeps every failed entry by its identity, text and bytes (`failed_identities`), in place of a text→bytes map.
  - Its count of unnamed failures is against the identities.
  - The session's manifest names an entry recorded by text alone (`named_raw`).
  - `wire_failed_identities` gives the exact set in wire form, one entry per identity, each with its own bytes. The session sends it; `wire_failed_paths` and `wire_failed_paths_raw` read from it.
- **The scan.** An unreadable entry (`Unreadable`) carries its exact bytes where the walk has them. The retry scan finds a requested name's entry by its text and its bytes.
- **The join.** `RunTotals::failed_files` joins by hash, in linear time.

## Guard proof
- `sink::tests::many_names_of_one_text_are_each_their_own_failure`: seventy failures of one text, each with its own bytes, leave nothing unnamed, and the wire form names all seventy, each by its bytes.
- `transfer_session::tests::a_scan_entry_is_found_by_its_bytes_too` (Unix): two requested names of one text; the scan's entry for `dup\xff` is that one's reason only, and `dup\xfe` is reported missing.
- Mutations, each restored byte for byte:
  - counted by the text → red;
  - the scan's entry found by its text alone → red.
- Checks: native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `failed_identities`, `wire_failed_identities`; guard.
- `crates/blit-core/src/remote/transfer/source.rs` — `Unreadable.raw`, `record_unreadable_entry`.
- `crates/blit-core/src/transfer_session/mod.rs` — the session sends the identities; the scan's lookup by bytes; guard.
- `crates/blit-core/src/run_log.rs` — the join by hash.

## Known gaps
- The join's speed has no guard of its own: a timing test would be flaky. The reviewer's 32,000-name case is now one hash lookup per name.
- An unreadable entry the walk reports without its bytes (a suppressed walk error, a Windows metadata failure) is found by its text alone.
