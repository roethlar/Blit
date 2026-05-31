# audit-6a-blit-app-filter-tests: unit tests for the shared filter-assembly helpers

**Severity**: Test Gap
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `8820226`
**Parent finding**: `audit-6-test-gaps` (item 1).

## What

audit-6 item 1 flagged blit-app's orchestration glue as untested. The
crate has since gained `#[cfg(test)]` modules in
endpoints/dispatch/check/resolution/remote/jobs/client/display, so the
"zero inline tests" premise is **stale** — but `transfers/filter.rs`
(`build` / `build_spec`), the pure filter-assembly helpers every
transfer/check verb routes through, still had none.

## Approach (no production change)

Added a `tests` module with 6 tests:

- `build_empty_inputs_yields_unconstrained_filter` — no constraints; no
  `reference_time` captured.
- `build_propagates_globs_and_sizes` — include/exclude verbatim; min/max
  size cross-checked against `blit_core::fs_enum::parse_size` (tests the
  wiring, not parse_size itself).
- `build_age_constraint_captures_reference_time` — an age bound captures
  `reference_time` once at build time.
- `build_rejects_malformed_glob_with_pointer` — `"a["` →
  "invalid filter pattern".
- `build_rejects_bad_size_with_flag_context` — bad `--min-size` surfaces
  the flag in the error chain.
- `build_spec_maps_age_to_seconds_and_propagates_globs` — Duration→secs
  mapping on the wire `FilterSpec`.

## Files changed

- `crates/blit-app/src/transfers/filter.rs`: `tests` module (6 tests). No
  production change.

## Scope

One sub-item of audit-6. Remaining: 6b (TUI render), 6e
(pull-move/push-move). 6c/6d/6f/6g verified.

## Reviewer comments

(empty — pending review)
