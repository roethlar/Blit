# ldt-4-r1-f7a — reject duplicate keys in immutable trace JSON

**Severity**: LOW — conflicting trace fields could collapse last-wins before
the analyzer's exact schema and evidence-integrity checks.
**Status**: Fixed and mutation-proved; neutral whole-change re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `efc796a`

## Evidence

`scripts/ldt4_rigw_analyze.py:1084-1105` used ordinary `json.loads` for every
session-phase line. JSON permits the parser to accept repeated object names;
Python retains only the last value. A raw event containing conflicting
`"reason"` fields therefore became one ordinary dictionary before
`_event_exact` or policy replay could see the duplication.

## Predicted observable failure

A corrupted or hand-edited evidence line with duplicate decision fields can
pass last-wins as structurally exact. This does not occur from the serde
producer, but it weakens the analyzer's registered immutable/tamper-evident
input contract.

## What

Reject a repeated key anywhere in a trace JSON object before ordinary event
validation. The refusal names the exact trace line and duplicate key.

## Approach

The central loader supplies an `object_pairs_hook` that constructs dictionaries
only after proving every key is unique. A dedicated exception carries the key
back to the trace-context error. Because JSON invokes the hook for nested
objects too, the rule covers the complete line rather than only top-level
event fields.

## Files changed

- `scripts/ldt4_rigw_analyze.py` — duplicate-rejecting JSON object hook and
  contextual refusal.
- `scripts/ldt4_rigw_analyze_test.py` — raw conflicting-key trace guard.

## Guard proof

- The full analyzer test inserts `"reason":"hysteresis"` immediately before
  the valid `"reason":"cheap-up"` in one raw trace and requires a duplicate-key
  `AnalysisError`.
- Reverting the loader to ordinary `json.loads` makes the new test fail because
  no error is raised. Exact restoration passes the focused guard and all 74
  analyzer tests.

## Coder dispute

None.

## Known gaps

None. CSV duplicate-field handling was already explicit and is outside this
JSON-specific finding.

## Reviewer comments

Claude Fable 5/max returned the bundled integrity candidate over exact
`e41b871..0e48721` with `guard_confirmed=true`. Intake split duplicate-key
handling from numeric tolerance to preserve one finding per commit. Final
fixed-SHA whole-change re-review is pending.
