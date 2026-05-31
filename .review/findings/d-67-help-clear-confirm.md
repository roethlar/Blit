# d-67-help-clear-confirm: flag the F4 clear y/N confirm in the keymap

**Severity**: Feature (doc-consistency — keymap honesty)
**Status**: In progress / pending review (round 2 — base refresh)
**Branch**: `phase5/a1`
**Commit**: `0f4cd64` (R1 `7c9589e`)

## What

d-66 put the F4 `[c] clear` behind a y/N confirm, but the `?`
help overlay still listed it as a bare "clear / disable / enable
history". Every other destructive keymap entry advertises its
prompt:

- F3 `m` / `v` / `D` — "y/N confirm"
- F2 `K` / `X` — "y/N prompt if [transfer] confirm_cancel"
- F4 `M` / `V` — "prompts before deleting …"
- **F4 `c` clear — (none)** ← this gap

So `c` was the lone destructive action whose help didn't tell
the operator it now prompts.

## Approach

- Annotate the shared `c / d / e` line inline:
  `clear (y/N) / disable / enable history`. Inline rather than a
  new split line because `d`/`e` are reversible (no confirm) and
  splitting would change `help_line_count` (which drives the
  d-31 scroll clamp) for no benefit.
- Extend `help_modal_documents_all_public_keys` to assert the
  F4 Profile-lifecycle section contains `y/N`, so the hint
  can't silently regress.

### Test-helper fix (related)

`section_contents` (the keymap test's section-slicer) still
mapped the header `"F1 · F3 navigation"`, but the rendered
header had grown to `"F1 · F2 · F3 navigation"` (F2 was folded
into that nav section around d-44 / d-44 F2 jump-nav) and the
helper was never updated. Effect: `next_header` lookup for the
global section found nothing → the "Navigation (global)" slice
silently ran to end-of-text. Corrected the chain to the real
header name.

## Considered and dropped

Also flagging the F1 `t` trigger's mirror/move confirm (d-65)
in its help line: the `t` value already overflows the modal's
inner width and clips mid-string (so does the pre-existing `K`
line), so an appended hint would never render. Widening the
modal is a separate concern (latent: long lines clip today),
out of scope for this slice. Left the `t` line unchanged.

## Files changed

- `crates/blit-tui/src/help.rs`: `c / d / e` line text;
  render-test assertion; `section_contents` header fix.

## Tests

557 total (unchanged count — the new assertion rides the
existing `help_modal_documents_all_public_keys` test).

## Known gaps

1. Help modal long-line clipping (`t`, `K` lines overflow at
   80 cols) — pre-existing, not addressed here.
2. **remote→remote (delegated)** trigger and **multi-daemon F2**
   remain the substantial outstanding TUI_DESIGN features.

## Round 2 (base refresh — no code change)

The R1 sentinel (`7c9589e`) was blocked only as a stacked descendant
of the reopened `d-65` base (the mirror-purge bug was still present in
the tree at that SHA). d-67's own help-text + test-helper change was
not faulted. Refreshed onto `0f4cd64` (d-65 R2 fixed); the d-67 change
itself is byte-identical.

## Reviewer comments

(empty — pending round-2 grade)
