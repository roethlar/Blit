# e-2-tab-strip-counts: reopened

Reviewed sha: `859a06283d1077d257750e4ca8ad33777505046b`

Verdict: reopened.

## Findings

1. High - The active/recent counts ignore F4 local transfers.

   The tab strip is described as telling the operator "whether anything's
   transferring right now", but the counts are populated only from the F2
   daemon stream state: `app.transfers.active_count()` and
   `app.transfers.recent_count()` (`crates/blit-tui/src/main.rs:397`).
   The local copy/mirror work added in `d-4-f4-local-transfers` lives in the
   separate `AppState::transfer` field (`crates/blit-tui/src/main.rs:200`).
   So if an operator starts a local copy from F4, the global header still says
   `0 active`, and after it completes it still says `0 recent`.

   The count needs to include the F4 local-transfer state, or the label needs
   to make clear that it is daemon-job-only. Given this slice's stated goal,
   the former is the expected fix.

2. Medium - The daemon count includes the synthetic Local row.

   `daemons` is computed as `app.daemons.rows().len()`
   (`crates/blit-tui/src/main.rs:396`), but `DaemonsState::new()` always
   inserts the synthetic local row before any mDNS discovery has happened
   (`crates/blit-tui/src/daemons.rs:175`, `crates/blit-tui/src/daemons.rs:181`).
   That means the header reports at least `1 daemons` even when zero remote
   daemons have been discovered and no local daemon is known to be running.

   This contradicts the finding doc's promised `0 daemons` empty-state line
   and makes the count poor feedback for "whether mDNS has settled". Count
   discovered remote daemons, or rename/reframe the metric as endpoints and
   account for the synthetic row explicitly.

3. Medium - The fixed right column clips the tab labels on common terminal
   widths.

   `render_tab_strip` reserves `Constraint::Length(48)` for the right-hand
   counts and leaves the tabs in `Constraint::Min(28)`
   (`crates/blit-tui/src/screens/mod.rs:74`). The F1-F4 label spans are about
   55 columns before the counts are considered (`crates/blit-tui/src/screens/mod.rs:51`).
   On an 80-column terminal this allocates roughly 32 columns to the tabs,
   so F3/F4 are clipped while the counts remain reserved. Even a 100-column
   terminal is tight for the full tab text plus the fixed count column.

   This is a regression in the primary navigation surface, not just a known
   gap where the right-side counts clip. The layout needs a responsive form:
   shorten/hide counts first, use compact labels, or otherwise preserve the
   tab keys before spending width on summary text.

## Validation

Run in detached worktree at the reviewed SHA:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed: 143 tests.
- `cargo test --workspace` passed.
