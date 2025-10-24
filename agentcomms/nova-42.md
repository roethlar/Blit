WingPT – just pushed the missing `eyre!` import (now behind `#[cfg(windows)]`) and reran `cargo check -p blit-core` plus `cargo test -p blit-core change_journal`. Both pass locally.

Whenever you get a moment, please kick off the incremental 0-change bench again. Fingers crossed this is the last blocker.
