# cr-ecm4-2: A `--files-from` list that holds no name was "no list" at a peer

**Severity**: HIGH — the peer applied the patterns the list overrides: it selected a file the user's list did not name and put a destination-only file in a mirror's deletion plan
**Status**: Verified — `exclude-case-r5` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..585d3599`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding (it found builds of contract 9 made before the fix accepting the request and discarding the marker — cr-ecm5-1 — and an empty list still letting empty directories into the deletion plan — cr-ecm5-5)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `697eefce`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r4` over `3bc3ee47..0669279f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r4.codex.md`.

## Evidence
With a `--files-from` file of comments only, `--include-from` holding `*.jpg`, and `--ignore-case`, the local filter selected no files. After encoding and a peer's decoding, the same filter selected `new.JPG` and put destination-only `only.JPG` in the deletion plan.

Cause: on the wire a list is `FilterSpec.files_from` plus `files_from_raw`, two repeated fields. A repeated field with no entry and a repeated field that was never set are one encoding, and `filter_from_spec` (`crates/blit-core/src/remote/transfer/operation_spec.rs`) built a list only when one of them held an entry. The ambiguity is older than this plan; the plan's promise that a list of exact names takes precedence over the patterns is what it broke.

## Predicted observable failure
`blit mirror --files-from LIST --include '*.jpg' SRC HOST:/mod/` where LIST holds no name (an empty file, or comments): this machine copies nothing, but the daemon runs with the include alone — it accepts every `.jpg` and deletes its own `.jpg` files the source lacks. The same in the other direction and between two daemons.

## Intake
Admitted, though the encoding predates this plan: the reviewer shows a deletion.

## What
The spec says whether a list was given, apart from what it holds: `FilterSpec.files_from_set` (field 10), set by `build_spec` whenever `--files-from` or a retry's list is in force. `filter_from_spec` builds the list when the field is set or either list holds an entry, so an empty list arrives as an empty list and lets nothing pass.

## Guard proof
- `transfers::filter::tests::a_list_that_holds_no_name_is_still_a_list_on_the_wire`: with no list the include decides at both ends; with the comments-only list nothing passes at either end (four names), the spec carries the flag and no entry, and the filter the peer builds is not "empty"; a retry's list sets the flag and still names its names.
- `ignore_case::a_list_that_holds_no_name_overrides_the_patterns_at_a_daemon_too`: through the binary against a real daemon, pushed and pulled — `mirror --ignore-case --include '*.jpg' --files-from LIST` copies nothing and deletes nothing.
- Mutations, each restored byte for byte: the spec never saying a list was given → both guards red; the peer reading an empty list as none → both guards red.

## Files changed
- `crates/blit-core/proto/blit.proto` — `FilterSpec.files_from_set = 10`.
- `crates/blit-core/src/transfers/filter.rs` — `build_spec` sets it; the unit guard.
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — `filter_from_spec` reads it; test literals.
- `crates/blit-daemon/src/service/delegated_pull.rs` — a test literal.
- `crates/blit-cli/tests/ignore_case.rs` — the end-to-end guard.

## Known gaps
- The field joins contract 9 and delegated spec version 4, both introduced by this plan and neither released. A build made between slice 3 and this commit speaks the same versions and does not read the field; against such a build the old behaviour remains. No such build was published (GitHub has had no push since 2026-10-24; no release carries contract 9).
- The daemon-to-daemon route is covered by the unit guard (one decoder, `filter_from_spec`, serves every route) and not by a run through two daemons.
- Run on Linux only.
