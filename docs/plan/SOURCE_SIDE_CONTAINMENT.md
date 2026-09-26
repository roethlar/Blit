# Source-Side Containment — a file the source cannot deliver is skipped, not fatal

**Status**: Draft — owner ordered the plan and a codex review loop to
consensus (2026-09-25); no code until `Active`. Open rulings D1–D4 below.
**Created**: 2026-09-25
**Supersedes**: `docs/plan/PER_FILE_ERROR_CONTAINMENT.md` §Non-goals
"Send-side source read stays fatal" (the deferred wire skip signal lands
here); `CHANGELOG.md` 0.1.2 Known limitation "Non-UTF-8 source filenames …
On REMOTE transfers it is not contained" (closed by ssc-2/ssc-5).
**Decision ref**: pending (D-2026-07-09-1 supplies the governing principle;
D-2026-08-01-2 shipped the destination half this plan completes)

## Goal

A transfer session survives every per-file failure that originates at the
SOURCE, exactly as it already survives destination write failures: a file
the source cannot open, cannot read, or whose size no longer matches its
manifest header is skipped, recorded through the existing `record_failure`
chokepoint, named in the end-of-run failure block, counted in
`files_failed`, and the run exits 2. The rest of the manifest lands. One
changing file can never corrupt the tar shard carrying its neighbours.

Motivating failure (2026-09-25, local mirror `C:\Users\michael\ → C:\temp\`,
cargo-installed 0.1.2 on Windows): OneDrive's embedded browser was
rewriting `AppData/Local/Microsoft/OneDrive/EBWebView/GPUPersistentCache/…/
cache.db-wal` and `cache.journal` while the mirror ran. The shard packer
wrote each member's tar header with the scan-time size, then streamed the
file's *current* bytes; the tar crate copies to EOF and pads on the bytes
it actually copied, so a member that grew after the scan pushed its extra
bytes into the next header slot. The destination parsed those bytes as a
tar header and the whole mirror died with
`session INTERNAL: writing payload: tar shard entry: numeric field was not a
number: when getting cksum for EBFGONED`.

Governing principle: D-2026-07-09-1 — "FAST, SIMPLE, RELIABLE file
transfer. if we abort the whole thing when we could have fixed or surfaced
a single error, we are violating all of those."

## Non-goals

- No consistent-snapshot copying (VSS, LVM, APFS snapshots). A file that
  changes during the run is reported and skipped (D1); blit never claims
  to have captured a consistent image of a live file.
- No in-session retry. Convergence-on-re-run stays the reliability model
  (D-2026-07-09-1 Q2); the failure block's re-run hint already says so.
- No new CLI flag or option of any kind (D-2026-08-01-1, SIMPLE). No
  "ignore changing files" mode.
- Session-fatal classes stay fatal: transport death, protocol violations,
  path-safety/containment violations, destination-root unavailability,
  volume-level write failures (`failure_is_containable`,
  `remote/transfer/sink.rs:487-493`), the **scan-time** incomplete-scan
  refusals for mirror and move at ManifestComplete
  (`transfer_session/mod.rs:4209-4229`). Tar-shard *structural* parse
  failures at the destination stay fatal too — this plan makes the source
  incapable of producing one from a changing file, it does not weaken the
  destination's refusal.
- Resume block diffing (`remote/transfer/resume_diff.rs:115`) is untouched:
  that EOF is the destination reading its own partial, a different class.
- Windows named-stream size drift (`windows_metadata.rs:802`) is untouched
  in this plan; it already errors per file and is contained by the
  destination-side classification.
- No version compatibility: the contract bump (Constraints) means a
  pre-plan peer refuses at session open, by design (D-2026-08-18-2).

## Constraints

- **Contract bump.** Both carriers gain an in-band skip record, so
  `CONTRACT_VERSION` (`crates/blit-core/src/transfer_session/mod.rs:96`,
  currently 6) becomes **7** in the same commit as the first wire change
  (ssc-1), with the history comment at `:88-95` extended. The mDNS
  `contract` TXT property (`mdns.rs:157`) follows automatically.
  `DelegatedPullSummary` needs no change: no `TransferSummary` field is
  added.
- **One chokepoint.** Every source-side skip reaches the destination's
  `contained_failures` accumulator (`mod.rs:4086`) through
  `SinkOutcome::record_failure` (`sink.rs:257-280`) — the same path the
  destination-side failures take, so the cap (64 listed / total counted),
  the CLI block, JSON fields, exit code 2 and the move source-delete gate
  need no parallel implementation.
- **Nothing lands under a false header.** A record whose bytes do not
  match its `FileHeader.size` is never finalised as that file: either the
  source detects the drift before announcing the record and skips it, or
  (ssc-4) it completes the record to length and retracts it.
- **Reason strings are prefixed by side** so a reader of the failure block
  can tell where the file failed: reasons recorded by the source start with
  `source:` (e.g. `source: changed size during transfer (manifest 4096
  bytes, now 8192)`, `source: cannot open: Access is denied. (os error 5)`).
  Destination reasons are unchanged. `FileFailure` keeps its two string
  fields (`blit.proto:1090-1093`); no kind enum.
- The mirror's deletion safety rests on **enumeration completeness only**:
  a file that enumerated but cannot be opened at payload time is in the
  manifest, so its destination counterpart is never extraneous. Skips
  therefore never gate mirror deletion (D3 removes the apply-time refusal
  that says otherwise).
- Rust edition 2021, rustfmt, `-D warnings` on native and Linux-cross
  clippy; tests deterministic and async-aware; verification per
  `.agents/repo-guidance.md` §Verification. Guards for behaviour that can
  only go red on Windows (sharing violation) live in `cfg(windows)` tests
  AND are covered by an ungated fault-injecting `TransferSource` so the
  containment itself is proven on every platform.
- Review track (owner, 2026-09-25): `openreview codex` on the plan,
  iterated until consensus; landed slices then follow D-2026-07-31-3's
  standing codex `codereview`.

## Acceptance criteria

- [ ] **A1 (the field failure, both carriers + local):** a tar shard whose
  member grew after the scan lands every other member intact, reports the
  grown member as `source: changed size during transfer (…)`, and the run
  exits 2. Red today with the exact `numeric field was not a number …
  cksum` structural fault (pin the message substring in the red proof).
- [ ] **A2:** a shard member that shrank or vanished after the scan is
  skipped and reported; shard-mates land.
- [ ] **A3:** a single-file (non-shard) payload whose source open fails
  (locked / denied / vanished) is skipped and reported on the in-stream
  carrier, the data plane, and the local route; nothing is announced for it
  on the wire (no `FileBegin`, no FILE record tag).
- [ ] **A4:** a single-file payload whose size changed between scan and
  open is skipped and reported before announcement, on both carriers.
- [ ] **A5:** the destination's "SourceDone with N needed file(s) never
  delivered" check (`mod.rs:4778-4786`) still fires for a need that is
  neither delivered nor skipped (protocol-violation pin unchanged), and a
  skip for a path that was never needed is a protocol violation.
- [ ] **A6:** `move` refuses source deletion when a file was skipped
  (existing gate, `crates/blit-core/src/transfers/failures.rs:56-78`,
  reads `files_failed`; pin with a source-side skip).
- [ ] **A7:** mirror under a source-side skip deletes extraneous
  destination entries and keeps the skipped file's destination counterpart
  (parallel to `mirror_deletes_extraneous_entries_under_a_contained_failure`).
- [ ] **A8 (D3):** a source file that enumerated cleanly and then became
  unopenable before apply on the LOCAL route is a `files_failed` entry with
  exit 2; the mirror completes and deletes extraneous entries.
  `mirror_refuses_when_availability_drops_after_clean_scan`
  (`transfer_session/local.rs:1222`) flips meaning and is renamed.
- [ ] **A9 (ssc-4, D2):** a read error or short EOF *after* a single-file
  record was announced is retracted: the destination has no file at that
  path when the run ends, the failure is reported, the session completes.
  Red today with `'<path>' hit EOF with N bytes still promised`
  session-fatal (`mod.rs:3090-3101`, `data_plane.rs:566-570`).
- [ ] **A10 (audit-18):** a source file whose name is not valid UTF-8 is
  reported as `source: filename is not valid UTF-8 (rename it to transfer)`
  with exit 2 on every route; the remote session no longer aborts. The
  0.1.2 CHANGELOG known-limitation text is retired in the Unreleased notes.
- [ ] **A11:** the CLI failure block header no longer says "could not be
  written" (`crates/blit-cli/src/transfers/failures.rs:95`); the move gate
  message (`blit-core/src/transfers/failures.rs`) no longer says "could not
  be written and remain un-landed"; both read correctly for a source-side
  failure. `docs/TRANSFER_SESSION.md` frame table (`:157-194`), the v6
  failure section (`:196-250`) and Errors (`:464-520`) describe v7.
- [ ] Every new guard is mutation-proven red/green (revert the fix, watch
  it fail with the pre-plan message, restore). Test count never drops;
  the one flipped test (A8) is renamed, not removed.
- [ ] Gate green on macOS + Linux-cross clippy; CI green on all three OSes
  at the slice head before the next slice starts (Windows is the platform
  that motivated the plan).

## Design

### D-A. The skip record (ssc-1)

One new in-band record per carrier, both meaning "the source will not
deliver this needed file; here is why":

- **In-stream:** `Frame::FileSkipped(FileFailure)` — new oneof field 21 in
  `TransferFrame` (`crates/blit-core/proto/blit.proto:1153-1176`), reusing
  the existing `FileFailure{relative_path, reason}` message. Add the
  variant to `frame_name` (`mod.rs:624-648`).
- **Data plane:** `DATA_PLANE_RECORD_SKIP = 4` (`remote/transfer/
  data_plane.rs:17-21`): `u32 path_len, path bytes, u32 reason_len, reason
  bytes`, bounded by `MAX_FAILURE_PATH_BYTES` / `MAX_FAILURE_REASON_BYTES`
  (`sink.rs:40-48`). The receiver loop (`pipeline.rs:1255-1431`) reads it
  sequentially like every other tag, so it is ordered against the record
  bodies around it.

Destination handling, both carriers, one function
(`claim_skip(outstanding, granted_headers, contained_failures, path,
reason)`):

1. `outstanding.remove(path)` succeeded → `contained_failures.record_failure
   (path, reason)`. The need is closed; the never-delivered check at
   `mod.rs:4778-4786` no longer counts it.
2. Path absent from `outstanding` **and** absent from `granted_headers` →
   `PROTOCOL_VIOLATION` ("skip for '…' which is not on the need list"),
   mirroring `mod.rs:4354-4363`.
3. Path in `granted_headers` but not `outstanding` (already announced or
   delivered) → ssc-4 retraction (D-D); until ssc-4 lands this is a
   protocol violation.

The `file_failed` predicate (`sink.rs:298-303`) already suppresses the
completion accounting for the path (`mod.rs:4379`, `pipeline.rs:1266`),
because `record_failure` inserts into `failed_paths`.

Source emission points:

- In-stream single file (`send_payload_records`, `mod.rs:3026-3155`):
  **open (and re-stat) before `FileBegin`** — today `FileBegin` goes out at
  `:3074` and `open_file` runs at `:3079`. Open failure → send
  `FileSkipped`, `continue`. Size mismatch (D-C) → same.
- Data plane single file (`DataPlaneSession::send_file`,
  `data_plane.rs:464-475`): the open already precedes the record tag;
  replace the `?` with a SKIP record write. Size mismatch → SKIP.
- Tar shards on every route: the `skipped` list on `PreparedPayload`
  (D-B) is emitted before the shard: in-stream at `mod.rs:3116` (one
  `FileSkipped` per entry, then `TarShardHeader` listing only packed
  members), data plane in `DataPlaneSink::write_payload` (`sink.rs:
  1927-1942`) as SKIP records before the shard record, local route inside
  `FsTransferSink::write_payload` (`sink.rs:1048-1093`) as
  `record_failure` calls before extraction.

The first user in ssc-1 is the single-file open failure (A3) because it
needs no packer change; a fault-injecting `TransferSource` whose
`open_file` fails for one path (pattern: `TruncatedReadSource`,
`crates/blit-core/tests/transfer_session_roles.rs:957-1000`) is the ungated
guard, and a `cfg(windows)` guard opens the file with
`OpenOptions::share_mode(0)` for the real sharing violation.

### D-B. Shard packer fidelity (ssc-2)

`build_tar_shard` (`remote/transfer/payload.rs:296-335`) currently trusts
`header.size` for the tar header and streams the file to EOF. New
contract: **a member is appended only from a buffer that is exactly
`header.size` bytes long.**

```
for header in headers:
    file = File::open(full_path)            -- Err → skipped.push(source: cannot open: {err}) ; continue
    buf  = Vec::with_capacity(size)
    (&mut file).take(size).read_to_end(&mut buf)   -- Err → skipped.push(source: read error: {err}); continue
    if buf.len() != size                     -- shrank/vanished mid-read
        skipped.push("source: changed size during transfer (manifest {size} bytes, now {buf.len()})"); continue
    probe: file.read(&mut [0u8;1]) == 1      -- grew
        skipped.push("source: changed size during transfer (manifest {size} bytes, now {size + …})"); continue
    tar_header.set_size(size); builder.append_data(&mut tar_header, rel, &buf[..])
    packed.push(header)
return TarShardBuild { data, headers: packed, skipped }
```

`file.metadata().len()` is the cheap pre-check before the read and gives
the exact "now" size for the message; the take+probe is the race-proof
check that makes the guarantee hold even if the file changes between stat
and read. Shards are already whole-in-memory (`Vec<u8>`), so the buffer
adds no new memory class. A shard whose every member was skipped produces
no shard payload at all — only skips.

`PreparedPayload::TarShard` (`payload.rs:137-140`) gains
`skipped: Vec<FileFailure>`; `prepare_payload` (`payload.rs:65-75`) passes
it through; `bound_in_stream_tar_headers` (`mod.rs:2986`) splits on the
packed headers only. `safe_extract_tar_shard` (`tar_safety.rs:111-231`)
is unchanged — the `TarShardHeader.files` list it checks against is the
packed list, so `require_exact_headers` still holds.

Red proof for A1 (ungated, local route or in-stream): write two files,
capture headers, append bytes to the first, call `prepare_payload` and
extract with `safe_extract_tar_shard` → today `tar shard entry: numeric
field was not a number` (or a size-mismatch bail, depending on alignment);
after ssc-2 the second file lands, the first is in `skipped`.

### D-C. Single-file size drift (ssc-3, part 1)

Before announcing a single-file record on either carrier, the source
compares `file.metadata()?.len()` (from the opened handle, not the path)
against `header.size`; a mismatch is a skip with the same
`source: changed size during transfer (…)` reason. Then the body is read
through `.take(header.size)` so growth after the check is harmless on
both carriers (today the in-stream loop already bounds by `remaining`;
the data-plane double-buffered loop at `data_plane.rs:483-535` reads
`file_size` bytes — confirm and pin). A shrink after the check is the
mid-record case (D-D).

The local route's `File` payload is copied by the destination sink from
the source path (`sink.rs:1300-1353`, contained via `per_file_failure`);
its size-drift semantics are "copy the current bytes, stamp the manifest
mtime", which converges on re-run because the source's size/mtime then
differ from the destination's. Left as is; documented in the plan record.

### D-D. Mid-record retraction (ssc-4, owner ruling D2)

After a single-file record is announced, a read error or a short read
(`mod.rs:3086-3101`, `data_plane.rs:566-570`, `:619-623`) is today
session-fatal. Under ssc-4 the source **completes the record to
`header.size` with zero bytes, then emits the skip record for the same
path** (`FileSkipped` on the control stream; SKIP tag on the data plane
immediately after the body). Destination rule 3 above: a skip for a path
in `granted_headers` but not in `outstanding` is a retraction — the file
was written by this session, so the destination unlinks
`safe_join_contained(dst_root, path)` (`path_safety.rs`, the same
chokepoint as `sink.rs:1325-1329`) and calls `record_failure`. Ordering
is guaranteed on both carriers because record bodies are consumed
sequentially before the next frame/tag is read (`receive_file_record`,
`mod.rs:6063-6121`; `pipeline.rs:1255-1261`). Deleting a file the session
itself just wrote is within the session's existing write authority; the
prior destination copy was already overwritten in place (D-2026-07-09-1
Q2), which is no worse than today's abort mid-write and now reported.

If D2 rules "keep fatal", ssc-4 is dropped, rule 3 stays a protocol
violation, and A9 is struck; the plan still closes A1–A8/A10–A11.

### D-E. Local route: availability pre-check retired (ssc-3, part 2, D3)

`LocalApply::plan_chunk` (`transfer_session/local.rs:537-574`) calls
`check_availability` → `filter_readable_headers` (`remote/transfer/
source.rs:638-681`), which pre-opens every needed file: NotFound and
PermissionDenied go to the scan's `unreadable` list and any other error is
fatal. Then `mod.rs:4736-4748` refuses the whole mirror after the apply
joined if that list is non-empty ("could not be read during the
transfer"). With D-A/D-B in place the pre-check is redundant (the packer
and the sink contain the same failures per file) and its refusal is
wrong on principle (Constraints: deletion safety is enumeration
completeness). ssc-3 deletes `check_availability`/`filter_readable_headers`
and the apply-time refusal; the `TransferSource` trait loses the method
(`source.rs:178`); the scan-time refusal at ManifestComplete and
`LocalMirrorSummary.unreadable_paths` for scan-time entries stay. This
also makes the LOCAL non-UTF-8 case a `files_failed` entry (today it is
an `unreadable_paths` entry, and the 0.1.2 CHANGELOG's "exits 2" claim was
not what the code does — fact sheet item 9).

### D-F. Reason for non-UTF-8 names (ssc-5, audit-18)

`relative_path_to_posix` (`path_posix.rs:36-44`) is lossy; the header
carries U+FFFD and the source cannot re-open the file by that name. In
the skip paths above, when the open fails with NotFound and the relative
path contains U+FFFD, the reason is `source: filename is not valid UTF-8
(rename it to transfer)` instead of the raw ENOENT. No proto change (the
field stays `string`); the honest fix for the name itself (bytes on the
wire) is a separate plan if ever wanted.

### D-G. Words and docs (ssc-5)

- `failure_block_styled` (`blit-cli/src/transfers/failures.rs:84-120`):
  header becomes "{n} file(s) did not land at the destination:"; the
  trailer keeps the re-run hint.
- `refuse_source_delete_on_failures` message: "… did not land at the
  destination" (drop "could not be written").
- `docs/TRANSFER_SESSION.md`: frame table gains `FileSkipped`; v7 section
  describes skip + retraction semantics and the reason prefix; Errors
  section drops "send-side source read" from the fatal list.
- `CHANGELOG.md` Unreleased: reliability entry + retire the 0.1.2 non-UTF-8
  remote caveat; `docs/plan/RELEASE_1_0.md` G3 lists this plan as the
  fix-now item (owner ruling D4); `docs/plan/PER_FILE_ERROR_CONTAINMENT.md`
  §Non-goals send-side line gets a pointer here.

### Risks

- **Protocol strictness.** A skip for an un-needed path must stay a
  violation, or a buggy source could silently drop files. A5 pins it.
- **Data-plane framing.** A malformed SKIP record (over-long path/reason)
  must bail like an unknown tag, never be skipped past; bound reads by
  the existing caps and pin with a corrupt-record test.
- **Retraction unlink (ssc-4)** is the only new destructive act; it is
  scoped to a path this session claimed and wrote, resolved through the
  containment chokepoint, and pinned by a test that plants a decoy outside
  the root.
- **Reporting cap.** Thousands of skips (a whole live profile) exceed the
  64-entry list; `files_failed_total` still counts them all and the block
  prints the elided count — existing behaviour, unchanged.

## Slices

One coherent, testable change per slice — each its own go, commit, full
gate, DEVLOG entry, CI on all three OSes before the next.

1. **ssc-1 — skip record + contract 7 (A3, A5, A6, A7).** Proto field
   21 + data-plane tag 4; `claim_skip` on the destination (rules 1–2;
   rule 3 = violation for now); in-stream open-before-`FileBegin` reorder;
   data-plane `send_file` open failure → SKIP. Guards: ungated
   fault-injecting source on both carriers + local route; `cfg(windows)`
   sharing-violation guard; never-delivered and un-needed-skip violation
   pins; move gate and mirror-extraneous pins with a source-side skip.
   Mutation proof: restore the `?` at `data_plane.rs:469-472` → the
   fatal returns. Docs: `CONTRACT_VERSION` comment; STATE.
2. **ssc-2 — shard packer fidelity (A1, A2).** `build_tar_shard` per D-B;
   `PreparedPayload::TarShard.skipped`; emission at all three consumers.
   Red proof reproduces the field message. Guards: grown member, shrunk
   member, vanished member, all-members-skipped shard, and the existing
   `tar_safety` exact-headers pins still green.
3. **ssc-3 — single-file drift pre-check + local pre-check retirement
   (A4, A8; owner D3).** D-C on both carriers; delete
   `check_availability`/`filter_readable_headers`/apply-time mirror
   refusal per D-E; rename-and-flip
   `mirror_refuses_when_availability_drops_after_clean_scan`;
   `VanishingSource` (`local.rs:1171-1210`) becomes the A8 fixture.
4. **ssc-4 — mid-record retraction (A9; owner D2).** D-D on both
   carriers; destination rule 3; decoy-outside-root pin for the unlink.
5. **ssc-5 — words, non-UTF-8 reason, docs (A10, A11).** D-F, D-G,
   CHANGELOG Unreleased, RELEASE_1_0 G3 (D4), PER_FILE_ERROR_CONTAINMENT
   pointer.

Executed order ssc-1 → ssc-2 → ssc-3 → ssc-5, with ssc-4 wherever D2
lands it (independent of ssc-3/ssc-5).

## Open questions

- **D1 — drift policy.** A file whose size changed since the scan: skip and
  report (this draft), or land the bytes actually read under a corrected
  header? Recommendation: skip. The manifest promised size+mtime; landing
  different bytes under that mtime makes the next compare call it
  converged when it is not. Skipping converges on re-run once the file is
  quiet; a never-quiet file never lands and is always reported.
  — owner
- **D2 — mid-record retraction (ssc-4).** Adopt D-D, or keep post-announce
  read failures fatal? Recommendation: adopt; it is the last way one live
  file can kill a run, and the ordering proof is already in the carriers.
  — owner
- **D3 — retire the local availability pre-check and its apply-time mirror
  refusal (D-E).** Recommendation: retire; deletion safety is the scan's
  completeness, not a file's openability. — owner
- **D4 — 1.0 gate.** Record this plan under `RELEASE_1_0.md` G3 as a
  fix-now item? Recommendation: yes; backing up a live home directory is
  the everyday workload. — owner
