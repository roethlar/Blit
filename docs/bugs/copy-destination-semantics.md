# Bug: `copy`/`mirror`/`move` destination semantics are inconsistent and non-rsync

**Component:** `blit-cli` (`crates/blit-cli/src/transfers/local.rs`, `crates/blit-cli/src/transfers/remote.rs`), `blit-core` (`crates/blit-core/src/orchestrator/orchestrator.rs`)
**Severity:** High — silent data placement corruption. Users lose the directory boundary of the thing they copied; contents get merged into the destination parent.
**Reported against:** `blit-cli` as built from current `main` (binary at `target/release/blit-cli`).

## Summary

`blit copy SRC DEST` has **different destination semantics depending on which transport path is hit**, and none of the paths implement rsync's trailing-slash convention that experienced users expect:

| Transport | Behavior when `SRC` is a directory and `DEST` is an existing directory |
|---|---|
| **Local → local** | Contents of `SRC` are merged into `DEST`. `DEST/<basename(SRC)>/` is **not** created. |
| **Remote → local** (pull) | `DEST/<basename(SRC)>/` **is** created (rsync-ish), based on `dest.is_dir()` — a trailing `/` on `SRC` is ignored. |
| **Local → remote / remote → remote** (push) | Not investigated in this report; needs audit. |

Additionally, **no path honors the rsync trailing-slash distinction** on the source (`SRC` vs `SRC/`). Both the remote-pull heuristic and the local path ignore it entirely.

The practical consequence: the "right" invocation of `blit copy` depends on which side is remote, and there is no way to express "copy the directory itself, don't merge its contents" in the local-to-local case short of pre-computing the full target path.

## Reproducer

```bash
mkdir -p /tmp/blit-bug/src/GameDir/{a,b}
echo one > /tmp/blit-bug/src/GameDir/a/one.txt
echo two > /tmp/blit-bug/src/GameDir/b/two.txt
mkdir -p /tmp/blit-bug/dst/common

# User expectation (rsync-style): GameDir/ appears inside common/
blit-cli copy /tmp/blit-bug/src/GameDir /tmp/blit-bug/dst/common/

# Actual result: common/ now contains a/ and b/ directly
ls /tmp/blit-bug/dst/common/
#   a  b                           <-- WRONG, expected: GameDir
ls /tmp/blit-bug/dst/common/GameDir 2>/dev/null || echo "no GameDir, as reported"
```

Trying the rsync workaround (no trailing slash on source) does not help:

```bash
blit-cli copy /tmp/blit-bug/src/GameDir  /tmp/blit-bug/dst/common   # still merges
blit-cli copy /tmp/blit-bug/src/GameDir/ /tmp/blit-bug/dst/common/  # identical
```

For contrast, the remote-pull path behaves differently (rsync-ish) from the same CLI:

```bash
# Assume a daemon serving /tmp/blit-bug/src as module "m"
blit-cli copy server:/m/GameDir /tmp/blit-bug/dst/common/
ls /tmp/blit-bug/dst/common/GameDir   # exists
```

So identical-looking CLI invocations produce different directory layouts depending only on whether the source is remote.

## Root cause

### Local-to-local path

`crates/blit-cli/src/transfers/local.rs` passes `dest_path` through to
`crates/blit-core/src/orchestrator/orchestrator.rs::execute_local_mirror` unmodified. The orchestrator treats `dest_root` as the literal target root and only creates its *parent*:

```rust
// crates/blit-core/src/orchestrator/orchestrator.rs:46-52
if !options.dry_run {
    if let Some(parent) = dest_root.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create destination parent {}", parent.display())
        })?;
    }
}
```

There is no `dest.is_dir()` check, no basename append, no trailing-slash inspection. Enumeration then walks `src_root`'s entries and writes them as siblings under `dest_root`, which is how the merge behavior arises.

A workspace-wide grep for `trailing`, `ends_with('/')`, `ends_with("/")`, and `is_dir.*dest`/`is_dir.*dst` confirms no slash-aware or dir-aware destination handling exists in the local path or in `blit-core`.

### Remote-pull path

`crates/blit-cli/src/transfers/remote.rs:22-48` *does* implement a heuristic, but only on pull and only based on `dest.is_dir()`:

```rust
/// Compute the actual destination path for a pull operation using rsync-style semantics:
/// - If dest exists and is a directory, append the source's basename to create dest/basename/
/// - Otherwise use dest as-is (will be created as the target)
fn compute_pull_destination(dest: &Path, remote: &RemoteEndpoint) -> Result<PathBuf> {
    let source_basename = match &remote.path {
        RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
            rel_path.file_name().map(|s| s.to_os_string())
        }
        RemotePath::Discovery => None,
    };
    if dest.is_dir() {
        if let Some(basename) = source_basename {
            let basename_str = basename.to_string_lossy();
            if !basename_str.is_empty() && basename_str != "." {
                return Ok(dest.join(basename));
            }
        }
    }
    Ok(dest.to_path_buf())
}
```

Two problems with this, even in isolation:
1. It is **not** the rsync rule — rsync keys off a trailing slash on the *source*, not on whether the dest already exists. `blit copy server:/m/foo /tmp/new` creates `/tmp/new` (treating it as the target). `blit copy server:/m/foo /tmp/existing-dir` creates `/tmp/existing-dir/foo`. Whether the user gets "replace" or "nest" depends on a pre-existing filesystem state, which is fragile and race-y.
2. The local path doesn't implement even this weaker heuristic, so `blit` is self-inconsistent.

### Documentation

`docs/cli/blit.1.md:25` says only:

> `copy` copies a `<SOURCE>` into `<DESTINATION>` without deleting extraneous files.

"into" is ambiguous between "merge contents into" and "place the source under". The man page does not mention trailing-slash semantics, rsync compatibility, or the local/remote divergence.

`--help` for `copy` likewise says only "Source path for the transfer" / "Destination path for the transfer".

## Impact

- **Real-world bite, `migrate_games.sh`:** A Steam-library migration script that does `blit copy "$SRC/common/<Game>" "$DEST/common/"` (expecting rsync semantics) ends up scattering every game's files into `$DEST/common/` as siblings. First game "succeeds" but creates no game folder; subsequent games silently merge on top, corrupting everyone. No error is raised. See `~/dev/Move-SteamGame.ps1/migrate_games.sh:131`.
- **Silent failure mode:** The copy reports success and correct byte counts. The only symptom is missing/merged directories at the destination, which the user may not notice until they try to launch the thing they copied.
- **Surprise factor:** Anyone carrying rsync muscle memory will get wrong results. The project README even positions Blit relative to rsync ("makes rsync look like a Model T"), which sets the expectation that at least the basic CLI contract matches.

## Proposed fix

Pick **one** semantic and apply it uniformly across local, remote-pull, remote-push, and remote-remote paths. Document it in the man page and `--help`. Two reasonable options:

### Option A — full rsync trailing-slash semantics (recommended)

- `blit copy SRC DEST` → create/overwrite `DEST` as a copy of `SRC` (the directory itself).
- `blit copy SRC/ DEST` → copy the **contents** of `SRC` into `DEST` (merge).
- Error (or confirm) if `DEST` exists as a file when `SRC` is a directory.
- Independent of whether `DEST` currently exists — the user's intent comes from the source slash, not filesystem state.

This matches every user who has typed `rsync -a` in the last 25 years, which is the expected audience.

Implementation sketch:
- Parse the raw CLI string before it becomes a `PathBuf` (since `PathBuf` strips trailing slashes on some platforms). Record a `source_is_dir_contents: bool` flag on `TransferArgs`.
- In the orchestrator, if `!source_is_dir_contents && src.is_dir()`, rewrite `dest_root = dest_root.join(src.file_name())` before planning.
- Apply identically in `transfers/local.rs`, `transfers/remote.rs` (both pull and push), and remote-remote.
- Delete `compute_pull_destination` — it becomes unnecessary once the rule is uniform.

### Option B — always literal destination, no magic

- `blit copy SRC DEST` → `DEST` is always the exact target path. Never append basenames. Never inspect `dest.is_dir()`.
- Remove `compute_pull_destination` so pull matches local.
- Document prominently that users who want rsync-style nesting must spell out `DEST/$(basename SRC)` themselves.

This is simpler to implement (just delete the remote-pull heuristic) but will surprise every rsync user and re-break `migrate_games.sh`-class scripts unless they are rewritten.

**Option A is preferred.** It matches the de-facto standard in the space, it's what users of a "better rsync" already assume, and it's the only option where the same invocation behaves the same regardless of which side is remote.

## Tests to add

Under `crates/blit-cli/tests/` (new file `copy_destination_semantics.rs`) and/or `tests/integration/`:

1. `local_copy_dir_no_slash_creates_nested` — `copy /a/src /b/dst/` produces `/b/dst/src/...`.
2. `local_copy_dir_with_slash_merges` — `copy /a/src/ /b/dst/` produces `/b/dst/<src contents>`.
3. `local_copy_dir_dest_missing_creates_target` — `copy /a/src /b/new` creates `/b/new` as a copy of `src` (whichever semantic wins, pin it).
4. `local_copy_file_into_existing_dir` — `copy /a/f.txt /b/dst/` lands at `/b/dst/f.txt` (matches rsync + cp).
5. `local_copy_file_to_exact_path` — `copy /a/f.txt /b/renamed.txt` writes `/b/renamed.txt`.
6. `remote_pull_*` — mirror each of the above against a `server:/m/...` source; behavior must match the local case byte-for-byte.
7. `remote_push_*` and `remote_remote_*` — same matrix.
8. `mirror_*` and `move_*` — same matrix; `mirror` and `move` must use the same destination-resolution rule as `copy`, otherwise `move = mirror + rm` stops being true.

## Out of scope / follow-ups

- **`mirror` safety:** With rsync semantics, `blit mirror src/ /home` would be catastrophic (deletes everything in `/home` that isn't in `src`). The existing `--yes` / confirmation prompt logic should be re-audited once the destination rule changes, since the set of files considered "extraneous" shifts.
- **Windows paths:** Verify trailing-slash parsing works for `C:\path\` as well as `C:/path/`. The existing endpoint parser in `crates/blit-core/src/remote/endpoint.rs` already normalizes separators for remote paths; local paths need the same treatment *before* `PathBuf` canonicalization eats the trailing slash.
- **Bare filename destinations:** Decide whether `blit copy foo bar` (both relative, neither exists) creates `bar` as a file-or-dir copy of `foo` or errors. Rsync creates.

## References

- Offending code:
  - `crates/blit-core/src/orchestrator/orchestrator.rs:36-52` — local orchestrator, no dest rewriting.
  - `crates/blit-cli/src/transfers/local.rs:12-88` — local CLI entry, passes `dest_path` through unchanged.
  - `crates/blit-cli/src/transfers/remote.rs:22-48` — pull-only rsync-ish heuristic, `is_dir`-based not slash-based.
- Docs that need updating alongside the fix:
  - `docs/cli/blit.1.md` (SYNOPSIS and "Commands" section).
  - `--help` text for `copy`, `mirror`, `move` in `crates/blit-cli/src/cli.rs` (or wherever `TransferArgs` is defined).
  - `README.md` if it makes any rsync-comparison claims about CLI compatibility.
