# Handoff: Remote pull single-file bug + audit follow-ups

**Date:** 2026-04-23 (paused for the night)
**Branch:** master, uncommitted WIP on 5 files
**Originated from:** validating feedback on the local single-file fix in `6cf07bf`

## Context

The user fixed the local single-file silent-noop in `6cf07bf` (see `docs/bugs/single-file-source-silent-noop.md`). User then asked me to audit three follow-ups:

1. Remote-pull audit — does the daemon-side enumeration have the same depth-0 skip?
2. NoWork diagnostic — distinguish "source yielded nothing" from "up-to-date"?
3. Idempotent test — strengthen the existing test?

All three were valid concerns. The audit uncovered a **much bigger** bug than single-file remote pulls: **any remote pull with a non-empty subpath double-nests** (files land at `dst/X/X/...` instead of `dst/X/...`). Tests passed because they all used `/test/` (empty subpath).

## What's fixed and verified working

Directory remote pulls — previously broken for any subpath:

```bash
# dir pull, no trailing slash → nest (correct)
blit copy server:/m/gamedir /tmp/dst/
# → /tmp/dst/gamedir/a.txt, /tmp/dst/gamedir/subdir/b.txt  ✓

# dir pull, trailing slash → merge (correct)
blit copy server:/m/gamedir/ /tmp/dst/
# → /tmp/dst/a.txt, /tmp/dst/subdir/b.txt  ✓
```

### Root cause (directory case)

The daemon prefixed header.relative_path with the requested subpath AND the CLI resolver appended the basename to dest. Client did `dest_root.join(header.rel)` → double nest.

### Fix applied (directory case — WORKING)

The daemon now sends header.relative_path RELATIVE TO the enumeration root (not prefixed with the requested subpath). The daemon's `FsTransferSource` is constructed with the enumeration root (`module.path.join(requested)`) not `module.path`, so the physical read `source_root.join(header.rel)` still works.

Files changed:
- `crates/blit-daemon/src/service/pull.rs` —
  - `collect_pull_entries_with_checksums`: header.relative_path = entry.relative_path (not requested-prefixed); PullEntry.relative_path = physical (requested + entry) for reads
  - `enumerate_to_channel`: same split
  - `stream_pull_non_streaming`: takes new `source_root: PathBuf` param, uses for `plan_transfer_payloads` and `accept_pull_data_connection`
  - `accept_pull_data_connection` / `accept_and_wrap_sinks`: rename `module_root` → `source_root`
  - `accept_pull_data_connection_streaming`: same rename
  - `stream_pull_streaming`: passes `root` (enumeration root) to accept_pull_data_connection_streaming AND plan_transfer_payloads
- `crates/blit-daemon/src/service/pull_sync.rs` —
  - `stream_via_data_plane`: new `source_root: &Path` param
  - Caller passes `&root` instead of `&module`
  - `FsTransferSource::new(source_root)` not `module.path`
  - `plan_transfer_payloads(headers, source_root, ...)` not `&module.path`
- `crates/blit-core/src/transfer_plan.rs` — `use_tar` requires `small_count >= 2` (single small files shouldn't become tar shards, especially problematic when their relative_path is empty)

## What's still broken (needs finishing tomorrow)

**Remote pull of single files** — error: `writing transfer terminator: Broken pipe (os error 32)`

### Why it's broken

Single-file pulls have `header.relative_path = ""` (empty). Multiple spots in the client and daemon use `dest_root.join("")` or `source_root.join("")` which preserves a trailing slash in the resulting PathBuf on Unix. Then `File::create`/`File::open` on that path fails with ENOTDIR.

### Fixes already applied (committed in WIP state)

1. `crates/blit-core/src/remote/pull.rs::sanitize_relative_path` — allows empty paths (returns `PathBuf::new()`) instead of bailing
2. `crates/blit-core/src/remote/pull.rs::pull()` and `::pull_sync()` — create `dest_root.parent()` instead of `dest_root` itself (so single-file target paths don't get mkdir'd)
3. `crates/blit-core/src/remote/transfer/source.rs::FsTransferSource::open_file` — uses `self.root.clone()` directly when `header.relative_path.is_empty()`
4. `crates/blit-core/src/remote/pull.rs` — added `resolve_pull_dest(dest_root, relative_path)` helper that returns `dest_root` directly when relative_path is empty; applied to `handle_file_record` only

### Fixes still needed

**A. Apply `resolve_pull_dest` helper to 6 more sites in pull.rs** — the ones that still do `dest_root.join(&relative_path)` directly:
```
167:                    let dest_path = dest_root.join(&relative_path);
468:                    let dest_path = dest_root.join(&relative_path);
535:                    let dest_path = dest_root.join(&relative_path);
580:                    let dest_path = dest_root.join(&relative_path);
1016:    let dest_path = dest_root.join(&relative_path);
1072:    let dest_path = dest_root.join(&relative_path);
```

Planned approach: sed-replace to use the helper:
```bash
sed -i 's|let dest_path = dest_root\.join(&relative_path);|let dest_path = resolve_pull_dest(dest_root, \&relative_path);|g' \
  /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs
```

**B. Test regression to investigate**: `test_remote_move_remote_to_local` started failing. Likely needs similar empty-path handling somewhere. Check where it fails after the other fixes land.

**C. Verify all 4 cases end-to-end** once (A) and (B) land:
```
# Case 1: pull file → container
blit copy server:/m/single.txt /tmp/dst/  →  /tmp/dst/single.txt

# Case 2: pull dir (no slash) → nest  ✓ ALREADY WORKING
# Case 3: pull dir/ (trailing slash) → merge  ✓ ALREADY WORKING

# Case 4: pull file → rename
blit copy server:/m/single.txt /tmp/dst/renamed.txt  →  /tmp/dst/renamed.txt
```

## Additional follow-ups not yet started

The user's feedback included three points; I'm tracking them as tasks #16-19:

- **#16 (in progress)** Remote pull double-nest fix — above
- **#17 (pending)** NoWork diagnostic — distinguish "source yielded nothing" from "up-to-date" in the CLI summary so the next class-of-bug silent failure gets caught immediately. Implementation sketch: add a discriminator to `FastPathDecision::NoWork` (or a new variant `NoEntries`), and in the CLI summary print "Source had no files to consider" vs "Up to date" accordingly. Also include the distinction in `--json` output.
- **#18 (pending)** Strengthen `single_file_copy_idempotent` test — pin that first run reports "1 files" AND second run reports "0 files" separately, so a regression to silent no-op can't pass a loose assertion. Current test only pins file contents.
- **#19 (pending)** File separate bug: **single-file remote PUSH** crashes with `opening /tmp/src/loose.txt/ during payload planning: Not a directory (os error 20)`. Different code path from pull (check_availability in `filter_readable_headers`). Different root cause — source_root = the file itself, and the push code assumes source_root is a directory. Write a bug report, defer the fix.

## Test status (end of session)

- `cargo check --workspace` — CLEAN ✓
- `cargo test --workspace` — 1 FAILURE: `test_remote_move_remote_to_local` in `crates/blit-cli/tests/remote_move.rs`. All other suites pass (86+ core, 10 admin, 21 utils, 4 parity, etc.)
- `cargo clippy --workspace --all-targets -- -D warnings` — not re-run since last round of changes; may have warnings from dead prints or unused imports

## Files in uncommitted WIP state

```
 M crates/blit-core/src/remote/pull.rs
 M crates/blit-core/src/remote/transfer/source.rs
 M crates/blit-core/src/transfer_plan.rs
 M crates/blit-daemon/src/service/pull.rs
 M crates/blit-daemon/src/service/pull_sync.rs
```

Recommendation for tomorrow: start by committing whatever is in a coherent state (directory-pull fix is complete), then tackle (A) → (B) → verification → commit.

## Verification harness

This worked well to iterate. Reuse it tomorrow:

```bash
rm -rf /tmp/blit-remote-test && mkdir -p /tmp/blit-remote-test/{module/gamedir/subdir,dst}
echo "a-content" > /tmp/blit-remote-test/module/gamedir/a.txt
echo "b-content" > /tmp/blit-remote-test/module/gamedir/subdir/b.txt
echo "solo-file-content" > /tmp/blit-remote-test/module/single.txt

cat > /tmp/blit-remote-test/blitd.toml <<EOF
[daemon]
bind = "127.0.0.1"
port = 19031
no_mdns = true

[[module]]
name = "test"
path = "/tmp/blit-remote-test/module"
EOF

./target/release/blit-daemon --config /tmp/blit-remote-test/blitd.toml > /tmp/blit-remote-test/daemon.log 2>&1 &
DAEMON_PID=$!
sleep 1

# Run each of the four cases...

kill $DAEMON_PID 2>/dev/null; wait $DAEMON_PID 2>/dev/null
```
