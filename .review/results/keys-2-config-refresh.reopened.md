Reviewed sha: `a5ecfcd4fd94aa4905d497bc18e367247d6271c9`

# Reopened: keys-2-config-refresh

## Finding

1. **Medium — valid key configs can silently make refresh unreachable.**

   `KeysDefaults` now accepts both `quit` and `refresh` as independent
   single-character bindings (`crates/blit-tui/src/config.rs:80`), but the
   dispatcher applies a fixed priority: quit first, then navigation aliases,
   then refresh (`crates/blit-tui/src/main.rs:5449` and
   `crates/blit-tui/src/main.rs:5497`). That means a valid config such as
   `[keys] quit = "r"` with the default `[keys] refresh = "r"` makes plain
   `r` quit and leaves no refresh key at all. Similarly, `[keys] refresh =
   "q"` with the default quit key is accepted but can never refresh because
   `q` quits first.

   This is a new behavior hole introduced by adding the second configurable
   key: single-character validation passes, no startup warning is emitted,
   but the requested refresh binding is not usable. The fix should either
   reject/warn on collisions and fall back to a reachable binding, or define
   and test an explicit collision policy so operators are not left with a
   silently dead refresh key.
