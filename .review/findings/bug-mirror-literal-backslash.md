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

## Round 1 — Reopened by GPT review

> Medium: `crates/blit-daemon/src/service/admin.rs:259` now canonicalizes
> completion input with `relative_str_to_posix`, which uses
> `Path::components()`. That drops trailing slashes. So a completion
> prefix like `sub/` becomes `sub`; `rsplit_once('/')` no longer sees a
> directory prefix, and `split_completion_prefix` searches module root
> for entries starting with `sub` instead of listing inside `sub/`.
> This regresses remote completions for directory prefixes. The fix
> should preserve "user typed trailing slash means complete inside that
> directory" while still avoiding destructive backslash replacement.
>
> Gates at `4dac1a4`: fmt / clippy / test all green.

Verified: reproduced with a minimal `Path::new("sub/").components()`
print — yields a single `Normal("sub")` component, so canonical output
is `"sub"`. `is_separator('/')` is true on POSIX and `is_separator('\')`
is false; that's the right primitive for restoring the trailing-slash
UX semantic without re-introducing the destructive backslash replace.

## Round 2 — Fix (commit `5a034dd`)

`relative_str_to_posix` now preserves trailing-separator semantics for
user input, while `relative_path_to_posix` (the wire/manifest canonical
form) is unchanged.

The new behavior detects whether the raw `&str` input ended with a
platform path separator via `std::path::is_separator` (POSIX: only `/`;
Windows: `\` or `/`) and re-attaches a trailing `/` after canonical
component-join when it did.

Behavior matrix on POSIX:

| Input | Output | Notes |
|---|---|---|
| `"sub/"` | `"sub/"` | trailing slash preserved (the regression fix) |
| `"sub"` | `"sub"` | no trailing slash invented |
| `"sub\"` | `"sub\"` | literal trailing `\` preserved (POSIX-legal char, NOT a separator) |
| `"Folder/sub/"` | `"Folder/sub/"` | |
| `"1\4 Single.pst"` | `"1\4 Single.pst"` | the original bug, still fixed |

On Windows, native trailing `\` correctly converts to `/`
(`is_separator('\')` is `true`).

Four new regression tests in `path_posix::tests` pin both halves of
the contract (preserve trailing `/`, do NOT invent one). All 10
`path_posix` tests + full workspace test suite green at `5a034dd`.

`relative_path_to_posix` (the public wire/manifest helper) is
deliberately left unchanged — manifest paths and tar entry names
should not carry trailing separators; the trailing-slash semantic is
strictly a user-input UX convention.
