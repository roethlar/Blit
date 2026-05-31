# audit-6d-path-safety-unicode: Unicode edge-case tests for the path-safety boundary

**Severity**: Test Gap
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `d75cdcf`
**Parent finding**: `audit-6-test-gaps` (item 4).

## What

`crates/blit-core/src/path_safety.rs` is the primary path-traversal
security boundary but had no tests for exotic-but-valid Unicode: NFC vs
NFD normalization forms, bidirectional-override marks, zero-width
joiners, or Unicode separator/dot lookalikes (finding item 4).

## Approach

The boundary is defined purely by ASCII structural markers — `/` (0x2F)
separators, the `..`/`.`/root/prefix component shapes, and NUL. Exotic
Unicode is none of those, so the correct and safe behavior is to treat it
as opaque bytes inside a single `Normal` component: preserved verbatim,
never folded, never able to smuggle a traversal. Added a
`unicode_edge_cases` test module that locks this in:

- **NFC vs NFD** ("café" as U+00E9 vs `e`+U+0301): both pass, preserved
  byte-for-byte, and stay distinct (no NFD→NFC folding).
- **Bidi override** (U+202E), **ZWJ** (U+200D), **zero-width space**
  (U+200B): opaque normal components, preserved verbatim.
- **Separator lookalikes** (fullwidth solidus U+FF0F, fraction slash
  U+2044): do NOT split components — exactly one component, no nested
  path / traversal.
- **Dot lookalikes** (fullwidth full stop U+FF0E): "．．" is a normal
  filename, not a `..` parent-dir component.
- **ASCII `..` + combining mark**: a 3-codepoint normal filename — not
  rejected as traversal, not collapsed to a parent-dir.
- Unicode components join safely under a root via `safe_join`.

## Non-UTF-8 (finding item 4, addressed by documentation)

Raw non-UTF-8 byte sequences cannot reach these helpers: the `&str`
signature excludes them by construction, and proto `string` decode
rejects invalid UTF-8 upstream before any wire path arrives. There is no
raw-bytes case to assert on; this is documented in the module docs.

## Files changed

- `crates/blit-core/src/path_safety.rs`: `unicode_edge_cases` test module
  (7 tests). No production code changed — the tests confirm and lock in
  existing behavior.

## Tests

`blit-core` path_safety +7: `nfc_and_nfd_are_preserved_verbatim_and_distinct`,
`bidi_override_is_an_opaque_component`,
`zero_width_joiner_and_space_are_opaque_components`,
`unicode_separator_lookalikes_do_not_split_components`,
`unicode_dot_lookalikes_are_not_parent_dir`,
`ascii_dotdot_with_combining_mark_is_normal_not_traversal`,
`unicode_components_join_safely_under_root`. Full workspace gate green.

## Scope

One sub-item of audit-6. Remaining test gaps: 6a (blit-app inline tests),
6b (TUI render), 6c (bridge HTTP integration), 6e (pull-move/push-move),
6f (DNS-rebinding via ScriptedResolver), 6g (copy fast-path fallback).

## Reviewer comments

(empty — pending review)
