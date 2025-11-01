use crate::config::config_dir;
use eyre::{eyre, Context, Result};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn journal_store_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("journal_cache.json"))
}

pub fn canonicalize(path: &Path) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let norm = normpath::BasePath::new(std::env::current_dir()?)
            .map_err(|err| eyre!("failed to resolve base path for canonicalisation: {err}"))?;
        let joined = norm.join(path);
        return Ok(joined.into_path_buf());
    }

    #[cfg(not(windows))]
    {
        std::fs::canonicalize(path)
            .with_context(|| format!("failed to canonicalize path {}", path.display()))
    }
}

pub fn canonical_to_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub fn system_time_to_epoch_ms(st: SystemTime) -> Result<i64> {
    let duration = st
        .duration_since(UNIX_EPOCH)
        .map_err(|err| eyre!("system time before epoch: {err}"))?;
    let millis = duration.as_millis();
    let millis_i64 =
        i64::try_from(millis).map_err(|_| eyre!("system time milliseconds exceed i64 range"))?;
    Ok(millis_i64)
}
