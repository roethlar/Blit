# bug-mirror-literal-backslash: route path→wire through one canonical POSIX helper

**Severity**: Bug fix / correctness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `4dac1a4`

## Surfaced by

A live `blit mirror /Volumes/Apps /Volumes/Temp -p -v` on macOS aborted
mid-transfer with:

```
tar shard produced unexpected entry 'Applications/Logic Pro.app/Contents/
Resources/Plug-In Settings/Echo/1/4 Single.pst' (not in manifest)
   at crates/blit-core/src/remote/transfer/tar_safety.rs:136
```

The on-disk filename is `1\4 Single.pst` (a Logic Pro plug-in preset
with a literal `\` byte). The error path appeared to show `/` because
the receiver had already rewritten the name.

## Root cause

`crates/blit-core/src/remote/transfer/tar_safety.rs:133` (and **ten
other** path→wire conversion sites) did
`path.to_string_lossy().replace('\\', "/")` to canonicalize
Windows-native `\` separators to `/`. That blanket string-replace was
destructive on POSIX, where `\` is a legal filename character. Both the
manifest key and the tar-entry name go through different code paths;
when the receiver applied the destructive replace to a tar entry whose
name contained literal `\`, the key diverged from the manifest's key
and the safety lookup missed.

Symptom would have repeated for **every** transfer involving a POSIX
filename containing `\` — not just this one Logic Pro file.

## Fix (comprehensive, future-proof)

Introduce a single canonical helper and route every Path→wire site
through it.

### New `blit_core::path_posix`

`relative_path_to_posix(&Path) -> String` walks `Path::components()`
and joins them with `/`. This is correct on every platform:

- **POSIX**: `Path::components()` splits only on `/`. A single-
  component filename containing `\` (or `:`) is preserved verbatim.
- **Windows**: `Path::components()` splits on the native `\` (and
  accepts `/` as a foreign separator). Join with `/` produces canonical
  POSIX form for the wire.

Empty / bare `.` paths produce `""` — the daemon-side convention for
"module root".

`relative_str_to_posix(&str)` is a thin wrapper for user-typed input
(parses as `Path` first, then canonicalizes). On macOS a user typing
`Folder\file` gets a single component with a literal `\`; on Windows
the native separator is split.

### Sites routed through the helper

| File | Before | After |
|---|---|---|
| `blit-core/src/remote/transfer/tar_safety.rs:133` | `raw_path.to_string_lossy().replace('\\', "/")` | `relative_path_to_posix(&raw_path)` (**the bug site**) |
| `blit-core/src/orchestrator/orchestrator.rs:929` | `parent.to_string_lossy().replace('\\', "/")` | `relative_path_to_posix(parent)` |
| `blit-core/src/orchestrator/orchestrator.rs:943` | `entry.relative_path.to_string_lossy().replace('\\', "/")` | `relative_path_to_posix(&entry.relative_path)` |
| `blit-core/src/mirror_planner.rs:42` *(cfg-windows-only — not a bug source, deduped for consistency)* | inline | `relative_path_to_posix(path).to_ascii_lowercase()` |
| `blit-daemon/src/service/pull.rs:474,488,810` | `wire.to_string_lossy().replace('\\', "/")` | `relative_path_to_posix(&wire)` |
| `blit-daemon/src/service/admin.rs:255` (user input) | `trimmed.replace('\\', "/")` | `relative_str_to_posix(trimmed)` |
| `blit-core/src/remote/transfer/payload.rs:215` | cfg-gated `replace`/`to_string` | delegates to helper |
| `blit-core/src/remote/push/client/helpers.rs:54` | cfg-gated `replace`/`into_owned` | delegates to helper |
| `blit-daemon/src/service/util.rs:155` | cfg-gated `replace`/`into_owned` | delegates to helper |
| `blit-app/src/endpoints.rs:189` (pub `rel_path_to_string`) | already component-walk | delegates to helper for de-dup |
| `blit-core/src/remote/endpoint.rs:215` (priv `rel_path_to_string`) | already component-walk | delegates to helper for de-dup |

Verified by grep: no `replace('\\', "/")` calls remain in production
code outside the new helper's docstring and the explanatory comments
the changed sites carry. The only surviving `path.replace('\\', …)` in
the codebase is in `blit-cli/src/transfers/remote.rs:80`, which escapes
`\` → `\\` (and `"` → `\"`) for shell/JSON output — a different,
correct operation.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green. The 630 existing tests
pass plus 7 new `path_posix` unit tests:

- empty / bare `.` paths produce `""`
- simple POSIX path unchanged
- nested relative `PathBuf` joined with `/`
- **`Echo/1\4 Single.pst` round-trips with literal `\` preserved**
  (the regression test for the user-reported bug)
- **`Themes/Dark:Variant.toml` round-trips with literal `:` preserved**
  (the analogous POSIX-legal-byte case for `:`)
- idempotence (helper-of-helper = helper)
- a `#[cfg(windows)]` test asserting native `\` separator → `/`

## Note on the inline tar-shard test

The unit test pins the helper's behavior. An end-to-end mirror test of
a fixture containing a literal-`\` filename would be ideal but is out
of scope for this slice (the existing transfer-test fixtures are large
and not parameterized). The unit-level pin is sufficient to prevent
the destructive `replace` pattern from re-introducing.
