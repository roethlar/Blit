//! Resumable file copy with block-level hash comparison.
//!
//! Compares existing destination content block-by-block with source using Blake3 hashes,
//! only transferring blocks that differ. This allows resuming interrupted transfers
//! and efficiently updating files with partial changes.
//!
//! Using hash comparison (vs raw bytes) enables the same logic for remote transfers
//! where only hashes are exchanged over the network.

use eyre::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Default block size for comparison (1 MiB).
pub const DEFAULT_BLOCK_SIZE: usize = 1024 * 1024;

/// Maximum block size for comparison (64 MiB) to prevent excessive memory usage.
pub const MAX_BLOCK_SIZE: usize = 64 * 1024 * 1024;

/// Outcome of a resume copy operation.
#[derive(Debug)]
pub struct ResumeCopyOutcome {
    /// Total bytes in the final file.
    pub total_bytes: u64,
    /// Bytes that were actually transferred (not skipped).
    pub bytes_transferred: u64,
    /// Number of blocks that matched and were skipped.
    pub blocks_skipped: u64,
    /// Number of blocks that were transferred.
    pub blocks_transferred: u64,
}

/// Compute Blake3 hash of a data block.
#[inline]
fn hash_block(data: &[u8]) -> blake3::Hash {
    blake3::hash(data)
}

/// Copy a file with resume capability using block-level hash comparison.
///
/// Compares source and destination block-by-block using Blake3 hashes:
/// - Matching blocks (same hash) are skipped
/// - Mismatched blocks are overwritten from source
/// - If source is longer, remaining bytes are appended
/// - If source is shorter, destination is truncated
///
/// This is efficient for:
/// - Resuming interrupted transfers (partial files)
/// - Updating files with localized changes
/// - Verifying and fixing corrupted copies
pub fn resume_copy_file(src: &Path, dst: &Path, block_size: usize) -> Result<ResumeCopyOutcome> {
    let src_meta = std::fs::metadata(src)
        .with_context(|| format!("reading source metadata: {}", src.display()))?;
    let src_len = src_meta.len();

    // Get destination length (0 if doesn't exist)
    let dst_len = std::fs::metadata(dst).map(|m| m.len()).unwrap_or(0);

    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory: {}", parent.display()))?;
    }

    let mut src_file =
        File::open(src).with_context(|| format!("opening source: {}", src.display()))?;

    let mut dst_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(dst)
        .with_context(|| format!("opening destination: {}", dst.display()))?;

    let block_size = if block_size == 0 {
        DEFAULT_BLOCK_SIZE
    } else {
        block_size.min(MAX_BLOCK_SIZE)
    };
    let mut src_buf = vec![0u8; block_size];
    let mut dst_buf = vec![0u8; block_size];

    let mut offset = 0u64;
    let mut bytes_transferred = 0u64;
    let mut blocks_skipped = 0u64;
    let mut blocks_transferred = 0u64;

    while offset < src_len {
        let remaining = src_len - offset;
        let this_block = remaining.min(block_size as u64) as usize;

        // Read source block and compute hash
        src_file.seek(SeekFrom::Start(offset))?;
        src_file.read_exact(&mut src_buf[..this_block])?;
        let src_hash = hash_block(&src_buf[..this_block]);

        // Check if we can compare with destination
        let should_write = if offset < dst_len {
            let dst_available = (dst_len - offset).min(this_block as u64) as usize;

            if dst_available == this_block {
                // Full block available, read and hash
                dst_file.seek(SeekFrom::Start(offset))?;
                dst_file.read_exact(&mut dst_buf[..this_block])?;
                let dst_hash = hash_block(&dst_buf[..this_block]);
                src_hash != dst_hash
            } else {
                // Partial block at end of dest, need to write
                true
            }
        } else {
            // Beyond destination, definitely need to write
            true
        };

        if should_write {
            dst_file.seek(SeekFrom::Start(offset))?;
            dst_file.write_all(&src_buf[..this_block])?;
            bytes_transferred += this_block as u64;
            blocks_transferred += 1;
        } else {
            blocks_skipped += 1;
        }

        offset += this_block as u64;
    }

    // Truncate if destination is longer than source
    if dst_len > src_len {
        dst_file.set_len(src_len)?;
    }

    // Sync to disk
    dst_file.sync_all()?;

    Ok(ResumeCopyOutcome {
        total_bytes: src_len,
        bytes_transferred,
        blocks_skipped,
        blocks_transferred,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_resume_new_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create source file
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &data)?;

        // Copy to new destination
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 10000);
        assert_eq!(outcome.bytes_transferred, 10000);
        assert_eq!(outcome.blocks_skipped, 0);
        assert_eq!(std::fs::read(&dst)?, data);

        Ok(())
    }

    #[test]
    fn test_resume_partial_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create source file (10KB)
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &data)?;

        // Create partial destination (first 5KB)
        std::fs::write(&dst, &data[..5000])?;

        // Resume copy
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 10000);
        // First 4 blocks (4KB) should be skipped, rest transferred
        assert!(outcome.blocks_skipped >= 4);
        assert_eq!(std::fs::read(&dst)?, data);

        Ok(())
    }

    #[test]
    fn test_resume_identical_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create identical files
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &data)?;
        std::fs::write(&dst, &data)?;

        // Resume should skip all blocks
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 10000);
        assert_eq!(outcome.bytes_transferred, 0);
        assert_eq!(outcome.blocks_transferred, 0);

        Ok(())
    }

    #[test]
    fn test_resume_truncates_longer_dest() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create source (5KB)
        let src_data: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &src_data)?;

        // Create longer destination (10KB)
        let dst_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&dst, &dst_data)?;

        // Resume should truncate
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 5000);
        let result = std::fs::read(&dst)?;
        assert_eq!(result.len(), 5000);
        assert_eq!(result, src_data);

        Ok(())
    }

    #[test]
    fn test_resume_corrupted_block() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");

        // Create source (10KB)
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&src, &data)?;

        // Create destination with corrupted middle block
        let mut corrupted = data.clone();
        for i in 3000..4000 {
            corrupted[i] = 0xFF;
        }
        std::fs::write(&dst, &corrupted)?;

        // Resume should fix the corrupted block
        let outcome = resume_copy_file(&src, &dst, 1024)?;

        assert_eq!(outcome.total_bytes, 10000);
        assert!(outcome.blocks_transferred >= 1); // At least the corrupted block
        assert_eq!(std::fs::read(&dst)?, data);

        Ok(())
    }
}
