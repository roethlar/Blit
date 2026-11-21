use crate::fs_capability::{get_platform_capability, FilesystemCapability};
use eyre::Result;
use std::path::Path;

pub(crate) fn preserve_metadata(src: &Path, dst: &Path) -> Result<()> {
    let fs_cap = get_platform_capability();
    let preserved = fs_cap.preserve_metadata(src, dst)?;

    if !preserved.mtime {
        log::debug!("Could not preserve mtime for {}", dst.display());
    }
    if !preserved.permissions {
        log::debug!("Could not preserve permissions for {}", dst.display());
    }

    Ok(())
}

/// ssc-4 (SOURCE_SIDE_CONTAINMENT D-C): preserve mtime (and, on Unix,
/// permissions) onto `dst` from the OPENED source handle's metadata —
/// the inode that was copied — never from a path re-stat that could
/// describe a replacement.
/// [`preserve_metadata_from_handle`] onto a destination that is HELD
/// (cr-ecm15-1): its time and mode set through `made`, never by a name.
pub(crate) fn preserve_metadata_into(src: &std::fs::File, made: &std::fs::File) -> Result<()> {
    use filetime::{set_file_handle_times, FileTime};
    let md = src.metadata()?;
    if let Ok(modified) = md.modified() {
        if set_file_handle_times(made, None, Some(FileTime::from_system_time(modified))).is_err() {
            log::debug!("Could not preserve mtime through the destination's handle");
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = md.permissions().mode();
        if made
            .set_permissions(std::fs::Permissions::from_mode(mode))
            .is_err()
        {
            log::debug!("Could not preserve permissions through the destination's handle");
        }
    }
    Ok(())
}

pub(crate) fn preserve_metadata_from_handle(src: &std::fs::File, dst: &Path) -> Result<()> {
    use filetime::{set_file_mtime, FileTime};
    let md = src.metadata()?;
    if let Ok(modified) = md.modified() {
        if set_file_mtime(dst, FileTime::from_system_time(modified)).is_err() {
            log::debug!("Could not preserve mtime for {}", dst.display());
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = md.permissions().mode();
        if std::fs::set_permissions(dst, std::fs::Permissions::from_mode(mode)).is_err() {
            log::debug!("Could not preserve permissions for {}", dst.display());
        }
    }
    Ok(())
}
