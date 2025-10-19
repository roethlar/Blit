//! Checksum and hashing utilities

use eyre::{bail, Context, Result};
use log::warn;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Available checksum algorithms
#[derive(Debug, Clone, Copy)]
pub enum ChecksumType {
    Blake3,
    XxHash3,
    Md5, // For compatibility
}

impl Default for ChecksumType {
    fn default() -> Self {
        Self::Blake3
    }
}

/// CHAR_OFFSET constant from rsync (for compatibility)
const CHAR_OFFSET: u32 = 31;

/// Fast rolling checksum implementation (based on rsync's get_checksum1)
/// This implements the same algorithm as rsync for compatibility
/// Rolling checksum state: `s1` and `s2` are the Adler-style accumulators
/// used by the rsync-compatible algorithm. `block_size` is the window size
/// (in bytes) for rolling operations and must fit in `u32`.
pub struct RollingChecksum {
    s1: u32,
    s2: u32,
    block_size: u32,
}

impl RollingChecksum {
    pub fn new(block_size: usize) -> Result<Self> {
        let bs: u32 = block_size.try_into().with_context(|| {
            format!(
                "block_size {} exceeds u32::MAX for rolling checksum",
                block_size
            )
        })?;
        Ok(Self {
            s1: 0,
            s2: 0,
            block_size: bs,
        })
    }

    /// Initialize checksum for a block of data (rsync-compatible)
    pub fn init(&mut self, data: &[u8]) {
        self.s1 = 0;
        self.s2 = 0;

        // Use rsync's algorithm: process 4 bytes at a time for speed
        let mut iter = data.chunks_exact(4);
        for chunk in &mut iter {
            let b0 = chunk[0] as u32;
            let b1 = chunk[1] as u32;
            let b2 = chunk[2] as u32;
            let b3 = chunk[3] as u32;

            self.s2 = self.s2.wrapping_add(
                4u32.wrapping_mul(self.s1.wrapping_add(b0))
                    .wrapping_add(3u32.wrapping_mul(b1))
                    .wrapping_add(2u32.wrapping_mul(b2))
                    .wrapping_add(b3)
                    .wrapping_add(10u32.wrapping_mul(CHAR_OFFSET)),
            );

            self.s1 = self.s1.wrapping_add(
                b0.wrapping_add(b1)
                    .wrapping_add(b2)
                    .wrapping_add(b3)
                    .wrapping_add(4u32.wrapping_mul(CHAR_OFFSET)),
            );
        }

        // Process remaining bytes
        for &byte in iter.remainder() {
            let v = byte as u32;
            self.s1 = self.s1.wrapping_add(v.wrapping_add(CHAR_OFFSET));
            self.s2 = self.s2.wrapping_add(self.s1);
        }
    }

    /// Roll the checksum by removing old byte and adding new byte
    pub fn roll(&mut self, old_byte: u8, new_byte: u8) {
        let old = old_byte as u32 + CHAR_OFFSET;
        let new = new_byte as u32 + CHAR_OFFSET;

        self.s1 = self.s1.wrapping_sub(old).wrapping_add(new);
        self.s2 = self
            .s2
            .wrapping_sub(self.block_size * old)
            .wrapping_add(self.s1);
    }

    /// Get current checksum value (rsync format: s1 in lower 16 bits, s2 in upper 16 bits)
    pub fn value(&self) -> u32 {
        (self.s1 & 0xFFFF) | (self.s2 << 16)
    }
}

/// Compute rolling checksum for a block (rsync-compatible)
pub fn rsync_rolling_checksum(data: &[u8]) -> Result<u32> {
    RollingChecksum::compute(data)
}

impl RollingChecksum {
    /// Convenience method: compute the rolling checksum for a slice
    pub fn compute(data: &[u8]) -> Result<u32> {
        let mut checksum = RollingChecksum::new(data.len())?;
        checksum.init(data);
        Ok(checksum.value())
    }
}

/// Compute strong checksum for data
pub fn strong_checksum(
    data: &[u8],
    checksum_type: ChecksumType,
    allow_md5: bool,
) -> Result<Vec<u8>> {
    match checksum_type {
        ChecksumType::Blake3 => {
            let hash = blake3::hash(data);
            Ok(hash.as_bytes().to_vec())
        }
        ChecksumType::XxHash3 => {
            let h = xxhash_rust::xxh3::xxh3_64(data);
            Ok(h.to_be_bytes().to_vec())
        }
        ChecksumType::Md5 => {
            // SECURITY WARNING: MD5 is cryptographically broken
            if !allow_md5 {
                bail!("MD5 is disabled; enable explicitly to use for compatibility");
            }
            warn!("MD5 is cryptographically broken; prefer Blake3 (default) or SHA-256");
            Ok(md5::compute(data).to_vec())
        }
    }
}

/// Hash a whole file with the given algorithm.
pub fn hash_file(path: &Path, ty: ChecksumType) -> Result<Vec<u8>> {
    match ty {
        ChecksumType::Blake3 => {
            let mut hasher = blake3::Hasher::new();
            let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
            let mut buf = vec![0u8; 256 * 1024];
            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            Ok(hasher.finalize().as_bytes().to_vec())
        }
        ChecksumType::XxHash3 => {
            let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
            let mut buf = vec![0u8; 256 * 1024];
            let mut state = xxhash_rust::xxh3::Xxh3::new();
            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                state.update(&buf[..n]);
            }
            Ok(state.digest().to_be_bytes().to_vec())
        }
        ChecksumType::Md5 => {
            let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
            let mut ctx = md5::Context::new();
            let mut buf = vec![0u8; 256 * 1024];
            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                ctx.consume(&buf[..n]);
            }
            Ok(ctx.compute().to_vec())
        }
    }
}

/// Compute a partial hash consisting of the first and last `bytes` of the file using BLAKE3.
/// If the file is smaller than 2*bytes, the whole file is hashed.
pub fn partial_hash_first_last(path: &Path, bytes: usize) -> Result<Vec<u8>> {
    let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let len = f.metadata()?.len();
    let mut hasher = blake3::Hasher::new();
    if len as usize <= bytes * 2 {
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
    } else {
        let mut first = vec![0u8; bytes];
        f.read_exact(&mut first)?;
        hasher.update(b"FIRST");
        hasher.update(&first);

        f.seek(SeekFrom::End(-(bytes as i64)))?;
        let mut last = vec![0u8; bytes];
        f.read_exact(&mut last)?;
        hasher.update(b"LAST");
        hasher.update(&last);
        hasher.update(&len.to_le_bytes());
    }
    Ok(hasher.finalize().as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_checksum_basic() {
        let data = b"Hello, World!";
        let checksum = rsync_rolling_checksum(data).unwrap();

        // Verify it produces a consistent result
        let checksum2 = rsync_rolling_checksum(data).unwrap();
        assert_eq!(checksum, checksum2);
    }

    #[test]
    fn test_rolling_checksum_rolling() {
        let data = b"abcdef";
        let mut rolling = RollingChecksum::new(3).unwrap();

        // Initialize with first 3 bytes: "abc"
        rolling.init(&data[0..3]);
        let initial = rolling.value();

        // Roll to next position: remove 'a', add 'd' -> "bcd"
        rolling.roll(data[0], data[3]);
        let rolled = rolling.value();

        // Should be different
        assert_ne!(initial, rolled);

        // Verify by computing fresh checksum for "bcd"
        let fresh = rsync_rolling_checksum(&data[1..4]).unwrap();
        assert_eq!(rolled, fresh);
    }

    #[test]
    fn test_rolling_checksum_edge_lengths() {
        for n in 0..=3 {
            let data = vec![0xAAu8; n];
            let _ = rsync_rolling_checksum(&data).unwrap();
        }
    }
}
