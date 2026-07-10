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

use eyre::Result;
use tokio::io::AsyncReadExt;

use crate::generated::FileHeader;

use super::source::TransferSource;

/// Sequential block reader + staleness filter over one resume-flagged
/// source file. Yields only the stale blocks; fresh blocks (hash match)
/// are read, verified, and skipped. The manifest promised
/// `header.size`, so hitting EOF short of it aborts exactly as a
/// whole-file record does — never pad, never silently truncate.
pub struct ResumeBlockDiff {
    reader: Box<dyn tokio::io::AsyncRead + Unpin + Send>,
    relative_path: String,
    size: u64,
    block_size: usize,
    dest_hashes: Vec<Vec<u8>>,
    buf: Vec<u8>,
    offset: u64,
    index: usize,
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
        let reader = source.open_file(header).await?;
        Ok(Self {
            reader,
            relative_path: header.relative_path.clone(),
            size: header.size,
            block_size,
            dest_hashes,
            buf: vec![0u8; block_size],
            offset: 0,
            index: 0,
        })
    }

    /// The next STALE block as `(offset, bytes)`, or `None` when the
    /// file is exhausted. The returned slice borrows this iterator's
    /// buffer and is valid until the next call.
    pub async fn next_stale(&mut self) -> Result<Option<(u64, &[u8])>> {
        while self.offset < self.size {
            let this = (self.size - self.offset).min(self.block_size as u64) as usize;
            let mut filled = 0usize;
            while filled < this {
                let got = self
                    .reader
                    .read(&mut self.buf[filled..this])
                    .await
                    .map_err(|e| {
                        eyre::Report::new(e)
                            .wrap_err(super::faulted_path::FaultedPath(self.relative_path.clone()))
                    })?;
                if got == 0 {
                    return Err(eyre::eyre!(
                        "'{}' hit EOF with {} bytes still promised",
                        self.relative_path,
                        self.size - self.offset - filled as u64
                    )
                    .wrap_err(super::faulted_path::FaultedPath(self.relative_path.clone())));
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
                return Ok(Some((block_offset, &self.buf[..this])));
            }
        }
        Ok(None)
    }
}
