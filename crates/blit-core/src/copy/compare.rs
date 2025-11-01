use crate::checksum::{self, ChecksumType};
use eyre::Result;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

/// Check if a file needs to be copied (for mirror mode)
pub fn file_needs_copy(src: &Path, dst: &Path, use_checksum: bool) -> Result<bool> {
    if !dst.exists() {
        return Ok(true);
    }

    let src_meta = src.metadata()?;
    let dst_meta = dst.metadata()?;

    if src_meta.len() != dst_meta.len() {
        return Ok(true);
    }

    if use_checksum {
        Ok(file_needs_copy_with_checksum_type(
            src,
            dst,
            Some(ChecksumType::Blake3),
        )?)
    } else {
        let src_time = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let dst_time = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        Ok(src_time
            .duration_since(dst_time)
            .is_ok_and(|diff| diff.as_secs() > 2))
    }
}

/// Like file_needs_copy, but with explicit checksum selection.
pub fn file_needs_copy_with_checksum_type(
    src: &Path,
    dst: &Path,
    checksum: Option<ChecksumType>,
) -> Result<bool> {
    if !dst.exists() {
        return Ok(true);
    }
    let src_meta = src.metadata()?;
    let dst_meta = dst.metadata()?;
    if src_meta.len() != dst_meta.len() {
        return Ok(true);
    }

    let ph_bytes = 1024 * 1024; // 1 MiB
    let src_ph = checksum::partial_hash_first_last(src, ph_bytes)?;
    let dst_ph = checksum::partial_hash_first_last(dst, ph_bytes)?;
    if src_ph != dst_ph {
        return Ok(true);
    }
    match checksum {
        Some(ChecksumType::Blake3) => {
            let a = checksum::hash_file(src, ChecksumType::Blake3)?;
            let b = checksum::hash_file(dst, ChecksumType::Blake3)?;
            Ok(a != b)
        }
        Some(_) | None => {
            let src_time = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let dst_time = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            Ok(src_time
                .duration_since(dst_time)
                .is_ok_and(|diff| diff.as_secs() > 2))
        }
    }
}

#[allow(dead_code)]
fn files_have_different_content(src: &Path, dst: &Path) -> Result<bool> {
    let src_hash = hash_file_content(src)?;
    let dst_hash = hash_file_content(dst)?;
    Ok(src_hash != dst_hash)
}

#[allow(dead_code)]
fn hash_file_content(path: &Path) -> Result<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut file = File::open(path)?;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().into())
}
