Reviewed sha: `16e9466f732df3cccad5cdc6c759798ddbdb4982`

# Reopened: keys-1-config-quit

## Finding

1. **Medium — remapped quit keys hijack Ctrl/Alt chords for the same character.**

   In `crates/blit-tui/src/main.rs:5440`, `key_action` checks `is_quit`
   before the modifier-aware actions. `is_quit` then treats `code == quit`
   as a quit regardless of modifiers (`crates/blit-tui/src/main.rs:5961`).
   With the new config surface, `[keys] quit = "r"` makes `Ctrl+R` return
   `UserAction::Quit` before the existing reload branch can run, even though
   `Ctrl-R` is still documented as reload in `crates/blit-tui/src/help.rs:102`
   and covered only under the default keymap. The same pattern can steal
   future Ctrl/Alt chords for whichever character the operator picks.

   The configured quit character should only claim the plain typed character
   (allowing Shift/capital forms as appropriate), while `Esc` and `Ctrl+C`
   remain the modifier-aware failsafes. A regression test such as
   `quit = "r"` plus `Ctrl+R -> ReloadConfig` would pin the contract.
