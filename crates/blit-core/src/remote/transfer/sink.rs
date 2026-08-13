//! Pluggable write backends for the transfer pipeline.
//!
//! Every src→dst combination flows through `TransferSource → plan → prepare → TransferSink`.
//! Implementations handle the actual write: local filesystem, TCP data plane, etc.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use eyre::{Context, Result};
use filetime::FileTime;

use crate::buffer::BufferSizer;
use crate::checksum::ChecksumType;
use crate::copy::{copy_file, resume_copy_file};
use crate::generated::{ComparisonMode, FileHeader};
use crate::remote::transfer::payload::PreparedPayload;
use crate::remote::transfer::progress::{ByteProgressSink, NoProbe, Probe};
use crate::remote::transfer::small_file_probe::{BoundSmallFileProbe, MemberTimingReport};
use crate::remote::transfer::source::TransferSource;

// Re-export for consumers.
pub use super::data_plane::DataPlaneSession;

/// Upper bound on the per-file failures one outcome reports.
/// [`SinkOutcome::files_failed_total`] keeps counting past it, so a
/// catastrophic run still carries an exact count behind a bounded list —
/// the same list the summary and the wire carry. The wire list is
/// bounded by encoded bytes as well
/// ([`MAX_WIRE_FAILURES_ENCODED_BYTES`]), so it can be shorter than this
/// count when the entries are long.
pub const MAX_REPORTED_FILE_FAILURES: usize = 64;

/// Ceiling on one reported failure's `reason`, in bytes (cr-pfc4-2).
/// A reason is a formatted `eyre` chain, whose length is bounded by
/// nothing — a deep `with_context` stack over a long path grows it
/// without limit. The report's job is to say WHY a file failed, and the
/// head of that chain is what says it, so the head is what survives the
/// bound.
pub const MAX_FAILURE_REASON_BYTES: usize = 1024;

/// Ceiling on one reported failure's `relative_path`, in bytes
/// (cr-pfc4-2). Deliberately generous: the path is the report's
/// identity half, so it is carried WHOLE for anything a real filesystem
/// holds — 4 KiB clears every practical manifest path, including a
/// Windows extended-length one. Past it the TAIL survives, because the
/// tail is what names the file.
pub const MAX_FAILURE_PATH_BYTES: usize = 4 * 1024;

/// Aggregate budget for one summary's encoded `failures` list, in bytes
/// (cr-pfc4-2).
///
/// [`MAX_REPORTED_FILE_FAILURES`] bounds how many entries the report
/// carries, not how many bytes they encode to — 64 near-maximum
/// path+reason strings encode to megabytes. The report rides the closing
/// `Summary` frame, which is ONE protobuf frame under tonic's default
/// 4 MiB decode limit, so an oversized report does not degrade the
/// report: the peer rejects the whole frame, and a session that
/// successfully contained its per-file failures faults at the very end
/// and delivers no summary at all.
///
/// Same frame discipline, and the same deliberately conservative
/// posture, as the in-stream carrier's `MAX_IN_STREAM_TAR_HEADER_BYTES`
/// (D-2026-07-10-1): bound the sender's encoded bytes far below the
/// limit instead of trusting the shape to stay small. 256 KiB leaves
/// the summary's other fields, the frame envelope, and any future field
/// more than an order of magnitude of headroom.
pub const MAX_WIRE_FAILURES_ENCODED_BYTES: usize = 256 * 1024;

/// Encoded overhead one `failures` entry costs its parent message: the
/// repeated field's tag plus its length delimiter. Field 7 of
/// `TransferSummary` tags in one byte and a per-entry-bounded entry's
/// length varint fits in two, so 8 is deliberately conservative — the
/// same "count the envelope, then round up" posture as
/// `transfer_session`'s `in_stream_tar_header_cost`.
const WIRE_FAILURE_ENVELOPE_BYTES: usize = 8;

/// One per-entry-bounded failure must always fit the aggregate budget.
/// A report that carries a non-zero count has to carry at least one
/// name: "10 files failed" with an empty list tells an operator nothing
/// to act on. Held here as a compile-time property of the constants
/// rather than a runtime special case in [`SinkOutcome::wire_failures`].
const _: () = assert!(
    MAX_FAILURE_PATH_BYTES + MAX_FAILURE_REASON_BYTES + 2 * WIRE_FAILURE_ENVELOPE_BYTES
        <= MAX_WIRE_FAILURES_ENCODED_BYTES,
    "one bounded failure entry must always fit the aggregate byte budget"
);

/// Marker a bounded report string carries so a reader can tell a
/// truncated string from a short one. ASCII on purpose: it is counted
/// against a BYTE budget, so its own length must not depend on the text
/// around it.
const TRUNCATION_MARKER: &str = "[truncated]";

/// Largest char boundary at or below `index`.
///
/// `str::floor_char_boundary` is still unstable, and this cannot be a
/// plain byte slice: slicing mid-character panics, and a byte-sliced
/// multibyte edge is not valid UTF-8 — exactly what a protobuf `string`
/// field may not carry.
fn floor_char_boundary(value: &str, index: usize) -> usize {
    if index >= value.len() {
        return value.len();
    }
    let mut at = index;
    while at > 0 && !value.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Smallest char boundary at or above `index` — the tail-preserving
/// counterpart. Rounding UP only ever shortens the kept tail, so it can
/// never break the byte bound it is computed against.
fn ceil_char_boundary(value: &str, index: usize) -> usize {
    if index >= value.len() {
        return value.len();
    }
    let mut at = index;
    while at < value.len() && !value.is_char_boundary(at) {
        at += 1;
    }
    at
}

/// `value` bounded to `max_bytes`, keeping its HEAD and marking the cut.
/// A value already inside the bound is copied byte-identically, so
/// ordinary reports pass through untouched. The result never exceeds
/// `max_bytes`: a bound too small for [`TRUNCATION_MARKER`] keeps
/// content over annotation rather than overshoot (unreachable for the
/// constants above, but the bound is honored unconditionally).
fn bounded_head(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    if max_bytes <= TRUNCATION_MARKER.len() {
        return value[..floor_char_boundary(value, max_bytes)].to_string();
    }
    let end = floor_char_boundary(value, max_bytes - TRUNCATION_MARKER.len());
    let mut bounded = String::with_capacity(end + TRUNCATION_MARKER.len());
    bounded.push_str(&value[..end]);
    bounded.push_str(TRUNCATION_MARKER);
    bounded
}

/// `value` bounded to `max_bytes`, keeping its TAIL and marking the cut.
/// Same byte guarantee and same byte-identical passthrough as
/// [`bounded_head`]; used for the path, whose tail is the filename.
fn bounded_tail(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    if max_bytes <= TRUNCATION_MARKER.len() {
        let start = ceil_char_boundary(value, value.len() - max_bytes);
        return value[start..].to_string();
    }
    let keep = max_bytes - TRUNCATION_MARKER.len();
    let start = ceil_char_boundary(value, value.len() - keep);
    let mut bounded = String::with_capacity(TRUNCATION_MARKER.len() + value.len() - start);
    bounded.push_str(TRUNCATION_MARKER);
    bounded.push_str(&value[start..]);
    bounded
}

/// One file the destination could not materialize, named for the
/// end-of-operation summary. `relative_path` is the manifest-relative
/// wire path, empty for the single-file destination convention (the
/// destination root IS the file — same rule as `FileHeader`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileFailure {
    pub relative_path: String,
    pub reason: String,
}

impl FileFailure {
    /// Wire form of one contained failure (contract v6), per-entry
    /// bounded (cr-pfc4-2).
    ///
    /// The per-string bounds live HERE, not at the call site, so no
    /// producer of a wire entry can put an unbounded string on the
    /// closing summary frame: `reason` keeps its head up to
    /// [`MAX_FAILURE_REASON_BYTES`], `relative_path` keeps its tail up
    /// to [`MAX_FAILURE_PATH_BYTES`]. Both cut on a char boundary and
    /// mark the cut; anything already inside its bound is copied
    /// byte-identically. The in-memory record keeps its full strings —
    /// the bound is the wire's, and the whole reason is already in the
    /// log `record_failure` wrote.
    pub fn to_wire(&self) -> crate::generated::FileFailure {
        crate::generated::FileFailure {
            relative_path: bounded_tail(&self.relative_path, MAX_FAILURE_PATH_BYTES),
            reason: bounded_head(&self.reason, MAX_FAILURE_REASON_BYTES),
        }
    }

    /// The inverse, for a caller reading a peer's summary back into the
    /// in-memory report (the local summary surface, pfc-5's renderer).
    pub fn from_wire(wire: &crate::generated::FileFailure) -> Self {
        Self {
            relative_path: wire.relative_path.clone(),
            reason: wire.reason.clone(),
        }
    }
}

/// Outcome of writing payload(s) to a sink.
#[derive(Debug, Default, Clone)]
pub struct SinkOutcome {
    pub files_written: usize,
    pub bytes_written: u64,
    /// Files that failed on their own while the session continued
    /// (D-2026-07-30-1), capped at [`MAX_REPORTED_FILE_FAILURES`].
    pub failures: Vec<FileFailure>,
    /// Every per-file failure counted, including those past the cap.
    pub files_failed_total: u64,
    /// Exact identity of the paths *this outcome's own payload* failed,
    /// uncapped and separate from the reported list (pfc-3). One payload
    /// is inherently bounded — the planner clamps a shard's member count
    /// to at most 4096 (`count_target.clamp(128, 4096)`,
    /// `transfer_plan.rs`), every other payload shape carries exactly one
    /// file — while the reported list is a session-wide report that has
    /// to stay capped for memory and the wire. Keeping identity here is
    /// what lets a shard that failed more members than the cap still
    /// name its healthy ones as complete.
    /// [`SinkOutcome::merge_failures`] deliberately does not extend it:
    /// the session-wide set has no such bound, and the resulting excess
    /// of `files_failed_total` over this list is exactly the signal
    /// [`SinkOutcome::file_failed`] reads as "identity is incomplete
    /// here, answer conservatively".
    failed_paths: std::collections::HashSet<String>,
}

impl SinkOutcome {
    /// A clean write of `files_written` file(s) carrying `bytes_written`
    /// bytes. Write sites construct through this rather than a struct
    /// literal so later fields do not ripple through every one of them.
    pub fn written(files_written: usize, bytes_written: u64) -> Self {
        Self {
            files_written,
            bytes_written,
            failures: Vec::new(),
            files_failed_total: 0,
            failed_paths: std::collections::HashSet::new(),
        }
    }

    /// One file that could not be written. Nothing is counted as landed
    /// for it — a failed file is never a written file, whatever partial
    /// bytes reached the destination.
    pub fn failed(relative_path: impl Into<String>, reason: impl Into<String>) -> Self {
        let mut outcome = Self::default();
        outcome.record_failure(relative_path, reason);
        outcome
    }

    /// Record one per-file failure. The reported list stops growing at
    /// [`MAX_REPORTED_FILE_FAILURES`]; the total keeps counting.
    pub fn record_failure(&mut self, relative_path: impl Into<String>, reason: impl Into<String>) {
        let relative_path = relative_path.into();
        let reason = reason.into();
        // Every contained failure is logged here, exactly once — the one
        // point every recorded failure passes through. A contained file
        // that logs nothing is indistinguishable from a transferred one,
        // and the log is the only surface a failure has until the summary
        // carries the report. `merge`/`merge_failures` copy records rather
        // than re-recording them, so no file is logged twice.
        let named = if relative_path.is_empty() {
            "<destination root>"
        } else {
            relative_path.as_str()
        };
        log::warn!("file failed, session continues: {named} ({reason})");
        self.files_failed_total = self.files_failed_total.saturating_add(1);
        self.failed_paths.insert(relative_path.clone());
        if self.failures.len() < MAX_REPORTED_FILE_FAILURES {
            self.failures.push(FileFailure {
                relative_path,
                reason,
            });
        }
    }

    /// Whether this outcome failed `relative_path`.
    ///
    /// Exact for the outcome of one payload, however many members that
    /// payload failed (pfc-3): a tar shard past the reported-list cap
    /// still answers `false` for the members that landed, so the
    /// progress lane completes them. A merged outcome carries a total
    /// larger than the identity it kept and answers `true` for every
    /// path rather than let a failed file be reported complete. The
    /// completion lanes ask a payload's own outcome before any merge,
    /// with ONE deliberate exception: the resume block-record lane
    /// (`receive_block_record`) reads an outcome merged across one
    /// FILE's block records, where the conservative answer coincides
    /// with the exact one. Any future aggregator that merges a
    /// MULTI-file outcome and feeds a completion lane would suppress
    /// healthy files' completions — keep merged outcomes away from
    /// completion filtering.
    pub fn file_failed(&self, relative_path: &str) -> bool {
        if self.files_failed_total > self.failed_paths.len() as u64 {
            return true;
        }
        self.failed_paths.contains(relative_path)
    }

    pub fn merge(&mut self, other: &SinkOutcome) {
        self.files_written += other.files_written;
        self.bytes_written += other.bytes_written;
        self.merge_failures(other);
    }

    /// Fold in `other`'s failure report without its written totals, for a
    /// caller that keeps those per lane (the session counts writes lane by
    /// lane but reports failures as one bounded list).
    ///
    /// Per-payload identity (`failed_paths`) is not folded in: a session's
    /// failed set is unbounded, and the plan's bounded-report constraint
    /// applies to everything a merged outcome carries. The merged total
    /// then exceeds the identity kept, so [`SinkOutcome::file_failed`]
    /// answers conservatively on the merged value — the safe direction.
    /// The only completion lane that reads a merged outcome is the
    /// single-file resume block record (see [`SinkOutcome::file_failed`]).
    pub fn merge_failures(&mut self, other: &SinkOutcome) {
        self.files_failed_total = self
            .files_failed_total
            .saturating_add(other.files_failed_total);
        let room = MAX_REPORTED_FILE_FAILURES.saturating_sub(self.failures.len());
        self.failures
            .extend(other.failures.iter().take(room).cloned());
    }

    /// This report's carried details in wire form, bounded by BOTH the
    /// entry count ([`MAX_REPORTED_FILE_FAILURES`]) and the encoded
    /// bytes the closing summary frame can afford
    /// ([`MAX_WIRE_FAILURES_ENCODED_BYTES`], contract v6).
    ///
    /// This is the one producer of the wire list: every summary
    /// construction site calls it, and the delegated topology's
    /// re-encode copies the already-bounded list off `TransferSummary`
    /// (`delegated_summary::delegated_summary_from_session`) rather than
    /// re-deriving one, so the bound is applied exactly once per report.
    ///
    /// Both halves are re-applied here rather than trusted from
    /// upstream: this is the sender-side bound the wire contract states,
    /// and it must hold for any producer of `failures`, not only the
    /// ones that build the list through
    /// [`SinkOutcome::record_failure`].
    ///
    /// cr-pfc4-2: the count cap alone bounds entries, not bytes. Each
    /// entry is per-entry bounded by [`FileFailure::to_wire`], and the
    /// aggregate budget then drops trailing entries once the encoded
    /// cost would exceed it — measured against prost's own
    /// `encoded_len`, plus the repeated field's envelope, so the number
    /// bounded is the number the frame actually pays.
    /// `files_failed_total` keeps the exact count the summary reports
    /// alongside the list, so a list shortened by either bound reads as
    /// "capped", never as "failures were forgotten".
    pub fn wire_failures(&self) -> Vec<crate::generated::FileFailure> {
        use prost::Message as _;

        let mut wire: Vec<crate::generated::FileFailure> = Vec::new();
        let mut remaining = MAX_WIRE_FAILURES_ENCODED_BYTES;
        for failure in self.failures.iter().take(MAX_REPORTED_FILE_FAILURES) {
            let entry = failure.to_wire();
            let cost = entry
                .encoded_len()
                .saturating_add(WIRE_FAILURE_ENVELOPE_BYTES);
            // Trailing entries are dropped whole, never squeezed: a
            // report is a prefix of the record in record order, so the
            // first failures — the ones that explain what went wrong —
            // are the ones that survive. The compile-time assertion on
            // the constants guarantees the first entry always fits, so a
            // non-zero count never ships an empty list.
            let Some(left) = remaining.checked_sub(cost) else {
                break;
            };
            remaining = left;
            wire.push(entry);
        }
        wire
    }
}

/// True while a per-file failure under `dst_root` is still attributable
/// to one file. Destination-root unavailability is not (every payload in
/// the session fails identically), so it stays session-fatal: a joined
/// wire path needs the root to be a directory, and the single-file
/// destination convention (empty wire path — the root IS the file) needs
/// the root's parent. A parent mkdir for one file's own subpath under a
/// live root is that file's failure.
fn destination_root_live(dst_root: &Path, relative_path: &str) -> bool {
    if relative_path.is_empty() {
        return dst_root
            .parent()
            .is_none_or(|parent| parent.as_os_str().is_empty() || parent.is_dir());
    }
    dst_root.is_dir()
}

/// Raw OS error codes that mean "this volume refuses writes" rather
/// than "this one path was refused" — write-protection and exhaustion
/// alike. Checked alongside the portable [`std::io::ErrorKind`]
/// spellings ([`std::io::ErrorKind::ReadOnlyFilesystem`],
/// [`std::io::ErrorKind::StorageFull`],
/// [`std::io::ErrorKind::QuotaExceeded`]): the kinds are what std names,
/// the numeric codes are what the OS actually returned and stay correct
/// even where a platform's std mapping does not name the kind. The code
/// namespaces are per-platform and overlap with unrelated meanings —
/// unix 19 is `ENODEV` where Windows 19 is `ERROR_WRITE_PROTECT`, unix
/// 30 is `EROFS` where Windows 30 is `ERROR_READ_FAULT`, unix 39 is
/// `ENOTEMPTY` (an ordinary per-file error) where Windows 39 is
/// `ERROR_HANDLE_DISK_FULL` — so each list is cfg-gated to its own
/// platform instead of merged into one set.
///
/// `EDQUOT` is deliberately absent from the raw lists: its number is not
/// portable across unix (122 on Linux, 69 on macOS/BSD), so one
/// unix-wide list would either miss it or claim an unrelated code, while
/// std maps it to [`std::io::ErrorKind::QuotaExceeded`] on every unix.
#[cfg(unix)]
const VOLUME_UNWRITABLE_OS_ERRORS: &[i32] = &[
    30, // EROFS
    28, // ENOSPC
];
#[cfg(windows)]
const VOLUME_UNWRITABLE_OS_ERRORS: &[i32] = &[
    19,  // ERROR_WRITE_PROTECT
    112, // ERROR_DISK_FULL
    39,  // ERROR_HANDLE_DISK_FULL
];
#[cfg(not(any(unix, windows)))]
const VOLUME_UNWRITABLE_OS_ERRORS: &[i32] = &[];

/// True when this one `io::Error` reports volume-level unwritability:
/// the medium refuses writes (read-only mount, write-protected disk), or
/// it has no room left for them (cr-pfc3-1: full volume, exhausted
/// quota).
fn io_error_says_volume_unwritable(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::ReadOnlyFilesystem
            | std::io::ErrorKind::StorageFull
            | std::io::ErrorKind::QuotaExceeded
    ) || error
        .raw_os_error()
        .is_some_and(|code| VOLUME_UNWRITABLE_OS_ERRORS.contains(&code))
}

/// cr-pfc2-1 + cr-pfc3-1: true when `error`'s chain carries a
/// volume-level unwritability signal — read-only filesystem,
/// write-protected medium, exhausted volume or quota.
/// That is a root-wide condition wearing a per-file error's clothes:
/// [`destination_root_live`] sees a perfectly good directory (a
/// read-only mount still reads as one), so without this check every
/// payload's write failure would be contained and a mirror to a
/// read-only mount would write nothing, delete nothing — a write-failed
/// file stays in the source manifest, so it is never extraneous — and
/// report success. The whole chain is walked because every write site
/// wraps its `io::Error` in `with_context` before the classifier sees
/// it.
///
/// Exhaustion (cr-pfc3-1) belongs to the same class: a destination that
/// fills mid-run fails every remaining payload identically — inside a
/// tar shard that is potentially thousands of members — so containing
/// those would let a mirror report success over a demonstrably
/// incomplete backup. The deliberate trade is that a transient `ENOSPC`
/// (space freed again later in the same run) loses per-file
/// continuation; re-running converges once there is room, whereas a
/// falsely successful mirror does not.
fn volume_unwritable(error: &eyre::Report) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(io_error_says_volume_unwritable)
    })
}

/// True while `error` is still attributable to exactly one file under
/// `dst_root`: the destination root is live AND the failure is not a
/// volume-level condition every other payload of the session would hit
/// identically. Both halves must hold — the root check alone cannot see
/// a read-only mount, and the error-kind check alone cannot see a root
/// that vanished.
///
/// Every containment decision in the transfer — single files, resume
/// records, and tar-shard members — routes through this one predicate so
/// the shard paths and the single-file paths can never drift apart on
/// what is per-file and what is session-fatal.
pub(super) fn failure_is_containable(
    dst_root: &Path,
    relative_path: &str,
    error: &eyre::Report,
) -> bool {
    destination_root_live(dst_root, relative_path) && !volume_unwritable(error)
}

/// Classify one file's write error: a recorded [`FileFailure`] the
/// caller reports while continuing with the next file, or the same error
/// back when the destination root itself, or the volume under it, is the
/// problem. Only call this where the error is attributable to exactly
/// one file — path-safety violations, transport errors and
/// destination-root failures must keep returning `Err` (session-fatal).
/// Callers resolve and containment-check the destination first, so a
/// hostile path can never arrive here.
fn per_file_failure(
    dst_root: &Path,
    relative_path: &str,
    error: eyre::Report,
) -> Result<SinkOutcome> {
    if failure_is_containable(dst_root, relative_path, &error) {
        Ok(SinkOutcome::failed(relative_path, format!("{error:#}")))
    } else {
        Err(error)
    }
}

/// One shard member's write result carrying the verdict already taken on
/// it. Produced by [`classify_shard_member`] in the worker that wrote the
/// member and consumed by [`fold_shard_member_results`], which re-derives
/// nothing.
///
/// cr-pfc3-2: [`failure_is_containable`] probes live destination state,
/// so classifying a shard's members only after every worker has finished
/// judges the destination as it is *then*, not as the failing write saw
/// it. A destination root that dropped mid-shard and reconnected (SMB) or
/// was recreated before the fold had its root-caused member errors
/// reclassified as one file's failure each, and the mirror reported
/// "incomplete" over a destination that had actually died. The verdict
/// therefore travels with the member from the moment the error existed.
pub(super) enum ClassifiedMember<S> {
    /// The member landed: payload bytes plus its writer's observation
    /// sample.
    Written(u64, S),
    /// The member failed and the shared classifier read that failure as
    /// attributable to this member alone: recorded, shard continues.
    Contained(eyre::Report),
    /// The member failed and the shared classifier read that failure as
    /// root-wide or volume-level: session-fatal, exactly as on the
    /// single-file paths.
    Fatal(eyre::Report),
}

/// Take one shard member's verdict *where its failure happened* — inside
/// the worker that wrote it, while the destination is still in the state
/// that write saw (cr-pfc3-2).
///
/// This is the only place a shard member is classified, and it decides
/// through the same [`failure_is_containable`] predicate the single-file
/// paths use, so the shard writers can never drift from them on what is
/// per-file and what is session-fatal.
pub(super) fn classify_shard_member<S>(
    dst_root: &Path,
    relative_path: &str,
    result: Result<(u64, S)>,
) -> ClassifiedMember<S> {
    match result {
        Ok((bytes, sample)) => ClassifiedMember::Written(bytes, sample),
        Err(error) if failure_is_containable(dst_root, relative_path, &error) => {
            ClassifiedMember::Contained(error)
        }
        Err(error) => ClassifiedMember::Fatal(error),
    }
}

/// A pluggable write backend for the transfer pipeline.
///
/// Implementations receive [`PreparedPayload`] items produced by a [`TransferSource`]
/// and write them to a destination (local filesystem, TCP stream, etc.).
#[async_trait]
pub trait TransferSink: Send + Sync {
    /// Write a single prepared payload to the destination.
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome>;

    /// Stream a file payload from a borrowed async reader.
    ///
    /// Used by the receive pipeline so file bytes that arrive on a TCP
    /// wire can be written through the same sink as local copies — no
    /// double-buffering into a `'static` reader. Outbound-only sinks
    /// (e.g. `DataPlaneSink`) inherit the default error implementation.
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        _reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        eyre::bail!(
            "{} does not support write_file_stream (called for {})",
            std::any::type_name::<Self>(),
            header.relative_path
        )
    }

    /// Signal that all payloads have been sent. Flushes buffers, sends terminators, etc.
    /// Default implementation is a no-op.
    async fn finish(&self) -> Result<()> {
        Ok(())
    }

    /// Destination root path (if applicable).
    fn root(&self) -> &Path;
}

// ---------------------------------------------------------------------------
// FsTransferSink — local filesystem writer
// ---------------------------------------------------------------------------

/// Configuration for filesystem sink writes.
#[derive(Debug, Clone)]
pub struct FsSinkConfig {
    pub preserve_times: bool,
    pub dry_run: bool,
    pub checksum: Option<ChecksumType>,
    pub resume: bool,
    /// R58-followup: comparison policy the sink uses when deciding
    /// whether to copy a `PreparedPayload::File`. The diff_planner
    /// upstream already filters by `compare_mode`, but
    /// `write_file_payload` re-checks before copying as a defense
    /// layer; pre-fix it called `file_needs_copy_with_checksum_type`
    /// which only knows SizeMtime + Checksum, so `Force` and
    /// `IgnoreTimes` were silently downgraded to SizeMtime and
    /// dropped at the sink layer. The default `SizeMtime` keeps
    /// pre-fix behavior for callers that haven't migrated.
    pub compare_mode: ComparisonMode,
}

impl Default for FsSinkConfig {
    fn default() -> Self {
        Self {
            preserve_times: true,
            dry_run: false,
            checksum: None,
            resume: false,
            compare_mode: ComparisonMode::SizeMtime,
        }
    }
}

/// Writes files directly to a local filesystem using zero-copy primitives
/// (copy_file_range, sendfile, clonefile, block clone) where available.
pub struct FsTransferSink {
    src_root: PathBuf,
    dst_root: PathBuf,
    /// Canonical form of `dst_root` (or its deepest existing
    /// ancestor) captured once at sink construction time. Every
    /// per-entry write resolves the lexical path under `dst_root`
    /// and then verifies it stays inside `canonical_dst_root`
    /// post-symlink. R46-F3: pre-fix the sink only ran lexical
    /// `safe_join`, so a peer-controlled relative path joined under
    /// a `dst_root/link → /outside` symlink would write outside
    /// the destination root.
    canonical_dst_root: Option<PathBuf>,
    config: FsSinkConfig,
    /// Optional byte-level progress sink. When set,
    /// `write_file_stream` passes it into
    /// `receive_stream_double_buffered` so chunk-granularity
    /// writes report cumulative byte progress against the
    /// daemon's per-transfer counter (c-1a). Unset on the CLI
    /// side; the daemon side sets it via
    /// [`FsTransferSink::with_byte_progress`] from
    /// `ActiveJobGuard::bytes_counter()`.
    byte_progress: Option<ByteProgressSink>,
    /// Separate otp-12 high-volume observer. `None` is the exact normal
    /// sink path: no clocks, per-member timing, or output.
    small_file_probe: Option<BoundSmallFileProbe>,
    /// First failure reason per relative path whose resume-block patch
    /// failed. Such a file must never be finalized: the completion record
    /// truncates and stamps the source mtime, so a partially patched
    /// destination would read as converged on every later compare.
    /// Leaving it unstamped is what makes the re-run resume it
    /// (D-2026-07-09-1). The file is reported failed exactly once, at its
    /// completion record, which also drains the entry. One sink instance
    /// serves every socket of a session's receive plane, so this holds
    /// for a file whose blocks and completion arrive on different
    /// sockets.
    failed_resume_reasons: std::sync::Mutex<std::collections::HashMap<String, String>>,
    /// Successful parent-directory readiness, shared by every receive
    /// worker that owns this session sink. The map lock only protects
    /// lookup/insertion; each parent has its own async once-cell so first
    /// use of distinct directories remains concurrent while simultaneous
    /// files in one directory share a single `create_dir_all` attempt.
    ready_parents:
        std::sync::Mutex<std::collections::HashMap<PathBuf, Arc<tokio::sync::OnceCell<()>>>>,
    /// Test-only proxy for the parent-directory syscall cost. Incremented
    /// immediately before each `create_dir_all` attempt so the sf-3b guard
    /// can pin one readiness check per shared parent without depending on
    /// an OS-specific syscall tracer in CI.
    #[cfg(test)]
    parent_create_attempts: std::sync::atomic::AtomicUsize,
}

impl FsTransferSink {
    pub fn new(src_root: PathBuf, dst_root: PathBuf, config: FsSinkConfig) -> Self {
        // Best-effort canonical root capture. We don't fail
        // construction if canonicalize fails (e.g. dst_root is a
        // not-yet-created path under a deeply unusual filesystem) —
        // instead we leave canonical_dst_root as None and the
        // per-write check degrades to lexical-only with a warn.
        // R46-F3: in the common case (dst_root or its ancestor
        // exists) this captures the canonical form needed for
        // symlink-escape rejection.
        let canonical_dst_root = crate::path_safety::canonical_dest_root(&dst_root).ok();
        Self {
            src_root,
            dst_root,
            canonical_dst_root,
            config,
            byte_progress: None,
            small_file_probe: None,
            failed_resume_reasons: std::sync::Mutex::new(std::collections::HashMap::new()),
            ready_parents: std::sync::Mutex::new(std::collections::HashMap::new()),
            #[cfg(test)]
            parent_create_attempts: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Attach a byte-level progress sink. When set,
    /// `write_file_stream` reports every chunk the data plane
    /// writes against this sink. Used by the daemon side of
    /// remote→remote transfers so `GetState.active[].bytes_completed`
    /// tracks live progress; CLI-side callers omit it.
    pub fn with_byte_progress(mut self, sink: ByteProgressSink) -> Self {
        self.byte_progress = Some(sink);
        self
    }

    pub(crate) fn with_small_file_probe(mut self, probe: Option<BoundSmallFileProbe>) -> Self {
        self.small_file_probe = probe;
        self
    }

    /// R46-F3: lexical resolve + canonical containment check in one
    /// call. Used by every per-entry write site on this sink so a
    /// peer-controlled relative path can't escape the destination
    /// root via a pre-existing symlink. Falls back to lexical-only
    /// (with a warn) if `canonical_dst_root` was None at
    /// construction time — that path remains exposed but is
    /// extremely unusual in practice.
    fn resolve_destination(&self, wire_path: &str) -> Result<PathBuf> {
        match self.canonical_dst_root.as_ref() {
            Some(canonical) => {
                crate::path_safety::safe_join_contained(canonical, &self.dst_root, wire_path)
            }
            None => {
                log::warn!(
                    "FsTransferSink at '{}' has no canonical root; \
                     receive falls back to lexical-only path check \
                     (R46-F3 escape protection unavailable)",
                    self.dst_root.display()
                );
                crate::path_safety::safe_join(&self.dst_root, wire_path)
            }
        }
    }

    /// Hold the first resume-block failure for `relative_path`. Nothing
    /// is reported yet: a resumed file fails once, at its completion
    /// record, so several failed blocks of one file cannot inflate the
    /// failure count.
    fn hold_resume_block_failure(&self, relative_path: &str, error: &eyre::Report) {
        self.failed_resume_reasons
            .lock()
            .expect("failed-resume-reasons lock poisoned")
            .entry(relative_path.to_string())
            .or_insert_with(|| format!("{error:#}"));
    }

    /// Take the held resume-block failure for `relative_path`, if any.
    fn take_resume_block_failure(&self, relative_path: &str) -> Option<String> {
        self.failed_resume_reasons
            .lock()
            .expect("failed-resume-reasons lock poisoned")
            .remove(relative_path)
    }

    async fn create_parent_directory(&self, parent: &Path) -> Result<()> {
        #[cfg(test)]
        self.parent_create_attempts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating directory {}", parent.display()))
    }

    fn parent_readiness(&self, parent: &Path) -> Arc<tokio::sync::OnceCell<()>> {
        Arc::clone(
            self.ready_parents
                .lock()
                .expect("ready-parents lock poisoned")
                .entry(parent.to_path_buf())
                .or_insert_with(|| Arc::new(tokio::sync::OnceCell::new())),
        )
    }

    /// Remove only the readiness generation this caller observed. Another
    /// worker may already have invalidated a stale generation and installed
    /// a successful replacement; a late failure must not evict that one.
    fn invalidate_parent_readiness(
        &self,
        parent: &Path,
        observed: &Arc<tokio::sync::OnceCell<()>>,
    ) {
        let mut ready_parents = self
            .ready_parents
            .lock()
            .expect("ready-parents lock poisoned");
        if ready_parents
            .get(parent)
            .is_some_and(|current| Arc::ptr_eq(current, observed))
        {
            ready_parents.remove(parent);
        }
    }

    async fn ensure_parent_ready(&self, parent: &Path) -> Result<Arc<tokio::sync::OnceCell<()>>> {
        let readiness = self.parent_readiness(parent);
        if let Err(error) = readiness
            .get_or_try_init(|| self.create_parent_directory(parent))
            .await
        {
            self.invalidate_parent_readiness(parent, &readiness);
            return Err(error);
        }
        Ok(readiness)
    }

    async fn create_stream_destination(
        &self,
        dst: &Path,
        windows_metadata: Option<&crate::generated::WindowsFileMetadata>,
    ) -> Result<tokio::fs::File> {
        let parent_readiness = match dst.parent() {
            Some(parent) => Some((parent, self.ensure_parent_ready(parent).await?)),
            None => None,
        };

        if let Err(error) = crate::windows_metadata::prepare_destination(dst, windows_metadata) {
            if let Some((parent, readiness)) = &parent_readiness {
                self.invalidate_parent_readiness(parent, readiness);
            }
            return Err(error);
        }

        match tokio::fs::File::create(dst).await {
            Ok(file) => Ok(file),
            Err(error) => {
                if let Some((parent, readiness)) = &parent_readiness {
                    self.invalidate_parent_readiness(parent, readiness);
                    if error.kind() == std::io::ErrorKind::NotFound {
                        self.ensure_parent_ready(parent).await?;
                        crate::windows_metadata::prepare_destination(dst, windows_metadata)?;
                        return tokio::fs::File::create(dst)
                            .await
                            .with_context(|| format!("creating {}", dst.display()));
                    }
                }
                Err(error).with_context(|| format!("creating {}", dst.display()))
            }
        }
    }

    #[cfg(test)]
    fn parent_create_attempts(&self) -> usize {
        self.parent_create_attempts
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait]
impl TransferSink for FsTransferSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        // Resume payloads need async I/O (file open + seek + write
        // through tokio). Local-source payloads (File / TarShard) stay
        // on a blocking thread so the zero-copy cascade and tar
        // extraction can use std::fs.
        let outcome = match payload {
            PreparedPayload::FileBlock {
                relative_path,
                offset,
                bytes,
            } => {
                // Path safety first and separately: a hostile relative
                // path is a protocol violation, never one file's failure.
                // The block's bytes arrive as a payload rather than an
                // unread wire record, so a contained failure leaves
                // nothing to drain.
                let dst = resolve_resume_destination(
                    &self.dst_root,
                    self.canonical_dst_root.as_deref(),
                    &relative_path,
                    "block-write",
                )?;
                match patch_file_block(&dst, offset, bytes).await {
                    Ok(outcome) => outcome,
                    Err(error)
                        if !failure_is_containable(&self.dst_root, &relative_path, &error) =>
                    {
                        // cr-pfc2-1: a block failing because the volume
                        // refuses writes must not be held and reported as
                        // one file's failure at its completion record —
                        // every block of every file fails the same way.
                        return Err(error);
                    }
                    Err(error) => {
                        self.hold_resume_block_failure(&relative_path, &error);
                        SinkOutcome::default()
                    }
                }
            }
            PreparedPayload::FileBlockComplete {
                relative_path,
                total_size,
                mtime_seconds,
                permissions,
                windows_metadata,
            } => {
                let dst = resolve_resume_destination(
                    &self.dst_root,
                    self.canonical_dst_root.as_deref(),
                    &relative_path,
                    "block-complete",
                )?;
                if let Some(reason) = self.take_resume_block_failure(&relative_path) {
                    // Finalizing here would stamp a stale file as
                    // converged (see `failed_resume_reasons`); the file
                    // fails once, and this is the record that names it.
                    SinkOutcome::failed(&relative_path, reason)
                } else {
                    match finalize_resumed_file(
                        &dst,
                        total_size,
                        mtime_seconds,
                        permissions,
                        windows_metadata,
                    )
                    .await
                    {
                        Ok(outcome) => outcome,
                        Err(error) => per_file_failure(&self.dst_root, &relative_path, error)?,
                    }
                }
            }
            // otp-7b: the composite resume payload is send-side only
            // (DataPlaneSink); the receive pipeline decodes per-block
            // FileBlock/FileBlockComplete, never this shape.
            PreparedPayload::ResumeFile { .. } => {
                eyre::bail!("FsTransferSink does not consume composite ResumeFile payloads")
            }
            PreparedPayload::File(_) | PreparedPayload::TarShard { .. } => {
                let src_root = self.src_root.clone();
                let dst_root = self.dst_root.clone();
                let canonical_dst_root = self.canonical_dst_root.clone();
                let config = self.config.clone();
                let tar_probe = self
                    .small_file_probe
                    .as_ref()
                    .and_then(|probe| match &payload {
                        PreparedPayload::TarShard { headers, .. } => {
                            let shard_id = probe.shard_id(headers);
                            Some((probe.clone(), shard_id, probe.start()))
                        }
                        _ => None,
                    });
                let outcome = tokio::task::spawn_blocking(move || match payload {
                    PreparedPayload::File(header) => write_file_payload(
                        &src_root,
                        &dst_root,
                        canonical_dst_root.as_deref(),
                        &header,
                        &config,
                    ),
                    PreparedPayload::TarShard { headers, data } => {
                        let worker_started = tar_probe.as_ref().map(|_| std::time::Instant::now());
                        let blocking_pool_wait = tar_probe.as_ref().zip(worker_started).map(
                            |((_, _, queued), worker)| worker.saturating_duration_since(*queued),
                        );
                        write_tar_shard_payload(
                            &src_root,
                            &dst_root,
                            canonical_dst_root.as_deref(),
                            &headers,
                            &data,
                            &config,
                            tar_probe.as_ref().zip(blocking_pool_wait).map(
                                |((probe, shard_id, queued), wait)| {
                                    (probe, shard_id.as_str(), *queued, wait)
                                },
                            ),
                        )
                    }
                    _ => unreachable!("outer match guarantees File or TarShard"),
                })
                .await
                .context("sink worker panicked")??;
                outcome
            }
        };
        // c-1b round 2: tar shards and resume blocks land via
        // write_payload, not write_file_stream, so the chunk-
        // granular `receive_stream_double_buffered` hook never
        // fires for them. Report `outcome.bytes_written` here so
        // `GetState.active[].bytes_completed` reflects bytes
        // landed on disk for ALL payload shapes, not just
        // streamed files. Dry-run write paths return
        // `bytes_written: 0` (see `write_file_payload` and
        // `write_tar_shard_payload`'s dry-run early returns), so
        // adding 0 is a no-op for previews — same semantics as
        // `write_file_stream`'s dry-run branch.
        if let Some(bp) = &self.byte_progress {
            bp.report(outcome.bytes_written);
        }
        Ok(outcome)
    }

    /// Stream file bytes from the wire to the destination filesystem
    /// using the same double-buffered helper the send side uses. This
    /// is what makes push and pull receive symmetric on the FsTransferSink.
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        use crate::remote::transfer::data_plane::{
            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
        };

        // R46-F3: lexical resolve + canonical containment check via
        // resolve_destination. Pre-fix this was a bare safe_join,
        // which rejected lexical traversal (`../`) but didn't catch
        // the case where dst_root contained a pre-existing symlink
        // pointing outside (`dst_root/link → /outside`); a peer-
        // controlled relative path `link/file` would then write to
        // `/outside/file`.
        let dst = self
            .resolve_destination(&header.relative_path)
            .with_context(|| format!("validating receive path {:?}", header.relative_path))?;

        // R58-F4: dry-run must be side-effect-free. Drain the wire
        // for protocol-stream alignment, but skip the parent-mkdir
        // and the file write. Pre-fix the parent-mkdir ran before
        // the dry-run check below, so `--dry-run` over a remote
        // transfer would create destination directories.
        if self.config.dry_run {
            let mut sink = tokio::io::sink();
            // Dry-run: drain wire bytes for protocol alignment.
            // Do NOT report against `byte_progress` — by contract
            // dry-run is side-effect-free and these bytes never
            // hit user disk; we don't want a daemon-side bytes_completed
            // counter to advance for an aborted preview.
            receive_stream_double_buffered(
                reader,
                &mut sink,
                header.size,
                RECEIVE_CHUNK_SIZE,
                None,
            )
            .await
            .with_context(|| format!("draining {} (dry-run)", header.relative_path))?;
            return Ok(SinkOutcome::written(1, 0));
        }

        // Peer-supplied metadata shape is a protocol violation, not one
        // file's failure — checked before any filesystem effect so the
        // fatal class stays above the per-file block below.
        crate::windows_metadata::validate_payload(header.windows_metadata.as_ref())
            .with_context(|| format!("validating Windows metadata for {}", header.relative_path))?;

        // Materializing the destination for this one file: mkdir of its
        // own subpath, readonly/stream preparation, create. Each failure
        // is this file's, but the record's bytes are still on the wire —
        // an undrained record would desync the protocol stream, so the
        // record is drained before the failure is contained. A drain
        // failure is transport, and stays fatal.
        let prepared = self
            .create_stream_destination(&dst, header.windows_metadata.as_ref())
            .await;
        let mut file = match prepared {
            Ok(file) => file,
            Err(error)
                if !failure_is_containable(&self.dst_root, &header.relative_path, &error) =>
            {
                // Fatal class: the session ends here, so there is no next
                // record to stay aligned with and nothing to drain.
                return Err(error);
            }
            Err(error) => {
                let mut drain = tokio::io::sink();
                receive_stream_double_buffered(
                    reader,
                    &mut drain,
                    header.size,
                    RECEIVE_CHUNK_SIZE,
                    None,
                )
                .await
                .with_context(|| {
                    format!(
                        "draining {} after its destination could not be opened",
                        header.relative_path
                    )
                })?;
                return per_file_failure(&self.dst_root, &header.relative_path, error);
            }
        };

        let written = {
            use tokio::io::AsyncWriteExt as _;
            // Wire read and disk write are one operation here: a failure
            // leaves an unknown number of record bytes unread, so the
            // stream cannot be resynchronized and this stays fatal.
            receive_stream_double_buffered(
                reader,
                &mut file,
                header.size,
                RECEIVE_CHUNK_SIZE,
                self.byte_progress.as_ref(),
            )
            .await
            .with_context(|| format!("writing {}", dst.display()))?;
            // Flush the tokio File's internal buffer state (does NOT
            // fsync — just ensures user-space buffering is drained
            // before we drop the handle and apply mtime). Without
            // this, set_file_mtime races with deferred writes from
            // tokio's blocking-thread pool: 5/8 of mtimes were
            // observed silently bumped to "now" on the receive side.
            //
            // POST_REVIEW_FIXES §1.1: flush failure is a data-loss
            // signal — the user believes the file is durable when it
            // isn't. Never swallowed; the record is fully consumed by
            // now, so it is this file's failure.
            file.flush()
                .await
                .with_context(|| format!("flushing {}", dst.display()))
        };
        // Handle dropped → kernel close() complete → no further
        // metadata churn from this file. Now safe to set mtime by path.
        drop(file);

        // Intentionally no sync_all: ZFS commits per fsync are
        // multi-second on spinning rust and crater throughput
        // (9.3 → 3.3 Gbps observed). The transfer's durability signal
        // is its END marker plus the OS's own flush; matches rsync's
        // default behavior. Add a config flag if a caller needs sync.

        // Metadata tail, past the last wire byte of the record: every
        // failure below concerns exactly this file.
        let windows_bytes =
            match written.and_then(|()| stamp_streamed_metadata(&dst, header, &self.config)) {
                Ok(windows_bytes) => windows_bytes,
                Err(error) => {
                    // pfc-4 byte-lane reconciliation (the pfc-2 landing
                    // note's owed item). The chunk hook above already
                    // reported this file's payload to the LIVE counter —
                    // exactly `header.size` bytes, since the stream returned
                    // Ok — while the outcome below counts zero for a
                    // contained failure and the summary reports zero with it.
                    // Give those bytes back so the live lane never claims
                    // work the authoritative summary denies. The sibling
                    // containment route above (destination could not be
                    // opened) drains with `None`, so it has nothing to
                    // withdraw.
                    if let Some(bp) = &self.byte_progress {
                        bp.withdraw(header.size);
                    }
                    return per_file_failure(&self.dst_root, &header.relative_path, error);
                }
            };

        Ok(SinkOutcome::written(
            1,
            header.size.saturating_add(windows_bytes),
        ))
    }

    fn root(&self) -> &Path {
        &self.dst_root
    }
}

/// Copy a single file using the zero-copy cascade in `copy::file_copy`.
fn write_file_payload(
    src_root: &Path,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    header: &FileHeader,
    config: &FsSinkConfig,
) -> Result<SinkOutcome> {
    // An empty relative_path means "the root itself" — the enumeration
    // root was a single file (same rule as FsTransferSource::open_file):
    // joining "" can yield a trailing-slash form the OS reads as
    // "descend into", which fails with ENOTDIR on a regular file. The
    // local session route (otp-11) is the first caller to send a
    // file-root File payload through here.
    if header.relative_path.is_empty() {
        return match copy_root_file_payload(src_root, dst_root, header, config) {
            Ok(outcome) => Ok(outcome),
            Err(error) => per_file_failure(dst_root, "", error),
        };
    }
    let src = src_root.join(&header.relative_path);
    // R47-F1: the FsTransferSink::write_payload arm for
    // PreparedPayload::File hit this helper, which previously
    // joined dst_root + header.relative_path lexically. A peer-
    // controlled `link/file` with a pre-existing `dst/link →
    // /outside` symlink would write outside the destination root.
    // Route through the same canonical-containment chokepoint that
    // write_file_stream uses.
    let dst = match canonical_dst_root {
        Some(canonical) => {
            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
                .with_context(|| {
                    format!("validating file payload path {:?}", header.relative_path)
                })?
        }
        None => {
            log::warn!(
                "write_file_payload at '{}' has no canonical root; \
                 falls back to lexical-only path check (R47-F1 \
                 escape protection unavailable)",
                dst_root.display()
            );
            crate::path_safety::safe_join(dst_root, &header.relative_path).with_context(|| {
                format!("validating file payload path {:?}", header.relative_path)
            })?
        }
    };

    // Past the containment check every remaining failure — this file's
    // own parent mkdir, the source read, the copy, its named streams and
    // its attributes — belongs to exactly this file.
    match copy_resolved_file_payload(&src, &dst, header, config) {
        Ok(outcome) => Ok(outcome),
        Err(error) => per_file_failure(dst_root, &header.relative_path, error),
    }
}

/// The file-root identity case of [`write_file_payload`]: `src_root`
/// IS the file and `dst_root` IS the exact target path, so there is
/// nothing to join and nothing to containment-check — the configured
/// root cannot escape itself.
fn copy_root_file_payload(
    src_root: &Path,
    dst_root: &Path,
    header: &FileHeader,
    config: &FsSinkConfig,
) -> Result<SinkOutcome> {
    copy_resolved_file_payload(src_root, dst_root, header, config)
}

/// Shared tail of the File-payload write: dry-run gate, parent mkdir,
/// resume/compare/copy cascade, mtime preservation.
fn copy_resolved_file_payload(
    src: &Path,
    dst: &Path,
    header: &FileHeader,
    config: &FsSinkConfig,
) -> Result<SinkOutcome> {
    // R58-F4: dry-run must be side-effect-free. Bail before the
    // parent-mkdir so a dry-run doesn't create destination
    // directories on disk.
    if config.dry_run {
        return Ok(SinkOutcome::written(1, 0));
    }

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    crate::windows_metadata::validate_payload(header.windows_metadata.as_ref())
        .with_context(|| format!("validating Windows metadata for {}", header.relative_path))?;
    crate::windows_metadata::prepare_destination(dst, header.windows_metadata.as_ref())?;

    let mut did_copy = false;
    if config.resume {
        let outcome = resume_copy_file(src, dst, 0)
            .with_context(|| format!("resume copy {}", header.relative_path))?;
        did_copy = outcome.bytes_transferred > 0;
    } else if crate::copy::file_needs_copy_with_mode(src, dst, config.compare_mode)? {
        let sizer = BufferSizer::default();
        copy_file(src, dst, &sizer, false)
            .with_context(|| format!("copy {}", header.relative_path))?;
        did_copy = true;
    }

    // Windows CopyFileEx preserves the source attributes, so a fresh copy of
    // a read-only source makes the destination read-only again after the
    // pre-copy preparation above. Clear that bit once more before replacing
    // named streams; apply_attributes below restores the exact source mask.
    if did_copy {
        crate::windows_metadata::prepare_destination(dst, header.windows_metadata.as_ref())?;
    }

    let windows_bytes =
        crate::windows_metadata::replace_streams(dst, header.windows_metadata.as_ref())?;

    if config.preserve_times {
        let fallback =
            (header.mtime_seconds > 0).then(|| FileTime::from_unix_time(header.mtime_seconds, 0));
        if let Some(ft) = source_file_mtime(src, fallback) {
            if let Err(e) = filetime::set_file_mtime(dst, ft) {
                log::warn!("set mtime on {}: {}", dst.display(), e);
            }
        }
    }
    crate::windows_metadata::apply_attributes(dst, header.windows_metadata.as_ref())?;

    Ok(SinkOutcome::written(
        1,
        (if did_copy { header.size } else { 0 }).saturating_add(windows_bytes),
    ))
}

/// Stamp the metadata tail of a streamed receive: named streams, mtime,
/// permissions, attributes. Returns the named-stream bytes applied.
/// Split out of [`FsTransferSink::write_file_stream`] so the tail — past
/// the record's last wire byte, therefore attributable to one file — is
/// classified in one place.
fn stamp_streamed_metadata(dst: &Path, header: &FileHeader, config: &FsSinkConfig) -> Result<u64> {
    let windows_bytes =
        crate::windows_metadata::replace_streams(dst, header.windows_metadata.as_ref())?;

    if config.preserve_times && header.mtime_seconds > 0 {
        let ft = FileTime::from_unix_time(header.mtime_seconds, 0);
        // Best-effort: cross-fs, root-owned, or ACL-protected
        // destinations can refuse mtime updates. Surface via
        // `log::warn!` so the failure is visible without making
        // it a hard transfer error. POST_REVIEW_FIXES §1.1.
        if let Err(e) = filetime::set_file_mtime(dst, ft) {
            log::warn!("set mtime on {}: {}", dst.display(), e);
        }
    }

    // Permissions arrive on the wire (Unix mode bits). Apply best-
    // effort; ignore failures (cross-fs, root-owned dst, etc.).
    #[cfg(unix)]
    if header.permissions != 0 {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) =
            std::fs::set_permissions(dst, std::fs::Permissions::from_mode(header.permissions))
        {
            log::warn!("set permissions on {}: {}", dst.display(), e);
        }
    }
    #[cfg(not(unix))]
    let _ = header.permissions;
    crate::windows_metadata::apply_attributes(dst, header.windows_metadata.as_ref())?;
    Ok(windows_bytes)
}

/// Read the local source timestamp at apply time so local copies retain the
/// precision that the second-granularity wire header cannot represent. The
/// header value remains a fallback when the source timestamp cannot be read.
fn source_file_mtime(source: &Path, fallback: Option<FileTime>) -> Option<FileTime> {
    std::fs::metadata(source)
        .and_then(|metadata| metadata.modified())
        .map(FileTime::from_system_time)
        .ok()
        .or(fallback)
}

/// Replace wire-derived tar timestamps with local source timestamps. An empty
/// source root is the transfer-session convention for a wire receive, which
/// must keep using the timestamp carried by its header.
fn restamp_local_tar_mtimes(src_root: &Path, files: &mut [super::tar_safety::ExtractedFile]) {
    if src_root.as_os_str().is_empty() {
        return;
    }
    for file in files {
        let source = if file.rel.is_empty() {
            src_root.to_path_buf()
        } else {
            src_root.join(&file.rel)
        };
        file.mtime = source_file_mtime(&source, file.mtime);
    }
}

/// Extract an in-memory tar shard to the destination directory.
fn write_tar_shard_payload(
    src_root: &Path,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    headers: &[FileHeader],
    data: &[u8],
    config: &FsSinkConfig,
    probe: Option<(
        &BoundSmallFileProbe,
        &str,
        std::time::Instant,
        std::time::Duration,
    )>,
) -> Result<SinkOutcome> {
    if config.dry_run {
        return Ok(SinkOutcome::written(headers.len(), 0));
    }

    // Two-phase extraction:
    //   1. Validate + parse the tar serially via the shared
    //      `tar_safety` helper. Tar is a sequential format — entries
    //      can't be read in parallel out of one Archive — and this
    //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
    //   2. Write files to disk in parallel via rayon. Inode creation
    //      and write are the bottleneck for many-small-files shards;
    //      4–8 worker cores can saturate ZFS' inode pipeline.
    //
    // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
    // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
    // toward CPU-or-fs limits.
    use rayon::prelude::*;

    use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};

    let parse_started = probe.map(|_| std::time::Instant::now());
    let opts = TarShardExtractOptions::default();
    let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;

    // R47-F1: tar shards arriving on FsTransferSink::write_payload
    // (push-receive on the daemon flows through here too) only had
    // lexical safe_join inside safe_extract_tar_shard. A pre-
    // existing dst/link → /outside escape symlink would let an
    // entry path like `link/victim` write through the symlink.
    // Verify each extracted entry's destination against the
    // canonical root before writing.
    if let Some(canonical) = canonical_dst_root {
        for f in &extracted {
            crate::path_safety::verify_contained(canonical, &f.dest_path).with_context(|| {
                format!("tar shard entry {:?} escapes destination root", f.dest_path)
            })?;
        }
    } else {
        log::warn!(
            "write_tar_shard_payload at '{}' has no canonical root; \
             tar-shard receive falls back to lexical-only path \
             checks (R47-F1 escape protection unavailable)",
            dst_root.display()
        );
    }

    // Honor the sink's preserve_times toggle by stripping mtimes that
    // the helper would otherwise apply. Permissions are best-effort
    // either way (matches the historical FsTransferSink policy).
    if config.preserve_times {
        restamp_local_tar_mtimes(src_root, &mut extracted);
    } else {
        for f in &mut extracted {
            f.mtime = None;
        }
    }

    let parse_validate = parse_started.map(|started| started.elapsed());

    // Write in parallel. Each closure does its own create_dir_all +
    // write + best-effort mtime/permission application — same policy as
    // `tar_safety::write_extracted_file` but inlined so we can return
    // per-file byte counts for the SinkOutcome. Each member carries its
    // own result: a member the destination refuses is that member's
    // failure, not the shard's (audit-17). Each worker also takes its own
    // member's verdict through the shared classifier before handing the
    // member on (cr-pfc3-2) — inside the closure is the only place the
    // destination is still in the state that write saw.
    if probe.is_none() {
        let results: Vec<(String, ClassifiedMember<()>)> = extracted
            .into_par_iter()
            .map(|f: ExtractedFile| {
                let written = write_shard_member(&f).map(|bytes| (bytes, ()));
                let member = classify_shard_member(dst_root, &f.rel, written);
                (f.rel, member)
            })
            .collect();
        return fold_shard_member_results(results, |()| {});
    }

    type MemberSample = (
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
    );
    let members_started = probe.map(|_| std::time::Instant::now());
    let results: Vec<(String, ClassifiedMember<MemberSample>)> = extracted
        .into_par_iter()
        .map(|f: ExtractedFile| {
            let written = (|| -> Result<(u64, MemberSample)> {
                use std::io::Write as _;

                let total_started = std::time::Instant::now();
                let mkdir_started = std::time::Instant::now();
                if let Some(parent) = f.dest_path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create dir {}", parent.display()))?;
                }
                let mkdir = mkdir_started.elapsed();

                crate::windows_metadata::prepare_destination(
                    &f.dest_path,
                    f.windows_metadata.as_ref(),
                )?;

                let open_started = std::time::Instant::now();
                let mut file = std::fs::File::create(&f.dest_path)
                    .with_context(|| format!("open {}", f.dest_path.display()))?;
                let open = open_started.elapsed();
                let write_started = std::time::Instant::now();
                file.write_all(&f.contents)
                    .with_context(|| format!("write {}", f.dest_path.display()))?;
                let write = write_started.elapsed();
                let close_started = std::time::Instant::now();
                drop(file);
                let close = close_started.elapsed();

                let metadata_started = std::time::Instant::now();
                let windows_bytes = stamp_shard_member_metadata(&f)?;
                let metadata = metadata_started.elapsed();
                Ok((
                    f.size.saturating_add(windows_bytes),
                    (mkdir, open, write, close, metadata, total_started.elapsed()),
                ))
            })();
            // Same worker-time classification as the probe-less path
            // above (cr-pfc3-2): this closure is where the error exists.
            let member = classify_shard_member(dst_root, &f.rel, written);
            (f.rel, member)
        })
        .collect();
    let member_parallel_wall = members_started.map(|started| started.elapsed());

    let mut member_timings = MemberTimingReport::default();
    let outcome =
        fold_shard_member_results(results, |(mkdir, open, write, close, metadata, total)| {
            member_timings.record(mkdir, open, write, close, metadata, total);
        })?;

    if let Some((probe, shard_id, started, blocking_pool_wait)) = probe {
        probe.note_shard_sink(
            shard_id.to_owned(),
            probe.carrier(),
            headers.len(),
            data.len() as u64,
            started,
            blocking_pool_wait,
            parse_validate.unwrap_or_default(),
            member_parallel_wall.unwrap_or_default(),
            started.elapsed(),
            member_timings,
        );
    }

    Ok(outcome)
}

/// Write one tar-shard member: its own parent directory, its contents,
/// then the metadata tail. Every failure here concerns exactly this
/// member — the shard's structural parse and its containment checks
/// already ran, serially, before any member was written.
fn write_shard_member(file: &super::tar_safety::ExtractedFile) -> Result<u64> {
    if let Some(parent) = file.dest_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    crate::windows_metadata::prepare_destination(&file.dest_path, file.windows_metadata.as_ref())?;
    std::fs::write(&file.dest_path, &file.contents)
        .with_context(|| format!("write {}", file.dest_path.display()))?;
    let windows_bytes = stamp_shard_member_metadata(file)?;
    Ok(file.size.saturating_add(windows_bytes))
}

/// Stamp a written shard member's metadata tail: named streams, mtime,
/// Unix permissions, attributes. Returns the named-stream bytes applied.
/// Shared by both rayon writers so the probed path measures exactly the
/// work the probe-less path does.
fn stamp_shard_member_metadata(file: &super::tar_safety::ExtractedFile) -> Result<u64> {
    let windows_bytes =
        crate::windows_metadata::replace_streams(&file.dest_path, file.windows_metadata.as_ref())?;
    if let Some(ft) = file.mtime {
        if let Err(e) = filetime::set_file_mtime(&file.dest_path, ft) {
            log::warn!("set mtime on {}: {}", file.dest_path.display(), e);
        }
    }
    #[cfg(unix)]
    if let Some(perms) = file.permissions {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) =
            std::fs::set_permissions(&file.dest_path, std::fs::Permissions::from_mode(perms))
        {
            log::warn!("set permissions on {}: {}", file.dest_path.display(), e);
        }
    }
    crate::windows_metadata::apply_attributes(&file.dest_path, file.windows_metadata.as_ref())?;
    Ok(windows_bytes)
}

/// Fold one shard's already-classified member results into that shard's
/// outcome.
///
/// A member failure [`classify_shard_member`] read as that member's own
/// is recorded and the shard keeps every other member — audit-17: one
/// filename the destination filesystem rejected used to abort an ~88k-entry
/// copy from inside the parallel-write closure. A failure the cr-pfc2-1 /
/// cr-pfc3-1 classifier read as root-wide or volume-level returns `Err`
/// and stays session-fatal, exactly as on the single-file paths;
/// structural tar and containment failures never reach here at all,
/// having already short-circuited the whole shard above.
///
/// This fold takes no destination root on purpose (cr-pfc3-2): every
/// verdict was decided in the worker, at the moment its error existed,
/// and re-deriving one here would judge the destination as it is after
/// all the workers finished. With no root in hand the deferred
/// classification cannot be reintroduced.
///
/// `record_sample` receives each landed member's observation payload
/// (`()` where nothing is observed), so the probed writer folds its
/// timings through the same pass.
pub(super) fn fold_shard_member_results<S>(
    results: Vec<(String, ClassifiedMember<S>)>,
    mut record_sample: impl FnMut(S),
) -> Result<SinkOutcome> {
    let mut outcome = SinkOutcome::default();
    for (relative_path, member) in results {
        match member {
            ClassifiedMember::Written(bytes, sample) => {
                outcome.files_written += 1;
                outcome.bytes_written = outcome.bytes_written.saturating_add(bytes);
                record_sample(sample);
            }
            ClassifiedMember::Fatal(error) => return Err(error),
            ClassifiedMember::Contained(error) => {
                outcome.record_failure(relative_path, format!("{error:#}"))
            }
        }
    }
    Ok(outcome)
}

/// Resume protocol: resolve one resume record's destination. Kept apart
/// from the patch and finalize helpers because a path-safety failure is a
/// protocol violation that must stay session-fatal, while the write it
/// guards is one file's business.
fn resolve_resume_destination(
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    relative_path: &str,
    record: &str,
) -> Result<PathBuf> {
    // R46-F3: contained resolve when canonical root is available.
    match canonical_dst_root {
        Some(canonical) => {
            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
                .with_context(|| format!("validating {record} path {relative_path:?}"))
        }
        None => crate::path_safety::safe_join(dst_root, relative_path)
            .with_context(|| format!("validating {record} path {relative_path:?}")),
    }
}

/// Resume protocol: overwrite a block of an existing file at the given offset.
async fn patch_file_block(dst: &Path, offset: u64, bytes: Vec<u8>) -> Result<SinkOutcome> {
    use tokio::io::{AsyncSeekExt, AsyncWriteExt};

    let bytes_len = bytes.len() as u64;
    // Resume blocks patch existing files at offset; we want to create
    // if missing but never truncate (subsequent block records share
    // the file).
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(dst)
        .await
        .with_context(|| format!("opening {} for block write", dst.display()))?;
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .with_context(|| format!("seeking {} to offset {}", dst.display(), offset))?;
    file.write_all(&bytes)
        .await
        .with_context(|| format!("writing block to {}", dst.display()))?;
    // tokio::fs::File buffers writes and performs them on the blocking
    // pool in the background — `write_all` returning does NOT mean the
    // bytes reached the OS. Without this flush an acknowledged block can
    // land arbitrarily late (or race the finalization truncate, which
    // runs on a separate handle) — observed as the otp-7b-2 flake where
    // a faulted session's already-applied block was missing from the
    // partial under full-suite load. Flush before reporting the write
    // done, so "record applied" means the OS has the bytes.
    file.flush()
        .await
        .with_context(|| format!("flushing block write to {}", dst.display()))?;
    // Resume blocks patch in-place; finalization counts the file.
    Ok(SinkOutcome::written(0, bytes_len))
}

/// Resume protocol: finalize a resumed file by truncating to total_size,
/// then stamp mtime + perms from the wire. The mtime stamp is what makes
/// the "mtime touched, content identical" mirror case correct — block-hash
/// compare sends zero blocks, but BLOCK_COMPLETE still updates the dest
/// mtime to match the source.
async fn finalize_resumed_file(
    dst: &Path,
    total_size: u64,
    mtime_seconds: i64,
    permissions: u32,
    windows_metadata: Option<crate::generated::WindowsFileMetadata>,
) -> Result<SinkOutcome> {
    crate::windows_metadata::prepare_destination(dst, windows_metadata.as_ref())?;
    {
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(dst)
            .await
            .with_context(|| format!("opening {} for truncation", dst.display()))?;
        file.set_len(total_size)
            .await
            .with_context(|| format!("truncating {} to {}", dst.display(), total_size))?;
        file.sync_all()
            .await
            .with_context(|| format!("syncing {}", dst.display()))?;
    }
    // Stamp mtime + perms after the file handle is closed (same race
    // dance as write_file_stream — see commit 946bd77).
    let windows_bytes = crate::windows_metadata::replace_streams(dst, windows_metadata.as_ref())?;
    if mtime_seconds > 0 {
        let ft = FileTime::from_unix_time(mtime_seconds, 0);
        if let Err(e) = filetime::set_file_mtime(dst, ft) {
            log::warn!("set mtime on {}: {}", dst.display(), e);
        }
    }
    #[cfg(unix)]
    if permissions != 0 {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(dst, std::fs::Permissions::from_mode(permissions))
        {
            log::warn!("set permissions on {}: {}", dst.display(), e);
        }
    }
    #[cfg(not(unix))]
    let _ = permissions;
    crate::windows_metadata::apply_attributes(dst, windows_metadata.as_ref())?;
    Ok(SinkOutcome::written(1, windows_bytes))
}

// ---------------------------------------------------------------------------
// DataPlaneSink — TCP data plane writer
// ---------------------------------------------------------------------------

/// Writes payloads to a remote daemon via the TCP data plane binary protocol.
///
/// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
/// transfers, the pipeline executor creates multiple DataPlaneSink instances.
///
/// Its outcomes describe TRANSMISSION, not landing: this end sends bytes and
/// cannot know whether the destination wrote them, so a clean outcome here
/// carries the payload's planned sizes and never a `FileFailure`. Only the
/// destination scores per-file failures, and the initiator learns them from
/// the closing `TransferSummary` — where `source_send_half` reconciles this
/// end's optimistic file and byte lanes (pfc-4, cr-pfc2-2).
pub struct DataPlaneSink<P: Probe = NoProbe> {
    session: tokio::sync::Mutex<DataPlaneSession<P>>,
    source: Arc<dyn TransferSource>,
    dst_root: PathBuf,
}

impl<P: Probe> DataPlaneSink<P> {
    pub fn new(
        session: DataPlaneSession<P>,
        source: Arc<dyn TransferSource>,
        dst_root: PathBuf,
    ) -> Self {
        Self {
            session: tokio::sync::Mutex::new(session),
            source,
            dst_root,
        }
    }
}

#[async_trait]
impl<P: Probe> TransferSink for DataPlaneSink<P> {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        let mut session = self.session.lock().await;
        match payload {
            PreparedPayload::File(header) => {
                let size = header
                    .size
                    .saturating_add(crate::windows_metadata::payload_bytes(&header));
                // otp-7b-2: name the file structurally on failure, so a
                // mid-record fault reaches the end-of-operation summary.
                session
                    .send_file(self.source.clone(), &header)
                    .await
                    .with_context(|| format!("sending {}", header.relative_path))
                    .map_err(|e| {
                        e.wrap_err(crate::remote::transfer::faulted_path::FaultedPath(
                            header.relative_path.clone(),
                        ))
                    })?;
                Ok(SinkOutcome::written(1, size))
            }
            PreparedPayload::TarShard { headers, data } => {
                let bytes: u64 = headers
                    .iter()
                    .map(|header| {
                        header
                            .size
                            .saturating_add(crate::windows_metadata::payload_bytes(header))
                    })
                    .sum();
                let count = headers.len();
                session
                    .send_prepared_tar_shard(headers, &data)
                    .await
                    .context("sending tar shard")?;
                Ok(SinkOutcome::written(count, bytes))
            }
            // Resume payloads can't be relayed without a reverse-resume
            // protocol on the next hop. Reject explicitly.
            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
                eyre::bail!("DataPlaneSink does not relay resume-block payloads")
            }
            // otp-7b: one resume-flagged file's whole block phase. The
            // session lock is held across the record, so every BLOCK and
            // the closing BLOCK_COMPLETE ride THIS socket in order —
            // the same strict serialization the in-stream carrier gets
            // from its single control lane. The complete record carries
            // mtime+perms from the manifest header so a zero-block
            // resume still stamps metadata at the destination.
            PreparedPayload::ResumeFile {
                header,
                block_size,
                dest_hashes,
            } => {
                use crate::remote::transfer::resume_diff::{ResumeBlockDiff, ResumeDiffEvent};
                let path = header.relative_path.clone();
                let record = async {
                    // codex otp-7b-1 F1: a mostly-matching scan is a
                    // long SILENT read+hash — arm keepalive ticks well
                    // inside the receiver's stall window and answer each
                    // with a zero-length BLOCK (a no-op in-place write),
                    // so a healthy scan never reads as a stalled peer.
                    let mut diff = ResumeBlockDiff::open(
                        &self.source,
                        &header,
                        block_size as usize,
                        dest_hashes,
                    )
                    .await?
                    .with_keepalive(
                        crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT / 3,
                    );
                    let mut bytes_written: u64 = 0;
                    while let Some(event) = diff.next_event().await? {
                        match event {
                            ResumeDiffEvent::Stale { offset, bytes } => {
                                session
                                    .send_block(&header.relative_path, offset, bytes)
                                    .await
                                    .context("sending resume block")?;
                                bytes_written += bytes.len() as u64;
                            }
                            ResumeDiffEvent::KeepAlive { offset } => {
                                session
                                    .send_block(&header.relative_path, offset, &[])
                                    .await
                                    .context("sending resume keepalive block")?;
                            }
                        }
                    }
                    session
                        .send_block_complete(
                            &header.relative_path,
                            header.size,
                            header.mtime_seconds,
                            header.permissions,
                            header.windows_metadata.as_ref(),
                        )
                        .await
                        .context("sending resume block complete")?;
                    Ok(SinkOutcome::written(
                        1,
                        bytes_written
                            .saturating_add(crate::windows_metadata::payload_bytes(&header)),
                    ))
                }
                .await;
                // otp-7b-2: any failure inside the record names its file
                // structurally (the end-of-operation summary's identity).
                record.map_err(|e: eyre::Report| {
                    e.wrap_err(crate::remote::transfer::faulted_path::FaultedPath(path))
                })
            }
        }
    }

    /// Relay case: bytes arrive on `reader` (e.g. from a DataPlaneSource
    /// during a remote→remote transfer) and forward to the next hop.
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        let size = header.size;
        let mut session = self.session.lock().await;
        session
            .send_file_from_reader(header, reader)
            .await
            .with_context(|| format!("relaying {}", header.relative_path))?;
        Ok(SinkOutcome::written(
            1,
            size.saturating_add(crate::windows_metadata::payload_bytes(header)),
        ))
    }

    async fn finish(&self) -> Result<()> {
        let mut session = self.session.lock().await;
        session.finish().await
    }

    fn root(&self) -> &Path {
        &self.dst_root
    }
}

// ---------------------------------------------------------------------------
// NullSink — discard data, count bytes (for benchmarking)
// ---------------------------------------------------------------------------

/// Discards all payload data, counting files and bytes.
///
/// Useful for benchmarking source + network throughput without destination
/// I/O as a bottleneck. The pipeline still prepares payloads (reading source
/// files, building tar shards) so this measures everything except the write.
pub struct NullSink {
    label: PathBuf,
}

impl Default for NullSink {
    fn default() -> Self {
        Self {
            label: PathBuf::from("/dev/null"),
        }
    }
}

impl NullSink {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TransferSink for NullSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        match payload {
            PreparedPayload::File(header) => Ok(SinkOutcome::written(
                1,
                header
                    .size
                    .saturating_add(crate::windows_metadata::payload_bytes(&header)),
            )),
            PreparedPayload::TarShard { headers, data } => Ok(SinkOutcome::written(
                headers.len(),
                (data.len() as u64).saturating_add(
                    headers
                        .iter()
                        .map(crate::windows_metadata::payload_bytes)
                        .sum(),
                ),
            )),
            PreparedPayload::FileBlock { bytes, .. } => {
                Ok(SinkOutcome::written(0, bytes.len() as u64))
            }
            PreparedPayload::FileBlockComplete { .. } => Ok(SinkOutcome::default()),
            // Send-side composite (otp-7b); the receive path this sink
            // benchmarks never produces it.
            PreparedPayload::ResumeFile { .. } => {
                eyre::bail!("NullSink does not consume composite ResumeFile payloads")
            }
        }
    }

    /// Drain the wire so the protocol stream stays aligned, then count
    /// the bytes. Lets `--null` benchmark the receive path end-to-end
    /// without paying for disk writes.
    async fn write_file_stream(
        &self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        use crate::remote::transfer::data_plane::{
            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
        };
        let mut sink = tokio::io::sink();
        // --null benchmark: bytes never land on user disk; do
        // not advance a daemon-side progress counter for these
        // drains. Same reasoning as the dry-run path on
        // FsTransferSink.
        let n = receive_stream_double_buffered(
            reader,
            &mut sink,
            header.size,
            RECEIVE_CHUNK_SIZE,
            None,
        )
        .await
        .with_context(|| format!("draining {} (null sink)", header.relative_path))?;
        Ok(SinkOutcome::written(1, n))
    }

    fn root(&self) -> &Path {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_file_header(rel: &str, size: u64) -> FileHeader {
        FileHeader {
            relative_path: rel.to_string(),
            size,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: Vec::new(),
            windows_metadata: None,
        }
    }

    /// otp-11a: a file-root File payload (empty relative_path — the
    /// enumeration root was a single file) writes dst_root itself;
    /// the joins would otherwise produce trailing-slash paths that
    /// fail ENOTDIR on a regular file. The local carrier is the only
    /// producer of this shape.
    #[test]
    fn file_root_payload_copies_root_to_root() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");
        std::fs::write(&src, b"root payload").unwrap();
        let header = make_file_header("", b"root payload".len() as u64);
        let outcome =
            write_file_payload(&src, &dst, None, &header, &FsSinkConfig::default()).unwrap();
        assert_eq!(outcome.files_written, 1);
        assert_eq!(std::fs::read(&dst).unwrap(), b"root payload");
    }

    #[test]
    fn source_mtime_keeps_subsecond_precision_over_wire_fallback() {
        let tmp = tempdir().unwrap();
        let source = tmp.path().join("source.bin");
        std::fs::write(&source, b"x").unwrap();
        let requested = FileTime::from_unix_time(1_700_000_000, 123_456_700);
        filetime::set_file_mtime(&source, requested).unwrap();
        let expected = FileTime::from_last_modification_time(&std::fs::metadata(&source).unwrap());
        assert_ne!(expected.nanoseconds(), 0, "fixture lost sub-second mtime");

        let actual = source_file_mtime(
            &source,
            Some(FileTime::from_unix_time(expected.unix_seconds(), 0)),
        )
        .unwrap();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn fs_sink_copies_file() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let content = b"hello world";
        std::fs::write(src.join("file.txt"), content).unwrap();

        let sink = FsTransferSink::new(
            src.clone(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("file.txt", content.len() as u64);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, content.len() as u64);
        assert_eq!(std::fs::read(dst.join("file.txt")).unwrap(), content);
    }

    #[tokio::test]
    async fn fs_sink_dry_run_does_not_write() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        std::fs::write(src.join("file.txt"), b"data").unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: true,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("file.txt", 4);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 0);
        assert!(!dst.join("file.txt").exists());
    }

    /// R58-F4 regression: dry-run for `write_payload` must NOT
    /// create destination subdirectories. Pre-fix `write_file_payload`
    /// mkdir'd `dst/sub/` for a header `sub/file.txt` before
    /// returning from the dry-run check.
    #[tokio::test]
    async fn fs_sink_dry_run_does_not_create_destination_dirs() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub/file.txt"), b"data").unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: true,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("sub/file.txt", 4);
        let _ = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert!(
            !dst.join("sub").exists(),
            "dry-run must not create destination subdirectories \
             (R58-F4 — pre-fix mkdir ran before the dry-run check)"
        );
    }

    /// R58-F4 regression for the streaming receive path. `write_file_stream`
    /// is used by remote pull receive on the CLI side and by daemon push
    /// receive — the pre-fix create_dir_all ran above the dry-run
    /// short-circuit on both.
    #[tokio::test]
    async fn fs_sink_dry_run_write_file_stream_does_not_create_dirs() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: true,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("nested/dir/file.txt", 4);
        let mut reader: &[u8] = b"data";
        let outcome = sink.write_file_stream(&header, &mut reader).await.unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 0);
        assert!(
            !dst.join("nested").exists(),
            "dry-run streaming receive must not create destination dirs"
        );
    }

    /// sf-3b: all receive workers share one sink, so concurrent files in
    /// one directory should pay for parent readiness once per session.
    /// The counter is a portable proxy for the `mkdir` syscall measured by
    /// sf-3a; it keeps this regression guard independent of `strace`.
    #[tokio::test]
    async fn fs_sink_prepares_a_shared_parent_once() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        let sink = Arc::new(FsTransferSink::new(
            tmp.path().join("src"),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        ));

        let writes = (0..16).map(|index| {
            let sink = Arc::clone(&sink);
            async move {
                let header = make_file_header(&format!("shared/file-{index}.txt"), 1);
                let mut reader: &[u8] = b"x";
                sink.write_file_stream(&header, &mut reader).await.unwrap();
            }
        });
        futures::future::join_all(writes).await;

        assert_eq!(sink.parent_create_attempts(), 1);
        for index in 0..16 {
            assert_eq!(
                std::fs::read(dst.join(format!("shared/file-{index}.txt"))).unwrap(),
                b"x"
            );
        }
    }

    /// A successful readiness observation is only a cache, not authority
    /// over later filesystem state. If the directory disappears during a
    /// session, the missing-parent create failure evicts that generation
    /// and retries after recreating the parent.
    #[tokio::test]
    async fn fs_sink_recreates_a_cached_parent_after_removal() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        let sink = FsTransferSink::new(
            tmp.path().join("src"),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let first = make_file_header("shared/first.txt", 1);
        let mut first_reader: &[u8] = b"a";
        sink.write_file_stream(&first, &mut first_reader)
            .await
            .unwrap();
        std::fs::remove_dir_all(dst.join("shared")).unwrap();

        let second = make_file_header("shared/second.txt", 1);
        let mut second_reader: &[u8] = b"b";
        let outcome = sink
            .write_file_stream(&second, &mut second_reader)
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(sink.parent_create_attempts(), 2);
        assert_eq!(std::fs::read(dst.join("shared/second.txt")).unwrap(), b"b");
    }

    #[tokio::test]
    async fn fs_sink_skips_unchanged_file() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let content = b"identical content";
        std::fs::write(src.join("same.txt"), content).unwrap();
        std::fs::write(dst.join("same.txt"), content).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst,
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("same.txt", content.len() as u64);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 0); // skipped — no copy needed
    }

    #[tokio::test]
    async fn fs_sink_extracts_tar_shard() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();

        // Build a tar archive in memory
        let mut builder = tar::Builder::new(Vec::new());
        let content_a = b"file a content";
        let content_b = b"file b content";

        let mut header_a = tar::Header::new_gnu();
        header_a.set_size(content_a.len() as u64);
        header_a.set_mode(0o644);
        header_a.set_cksum();
        builder
            .append_data(&mut header_a, "a.txt", &content_a[..])
            .unwrap();

        let mut header_b = tar::Header::new_gnu();
        header_b.set_size(content_b.len() as u64);
        header_b.set_mode(0o644);
        header_b.set_cksum();
        builder
            .append_data(&mut header_b, "sub/b.txt", &content_b[..])
            .unwrap();

        let tar_data = builder.into_inner().unwrap();

        let headers = vec![
            make_file_header("a.txt", content_a.len() as u64),
            make_file_header("sub/b.txt", content_b.len() as u64),
        ];

        // Use a dummy src_root (not used for tar shards)
        let sink = FsTransferSink::new(
            tmp.path().to_path_buf(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let outcome = sink
            .write_payload(PreparedPayload::TarShard {
                headers,
                data: tar_data,
            })
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 2);
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), content_a);
        assert_eq!(std::fs::read(dst.join("sub/b.txt")).unwrap(), content_b);
    }

    #[tokio::test]
    async fn local_tar_shard_preserves_source_subsecond_mtime() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let content = b"timestamped tar member";
        let source = src.join("member.bin");
        std::fs::write(&source, content).unwrap();
        filetime::set_file_mtime(
            &source,
            FileTime::from_unix_time(1_700_000_000, 123_456_700),
        )
        .unwrap();
        let source_time =
            FileTime::from_last_modification_time(&std::fs::metadata(&source).unwrap());
        assert_ne!(
            source_time.nanoseconds(),
            0,
            "fixture lost sub-second mtime"
        );

        let mut builder = tar::Builder::new(Vec::new());
        let mut tar_header = tar::Header::new_gnu();
        tar_header.set_size(content.len() as u64);
        tar_header.set_mode(0o644);
        tar_header.set_cksum();
        builder
            .append_data(&mut tar_header, "member.bin", &content[..])
            .unwrap();
        let data = builder.into_inner().unwrap();

        let mut file_header = make_file_header("member.bin", content.len() as u64);
        file_header.mtime_seconds = source_time.unix_seconds();
        let sink = FsTransferSink::new(src, dst.clone(), FsSinkConfig::default());
        sink.write_payload(PreparedPayload::TarShard {
            headers: vec![file_header],
            data,
        })
        .await
        .unwrap();

        let destination_time = FileTime::from_last_modification_time(
            &std::fs::metadata(dst.join("member.bin")).unwrap(),
        );
        assert_eq!(destination_time, source_time);
    }

    #[tokio::test]
    async fn fs_sink_creates_nested_directories() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(src.join("a/b/c")).unwrap();

        let content = b"deep file";
        std::fs::write(src.join("a/b/c/deep.txt"), content).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("a/b/c/deep.txt", content.len() as u64);
        sink.write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(std::fs::read(dst.join("a/b/c/deep.txt")).unwrap(), content);
    }

    #[tokio::test]
    async fn null_sink_counts_file() {
        let sink = NullSink::new();
        let header = make_file_header("test.bin", 1024);
        let outcome = sink
            .write_payload(PreparedPayload::File(header))
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 1024);
    }

    #[tokio::test]
    async fn null_sink_counts_tar_shard() {
        let sink = NullSink::new();
        let headers = vec![
            make_file_header("a.txt", 100),
            make_file_header("b.txt", 200),
            make_file_header("c.txt", 300),
        ];
        let data = vec![0u8; 4096]; // fake tar data

        let outcome = sink
            .write_payload(PreparedPayload::TarShard { headers, data })
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 3);
        assert_eq!(outcome.bytes_written, 4096);
    }

    #[tokio::test]
    async fn null_sink_root_is_dev_null() {
        let sink = NullSink::new();
        assert_eq!(sink.root(), Path::new("/dev/null"));
    }

    // ─── Path-safety end-to-end (F1) ──────────────────────────────────
    //
    // The shared `path_safety` module has its own unit tests covering the
    // validator's surface. These tests exercise the FsTransferSink end of
    // the chain to confirm a malicious wire path is rejected before any
    // filesystem write happens. They protect against future regressions
    // where a sink-level call site bypasses `safe_join`.

    async fn assert_sink_rejects(rel: &str) {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );
        let header = make_file_header(rel, 4);
        // Use write_file_stream so we exercise the sink.rs:218 site that
        // F1 hardens. An empty reader is fine — validation happens before
        // any byte is consumed.
        let mut empty: &[u8] = b"";
        let result = sink.write_file_stream(&header, &mut empty).await;
        assert!(
            result.is_err(),
            "expected rejection for malicious wire path {:?}, but got Ok",
            rel
        );
        // Sibling-of-dst guard: nothing was written to a sibling
        // directory under tmp.
        let sibling_attack = tmp.path().join("evil");
        assert!(
            !sibling_attack.exists(),
            "malicious path {:?} caused write outside dst_root",
            rel
        );
    }

    #[tokio::test]
    async fn fs_sink_rejects_parent_dir_traversal() {
        assert_sink_rejects("../evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_nested_parent_dir() {
        assert_sink_rejects("subdir/../../../evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_unix_absolute() {
        assert_sink_rejects("/tmp/evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_windows_drive() {
        assert_sink_rejects("C:\\evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_unc() {
        assert_sink_rejects("\\\\server\\share\\evil").await;
    }

    #[tokio::test]
    async fn fs_sink_rejects_nul_byte() {
        assert_sink_rejects("foo\0bar").await;
    }

    #[tokio::test]
    async fn fs_sink_accepts_filename_containing_dot_dot() {
        // `foo..bar` is a valid filename — only `..` as a *component* is
        // dangerous. Confirms the new validator is precise enough to not
        // reject legitimate names (the previous `rel.contains("..")`
        // check was too aggressive here).
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let content = b"valid";
        let header = make_file_header("foo..bar.txt", content.len() as u64);
        let mut reader: &[u8] = content;
        let outcome = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect("filename containing literal `..` must be accepted");

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, content.len() as u64);
        assert_eq!(std::fs::read(dst.join("foo..bar.txt")).unwrap(), content);
    }

    #[tokio::test]
    async fn fs_sink_accepts_empty_path_for_single_file_dest() {
        // Single-file destination case: dst_root is itself the final
        // file path, header.relative_path == "" by convention. This
        // path must remain working even with the safe_join chokepoint.
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        // dst_root is the file path itself, not a directory.
        let dst_root = tmp.path().join("output.bin");

        let sink = FsTransferSink::new(
            src,
            dst_root.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let content = b"single-file content";
        let header = make_file_header("", content.len() as u64);
        let mut reader: &[u8] = content;
        let outcome = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect("empty relative_path must use dst_root verbatim");

        assert_eq!(outcome.bytes_written, content.len() as u64);
        assert_eq!(std::fs::read(&dst_root).unwrap(), content);
    }

    /// R46-F3 regression: a destination root containing a pre-
    /// existing escape symlink must reject any peer-controlled
    /// wire path that would write through it. Pre-fix the sink ran
    /// only `safe_join` (lexical), which accepts `link/file.txt`
    /// because lexically that's just two components — the symlink
    /// resolution happens at write time and would land outside the
    /// destination root. unix-only because the test relies on
    /// `std::os::unix::fs::symlink`.
    #[cfg(unix)]
    #[tokio::test]
    async fn fs_sink_rejects_write_through_pre_existing_escape_symlink() {
        use std::os::unix::fs::symlink;

        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        // Pre-existing escape symlink inside dst.
        symlink(&outside, dst.join("link")).unwrap();

        let sink = FsTransferSink::new(
            src,
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        // Wire path joining through `link` resolves to /outside/victim.txt.
        let content = b"would-be exfiltration";
        let header = make_file_header("link/victim.txt", content.len() as u64);
        let mut reader: &[u8] = content;
        let err = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect_err("R46-F3: write through escape symlink must be rejected");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escape") || msg.contains("escapes"),
            "expected canonical-escape rejection, got: {msg}"
        );
        assert!(
            !outside.join("victim.txt").exists(),
            "victim file should NOT have been written outside dst"
        );
    }

    /// R47-F1 regression: the `write_payload` arm for
    /// `PreparedPayload::File` must reject a wire-controlled path
    /// that would write through a pre-existing dst escape symlink.
    /// Pre-fix `write_file_payload` lexically joined dst_root +
    /// header.relative_path, so `dst/link → /outside` plus a
    /// payload header for `link/victim` would land outside dst.
    /// The daemon's push-receive path flows through this same
    /// helper via `execute_receive_pipeline`, so this also closes
    /// the daemon-side push escape vector.
    #[cfg(unix)]
    #[tokio::test]
    async fn fs_sink_write_payload_file_rejects_escape() {
        use std::os::unix::fs::symlink;

        let tmp = tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        // Source file the planner would have prepared.
        std::fs::write(src_root.join("link/victim.txt"), b"payload").ok();
        std::fs::create_dir_all(src_root.join("link")).unwrap();
        std::fs::write(src_root.join("link/victim.txt"), b"payload").unwrap();

        // Pre-existing escape symlink in the destination.
        symlink(&outside, dst.join("link")).unwrap();

        let sink = FsTransferSink::new(
            src_root.clone(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let header = make_file_header("link/victim.txt", 7);
        let payload = PreparedPayload::File(header);
        let err = sink
            .write_payload(payload)
            .await
            .expect_err("R47-F1: PreparedPayload::File through escape symlink must reject");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escape") || msg.contains("escapes"),
            "expected canonical-escape rejection, got: {msg}"
        );
        assert!(
            !outside.join("victim.txt").exists(),
            "file payload must not write outside dst"
        );
    }

    /// R47-F1 regression: the `write_payload` arm for
    /// `PreparedPayload::TarShard` must reject any extracted entry
    /// whose destination path resolves outside dst via a pre-
    /// existing dst escape symlink. Pre-fix `write_tar_shard_payload`
    /// used `safe_extract_tar_shard` which does lexical
    /// validation but not canonical containment, so a tar with
    /// entry path `link/victim` plus `dst/link → /outside` would
    /// land bytes in /outside/victim.
    #[cfg(unix)]
    #[tokio::test]
    async fn fs_sink_write_payload_tar_shard_rejects_escape() {
        use std::os::unix::fs::symlink;
        use tar::{Builder, EntryType, Header as TarHeader};

        let tmp = tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        // Pre-existing escape symlink in destination.
        symlink(&outside, dst.join("link")).unwrap();

        // Build a tar with a single entry pointing through `link/`.
        let content = b"tar-shard payload";
        let mut tar_buf: Vec<u8> = Vec::new();
        {
            let mut builder = Builder::new(&mut tar_buf);
            let mut hdr = TarHeader::new_gnu();
            hdr.set_entry_type(EntryType::Regular);
            hdr.set_size(content.len() as u64);
            hdr.set_mode(0o644);
            hdr.set_path("link/victim.txt").unwrap();
            hdr.set_cksum();
            builder.append(&hdr, &content[..]).unwrap();
            builder.finish().unwrap();
        }

        let headers = vec![make_file_header("link/victim.txt", content.len() as u64)];

        let sink = FsTransferSink::new(
            src_root.clone(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        );

        let payload = PreparedPayload::TarShard {
            headers,
            data: tar_buf,
        };
        let err = sink
            .write_payload(payload)
            .await
            .expect_err("R47-F1: tar shard entry through escape symlink must reject");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escape") || msg.contains("escapes"),
            "expected canonical-escape rejection, got: {msg}"
        );
        assert!(
            !outside.join("victim.txt").exists(),
            "tar shard must not write outside dst"
        );
    }

    /// c-1b round 2 regression: tar shards land via `write_payload`,
    /// not `write_file_stream`, so the chunk-granular byte hook
    /// inside `receive_stream_double_buffered` never fires for them.
    /// `write_payload` now reports `outcome.bytes_written` against
    /// the sink's byte counter for non-streamed records.
    #[tokio::test]
    async fn write_payload_reports_tar_shard_bytes_against_byte_progress() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();

        let mut builder = tar::Builder::new(Vec::new());
        let content_a = b"alpha shard content";
        let content_b = b"beta shard content!";
        let mut header_a = tar::Header::new_gnu();
        header_a.set_size(content_a.len() as u64);
        header_a.set_mode(0o644);
        header_a.set_cksum();
        builder
            .append_data(&mut header_a, "a.txt", &content_a[..])
            .unwrap();
        let mut header_b = tar::Header::new_gnu();
        header_b.set_size(content_b.len() as u64);
        header_b.set_mode(0o644);
        header_b.set_cksum();
        builder
            .append_data(&mut header_b, "b.txt", &content_b[..])
            .unwrap();
        let tar_data = builder.into_inner().unwrap();
        let headers = vec![
            make_file_header("a.txt", content_a.len() as u64),
            make_file_header("b.txt", content_b.len() as u64),
        ];

        let byte_progress = ByteProgressSink::new();
        let probe_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        // Use `from_counter` so we can read the atomic directly
        // for the assertion. Cloning the sink would also work but
        // requires re-exposing a load() — `from_counter` is the
        // cleaner observer pattern.
        let sink_progress = ByteProgressSink::from_counter(std::sync::Arc::clone(&probe_counter));
        let _ = byte_progress; // keep `new()` covered too

        let sink = FsTransferSink::new(
            tmp.path().to_path_buf(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        )
        .with_byte_progress(sink_progress);

        let outcome = sink
            .write_payload(PreparedPayload::TarShard {
                headers,
                data: tar_data,
            })
            .await
            .unwrap();

        assert_eq!(outcome.files_written, 2);
        let expected = (content_a.len() + content_b.len()) as u64;
        assert_eq!(outcome.bytes_written, expected);
        assert_eq!(
            probe_counter.load(std::sync::atomic::Ordering::Relaxed),
            expected,
            "tar shard byte progress must equal outcome.bytes_written"
        );
    }

    /// c-1b round 2 regression: resume `FileBlock` payloads
    /// also land via `write_payload`. Their `bytes_written`
    /// reflects the bytes seeked-and-written; the byte counter
    /// must see them too.
    #[tokio::test]
    async fn write_payload_reports_file_block_bytes_against_byte_progress() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        // FileBlock writes seek into an existing destination file.
        // Pre-create the target with a placeholder of the right size.
        std::fs::write(dst.join("resume.bin"), vec![0u8; 64]).unwrap();

        let probe_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let sink_progress = ByteProgressSink::from_counter(std::sync::Arc::clone(&probe_counter));

        let sink = FsTransferSink::new(
            tmp.path().to_path_buf(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: ComparisonMode::SizeMtime,
            },
        )
        .with_byte_progress(sink_progress);

        let block_bytes = vec![0xABu8; 32];
        let outcome = sink
            .write_payload(PreparedPayload::FileBlock {
                relative_path: "resume.bin".to_string(),
                offset: 16,
                bytes: block_bytes.clone(),
            })
            .await
            .expect("block write succeeds against pre-created file");

        // FileBlock's outcome.bytes_written reflects bytes
        // landed on disk for this block.
        assert_eq!(outcome.bytes_written, block_bytes.len() as u64);
        assert_eq!(
            probe_counter.load(std::sync::atomic::Ordering::Relaxed),
            block_bytes.len() as u64,
            "FileBlock byte progress must equal outcome.bytes_written"
        );
    }

    // ─── Per-file containment (pfc-2, D-2026-07-30-1) ─────────────────

    /// A destination position blocked by a directory fails exactly that
    /// file: the outcome is `Ok` so the session continues, names the
    /// file, and the next payload of the same run still lands.
    #[tokio::test]
    async fn write_payload_contains_one_files_failure_and_keeps_writing() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(src.join("blocked.txt"), b"blocked payload").unwrap();
        std::fs::write(src.join("fine.txt"), b"fine payload").unwrap();
        // A directory where the file belongs: the portable way to make
        // one file's write — and only that one — fail.
        std::fs::create_dir_all(dst.join("blocked.txt")).unwrap();

        let sink = FsTransferSink::new(src, dst.clone(), FsSinkConfig::default());

        let blocked = sink
            .write_payload(PreparedPayload::File(make_file_header("blocked.txt", 15)))
            .await
            .expect("one file's write failure must not fail the session");
        assert_eq!(blocked.files_written, 0);
        assert_eq!(blocked.bytes_written, 0);
        assert_eq!(blocked.files_failed_total, 1);
        assert_eq!(blocked.failures.len(), 1);
        assert_eq!(blocked.failures[0].relative_path, "blocked.txt");
        assert!(
            !blocked.failures[0].reason.is_empty(),
            "a recorded failure carries its reason chain"
        );

        let healthy = sink
            .write_payload(PreparedPayload::File(make_file_header("fine.txt", 12)))
            .await
            .expect("the next file writes normally");
        assert_eq!(healthy.files_written, 1);
        assert_eq!(healthy.files_failed_total, 0);
        assert_eq!(
            std::fs::read(dst.join("fine.txt")).unwrap(),
            b"fine payload"
        );
    }

    /// A streamed receive whose destination cannot be opened fails that
    /// one file — and must still consume its record's bytes, or the next
    /// record would be parsed out of this file's payload.
    #[tokio::test]
    async fn write_file_stream_contains_failure_after_draining_the_record() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(dst.join("blocked.bin")).unwrap();

        let sink = FsTransferSink::new(tmp.path().join("src"), dst, FsSinkConfig::default());
        let header = make_file_header("blocked.bin", 7);
        let mut reader: &[u8] = b"payloadNEXT-RECORD";
        let outcome = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect("an unopenable destination is one file's failure");

        assert_eq!(outcome.files_written, 0);
        assert_eq!(outcome.files_failed_total, 1);
        assert_eq!(outcome.failures[0].relative_path, "blocked.bin");
        assert_eq!(
            reader,
            &b"NEXT-RECORD"[..],
            "exactly the failed record's bytes are drained"
        );
    }

    /// A destination that could not be opened never reported a byte to
    /// the live counter (its record is drained with `None`), so the
    /// pfc-4 withdrawal must not fire for it — a withdrawal here would
    /// under-count the files that DID land.
    #[tokio::test]
    async fn write_file_stream_open_failure_withdraws_no_live_bytes() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(dst.join("blocked.bin")).unwrap();

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(64));
        let sink = FsTransferSink::new(tmp.path().join("src"), dst, FsSinkConfig::default())
            .with_byte_progress(ByteProgressSink::from_counter(std::sync::Arc::clone(
                &counter,
            )));
        let header = make_file_header("blocked.bin", 7);
        let mut reader: &[u8] = b"payload";
        let outcome = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect("an unopenable destination is one file's failure");

        assert_eq!(outcome.files_failed_total, 1);
        assert_eq!(
            counter.load(std::sync::atomic::Ordering::Relaxed),
            64,
            "no bytes were reported for this file, so none come back"
        );
    }

    /// pfc-4 (the pfc-2 landing note's owed byte-accounting item): the
    /// live byte counter is fed chunk-by-chunk DURING the streamed
    /// write, before the metadata tail that can still fail the file. A
    /// failure contained there counts zero bytes for the file in the
    /// outcome — and in the `TransferSummary` built from it — so the
    /// live counter must give the streamed bytes back.
    ///
    /// Windows-only because the tail is only reachable by a real
    /// filesystem refusal: the production routes are a flush failure
    /// (ENOSPC/EIO, not injectable portably) and this one, a named
    /// stream the destination volume refuses. The wire contract caps a
    /// stream name at `MAX_WINDOWS_STREAM_NAME_BYTES` (1,024) while NTFS
    /// stops at 255 characters, so a 300-character name is a legitimate
    /// payload this destination cannot materialize — the refusal lands
    /// exactly where any stream-hostile volume's would. On non-Windows
    /// the same header is refused by `prepare_destination` BEFORE any
    /// byte is reported, which is the drained-open route the test above
    /// pins.
    #[cfg(windows)]
    #[tokio::test]
    async fn write_file_stream_withdraws_live_bytes_when_the_metadata_tail_fails() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let sink =
            FsTransferSink::new(tmp.path().join("src"), dst.clone(), FsSinkConfig::default())
                .with_byte_progress(ByteProgressSink::from_counter(std::sync::Arc::clone(
                    &counter,
                )));

        let content = b"stream content";
        let mut header = make_file_header("tail.bin", 7);
        header.windows_metadata = Some(crate::generated::WindowsFileMetadata {
            file_attributes: 0,
            named_streams: vec![crate::generated::WindowsNamedStream {
                name: "s".repeat(300),
                size: content.len() as u64,
                checksum: blake3::hash(content).as_bytes().to_vec(),
                content: content.to_vec(),
            }],
        });
        let mut reader: &[u8] = b"payloadNEXT-RECORD";
        let outcome = sink
            .write_file_stream(&header, &mut reader)
            .await
            .expect("a refused metadata tail is one file's failure");

        assert_eq!(outcome.files_failed_total, 1, "got: {outcome:?}");
        assert_eq!(outcome.bytes_written, 0);
        assert_eq!(
            counter.load(std::sync::atomic::Ordering::Relaxed),
            0,
            "the live counter must not claim bytes the summary counts as zero"
        );
    }

    /// Classification boundary: a hostile wire path is a protocol
    /// violation, never one file's failure — it still aborts.
    #[tokio::test]
    async fn write_payload_file_containment_violation_stays_fatal() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        let sink = FsTransferSink::new(src, dst, FsSinkConfig::default());
        let err = sink
            .write_payload(PreparedPayload::File(make_file_header("../evil.txt", 4)))
            .await
            .expect_err("path-safety violations stay session-fatal");
        assert!(
            format!("{err:#}").contains("validating file payload path"),
            "expected the path-validation chain; got: {err:#}"
        );
        assert!(!tmp.path().join("evil.txt").exists());
    }

    /// Classification boundary: with the destination root itself
    /// unusable every payload would fail identically, so root
    /// unavailability is session-fatal rather than a per-file failure.
    #[tokio::test]
    async fn write_payload_unusable_destination_root_stays_fatal() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"payload").unwrap();
        // The configured destination root is a regular file: nothing can
        // be written beneath it, for any payload.
        let dst_root = tmp.path().join("root-is-a-file");
        std::fs::write(&dst_root, b"not a directory").unwrap();

        let sink = FsTransferSink::new(src, dst_root.clone(), FsSinkConfig::default());
        let err = sink
            .write_payload(PreparedPayload::File(make_file_header("a.txt", 7)))
            .await
            // The PROPERTY under test: fatal, i.e. `Err`, rather than an `Ok`
            // carrying a contained per-file failure. That distinction is the
            // whole point of the classification boundary.
            .expect_err("an unusable destination root is session-fatal");

        // Deliberately NOT asserting which stage produced the error. The two
        // platforms fail at different points for the same reason — Windows
        // when the parent mkdir is refused, Unix when the containment
        // canonicalize hits ENOTDIR — and pinning one spelling made this a
        // Windows-only guard that failed on Linux for a difference that does
        // not matter. What must hold everywhere is that it is fatal AND that
        // it is about this destination, not some incidental error.
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains(&dst_root.display().to_string()) || rendered.contains("a.txt"),
            "the fatal error must name the destination it could not use; got: {rendered}"
        );
    }

    // ─── Volume-level unwritability (cr-pfc2-1) ───────────────────────
    //
    // A read-only filesystem or write-protected medium fails every write
    // of the session identically while the destination root still reads
    // as a live directory — `destination_root_live` alone cannot tell the
    // two apart. Containing those per file is what would let a mirror to
    // a read-only mount exit 0 having written nothing. Synthesized
    // `io::Error` values keep this deterministic: no real read-only
    // mount, no privileged setup, same verdict on every platform.

    /// This platform's numeric spelling of "the volume refuses writes",
    /// pinned here independently of the production constant so the fix
    /// cannot be made vacuous by editing that constant alone.
    #[cfg(unix)]
    const VOLUME_UNWRITABLE_CODE: i32 = 30; // EROFS
    #[cfg(windows)]
    const VOLUME_UNWRITABLE_CODE: i32 = 19; // ERROR_WRITE_PROTECT

    /// One file's write error in the exact `with_context` shape the
    /// sink's write sites hand the classifier.
    fn write_error(source: std::io::Error) -> eyre::Report {
        eyre::Report::new(source).wrap_err("creating /dst/one.bin")
    }

    /// One shard member handed to the fold exactly as a production worker
    /// hands it over: its write result classified against the destination
    /// as it stands at that moment (cr-pfc3-2). Tests that need the
    /// window between the failure and the fold — the root recovering —
    /// call `classify_shard_member` themselves so they can act in it.
    fn classified_member(
        dst_root: &Path,
        relative_path: &str,
        result: Result<(u64, ())>,
    ) -> (String, ClassifiedMember<()>) {
        (
            relative_path.to_string(),
            classify_shard_member(dst_root, relative_path, result),
        )
    }

    /// A read-only filesystem is root-wide, so it is refused containment
    /// and stays session-fatal even though the destination root is a
    /// perfectly live directory.
    #[test]
    fn read_only_filesystem_error_refuses_containment() {
        let tmp = tempdir().unwrap();
        assert!(
            destination_root_live(tmp.path(), "one.bin"),
            "fixture must reproduce the dangerous case: a live root"
        );

        let error = write_error(std::io::Error::from(std::io::ErrorKind::ReadOnlyFilesystem));
        assert!(
            !failure_is_containable(tmp.path(), "one.bin", &error),
            "the write-path guards must read a read-only volume as fatal"
        );

        let err = per_file_failure(tmp.path(), "one.bin", error)
            .expect_err("a read-only volume must stay session-fatal");
        assert!(
            format!("{err:#}").contains("creating /dst/one.bin"),
            "the original chain is handed back unchanged; got: {err:#}"
        );
    }

    /// The signature is found by raw OS code too, however deep in the
    /// context chain the `io::Error` sits.
    #[cfg(any(unix, windows))]
    #[test]
    fn write_protected_volume_raw_os_code_refuses_containment() {
        let tmp = tempdir().unwrap();
        let error = eyre::Report::new(std::io::Error::from_raw_os_error(VOLUME_UNWRITABLE_CODE))
            .wrap_err("writing /dst/deep/one.bin")
            .wrap_err("copy deep/one.bin");

        assert!(
            per_file_failure(tmp.path(), "deep/one.bin", error).is_err(),
            "the volume signature is found however deep the chain"
        );
    }

    /// The single-file destination convention (empty wire path — the root
    /// IS the file) takes the other `destination_root_live` branch, and
    /// is refused containment on a read-only volume just the same.
    #[test]
    fn read_only_volume_refuses_containment_for_single_file_destination() {
        let tmp = tempdir().unwrap();
        let dst_root = tmp.path().join("output.bin");
        assert!(destination_root_live(&dst_root, ""));

        let error = write_error(std::io::Error::from(std::io::ErrorKind::ReadOnlyFilesystem));
        assert!(
            per_file_failure(&dst_root, "", error).is_err(),
            "a read-only volume is fatal for the file-root shape too"
        );
    }

    /// Boundary in the other direction: an ordinary refusal of one path
    /// carries no volume signature and contains exactly as pfc-2 landed.
    #[test]
    fn ordinary_permission_denied_still_contains_one_file() {
        let tmp = tempdir().unwrap();
        let error = write_error(std::io::Error::from(std::io::ErrorKind::PermissionDenied));

        let outcome = per_file_failure(tmp.path(), "denied.bin", error)
            .expect("one denied file is still one file's failure");
        assert_eq!(outcome.files_written, 0);
        assert_eq!(outcome.files_failed_total, 1);
        assert_eq!(outcome.failures.len(), 1);
        assert_eq!(outcome.failures[0].relative_path, "denied.bin");
        assert!(
            outcome.failures[0].reason.contains("creating /dst/one.bin"),
            "the recorded reason keeps the chain; got: {}",
            outcome.failures[0].reason
        );
    }

    /// Raw OS codes live in per-platform namespaces: unix 19 is `ENODEV`
    /// and Windows 30 is `ERROR_READ_FAULT`. Reading the other
    /// platform's number as write-protection would turn an ordinary
    /// per-file error into a session abort, so the lists stay cfg-gated.
    #[cfg(any(unix, windows))]
    #[test]
    fn the_other_platforms_code_is_not_a_volume_signal() {
        let tmp = tempdir().unwrap();
        #[cfg(unix)]
        let foreign = 19; // ENODEV here, ERROR_WRITE_PROTECT on Windows
        #[cfg(windows)]
        let foreign = 30; // ERROR_READ_FAULT here, EROFS on unix

        let error = write_error(std::io::Error::from_raw_os_error(foreign));
        assert!(
            per_file_failure(tmp.path(), "one.bin", error).is_ok(),
            "a code from the other platform's namespace must not be read \
             as volume unwritability"
        );
    }

    // ─── Volume exhaustion (cr-pfc3-1) ────────────────────────────────
    //
    // A destination that runs out of room fails every remaining payload
    // of the session exactly as a read-only mount does, while the root
    // still reads as a live directory. Containing those per file is what
    // would let a mirror to a filled volume exit 0 over a demonstrably
    // incomplete backup, so the disk-exhaustion family joins the pfc-2
    // volume classifier. Synthesized `io::Error` values keep this
    // deterministic: no real full volume, no privileged setup, same
    // verdict on every platform.

    /// This platform's numeric spellings of "the volume has no room",
    /// pinned here independently of the production list so the fix cannot
    /// be made vacuous by editing that list alone.
    #[cfg(unix)]
    const VOLUME_FULL_CODES: &[i32] = &[28]; // ENOSPC
    #[cfg(windows)]
    const VOLUME_FULL_CODES: &[i32] = &[112, 39]; // ERROR_DISK_FULL, ERROR_HANDLE_DISK_FULL
    #[cfg(not(any(unix, windows)))]
    const VOLUME_FULL_CODES: &[i32] = &[];

    /// Every spelling of exhaustion the classifier must refuse: both
    /// portable kinds plus this platform's raw codes. Where std already
    /// decodes a raw code to one of the kinds (Windows 112 and 39 both
    /// land on `StorageFull`) the kind branch answers first and the raw
    /// entry is the backstop for platforms whose mapping does not name
    /// it — the same division of labour as pfc-2's EROFS entry.
    fn volume_full_signatures() -> Vec<(String, std::io::Error)> {
        let mut signatures = vec![
            (
                "StorageFull kind".to_string(),
                std::io::Error::from(std::io::ErrorKind::StorageFull),
            ),
            (
                "QuotaExceeded kind".to_string(),
                std::io::Error::from(std::io::ErrorKind::QuotaExceeded),
            ),
        ];
        for code in VOLUME_FULL_CODES {
            signatures.push((
                format!("raw os error {code}"),
                std::io::Error::from_raw_os_error(*code),
            ));
        }
        signatures
    }

    /// A full volume — every spelling of it — is refused containment on
    /// the single-file write path and stays session-fatal with its
    /// context chain handed back intact, however deep in that chain the
    /// `io::Error` sits.
    #[test]
    fn volume_exhaustion_refuses_containment_on_the_single_file_path() {
        let tmp = tempdir().unwrap();
        assert!(
            destination_root_live(tmp.path(), "one.bin"),
            "fixture must reproduce the dangerous case: a live root"
        );

        for (label, source) in volume_full_signatures() {
            let error = write_error(source).wrap_err("copy one.bin");
            assert!(
                !failure_is_containable(tmp.path(), "one.bin", &error),
                "{label} must read as volume-level, not one file's failure"
            );

            let err = per_file_failure(tmp.path(), "one.bin", error)
                .expect_err("an exhausted volume must stay session-fatal");
            assert!(
                format!("{err:#}").contains("creating /dst/one.bin"),
                "{label}: the original chain is handed back unchanged; got: {err:#}"
            );
        }
    }

    /// Disk-full numbers live in per-platform namespaces too, and the
    /// collision is sharper than pfc-2's: unix 39 is `ENOTEMPTY`, an
    /// ordinary per-file error, where Windows 39 is
    /// `ERROR_HANDLE_DISK_FULL`; Windows 28 is `ERROR_OUT_OF_PAPER`
    /// where unix 28 is `ENOSPC`. Reading the other platform's number as
    /// exhaustion would abort a whole session over one file, so the
    /// boundary is pinned on both the single-file path and the fold.
    #[cfg(any(unix, windows))]
    #[test]
    fn the_other_platforms_disk_full_codes_are_not_volume_signals() {
        let tmp = tempdir().unwrap();
        // ERROR_DISK_FULL / ERROR_HANDLE_DISK_FULL there; EHOSTDOWN /
        // ENOTEMPTY here.
        #[cfg(unix)]
        let foreign: &[i32] = &[112, 39];
        // ENOSPC there; ERROR_OUT_OF_PAPER here.
        #[cfg(windows)]
        let foreign: &[i32] = &[28];

        for &code in foreign {
            let error = write_error(std::io::Error::from_raw_os_error(code));
            assert!(
                per_file_failure(tmp.path(), "one.bin", error).is_ok(),
                "raw code {code} from the other platform's namespace must \
                 not be read as volume exhaustion"
            );
        }

        let results = vec![
            classified_member(tmp.path(), "landed.txt", Ok((7, ()))),
            classified_member(
                tmp.path(),
                "odd.txt",
                Err(write_error(std::io::Error::from_raw_os_error(foreign[0]))),
            ),
        ];
        let outcome = fold_shard_member_results(results, |()| {})
            .expect("a foreign-namespace code is still one member's failure");
        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.files_failed_total, 1);
        assert_eq!(outcome.failures[0].relative_path, "odd.txt");
    }

    /// The reported list is bounded; the total keeps counting past it, so
    /// a catastrophic run stays reportable without unbounded growth.
    #[test]
    fn file_failures_stop_at_the_cap_while_the_total_keeps_counting() {
        let mut outcome = SinkOutcome::default();
        for index in 0..70 {
            outcome.record_failure(format!("f{index}.bin"), "synthetic");
        }
        assert_eq!(outcome.failures.len(), MAX_REPORTED_FILE_FAILURES);
        assert_eq!(outcome.files_failed_total, 70);
        assert_eq!(outcome.failures[0].relative_path, "f0.bin");
        assert_eq!(
            outcome.failures[MAX_REPORTED_FILE_FAILURES - 1].relative_path,
            format!("f{}.bin", MAX_REPORTED_FILE_FAILURES - 1)
        );
        // pfc-3 adapts the identity half of this contract. pfc-2 kept
        // identity only in the capped list, so an overflowed outcome
        // answered `true` for every path — safe, but it would suppress
        // the completions of a tar shard's healthy members whenever the
        // shard failed more than the cap. Identity now lives outside the
        // cap, per outcome, so an overflowed outcome answers exactly. The
        // bound the cap exists for is unchanged: `failures` above.
        assert!(outcome.file_failed("f0.bin"));
        assert!(
            outcome.file_failed(&format!("f{}.bin", MAX_REPORTED_FILE_FAILURES + 3)),
            "a failure past the cap is still named by identity"
        );
        assert!(!outcome.file_failed("never-seen.bin"));
    }

    /// pfc-4 (contract v6): the SENDER caps the list the wire carries at
    /// MAX_REPORTED_FILE_FAILURES while `files_failed_total` stays
    /// exact — one over the cap carries 64 details and a total of 65.
    /// The cap is applied by the conversion itself, not inherited from
    /// whoever filled `failures`, so no producer can put an unbounded
    /// list on the wire.
    #[test]
    fn the_wire_report_is_capped_by_the_sender_while_the_total_stays_exact() {
        let mut outcome = SinkOutcome::default();
        for index in 0..MAX_REPORTED_FILE_FAILURES + 1 {
            outcome.record_failure(format!("f{index}.bin"), format!("reason {index}"));
        }
        assert_eq!(outcome.files_failed_total, 65);
        let wire = outcome.wire_failures();
        assert_eq!(wire.len(), MAX_REPORTED_FILE_FAILURES);
        assert_eq!(wire[0].relative_path, "f0.bin");
        assert_eq!(wire[0].reason, "reason 0");
        assert_eq!(
            wire[MAX_REPORTED_FILE_FAILURES - 1].relative_path,
            format!("f{}.bin", MAX_REPORTED_FILE_FAILURES - 1)
        );

        // An over-long list handed in directly (a future producer that
        // does not build through `record_failure`) is still bounded.
        let mut unbounded = SinkOutcome::default();
        unbounded.failures = (0..MAX_REPORTED_FILE_FAILURES + 10)
            .map(|index| FileFailure {
                relative_path: format!("x{index}.bin"),
                reason: "direct".into(),
            })
            .collect();
        unbounded.files_failed_total = unbounded.failures.len() as u64;
        assert_eq!(
            unbounded.wire_failures().len(),
            MAX_REPORTED_FILE_FAILURES,
            "the conversion re-applies the wire bound"
        );
    }

    /// tonic's default decode limit — the ceiling the closing `Summary`
    /// frame lives under (`transfer_session`'s frame discipline,
    /// D-2026-07-10-1). Restated locally so this module's bound can be
    /// asserted against the thing that actually rejects an oversized
    /// frame, not just against its own constant.
    const TONIC_DECODE_LIMIT_BYTES: usize = 4 * 1024 * 1024;

    /// The encoded cost of a report as its parent summary pays it.
    fn encoded_failures_cost(wire: &[crate::generated::FileFailure]) -> usize {
        use prost::Message as _;

        wire.iter()
            .map(|entry| entry.encoded_len() + WIRE_FAILURE_ENVELOPE_BYTES)
            .sum()
    }

    /// Build the summary exactly as the session's single construction
    /// site does (`transfer_session::run_session`): the exact total plus
    /// the sender-bounded list.
    fn wire_summary(outcome: &SinkOutcome) -> crate::generated::TransferSummary {
        crate::generated::TransferSummary {
            files_transferred: 3,
            bytes_transferred: 4096,
            entries_deleted: 0,
            in_stream_carrier_used: true,
            files_resumed: 0,
            files_failed: outcome.files_failed_total,
            failures: outcome.wire_failures(),
        }
    }

    /// cr-pfc4-2: the entry-count cap bounds entries, not BYTES. 64
    /// near-maximum path+reason strings encode to megabytes, and the
    /// report rides one `Summary` frame under tonic's 4 MiB decode
    /// limit — so an unbounded report does not degrade the report, it
    /// makes the peer reject the frame and the session that successfully
    /// contained its failures fault at the very end with no summary at
    /// all.
    ///
    /// The fixture is the observable failure: 70 entries whose paths sit
    /// at the Windows extended-length ceiling and whose reasons are
    /// 64 KiB eyre chains. Pre-fix that summary encodes to ~6 MiB.
    #[test]
    fn the_wire_report_stays_far_under_the_summary_frame_limit() {
        use prost::Message as _;

        // Near-maximum by construction, not by taste: 32_767 bytes is
        // the Windows extended-length path ceiling, and a deep
        // `with_context` chain over such a path is how a reason grows.
        let deep_dir = "d".repeat(32_767);
        let long_chain = "x".repeat(64 * 1024);
        let mut outcome = SinkOutcome::written(3, 4096);
        for index in 0..MAX_REPORTED_FILE_FAILURES + 6 {
            outcome.record_failure(
                format!("{deep_dir}/f{index}.bin"),
                format!("synthetic {index}: {long_chain}"),
            );
        }
        assert_eq!(
            outcome.files_failed_total,
            (MAX_REPORTED_FILE_FAILURES + 6) as u64,
            "the exact total survives every bound"
        );

        // The unbounded report is what the frame would have had to
        // carry — the fixture must genuinely blow the limit, or this
        // test proves nothing.
        let unbounded: usize = outcome
            .failures
            .iter()
            .map(|failure| {
                failure.relative_path.len() + failure.reason.len() + 2 * WIRE_FAILURE_ENVELOPE_BYTES
            })
            .sum();
        assert!(
            unbounded > TONIC_DECODE_LIMIT_BYTES,
            "fixture must exceed the frame limit unbounded to exercise the fix (got {unbounded})"
        );

        let summary = wire_summary(&outcome);
        let carried = encoded_failures_cost(&summary.failures);
        assert!(
            carried <= MAX_WIRE_FAILURES_ENCODED_BYTES,
            "the aggregate byte budget bounds the report (got {carried})"
        );
        assert!(
            summary.encoded_len() < TONIC_DECODE_LIMIT_BYTES,
            "the whole summary frame must clear the decode limit (got {})",
            summary.encoded_len()
        );

        // A bounded report is still a REPORT: the count alone names
        // nothing an operator can act on.
        assert!(
            !summary.failures.is_empty(),
            "a non-zero count must ship at least one named failure"
        );
        assert!(
            summary.failures.len() < MAX_REPORTED_FILE_FAILURES,
            "these entries are long enough that the byte budget binds \
             before the count cap (got {})",
            summary.failures.len()
        );
        // Order-preserving prefix, with both truncation directions doing
        // their job: the reason's head says what happened, the path's
        // tail says to which file.
        assert!(summary.failures[0].reason.starts_with("synthetic 0: x"));
        assert!(summary.failures[0].reason.ends_with(TRUNCATION_MARKER));
        assert!(summary.failures[0].relative_path.ends_with("/f0.bin"));
        assert!(summary.failures[0]
            .relative_path
            .starts_with(TRUNCATION_MARKER));
        for entry in &summary.failures {
            assert!(entry.relative_path.len() <= MAX_FAILURE_PATH_BYTES);
            assert!(entry.reason.len() <= MAX_FAILURE_REASON_BYTES);
        }

        // The exact count is what tells the operator files are missing;
        // it is never the list's length.
        assert_eq!(
            summary.files_failed,
            (MAX_REPORTED_FILE_FAILURES + 6) as u64
        );
    }

    /// The bound is a BYTE bound applied to UTF-8 text, so every cut
    /// lands on a char boundary — a byte-sliced multibyte edge is not
    /// valid UTF-8, and a protobuf `string` field may not carry it.
    #[test]
    fn report_string_bounds_cut_on_char_boundaries() {
        let crab = "🦀"; // 4 bytes
        let two_byte = "é"; // 2 bytes

        let mut outcome = SinkOutcome::default();
        outcome.record_failure(
            format!("{}/tärget.bin", crab.repeat(20_000)),
            two_byte.repeat(20_000),
        );
        let wire = outcome.wire_failures();
        assert_eq!(wire.len(), 1);

        assert!(wire[0].reason.len() <= MAX_FAILURE_REASON_BYTES);
        assert!(wire[0].reason.ends_with(TRUNCATION_MARKER));
        assert!(
            wire[0]
                .reason
                .trim_end_matches(TRUNCATION_MARKER)
                .chars()
                .all(|c| c == 'é'),
            "no character survived half-cut"
        );

        assert!(wire[0].relative_path.len() <= MAX_FAILURE_PATH_BYTES);
        assert!(wire[0].relative_path.starts_with(TRUNCATION_MARKER));
        assert!(
            wire[0].relative_path.ends_with("/tärget.bin"),
            "the tail names the file, so the tail is what survives"
        );
        assert!(
            wire[0]
                .relative_path
                .trim_start_matches(TRUNCATION_MARKER)
                .chars()
                .all(|c| c == '🦀' || "/tärget.bin".contains(c)),
            "no character survived half-cut"
        );

        // Every bound, including ones smaller than the marker itself, at
        // every awkward offset inside a multibyte run: the byte bound is
        // honored and the result is always valid UTF-8 (a `String`
        // cannot be otherwise, so a mid-character cut would panic here).
        let multibyte = format!("{}{}", crab.repeat(8), two_byte.repeat(8));
        for max_bytes in 0..=multibyte.len() + 4 {
            let head = bounded_head(&multibyte, max_bytes);
            let tail = bounded_tail(&multibyte, max_bytes);
            assert!(
                head.len() <= max_bytes,
                "head bound honored at {max_bytes} (got {})",
                head.len()
            );
            assert!(
                tail.len() <= max_bytes,
                "tail bound honored at {max_bytes} (got {})",
                tail.len()
            );
            assert!(
                multibyte.contains(head.trim_end_matches(TRUNCATION_MARKER)),
                "the kept head is a whole-character prefix of the input"
            );
            assert!(
                multibyte.contains(tail.trim_start_matches(TRUNCATION_MARKER)),
                "the kept tail is a whole-character suffix of the input"
            );
        }
    }

    /// The bounds must be invisible to every report a real run produces:
    /// an ordinary failure reaches the wire byte-identically, marker-free
    /// and round-trippable. A bound that rewrote ordinary reasons would
    /// be a regression in the surface it exists to protect.
    #[test]
    fn an_ordinary_report_passes_through_byte_identical() {
        let mut outcome = SinkOutcome::written(2, 20);
        outcome.record_failure(
            "dir/sub/one.bin",
            "copy dir/sub/one.bin: Access is denied. (os error 5)",
        );
        outcome.record_failure("", "creating C:\\dst: The system cannot find the path");

        let wire = outcome.wire_failures();
        assert_eq!(wire.len(), 2);
        for (entry, record) in wire.iter().zip(outcome.failures.iter()) {
            assert_eq!(entry.relative_path, record.relative_path);
            assert_eq!(entry.reason, record.reason);
            assert!(!entry.relative_path.contains(TRUNCATION_MARKER));
            assert!(!entry.reason.contains(TRUNCATION_MARKER));
            assert_eq!(&FileFailure::from_wire(entry), record);
        }
        assert_eq!(outcome.files_failed_total, 2);
        // The empty-path convention (the destination root IS the file)
        // must not acquire a marker either.
        assert_eq!(wire[1].relative_path, "");
    }

    /// The wire form round-trips both fields, so a peer's report reads
    /// back into the in-memory shape the local summary carries.
    #[test]
    fn a_file_failure_round_trips_its_wire_form() {
        let failure = FileFailure {
            relative_path: "dir/one.bin".into(),
            reason: "write dir/one.bin: Access is denied".into(),
        };
        assert_eq!(FileFailure::from_wire(&failure.to_wire()), failure);
    }

    /// Merging drops per-payload identity on purpose — a session's failed
    /// set has no bound — so a merged outcome keeps pfc-2's conservative
    /// answer. Nothing reads completions off a merged outcome: every
    /// completion lane asks the payload's own outcome, before the merge.
    #[test]
    fn a_merged_outcome_answers_conservatively_for_every_path() {
        let mut total = SinkOutcome::written(1, 10);
        let mut other = SinkOutcome::default();
        other.record_failure("b.bin", "synthetic");
        total.merge(&other);

        assert_eq!(total.files_failed_total, 1);
        assert!(total.file_failed("b.bin"));
        assert!(
            total.file_failed("a.bin"),
            "identity is not merged, so the merged answer stays conservative"
        );
    }

    /// Merging holds the same bound and sums both totals, so the
    /// pipeline-wide total cannot grow without limit either.
    #[test]
    fn merging_failures_respects_the_cap_and_sums_totals() {
        let mut total = SinkOutcome::written(1, 10);
        for index in 0..40 {
            total.record_failure(format!("a{index}.bin"), "first member");
        }
        let mut other = SinkOutcome::written(2, 20);
        for index in 0..40 {
            other.record_failure(format!("b{index}.bin"), "second member");
        }
        total.merge(&other);

        assert_eq!(total.files_written, 3);
        assert_eq!(total.bytes_written, 30);
        assert_eq!(total.failures.len(), MAX_REPORTED_FILE_FAILURES);
        assert_eq!(total.files_failed_total, 80);
        assert_eq!(total.failures[40].relative_path, "b0.bin");
    }

    /// A resume block that cannot be patched fails its file once, and the
    /// completion record must not stamp it: a stamped partial would read
    /// as converged on every later compare instead of resuming.
    #[tokio::test]
    async fn failed_resume_block_fails_once_and_blocks_finalization() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        // A directory at the resumed file's path blocks the patch.
        std::fs::create_dir_all(dst.join("resume.bin")).unwrap();

        let sink =
            FsTransferSink::new(tmp.path().join("src"), dst.clone(), FsSinkConfig::default());

        for offset in [0u64, 32] {
            let outcome = sink
                .write_payload(PreparedPayload::FileBlock {
                    relative_path: "resume.bin".to_string(),
                    offset,
                    bytes: vec![0xABu8; 32],
                })
                .await
                .expect("a block that cannot be patched is one file's failure");
            assert_eq!(
                outcome.files_failed_total, 0,
                "a resumed file is reported once, at its completion record"
            );
        }

        let complete = sink
            .write_payload(PreparedPayload::FileBlockComplete {
                relative_path: "resume.bin".to_string(),
                total_size: 64,
                mtime_seconds: 1_700_000_000,
                permissions: 0o644,
                windows_metadata: None,
            })
            .await
            .expect("the completion record reports, it does not fault");
        assert_eq!(complete.files_written, 0);
        assert_eq!(complete.files_failed_total, 1);
        assert_eq!(complete.failures.len(), 1);
        assert_eq!(complete.failures[0].relative_path, "resume.bin");
        assert!(
            dst.join("resume.bin").is_dir(),
            "the finalization must not have run against the blocked path"
        );
    }

    // ─── Tar-shard per-member containment (pfc-3) ─────────────────────
    //
    // A shard is many files in one payload. Before pfc-3 the rayon
    // writers folded `result?`, so the first member the destination
    // refused abandoned every other member of the shard and faulted the
    // session (audit-17). Structural and containment failures of the
    // shard itself still short-circuit above the writers, and the
    // cr-pfc2-1 classifier still decides what "one member's failure"
    // means.

    /// Build an in-memory tar shard and the manifest headers describing
    /// it, in the exact shape `PreparedPayload::TarShard` carries.
    fn shard_payload(members: &[(&str, &[u8])]) -> (Vec<FileHeader>, Vec<u8>) {
        let mut builder = tar::Builder::new(Vec::new());
        for (rel, contents) in members {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, rel, *contents).unwrap();
        }
        let data = builder.into_inner().unwrap();
        let headers = members
            .iter()
            .map(|(rel, contents)| make_file_header(rel, contents.len() as u64))
            .collect();
        (headers, data)
    }

    /// audit-17 regression: one shard member the destination filesystem
    /// refuses is contained as that member's failure, and the rest of the
    /// shard still lands. The field failure was `create_dir_all` inside
    /// this parallel-write closure returning `Invalid argument` for a
    /// name the destination could not represent, ~88k entries into a
    /// copy; a regular file sitting where the member's parent directory
    /// belongs is the portable stand-in — the same mkdir, refused for a
    /// different reason.
    #[tokio::test]
    async fn tar_shard_member_with_an_unmakeable_parent_fails_alone() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        // A regular file occupies the directory the blocked member needs.
        std::fs::write(dst.join("cache"), b"not a directory").unwrap();

        let (headers, data) = shard_payload(&[
            ("before.txt", b"before"),
            ("cache/blocked.txt", b"blocked"),
            ("after.txt", b"after!"),
        ]);
        let sink =
            FsTransferSink::new(tmp.path().join("src"), dst.clone(), FsSinkConfig::default());

        let outcome = sink
            .write_payload(PreparedPayload::TarShard { headers, data })
            .await
            .expect("one member's failure must not abort the whole shard");

        assert_eq!(outcome.files_written, 2);
        assert_eq!(outcome.files_failed_total, 1);
        assert_eq!(outcome.failures.len(), 1);
        assert_eq!(outcome.failures[0].relative_path, "cache/blocked.txt");
        assert!(
            !outcome.failures[0].reason.is_empty(),
            "a recorded member failure carries its reason chain"
        );
        assert!(outcome.file_failed("cache/blocked.txt"));
        assert!(!outcome.file_failed("before.txt"));
        assert_eq!(std::fs::read(dst.join("before.txt")).unwrap(), b"before");
        assert_eq!(std::fs::read(dst.join("after.txt")).unwrap(), b"after!");
    }

    /// pfc-3 cap identity: a shard that fails more members than the
    /// report can carry still names its healthy members as landed, so
    /// their completions are never suppressed. The carried details stay
    /// capped and the total stays exact.
    #[tokio::test]
    async fn tar_shard_past_the_report_cap_still_completes_its_healthy_members() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();

        let blocked: Vec<String> = (0..MAX_REPORTED_FILE_FAILURES + 6)
            .map(|index| format!("blocked{index}.txt"))
            .collect();
        // A directory where each blocked member's file belongs: the
        // portable way to fail exactly those members and no others.
        for rel in &blocked {
            std::fs::create_dir_all(dst.join(rel)).unwrap();
        }
        let healthy = ["healthy-a.txt", "healthy-b.txt"];
        let mut members: Vec<(&str, &[u8])> = blocked
            .iter()
            .map(|rel| (rel.as_str(), &b"x"[..]))
            .collect();
        members.extend(healthy.iter().map(|rel| (*rel, &b"landed"[..])));
        let (headers, data) = shard_payload(&members);

        let sink =
            FsTransferSink::new(tmp.path().join("src"), dst.clone(), FsSinkConfig::default());
        let outcome = sink
            .write_payload(PreparedPayload::TarShard { headers, data })
            .await
            .expect("a shard past the report cap is still not a session failure");

        assert_eq!(outcome.files_written, healthy.len());
        assert_eq!(outcome.files_failed_total, blocked.len() as u64);
        assert_eq!(
            outcome.failures.len(),
            MAX_REPORTED_FILE_FAILURES,
            "carried details stay bounded"
        );
        for rel in &blocked {
            assert!(outcome.file_failed(rel), "{rel} failed and must say so");
        }
        for rel in healthy {
            assert!(
                !outcome.file_failed(rel),
                "{rel} landed; a shard past the cap must not suppress its \
                 healthy members' completions"
            );
            assert_eq!(std::fs::read(dst.join(rel)).unwrap(), b"landed");
        }
    }

    /// The probed writer is a second, separately-coded rayon path (the
    /// otp-12 timing instrumentation). It contains a member failure the
    /// same way the probe-less one does.
    #[tokio::test]
    async fn probed_tar_shard_writer_contains_one_members_failure() {
        use crate::remote::transfer::session_phase::SessionPhaseRole;
        use crate::remote::transfer::small_file_probe::{SmallFileCarrier, SmallFileProbe};

        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        // A directory where the blocked member's file belongs.
        std::fs::create_dir_all(dst.join("blocked.txt")).unwrap();

        let probe = SmallFileProbe::capture("pfc-3", |_report| {})
            .bind(
                None,
                SessionPhaseRole::Destination,
                SessionPhaseRole::Source,
                SmallFileCarrier::Tcp,
            )
            .expect("a capturing probe binds");
        let sink =
            FsTransferSink::new(tmp.path().join("src"), dst.clone(), FsSinkConfig::default())
                .with_small_file_probe(Some(probe));

        let (headers, data) = shard_payload(&[("blocked.txt", b"blocked"), ("fine.txt", b"fine")]);
        let outcome = sink
            .write_payload(PreparedPayload::TarShard { headers, data })
            .await
            .expect("the probed writer contains a member failure too");

        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.files_failed_total, 1);
        assert_eq!(outcome.failures[0].relative_path, "blocked.txt");
        assert!(!outcome.file_failed("fine.txt"));
        assert_eq!(std::fs::read(dst.join("fine.txt")).unwrap(), b"fine");
    }

    /// Classification boundary: a shard whose tar will not parse failed
    /// structurally, as a whole record — no member owns that error, so it
    /// stays session-fatal.
    #[tokio::test]
    async fn tar_shard_structural_parse_failure_stays_fatal() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        let sink =
            FsTransferSink::new(tmp.path().join("src"), dst.clone(), FsSinkConfig::default());

        let err = sink
            .write_payload(PreparedPayload::TarShard {
                headers: vec![make_file_header("a.txt", 5)],
                data: vec![0x41u8; 2048], // not a tar
            })
            .await
            .expect_err("a shard that will not parse is session-fatal");
        assert!(
            format!("{err:#}").contains("tar shard"),
            "expected the shard-structural chain; got: {err:#}"
        );
        assert_eq!(
            std::fs::read_dir(&dst).unwrap().count(),
            0,
            "a shard that never parsed writes nothing"
        );
    }

    /// Classification boundary: a shard entry that escapes the
    /// destination root is a containment violation (R47-F1 class), never
    /// one member's failure. Hand-crafted bytes because the tar builder
    /// refuses to write a traversal path — the same technique
    /// `tar_safety`'s own traversal test uses.
    #[tokio::test]
    async fn tar_shard_traversal_entry_stays_fatal() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();

        let (_, mut data) = shard_payload(&[("aaaaaaaaa.txt", b"pwn")]);
        let bad_name = b"../escape.txt\0";
        data[..bad_name.len()].copy_from_slice(bad_name);
        let mut sum: u32 = 0;
        for (index, byte) in data[..512].iter().enumerate() {
            sum += if (148..156).contains(&index) {
                0x20
            } else {
                *byte as u32
            };
        }
        data[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());

        let sink = FsTransferSink::new(tmp.path().join("src"), dst, FsSinkConfig::default());
        let err = sink
            .write_payload(PreparedPayload::TarShard {
                headers: vec![make_file_header("../escape.txt", 3)],
                data,
            })
            .await
            .expect_err("a traversal entry is a containment violation, not a member failure");
        assert!(
            format!("{err:#}")
                .to_lowercase()
                .contains("validating tar shard entry"),
            "expected the path-validation chain; got: {err:#}"
        );
        assert!(!tmp.path().join("escape.txt").exists());
    }

    /// cr-pfc2-1 inside the shard fold: a member whose write failed
    /// because the volume refuses writes is not that member's failure —
    /// every other member fails identically — so the shard returns `Err`
    /// even though its other members landed. Synthesized `io::Error`
    /// values keep this deterministic, as in the pfc-2 tests above.
    #[test]
    fn tar_shard_member_on_a_read_only_volume_stays_fatal() {
        let tmp = tempdir().unwrap();
        let results = vec![
            classified_member(tmp.path(), "landed.txt", Ok((7, ()))),
            classified_member(
                tmp.path(),
                "refused.txt",
                Err(write_error(std::io::Error::from(
                    std::io::ErrorKind::ReadOnlyFilesystem,
                ))),
            ),
        ];

        let err = fold_shard_member_results(results, |()| {})
            .expect_err("a volume-level refusal inside a shard stays session-fatal");
        assert!(
            format!("{err:#}").contains("creating /dst/one.bin"),
            "the original chain is handed back unchanged; got: {err:#}"
        );
    }

    /// cr-pfc3-1 inside the shard fold — the path the finding names as
    /// the dangerous one: a destination that fills mid-shard fails every
    /// remaining member, so a member's out-of-space error is not that
    /// member's failure and the shard returns `Err` even though earlier
    /// members landed. Both kinds and this platform's raw codes are
    /// checked, since the fold shares one classifier with the
    /// single-file paths and must never drift from it.
    #[test]
    fn tar_shard_member_on_an_exhausted_volume_stays_fatal() {
        let tmp = tempdir().unwrap();
        for (label, source) in volume_full_signatures() {
            let results = vec![
                classified_member(tmp.path(), "landed.txt", Ok((7, ()))),
                classified_member(tmp.path(), "refused.txt", Err(write_error(source))),
                classified_member(tmp.path(), "never-tried.txt", Ok((3, ()))),
            ];

            let err = fold_shard_member_results(results, |()| {})
                .expect_err("an exhausted volume inside a shard stays session-fatal");
            assert!(
                format!("{err:#}").contains("creating /dst/one.bin"),
                "{label}: the member's chain is handed back unchanged; got: {err:#}"
            );
        }
    }

    /// Boundary the other way: an ordinary refusal of one member's own
    /// path carries no volume signature and is contained, with the
    /// shard's healthy members counted.
    #[test]
    fn tar_shard_member_denied_by_its_own_path_is_contained() {
        let tmp = tempdir().unwrap();
        let results = vec![
            classified_member(tmp.path(), "landed.txt", Ok((7, ()))),
            classified_member(
                tmp.path(),
                "denied.txt",
                Err(write_error(std::io::Error::from(
                    std::io::ErrorKind::PermissionDenied,
                ))),
            ),
        ];

        let outcome = fold_shard_member_results(results, |()| {})
            .expect("one denied member is still one member's failure");
        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 7);
        assert_eq!(outcome.files_failed_total, 1);
        assert_eq!(outcome.failures[0].relative_path, "denied.txt");
        assert!(!outcome.file_failed("landed.txt"));
    }

    // ─── Classify where the failure happened (cr-pfc3-2) ───────────────
    //
    // `failure_is_containable` probes live destination state, so the
    // verdict is only sound while the destination is still in the state
    // the failing write saw. Classifying a shard's members after every
    // worker finished is a time-of-check/time-of-use split: an SMB root
    // that dropped mid-shard and reconnected before the fold — or was
    // recreated — had its root-caused member errors reclassified as
    // per-file, and the mirror finished "incomplete" over a destination
    // that had actually died.
    //
    // The recovery is what makes this deterministic: no threads and no
    // timing window are needed. The verdict is taken by the production
    // classifier against a dead root, the root is then restored, and only
    // then does the fold run — the exact sequence the pre-fix fold
    // observed, with the failure and the fold seeing opposite root states.

    /// One shard member in the shape the rayon writers hand
    /// `write_shard_member`, so a member error in these tests is produced
    /// by the real production writer rather than synthesized.
    fn shard_member(
        dst_root: &Path,
        rel: &str,
        contents: &[u8],
    ) -> crate::remote::transfer::tar_safety::ExtractedFile {
        crate::remote::transfer::tar_safety::ExtractedFile {
            rel: rel.to_string(),
            dest_path: dst_root.join(rel),
            contents: contents.to_vec(),
            mtime: None,
            permissions: None,
            size: contents.len() as u64,
            windows_metadata: None,
        }
    }

    /// A regular file where the destination root belongs: the portable,
    /// unprivileged stand-in for "the root is gone" — it fails every
    /// member identically and `destination_root_live` reads it as dead,
    /// exactly as a vanished SMB mount.
    fn kill_root(dst_root: &Path) {
        if dst_root.is_dir() {
            std::fs::remove_dir_all(dst_root).unwrap();
        }
        std::fs::write(dst_root, b"the destination root is not a directory").unwrap();
        assert!(
            !destination_root_live(dst_root, "one.bin"),
            "fixture must reproduce the dangerous case: a dead root"
        );
    }

    /// The root comes back — the reconnect (or recreate) that used to
    /// launder a dead-root shard into per-file failures.
    fn revive_root(dst_root: &Path) {
        std::fs::remove_file(dst_root).unwrap();
        std::fs::create_dir(dst_root).unwrap();
        assert!(
            destination_root_live(dst_root, "one.bin"),
            "the fold must run against a live root or this proves nothing"
        );
    }

    /// cr-pfc3-2: a member whose write failed while the destination root
    /// was dead stays session-fatal even though the root is live again by
    /// the time the shard is folded. Pre-fix the fold called
    /// `failure_is_containable` itself, so this same sequence contained
    /// the root's failure as one member's own and the shard reported
    /// success-with-a-skip.
    #[test]
    fn a_member_that_failed_on_a_dead_root_stays_fatal_after_the_root_recovers() {
        let tmp = tempdir().unwrap();
        let dst_root = tmp.path().join("root");
        kill_root(&dst_root);

        // The real member writer, failing for the real reason.
        let member = shard_member(&dst_root, "deep/one.bin", b"payload");
        let written = write_shard_member(&member).map(|bytes| (bytes, ()));
        let chain = format!(
            "{:#}",
            written
                .as_ref()
                .expect_err("a dead root must fail the member write")
        );
        // The worker's verdict, taken while the root is still dead.
        let verdict = classify_shard_member(&dst_root, &member.rel, written);

        revive_root(&dst_root);

        let err = fold_shard_member_results(vec![(member.rel.clone(), verdict)], |()| {})
            .expect_err("a root that died during the write stays session-fatal");
        assert_eq!(
            format!("{err:#}"),
            chain,
            "the failing member's chain is handed back unchanged"
        );
    }

    /// The same window, with healthy members either side: the shard is
    /// still fatal and the recovered root does not turn the dead-root
    /// error into a recorded per-member failure.
    #[test]
    fn a_dead_root_verdict_is_fatal_even_beside_members_that_landed() {
        let tmp = tempdir().unwrap();
        let dst_root = tmp.path().join("root");
        std::fs::create_dir(&dst_root).unwrap();

        // One member lands while the root is alive.
        let landed = shard_member(&dst_root, "landed.bin", b"landed!");
        let landed_result = write_shard_member(&landed).map(|bytes| (bytes, ()));
        assert!(
            landed_result.is_ok(),
            "the fixture's healthy member must actually land"
        );
        let landed_verdict = classify_shard_member(&dst_root, &landed.rel, landed_result);

        // Then the root dies under a concurrent member.
        kill_root(&dst_root);
        let refused = shard_member(&dst_root, "refused.bin", b"refused");
        let refused_result = write_shard_member(&refused).map(|bytes| (bytes, ()));
        assert!(
            refused_result.is_err(),
            "the member must fail against the dead root"
        );
        let refused_verdict = classify_shard_member(&dst_root, &refused.rel, refused_result);

        revive_root(&dst_root);

        assert!(
            fold_shard_member_results(
                vec![
                    (landed.rel.clone(), landed_verdict),
                    (refused.rel.clone(), refused_verdict),
                ],
                |()| {},
            )
            .is_err(),
            "a dead-root member is session-fatal however many members landed"
        );
    }

    /// The boundary in the other direction, which is what keeps the fix
    /// honest: a verdict of "this member's own failure", taken while the
    /// root was live, is still contained when the root dies before the
    /// fold. The recorded verdict is honored in both directions — the
    /// fold neither re-derives a contained verdict into a fatal one nor
    /// the reverse — so a fix that simply called everything fatal would
    /// fail here.
    #[test]
    fn a_member_denied_under_a_live_root_stays_contained_if_the_root_dies_later() {
        let tmp = tempdir().unwrap();
        let dst_root = tmp.path().join("root");
        std::fs::create_dir(&dst_root).unwrap();

        let results = vec![
            classified_member(&dst_root, "landed.txt", Ok((7, ()))),
            classified_member(
                &dst_root,
                "denied.txt",
                Err(write_error(std::io::Error::from(
                    std::io::ErrorKind::PermissionDenied,
                ))),
            ),
        ];

        // The root dies after every member's verdict is recorded. The
        // next payload of the session will see it; this shard's members
        // were judged when their writes happened.
        kill_root(&dst_root);

        let outcome = fold_shard_member_results(results, |()| {})
            .expect("a verdict taken under a live root stays that member's failure");
        assert_eq!(outcome.files_written, 1);
        assert_eq!(outcome.bytes_written, 7);
        assert_eq!(outcome.files_failed_total, 1);
        assert_eq!(outcome.failures[0].relative_path, "denied.txt");
    }

    /// A volume-exhaustion member error is refused containment at the
    /// moment of classification too (cr-pfc3-1 rides the same seam), and
    /// no later state of the destination can launder it: the root is live
    /// throughout, so only the recorded verdict can make this fatal.
    #[test]
    fn a_volume_exhaustion_verdict_survives_the_fold() {
        let tmp = tempdir().unwrap();
        for (label, source) in volume_full_signatures() {
            let results = vec![
                classified_member(tmp.path(), "landed.txt", Ok((7, ()))),
                classified_member(tmp.path(), "refused.txt", Err(write_error(source))),
            ];
            assert!(
                destination_root_live(tmp.path(), "refused.txt"),
                "{label}: the root stays live, so the verdict is the only signal"
            );

            let err = fold_shard_member_results(results, |()| {})
                .expect_err("an exhausted volume stays session-fatal through the fold");
            assert!(
                format!("{err:#}").contains("creating /dst/one.bin"),
                "{label}: the member's chain is handed back unchanged; got: {err:#}"
            );
        }
    }

    /// Records every warn the `log` facade emits. The facade takes one
    /// backend per process, so this one is installed once and read by the
    /// test that needs it; other tests assert nothing about logging.
    static CAPTURED_WARNINGS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

    struct CaptureWarnings;

    impl log::Log for CaptureWarnings {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Warn
        }

        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                CAPTURED_WARNINGS
                    .lock()
                    .expect("captured-warnings lock poisoned")
                    .push(record.args().to_string());
            }
        }

        fn flush(&self) {}
    }

    /// A contained failure's only surface until the summary carries the
    /// report is its log line: a file that is skipped without a word is
    /// indistinguishable from a transferred one.
    #[test]
    fn a_contained_failure_is_logged_with_its_path_and_reason() {
        log::set_logger(&CaptureWarnings).expect("no other log backend in the lib tests");
        log::set_max_level(log::LevelFilter::Warn);

        let outcome = SinkOutcome::failed("logged/one.bin", "access is denied");
        assert_eq!(outcome.files_failed_total, 1);

        let logged = CAPTURED_WARNINGS
            .lock()
            .expect("captured-warnings lock poisoned");
        assert!(
            logged
                .iter()
                .any(|line| line.contains("logged/one.bin") && line.contains("access is denied")),
            "expected a warn naming the failed file and its reason; got: {logged:?}"
        );
    }
}
