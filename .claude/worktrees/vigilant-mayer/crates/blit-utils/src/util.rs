use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use eyre::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

pub enum Endpoint {
    Local(PathBuf),
    Remote(RemoteEndpoint),
}

pub fn parse_endpoint_or_local(input: &str) -> Endpoint {
    match RemoteEndpoint::parse(input) {
        Ok(endpoint) => Endpoint::Remote(endpoint),
        Err(_) => Endpoint::Local(PathBuf::from(input)),
    }
}

pub fn module_and_rel_path(remote: &RemoteEndpoint) -> Result<(String, PathBuf)> {
    match &remote.path {
        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
        RemotePath::Root { .. } => {
            bail!("module name required (server:/module/...)");
        }
        RemotePath::Discovery => {
            bail!("remote target must include a module path");
        }
    }
}

pub fn rel_path_to_string(path: &Path) -> String {
    if path.as_os_str().is_empty() || path == Path::new(".") {
        String::new()
    } else {
        path.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

pub fn append_completion_prefix(base: &Path, extra: Option<&str>) -> String {
    let mut prefix = rel_path_to_string(base);
    if !prefix.is_empty() && !prefix.ends_with('/') {
        prefix.push('/');
    }
    if let Some(extra) = extra {
        prefix.push_str(extra);
    }
    prefix
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.2} {}", UNITS[unit])
}

pub fn metadata_mtime_seconds(meta: &fs::Metadata) -> Option<i64> {
    use std::time::UNIX_EPOCH;

    let modified = meta.modified().ok()?;
    match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_secs() as i64),
        Err(err) => {
            let dur = err.duration();
            Some(-(dur.as_secs() as i64))
        }
    }
}
