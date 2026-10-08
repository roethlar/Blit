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

/// The platform's per-user folder: `~/Library/Application Support/com.Blit.Blit`
/// on macOS, `$XDG_CONFIG_HOME/blit` (`~/.config/blit`) on Linux, and
/// `%LOCALAPPDATA%\Blit` on Windows — the local profile, not the roaming one
/// (JOB_LOGS R9, jl-2): job logs can be large, and a roaming profile copies
/// its folder at every sign-in.
fn platform_config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        BaseDirs::new().map(|base| base.data_local_dir().join("Blit"))
    }
    #[cfg(not(windows))]
    {
        ProjectDirs::from("com", "Blit", "Blit").map(|proj| proj.config_dir().to_path_buf())
    }
}

/// Before jl-2, Windows kept the per-user folder in the roaming profile
/// (`%APPDATA%\Blit\Blit\config`); move what is there into `new` once per
/// process. A failure is a warning: the files stay where they were.
#[cfg(windows)]
fn migrate_roaming_dir(new: &Path) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(old) =
            ProjectDirs::from("com", "Blit", "Blit").map(|proj| proj.config_dir().to_path_buf())
        else {
            return;
        };
        if let Err(error) = move_folder_contents(&old, new) {
            log::warn!(
                "could not move blit's settings from {} to {}: {error}",
                old.display(),
                new.display()
            );
        }
    });
}

/// Move each entry of `old` that `new` does not already hold into `new`, then
/// remove `old` and its two parents when that leaves them empty. Nothing in
/// `new` is overwritten; an entry it already holds stays in `old`. Returns
/// how many entries moved, or the first error after trying them all.
#[cfg(any(windows, test))]
fn move_folder_contents(old: &Path, new: &Path) -> std::io::Result<usize> {
    let entries = match std::fs::read_dir(old) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    std::fs::create_dir_all(new)?;
    let mut moved = 0;
    let mut first_error = None;
    for entry in entries {
        let result = entry.and_then(|entry| {
            let target = new.join(entry.file_name());
            if target.exists() {
                return Ok(false);
            }
            std::fs::rename(entry.path(), target).map(|()| true)
        });
        match result {
            Ok(true) => moved += 1,
            Ok(false) => {}
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    // `remove_dir` removes only an empty folder, so this stops at the first
    // ancestor that still holds anything (and never reaches the profile).
    for dir in old.ancestors().take(3) {
        if std::fs::remove_dir(dir).is_err() {
            break;
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(moved),
    }
}

/// The per-user settings file in [`config_dir`] (JOB_LOGS R9/R10): TOML, so
/// a person can edit it and leave comments.
pub const SETTINGS_FILE: &str = "config.toml";

/// Per-user settings from `config.toml`. Every field has a default; a
/// missing file means all defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// `[jobs] keep`: how many finished job logs this machine keeps (R7).
    pub jobs_keep: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            jobs_keep: crate::job_log::DEFAULT_KEEP,
        }
    }
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawSettings {
    #[serde(default)]
    jobs: RawJobsSettings,
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawJobsSettings {
    keep: Option<usize>,
}

/// Read `config.toml` in `dir`. A missing file is all defaults; a file that
/// cannot be read or parsed is an error naming it — a caller that must not
/// fail (logging) warns and uses the defaults.
pub fn load_settings(dir: &Path) -> Result<Settings> {
    let path = dir.join(SETTINGS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Settings::default())
        }
        Err(error) => return Err(eyre!("could not read {}: {error}", path.display())),
    };
    let raw: RawSettings = toml::from_str(&text)
        .map_err(|error| eyre!("could not parse {}: {error}", path.display()))?;
    let defaults = Settings::default();
    Ok(Settings {
        jobs_keep: raw.jobs.keep.unwrap_or(defaults.jobs_keep),
    })
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

/// Resolve the configuration directory — the per-user folder for settings,
/// history and job records.
/// Priority: explicit override -> platform standard -> ~/.config/blit
pub fn config_dir() -> Result<PathBuf> {
    if let Some(path) = CONFIG_DIR_OVERRIDE.read().clone() {
        return Ok(path);
    }

    if let Some(dir) = platform_config_dir() {
        #[cfg(windows)]
        migrate_roaming_dir(&dir);
        return Ok(dir);
    }

    if let Some(base) = BaseDirs::new() {
        return Ok(base.home_dir().join(".config").join("blit"));
    }

    Err(eyre!(
        "unable to determine configuration directory for blit (no override and no platform default)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_and_read_the_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_settings(dir.path()).unwrap(), Settings::default());
        assert_eq!(Settings::default().jobs_keep, 50);
        std::fs::write(
            dir.path().join("config.toml"),
            "# how many job logs\n[jobs]\nkeep = 7\n",
        )
        .unwrap();
        assert_eq!(load_settings(dir.path()).unwrap().jobs_keep, 7);
        std::fs::write(dir.path().join("config.toml"), "[jobs]\nkeep = \"many\"\n").unwrap();
        let error = load_settings(dir.path()).unwrap_err().to_string();
        assert!(error.contains("config.toml"), "{error}");
    }

    #[test]
    fn the_old_folder_moves_once_without_overwriting() {
        let root = tempfile::tempdir().unwrap();
        let old = root
            .path()
            .join("Roaming")
            .join("Blit")
            .join("Blit")
            .join("config");
        let new = root.path().join("Local").join("Blit");
        std::fs::create_dir_all(old.join("jobs")).unwrap();
        std::fs::write(old.join("recents.jsonl"), b"old").unwrap();
        std::fs::write(old.join("jobs").join("x"), b"x").unwrap();
        std::fs::write(old.join("perf_local.jsonl"), b"old perf").unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("perf_local.jsonl"), b"new perf").unwrap();

        assert_eq!(move_folder_contents(&old, &new).unwrap(), 2);

        assert_eq!(std::fs::read(new.join("recents.jsonl")).unwrap(), b"old");
        assert_eq!(std::fs::read(new.join("jobs").join("x")).unwrap(), b"x");
        // Never overwritten; the old copy stays, so the folder does too.
        assert_eq!(
            std::fs::read(new.join("perf_local.jsonl")).unwrap(),
            b"new perf"
        );
        assert_eq!(
            std::fs::read(old.join("perf_local.jsonl")).unwrap(),
            b"old perf"
        );

        std::fs::remove_file(old.join("perf_local.jsonl")).unwrap();
        assert_eq!(move_folder_contents(&old, &new).unwrap(), 0);
        // Empty now: the old folder and its two Blit parents are gone.
        assert!(!root.path().join("Roaming").join("Blit").exists());
        assert!(root.path().join("Roaming").exists());
        // Nothing there at all is fine.
        assert_eq!(move_folder_contents(&old, &new).unwrap(), 0);
    }

    #[cfg(windows)]
    #[test]
    fn windows_keeps_the_folder_in_the_local_profile() {
        let dir = platform_config_dir().expect("a per-user folder");
        let local = BaseDirs::new().unwrap().data_local_dir().to_path_buf();
        assert_eq!(dir, local.join("Blit"));
    }
}
