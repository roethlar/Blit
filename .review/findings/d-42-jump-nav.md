# d-42-jump-nav: g/G jump-to-first/last on F1 and F3

**Severity**: Feature (navigation polish)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `4ee9d24`

## What

The F1 daemon list and F3 browse list support `j`/`k` (and
arrows) for one-row cursor moves, but no way to jump to the top
or bottom of a long list. d-42 adds vim-style `g` (first row)
and `G` (last row), with `Home`/`End` as aliases for operators
who don't think in vim.

This extends the existing cursor model the same way the d-19
digit-tab aliases extended pane navigation beyond the strict
design — a small, expected ergonomic that costs one keystroke
instead of holding `k` down a 50-row list.

## Approach

- `browse::BrowseState::select_first` / `select_last` — **filter
  aware**: they land on the first / last row that matches the
  active d-26 filter (via the existing `first_matching_row` and a
  reverse scan), never on a row hidden by the filter. With no
  filter they're just row 0 / last row.
- `daemons::DaemonsState::select_first` / `select_last` — row 0 /
  last row (the F1 list has no filter), no-op when empty.
- `UserAction::SelectFirst` / `SelectLast`; `key_action` maps
  `g`/`Home` → SelectFirst, `G`/`End` → SelectLast.
- F1 and F3 dispatch arms call the new state methods. F2/F4
  ignore the variants (no arm) — F2's cursor is transfer_id
  anchored (d-21), a different model, intentionally left out.

### Single `g`, not `gg`

Vim's "first line" is the `gg` chord. d-42 uses a single `g`
because a chord needs a pending-key state machine (track "saw
one g, waiting for the second") with a timeout — disproportionate
for a list jump. `G` is the natural single-key pair, and `g`
alone is unambiguous here since nothing else binds it.

### Input-mode safety

`g`/`G` are global in `key_action`, but the F3 filter-edit and
pull-dest input handlers run *before* the dispatcher and absorb
characters while active — so typing `g` into a filter or a
destination path inserts the letter rather than jumping. Same
guard the existing `/`, `p`, `u` keys rely on.

## Files changed

- `crates/blit-tui/src/browse.rs`: `select_first` / `select_last`
  (filter-aware) + 3 tests.
- `crates/blit-tui/src/daemons.rs`: `select_first` / `select_last`
  + 1 test.
- `crates/blit-tui/src/main.rs`: `UserAction::SelectFirst` /
  `SelectLast`; `g`/`G`/`Home`/`End` key mapping; F1 + F3
  dispatch arms; 1 key_action test.
- `crates/blit-tui/src/help.rs`: `g / G` keymap row; modal height
  39→40; keymap test asserts the new row.

## Tests

+5 tests (428 → 433 in blit-tui):

- `browse::select_first_and_last_no_filter` — `g`/`G` land on the
  alphabetically-sorted first/last (backups / scratch).
- `browse::select_first_and_last_are_filter_aware` — with filter
  `s`, `g` lands on the first *matching* row (backups, not raw
  row 0 home) and `G` on the last match (scratch).
- `browse::select_first_last_noop_on_empty`.
- `daemons::select_first_and_last_jump_the_cursor`.
- `main::key_action_maps_jump_keys` — g/Home → SelectFirst,
  G/End → SelectLast.

`cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` all
green.

## Known gaps

1. **F2 not covered.** F2's active-row cursor is anchored by
   `transfer_id` (d-21) and the row set churns live from the
   Subscribe stream, so "first/last" is a moving target with
   different semantics. Left out deliberately to keep this
   atomic; a future slice could add it against the d-21 anchor.

2. **No `gg` chord.** Single `g` only (see Approach). If muscle
   memory demands `gg`, a pending-key layer is a separate slice.

## Out of scope

- F2 jump navigation.
- Vim `gg` double-tap chord.
- PageUp/PageDown half-page scrolling.

## Reviewer comments

(empty — pending grade)
