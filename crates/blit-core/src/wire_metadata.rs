//! Canonical filesystem-metadata conversion for wire and presentation fields.
//!
//! Producers use one error convention: an unavailable modification time is
//! [`None`]. Wire fields that require an integer must choose their fallback
//! explicitly at the call site (currently zero). Pre-epoch times retain the
//! existing negative-seconds representation.

use std::fs::Metadata;
use std::time::{SystemTime, UNIX_EPOCH};

fn system_time_to_unix_seconds(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(error) => -(error.duration().as_secs() as i64),
    }
}

/// Convert filesystem modification time to signed Unix seconds.
///
/// Returns `None` when the platform cannot read the modification time.
pub fn mtime_seconds(metadata: &Metadata) -> Option<i64> {
    metadata.modified().ok().map(system_time_to_unix_seconds)
}

/// Return the Unix permission mode carried by transfer headers.
///
/// Non-Unix peers carry zero; Windows attributes use the separate Windows
/// metadata contract.
pub fn permissions_mode(metadata: &Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn signed_unix_seconds_preserve_both_epoch_sides() {
        let after = UNIX_EPOCH + Duration::from_secs(17);
        let before = UNIX_EPOCH - Duration::from_secs(9);
        assert_eq!(system_time_to_unix_seconds(after), 17);
        assert_eq!(system_time_to_unix_seconds(before), -9);
    }

    #[test]
    fn metadata_mtime_is_available_for_a_regular_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("file");
        fs::write(&path, b"x").expect("write fixture");
        assert!(mtime_seconds(&fs::metadata(path).expect("metadata")).is_some());
    }

    #[cfg(unix)]
    #[test]
    fn permissions_preserve_unix_mode_bits() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("file");
        fs::write(&path, b"x").expect("write fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("set permissions");
        assert_eq!(
            permissions_mode(&fs::metadata(path).expect("metadata")) & 0o777,
            0o640
        );
    }

    #[cfg(not(unix))]
    #[test]
    fn permissions_are_zero_off_unix() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("file");
        fs::write(&path, b"x").expect("write fixture");
        assert_eq!(permissions_mode(&fs::metadata(path).expect("metadata")), 0);
    }
}
