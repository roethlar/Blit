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
