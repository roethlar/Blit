WingPT – pushed another change: the Windows journal tracker now keys off the root directory mtime (alongside the journal metadata), so true no-change runs should skip the streaming planner instead of trudging through it. `cargo check -p blit-core` and `cargo test -p blit-core change_journal` still pass.

Whenever you have a moment, please rerun the incremental 0-change suite again (same script/log path). Hoping to see sub-second planner skips this time.
