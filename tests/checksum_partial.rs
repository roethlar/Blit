use anyhow::Result;
use std::fs;
use std::io::Write;

use blit_core::checksum;

#[test]
fn partial_hash_detects_mismatch() -> Result<()> {
    let tmp = tempfile::TempDir::new()?;
    let a = tmp.path().join("a.bin");
    let b = tmp.path().join("b.bin");

    // Same size, different last bytes
    let mut fa = fs::File::create(&a)?;
    let mut fb = fs::File::create(&b)?;
    let size = 2 * 1024 * 1024 + 16; // > 2 MiB
    fa.write_all(&vec![0xAA; size])?;
    fb.write_all(&vec![0xAA; size - 1])?;
    fb.write_all(&[0xBB])?;

    let ha = checksum::partial_hash_first_last(&a, 1024 * 1024)?;
    let hb = checksum::partial_hash_first_last(&b, 1024 * 1024)?;
    assert_ne!(ha, hb, "partial hash should differ");
    Ok(())
}

#[test]
fn file_needs_copy_partial_short_circuit() -> Result<()> {
    use blit_core::checksum::ChecksumType;
    use blit_core::copy::file_needs_copy_with_checksum_type as needs;
    let tmp = tempfile::TempDir::new()?;
    let a = tmp.path().join("a.bin");
    let b = tmp.path().join("b.bin");

    // Equal data
    let data = vec![7u8; 3 * 1024 * 1024];
    fs::write(&a, &data)?;
    fs::write(&b, &data)?;
    // Expect no copy when using Blake3 (full match)
    assert_eq!(needs(&a, &b, Some(ChecksumType::Blake3))?, false);

    // Modify last byte => partial hash should detect and ask to copy
    let mut d2 = data.clone();
    let last = d2.len() - 1;
    d2[last] ^= 0xFF;
    fs::write(&b, &d2)?;
    assert_eq!(needs(&a, &b, Some(ChecksumType::Blake3))?, true);
    Ok(())
}
