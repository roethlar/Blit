# Bug: `blit copy <file> <dest>` silently copies nothing

**Status:** **RESOLVED.** Fixed in commit `6cf07bf` via a dedicated `execute_single_file_copy` branch in the orchestrator that handles file sources without routing through the dir-oriented enumeration pipeline. The "silent no-op" class of error was further guarded in commit `67bde6d` by distinguishing `TransferOutcome::UpToDate`/`SourceEmpty`/`Transferred` so a future regression to "0 files" would print a disambiguated message instead of a false "Copy complete". Regression test: `single_file_copy_idempotent` in `crates/blit-cli/tests/single_file_copy.rs`.

**Component:** `blit-core` (`crates/blit-core/src/enumeration.rs`, `crates/blit-core/src/orchestrator/fast_path.rs`, `crates/blit-core/src/orchestrator/orchestrator.rs`)
**Severity:** High — silent data loss. Blit reports "Copy complete" with success exit code, but the destination file does not exist. Callers that rely on the exit code (scripts, pipelines) cannot detect the failure.
**Reported against:** `blit-cli` as built from current `main` (binary at `target/release/blit-cli`, version line reports `blit v0.1.0`). Post-`2a72eb6` (the rsync-destination fix) — that fix resolves the *target path* correctly; this bug is in the *transfer* itself.

## Summary

Any local-to-local copy whose **source is a regular file** (as opposed to a directory) is silently treated as "no work". Blit prints the correct resolved destination path and a `Copy complete: 0 files, 0 B` summary, exits 0, but creates nothing at the destination.

This is not a trailing-slash / rsync-semantics issue — the destination resolution is correct. The transfer pipeline itself never enumerates the source file.

## Reproducer

```console
$ rm -rf /tmp/repro && mkdir -p /tmp/repro/{src,dst}
$ echo "hello world" > /tmp/repro/src/file.txt
$ blit-cli copy /tmp/repro/src/file.txt /tmp/repro/dst/
blit v0.1.0: starting copy /tmp/repro/src/file.txt -> /tmp/repro/dst/file.txt
Copy complete: 0 files, 0 B in 83.46µs
• Throughput: 0 B/s | Workers used: 32
$ echo $?
0
$ ls /tmp/repro/dst/
$                                   # EMPTY. File was not copied.
```

Note the first line shows destination resolution worked correctly (`.../dst/file.txt`). The problem is purely in the transfer.

All of these variants exhibit the same silent no-op (size, path style, and resolution style are all irrelevant):

| Invocation | Expected | Actual |
|---|---|---|
| `blit copy src/file.txt dst/` | `dst/file.txt` created | 0 files copied |
| `blit copy src/file.txt dst/renamed.txt` | `dst/renamed.txt` created | 0 files copied |
| `blit copy src/file.txt existing-dir/` | `existing-dir/file.txt` created | 0 files copied |
| `blit copy /big/single.iso /tmp/single.iso` (4 GB) | file copied | 0 files copied |
| `blit copy ./lone.txt ./lone-copy.txt` | `lone-copy.txt` created | 0 files copied |

The corresponding *directory* copy (`blit copy src/ dst/`) works. The bug is specific to file sources.

### Real-world trigger

Steam library migration:

```
=== [1778820] TEKKEN 8 ===
  -> acf:  appmanifest_1778820.acf
blit v0.1.0: starting copy /.../steamapps/appmanifest_1778820.acf -> /.../steamapps/appmanifest_1778820.acf
Copy complete: 0 files, 0 B in 88.37µs
  -> data: common/TEKKEN 8
blit v0.1.0: starting copy /.../steamapps/common/TEKKEN 8 -> /.../steamapps/common/TEKKEN 8
Copy complete: 490 files, 126.39 GiB in 77.02s
```

126 GiB of game assets copied fine (directory source). The 1.2 KB `.acf` Steam manifest was silently dropped (file source), leaving Steam unable to recognize the installed game until Steam itself regenerated the manifest via a verification pass.

## Root cause

Three layers of code cooperate to swallow the file source without anyone noticing.

### Layer 1 — `FileEnumerator` skips the walker's depth-0 entry unconditionally

`crates/blit-core/src/enumeration.rs:73-161`, specifically lines 100-102:

```rust
while let Some(next) = walker.next() {
    let entry = /* ... */;
    let path = entry.path();

    if entry.depth() == 0 {
        continue;                    // <-- swallows the single-file root
    }

    if entry.file_type().is_dir() {
        /* ... visit Directory ... */
    } else if entry.file_type().is_file() {
        /* ... visit File ... */
    }
    // ...
}
```

When called with `root = /some/file.txt`, `WalkDir` yields exactly one entry — the file itself, at `depth() == 0` — and the enumerator skips it. The visitor closure never fires.

Interestingly, the sibling function `relative_path` (lines 164-170) already handles the file-root case and returns `PathBuf::from(".")`, so someone at least *thought* about it, but the enumeration loop shorts out before that code is reached.

### Layer 2 — Fast-path treats an empty enumeration as NoWork

`crates/blit-core/src/orchestrator/fast_path.rs:73-128`:

```rust
let scan_result = enumerator.enumerate_local_streaming(src_root, |entry| {
    /* collect files into the `files` vec */
    Ok(())
});
/* ... */
if files.is_empty() {
    return Ok(FastPathOutcome::fast_path(FastPathDecision::NoWork));
}
```

Because Layer 1 returned no entries, `files` is empty, and the fast-path reports `NoWork`. No distinction is drawn between "source is a file and was skipped" and "source is an empty directory" and "source is a directory where every file is already up to date at the destination".

### Layer 3 — Orchestrator has no separate single-file code path

`crates/blit-core/src/orchestrator/orchestrator.rs:36-266` treats `src_root` as a generic "root" with no `is_file()` branch. Nothing upstream of the enumerator intercepts file sources. `execute_local_mirror` is named for mirrors but is also the entry point for `copy`, and neither mode has file-source handling.

So the call chain for `blit copy /tmp/repro/src/file.txt /tmp/repro/dst/` is:

1. CLI resolves destination to `/tmp/repro/dst/file.txt` (correct — the recent rsync fix).
2. `execute_local_mirror(src_root = .../file.txt, dest_root = .../dst/file.txt, ...)`.
3. Creates `dest_root.parent()` (i.e. `.../dst`), which already exists.
4. Calls `maybe_select_fast_path`.
5. Calls `enumerate_local_streaming(.../file.txt, visit)`.
6. `WalkDir` yields `.../file.txt` at depth 0; enumerator skips it.
7. Visitor never fires; `files` empty.
8. Fast-path returns `NoWork`.
9. Orchestrator prints `Copy complete: 0 files, 0 B`, returns success.

## Impact

- **Silent data loss.** No error, no warning, exit 0, `stderr` is empty.
- **Cross-transport.** Remote-pull and remote-push paths share the same orchestrator/enumerator for the local side and very likely exhibit the same symptom for file sources — not verified in this bug, but should be audited.
- **Tooling blind to the failure.** Scripts that check `$?`, `$LASTEXITCODE`, or the `--json` summary will see success. The `--json` output will presumably report `files_transferred: 0` — indistinguishable from a legitimately up-to-date directory copy.
- **Hits every "copy a single config/manifest/key" case.** ACF files, `.env` files, SSH keys, single binaries — anything that's a lone file. These are common.

## Proposed fix

The minimal, localized fix is at **Layer 1**: have the enumerator emit the root entry when the root is itself a file. Something like:

```rust
// crates/blit-core/src/enumeration.rs, inside enumerate_local_streaming loop
if entry.depth() == 0 {
    if entry.file_type().is_file() {
        let metadata = entry
            .metadata()
            .with_context(|| format!("stat file {}", path.display()))?;
        if filter.allows_file(path, metadata.len()) {
            let size = metadata.len();
            visit(EnumeratedEntry {
                absolute_path: path.to_path_buf(),
                relative_path: relative_path(root, path), // already returns "."
                metadata,
                kind: EntryKind::File { size },
            })?;
        }
    }
    // Symlink-at-root: handle analogously if include_symlinks.
    continue;
}
```

This preserves the existing `continue` for directory roots (which is needed to avoid emitting the root dir as a child of itself) and adds the file-root case.

With this change, the downstream code largely "just works":
- `fast_path.rs` will collect one file, trip `FastPathDecision::Tiny` (or `Huge` for >=1 GiB single files), and invoke `copy_paths_blocking`/`copy_large_blocking`.
- `relative_path` returns `"."` for the root file, so `copy_paths_blocking(src_root, dest_root, &[PathBuf::from(".")], ...)` would need to interpret `"."` as "the root itself" — **this needs verification**. If it doesn't, adjust `relative_path` to return the file's basename instead, or have the file-root branch emit an empty relative path and teach the downstream copy to use `src_root` / `dest_root` directly.

**Recommended belt-and-suspenders:**

1. Add an explicit file-source branch in `execute_local_mirror` that bypasses the enumerator entirely and does a direct `copy_large_blocking(src_root, dest_root, &PathBuf::new(), ...)` (or equivalent). Cleaner than threading the "root-is-file" special case through the whole pipeline.
2. Change `FastPathDecision::NoWork` to distinguish "source enumerated but nothing to copy (up-to-date)" from "source yielded no entries at all" — the latter should bail loudly unless the source is a known-empty directory.
3. In the CLI summary, when the decision is `NoWork` *and* the run was not preceded by successful journal-skip or checksum-skip, print a `• Source had no files to consider.` line. Users reading `0 files` currently cannot tell whether that means "up to date" or "I did nothing".

## Tests to add

Under `crates/blit-core/src/enumeration.rs` `#[cfg(test)]` module:

1. `enumerate_file_root_emits_file` — `enumerate_local_streaming("/tmp/x/file.txt", v)` visits one `EntryKind::File`.
2. `enumerate_file_root_relative_path_is_dot_or_basename` — pin the contract.
3. `enumerate_empty_dir_emits_nothing` — sanity.
4. `enumerate_dir_root_does_not_emit_self` — ensure the existing behavior doesn't regress.

Under `crates/blit-cli/tests/` (new `single_file_copy.rs`):

5. `copy_single_file_to_dir` — `blit copy src/file.txt dst/` creates `dst/file.txt` with matching bytes.
6. `copy_single_file_rename` — `blit copy src/file.txt dst/renamed.txt` creates `dst/renamed.txt`.
7. `copy_single_file_large` — source ≥ 1 GiB to exercise the `Huge` fast-path.
8. `copy_single_file_remote_pull` — same matrix over a remote source.
9. `copy_single_file_remote_push` — same matrix over a remote destination.
10. `copy_single_file_exit_code_on_missing_source` — `blit copy /does/not/exist /tmp/x` exits non-zero with a useful error (today it will, via the `src_root.exists()` check in `execute_local_mirror`, but pin it).
11. `copy_single_file_idempotent` — running the same copy twice leaves dest correct; second run can be a legitimate `NoWork` via skip_unchanged.

## Related code / cross-references

- **Offending code:**
  - `crates/blit-core/src/enumeration.rs:100-102` — the unconditional `continue` at depth 0.
  - `crates/blit-core/src/orchestrator/fast_path.rs:126-128` — `files.is_empty() → NoWork` with no root-type check.
  - `crates/blit-core/src/orchestrator/orchestrator.rs:36-52` — no `src_root.is_file()` branch before entering the planner/fast-path.
- **Related fix just landed:** `2a72eb6 fix: implement rsync-style destination semantics uniformly` — that fix is correct and required; it made the destination resolution right. This bug has existed independently of that commit and likely predates it.
- **Docs that may need a note:** `docs/cli/blit.1.md` does not currently clarify that `copy` accepts file or directory sources; once fixed, the man page should state that explicitly.

## Report provenance

Surfaced while debugging `~/dev/Move-SteamGame.ps1/migrate_games.sh` — a Steam-library migration tool that issues two `blit copy` calls per game: one for the `.acf` manifest (a file) and one for the game's data directory. The directory copy worked (126 GiB in 77 s); the `.acf` copy silently no-op'd. Reproducer above isolates the behavior to a 12-byte file, so it is not size-, path-, or filesystem-specific.
