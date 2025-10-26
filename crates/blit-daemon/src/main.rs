use base64::{engine::general_purpose, Engine as _};
use blit_core::enumeration::{EntryKind, FileEnumerator};
use blit_core::fs_enum::FileFilter;
use blit_core::generated::blit_server::{Blit, BlitServer};
use blit_core::generated::{
    client_push_request, pull_chunk::Payload as PullPayload, server_push_response, Ack,
    ClientPushRequest, CompletionRequest, CompletionResponse, DataTransferNegotiation,
    DiskUsageEntry, DiskUsageRequest, FileData, FileHeader, FileInfo, FileList,
    FilesystemStatsRequest, FilesystemStatsResponse, FindEntry, FindRequest, ListModulesRequest,
    ListModulesResponse, ListRequest, ListResponse, ModuleInfo, PullChunk, PullRequest,
    PurgeRequest, PurgeResponse, PushSummary, ServerPushResponse,
};
use blit_core::mdns::{self, AdvertiseOptions, MdnsAdvertiser};
use clap::Parser;
use rand::{rngs::OsRng, RngCore};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sysinfo::Disks;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use eyre::{eyre, Context, Result};

type PushSender = mpsc::Sender<Result<ServerPushResponse, Status>>;
type PullSender = mpsc::Sender<Result<PullChunk, Status>>;
type FindSender = mpsc::Sender<Result<FindEntry, Status>>;
type DiskUsageSender = mpsc::Sender<Result<DiskUsageEntry, Status>>;

const TOKEN_LEN: usize = 32;
const FILE_LIST_BATCH_TARGET_BYTES: usize = 3 * 1024 * 1024;
const FILE_LIST_BATCH_MAX_ENTRIES: usize = 2048;

struct FileListBatcher {
    tx: PushSender,
    batch: Vec<String>,
    batch_bytes: usize,
    sent_any: bool,
}

impl FileListBatcher {
    fn new(tx: PushSender) -> Self {
        Self {
            tx,
            batch: Vec::new(),
            batch_bytes: 0,
            sent_any: false,
        }
    }

    async fn push(&mut self, path: String) -> Result<(), Status> {
        let entry_bytes = path.as_bytes().len();
        let would_exceed_size = self.batch_bytes + entry_bytes + 1 > FILE_LIST_BATCH_TARGET_BYTES;
        let would_exceed_count = self.batch.len() >= FILE_LIST_BATCH_MAX_ENTRIES;

        if !self.batch.is_empty() && (would_exceed_size || would_exceed_count) {
            self.flush().await?;
        }

        self.batch_bytes = self.batch_bytes.saturating_add(entry_bytes + 1);
        self.batch.push(path);
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Status> {
        if self.batch.is_empty() {
            return Ok(());
        }

        self.sent_any = true;
        let payload = server_push_response::Payload::FilesToUpload(FileList {
            relative_paths: std::mem::take(&mut self.batch),
        });
        self.batch_bytes = 0;
        send_control_message(&self.tx, payload).await
    }

    async fn finish(mut self) -> Result<(), Status> {
        if !self.batch.is_empty() {
            self.flush().await?;
        } else if !self.sent_any {
            send_control_message(
                &self.tx,
                server_push_response::Payload::FilesToUpload(FileList {
                    relative_paths: Vec::new(),
                }),
            )
            .await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ModuleConfig {
    name: String,
    path: PathBuf,
    read_only: bool,
    _comment: Option<String>,
    _use_chroot: bool,
}

#[derive(Debug, Clone)]
struct RootExport {
    path: PathBuf,
    read_only: bool,
    use_chroot: bool,
}

#[derive(Debug, Clone)]
struct RootSpec {
    path: PathBuf,
    read_only: bool,
    use_chroot: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct DeletionStats {
    files: u64,
    dirs: u64,
}

impl DeletionStats {
    fn total(self) -> u64 {
        self.files + self.dirs
    }
}

#[derive(Debug, Clone, Default)]
struct MdnsConfig {
    disabled: bool,
    name: Option<String>,
}

#[derive(Debug)]
struct DaemonRuntime {
    bind_host: String,
    port: u16,
    modules: HashMap<String, ModuleConfig>,
    default_root: Option<RootExport>,
    mdns: MdnsConfig,
    motd: Option<String>,
    warnings: Vec<String>,
}

fn sanitize_request_paths(paths: Vec<String>) -> Result<Vec<PathBuf>, Status> {
    let mut sanitized = Vec::new();
    for raw in paths {
        if raw.trim().is_empty() {
            return Err(Status::invalid_argument(
                "paths_to_delete cannot contain empty entries",
            ));
        }
        let rel = resolve_relative_path(&raw)?;
        if rel.as_os_str().is_empty() || rel == Path::new(".") {
            return Err(Status::invalid_argument(
                "refusing to delete module root; specify a sub-path",
            ));
        }
        sanitized.push(rel);
    }
    Ok(sanitized)
}

async fn delete_rel_paths(
    module_path: PathBuf,
    rel_paths: Vec<PathBuf>,
) -> Result<DeletionStats, Status> {
    tokio::task::spawn_blocking(move || delete_rel_paths_sync(&module_path, rel_paths))
        .await
        .map_err(|err| Status::internal(format!("purge task failed: {}", err)))?
}

async fn purge_extraneous_entries(
    module_path: PathBuf,
    expected_files: Vec<PathBuf>,
) -> Result<DeletionStats, Status> {
    tokio::task::spawn_blocking(move || {
        let extraneous = plan_extraneous_entries(&module_path, &expected_files)?;
        if extraneous.is_empty() {
            return Ok(DeletionStats::default());
        }
        delete_rel_paths_sync(&module_path, extraneous)
    })
    .await
    .map_err(|err| Status::internal(format!("purge task failed: {}", err)))?
}

fn plan_extraneous_entries(
    module_path: &Path,
    expected_files: &[PathBuf],
) -> Result<Vec<PathBuf>, Status> {
    let enumerator = FileEnumerator::new(FileFilter::default());
    let entries = enumerator.enumerate_local(module_path).map_err(|err| {
        Status::internal(format!(
            "enumerating target {}: {}",
            module_path.display(),
            err
        ))
    })?;

    let mut expected_file_set: HashSet<PathBuf> = HashSet::new();
    let mut expected_dirs: HashSet<PathBuf> = HashSet::new();
    expected_dirs.insert(PathBuf::from("."));

    for rel in expected_files {
        expected_file_set.insert(rel.clone());
        let mut current = rel.parent();
        while let Some(parent) = current {
            if parent.as_os_str().is_empty() {
                expected_dirs.insert(PathBuf::from("."));
                break;
            }
            expected_dirs.insert(parent.to_path_buf());
            current = parent.parent();
        }
    }

    let mut files_to_delete = Vec::new();
    let mut dirs_to_delete = Vec::new();

    for entry in entries {
        let rel = entry.relative_path;
        if rel.as_os_str().is_empty() || rel == Path::new(".") {
            continue;
        }
        match &entry.kind {
            EntryKind::Directory => {
                if !expected_dirs.contains(&rel) {
                    dirs_to_delete.push(rel);
                }
            }
            _ => {
                if !expected_file_set.contains(&rel) {
                    files_to_delete.push(rel);
                }
            }
        }
    }

    dirs_to_delete.sort_by_key(|p| p.components().count());
    dirs_to_delete.reverse();

    files_to_delete.extend(dirs_to_delete);
    Ok(files_to_delete)
}

fn delete_rel_paths_sync(
    module_path: &Path,
    rel_paths: Vec<PathBuf>,
) -> Result<DeletionStats, Status> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    for rel in rel_paths {
        if rel.as_os_str().is_empty() || rel == Path::new(".") {
            continue;
        }

        let target = module_path.join(&rel);
        let metadata = match std::fs::symlink_metadata(&target) {
            Ok(meta) => meta,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(Status::internal(format!(
                    "stat {}: {}",
                    target.display(),
                    err
                )));
            }
        };

        if metadata.file_type().is_dir() {
            dirs.push(rel);
        } else {
            files.push(rel);
        }
    }

    let mut stats = DeletionStats::default();

    for rel in files {
        let target = module_path.join(&rel);
        #[cfg(windows)]
        {
            blit_core::win_fs::clear_readonly_recursive(&target);
        }
        match std::fs::remove_file(&target) {
            Ok(_) => {
                stats.files += 1;
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) if err.kind() == ErrorKind::IsADirectory => {
                match std::fs::remove_dir_all(&target) {
                    Ok(_) => {
                        stats.dirs += 1;
                    }
                    Err(inner) if inner.kind() == ErrorKind::NotFound => {}
                    Err(inner) => {
                        return Err(Status::internal(format!(
                            "remove_dir_all {}: {}",
                            target.display(),
                            inner
                        )));
                    }
                }
            }
            Err(err) => {
                return Err(Status::internal(format!(
                    "remove_file {}: {}",
                    target.display(),
                    err
                )));
            }
        }
    }

    dirs.sort_by_key(|p| p.components().count());
    dirs.reverse();

    for rel in dirs {
        let target = module_path.join(&rel);
        #[cfg(windows)]
        {
            blit_core::win_fs::clear_readonly_recursive(&target);
        }
        match std::fs::remove_dir_all(&target) {
            Ok(_) => {
                stats.dirs += 1;
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(Status::internal(format!(
                    "remove_dir_all {}: {}",
                    target.display(),
                    err
                )));
            }
        }
    }

    Ok(stats)
}

fn split_completion_prefix(raw: &str) -> Result<(PathBuf, String, String), Status> {
    let trimmed = raw.trim().trim_start_matches("./");
    let (dir_part, leaf) = if trimmed.is_empty() {
        ("", "")
    } else if trimmed.ends_with('/') {
        (trimmed.trim_end_matches('/'), "")
    } else if let Some((dir, name)) = trimmed.rsplit_once('/') {
        (dir, name)
    } else {
        ("", trimmed)
    };

    let rel_path = if dir_part.is_empty() {
        PathBuf::from(".")
    } else {
        resolve_relative_path(dir_part)?
    };

    let display = if dir_part.is_empty() {
        String::new()
    } else {
        dir_part.replace('\\', "/")
    };

    Ok((rel_path, display, leaf.to_string()))
}

fn list_completions(
    base_path: &Path,
    display_prefix: &str,
    prefix: &str,
    include_files: bool,
    include_dirs: bool,
) -> Result<Vec<String>, Status> {
    let read_dir = match std::fs::read_dir(base_path) {
        Ok(iter) => iter,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(Status::internal(format!(
                "read_dir {}: {}",
                base_path.display(),
                err
            )))
        }
    };

    let mut results = Vec::new();
    for entry in read_dir {
        let entry = match entry {
            Ok(item) => item,
            Err(err) => {
                eprintln!(
                    "[warn] failed to read completion entry under {}: {}",
                    base_path.display(),
                    err
                );
                continue;
            }
        };

        let name_os = entry.file_name();
        let name = name_os.to_string_lossy();
        if !name.starts_with(prefix) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(err) => {
                eprintln!(
                    "[warn] failed to stat completion candidate {}: {}",
                    entry.path().display(),
                    err
                );
                continue;
            }
        };

        let is_dir = metadata.is_dir();
        if is_dir && !include_dirs {
            continue;
        }
        if !is_dir && !include_files {
            continue;
        }

        let mut completion = String::new();
        if !display_prefix.is_empty() {
            completion.push_str(display_prefix);
            if !display_prefix.ends_with('/') {
                completion.push('/');
            }
        }
        completion.push_str(&name);
        if is_dir {
            completion.push('/');
        }
        results.push(completion);
    }

    results.sort();
    results.dedup();
    Ok(results)
}

#[derive(Default)]
struct UsageAccum {
    bytes: u64,
    files: u64,
    dirs: u64,
}

fn stream_disk_usage(
    module_root: PathBuf,
    start_rel: PathBuf,
    max_depth: Option<usize>,
    sender: &DiskUsageSender,
) -> Result<(), Status> {
    let start_abs = module_root.join(&start_rel);
    if !start_abs.exists() {
        return Err(Status::not_found(format!(
            "start path not found for disk usage: {}",
            pathbuf_to_display(&start_rel)
        )));
    }

    let mut accum: HashMap<PathBuf, UsageAccum> = HashMap::new();
    accum.entry(PathBuf::from(".")).or_default();

    let add_file = |accum: &mut HashMap<PathBuf, UsageAccum>,
                    rel: &Path,
                    size: u64,
                    max_depth: Option<usize>| {
        let prefixes = prefix_paths(rel);
        for (depth, prefix) in prefixes.into_iter().enumerate() {
            if let Some(max) = max_depth {
                if depth > max {
                    break;
                }
            }
            let entry = accum.entry(prefix).or_default();
            entry.bytes += size;
            entry.files += 1;
        }
    };

    let add_dir =
        |accum: &mut HashMap<PathBuf, UsageAccum>, rel: &Path, max_depth: Option<usize>| {
            let prefixes = prefix_paths(rel);
            for (depth, prefix) in prefixes.into_iter().enumerate() {
                if let Some(max) = max_depth {
                    if depth > max {
                        break;
                    }
                }
                let entry = accum.entry(prefix).or_default();
                entry.dirs += 1;
            }
        };

    let metadata = start_abs
        .metadata()
        .map_err(|err| Status::internal(format!("stat {}: {}", start_abs.display(), err)))?;

    if start_abs.is_file() {
        add_file(&mut accum, &start_rel, metadata.len(), max_depth);
    } else {
        if start_rel != PathBuf::from(".") {
            add_dir(&mut accum, &start_rel, max_depth);
        }
        let enumerator = FileEnumerator::new(FileFilter::default());
        enumerator
            .enumerate_local_streaming(&start_abs, |entry| {
                let rel_from_root = if start_rel == PathBuf::from(".") {
                    entry.relative_path.clone()
                } else {
                    let mut combined = start_rel.clone();
                    if entry.relative_path != PathBuf::from(".") {
                        combined.push(&entry.relative_path);
                    }
                    combined
                };

                match entry.kind {
                    EntryKind::Directory => {
                        add_dir(&mut accum, &rel_from_root, max_depth);
                    }
                    EntryKind::File { size } => {
                        add_file(&mut accum, &rel_from_root, size, max_depth);
                    }
                    EntryKind::Symlink { .. } => {}
                }
                Ok(())
            })
            .map_err(|err| Status::internal(format!("disk usage enumeration failed: {err}")))?;
    }

    let mut entries: Vec<(usize, PathBuf, UsageAccum)> = accum
        .into_iter()
        .map(|(path, usage)| {
            let depth = if path == PathBuf::from(".") {
                0
            } else {
                path.components().count()
            };
            (depth, path, usage)
        })
        .collect();

    entries.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| pathbuf_to_display(&a.1).cmp(&pathbuf_to_display(&b.1)))
    });

    for (depth, path, usage) in entries {
        if let Some(max) = max_depth {
            if depth > max {
                continue;
            }
        }
        let entry = DiskUsageEntry {
            relative_path: pathbuf_to_display(&path),
            byte_total: usage.bytes,
            file_count: usage.files,
            dir_count: usage.dirs,
        };
        sender
            .blocking_send(Ok(entry))
            .map_err(|_| Status::internal("client dropped disk usage stream"))?;
    }

    Ok(())
}

fn prefix_paths(rel: &Path) -> Vec<PathBuf> {
    if rel == Path::new(".") {
        return vec![PathBuf::from(".")];
    }
    let mut prefixes = Vec::new();
    prefixes.push(PathBuf::from(".")); // root
    let mut current = PathBuf::new();
    for component in rel.components() {
        current.push(component.as_os_str());
        prefixes.push(current.clone());
    }
    prefixes
}
fn stream_find_entries(
    module_root: PathBuf,
    start_rel: PathBuf,
    pattern: String,
    case_sensitive: bool,
    include_files: bool,
    include_dirs: bool,
    max_results: Option<usize>,
    sender: &FindSender,
) -> Result<(), Status> {
    let start_abs = module_root.join(&start_rel);
    if !start_abs.exists() {
        return Err(Status::not_found(format!(
            "start path not found for find: {}",
            pathbuf_to_display(&start_rel)
        )));
    }

    let matcher = if pattern.is_empty() {
        None
    } else if case_sensitive {
        Some(pattern)
    } else {
        Some(pattern.to_lowercase())
    };

    let mut sent = 0usize;
    let limit = max_results.filter(|&m| m > 0);

    let mut maybe_emit =
        |rel_path: PathBuf, metadata: std::fs::Metadata, is_dir: bool| -> Result<(), Status> {
            if let Some(limit) = limit {
                if sent >= limit {
                    return Ok(());
                }
            }
            if is_dir && !include_dirs {
                return Ok(());
            }
            if !is_dir && !include_files {
                return Ok(());
            }

            let rel_display = pathbuf_to_display(&rel_path);
            if let Some(ref pat) = matcher {
                let candidate = if case_sensitive {
                    rel_display.clone()
                } else {
                    rel_display.to_lowercase()
                };
                if !candidate.contains(pat) {
                    return Ok(());
                }
            }

            let entry = FindEntry {
                relative_path: rel_display,
                is_dir,
                size: if is_dir { 0 } else { metadata.len() },
                mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            };
            sender
                .blocking_send(Ok(entry))
                .map_err(|_| Status::internal("client dropped find stream"))?;
            sent += 1;
            Ok(())
        };

    let metadata = start_abs
        .metadata()
        .map_err(|err| Status::internal(format!("stat {}: {}", start_abs.display(), err)))?;

    if start_abs.is_file() {
        maybe_emit(start_rel.clone(), metadata, false)?;
        return Ok(());
    }

    if include_dirs && start_rel != PathBuf::from(".") {
        maybe_emit(start_rel.clone(), metadata, true)?;
    }

    let enumerator = FileEnumerator::new(FileFilter::default());
    enumerator
        .enumerate_local_streaming(&start_abs, |entry| {
            let rel_from_root = if start_rel == PathBuf::from(".") {
                entry.relative_path.clone()
            } else {
                let mut combined = start_rel.clone();
                if entry.relative_path != PathBuf::from(".") {
                    combined.push(&entry.relative_path);
                }
                combined
            };
            let is_dir = matches!(entry.kind, EntryKind::Directory);
            maybe_emit(rel_from_root, entry.metadata, is_dir)?;
            Ok(())
        })
        .map_err(|err| Status::internal(format!("find enumeration failed: {err}")))?;

    Ok(())
}

fn filesystem_stats_for_path(path: &Path) -> Result<FilesystemStatsResponse, Status> {
    let canonical = fs::canonicalize(path).map_err(|err| {
        Status::internal(format!(
            "failed to resolve filesystem stats path {}: {}",
            path.display(),
            err
        ))
    })?;

    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh_list();
    disks.refresh();

    let mut best_match = None;
    let mut best_len = 0usize;
    for disk in disks.iter() {
        let mount = disk.mount_point();
        if canonical.starts_with(mount) {
            let depth = mount.components().count();
            if depth >= best_len {
                best_len = depth;
                best_match = Some(disk);
            }
        }
    }

    let disk = best_match.ok_or_else(|| {
        Status::internal(format!(
            "no filesystem information available for {}",
            path.display()
        ))
    })?;

    Ok(FilesystemStatsResponse {
        module: pathbuf_to_display(path),
        total_bytes: disk.total_space(),
        used_bytes: disk.total_space().saturating_sub(disk.available_space()),
        free_bytes: disk.available_space(),
    })
}

fn pathbuf_to_display(path: &Path) -> String {
    if path == Path::new(".") {
        return ".".to_string();
    }
    path.components()
        .map(|comp| comp.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    daemon: RawDaemonSection,
    #[serde(default, rename = "module")]
    modules: Vec<RawModule>,
}

#[derive(Debug, Default, Deserialize)]
struct RawDaemonSection {
    bind: Option<String>,
    port: Option<u16>,
    motd: Option<String>,
    no_mdns: Option<bool>,
    mdns_name: Option<String>,
    root: Option<PathBuf>,
    #[serde(default)]
    root_read_only: bool,
    #[serde(default)]
    root_use_chroot: bool,
}

#[derive(Debug, Deserialize)]
struct RawModule {
    name: String,
    path: PathBuf,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    use_chroot: bool,
}

fn default_config_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\ProgramData\Blit\config.toml")
    } else {
        PathBuf::from("/etc/blit/config.toml")
    }
}

fn load_runtime(args: &DaemonArgs) -> Result<DaemonRuntime> {
    let mut warnings = Vec::new();

    let config_path = if let Some(path) = &args.config {
        Some(path.clone())
    } else {
        let candidate = default_config_path();
        if candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    };

    let raw = if let Some(ref path) = config_path {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        toml::from_str::<RawConfig>(&contents)
            .with_context(|| format!("failed to parse config file {}", path.display()))?
    } else {
        RawConfig::default()
    };

    let bind_host = args
        .bind
        .clone()
        .or_else(|| raw.daemon.bind.clone())
        .unwrap_or_else(|| "0.0.0.0".to_string());
    let port = args.port.or(raw.daemon.port).unwrap_or(9031);

    let motd = raw.daemon.motd.clone();
    let mdns_disabled = if args.no_mdns {
        true
    } else {
        raw.daemon.no_mdns.unwrap_or(false)
    };
    let mdns_name = args.mdns_name.clone().or(raw.daemon.mdns_name.clone());
    let mdns = MdnsConfig {
        disabled: mdns_disabled,
        name: mdns_name,
    };

    let mut modules = HashMap::new();
    for module in raw.modules {
        if module.name.trim().is_empty() {
            return Err(eyre!("module names cannot be empty"));
        }
        if modules.contains_key(&module.name) {
            return Err(eyre!("duplicate module '{}' in config", module.name));
        }
        let canonical = fs::canonicalize(&module.path).with_context(|| {
            format!(
                "failed to resolve path '{}' for module '{}'",
                module.path.display(),
                module.name
            )
        })?;
        modules.insert(
            module.name.clone(),
            ModuleConfig {
                name: module.name,
                path: canonical,
                read_only: module.read_only,
                _comment: module.comment,
                _use_chroot: module.use_chroot,
            },
        );
    }

    let mut root_spec = if let Some(cli_root) = &args.root {
        Some(RootSpec {
            path: cli_root.clone(),
            read_only: false,
            use_chroot: raw.daemon.root_use_chroot,
        })
    } else if let Some(cfg_root) = raw.daemon.root.clone() {
        Some(RootSpec {
            path: cfg_root,
            read_only: raw.daemon.root_read_only,
            use_chroot: raw.daemon.root_use_chroot,
        })
    } else {
        None
    };

    let mut default_root = None;

    if modules.is_empty() {
        let chosen = if let Some(spec) = root_spec.take() {
            spec
        } else {
            let cwd = std::env::current_dir().context("failed to determine working directory")?;
            warnings.push(format!(
                "no modules configured; exporting working directory {} as 'default'",
                cwd.display()
            ));
            RootSpec {
                path: cwd,
                read_only: false,
                use_chroot: false,
            }
        };
        let canonical = fs::canonicalize(&chosen.path).with_context(|| {
            format!(
                "failed to resolve default export path '{}'",
                chosen.path.display()
            )
        })?;
        modules.insert(
            "default".to_string(),
            ModuleConfig {
                name: "default".to_string(),
                path: canonical.clone(),
                read_only: chosen.read_only,
                _comment: None,
                _use_chroot: chosen.use_chroot,
            },
        );
        default_root = Some(RootExport {
            path: canonical,
            read_only: chosen.read_only,
            use_chroot: chosen.use_chroot,
        });
    } else if let Some(spec) = root_spec {
        let canonical = fs::canonicalize(&spec.path).with_context(|| {
            format!(
                "failed to resolve root export path '{}'",
                spec.path.display()
            )
        })?;
        default_root = Some(RootExport {
            path: canonical,
            read_only: spec.read_only,
            use_chroot: spec.use_chroot,
        });
    } else if !modules.contains_key("default") {
        warnings.push(
            "no default root configured; server:// requests will be rejected until --root or config root is provided"
                .to_string(),
        );
    }

    Ok(DaemonRuntime {
        bind_host,
        port,
        modules,
        default_root,
        mdns,
        motd,
        warnings,
    })
}
#[derive(Debug, Default)]
struct TransferStats {
    files_transferred: u64,
    bytes_transferred: u64,
    bytes_zero_copy: u64,
}

#[derive(Parser, Debug)]
#[command(name = "blit-daemon", about = "Remote transfer daemon for blit v2")]
struct DaemonArgs {
    /// Path to the daemon configuration file (TOML). Defaults to /etc/blit/config.toml when present.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Host/IP address to bind (overrides config file)
    #[arg(long)]
    bind: Option<String>,
    /// Port to bind (overrides config file)
    #[arg(long)]
    port: Option<u16>,
    /// Exported root path for server:// when no modules are defined
    #[arg(long)]
    root: Option<PathBuf>,
    /// Disable mDNS advertisement even if enabled in config
    #[arg(long)]
    no_mdns: bool,
    /// Override the advertised mDNS instance name
    #[arg(long)]
    mdns_name: Option<String>,
    /// Force the daemon to use the gRPC data plane instead of TCP
    #[arg(long)]
    force_grpc_data: bool,
}

pub struct BlitService {
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    force_grpc_data: bool,
}

impl BlitService {
    pub(crate) fn from_runtime(
        modules: HashMap<String, ModuleConfig>,
        default_root: Option<RootExport>,
        force_grpc_data: bool,
    ) -> Self {
        Self {
            modules: Arc::new(Mutex::new(modules)),
            default_root,
            force_grpc_data,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_modules(
        modules: HashMap<String, ModuleConfig>,
        force_grpc_data: bool,
    ) -> Self {
        Self::from_runtime(modules, None, force_grpc_data)
    }
}

#[tonic::async_trait]
impl Blit for BlitService {
    type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
    type PullStream = tokio_stream::wrappers::ReceiverStream<Result<PullChunk, Status>>;
    type FindStream = ReceiverStream<Result<FindEntry, Status>>;
    type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;

    async fn push(
        &self,
        request: Request<Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        let modules = Arc::clone(&self.modules);
        let (tx, rx) = mpsc::channel(32);
        let stream = request.into_inner();
        let force_grpc_data = self.force_grpc_data;
        let default_root = self.default_root.clone();

        tokio::spawn(async move {
            if let Err(status) =
                handle_push_stream(modules, default_root, stream, tx.clone(), force_grpc_data).await
            {
                let _ = tx.send(Err(status)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn pull(
        &self,
        request: Request<PullRequest>,
    ) -> Result<Response<Self::PullStream>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;

        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            if let Err(status) = stream_pull(module, req.path, tx.clone()).await {
                let _ = tx.send(Err(status)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;

        let requested = if req.path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(&req.path)?
        };

        let target = module.path.join(&requested);
        if !target.exists() {
            return Err(Status::not_found(format!(
                "path not found in module '{}': {}",
                module.name, req.path
            )));
        }

        let response_entries =
            tokio::task::spawn_blocking(move || -> Result<Vec<FileInfo>, Status> {
                let metadata = std::fs::metadata(&target).map_err(|err| {
                    Status::internal(format!("stat {}: {}", target.display(), err))
                })?;

                if metadata.is_file() {
                    let name = requested
                        .iter()
                        .map(|c| c.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    let info = FileInfo {
                        name: if name.is_empty() {
                            target
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| ".".to_string())
                        } else {
                            name
                        },
                        is_dir: false,
                        size: metadata.len(),
                        mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
                    };
                    Ok(vec![info])
                } else if metadata.is_dir() {
                    let mut infos = Vec::new();
                    let entries = std::fs::read_dir(&target).map_err(|err| {
                        Status::internal(format!("read_dir {}: {}", target.display(), err))
                    })?;
                    for entry in entries {
                        let entry = entry.map_err(|err| {
                            Status::internal(format!(
                                "read_dir entry {}: {}",
                                target.display(),
                                err
                            ))
                        })?;
                        let path = entry.path();
                        let meta = entry.metadata().map_err(|err| {
                            Status::internal(format!("metadata {}: {}", path.display(), err))
                        })?;
                        let name = entry.file_name().to_string_lossy().into_owned();
                        infos.push(FileInfo {
                            name,
                            is_dir: meta.is_dir(),
                            size: meta.len(),
                            mtime_seconds: metadata_mtime_seconds(&meta).unwrap_or(0),
                        });
                    }
                    infos.sort_by(|a, b| a.name.cmp(&b.name));
                    Ok(infos)
                } else {
                    Err(Status::invalid_argument(format!(
                        "unsupported path type for list: {}",
                        target.display()
                    )))
                }
            })
            .await
            .map_err(|err| Status::internal(format!("list task failed: {}", err)))??;

        Ok(Response::new(ListResponse {
            entries: response_entries,
        }))
    }

    async fn purge(
        &self,
        request: Request<PurgeRequest>,
    ) -> Result<Response<PurgeResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        if module.read_only {
            return Err(Status::permission_denied(format!(
                "module '{}' is read-only",
                module.name
            )));
        }

        let sanitized = sanitize_request_paths(req.paths_to_delete)?;
        if sanitized.is_empty() {
            return Ok(Response::new(PurgeResponse { files_deleted: 0 }));
        }

        let stats = delete_rel_paths(module.path.clone(), sanitized).await?;

        Ok(Response::new(PurgeResponse {
            files_deleted: stats.total(),
        }))
    }

    async fn complete_path(
        &self,
        request: Request<CompletionRequest>,
    ) -> Result<Response<CompletionResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        if !req.include_files && !req.include_directories {
            return Err(Status::invalid_argument(
                "at least one of include_files or include_directories must be true",
            ));
        }

        let (dir_rel, display_prefix, leaf_prefix) =
            split_completion_prefix(req.path_prefix.as_str())?;
        let search_root = module.path.join(&dir_rel);
        let include_files = req.include_files;
        let include_dirs = req.include_directories;

        let display_prefix_owned = display_prefix.clone();
        let leaf_prefix_owned = leaf_prefix.clone();
        let entries = tokio::task::spawn_blocking(move || {
            list_completions(
                &search_root,
                &display_prefix_owned,
                &leaf_prefix_owned,
                include_files,
                include_dirs,
            )
        })
        .await
        .map_err(|err| Status::internal(format!("completion task failed: {}", err)))??;

        Ok(Response::new(CompletionResponse {
            completions: entries,
        }))
    }

    async fn list_modules(
        &self,
        _request: Request<ListModulesRequest>,
    ) -> Result<Response<ListModulesResponse>, Status> {
        let guard = self.modules.lock().await;
        let mut modules: Vec<ModuleInfo> = guard
            .values()
            .map(|module| ModuleInfo {
                name: module.name.clone(),
                path: module.path.to_string_lossy().into_owned(),
                read_only: module.read_only,
            })
            .collect();
        modules.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Response::new(ListModulesResponse { modules }))
    }

    async fn find(
        &self,
        request: Request<FindRequest>,
    ) -> Result<Response<Self::FindStream>, Status> {
        let req = request.into_inner();
        if !req.include_files && !req.include_directories {
            return Err(Status::invalid_argument(
                "at least one of include_files or include_directories must be true",
            ));
        }
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let start_rel = if req.start_path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(req.start_path.trim())?
        };
        let pattern = req.pattern.clone();
        let case_sensitive = req.case_sensitive;
        let include_files = req.include_files;
        let include_dirs = req.include_directories;
        let max_results = if req.max_results == 0 {
            None
        } else {
            Some(req.max_results as usize)
        };

        let (tx, rx) = mpsc::channel(64);
        let module_root = module.path.clone();
        tokio::spawn(async move {
            let err_sender = tx.clone();
            let result = tokio::task::spawn_blocking(move || {
                stream_find_entries(
                    module_root,
                    start_rel,
                    pattern,
                    case_sensitive,
                    include_files,
                    include_dirs,
                    max_results,
                    &tx,
                )
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(status)) => {
                    let _ = err_sender.send(Err(status)).await;
                }
                Err(join_err) => {
                    let _ = err_sender
                        .send(Err(Status::internal(format!(
                            "find worker failed: {}",
                            join_err
                        ))))
                        .await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn disk_usage(
        &self,
        request: Request<DiskUsageRequest>,
    ) -> Result<Response<Self::DiskUsageStream>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let start_rel = if req.start_path.trim().is_empty() {
            PathBuf::from(".")
        } else {
            resolve_relative_path(req.start_path.trim())?
        };
        let max_depth = if req.max_depth == 0 {
            None
        } else {
            Some(req.max_depth as usize)
        };

        let (tx, rx) = mpsc::channel(32);
        let module_root = module.path.clone();
        tokio::spawn(async move {
            let err_sender = tx.clone();
            let result = tokio::task::spawn_blocking(move || {
                stream_disk_usage(module_root, start_rel, max_depth, &tx)
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(status)) => {
                    let _ = err_sender.send(Err(status)).await;
                }
                Err(join_err) => {
                    let _ = err_sender
                        .send(Err(Status::internal(format!(
                            "disk usage worker failed: {}",
                            join_err
                        ))))
                        .await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn filesystem_stats(
        &self,
        request: Request<FilesystemStatsRequest>,
    ) -> Result<Response<FilesystemStatsResponse>, Status> {
        let req = request.into_inner();
        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
        let stats = filesystem_stats_for_path(&module.path)?;
        Ok(Response::new(stats))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = DaemonArgs::parse();
    let runtime = load_runtime(&args)?;
    let DaemonRuntime {
        bind_host,
        port,
        modules,
        default_root,
        mdns,
        motd,
        warnings,
    } = runtime;

    for warning in &warnings {
        eprintln!("[warn] {warning}");
    }

    let addr: SocketAddr = format!("{}:{}", bind_host, port).parse()?;
    if let Some(motd) = motd {
        println!("motd: {motd}");
    }
    if let Some(root) = &default_root {
        eprintln!(
            "[info] default root export: {}{}{}",
            root.path.display(),
            if root.read_only { " (read-only)" } else { "" },
            if root.use_chroot { " [chroot]" } else { "" }
        );
    }

    let module_names: Vec<String> = modules.keys().cloned().collect();
    let mdns_guard: Option<MdnsAdvertiser> = if mdns.disabled {
        if let Some(name) = &mdns.name {
            eprintln!(
                "[info] mDNS advertising disabled; instance name '{}' ignored",
                name
            );
        }
        None
    } else {
        match mdns::advertise(AdvertiseOptions {
            port,
            instance_name: mdns.name.as_deref(),
            module_names: &module_names,
        }) {
            Ok(handle) => {
                eprintln!(
                    "[info] mDNS advertising '{}' on port {}",
                    handle.instance_name(),
                    port
                );
                Some(handle)
            }
            Err(err) => {
                eprintln!("[warn] failed to advertise mDNS service: {err:?}");
                None
            }
        }
    };

    let service = BlitService::from_runtime(modules, default_root, args.force_grpc_data);

    println!("blitd v2 listening on {}", addr);

    Server::builder()
        .add_service(BlitServer::new(service))
        .serve(addr)
        .await?;

    drop(mdns_guard);

    Ok(())
}

async fn handle_push_stream(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    mut stream: Streaming<ClientPushRequest>,
    tx: PushSender,
    force_grpc_data: bool,
) -> Result<(), Status> {
    let mut module: Option<ModuleConfig> = None;
    let mut files_to_upload: Vec<FileHeader> = Vec::new();
    let mut manifest_complete = false;
    let mut mirror_mode = false;
    let mut expected_rel_files: Vec<PathBuf> = Vec::new();
    let mut force_grpc_client = false;
    let mut need_list_sender = FileListBatcher::new(tx.clone());

    while let Some(request) = stream.message().await? {
        match request.payload {
            Some(client_push_request::Payload::Header(header)) => {
                if module.is_some() {
                    return Err(Status::invalid_argument("duplicate push header received"));
                }
                let mut config =
                    resolve_module(&modules, default_root.as_ref(), &header.module).await?;
                if config.read_only {
                    return Err(Status::permission_denied(format!(
                        "module '{}' is read-only",
                        config.name
                    )));
                }
                mirror_mode = header.mirror_mode;
                force_grpc_client = header.force_grpc;
                let dest_path = header.destination_path.trim();
                if !dest_path.is_empty() {
                    let rel = resolve_relative_path(dest_path)?;
                    config.path = config.path.join(rel);
                }
                module = Some(config);
                send_control_message(&tx, server_push_response::Payload::Ack(Ack {})).await?;
            }
            Some(client_push_request::Payload::FileManifest(mut file)) => {
                let module_ref = module.as_ref().ok_or_else(|| {
                    Status::failed_precondition("push manifest received before header")
                })?;
                let rel = resolve_relative_path(&file.relative_path)?;
                expected_rel_files.push(rel.clone());
                let sanitized = rel.to_string_lossy().to_string();

                if file_requires_upload(module_ref, &rel, &file)? {
                    file.relative_path = sanitized.clone();
                    need_list_sender.push(sanitized).await?;
                    files_to_upload.push(file);
                }
            }
            Some(client_push_request::Payload::ManifestComplete(_)) => {
                manifest_complete = true;
                break;
            }
            Some(client_push_request::Payload::FileData(_)) => {
                return Err(Status::failed_precondition(
                    "data payload received before negotiation",
                ));
            }
            Some(client_push_request::Payload::UploadComplete(_)) => {
                // Ignore; summary is driven once data plane completes.
            }
            None => {}
        }
    }

    let module = module.ok_or_else(|| Status::invalid_argument("push stream missing header"))?;
    if !manifest_complete {
        return Err(Status::invalid_argument(
            "push stream ended before manifest completion",
        ));
    }

    need_list_sender.finish().await?;
    let files_requested = files_to_upload;

    let force_grpc_effective = force_grpc_data || force_grpc_client;
    let (transfer_stats, tcp_fallback_used) = if files_requested.is_empty() || force_grpc_effective
    {
        (
            execute_grpc_fallback(&tx, &mut stream, &module, files_requested.clone()).await?,
            true,
        )
    } else {
        match bind_data_plane_listener().await {
            Ok(listener) => {
                let port = listener
                    .local_addr()
                    .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
                    .port();

                let token = generate_token();
                let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);

                let (summary_tx, summary_rx) = oneshot::channel();
                let module_for_transfer = module.clone();
                let files_for_transfer = files_requested.clone();

                tokio::spawn(async move {
                    let result = accept_data_connection(
                        listener,
                        token,
                        module_for_transfer,
                        files_for_transfer,
                    )
                    .await;
                    let _ = summary_tx.send(result);
                });

                send_control_message(
                    &tx,
                    server_push_response::Payload::Negotiation(DataTransferNegotiation {
                        tcp_port: port as u32,
                        one_time_token: token_string,
                        tcp_fallback: false,
                    }),
                )
                .await?;

                let stats = summary_rx
                    .await
                    .map_err(|_| Status::internal("data plane task cancelled"))??;

                (stats, false)
            }
            Err(_) => (
                execute_grpc_fallback(&tx, &mut stream, &module, files_requested.clone()).await?,
                true,
            ),
        }
    };

    let mut entries_deleted = 0u64;
    if mirror_mode {
        let purge_stats = purge_extraneous_entries(module.path.clone(), expected_rel_files).await?;
        entries_deleted = purge_stats.total();
    }

    send_control_message(
        &tx,
        server_push_response::Payload::Summary(PushSummary {
            files_transferred: transfer_stats.files_transferred,
            bytes_transferred: transfer_stats.bytes_transferred,
            bytes_zero_copy: transfer_stats.bytes_zero_copy,
            tcp_fallback_used,
            entries_deleted,
        }),
    )
    .await?;

    Ok(())
}

async fn resolve_module(
    modules: &Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<&RootExport>,
    name: &str,
) -> Result<ModuleConfig, Status> {
    if name.trim().is_empty() {
        if let Some(root) = default_root {
            return Ok(ModuleConfig {
                name: "default".to_string(),
                path: root.path.clone(),
                read_only: root.read_only,
                _comment: None,
                _use_chroot: root.use_chroot,
            });
        } else {
            return Err(Status::not_found(
                "default root is not configured on the remote daemon",
            ));
        }
    }

    let guard = modules.lock().await;
    guard
        .get(name)
        .cloned()
        .ok_or_else(|| Status::not_found(format!("module '{}' not found", name)))
}

async fn send_control_message(
    tx: &PushSender,
    payload: server_push_response::Payload,
) -> Result<(), Status> {
    tx.send(Ok(ServerPushResponse {
        payload: Some(payload),
    }))
    .await
    .map_err(|_| Status::internal("failed to send push response"))
}

fn file_requires_upload(
    module: &ModuleConfig,
    rel: &Path,
    header: &FileHeader,
) -> Result<bool, Status> {
    let full_path = module.path.join(rel);
    let requires_upload = match fs::metadata(&full_path) {
        Ok(meta) => {
            if !meta.is_file() {
                true
            } else {
                let same_size = meta.len() == header.size;
                let same_mtime = metadata_mtime_seconds(&meta)
                    .map(|seconds| seconds == header.mtime_seconds)
                    .unwrap_or(false);
                !(same_size && same_mtime)
            }
        }
        Err(_) => true,
    };
    Ok(requires_upload)
}

#[allow(clippy::result_large_err)]
fn resolve_relative_path(rel: &str) -> Result<PathBuf, Status> {
    #[cfg(windows)]
    {
        if rel.starts_with('/') || rel.starts_with('\\') {
            return Err(Status::invalid_argument(format!(
                "absolute-style path not allowed in manifest: {}",
                rel
            )));
        }
    }

    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(Status::invalid_argument(format!(
            "absolute paths not allowed in manifest: {}",
            rel
        )));
    }

    use std::path::Component;
    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(Status::invalid_argument(format!(
            "parent directory segments not allowed: {}",
            rel
        )));
    }

    Ok(path.to_path_buf())
}

fn metadata_mtime_seconds(meta: &fs::Metadata) -> Option<i64> {
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

fn permissions_mode(meta: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        0
    }
}

async fn bind_data_plane_listener() -> Result<TcpListener, Status> {
    TcpListener::bind("0.0.0.0:0")
        .await
        .map_err(|err| Status::internal(format!("failed to bind data plane socket: {}", err)))
}

fn generate_token() -> Vec<u8> {
    let mut buf = vec![0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut buf);
    buf
}

async fn accept_data_connection(
    listener: TcpListener,
    expected_token: Vec<u8>,
    module: ModuleConfig,
    files: Vec<FileHeader>,
) -> Result<TransferStats, Status> {
    let (mut socket, _) = listener
        .accept()
        .await
        .map_err(|err| Status::internal(format!("data plane accept failed: {}", err)))?;

    let mut token_buf = vec![0u8; expected_token.len()];
    socket
        .read_exact(&mut token_buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read data plane token: {}", err)))?;
    if token_buf != expected_token {
        return Err(Status::permission_denied("invalid data plane token"));
    }

    let mut pending: HashMap<String, FileHeader> = files
        .into_iter()
        .map(|header| (header.relative_path.clone(), header))
        .collect();

    let mut stats = TransferStats::default();

    loop {
        let path_len = read_u32(&mut socket).await?;
        if path_len == 0 {
            break;
        }

        let mut path_bytes = vec![0u8; path_len as usize];
        socket
            .read_exact(&mut path_bytes)
            .await
            .map_err(|err| Status::internal(format!("failed to read path bytes: {}", err)))?;
        let rel_string = String::from_utf8(path_bytes)
            .map_err(|_| Status::invalid_argument("data plane path not valid UTF-8"))?;

        let header = pending
            .remove(&rel_string)
            .ok_or_else(|| Status::invalid_argument(format!("unexpected file '{}'", rel_string)))?;

        let file_size = read_u64(&mut socket).await?;
        if file_size != header.size {
            return Err(Status::invalid_argument(format!(
                "size mismatch for {} (declared {}, expected {})",
                rel_string, file_size, header.size
            )));
        }
        let rel_path = resolve_relative_path(&rel_string)?;
        let dest_path = module.path.join(&rel_path);

        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|err| {
                Status::internal(format!("create dir {}: {}", parent.display(), err))
            })?;
        }

        let mut file = tokio::fs::File::create(&dest_path).await.map_err(|err| {
            Status::internal(format!("create file {}: {}", dest_path.display(), err))
        })?;

        let mut limited = (&mut socket).take(file_size);
        let bytes_copied = tokio::io::copy(&mut limited, &mut file)
            .await
            .map_err(|err| Status::internal(format!("writing {}: {}", dest_path.display(), err)))?;
        if bytes_copied != file_size {
            return Err(Status::internal(format!(
                "short transfer for {} (expected {} bytes, received {})",
                rel_string, file_size, bytes_copied
            )));
        }

        stats.files_transferred += 1;
        stats.bytes_transferred += bytes_copied;
    }

    if !pending.is_empty() {
        let missing: Vec<String> = pending.into_keys().collect();
        return Err(Status::internal(format!(
            "transfer incomplete; missing files: {:?}",
            missing
        )));
    }

    Ok(stats)
}

async fn receive_fallback_data(
    stream: &mut Streaming<ClientPushRequest>,
    module: &ModuleConfig,
    files: Vec<FileHeader>,
) -> Result<TransferStats, Status> {
    let mut pending: HashMap<String, FileHeader> = files
        .into_iter()
        .map(|header| (header.relative_path.clone(), header))
        .collect();

    struct ActiveFile {
        header: FileHeader,
        file: tokio::fs::File,
        remaining: u64,
        dest_path: PathBuf,
    }

    let mut current: Option<ActiveFile> = None;
    let mut stats = TransferStats::default();

    while let Some(req) = stream.message().await? {
        match req.payload {
            Some(client_push_request::Payload::FileManifest(header)) => {
                if current.is_some() {
                    return Err(Status::failed_precondition(
                        "received new file manifest before completing prior file",
                    ));
                }

                let expected = pending.remove(&header.relative_path).ok_or_else(|| {
                    Status::invalid_argument(format!(
                        "unexpected fallback file manifest '{}'",
                        header.relative_path
                    ))
                })?;

                let rel_path = resolve_relative_path(&expected.relative_path)?;
                let dest_path = module.path.join(&rel_path);
                if let Some(parent) = dest_path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|err| {
                        Status::internal(format!("create dir {}: {}", parent.display(), err))
                    })?;
                }

                let file = tokio::fs::File::create(&dest_path).await.map_err(|err| {
                    Status::internal(format!("create file {}: {}", dest_path.display(), err))
                })?;

                if expected.size == 0 {
                    stats.files_transferred += 1;
                    continue;
                }

                let size = expected.size;
                current = Some(ActiveFile {
                    header: expected,
                    file,
                    remaining: size,
                    dest_path,
                });
            }
            Some(client_push_request::Payload::FileData(data)) => {
                let active = current.as_mut().ok_or_else(|| {
                    Status::invalid_argument("file data received before file manifest")
                })?;

                let chunk_len = data.content.len() as u64;
                if chunk_len > active.remaining {
                    return Err(Status::invalid_argument(format!(
                        "received {} bytes for '{}' but only {} bytes remain",
                        chunk_len, active.header.relative_path, active.remaining
                    )));
                }

                active.file.write_all(&data.content).await.map_err(|err| {
                    Status::internal(format!("write {}: {}", active.dest_path.display(), err))
                })?;

                active.remaining -= chunk_len;
                stats.bytes_transferred += chunk_len;

                if active.remaining == 0 {
                    stats.files_transferred += 1;
                    current = None;
                }
            }
            Some(client_push_request::Payload::UploadComplete(_)) => break,
            Some(_) => {
                return Err(Status::invalid_argument(
                    "unexpected message during fallback transfer",
                ));
            }
            None => break,
        }
    }

    if let Some(active) = current {
        return Err(Status::invalid_argument(format!(
            "fallback transfer ended mid-file ({} bytes remaining for '{}')",
            active.remaining, active.header.relative_path
        )));
    }

    if !pending.is_empty() {
        let missing: Vec<String> = pending.into_keys().collect();
        return Err(Status::internal(format!(
            "fallback transfer incomplete; missing files: {:?}",
            missing
        )));
    }

    Ok(stats)
}

async fn execute_grpc_fallback(
    tx: &PushSender,
    stream: &mut Streaming<ClientPushRequest>,
    module: &ModuleConfig,
    files_requested: Vec<FileHeader>,
) -> Result<TransferStats, Status> {
    send_control_message(
        tx,
        server_push_response::Payload::Negotiation(DataTransferNegotiation {
            tcp_port: 0,
            one_time_token: String::new(),
            tcp_fallback: true,
        }),
    )
    .await?;

    let stats = receive_fallback_data(stream, module, files_requested).await?;

    Ok(stats)
}

async fn stream_pull(
    module: ModuleConfig,
    requested_path: String,
    tx: PullSender,
) -> Result<(), Status> {
    let requested = if requested_path.trim().is_empty() {
        PathBuf::from(".")
    } else {
        resolve_relative_path(&requested_path)?
    };

    let root = module.path.join(&requested);

    if !root.exists() {
        return Err(Status::not_found(format!(
            "path not found in module '{}': {}",
            module.name, requested_path
        )));
    }

    if root.is_file() {
        let relative_name = if requested == PathBuf::from(".") {
            root.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            requested.clone()
        };
        stream_single_file(&tx, &relative_name, &root).await?;
    } else if root.is_dir() {
        let root_clone = root.clone();
        let entries = tokio::task::spawn_blocking(move || {
            let enumerator = FileEnumerator::new(FileFilter::default());
            enumerator.enumerate_local(&root_clone)
        })
        .await
        .map_err(|e| Status::internal(format!("enumeration task failed: {}", e)))?
        .map_err(|e| Status::internal(format!("enumeration error: {}", e)))?;

        for entry in entries {
            if matches!(entry.kind, EntryKind::File { .. }) {
                stream_single_file(&tx, &entry.relative_path, &entry.absolute_path).await?;
            }
        }
    } else {
        return Err(Status::invalid_argument(format!(
            "unsupported path type for pull: {}",
            requested_path
        )));
    }

    Ok(())
}

async fn stream_single_file(
    tx: &PullSender,
    relative: &Path,
    abs_path: &Path,
) -> Result<(), Status> {
    let metadata = tokio::fs::metadata(abs_path)
        .await
        .map_err(|err| Status::internal(format!("stat {}: {}", abs_path.display(), err)))?;

    let normalized = normalize_relative_path(relative);

    tx.send(Ok(PullChunk {
        payload: Some(PullPayload::FileHeader(FileHeader {
            relative_path: normalized,
            size: metadata.len(),
            mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            permissions: permissions_mode(&metadata),
        })),
    }))
    .await
    .map_err(|_| Status::internal("failed to send pull header"))?;

    let mut file = tokio::fs::File::open(abs_path)
        .await
        .map_err(|err| Status::internal(format!("open {}: {}", abs_path.display(), err)))?;
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|err| Status::internal(format!("read {}: {}", abs_path.display(), err)))?;
        if read == 0 {
            break;
        }

        tx.send(Ok(PullChunk {
            payload: Some(PullPayload::FileData(FileData {
                content: buffer[..read].to_vec(),
            })),
        }))
        .await
        .map_err(|_| Status::internal("failed to send pull chunk"))?;
    }

    Ok(())
}

fn normalize_relative_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        raw.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        raw.into_owned()
    }
}
async fn read_u32(stream: &mut TcpStream) -> Result<u32, Status> {
    let mut buf = [0u8; 4];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read u32: {}", err)))?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_u64(stream: &mut TcpStream) -> Result<u64, Status> {
    let mut buf = [0u8; 8];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|err| Status::internal(format!("failed to read u64: {}", err)))?;
    Ok(u64::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::remote::{RemoteEndpoint, RemotePullClient};
    use eyre::Result;
    use tempfile::tempdir;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{Code, Request};

    #[test]
    fn resolve_relative_path_rejects_parent_segments() {
        assert!(resolve_relative_path("../evil").is_err());
        assert!(resolve_relative_path("sub/../../evil").is_err());
        #[cfg(unix)]
        {
            assert!(resolve_relative_path("/abs/path").is_err());
        }
        #[cfg(windows)]
        {
            assert!(resolve_relative_path("/abs/path").is_err());
            assert!(resolve_relative_path("\\abs\\path").is_err());
            assert!(resolve_relative_path("C:\\abs\\path").is_err());
        }
    }

    async fn spawn_test_daemon(
        root: PathBuf,
        force_grpc_data: bool,
    ) -> (
        SocketAddr,
        oneshot::Sender<()>,
        JoinHandle<Result<(), tonic::transport::Error>>,
    ) {
        let mut map = HashMap::new();
        map.insert(
            "default".to_string(),
            ModuleConfig {
                name: "default".to_string(),
                path: root,
                read_only: false,
                _comment: None,
                _use_chroot: false,
            },
        );
        let service = BlitService::with_modules(map, force_grpc_data);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(BlitServer::new(service))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        (addr, shutdown_tx, server)
    }

    fn module_endpoint(addr: SocketAddr, rel_path: &str) -> Result<RemoteEndpoint> {
        let authority = format!("{}:{}", addr.ip(), addr.port());
        if rel_path.is_empty() {
            RemoteEndpoint::parse(&format!("{authority}:/default/"))
        } else {
            RemoteEndpoint::parse(&format!(
                "{authority}:/default/{}",
                rel_path.trim_start_matches('/')
            ))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn purge_removes_files_and_directories() -> Result<()> {
        let root = tempdir()?;
        let file_path = root.path().join("orphan.txt");
        fs::write(&file_path, b"orphan")?;
        let dir_path = root.path().join("stale_dir");
        fs::create_dir_all(dir_path.join("nested"))?;
        fs::write(dir_path.join("nested").join("ghost.txt"), b"ghost")?;

        let mut modules = HashMap::new();
        modules.insert(
            "default".to_string(),
            ModuleConfig {
                name: "default".to_string(),
                path: root.path().to_path_buf(),
                read_only: false,
                _comment: None,
                _use_chroot: false,
            },
        );
        let service = BlitService::from_runtime(modules, None, false);

        let response = service
            .purge(Request::new(PurgeRequest {
                module: "default".to_string(),
                paths_to_delete: vec!["orphan.txt".into(), "stale_dir".into()],
            }))
            .await?
            .into_inner();

        assert_eq!(response.files_deleted, 2);
        assert!(!file_path.exists());
        assert!(!dir_path.exists());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn purge_respects_read_only_modules() -> Result<()> {
        let root = tempdir()?;
        let file_path = root.path().join("protected.txt");
        fs::write(&file_path, b"protected")?;

        let mut modules = HashMap::new();
        modules.insert(
            "readonly".to_string(),
            ModuleConfig {
                name: "readonly".to_string(),
                path: root.path().to_path_buf(),
                read_only: true,
                _comment: None,
                _use_chroot: false,
            },
        );
        let service = BlitService::from_runtime(modules, None, false);

        let err = service
            .purge(Request::new(PurgeRequest {
                module: "readonly".to_string(),
                paths_to_delete: vec!["protected.txt".into()],
            }))
            .await
            .expect_err("read-only module should reject purge");

        assert_eq!(err.code(), Code::PermissionDenied);
        assert!(file_path.exists(), "read-only file should not be removed");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn purge_extraneous_entries_removes_unexpected_paths() -> Result<()> {
        let root = tempdir()?;
        let keep = root.path().join("keep.txt");
        fs::write(&keep, b"keep")?;
        let stale = root.path().join("stale.txt");
        fs::write(&stale, b"stale")?;
        let orphan_dir = root.path().join("orphan_dir");
        fs::create_dir_all(orphan_dir.join("nested"))?;
        fs::write(orphan_dir.join("nested").join("ghost.txt"), b"ghost")?;

        let stats =
            purge_extraneous_entries(root.path().to_path_buf(), vec![PathBuf::from("keep.txt")])
                .await?;

        assert_eq!(stats.files, 2);
        assert_eq!(stats.dirs, 2);
        assert!(keep.exists(), "expected file should remain");
        assert!(!stale.exists(), "stale file should be purged");
        assert!(
            !orphan_dir.exists(),
            "orphan directory hierarchy should be removed"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_transfers_directory_tree() -> Result<()> {
        let src = tempdir()?;
        let nested = src.path().join("nested");
        fs::create_dir_all(&nested)?;
        fs::write(src.path().join("alpha.txt"), b"alpha")?;
        fs::write(nested.join("beta.txt"), b"beta")?;

        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), false).await;

        let endpoint = module_endpoint(addr, "")?;
        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull(dest.path(), false).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();
        let report = pull_result?;

        assert_eq!(report.files_transferred, 2);
        assert_eq!(
            std::fs::read_to_string(dest.path().join("alpha.txt"))?,
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("nested").join("beta.txt"))?,
            "beta"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_transfers_directory_tree_with_forced_grpc() -> Result<()> {
        let src = tempdir()?;
        let nested = src.path().join("nested");
        fs::create_dir_all(&nested)?;
        fs::write(src.path().join("alpha.txt"), b"alpha")?;
        fs::write(nested.join("beta.txt"), b"beta")?;

        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), true).await;

        let endpoint = module_endpoint(addr, "")?;
        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull(dest.path(), true).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();
        let report = pull_result?;

        assert_eq!(report.files_transferred, 2);
        assert_eq!(
            std::fs::read_to_string(dest.path().join("alpha.txt"))?,
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("nested").join("beta.txt"))?,
            "beta"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_transfers_single_file() -> Result<()> {
        let src = tempdir()?;
        let nested = src.path().join("nested");
        fs::create_dir_all(&nested)?;
        fs::write(nested.join("beta.txt"), b"beta")?;

        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), false).await;

        let endpoint = module_endpoint(addr, "nested/beta.txt")?;
        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull(dest.path(), false).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();
        let report = pull_result?;

        assert_eq!(report.files_transferred, 1);
        assert_eq!(
            std::fs::read_to_string(dest.path().join("nested").join("beta.txt"))?,
            "beta"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_rejects_parent_segments_request() -> Result<()> {
        let src = tempdir()?;
        fs::write(src.path().join("file.txt"), b"content")?;
        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), false).await;

        let endpoint = module_endpoint(addr, "../secret")?;
        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull(dest.path(), false).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();

        assert!(pull_result.is_err());
        let err = pull_result.unwrap_err().to_string();
        assert!(
            err.contains("parent directory"),
            "unexpected error message: {err}"
        );
        assert!(
            dest.path().read_dir()?.next().is_none(),
            "destination should remain empty"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_pull_reports_missing_paths() -> Result<()> {
        let src = tempdir()?;
        fs::write(src.path().join("file.txt"), b"content")?;
        let dest = tempdir()?;

        let (addr, shutdown, server) = spawn_test_daemon(src.path().to_path_buf(), false).await;

        let endpoint = module_endpoint(addr, "missing.txt")?;
        let mut client = RemotePullClient::connect(endpoint).await?;
        let pull_result = client.pull(dest.path(), false).await;
        drop(client);
        let _ = shutdown.send(());
        server.await.unwrap().unwrap();

        assert!(pull_result.is_err());
        let err = pull_result.unwrap_err().to_string();
        assert!(
            err.contains("path not found"),
            "unexpected error message: {err}"
        );
        assert!(
            dest.path().read_dir()?.next().is_none(),
            "destination should remain empty"
        );

        Ok(())
    }

    #[test]
    fn file_requires_upload_detects_missing_and_outdated_files() {
        let dir = tempdir().unwrap();
        let module = ModuleConfig {
            name: "default".to_string(),
            path: dir.path().to_path_buf(),
            read_only: false,
            _comment: None,
            _use_chroot: false,
        };

        let match_path = dir.path().join("match.txt");
        fs::write(&match_path, b"hello").unwrap();
        let match_meta = fs::metadata(&match_path).unwrap();
        let match_header = FileHeader {
            relative_path: "match.txt".into(),
            size: match_meta.len(),
            mtime_seconds: metadata_mtime_seconds(&match_meta).unwrap(),
            permissions: 0,
        };

        let missing_header = FileHeader {
            relative_path: "missing.txt".into(),
            size: 42,
            mtime_seconds: 0,
            permissions: 0,
        };

        let stale_path = dir.path().join("stale.txt");
        fs::write(&stale_path, b"old").unwrap();
        let stale_meta = fs::metadata(&stale_path).unwrap();
        let stale_header = FileHeader {
            relative_path: "stale.txt".into(),
            size: stale_meta.len() + 10,
            mtime_seconds: metadata_mtime_seconds(&stale_meta).unwrap(),
            permissions: 0,
        };

        let needs_match =
            file_requires_upload(&module, Path::new("match.txt"), &match_header).unwrap();
        assert!(
            !needs_match,
            "identical file should not be requested for upload"
        );

        let needs_missing =
            file_requires_upload(&module, Path::new("missing.txt"), &missing_header).unwrap();
        assert!(needs_missing, "missing file should be requested");

        let needs_stale =
            file_requires_upload(&module, Path::new("stale.txt"), &stale_header).unwrap();
        assert!(needs_stale, "stale file should be requested");
    }
}
