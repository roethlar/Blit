# Source-Side Containment — a file the source cannot deliver is skipped, not fatal

**Status**: Draft — owner ordered the plan and a codex review loop to
consensus (2026-09-25); no code until `Active`. Open rulings D1–D4 below.
Review record: `REVIEW.md` §Plan reviews (openreview), rows
`plan-ssc-2026-09-25-r*`; r1 `acceptable_with_changes` (5 material
changes, all adopted in this revision — see §Review history).
**Created**: 2026-09-25
**Supersedes**: `docs/plan/PER_FILE_ERROR_CONTAINMENT.md` §Non-goals
"Send-side source read stays fatal" (the deferred wire skip signal lands
here); `CHANGELOG.md` 0.1.2 Known limitation "Non-UTF-8 source filenames …
On REMOTE transfers it is not contained" (closed by ssc-1/ssc-5).
**Decision ref**: pending (D-2026-07-09-1 supplies the governing principle;
D-2026-08-01-2 shipped the destination half this plan completes)

## Goal

A transfer session survives every per-file failure that originates at the
SOURCE, exactly as it already survives destination write failures: a file
the source cannot open, cannot hydrate, cannot read, or whose size no
longer matches its manifest header is skipped — before its record is
announced when possible, retracted after it when not — recorded through
the existing `record_failure` chokepoint, named in the end-of-run failure
block, counted in `files_failed`, and the run exits 2. The rest of the
manifest lands. One changing file can never corrupt the tar shard carrying
its neighbours, and nothing is ever finalised at the destination under a
header whose bytes it does not match.

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
- Lossless (byte) path identity on the wire. `FileHeader.relative_path`
  stays a proto `string`; this plan marks a lossy name so it is reported
  honestly (D-F) but does not carry the original bytes. Two source names
  that collapse to the same lossy string are a pre-existing manifest
  collision, out of scope here.
- No version compatibility: the contract bump (Constraints) means a
  pre-plan peer refuses at session open, by design (D-2026-08-18-2).

## Constraints

- **Contract bump.** Both carriers change shape (skip record, record
  terminators, resume-completion status, header flag), so
  `CONTRACT_VERSION` (`crates/blit-core/src/transfer_session/mod.rs:96`,
  currently 6) becomes **7** in ssc-1, with the history comment at
  `:88-95` extended. Every wire change in this plan lands under 7; if a
  release ships between slices the next wire slice bumps again. The mDNS
  `contract` TXT property (`mdns.rs:157`) follows automatically.
  `DelegatedPullSummary` needs no change: no `TransferSummary` field is
  added.
- **One chokepoint.** Every source-side skip or retraction reaches the
  destination's `contained_failures` accumulator (`mod.rs:4086`) through
  `SinkOutcome::record_failure` (`sink.rs:257-280`) — the same path the
  destination-side failures take, so the cap (64 listed / total counted),
  the CLI block, JSON fields, exit code 2 and the move source-delete gate
  need no parallel implementation.
- **Nothing is finalised under a false header.** A destination record is
  marked complete (mtime/attributes stamped, counted as transferred) only
  when its terminator says the source delivered exactly `header.size`
  bytes from a file that still had that size afterwards. Anything else is
  discarded by the sink and reported.
- **Explicit per-need state, one ledger.** The destination tracks each
  granted path as `Granted → Active(lane, record) → Completed | Failed`
  (D-A). Skips are accepted only in `Granted`; retractions only for the
  `Active` record on the lane they arrive on. Anything else is a protocol
  violation. There is no path-lookup heuristic and no unlink-by-path of a
  file this session did not itself write in the active record.
- **Reason strings are prefixed by side** so a reader of the failure block
  can tell where the file failed: reasons recorded by the source start with
  `source:` (e.g. `source: changed size during transfer (manifest 4096
  bytes, now 8192)`, `source: cannot open: Access is denied. (os error 5)`,
  `source: filename is not valid UTF-8 (rename it to transfer)`).
  Destination reasons are unchanged. `FileFailure` keeps its two string
  fields (`blit.proto:1090-1093`); no kind enum.
- The mirror's deletion safety rests on **enumeration completeness only**:
  a file that enumerated but cannot be delivered at payload time is in the
  manifest, so its destination counterpart is never extraneous. Skips and
  retractions never gate mirror deletion (D3 removes the apply-time
  refusal that says otherwise).
- Rust edition 2021, rustfmt, `-D warnings` on native and Linux-cross
  clippy; tests deterministic and async-aware; verification per
  `.agents/repo-guidance.md` §Verification. Guards for behaviour that can
  only go red on Windows (sharing violation, named-stream drift) live in
  `cfg(windows)` tests AND are covered by an ungated fault-injecting
  `TransferSource` so the containment itself is proven on every platform.
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
- [ ] **A3:** a single-file payload whose source open fails (locked /
  denied / vanished) is skipped before announcement on the in-stream
  carrier, the data plane, and the local route; nothing is announced for
  it on the wire (no `FileBegin`, no FILE record tag).
- [ ] **A4:** a single-file payload whose size changed between scan and
  open is skipped before announcement on both carriers.
- [ ] **A5:** the destination's "SourceDone with N needed file(s) never
  delivered" check (`mod.rs:4778-4786`) and its resume twin
  (`:4794-4802`) still fire for a need that is neither delivered, skipped,
  nor retracted; a skip for a path not in `Granted`, or a failed
  terminator on a lane with no `Active` record, is a protocol violation.
- [ ] **A6:** `move` refuses source deletion when a file was skipped or
  retracted (existing gate, `crates/blit-core/src/transfers/failures.rs:
  56-78`, reads `files_failed`; pin with a source-side skip).
- [ ] **A7:** mirror under a source-side skip deletes extraneous
  destination entries and keeps the skipped file's destination counterpart
  (parallel to `mirror_deletes_extraneous_entries_under_a_contained_failure`).
- [ ] **A8 (D3):** a source file that enumerated cleanly and then became
  unopenable before apply on the LOCAL route is a `files_failed` entry with
  exit 2; the mirror completes and deletes extraneous entries.
  `mirror_refuses_when_availability_drops_after_clean_scan`
  (`transfer_session/local.rs:1222`) flips meaning and is renamed.
- [ ] **A9 (retraction):** a read error or short EOF *after* a single-file
  record was announced, or a file whose size differs when re-checked after
  its body was sent, ends with no file at that destination path, the
  failure reported, the session complete — on both carriers. Red today
  with `'<path>' hit EOF with N bytes still promised` session-fatal
  (`mod.rs:3090-3101`, `data_plane.rs:566-570`).
- [ ] **A10 (resume):** a resume-granted file whose source open fails is
  skipped; one whose source read fails or shrinks mid-diff is closed as
  failed — the destination's partial is left in place and **not stamped**,
  the failure is reported, the session completes. Red today via
  `ResumeBlockDiff::open`/read errors (`remote/transfer/resume_diff.rs:
  66-77`, `:101-119`) propagating through `mod.rs:3226`.
- [ ] **A11 (hydration):** on Windows, a file whose metadata hydration or
  named-stream read fails or drifts after the scan
  (`remote/transfer/payload.rs:53-75`, `windows_metadata.rs:802`) is a
  per-file skip, never a pipeline failure (`pipeline.rs:472-476`);
  `cfg(windows)` guard plus an ungated guard through a fault-injecting
  hydrator.
- [ ] **A12 (local drift, D1):** a local-route single-file payload whose
  source size differs from its header after the copy is reported as
  `source: changed size during transfer (…)`, the partial destination file
  is removed, exit 2. Red today: the current bytes land under the manifest
  mtime (`sink.rs:1371-1400`).
- [ ] **A13 (non-UTF-8):** a source file whose name is not valid UTF-8 is
  reported as `source: filename is not valid UTF-8 (rename it to transfer)`
  with exit 2 on every route, without the source opening any path by the
  lossy name; the remote session no longer aborts. The 0.1.2 CHANGELOG
  known-limitation text is retired in the Unreleased notes.
- [ ] **A14 (words/docs):** the CLI failure block header no longer says
  "could not be written" (`crates/blit-cli/src/transfers/failures.rs:95`);
  the move gate message (`blit-core/src/transfers/failures.rs`) no longer
  says "could not be written and remain un-landed"; both read correctly
  for a source-side failure. `docs/TRANSFER_SESSION.md` frame table
  (`:157-194`), the v6 failure section (`:196-250`) and Errors
  (`:464-520`) describe v7.
- [ ] Every new guard is mutation-proven red/green (revert the fix, watch
  it fail with the pre-plan message, restore). Test count never drops;
  the one flipped test (A8) is renamed, not removed.
- [ ] Gate green on macOS + Linux-cross clippy; CI green on all three OSes
  at the slice head before the next slice starts (Windows is the platform
  that motivated the plan).

## Design

### D-A. Need ledger, skip record, record terminators (ssc-1)

**Ledger.** Replace the two destination sets — `OutstandingNeeds`
(`transfer_session/data_plane.rs:76`, created `mod.rs:3952`) and
`GrantedHeaders` (`data_plane.rs:80`, `mod.rs:3953`), which today are both
removed at claim (`mod.rs:4354-4367`, `data_plane.rs:2007-2011`) — with one
`NeedLedger: HashMap<String, NeedState>`:

```
enum NeedState {
    Granted { header: FileHeader, resume: bool },   // need sent, nothing announced
    Active  { header, lane: Lane, kind: File | Resume },  // record announced on `lane`
    Completed,
    Failed,
}
enum Lane { Control, DataPlane }   // in-stream control stream vs the TCP socket
```

Transitions, each a protocol violation if the current state does not
allow it:

| event (on lane L) | from | to | side effect |
|---|---|---|---|
| grant (need sent) | absent | Granted | — |
| skip record for path | Granted | Failed | `record_failure(path, reason)` |
| `FileBegin` / FILE tag / shard header member | Granted | Active(L) (shard members go straight to Completed on shard success, Failed on per-member containment as today) | — |
| terminator ok | Active(L), same L | Completed | sink finalises (stamp, count) |
| terminator failed(reason) | Active(L), same L | Failed | sink **discards** the record's write; `record_failure` |
| resume `BlockComplete` ok | Active(L, Resume) | Completed | stamp as today |
| resume `BlockComplete` failed(reason) | Active(L, Resume) | Failed | leave partial unstamped; `record_failure` |
| `SourceDone` | any Granted/Active remaining | — | violation, as today (`mod.rs:4778-4802`) |

A lane processes records sequentially (`receive_file_record`,
`mod.rs:6063-6121`; `pipeline.rs:1255-1261` reads the body through
`take(file_size)` before the next tag), so "the Active record on this
lane" is unambiguous: at most one per lane. The data-plane sink's
`claim`/`claim_shard` (`data_plane.rs:2071-2115`) become ledger
transitions; the in-stream arms at `mod.rs:4354` and `:4514-4548` likewise.

**Skip record** ("the source will not deliver this granted file"):

- In-stream: `Frame::FileSkipped(FileFailure)` — new oneof field 21 in
  `TransferFrame` (`crates/blit-core/proto/blit.proto:1153-1176`); add to
  `frame_name` (`mod.rs:624-648`).
- Data plane: `DATA_PLANE_RECORD_SKIP = 4` (`data_plane.rs:17-21`):
  `u32 path_len, path, u32 reason_len, reason`, each bounded by
  `MAX_FAILURE_PATH_BYTES` / `MAX_FAILURE_REASON_BYTES` (`sink.rs:40-48`);
  an over-long length bails like an unknown tag (`pipeline.rs:1431`).

**Record terminators** (every single-file record now ends explicitly):

- In-stream: `Frame::FileEnd(RecordEnd)` — field 22;
  `RecordEnd { bool ok = 1; string reason = 2; }`. The destination stops
  counting body bytes on `FileEnd`: `ok` requires cumulative bytes ==
  `header.size` (else violation); `!ok` is accepted at any cumulative
  count (the source need not pad on this carrier).
- Data plane: after the `file_size` body bytes, one status byte
  (`0` ok; `1` failed, followed by `u32 reason_len, reason`). Because the
  receiver reads exactly `file_size` bytes first, a source that fails
  mid-body **pads with zeros to `file_size`** and then writes the failed
  trailer.
- Resume: `BlockComplete` (`blit.proto` field 15; data-plane tag
  `BLOCK_COMPLETE`) gains the same `ok`/`reason` pair.
- Tar shards already end with `TarShardComplete`; members never fail
  mid-record because shards are built whole in memory (D-B).

**Sink discard.** `TransferSink` gains `discard_active(path)`: the
`FsTransferSink` streamed receive (`write_file_stream`, sf-3c's retained
handle) drops the handle and unlinks the path it opened for this record —
never a path it did not open — and never stamps. On the local route the
sink both reads and writes (D-E), so discard is internal to
`copy_resolved_file_payload`. `NullSink` discards nothing.

**Source emission.** In-stream `send_payload_records` (`mod.rs:3026-3155`):
open and re-stat **before** `FileBegin` (today `FileBegin` at `:3074`
precedes `open_file` at `:3079`); open/stat failure → `FileSkipped`,
`continue`; then body; then re-stat the handle (D-C); then `FileEnd`.
Data plane `DataPlaneSession::send_file` (`data_plane.rs:464-475`): open
already precedes the tag; failure → SKIP record instead of `?`; body via
`take(size)`; re-stat; trailer. Resume: `ResumeBlockDiff::open` failure
(`resume_diff.rs:66-77`) → skip; read/EOF error (`:101-119`) → failed
`BlockComplete` on both carriers (`send_resume_block_records`, `mod.rs:
3220-3226`; `DataPlaneSink` BLOCK records). Shard skips (D-B) are emitted
before the shard header on both carriers and recorded directly on the
local route.

Guards for ssc-1: ungated fault-injecting `TransferSource` whose
`open_file` fails for one path (pattern: `TruncatedReadSource`,
`crates/blit-core/tests/transfer_session_roles.rs:957-1000`) on both
carriers; `cfg(windows)` guard opening the file with
`OpenOptions::share_mode(0)`; ledger violation pins (A5); mutation proof:
restore the `?` at `data_plane.rs:469-472` → the fatal returns.

### D-B. Shard packer fidelity (ssc-2)

`build_tar_shard` (`remote/transfer/payload.rs:296-335`) currently trusts
`header.size` for the tar header and streams the file to EOF. New
contract: **a member is appended only from a buffer that is exactly
`header.size` bytes long.**

```
for header in headers:
    file = File::open(full_path)            -- Err → skipped.push(source: cannot open: {err}); continue
    now  = file.metadata().len()            -- != size → skipped.push(source: changed size during transfer (manifest {size} bytes, now {now})); continue
    buf  = Vec::with_capacity(size)
    (&mut file).take(size).read_to_end(&mut buf)   -- Err → skipped.push(source: read error: {err}); continue
    if buf.len() != size                     -- shrank mid-read
        skipped.push(changed size …); continue
    if file.read(&mut [0u8;1]) == 1          -- grew mid-read
        skipped.push(changed size …); continue
    tar_header.set_size(size); builder.append_data(&mut tar_header, rel, &buf[..])
    packed.push(header)
return TarShardBuild { data, headers: packed, skipped }
```

The stat is the cheap pre-check with the exact "now" size for the
message; take+probe is the race-proof check. Shards are already
whole-in-memory (`Vec<u8>`), so the buffer adds no new memory class. A
shard whose every member was skipped produces no shard payload — only
skips.

`PreparedPayload::TarShard` (`payload.rs:137-140`) gains
`skipped: Vec<FileFailure>`; `bound_in_stream_tar_headers` (`mod.rs:2986`)
splits on the packed headers only. `safe_extract_tar_shard`
(`tar_safety.rs:111-231`) is unchanged — `TarShardHeader.files` is the
packed list, so `require_exact_headers` still holds.

Red proof for A1 (ungated): write two files, capture headers, append
bytes to the first, `prepare_payload`, extract with
`safe_extract_tar_shard` → today `tar shard entry: numeric field was not a
number` (or a size-mismatch bail, depending on alignment); after ssc-2 the
second file lands and the first is in `skipped`.

### D-C. Size drift: before, during and after the body (ssc-1/ssc-3)

D1 policy, applied uniformly: a file whose size is not `header.size` at
the moment the source finishes reading it is not delivered.

- **Before announcement (both carriers):** `file.metadata().len()` from
  the opened handle ≠ `header.size` → skip.
- **During:** the body is read through `.take(header.size)` on both
  carriers, so growth can never spill into the framing (the in-stream loop
  already bounds by `remaining`, `mod.rs:3083`; the data-plane
  double-buffered loop at `data_plane.rs:483-535` reads `file_size` bytes
  — pin it). A short read (shrink) → failed terminator (A9).
- **After the body, before the terminator:** re-stat the handle; `len ≠
  header.size` → failed terminator with the changed-size reason (A9). A
  file that grew *after* the bytes were read is therefore retracted, not
  delivered as a stale prefix under a stale mtime.
- **Local route single file** (`copy_resolved_file_payload`,
  `sink.rs:1371-1400`, `resume_copy_file`): open the source handle first,
  stat it, copy bounded to `header.size` (the zero-copy/clonefile fast
  paths copy whole files, so they are taken only when the post-copy
  re-stat still matches; otherwise the partial is removed), re-stat after
  the copy, and on any mismatch remove the destination file and
  `record_failure` with the changed-size reason (A12). Stamping happens
  only after the re-stat matches.

### D-D. Retraction is a terminator, not a heuristic (ssc-3)

A post-announcement failure is expressed **inside the record** by its
terminator (D-A), so the destination never has to decide whether a
trailing skip refers to the record it just finished, an earlier one, or a
pre-existing file — r1 F3. Ordering is the lane's own: the terminator is
the next thing read after the body. The only destructive act is the
sink's `discard_active`, scoped to the handle the sink opened for the
active record. Pin with a decoy file at the same relative path planted
before the session (the discard must remove only what this record wrote —
on the in-place write model the decoy is overwritten and then removed;
the test asserts the path is absent and the failure reported) and a decoy
outside the destination root (untouched).

### D-E. Preparation returns per-file outcomes (ssc-4)

`prepare_payload` (`payload.rs:48-110`) propagates
`hydrate_payload_header` errors with `?` for `File` (`:53-64`) and for
every shard member (`:65-75`); the pipeline turns any preparation error
into a session failure (`pipeline.rs:472-476`, `:548-552`). New shape:

```
PreparedPayload::File(header) | PreparedPayload::Skipped(FileFailure)
PreparedPayload::TarShard { headers, data, skipped }
```

Hydration is per member; a member whose hydration fails (vanished,
access denied, named-stream size drift at `windows_metadata.rs:802`) goes
to `skipped` with a `source:` reason. Pipeline-level `Err` is reserved for
non-file infrastructure failures (worker panic, source root gone). Every
consumer of `PreparedPayload` (in-stream `send_payload_records`,
`DataPlaneSink::write_payload` `sink.rs:1906-2020`, `FsTransferSink::
write_payload` `sink.rs:1048-1093`, `NullSink`) handles `Skipped` by
emitting the carrier's skip (D-A) or recording directly (local).

**Local availability pre-check retired (D3).** `LocalApply::plan_chunk`
(`transfer_session/local.rs:537-574`) calls `check_availability` →
`filter_readable_headers` (`remote/transfer/source.rs:638-681`), which
pre-opens every needed file: NotFound/PermissionDenied go to the scan's
`unreadable` list and any other error is fatal; `mod.rs:4736-4748` then
refuses the whole mirror after the apply joined if that list is non-empty.
With D-A/D-B/D-E the pre-check is redundant (the packer and the sink
contain the same failures per file) and its refusal is wrong on
principle (Constraints: deletion safety is enumeration completeness).
ssc-4 deletes `check_availability`/`filter_readable_headers` and the
apply-time refusal; the `TransferSource` trait loses the method
(`source.rs:178`); the scan-time refusal at ManifestComplete and
`LocalMirrorSummary.unreadable_paths` for scan-time entries stay. This
also makes the LOCAL non-UTF-8 case a `files_failed` entry (today it is
an `unreadable_paths` entry, and the 0.1.2 CHANGELOG's "exits 2" claim was
not what the code does).

### D-F. Lossy names are flagged at the scan, never inferred (ssc-1 wire, ssc-5 reasons)

`relative_path_to_posix` (`path_posix.rs:36-44`) converts each component
with `to_string_lossy`; the scan (`source.rs:482-485`) opens the real
`absolute` path but emits the lossy `rel`, so the source later cannot
re-open the file by its own header. Inferring lossiness from U+FFFD in the
string is unsound (a legitimate U+FFFD name aliases it — r1 F5). Instead:

- `FileHeader` gains `bool name_lossy = 7` (contract 7, ssc-1), set by
  the scan when any component's `to_str()` is `None` (checked on the
  `OsStr` before conversion). The lossy string stays the manifest identity
  so mirror matching and deletion safety behave exactly as today.
- Every source payload path (`send_payload_records`, `send_file`,
  `build_tar_shard`, `ResumeBlockDiff::open`, hydration) skips a
  `name_lossy` header **without opening anything**, reason
  `source: filename is not valid UTF-8 (rename it to transfer)`.
- The destination never needs the flag; it is informational there.

### D-G. Words and docs (ssc-5)

- `failure_block_styled` (`blit-cli/src/transfers/failures.rs:84-120`):
  header becomes "{n} file(s) did not land at the destination:"; the
  trailer keeps the re-run hint.
- `refuse_source_delete_on_failures` message: "… did not land at the
  destination" (drop "could not be written").
- `docs/TRANSFER_SESSION.md`: frame table gains `FileSkipped`, `FileEnd`,
  the `BlockComplete` status, the data-plane SKIP tag and trailer; the v7
  section describes the ledger, skip and retraction semantics and the
  reason prefix; Errors section drops "send-side source read" from the
  fatal list.
- `CHANGELOG.md` Unreleased: reliability entry + retire the 0.1.2 non-UTF-8
  remote caveat; `docs/plan/RELEASE_1_0.md` G3 lists this plan as the
  fix-now item (owner ruling D4); `docs/plan/PER_FILE_ERROR_CONTAINMENT.md`
  §Non-goals send-side line gets a pointer here.

### Risks

- **Protocol strictness.** Every ledger transition not in the table is a
  violation, so a buggy source cannot silently drop or double-deliver a
  file. A5 pins the never-delivered check, the un-granted skip, and the
  terminator-without-active-record cases.
- **Data-plane framing.** A malformed SKIP record or trailer (over-long
  lengths, unknown status byte) bails like an unknown tag, never skips
  past; pin with corrupt-record tests.
- **Discard scope.** `discard_active` acts only through the handle the
  sink opened for the active record; the decoy tests in D-D pin it.
- **Zero padding on the data plane** costs up to `header.size` bytes of
  zeros for a file that shrank mid-body — bounded by the file's own
  manifest size, paid only on the failure path, and cheaper than the
  session abort it replaces.
- **Reporting cap.** Thousands of skips (a whole live profile) exceed the
  64-entry list; `files_failed_total` still counts them all and the block
  prints the elided count — existing behaviour, unchanged.

## Slices

One coherent, testable change per slice — each its own go, commit, full
gate, DEVLOG entry, CI on all three OSes before the next.

1. **ssc-1 — contract 7: ledger, skip record, terminators, `name_lossy`
   (A3, A4, A5, A6, A7, A13-wire).** Proto: `FileSkipped`=21,
   `FileEnd`=22, `RecordEnd`, `BlockComplete` ok/reason, `FileHeader.
   name_lossy`=7; data plane: SKIP tag 4, body trailer, BLOCK_COMPLETE
   status. Destination `NeedLedger` replacing `OutstandingNeeds` +
   `GrantedHeaders`; every existing record path emits/expects an ok
   terminator; `discard_active` on the sink; skip-before-announce for
   single-file open/stat failure and `name_lossy` on both carriers and
   the local route. Guards per D-A. Mutation proof: restore the `?` at
   `data_plane.rs:469-472`.
2. **ssc-2 — shard packer fidelity (A1, A2).** D-B; `PreparedPayload::
   TarShard.skipped`; emission at all three consumers. Red proof
   reproduces the field message. Guards: grown, shrunk, vanished member;
   all-members-skipped shard; existing `tar_safety` exact-header pins
   still green.
3. **ssc-3 — retraction and post-body drift, resume (A9, A10).** D-C
   during/after checks on both carriers; failed terminators; resume
   open→skip and mid-diff→failed `BlockComplete`; decoy pins (D-D).
4. **ssc-4 — per-file preparation, local route (A8, A11, A12; owner D3).**
   D-E: `PreparedPayload::Skipped`, per-member hydration, pipeline `Err`
   reserved for infrastructure; delete `check_availability`/
   `filter_readable_headers`/the apply-time mirror refusal; rename-and-flip
   `mirror_refuses_when_availability_drops_after_clean_scan`;
   `VanishingSource` (`local.rs:1171-1210`) becomes the A8 fixture; local
   bounded copy + post-copy validation (A12).
5. **ssc-5 — words, non-UTF-8 reasons, docs (A13, A14).** D-F reasons,
   D-G, CHANGELOG Unreleased, RELEASE_1_0 G3 (D4),
   PER_FILE_ERROR_CONTAINMENT pointer.

Executed order ssc-1 → ssc-2 → ssc-3 → ssc-4 → ssc-5.

## Review history

- **r1** (codex-cli 0.156.0 / gpt-5.6-sol / xhigh / frontier, grade
  fallback; over `0bcad512..30534b07`): `acceptable_with_changes`, 5
  material changes, 5 findings (3 HIGH, 2 MEDIUM), all verified against
  the code and adopted: F1 resume diff opens the SOURCE file (plan had it
  as destination-side) → A10/D-A resume rows; F2 hydration `?` bypasses
  containment → D-E; F3 `granted_headers` is removed at claim so the
  retraction heuristic could not work → ledger + terminators (D-A/D-D);
  F4 drift checked only pre-announce and local fast path copies current
  bytes → D-C after-body re-stat + A12; F5 U+FFFD inference aliases
  legitimate names → `name_lossy` flag (D-F). Records:
  `.review/results/2026-09-25-source-side-containment-plan-r1-*`.

## Open questions

- **D1 — drift policy.** A file whose size changed since the scan: skip and
  report (this draft), or land the bytes actually read under a corrected
  header? Recommendation: skip. The manifest promised size+mtime; landing
  different bytes under that mtime makes the next compare call it
  converged when it is not. Skipping converges on re-run once the file is
  quiet; a never-quiet file never lands and is always reported.
  — owner
- **D2 — retraction (ssc-3).** Adopt D-D, or keep post-announce read
  failures fatal? Recommendation: adopt; with terminators it is the
  natural shape of the record, and it is the last way one live file can
  kill a run. If "keep fatal": ssc-3 shrinks to the resume open-skip, A9
  and the retraction rows are struck, `FileEnd` keeps only `ok = true`.
  — owner
- **D3 — retire the local availability pre-check and its apply-time mirror
  refusal (D-E).** Recommendation: retire; deletion safety is the scan's
  completeness, not a file's openability. — owner
- **D4 — 1.0 gate.** Record this plan under `RELEASE_1_0.md` G3 as a
  fix-now item? Recommendation: yes; backing up a live home directory is
  the everyday workload. — owner
