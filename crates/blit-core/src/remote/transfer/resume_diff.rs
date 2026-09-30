//! otp-7b: the SOURCE-side resume block diff, single-sourced for both
//! byte carriers. The in-stream carrier (`transfer_session`'s
//! `send_resume_block_records`) emits a `BlockTransfer` frame per stale
//! block; the TCP data plane (`DataPlaneSink`) emits a binary `BLOCK`
//! record per stale block. Both drive this iterator, so the D1
//! semantics — an index beyond the destination's list is stale, a
//! differing hash is stale, and a MALFORMED hash (wrong length) counts
//! as differing so garbage degrades to sending the block, never to
//! trusting it — live in exactly one place (plan D1/D3,
//! `docs/plan/OTP7_RESUME.md`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use eyre::Result;
use tokio::io::AsyncReadExt;

use crate::generated::FileHeader;

use super::faulted_path::FaultedPath;
use super::source::{OpenedSourceFile, TransferSource};

/// One step of the resume block diff.
#[derive(Debug)]
pub enum ResumeDiffEvent<'a> {
    /// A stale block that must be sent: `(offset, bytes)`. The slice
    /// borrows the iterator's buffer and is valid until the next call.
    Stale { offset: u64, bytes: &'a [u8] },
    /// review otp-7b-1 F1: emitted when the configured keepalive
    /// interval elapsed while skipping MATCHING blocks. A long
    /// mostly-matching scan produces no stale blocks — and therefore no
    /// socket traffic — for as long as the file takes to read+hash; a
    /// receiver guarding its socket with the transfer stall timeout
    /// would abort a perfectly healthy resume. The data-plane caller
    /// answers this event with a zero-length `BLOCK` record (a no-op
    /// in-place write at `offset`) so the socket shows liveness; the
    /// in-stream caller ignores it (the control lane carries no stall
    /// guard). Never emitted unless [`ResumeBlockDiff::with_keepalive`]
    /// arms it.
    KeepAlive { offset: u64 },
}

/// Sequential block reader + staleness filter over one resume-flagged
/// source file. Yields the stale blocks (and, when armed, keepalive
/// ticks); fresh blocks (hash match) are read, verified, and skipped.
/// The manifest promised `header.size`, so hitting EOF short of it, or a
/// read error, ends the diff with a `source:`-prefixed error
/// (`Self::next_event`'s `Err`) that the caller closes the record with
/// as a FAILED `BlockComplete` (contract v7, SOURCE_SIDE_CONTAINMENT
/// ssc-3) — never pad, never silently truncate, never end the session.
pub struct ResumeBlockDiff {
    reader: OpenedSourceFile,
    size: u64,
    block_size: usize,
    dest_hashes: Vec<Vec<u8>>,
    buf: Vec<u8>,
    offset: u64,
    index: usize,
    keepalive: Option<Duration>,
    last_emit: Instant,
    /// cr-ssc3-1: the post-diff re-stat runs exactly once.
    end_checked: bool,
}

impl ResumeBlockDiff {
    /// Open the source file for `header` and prepare the diff at
    /// `block_size` (the DESTINATION chose it — plan D5 — and the
    /// caller range-validated it at frame arrival).
    pub async fn open(
        source: &Arc<dyn TransferSource>,
        header: &FileHeader,
        block_size: usize,
        dest_hashes: Vec<Vec<u8>>,
    ) -> Result<Self> {
        // otp-7b-2 review G2: the open failure names its file too, on
        // both carriers.
        let mut reader = source
            .open_file(header)
            .await
            .map_err(|e| e.wrap_err(FaultedPath(header.relative_path.clone())))?;
        // cr-ssc3-1 (D-C, the whole-file rule applied to resume): the
        // opened handle must still be the manifest's size before any
        // block goes out. A grown or shrunk file is skipped here — never
        // resumed short and stamped as converged.
        let now = match reader.len().await {
            Ok(now) => now,
            Err(e) => {
                return Err(eyre::eyre!("source: cannot read metadata: {e}")
                    .wrap_err(FaultedPath(header.relative_path.clone())))
            }
        };
        if now != header.size {
            return Err(eyre::eyre!(
                "source: changed size during transfer (manifest {} bytes, now {now})",
                header.size
            )
            .wrap_err(FaultedPath(header.relative_path.clone())));
        }
        Ok(Self {
            reader,
            size: header.size,
            block_size,
            dest_hashes,
            buf: vec![0u8; block_size],
            offset: 0,
            index: 0,
            keepalive: None,
            last_emit: Instant::now(),
            end_checked: false,
        })
    }

    /// The `source:` skip reason for an `open` failure: the innermost
    /// `source:`-prefixed message when the failure is the file's own
    /// (size drift, metadata), else the open error itself.
    pub fn skip_reason(err: &eyre::Report) -> String {
        for cause in err.chain() {
            let text = cause.to_string();
            if text.starts_with("source:") {
                return text;
            }
        }
        format!("source: cannot open: {err:#}")
    }

    /// Arm keepalive ticks: a [`ResumeDiffEvent::KeepAlive`] is emitted
    /// whenever `interval` passes without a stale block being yielded.
    pub fn with_keepalive(mut self, interval: Duration) -> Self {
        self.keepalive = Some(interval);
        self.last_emit = Instant::now();
        self
    }

    /// The next diff event, or `None` when the file is exhausted.
    pub async fn next_event(&mut self) -> Result<Option<ResumeDiffEvent<'_>>> {
        while self.offset < self.size {
            let this = (self.size - self.offset).min(self.block_size as u64) as usize;
            let mut filled = 0usize;
            while filled < this {
                let got = match self.reader.read(&mut self.buf[filled..this]).await {
                    Ok(got) => got,
                    Err(e) => return Err(eyre::eyre!("source: read error: {e}")),
                };
                if got == 0 {
                    // Shorter on disk than the manifest promised: the
                    // record cannot complete at `size`. Report the size
                    // the handle sees now (the bytes actually read when
                    // the handle cannot say).
                    let read_so_far = self.offset + filled as u64;
                    let now = self.reader.len().await.unwrap_or(read_so_far);
                    return Err(eyre::eyre!(
                        "source: changed size during transfer (manifest {} bytes, now {now})",
                        self.size
                    ));
                }
                filled += got;
            }
            let stale = match self.dest_hashes.get(self.index) {
                Some(expected) => blake3::hash(&self.buf[..this]).as_bytes()[..] != expected[..],
                None => true,
            };
            let block_offset = self.offset;
            self.offset += this as u64;
            self.index += 1;
            if stale {
                self.last_emit = Instant::now();
                return Ok(Some(ResumeDiffEvent::Stale {
                    offset: block_offset,
                    bytes: &self.buf[..this],
                }));
            }
            if let Some(interval) = self.keepalive {
                if self.last_emit.elapsed() >= interval {
                    self.last_emit = Instant::now();
                    return Ok(Some(ResumeDiffEvent::KeepAlive {
                        offset: block_offset,
                    }));
                }
            }
        }
        // cr-ssc3-1: the diff read exactly `size` bytes; before the
        // record can close ok, the same handle must still report that
        // size — a file that grew while being diffed would otherwise be
        // finalised short (and, under `move --resume`, its tail deleted
        // at the source). Exactly once.
        if !self.end_checked {
            self.end_checked = true;
            let now = match self.reader.len().await {
                Ok(now) => now,
                Err(e) => return Err(eyre::eyre!("source: cannot read metadata: {e}")),
            };
            if now != self.size {
                return Err(eyre::eyre!(
                    "source: changed size during transfer (manifest {} bytes, now {now})",
                    self.size
                ));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::transfer::source::FsTransferSource;

    /// review otp-7b-1 F1: an all-matching scan armed with keepalive
    /// yields one KeepAlive per skipped block at a zero interval (the
    /// liveness signal exists), and yields nothing when unarmed (the
    /// in-stream carrier's behavior is unchanged).
    #[tokio::test]
    async fn all_matching_scan_emits_keepalives_when_armed() {
        let dir = tempfile::tempdir().unwrap();
        let bs = 64 * 1024usize;
        let content: Vec<u8> = (0..3 * bs).map(|i| (i % 251) as u8).collect();
        std::fs::write(dir.path().join("f.bin"), &content).unwrap();
        let header = FileHeader {
            relative_path: "f.bin".to_string(),
            size: content.len() as u64,
            ..Default::default()
        };
        let hashes: Vec<Vec<u8>> = content
            .chunks(bs)
            .map(|c| blake3::hash(c).as_bytes().to_vec())
            .collect();
        let source: Arc<dyn TransferSource> =
            Arc::new(FsTransferSource::new(dir.path().to_path_buf()));

        // Armed at zero interval: every matching block ticks.
        let mut diff = ResumeBlockDiff::open(&source, &header, bs, hashes.clone())
            .await
            .unwrap()
            .with_keepalive(Duration::ZERO);
        let mut keepalives = 0;
        while let Some(event) = diff.next_event().await.unwrap() {
            match event {
                ResumeDiffEvent::Stale { .. } => panic!("all blocks match; nothing is stale"),
                ResumeDiffEvent::KeepAlive { offset } => {
                    assert!(offset < content.len() as u64);
                    keepalives += 1;
                }
            }
        }
        assert_eq!(keepalives, 3, "one tick per skipped block at interval 0");

        // Unarmed: an all-matching scan yields nothing at all.
        let mut diff = ResumeBlockDiff::open(&source, &header, bs, hashes)
            .await
            .unwrap();
        assert!(
            diff.next_event().await.unwrap().is_none(),
            "unarmed diff emits no keepalives"
        );
    }
}
