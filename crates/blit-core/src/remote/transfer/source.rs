use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use eyre::Result;
use tokio::fs;
use tokio::sync::mpsc;

use crate::fs_enum::FileFilter;
use crate::generated::FileHeader;
use crate::remote::transfer::abort_on_drop::AbortOnDrop;
use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};

/// All tasks that produce one streamed manifest. Decorators append their
/// forwarding or hashing tasks instead of detaching them, so a failed session
/// can abort and reap the complete validation chain before returning.
enum SourceScanTask {
    Count {
        task: AbortOnDrop<Result<u64>>,
        reported: bool,
    },
    Auxiliary(AbortOnDrop<()>),
}

enum SourceScanJoin {
    Count(std::result::Result<Result<u64>, tokio::task::JoinError>),
    Auxiliary(std::result::Result<(), tokio::task::JoinError>),
}

impl SourceScanTask {
    async fn join(&mut self) -> SourceScanJoin {
        match self {
            Self::Count { task, .. } => SourceScanJoin::Count(task.join_mut().await),
            Self::Auxiliary(task) => SourceScanJoin::Auxiliary(task.join_mut().await),
        }
    }

    async fn abort_and_join(&mut self) {
        match self {
            Self::Count { task, .. } => {
                let _ = task.abort_and_join_mut().await;
            }
            Self::Auxiliary(task) => {
                let _ = task.abort_and_join_mut().await;
            }
        }
    }

    fn is_reported_count(&self) -> bool {
        matches!(self, Self::Count { reported: true, .. })
    }
}

pub struct SourceScan {
    /// Creation order is dependency order: each decorator/replacement owns
    /// the preceding stage's receiver. Error cleanup reaps this stack in
    /// reverse so downstream receivers close before upstream producers.
    tasks: Vec<SourceScanTask>,
}

impl SourceScan {
    pub fn new(primary: tokio::task::JoinHandle<Result<u64>>) -> Self {
        Self {
            tasks: vec![SourceScanTask::Count {
                task: AbortOnDrop::new(primary),
                reported: true,
            }],
        }
    }

    pub fn add_auxiliary(&mut self, task: tokio::task::JoinHandle<()>) {
        self.tasks
            .push(SourceScanTask::Auxiliary(AbortOnDrop::new(task)));
    }

    /// Replace the reported count producer while retaining the prior scan as
    /// an owned child whose failure and lifetime remain part of this run.
    pub fn replace_primary(&mut self, primary: tokio::task::JoinHandle<Result<u64>>) {
        for task in &mut self.tasks {
            if let SourceScanTask::Count { reported, .. } = task {
                *reported = false;
            }
        }
        self.tasks.push(SourceScanTask::Count {
            task: AbortOnDrop::new(primary),
            reported: true,
        });
    }

    pub async fn finish(&mut self) -> Result<u64> {
        let reported_index = self
            .tasks
            .iter()
            .position(SourceScanTask::is_reported_count)
            .expect("SourceScan::finish called once");
        let reported = self.tasks[reported_index].join().await;
        self.tasks.remove(reported_index);
        let count = match reported {
            SourceScanJoin::Count(Ok(Ok(count))) => count,
            SourceScanJoin::Count(Ok(Err(error))) => {
                self.abort_and_join().await;
                return Err(error);
            }
            SourceScanJoin::Count(Err(error)) => {
                self.abort_and_join().await;
                return Err(eyre::eyre!("manifest scan task panicked: {error}"));
            }
            SourceScanJoin::Auxiliary(_) => unreachable!("reported task is always a count"),
        };

        while !self.tasks.is_empty() {
            let joined = self
                .tasks
                .last_mut()
                .expect("non-empty manifest task stack")
                .join()
                .await;
            self.tasks.pop();
            match joined {
                SourceScanJoin::Count(Ok(Ok(_))) | SourceScanJoin::Auxiliary(Ok(())) => {}
                SourceScanJoin::Count(Ok(Err(error))) => {
                    self.abort_and_join().await;
                    return Err(error);
                }
                SourceScanJoin::Count(Err(error)) => {
                    self.abort_and_join().await;
                    return Err(eyre::eyre!("manifest scan task panicked: {error}"));
                }
                SourceScanJoin::Auxiliary(Err(error)) => {
                    self.abort_and_join().await;
                    return Err(eyre::eyre!("manifest helper panicked: {error}"));
                }
            }
        }
        Ok(count)
    }

    pub async fn abort_and_join(&mut self) {
        while !self.tasks.is_empty() {
            self.tasks
                .last_mut()
                .expect("non-empty manifest task stack")
                .abort_and_join()
                .await;
            self.tasks.pop();
        }
    }
}

#[async_trait]
pub trait TransferSource: Send + Sync {
    /// Scans the source and streams discovered file headers.
    /// Returns a receiver for the headers and the owned validation-task run.
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan);

    /// Scan under the explicit lossy policy. Implementations that can avoid
    /// inspecting Windows metadata should override this; the default remains
    /// correct for abstract/test sources by stripping their emitted headers.
    fn scan_without_windows_metadata(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let (header_rx, scan) = self.scan(filter, unreadable_paths);
        strip_windows_metadata_from_scan(header_rx, scan)
    }

    /// Prepares a payload for transfer (e.g. opens a file or builds a tar shard).
    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload>;

    /// Checks if the files in the headers are available for transfer.
    /// Returns a list of available headers.
    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>>;

    /// Opens a file for reading.
    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>>;

    /// Returns the root path of the source (if applicable).
    fn root(&self) -> &Path;
}

pub struct FsTransferSource {
    root: PathBuf,
}

impl FsTransferSource {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl TransferSource for FsTransferSource {
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let (headers, task) = spawn_manifest_task(
            self.root.clone(),
            filter.unwrap_or_default(),
            unreadable_paths,
            true,
        );
        (headers, SourceScan::new(task))
    }

    fn scan_without_windows_metadata(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let (headers, task) = spawn_manifest_task(
            self.root.clone(),
            filter.unwrap_or_default(),
            unreadable_paths,
            false,
        );
        (headers, SourceScan::new(task))
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
        use crate::remote::transfer::payload::prepare_payload;
        prepare_payload(payload, self.root.clone()).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>> {
        filter_readable_headers(&self.root, headers, &unreadable_paths).await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        // An empty relative_path means "the root itself" — used when the
        // enumeration root is a single file. Don't join, because
        // PathBuf::join with some Path representations can produce a
        // trailing-slash form that OS interprets as "descend into" and
        // fails with ENOTDIR when the root is a regular file.
        let path = if header.relative_path.is_empty() {
            self.root.clone()
        } else {
            self.root.join(&header.relative_path)
        };
        let file = fs::File::open(&path).await?;
        Ok(Box::new(file))
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

/// Stream a manifest scan of `root` as `FileHeader`s (otp-10c-2:
/// relocated verbatim from the deleted push driver's
/// `client::helpers` — `FsTransferSource` is its only consumer now).
///
/// R46-F2: suppressed walk errors and unreadable files land in
/// `unreadable` so a downstream mirror-deletion gate sees "scan was
/// incomplete" via a single check.
fn spawn_manifest_task(
    root: PathBuf,
    filter: FileFilter,
    unreadable: Arc<Mutex<Vec<String>>>,
    preserve_windows_metadata: bool,
) -> (
    mpsc::Receiver<FileHeader>,
    tokio::task::JoinHandle<Result<u64>>,
) {
    use crate::enumeration::{EntryKind, FileEnumerator};
    use eyre::eyre;
    use std::io::ErrorKind;
    use std::time::{Duration, Instant};

    let (manifest_tx, manifest_rx) = mpsc::channel::<FileHeader>(64);
    let handle = tokio::task::spawn_blocking(move || -> Result<u64> {
        let enumerator = FileEnumerator::new(filter);
        let start = Instant::now();
        let mut last_log = start;
        let mut enumerated: u64 = 0;
        let unreadable = unreadable;
        let scan_outcome = enumerator.enumerate_local_streaming_capturing(&root, |entry| {
            if let EntryKind::File { size } = entry.kind {
                let rel = crate::path_posix::relative_path_to_posix(&entry.relative_path);
                let absolute = entry.absolute_path.clone();

                if let Err(err) = std::fs::File::open(&absolute) {
                    match err.kind() {
                        ErrorKind::PermissionDenied => {
                            record_unreadable_entry(&unreadable, &rel, "permission denied");
                            return Ok(());
                        }
                        ErrorKind::NotFound => {
                            record_unreadable_entry(&unreadable, &rel, "not found");
                            return Ok(());
                        }
                        _ => {
                            return Err(eyre!(format!(
                                "manifest open {}: {}",
                                absolute.display(),
                                err
                            )));
                        }
                    }
                }

                let mtime = crate::wire_metadata::mtime_seconds(&entry.metadata).unwrap_or(0);
                let permissions = crate::wire_metadata::permissions_mode(&entry.metadata);
                let Some(header) = file_header_with_windows_metadata_policy(
                    rel,
                    size,
                    mtime,
                    permissions,
                    &absolute,
                    &unreadable,
                    preserve_windows_metadata,
                    crate::windows_metadata::read_manifest,
                ) else {
                    return Ok(());
                };
                manifest_tx
                    .blocking_send(header)
                    .map_err(|_| eyre!("failed to queue manifest entry"))?;
                enumerated += 1;
                if last_log.elapsed() >= Duration::from_secs(1) {
                    // R46-F4: progress to stderr, never stdout — the
                    // CLI's `--json` modes own stdout.
                    eprintln!("Enumerated {} entries… (streaming manifest)", enumerated);
                    last_log = Instant::now();
                }
            }
            Ok(())
        })?;
        for suppressed in &scan_outcome.suppressed_errors {
            record_unreadable_entry(
                &unreadable,
                &suppressed.path,
                &format!("scan suppressed: {}", suppressed.message),
            );
        }
        eprintln!(
            "Manifest enumeration complete in {:.2?} ({} entries)",
            start.elapsed(),
            enumerated
        );
        Ok(enumerated)
    });

    (manifest_rx, handle)
}

#[allow(clippy::too_many_arguments)]
fn file_header_with_windows_metadata_policy(
    relative_path: String,
    size: u64,
    mtime_seconds: i64,
    permissions: u32,
    absolute_path: &Path,
    unreadable: &Arc<Mutex<Vec<String>>>,
    preserve_windows_metadata: bool,
    read_metadata: impl FnOnce(&Path) -> Result<Option<crate::generated::WindowsFileMetadata>>,
) -> Option<FileHeader> {
    if !preserve_windows_metadata {
        return Some(FileHeader {
            relative_path,
            size,
            mtime_seconds,
            permissions,
            checksum: Vec::new(),
            windows_metadata: None,
        });
    }
    file_header_with_windows_metadata(
        relative_path,
        size,
        mtime_seconds,
        permissions,
        absolute_path,
        unreadable,
        read_metadata,
    )
}

fn file_header_with_windows_metadata(
    relative_path: String,
    size: u64,
    mtime_seconds: i64,
    permissions: u32,
    absolute_path: &Path,
    unreadable: &Arc<Mutex<Vec<String>>>,
    read_metadata: impl FnOnce(&Path) -> Result<Option<crate::generated::WindowsFileMetadata>>,
) -> Option<FileHeader> {
    let windows_metadata = match read_metadata(absolute_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            record_unreadable_entry(
                unreadable,
                &relative_path,
                &format!("Windows metadata: {error:#}"),
            );
            return None;
        }
    };
    Some(FileHeader {
        relative_path,
        size,
        mtime_seconds,
        permissions,
        checksum: vec![],
        windows_metadata,
    })
}

fn record_unreadable_entry(list: &Arc<Mutex<Vec<String>>>, rel: &str, reason: &str) {
    log::warn!("scan skipping '{}' ({})", rel, reason);
    if let Ok(mut guard) = list.lock() {
        guard.push(format!("{} ({})", rel, reason));
    }
}

/// Filter `headers` down to the ones whose files are still readable
/// under `source_root`, recording the rest in `unreadable`
/// (otp-10c-2: relocated verbatim from the deleted push driver's
/// `client::helpers`).
async fn filter_readable_headers(
    source_root: &Path,
    headers: Vec<FileHeader>,
    unreadable: &Arc<Mutex<Vec<String>>>,
) -> Result<Vec<FileHeader>> {
    use eyre::eyre;
    use std::io::ErrorKind;

    let mut filtered = Vec::with_capacity(headers.len());
    for header in headers {
        let rel = header.relative_path.clone();
        // Empty relative_path means "the root is itself the file" — a
        // single-file source. `source_root.join("")` preserves a
        // trailing separator that `File::open` then rejects as
        // ENOTDIR, so treat the empty case specially.
        let path = if rel.is_empty() {
            source_root.to_path_buf()
        } else {
            source_root.join(&rel)
        };
        match fs::File::open(&path).await {
            Ok(file) => drop(file),
            Err(err) => match err.kind() {
                ErrorKind::PermissionDenied => {
                    record_unreadable_entry(unreadable, &rel, "permission denied");
                    continue;
                }
                ErrorKind::NotFound => {
                    record_unreadable_entry(unreadable, &rel, "not found");
                    continue;
                }
                _ => {
                    return Err(eyre!(format!(
                        "opening {} during payload planning: {}",
                        path.display(),
                        err
                    )));
                }
            },
        }
        filtered.push(header);
    }
    Ok(filtered)
}

/// Decorator that applies a `FileFilter` uniformly to any inner
/// `TransferSource`'s scan output. This is the SINGLE filter chokepoint
/// for every src/dst combination (local→local, push, pull, remote→remote);
/// per-source filtering is intentionally avoided so that future source
/// implementations get filtering for free and parity is enforced.
///
/// The orchestrator/CLI wraps the real source once with this decorator
/// before handing it to the pipeline. All other methods delegate to the
/// inner source unchanged.
pub struct FilteredSource {
    inner: Arc<dyn TransferSource>,
    filter: FileFilter,
}

/// Explicit lossy-policy decorator. It removes Windows attributes and named
/// stream descriptors before the manifest leaves the SOURCE. Payload planning
/// therefore never hydrates, reads, or sends the discarded stream content.
pub struct WindowsMetadataDroppingSource {
    inner: Arc<dyn TransferSource>,
}

impl WindowsMetadataDroppingSource {
    pub fn new(inner: Arc<dyn TransferSource>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl TransferSource for WindowsMetadataDroppingSource {
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.inner
            .scan_without_windows_metadata(filter, unreadable_paths)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

impl FilteredSource {
    pub fn new(inner: Arc<dyn TransferSource>, filter: FileFilter) -> Self {
        Self { inner, filter }
    }
}

#[async_trait]
impl TransferSource for FilteredSource {
    fn scan(
        &self,
        // Ignored: the wrapper carries the filter that's been calculated
        // by the orchestrator. Inner source emits unfiltered headers.
        _filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.scan_with_metadata_policy(unreadable_paths, true)
    }

    fn scan_without_windows_metadata(
        &self,
        _filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.scan_with_metadata_policy(unreadable_paths, false)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

impl FilteredSource {
    fn scan_with_metadata_policy(
        &self,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
        preserve_windows_metadata: bool,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let (header_rx, mut scan) = if preserve_windows_metadata {
            self.inner.scan(None, unreadable_paths)
        } else {
            self.inner
                .scan_without_windows_metadata(None, unreadable_paths)
        };
        if self.filter.is_empty() {
            // Fast path — no filter installed, return the inner channel
            // directly so we don't add a per-header forwarding hop.
            return (header_rx, scan);
        }
        let filter = self.filter.clone_without_cache();
        let (tx, rx_filtered) = mpsc::channel::<FileHeader>(64);
        // R59 finding #4: pass the inner source root so filter_headers
        // can fall back to the root's basename when relative_path is
        // empty. For single-file push, FsTransferSource emits the
        // entry with relative_path = "" (see open_file at source.rs:100);
        // pre-fix filter_headers asked allows_entry to match against
        // an empty PathBuf, so basename globs like `*.txt` silently
        // rejected the file.
        let source_root = self.inner.root().to_path_buf();
        let task = tokio::spawn(filter_headers(header_rx, tx, filter, source_root));
        scan.add_auxiliary(task);
        (rx_filtered, scan)
    }
}

/// Decorator that fills each scanned header's `checksum` by hashing the
/// file's content through the inner source's own `open_file` (otp-10b-1:
/// the SOURCE side of a `COMPARISON_MODE_CHECKSUM` session). Reading via
/// the trait keeps it source-impl-agnostic — the same chokepoint
/// reasoning as [`FilteredSource`], which it composes with (wrap OUTSIDE
/// the filter so only in-scope files pay the hash).
///
/// A file that cannot be opened or read for hashing is still EMITTED,
/// with an empty checksum — `compare_file`'s missing-checksum arm then
/// transfers it unconditionally (codex otp-10b-1 F1: dropping it would
/// let a pull "succeed" with the file silently absent, since only the
/// SOURCE end sees its own unreadable list; a genuinely unreadable file
/// then fails loudly at payload time like any other read failure).
pub struct ChecksummingSource {
    inner: Arc<dyn TransferSource>,
}

impl ChecksummingSource {
    pub fn new(inner: Arc<dyn TransferSource>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl TransferSource for ChecksummingSource {
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.scan_with_metadata_policy(filter, unreadable_paths, true)
    }

    fn scan_without_windows_metadata(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.scan_with_metadata_policy(filter, unreadable_paths, false)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

impl ChecksummingSource {
    fn scan_with_metadata_policy(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
        preserve_windows_metadata: bool,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let (mut header_rx, mut scan) = if preserve_windows_metadata {
            self.inner.scan(filter, unreadable_paths)
        } else {
            self.inner
                .scan_without_windows_metadata(filter, unreadable_paths)
        };
        let (tx, rx_hashed) = mpsc::channel::<FileHeader>(64);
        let inner = Arc::clone(&self.inner);
        // codex otp-10b-1 F2: the hashing task must not outlive its
        // consumer by a whole (arbitrarily large) file — the stop probe
        // is checked between 64 KiB hash chunks, bounding residual work
        // after a session ends to one chunk.
        let stop_probe = tx.clone();
        let task = tokio::spawn(async move {
            let stop = move || stop_probe.is_closed();
            while let Some(mut header) = header_rx.recv().await {
                match hash_header_content(inner.as_ref(), &header, &stop).await {
                    Ok(Some(checksum)) => header.checksum = checksum,
                    // Receiver gone mid-hash — the session ended; stop.
                    Ok(None) => break,
                    Err(err) => {
                        log::warn!(
                            "checksum scan: cannot hash '{}', transferring \
                             unconditionally: {err:#}",
                            header.relative_path
                        );
                        header.checksum = Vec::new();
                    }
                }
                if tx.send(header).await.is_err() {
                    break;
                }
            }
        });
        scan.add_auxiliary(task);
        (rx_hashed, scan)
    }
}

fn strip_windows_metadata_from_scan(
    mut header_rx: mpsc::Receiver<FileHeader>,
    mut scan: SourceScan,
) -> (mpsc::Receiver<FileHeader>, SourceScan) {
    let (tx, rx_stripped) = mpsc::channel::<FileHeader>(64);
    let task = tokio::spawn(async move {
        while let Some(mut header) = header_rx.recv().await {
            header.windows_metadata = None;
            if tx.send(header).await.is_err() {
                break;
            }
        }
    });
    scan.add_auxiliary(task);
    (rx_stripped, scan)
}

/// Blake3 of one header's content via the source's `open_file`.
/// Incremental 64 KiB reads keep memory flat; blake3 itself is fast
/// enough that hashing inline with the (I/O-bound) read is the simple
/// and adequate shape here. Returns `Ok(None)` when `stop` reports the
/// consumer is gone (checked between chunks — codex otp-10b-1 F2).
async fn hash_header_content(
    source: &dyn TransferSource,
    header: &FileHeader,
    stop: &(dyn Fn() -> bool + Send + Sync),
) -> Result<Option<Vec<u8>>> {
    use tokio::io::AsyncReadExt;
    let mut reader = source.open_file(header).await?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        if stop() {
            return Ok(None);
        }
        let got = reader.read(&mut buf).await?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
    }
    Ok(Some(hasher.finalize().as_bytes().to_vec()))
}

async fn filter_headers(
    mut rx: mpsc::Receiver<FileHeader>,
    tx: mpsc::Sender<FileHeader>,
    filter: FileFilter,
    source_root: PathBuf,
) {
    use std::time::{Duration, UNIX_EPOCH};
    while let Some(header) = rx.recv().await {
        let rel = PathBuf::from(&header.relative_path);
        let mtime = if header.mtime_seconds > 0 {
            UNIX_EPOCH.checked_add(Duration::from_secs(header.mtime_seconds as u64))
        } else {
            None
        };
        // R59 finding #4: an empty relative_path is the wire signal
        // for "this entry IS the source root" (single-file push).
        // Use the source root itself for filter matching so basename
        // globs work — otherwise filename derives from PathBuf::new(),
        // which has no file_name(), and `--include '*.txt'` rejects
        // every single-file push regardless of the actual filename.
        let (rel_for_filter, abs_for_filter): (Option<&Path>, &Path) = if rel.as_os_str().is_empty()
        {
            let root_name = source_root.file_name().map(Path::new);
            (root_name, source_root.as_path())
        } else {
            (Some(rel.as_path()), rel.as_path())
        };
        if !filter.allows_entry(rel_for_filter, abs_for_filter, header.size, mtime) {
            continue;
        }
        if tx.send(header).await.is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod filtered_source_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex as StdMutex;
    use tokio::sync::mpsc::channel;

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn windows_metadata_scan_error_marks_one_file_unreadable_and_keeps_neighbor() {
        let unreadable: Arc<Mutex<Vec<String>>> = Arc::default();
        let failed = file_header_with_windows_metadata(
            "bad.bin".into(),
            1,
            0,
            0,
            Path::new("bad.bin"),
            &unreadable,
            |_| Err(eyre::eyre!("oversized named stream")),
        );
        assert!(
            failed.is_none(),
            "the affected file must not enter the manifest"
        );

        let neighbor = file_header_with_windows_metadata(
            "good.bin".into(),
            1,
            0,
            0,
            Path::new("good.bin"),
            &unreadable,
            |_| Ok(None),
        )
        .expect("an unrelated readable file still enters the manifest");
        assert_eq!(neighbor.relative_path, "good.bin");
        let unreadable = unreadable.lock().unwrap();
        assert_eq!(unreadable.len(), 1);
        assert!(unreadable[0].contains("bad.bin (Windows metadata:"));
    }

    #[test]
    fn explicit_lossy_fs_scan_does_not_inspect_windows_metadata() {
        let unreadable: Arc<Mutex<Vec<String>>> = Arc::default();
        let header = file_header_with_windows_metadata_policy(
            "file.bin".into(),
            7,
            11,
            0,
            Path::new("file.bin"),
            &unreadable,
            false,
            |_| panic!("lossy source scan must not enumerate or hash named streams"),
        )
        .expect("the primary file still enters the manifest");
        assert_eq!(header.relative_path, "file.bin");
        assert!(header.windows_metadata.is_none());
        assert!(unreadable.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn abort_reaps_downstream_replacement_before_blocked_scan_producer() {
        let (tx, rx) = mpsc::channel(1);
        let second_send = Arc::new(tokio::sync::Notify::new());
        let second_send_task = Arc::clone(&second_send);
        let child = tokio::task::spawn_blocking(move || {
            tx.blocking_send(()).expect("first manifest item queued");
            second_send_task.notify_one();
            let _ = tx.blocking_send(());
            Ok(1)
        });

        let replacement_dropped = Arc::new(AtomicBool::new(false));
        let replacement_dropped_task = Arc::clone(&replacement_dropped);
        let replacement_entered = Arc::new(tokio::sync::Notify::new());
        let replacement_entered_task = Arc::clone(&replacement_entered);
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let replacement = tokio::spawn(async move {
            let _drop_flag = DropFlag(replacement_dropped_task);
            replacement_entered_task.notify_one();
            let _ = release_rx.await;
            drop(rx);
            Ok(2)
        });
        let mut scan = SourceScan::new(child);
        scan.replace_primary(replacement);
        replacement_entered.notified().await;
        second_send.notified().await;

        if tokio::time::timeout(std::time::Duration::from_secs(5), scan.abort_and_join())
            .await
            .is_err()
        {
            // Mutation cleanup: release the replacement so an upstream-first
            // implementation cannot strand the blocking pool after failing.
            let _ = release_tx.send(());
            scan.abort_and_join().await;
            panic!("scan cleanup awaited its blocked producer before closing the receiver");
        }
        assert!(
            replacement_dropped.load(Ordering::SeqCst),
            "downstream receiver owner is destroyed before cleanup returns"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn scan_error_reaps_pending_validation_helpers_before_return() {
        let release = Arc::new(tokio::sync::Notify::new());
        let release_task = Arc::clone(&release);
        let primary = tokio::spawn(async move {
            release_task.notified().await;
            Err(eyre::eyre!("injected manifest failure"))
        });
        let entered = Arc::new(tokio::sync::Notify::new());
        let entered_task = Arc::clone(&entered);
        let dropped = Arc::new(AtomicBool::new(false));
        let dropped_task = Arc::clone(&dropped);
        let helper = tokio::spawn(async move {
            let _drop_flag = DropFlag(dropped_task);
            entered_task.notify_one();
            std::future::pending::<()>().await;
        });
        let mut scan = SourceScan::new(primary);
        scan.add_auxiliary(helper);
        tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
            .await
            .expect("validation helper entered");
        release.notify_one();

        let error = tokio::time::timeout(std::time::Duration::from_secs(5), scan.finish())
            .await
            .expect("scan failure cleanup timed out")
            .expect_err("primary scan must fail");
        assert!(format!("{error:#}").contains("injected manifest failure"));
        assert!(
            dropped.load(Ordering::SeqCst),
            "validation helper must be destroyed before scan failure returns"
        );
    }

    /// Stub source that emits a fixed list of headers. Lets us verify
    /// `FilteredSource` filtering behavior independent of the real fs/remote
    /// source impls.
    struct StubSource {
        headers: StdMutex<Option<Vec<FileHeader>>>,
        root: PathBuf,
    }

    impl StubSource {
        fn new(headers: Vec<FileHeader>) -> Self {
            Self {
                headers: StdMutex::new(Some(headers)),
                root: PathBuf::from("/stub"),
            }
        }

        fn with_root(headers: Vec<FileHeader>, root: PathBuf) -> Self {
            Self {
                headers: StdMutex::new(Some(headers)),
                root,
            }
        }
    }

    #[async_trait]
    impl TransferSource for StubSource {
        fn scan(
            &self,
            _filter: Option<FileFilter>,
            _unreadable: Arc<Mutex<Vec<String>>>,
        ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
            let headers = self.headers.lock().unwrap().take().unwrap_or_default();
            let (tx, rx) = channel(64);
            let count = headers.len() as u64;
            let handle = tokio::spawn(async move {
                for h in headers {
                    if tx.send(h).await.is_err() {
                        break;
                    }
                }
                Ok(count)
            });
            (rx, SourceScan::new(handle))
        }

        async fn prepare_payload(&self, _: TransferPayload) -> Result<PreparedPayload> {
            unimplemented!()
        }

        async fn check_availability(
            &self,
            h: Vec<FileHeader>,
            _: Arc<Mutex<Vec<String>>>,
        ) -> Result<Vec<FileHeader>> {
            Ok(h)
        }

        async fn open_file(
            &self,
            _: &FileHeader,
        ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
            unimplemented!()
        }

        fn root(&self) -> &Path {
            &self.root
        }
    }

    fn header(rel: &str, size: u64) -> FileHeader {
        FileHeader {
            relative_path: rel.into(),
            size,
            mtime_seconds: 0,
            permissions: 0,
            checksum: vec![],
            windows_metadata: None,
        }
    }

    async fn collect(mut rx: mpsc::Receiver<FileHeader>) -> Vec<String> {
        let mut out = Vec::new();
        while let Some(h) = rx.recv().await {
            out.push(h.relative_path);
        }
        out
    }

    #[tokio::test]
    async fn explicit_lossy_source_strips_windows_metadata_before_manifest() {
        let mut metadata_header = header("metadata.bin", 10);
        metadata_header.windows_metadata = Some(crate::generated::WindowsFileMetadata {
            file_attributes: 0x20,
            named_streams: vec![crate::generated::WindowsNamedStream {
                name: "tag".into(),
                size: 3,
                checksum: blake3::hash(b"tag").as_bytes().to_vec(),
                content: Vec::new(),
            }],
        });
        let inner: Arc<dyn TransferSource> = Arc::new(StubSource::new(vec![
            metadata_header,
            header("plain.bin", 5),
        ]));
        let source = WindowsMetadataDroppingSource::new(inner);
        let (mut rx, mut scan) = source.scan(None, Arc::new(Mutex::new(Vec::new())));

        let mut emitted = Vec::new();
        while let Some(header) = rx.recv().await {
            emitted.push(header);
        }
        assert_eq!(scan.finish().await.expect("scan completes"), 2);
        assert_eq!(emitted.len(), 2);
        assert_eq!(emitted[0].relative_path, "metadata.bin");
        assert!(
            emitted
                .iter()
                .all(|header| header.windows_metadata.is_none()),
            "the lossy policy must remove metadata from every emitted header"
        );
    }

    #[tokio::test]
    async fn empty_filter_passes_everything() {
        let inner: Arc<dyn TransferSource> = Arc::new(StubSource::new(vec![
            header("a.txt", 10),
            header("b.log", 20),
        ]));
        let filtered = FilteredSource::new(inner, FileFilter::default());
        let (rx, _h) = filtered.scan(None, Arc::new(Mutex::new(Vec::new())));
        let names = collect(rx).await;
        assert_eq!(names, vec!["a.txt", "b.log"]);
    }

    #[tokio::test]
    async fn exclude_pattern_drops_match() {
        let inner: Arc<dyn TransferSource> = Arc::new(StubSource::new(vec![
            header("keep.txt", 10),
            header("drop.tmp", 20),
            header("also.tmp", 30),
        ]));
        let mut filter = FileFilter::default();
        filter.exclude_files = vec!["*.tmp".to_string()];
        let filtered = FilteredSource::new(inner, filter);
        let (rx, _h) = filtered.scan(None, Arc::new(Mutex::new(Vec::new())));
        let names = collect(rx).await;
        assert_eq!(names, vec!["keep.txt"]);
    }

    #[tokio::test]
    async fn include_pattern_restricts_to_match() {
        let inner: Arc<dyn TransferSource> = Arc::new(StubSource::new(vec![
            header("a.log", 10),
            header("b.txt", 20),
            header("c.log", 30),
        ]));
        let mut filter = FileFilter::default();
        filter.include_files = vec!["*.log".to_string()];
        let filtered = FilteredSource::new(inner, filter);
        let (rx, _h) = filtered.scan(None, Arc::new(Mutex::new(Vec::new())));
        let mut names = collect(rx).await;
        names.sort();
        assert_eq!(names, vec!["a.log", "c.log"]);
    }

    #[tokio::test]
    async fn size_filter_applied() {
        let inner: Arc<dyn TransferSource> = Arc::new(StubSource::new(vec![
            header("small", 50),
            header("medium", 500),
            header("large", 5000),
        ]));
        let mut filter = FileFilter::default();
        filter.min_size = Some(100);
        filter.max_size = Some(1000);
        let filtered = FilteredSource::new(inner, filter);
        let (rx, _h) = filtered.scan(None, Arc::new(Mutex::new(Vec::new())));
        let names = collect(rx).await;
        assert_eq!(names, vec!["medium"]);
    }

    /// R59 finding #4: single-file push emits a header with an
    /// empty `relative_path`. Pre-fix the filter ran allows_entry
    /// against an empty PathBuf, so `--include '*.txt'` rejected
    /// the file even when the source root's basename matched.
    /// Post-fix the filter falls back to the source root's
    /// basename when the relative path is empty.
    #[tokio::test]
    async fn single_file_root_matches_basename_globs() {
        let inner: Arc<dyn TransferSource> = Arc::new(StubSource::with_root(
            vec![header("", 42)],
            PathBuf::from("/tmp/payload.txt"),
        ));
        let mut filter = FileFilter::default();
        filter.include_files = vec!["*.txt".to_string()];
        let filtered = FilteredSource::new(inner, filter);
        let (rx, _h) = filtered.scan(None, Arc::new(Mutex::new(Vec::new())));
        let names = collect(rx).await;
        assert_eq!(
            names,
            vec![""],
            "single-file root with basename matching --include must pass"
        );
    }

    /// R59 finding #4 (negative case): with the same wire shape,
    /// a non-matching basename glob must reject — confirms the
    /// fallback uses the actual basename, not a permissive
    /// "anything passes when rel is empty" shortcut.
    #[tokio::test]
    async fn single_file_root_basename_glob_can_exclude() {
        let inner: Arc<dyn TransferSource> = Arc::new(StubSource::with_root(
            vec![header("", 42)],
            PathBuf::from("/tmp/payload.log"),
        ));
        let mut filter = FileFilter::default();
        filter.include_files = vec!["*.txt".to_string()];
        let filtered = FilteredSource::new(inner, filter);
        let (rx, _h) = filtered.scan(None, Arc::new(Mutex::new(Vec::new())));
        let names = collect(rx).await;
        assert!(
            names.is_empty(),
            "single-file root that doesn't match --include must be rejected"
        );
    }

    #[tokio::test]
    async fn ignores_caller_filter_using_baked_in() {
        // Verify the decorator's filter wins over any filter passed to
        // scan() — this ensures the universal chokepoint isn't bypassed
        // by leaf code passing its own filter.
        let inner: Arc<dyn TransferSource> = Arc::new(StubSource::new(vec![header("a.tmp", 10)]));
        let mut baked_in = FileFilter::default();
        baked_in.exclude_files = vec!["*.tmp".to_string()];
        let filtered = FilteredSource::new(inner, baked_in);
        // Caller passes empty filter; baked-in still applies
        let (rx, _h) = filtered.scan(
            Some(FileFilter::default()),
            Arc::new(Mutex::new(Vec::new())),
        );
        let names = collect(rx).await;
        assert!(names.is_empty(), "baked-in filter should drop a.tmp");
    }
}

#[cfg(test)]
mod checksumming_source_tests {
    use super::*;
    use eyre::bail;
    use std::sync::Mutex as StdMutex;
    use tokio::sync::mpsc::channel;

    /// Stub whose `open_file` serves bytes for every header except
    /// ones named `unhashable*`, which error — the codex otp-10b-1 F1
    /// shape (a file the scan listed but the hash pass cannot read).
    struct HashStub {
        headers: StdMutex<Option<Vec<FileHeader>>>,
        root: PathBuf,
    }

    #[async_trait]
    impl TransferSource for HashStub {
        fn scan(
            &self,
            _filter: Option<FileFilter>,
            _unreadable: Arc<Mutex<Vec<String>>>,
        ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
            let headers = self.headers.lock().unwrap().take().unwrap_or_default();
            let (tx, rx) = channel(64);
            let count = headers.len() as u64;
            let handle = tokio::spawn(async move {
                for h in headers {
                    if tx.send(h).await.is_err() {
                        break;
                    }
                }
                Ok(count)
            });
            (rx, SourceScan::new(handle))
        }

        async fn prepare_payload(&self, _: TransferPayload) -> Result<PreparedPayload> {
            unimplemented!()
        }

        async fn check_availability(
            &self,
            h: Vec<FileHeader>,
            _: Arc<Mutex<Vec<String>>>,
        ) -> Result<Vec<FileHeader>> {
            Ok(h)
        }

        async fn open_file(
            &self,
            header: &FileHeader,
        ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
            if header.relative_path.starts_with("unhashable") {
                bail!("permission denied (stub)");
            }
            Ok(Box::new(std::io::Cursor::new(b"content".to_vec())))
        }

        fn root(&self) -> &Path {
            &self.root
        }
    }

    fn header(rel: &str) -> FileHeader {
        FileHeader {
            relative_path: rel.into(),
            size: 7,
            mtime_seconds: 0,
            permissions: 0,
            checksum: vec![],
            windows_metadata: None,
        }
    }

    /// codex otp-10b-1 F1: an unhashable file must still be EMITTED —
    /// with an empty checksum, so the destination's missing-checksum
    /// arm transfers it unconditionally. Dropping it would let a pull
    /// report success with the file silently absent (only the SOURCE
    /// sees its own unreadable list). Hashable neighbors get real
    /// checksums.
    #[tokio::test]
    async fn unhashable_files_are_emitted_with_empty_checksums() {
        let stub = Arc::new(HashStub {
            headers: StdMutex::new(Some(vec![header("ok.txt"), header("unhashable.txt")])),
            root: PathBuf::from("/stub"),
        });
        let source = ChecksummingSource::new(stub);
        let (mut rx, mut scan) = source.scan(None, Arc::default());

        let mut got = std::collections::BTreeMap::new();
        while let Some(h) = rx.recv().await {
            got.insert(h.relative_path.clone(), h.checksum);
        }
        scan.finish().await.unwrap();

        assert_eq!(
            got.len(),
            2,
            "every scanned header is emitted, hashable or not"
        );
        assert_eq!(
            got["ok.txt"],
            blake3::hash(b"content").as_bytes().to_vec(),
            "hashable files carry their real Blake3"
        );
        assert!(
            got["unhashable.txt"].is_empty(),
            "unhashable files carry the empty checksum (conservative transfer)"
        );
    }
}
