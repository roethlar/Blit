Reviewed sha: `5f92b668a22ec0efc3b181e5b303e79cd298be78`

Reopened.

Finding:

1. `crates/blit-cli/tests/remote_move.rs:173-181` only asserts that the remote source files inside `tree/` are gone after the remote→local directory move. It never asserts that `remote_sub` / `tree/` itself, or the nested `inner/` directory, was removed. The local→remote direction does assert the source root is fully removed at line 133, and this finding is specifically about directory-tree move coverage. Please add the symmetric remote-side directory assertions so the test pins recursive source removal, not only file unlinking.

I did not run cargo gates because this is a test-coverage reopen found during inspection.
