use directories::{BaseDirs, ProjectDirs};
use eyre::{eyre, Result};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};

static CONFIG_DIR_OVERRIDE: Lazy<RwLock<Option<PathBuf>>> = Lazy::new(|| RwLock::new(None));

/// Override the configuration directory for the current process.
/// Subsequent calls replace the previous override.
pub fn set_config_dir<P: AsRef<Path>>(path: P) {
    *CONFIG_DIR_OVERRIDE.write() = Some(path.as_ref().to_path_buf());
}

/// Clear any previously configured override.
pub fn clear_config_dir_override() {
    CONFIG_DIR_OVERRIDE.write().take();
}

/// Return the current override path, if one has been set.
pub fn config_dir_override() -> Option<PathBuf> {
    CONFIG_DIR_OVERRIDE.read().clone()
}

/// Where a daemon keeps its own state (performance history, job logs):
/// systemd's `$STATE_DIRECTORY` when the unit sets `StateDirectory=` (the
/// explicitly writable service data directory under
/// `ProtectSystem=strict`), else [`config_dir`] (foreground and dev runs,
/// where it is writable).
pub fn daemon_state_dir() -> Result<PathBuf> {
    if let Some(raw) = std::env::var_os("STATE_DIRECTORY") {
        // systemd passes a colon-separated list when multiple directories
        // are configured; the first is ours.
        if let Some(first) = std::env::split_paths(&raw).next() {
            if !first.as_os_str().is_empty() {
                return Ok(first);
            }
        }
    }
    config_dir()
}

/// Resolve the configuration directory.
/// Priority: explicit override -> platform standard -> ~/.config/blit
pub fn config_dir() -> Result<PathBuf> {
    if let Some(path) = CONFIG_DIR_OVERRIDE.read().clone() {
        return Ok(path);
    }

    if let Some(proj) = ProjectDirs::from("com", "Blit", "Blit") {
        return Ok(proj.config_dir().to_path_buf());
    }

    if let Some(base) = BaseDirs::new() {
        return Ok(base.home_dir().join(".config").join("blit"));
    }

    Err(eyre!(
        "unable to determine configuration directory for blit (no override and no platform default)"
    ))
}
