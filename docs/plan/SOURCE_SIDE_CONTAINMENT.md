# Source-Side Containment — a file the source cannot deliver is skipped, not fatal

**Status**: Draft — owner ordered the plan and a codex review loop to
consensus (2026-09-25); no code until `Active`. Open rulings D1–D6 below.
Review record: `REVIEW.md` §Plan reviews (openreview), rows
`plan-ssc-2026-09-25-r*`; r1–r4 `acceptable_with_changes` (21 material
changes: 17 adopted, staged writes carried into the design behind owner
ruling D5 with its staging-budget sub-question, manifest-entry IDs
declined → D6 — see §Review history).
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
- **Resume patches stay in place** (owner ruling D-2026-07-09-1 Q2: "in-place
  patch stays (no temp+rename atomicity …) — convergence-on-retry is the
  reliability model"). That ruling is scoped to resume block patching;
  streamed full-file records are a different write and are staged under
  D-H, subject to owner ruling D5.
- **No manifest-entry IDs.** Path strings remain the protocol identity for
  manifest, needs, records, skips and terminators, as they are for every
  frame today; a lossless or opaque identity is a protocol redesign with
  its own plan (D6). The collisions it would prevent are detected and
  reported at manifest intake instead (D-F).
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
  that collapse to the same lossy string collide in every path-keyed map
  (`payload.rs:200-203`, `mod.rs:1922`); D-F reports the duplicate
  instead of letting it overwrite (D6 would remove the collision itself).
- No version compatibility: the contract bump (Constraints) means a
  pre-plan peer refuses at session open, by design (D-2026-08-18-2).

## Constraints

- **Contract bump.** Both carriers change shape (skip record, chunked
  bodies with terminators, resume-completion status, header flag), so
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
  committed (mtime/attributes stamped, counted as transferred, and — for
  staged streamed records, D-H — renamed over the target) only when its
  terminator says the source delivered exactly `header.size` bytes from a
  handle that still had that size afterwards. Anything else is aborted by
  the sink and reported, and the target path is left exactly as it was
  before the record began.
- **Explicit per-need state, one ledger.** The destination tracks each
  granted path as `Granted → Active(lane) → Completed | Failed` (D-A).
  Skips are accepted only in `Granted`; a terminator only for the
  `Active` record on the lane it arrives on. Anything else is a protocol
  violation. There is no path-lookup heuristic and no unlink-by-path: a
  sink aborts only the record writer it opened. A destination-side
  contained failure at record open (today's drain-then-contain,
  `sink.rs:1168-1185`) is a *discarding* writer, never a session error,
  and the ledger's final state is derived from the sink's outcome.
- **Record identity on the wire is the exact header path.** Skip records
  and terminators carry `relative_path` verbatim, bounded only by the
  existing per-carrier path bound (`data_plane.rs:493`, "relative path too
  long for transfer"); the `MAX_FAILURE_PATH_BYTES` truncation
  (`sink.rs:48`, `:188-191`) applies to the *reported* `FileFailure` at
  `to_wire` only, never to a ledger key.
- **FAST is not traded for this.** The data-plane body framing changes
  (D-A: length-prefixed chunks); the chunk is the existing double-buffer
  size (`data_plane.rs:483-535`) so the per-chunk overhead is a `u32` per
  buffer, and ssc-1 records a before/after run of the data-plane tripwire
  bench (`scripts/bench_tripwires.sh`) on one rig showing no change beyond
  noise before the slice is called green.
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
- **Same handle, every route.** Every size or identity check on the
  source — remote carriers and the local copy cascade alike — uses the
  handle the bytes are read from (`OpenedSourceFile`, D-A); no route
  validates one file and copies another.
- **Every slice lands green on its own.** A slice claims only the guards
  its own code can pass; local-route containment of open failures cannot
  be proven while the local availability pre-check still intercepts them,
  so it is ssc-4's, not ssc-1's (r2 F6).
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
  carrier and the data plane (ssc-1) and on the local route (ssc-4);
  nothing is announced for it on the wire (no `FileBegin`, no FILE record
  tag).
- [ ] **A4:** a single-file payload whose size changed between scan and
  open is skipped before announcement on both carriers.
- [ ] **A5:** the destination's "SourceDone with N needed file(s) never
  delivered" check (`mod.rs:4778-4786`) and its resume twin
  (`:4794-4802`) still fire for a need that is neither delivered, skipped,
  nor retracted; a skip for a path not in `Granted`, or a terminator on a
  lane with no `Active` record, is a protocol violation.
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
  its body was sent, ends with the destination path exactly as it was
  before the record (a pre-existing copy intact, D-H; absent if there was
  none), the failure reported, the session complete — on both carriers, and
  through the relay sink (`DataPlaneSink::write_file_stream`,
  `sink.rs:2022-2038`, live at `transfer_session/data_plane.rs:2406`),
  where the failed terminator is forwarded downstream and the downstream
  destination reports the same failure. Red today with `'<path>' hit EOF
  with N bytes still promised` session-fatal (`mod.rs:3090-3101`,
  `data_plane.rs:566-570`). A failed record transmits no bytes beyond the
  chunk in flight (no padding; pin by byte count).
- [ ] **A10 (resume):** a resume-granted file whose source open fails is
  skipped; one whose source read fails or shrinks mid-diff is closed as
  failed — the destination's partial is left in place and **not stamped**,
  the failure is reported, the session completes. A resume whose blocks
  all match (zero block records, `BlockComplete` straight from the grant —
  today's `claim_block_complete`, `data_plane.rs:2239`) still completes
  as it does now, on both carriers. Red today via
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
  `source: changed size during transfer (…)`, the destination path is as
  it was before, exit 2. The copy and the validation use one opened
  source handle: a test that atomically replaces the source path between
  open and copy lands the bytes of the opened file or nothing — never the
  replacement's bytes under the manifest header. Red today: the current
  bytes land under the manifest mtime and the cascade reopens by path
  (`sink.rs:1371-1400`, `copy_file(src, dst, …)` at `:1398`).
- [ ] **A13 (non-UTF-8):** a source file whose name is not valid UTF-8 is
  reported as `source: filename is not valid UTF-8 (rename it to transfer)`
  with exit 2 on every route (remote in ssc-1, local in ssc-4) **whatever
  the diff would have decided for it** — a lossy entry the destination
  would otherwise judge converged is still reported — and no payload is
  ever requested for it; the remote session no longer aborts. Two manifest
  entries that collapse to one path are reported as
  `source: duplicate manifest path (lossy name collision)` for the second,
  never silently overwritten. The 0.1.2 CHANGELOG known-limitation text is retired in
  the Unreleased notes.
- [ ] **A14 (words/docs):** the CLI failure block header no longer says
  "could not be written" (`crates/blit-cli/src/transfers/failures.rs:95`);
  the move gate message (`blit-core/src/transfers/failures.rs`) no longer
  says "could not be written and remain un-landed"; both read correctly
  for a source-side failure. `docs/TRANSFER_SESSION.md` frame table
  (`:157-194`), the v6 failure section (`:196-250`) and Errors
  (`:464-520`) describe v7.
- [ ] **A15 (FAST):** the data-plane tripwire bench shows no throughput
  change beyond noise across the ssc-1 body-framing change (one rig,
  before/after, recorded in the slice's DEVLOG entry).
- [ ] **A16 (destination-open containment preserved):** a destination path
  that cannot be created for one streamed record (today's
  drain-then-contain, `sink.rs:1168-1185`, pinned by the existing
  `one_blocked_file_fixture` tests) is still contained under the new sink
  lifecycle: the record's framing is consumed through its terminator, the
  failure is that file's, the session continues.
- [ ] **A18 (dry-run):** a `--dry-run` on every route creates no parent
  directory and no staging file under the new lifecycle (guard asserts
  the destination tree is unchanged, including after a mid-run
  cancellation); the existing R58-F4 pins stay green.
- [ ] **A17 (FAST, local):** the local large-file copy shows no throughput
  change beyond noise across the handle-based cascade change (ssc-4;
  `scripts/bench_local_mirror.sh` or its macOS/Windows variant on one
  host, before/after, in the slice's DEVLOG entry).
- [ ] Every new guard is mutation-proven red/green (revert the fix, watch
  it fail with the pre-plan message, restore). Test count never drops;
  the one flipped test (A8) is renamed, not removed.
- [ ] Gate green on macOS + Linux-cross clippy; CI green on all three OSes
  at the slice head before the next slice starts (Windows is the platform
  that motivated the plan).

## Design

### D-A. Need ledger, skip record, chunked records with terminators (ssc-1)

**Ledger.** Replace the two destination sets — `OutstandingNeeds`
(`transfer_session/data_plane.rs:76`, created `mod.rs:3952`) and
`GrantedHeaders` (`data_plane.rs:80`, `mod.rs:3953`), which today are both
removed at claim (`mod.rs:4354-4367`, `data_plane.rs:2007-2011`) — with one
`NeedLedger: HashMap<String, NeedState>` keyed by the exact header path:

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
| `FileBegin` / FILE tag / shard header member | Granted | Active(L) (shard members go straight to Completed on shard success, Failed on per-member containment as today) | sink `begin_record` → real writer, or a **discarding** writer carrying a contained destination failure (A16) |
| terminator ok | Active(L), same L | Completed if the writer committed; Failed if it was discarding | sink `commit` (stamp, count, rename-over for staged records D-H); violation unless bytes == `header.size` |
| terminator failed(reason) | Active(L), same L | Failed | sink `abort`; `record_failure` |
| first `BlockTransfer` / BLOCK record | Granted(resume) | Active(L, Resume) | patch in place as today |
| resume `BlockComplete` ok | Granted(resume) or Active(L, Resume), same L | Completed | stamp as today (zero-block case straight from the grant, A10) |
| resume `BlockComplete` failed(reason) | Granted(resume) or Active(L, Resume), same L | Failed | leave partial unstamped; `record_failure` |
| `SourceDone` | any Granted/Active remaining | — | violation, as today (`mod.rs:4778-4802`) |

A lane processes records sequentially (`receive_file_record`,
`mod.rs:6063-6121`; `pipeline.rs:1255-1261`), so "the Active record on
this lane" is unambiguous: at most one per lane. The data-plane sink's
`claim`/`claim_shard` (`data_plane.rs:2071-2115`) become ledger
transitions; the in-stream arms at `mod.rs:4354` and `:4514-4548` likewise.

**Skip record** ("the source will not deliver this granted file"):

- In-stream: `Frame::FileSkipped(FileFailure)` — new oneof field 21 in
  `TransferFrame` (`crates/blit-core/proto/blit.proto:1153-1176`); add to
  `frame_name` (`mod.rs:624-648`). `relative_path` is the exact header
  path; `reason` is bounded by `MAX_FAILURE_REASON_BYTES`.
- Data plane: `DATA_PLANE_RECORD_SKIP = 4` (`data_plane.rs:17-21`):
  `u32 path_len, path, u32 reason_len, reason`; path bounded by the
  carrier's existing path bound, reason by `MAX_FAILURE_REASON_BYTES`;
  an over-long length bails like an unknown tag (`pipeline.rs:1431`).

**Chunked bodies and terminators** (every single-file record now ends
explicitly, and a failed record ends at once — no padding, r2 F2):

- In-stream: the body already arrives as discrete `FileData` frames
  (`blit.proto:238`); add `Frame::FileEnd(RecordEnd)` — field 22,
  `RecordEnd { bool ok = 1; string reason = 2; }`. `ok` requires
  cumulative bytes == `header.size` (else violation); `!ok` is accepted at
  any cumulative count.
- Data plane: the FILE record body becomes `repeat { u32 len; len bytes }`
  terminated by `len = 0`, followed by one status byte (`0` ok; `1`
  failed, then `u32 reason_len, reason`). `len` is bounded by the
  double-buffer size (`data_plane.rs:483-535`); the receiver
  (`pipeline.rs:1255-1261`, today `take(file_size)`) loops on chunks into
  the same buffers, sums them, and applies the `ok`-requires-`header.size`
  rule. A source that fails mid-body writes the `0` sentinel and the
  failed status immediately after the last chunk it sent.
- Resume: `BlockComplete` (`blit.proto` field 15; data-plane tag
  `BLOCK_COMPLETE`) gains the same `ok`/`reason` pair.
- Tar shards already end with `TarShardComplete`; members never fail
  mid-record because shards are built whole in memory (D-B).

**Sink lifecycle** (r2 F3): `TransferSink` (`sink.rs:568-597`) gains a
record lifecycle beside `write_payload`:

```
async fn begin_record(&self, header: &FileHeader) -> Result<Box<dyn RecordWriter>>;
trait RecordWriter { write(bytes); commit() -> SinkOutcome; abort(reason) -> SinkOutcome }
```

`write_file_stream` (`sink.rs:578`) is replaced by it; the caller
(`receive_file_record`, `mod.rs:6061`, and the data-plane receiver) drives
the writer and calls `commit` on an ok terminator or `abort` on a failed
one. `begin_record` returns `Err` only for the fatal classes
(path-safety, volume, destination root, transport); a **containable**
destination failure (today's drain-then-contain at `sink.rs:1168-1185`)
returns a discarding writer that consumes the body and reports that
file's failure at `commit`, so the already-shipped behaviour is
preserved (A16) and the ledger's final state follows the sink outcome.
In **dry-run** (`FsSinkConfig.dry_run`, today's R58-F4 short-circuits at
`sink.rs:1138` and `:1377`) `begin_record` returns a non-writing writer
that validates framing and counts, creating neither a parent directory
nor a staging file (A18).
Implementations: `FsTransferSink` — `begin_record` opens a **staging
file beside the target** (D-H) with sf-3c's retained handle, `commit`
stamps metadata on the staging file and renames it over the target,
`abort` drops the handle and removes only the staging file, leaving any
prior destination copy intact.
`DataPlaneSink` (the relay, `sink.rs:2022-2038`) — `begin_record`
announces the downstream FILE record, `write` forwards chunks, `commit`
writes the ok trailer, `abort` writes the failed trailer with the
upstream reason, so a contained failure propagates as a contained failure
through every hop (A9 relay case). `NullSink` — counts only. The local
route's `LocalApply` wrappers (`local.rs:1361`, `:1631`) delegate.

**Opened source file** (r2 F4, r3 F4, r4 F3): `TransferSource::open_file`
(`remote/transfer/source.rs:184-188`) returns an `OpenedSourceFile`:

```
enum OpenedSourceFile {
    Fs { file: tokio::fs::File, path: PathBuf },          // production: owns the descriptor/handle
    Virtual { reader: Box<dyn AsyncRead + Unpin + Send>, len: u64 },  // tests, fault injection
}
impl OpenedSourceFile { async fn metadata(&self) -> io::Result<Metadata>; fn reader(&mut self) -> &mut (dyn AsyncRead + Unpin + Send); }
```

`Fs` exposes the concrete handle to the local copy cascade (D-C) so the
platform fast paths run on the *opened* file: Linux `copy_file_range` /
`sendfile` already take the source `File` (`copy/file_copy/mod.rs:
162-180`); macOS switches `clonefile(src_path, …)` to `fclonefileat(src_fd,
…)` and `fcopyfile(src_fd, dst_fd)`; Windows has no handle-based
`CopyFileEx`, so the buffered copy runs on the handle. Nothing in the
cascade reopens `src` by path any more (`copy_file(src: &Path, …)`,
`mod.rs:29`, becomes `copy_opened(&OpenedSourceFile, dst, …)`). Every
size check in D-C reads the file actually being sent, never a path that
may now name a replacement inode; a test that atomically replaces the
source path after open proves it (A12). `Virtual` is what fault-injecting
test sources return.

**Source emission.** In-stream `send_payload_records` (`mod.rs:3026-3155`):
open and stat **before** `FileBegin` (today `FileBegin` at `:3074`
precedes `open_file` at `:3079`); open/stat failure → `FileSkipped`,
`continue`; body through `take(size)`; re-stat (D-C); `FileEnd`. Data
plane `DataPlaneSession::send_file` (`data_plane.rs:464-475`): open
already precedes the tag; failure → SKIP record instead of `?`; chunked
body; re-stat; status. Resume: `ResumeBlockDiff::open` failure
(`resume_diff.rs:66-77`) → skip; read/EOF error (`:101-119`) → failed
`BlockComplete` on both carriers (`send_resume_block_records`, `mod.rs:
3220-3226`; `DataPlaneSink` BLOCK records). Shard skips (D-B) are emitted
before the shard header on both carriers and recorded directly on the
local route (ssc-2).

Guards for ssc-1: ungated fault-injecting `TransferSource` whose
`open_file` fails for one path (pattern: `TruncatedReadSource`,
`crates/blit-core/tests/transfer_session_roles.rs:957-1000`) on both
carriers; `cfg(windows)` guard opening the file with
`OpenOptions::share_mode(0)`; ledger violation pins (A5); corrupt
SKIP/trailer/chunk-length pins; mutation proof: restore the `?` at
`data_plane.rs:469-472` → the fatal returns; tripwire bench (A15).

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
the moment the source finishes reading it is not delivered. All checks
use the `OpenedSourceFile` handle (D-A), never the path.

- **Before announcement (both carriers):** handle `metadata().len()` ≠
  `header.size` → skip.
- **During:** the body is read through `.take(header.size)` on both
  carriers, so growth can never spill into the framing (the in-stream loop
  already bounds by `remaining`, `mod.rs:3083`; the data-plane chunk loop
  is bounded by construction). A short read (shrink) → failed terminator
  (A9).
- **After the body, before the terminator:** re-stat the handle; `len ≠
  header.size` → failed terminator with the changed-size reason (A9). A
  file that grew *after* the bytes were read is therefore retracted, not
  delivered as a stale prefix under a stale mtime.
- **Local route single file** (`copy_resolved_file_payload`,
  `sink.rs:1371-1400`): the cascade today calls path-based
  `copy_file(src, dst, …)` (`:1398`, `crate::copy`) and `resume_copy_file
  (src, dst, 0)` (`:1395`), which reopen the source independently, so a
  handle opened for validation can describe a different inode than the
  one copied (r3 F4). New shape: open the `OpenedSourceFile` once, stat
  it, and pass the handle through the cascade (`copy_opened`, D-A —
  descriptor fast paths on Linux and macOS, buffered-on-handle on Windows;
  the `CopyFileEx` attribute-preservation quirk noted at `:1404-1407`
  disappears with it). Dry-run keeps its pre-mkdir short-circuit
  (`sink.rs:1374-1380`) untouched (A18).
  Copy bounded to `header.size`, re-stat the same handle after, and on any
  mismatch abort the staged destination (D-H) and `record_failure` with
  the changed-size reason (A12). Stamping happens only after the re-stat
  matches. A17 guards the cost. Resume (`resume_copy_file`) keeps the
  in-place model (Q2) but reads through the same handle.

### D-D. Retraction is the record's own terminator (ssc-3)

A post-announcement failure is expressed **inside the record** by its
terminator (D-A), so the destination never has to decide whether a
trailing skip refers to the record it just finished, an earlier one, or a
pre-existing file (r1 F3). Ordering is the lane's own: the terminator is
the next thing read after the last chunk. The only destructive act is
`RecordWriter::abort`, scoped to the staging file the sink opened for
the active record (D-H). Pins: a decoy at the same relative path planted
before the session is byte-identical afterwards and the failure is
reported; a decoy outside the destination root is untouched; the relay
forwards the failed terminator (A9).

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
`LocalMirrorSummary.unreadable_paths` for scan-time entries stay. Local
A3/A8/A13 are proven here (r2 F6). This also makes the LOCAL non-UTF-8
case a `files_failed` entry (today it is an `unreadable_paths` entry, and
the 0.1.2 CHANGELOG's "exits 2" claim was not what the code does).

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
- **The destination acts on the flag at manifest intake** (r3 F5): a
  `name_lossy` header is recorded through `record_failure` with reason
  `source: filename is not valid UTF-8 (rename it to transfer)` the moment
  it arrives (`mod.rs` manifest arm, before the diff), is never granted as
  a need, and stays in `source_files` so a mirror never deletes its
  destination counterpart. This holds whatever the diff would have said —
  a lossy entry that looks converged is still reported (A13). On the
  local route the same intake step does the same.
- **Duplicate manifest paths are reported, not overwritten**: the retained
  manifest map (`mod.rs:1922`, `sent.insert`) and the planner map
  (`payload.rs:200-203`) treat a second header for a path already present
  as `source: duplicate manifest path (lossy name collision)` — recorded,
  not granted; the first header wins (A13). This is the narrow fix for
  the collision D6 would remove structurally.
- Belt and braces at the source: every payload path
  (`send_payload_records`, `send_file`, `build_tar_shard`,
  `ResumeBlockDiff::open`, hydration) refuses to open a `name_lossy` header
  and emits the skip instead, so a destination that ever granted one
  cannot make the source open the wrong path.

### D-H. Staged streamed records (ssc-3, owner ruling D5)

Today `FsTransferSink` creates and truncates the target directly
(`sink.rs:876`), so a session abort — and, without this section, a
retracted record — leaves the last good destination copy destroyed (r2
F1, r3 F1). D-2026-07-09-1 Q2 ruled in-place for **resume block
patches** only. For **streamed single-file records** (and the local
single-file copy, D-C), the sink writes to a staging file in the
validated target directory named with a **fixed-length,
target-independent basename** (`.blit-stage-<16 hex>`), created with
`create_new` and retried on collision, so a target already at the
filesystem's component-length limit still transfers (r4 F4); the
target mapping lives only in the `RecordWriter`. Metadata is stamped on
the staging file and `commit` renames it over the target
(`std::fs::rename` replaces an existing file on every supported
platform); `abort` removes only the staging file. Tar-shard members are unchanged (whole-in-memory shard,
destination-side per-member containment as today); resume stays in place
(Q2). Costs and their handling: transient space equal to the **sum of the
in-flight streamed records' sizes** — one receive task per inbound TCP
connection (`pipeline.rs:1202-1210`, `data_plane.rs:294`) and the local
pipeline's workers each hold one staging file while its prior target
still exists (r4 F2), so the bound is workers × their current records,
not one file; a destination that fills stays the volume-level fatal it
is today (`failure_is_containable`), every in-process abort removes its
staging file, and a fault-injected low-space run pins both. No admission
budget is added (D5 sub-question: the reviewer proposed a byte-weighted
budget across workers; this draft declines it as machinery for a
condition that is already fatal and rare). One rename per streamed
file, negligible next to a ≥ shard-threshold body; a staging file orphaned by a crash carries the recognisable prefix
and is removed by the next run's sink before it stages the same target
(and is extraneous under a mirror). Windows: the rename uses
`MOVEFILE_REPLACE_EXISTING` semantics via `std::fs::rename`; a target
held open by another process fails the rename → that file's contained
failure, the staging file removed. If D5 rules "keep in place", `abort`
removes the truncated target and A9/A12 read "path absent" instead.

### D-G. Words and docs (ssc-5)

- `failure_block_styled` (`blit-cli/src/transfers/failures.rs:84-120`):
  header becomes "{n} file(s) did not land at the destination:"; the
  trailer keeps the re-run hint.
- `refuse_source_delete_on_failures` message: "… did not land at the
  destination" (drop "could not be written").
- `docs/TRANSFER_SESSION.md`: frame table gains `FileSkipped`, `FileEnd`,
  the `BlockComplete` status, the data-plane SKIP tag, chunked body and
  status trailer; the v7 section describes the ledger, skip and
  retraction semantics, the sink lifecycle and the reason prefix; Errors
  section drops "send-side source read" from the fatal list.
- `CHANGELOG.md` Unreleased: reliability entry + retire the 0.1.2 non-UTF-8
  remote caveat; `docs/plan/RELEASE_1_0.md` G3 lists this plan as the
  fix-now item (owner ruling D4); `docs/plan/PER_FILE_ERROR_CONTAINMENT.md`
  §Non-goals send-side line gets a pointer here.

### Risks

- **Protocol strictness.** Every ledger transition not in the table is a
  violation, so a buggy source cannot silently drop or double-deliver a
  file. A5 pins the never-delivered check, the un-granted skip, and the
  terminator-without-active-record cases.
- **Data-plane framing.** A malformed SKIP record, chunk length or trailer
  (over-long lengths, unknown status byte, chunk sum ≠ `header.size` with
  `ok`) bails like an unknown tag, never skips past; pin with
  corrupt-record tests. The chunk loop touches the throughput hot path;
  A15 guards it.
- **Abort scope.** `RecordWriter::abort` acts only on the staging file the
  sink opened for the active record; the decoy tests in D-D pin it.
- **Staging (D-H).** Transient space, rename-over on SMB/Windows targets,
  orphaned staging files after a crash — each named with its handling in
  D-H; the D5 ruling decides.
- **Reporting cap.** Thousands of skips (a whole live profile) exceed the
  64-entry list; `files_failed_total` still counts them all and the block
  prints the elided count — existing behaviour, unchanged.

## Slices

One coherent, testable change per slice — each its own go, commit, full
gate, DEVLOG entry, CI on all three OSes before the next.

1. **ssc-1 — contract 7: ledger, skip record, chunked records +
   terminators, sink lifecycle, `OpenedSourceFile`, `name_lossy` intake
   (A3 remote, A4, A5, A6, A7, A13 remote, A15, A16).** Proto: `FileSkipped`=21,
   `FileEnd`=22, `RecordEnd`, `BlockComplete` ok/reason, `FileHeader.
   name_lossy`=7; data plane: SKIP tag 4, chunked FILE body + status,
   BLOCK_COMPLETE status. Destination `NeedLedger` replacing
   `OutstandingNeeds` + `GrantedHeaders`; `begin_record`/`RecordWriter`
   on every sink including the relay; every existing record path
   emits/expects an ok terminator; skip-before-announce for single-file
   open/stat failure and `name_lossy` on both carriers. Guards per D-A;
   tripwire bench before/after. Mutation proof: restore the `?` at
   `data_plane.rs:469-472`.
2. **ssc-2 — shard packer fidelity (A1, A2).** D-B; `PreparedPayload::
   TarShard.skipped`; emission at all three consumers (the local route's
   shard skips are recorded by the sink directly and need no pre-check
   change). Red proof reproduces the field message. Guards: grown,
   shrunk, vanished member; all-members-skipped shard; existing
   `tar_safety` exact-header pins still green.
3. **ssc-3 — retraction and post-body drift, resume, relay, staging (A9,
   A10; owner D2, D5).** D-C during/after checks on both carriers; failed
   terminators; resume open→skip and mid-diff→failed `BlockComplete`
   including the zero-block terminal; relay abort forwarding; staged
   streamed records (D-H); decoy pins (D-D).
4. **ssc-4 — per-file preparation, local route (A3 local, A8, A11, A12,
   A13 local; owner D3).** D-E: `PreparedPayload::Skipped`, per-member
   hydration, pipeline `Err` reserved for infrastructure; delete
   `check_availability`/`filter_readable_headers`/the apply-time mirror
   refusal; rename-and-flip
   `mirror_refuses_when_availability_drops_after_clean_scan`;
   `VanishingSource` (`local.rs:1171-1210`) becomes the A8 fixture; local
   handle-based cascade + post-copy validation on the same handle (A12,
   A17).
5. **ssc-5 — words, non-UTF-8 reasons, docs (A13 reasons, A14).** D-F
   reasons, D-G, CHANGELOG Unreleased, RELEASE_1_0 G3 (D4),
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
- **r2** (same reviewer; over `0bcad512..86a3936b`):
  `acceptable_with_changes`, 6 material changes, 6 findings (3 HIGH, 2
  MEDIUM, 1 LOW). Adopted (verified): M2/F2 zero padding → chunked
  data-plane bodies with an immediate failed terminator (D-A, A9, A15);
  M3/F3 relay sink (`sink.rs:2022-2038`, live at `data_plane.rs:2406`)
  not covered → `begin_record`/`RecordWriter` lifecycle on every sink,
  relay forwards the failed terminator (D-A, A9); M4/F4 `open_file`
  returns only a reader → `OpenedSourceFile` with same-handle metadata
  (D-A, D-C); M5/F5 narrow half — skip/terminator paths were bounded by
  the 4 KiB report cap → exact header path is the ledger key, report
  truncation only at `to_wire` (Constraints); M6/F6 ssc-1 claimed local
  guards the pre-check would defeat → local A3/A8/A13 move to ssc-4.
  Contested, routed to the owner: M1/F1 staged temp+rename writes so an
  aborted record preserves the prior destination copy — contradicts
  D-2026-07-09-1 Q2 and is a whole-write-path change beyond this plan's
  scope (D5); M5/F5 wide half — opaque manifest-entry IDs replacing path
  identity on every frame — a protocol redesign beyond this plan's scope,
  and the lossy-name collision it would fix is pre-existing (D6). Records:
  `.review/results/2026-09-25-source-side-containment-plan-r2-*`.
- **r3** (same reviewer; over `0bcad512..90ffbe27`):
  `acceptable_with_changes`, 5 material changes, 5 findings (4 HIGH, 1
  MEDIUM), all verified. Adopted: M3/F2 resume `BlockComplete` is claimed
  straight from the grant today (`data_plane.rs:2239`, zero-block case)
  and the r2 table had no such transition → resume rows rewritten (D-A,
  A10); M4/F3 `begin_record` as drafted would turn today's
  drain-then-contain destination-open failure (`sink.rs:1168-1185`) into
  a session error → discarding writer + outcome-derived ledger state
  (D-A, A16); M5/F4 the local cascade reopens by path (`copy_file`,
  `sink.rs:1398`) so validation and copy can see different inodes →
  handle-based cascade (D-C, A12, A17); the narrow half of M2/F5 — a
  lossy entry the diff judges converged is never reported, and a
  duplicate path overwrites the retained header (`mod.rs:1922`) →
  destination-intake failure + duplicate detection (D-F, A13). M1/F1
  staged writes, raised again with the correct observation that
  D-2026-07-09-1 Q2 is scoped to resume patches: carried into the design
  as D-H for streamed records only, behind owner ruling D5 (recommendation
  changed to adopt, scoped). The wide half of M2 (manifest-entry IDs)
  stays declined for this plan (D6). Records:
  `.review/results/2026-09-25-source-side-containment-plan-r3-*`.
- **r4** (same reviewer; over `0bcad512..4e86bfad`):
  `acceptable_with_changes`, 5 material changes, 5 findings (3 HIGH, 1
  MEDIUM, 1 LOW), all verified; neither contested item was raised again.
  Adopted: M1/F1 dry-run must stay side-effect-free (`sink.rs:1138`,
  `:1377`) → non-writing writer + A18; M3/F3 the drafted
  `OpenedSourceFile` (reader + metadata) could not drive the
  handle-based cascade (`copy/file_copy/mod.rs:29`, `:162-180` reopen by
  path / use concrete `File`s) → `Fs`/`Virtual` variants and
  `copy_opened` (D-A, D-C); M4/F4 `.<name>.blit-<nonce>` breaks
  max-length names → fixed-length `create_new` basename (D-H); M5/F5
  `docs/STATE.md` said D1–D4 → D1–D6. M2/F2 the "one largest file"
  staging bound was wrong (one receive task per TCP connection,
  `pipeline.rs:1202-1210`, `data_plane.rs:294`) → corrected to the
  in-flight sum with low-space and concurrency pins; the proposed
  byte-weighted admission budget is declined in this draft and put to
  the owner inside D5. Records:
  `.review/results/2026-09-25-source-side-containment-plan-r4-*`.

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
- **D5 — staged streamed records (codex r2 M1 + r3 M1, HIGH; D-H).** An
  aborted or retracted streamed record on the in-place model destroys the
  last good destination copy (already true of any abort today). Q2 of
  D-2026-07-09-1 ruled in-place for resume block patches; it did not rule
  on streamed full-file writes. Options: (a) adopt D-H — stage streamed
  single-file records and the local single-file copy, rename on commit;
  resume and shard members unchanged; costs: transient space equal to
  the in-flight streamed records (workers × current record), one rename
  per streamed file, crash-orphan cleanup by prefix; sub-question: add a
  byte-weighted staging admission budget across workers (codex r4 M2) or
  accept volume-full as the already-fatal class it is (this draft);
  (b) keep in place — `abort` removes the truncated target, A9/A12 read
  "path absent". Recommendation: (a); it is the only way "a source
  failure never costs the destination a file it already had" can be
  true, and it is confined to the record type where a mid-body failure
  can happen. — owner
- **D6 — manifest-entry IDs (codex r2 M5 + r3 M2, wide half).** Replace
  path strings with opaque IDs on needs, records, skips and terminators;
  keep the exact `PathBuf` at the source. Fixes the lossy-name collision
  structurally and makes identity independent of path bounds. It is a
  protocol redesign touching every frame; D-F now reports the collision
  instead of suffering it. Recommend declining for this plan and filing a
  TODO; the reviewer has raised it in every round, so this is the
  adjudication the openreview playbook routes to the owner. — owner
