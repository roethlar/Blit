use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use eyre::{bail, Result};
use tokio::fs;
use tokio::sync::mpsc;

use crate::fs_enum::FileFilter;
use crate::generated::FileHeader;
use crate::remote::pull::RemotePullClient;
use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
use tokio::io::AsyncReadExt;

#[async_trait]
pub trait TransferSource: Send + Sync {
    /// Scans the source and streams discovered file headers.
    /// Returns a receiver for the headers and a join handle for the scan task.
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (
        mpsc::Receiver<FileHeader>,
        tokio::task::JoinHandle<Result<u64>>,
    );

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
    ) -> (
        mpsc::Receiver<FileHeader>,
        tokio::task::JoinHandle<Result<u64>>,
    ) {
        use crate::remote::push::client::helpers::spawn_manifest_task;
        spawn_manifest_task(
            self.root.clone(),
            filter.unwrap_or_default(),
            unreadable_paths,
        )
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
        use crate::remote::push::client::helpers::filter_readable_headers;
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

/// Validate `FileHeader` sizes for a remote-source tar shard against
/// the shared cap. Both the per-entry size and the cumulative shard
/// size must stay within `tar_safety::MAX_TAR_SHARD_BYTES` so a
/// hostile or buggy remote source can't force the relay into an
/// unbounded allocation while building the tar.
///
/// Extracted from `RemoteTransferSource::prepare_payload` so the
/// bounds are unit-testable without a `RemotePullClient` mock.
/// Closes F7 of `docs/reviews/codebase_review_2026-05-01.md`.
fn validate_remote_tar_shard_sizes(headers: &[FileHeader]) -> Result<()> {
    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;

    for header in headers {
        if header.size > MAX_TAR_SHARD_BYTES {
            bail!(
                "remote-source tar entry '{}' size {} exceeds local cap {} bytes",
                header.relative_path,
                header.size,
                MAX_TAR_SHARD_BYTES
            );
        }
    }
    let total_bytes: u64 = headers
        .iter()
        .try_fold(0u64, |acc, h| acc.checked_add(h.size))
        .ok_or_else(|| eyre::eyre!("remote-source tar shard size sum overflows u64"))?;
    if total_bytes > MAX_TAR_SHARD_BYTES {
        bail!(
            "remote-source tar shard total size {} exceeds local cap {} bytes",
            total_bytes,
            MAX_TAR_SHARD_BYTES
        );
    }
    Ok(())
}

/// Read exactly `expected_size` bytes from a remote-source stream
/// into a bounded `Vec<u8>`. Closes R11-F1 of
/// `docs/reviews/followup_review_2026-05-02.md`: previously the
/// caller did `try_reserve_exact(size)` then `read_to_end(...)`,
/// which only bounded the *reservation* — `read_to_end` would still
/// grow the Vec past the bound if the remote source streamed extra
/// bytes. Now the read itself is wrapped with `take(size + 1)` so
/// over-reads are bounded at one byte past the declared size, and
/// the post-read length check rejects both lie-large and lie-small.
///
/// Extracted as a free function so it's unit-testable against any
/// `AsyncRead` (a real `RemotePullClient` stream isn't required).
async fn read_remote_entry_bounded<R>(reader: R, expected_size: u64, label: &str) -> Result<Vec<u8>>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;

    // Defense-in-depth: this helper is private and current callers
    // pre-validate, but reject explicitly so any future caller that
    // bypasses validation can't allocate past the cap.
    if expected_size > MAX_TAR_SHARD_BYTES {
        bail!(
            "remote-source entry '{}' size {} exceeds local cap {} bytes",
            label,
            expected_size,
            MAX_TAR_SHARD_BYTES
        );
    }

    let mut data: Vec<u8> = Vec::new();
    data.try_reserve_exact(expected_size as usize)
        .map_err(|err| {
            eyre::eyre!(
                "allocating buffer for remote-source entry '{}' (size {}): {}",
                label,
                expected_size,
                err
            )
        })?;

    // Read at most `expected_size + 1` bytes. The +1 is the over-read
    // canary: if the post-read length is `expected_size + 1` we know
    // the source sent more than declared. `expected_size` is bounded
    // at `MAX_TAR_SHARD_BYTES` (above), so the addition can't overflow.
    let read_limit = expected_size + 1;
    let mut limited = reader.take(read_limit);
    limited.read_to_end(&mut data).await?;

    if data.len() as u64 != expected_size {
        bail!(
            "remote-source entry '{}' returned {} bytes; manifest declared {}",
            label,
            data.len(),
            expected_size
        );
    }
    Ok(data)
}

pub struct RemoteTransferSource {
    client: RemotePullClient,
    root: PathBuf,
}

impl RemoteTransferSource {
    pub fn new(client: RemotePullClient, root: PathBuf) -> Self {
        Self { client, root }
    }
}

#[async_trait]
impl TransferSource for RemoteTransferSource {
    fn scan(
        &self,
        _filter: Option<FileFilter>,
        _unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (
        mpsc::Receiver<FileHeader>,
        tokio::task::JoinHandle<Result<u64>>,
    ) {
        let (tx, rx) = mpsc::channel(64);
        let mut client = self.client.clone();
        let root = self.root.clone();
        // Filter is NOT applied here — the universal `FilteredSource`
        // decorator (see this module) is the single chokepoint that all
        // src/dst combinations route through. Per-source filtering would
        // mean every new TransferSource impl must remember to wire it up.

        let handle = tokio::spawn(async move {
            let headers = client.scan_remote_files(&root).await?;
            let count = headers.len() as u64;
            for header in headers {
                if tx.send(header).await.is_err() {
                    break;
                }
            }
            Ok(count)
        });

        (rx, handle)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
        match payload {
            TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
            TransferPayload::TarShard { headers } => {
                // F7 of docs/reviews/codebase_review_2026-05-01.md: bound
                // the relay's allocation against a hostile or buggy
                // remote source. This is the send-side mirror of R6-F1.
                // The size validation is extracted into a testable
                // helper so the bounds are pinned without needing a
                // mock RemotePullClient.
                validate_remote_tar_shard_sizes(&headers)?;

                let mut builder = tar::Builder::new(Vec::new());
                for header in headers.clone() {
                    let stream = self
                        .client
                        .open_remote_file(Path::new(&header.relative_path))
                        .await?;
                    let data =
                        read_remote_entry_bounded(stream, header.size, &header.relative_path)
                            .await?;

                    let mut tar_header = tar::Header::new_gnu();
                    tar_header.set_path(&header.relative_path)?;
                    tar_header.set_size(header.size);
                    tar_header.set_mode(header.permissions);
                    tar_header.set_mtime(header.mtime_seconds as u64);
                    tar_header.set_cksum();

                    builder.append_data(&mut tar_header, &header.relative_path, &data[..])?;
                }
                let data = builder.into_inner()?;
                Ok(PreparedPayload::TarShard { headers, data })
            }
            // Resume payloads originate on the receive side only.
            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
                bail!("FileBlock payloads cannot be prepared from a remote source")
            }
        }
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        _unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>> {
        // Assume all remote files are available if we scanned them
        Ok(headers)
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        let stream = self
            .client
            .open_remote_file(Path::new(&header.relative_path))
            .await?;
        Ok(Box::new(stream))
    }

    fn root(&self) -> &Path {
        &self.root
    }
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
    ) -> (
        mpsc::Receiver<FileHeader>,
        tokio::task::JoinHandle<Result<u64>>,
    ) {
        let (header_rx, scan_handle) = self.inner.scan(None, unreadable_paths);
        if self.filter.is_empty() {
            // Fast path — no filter installed, return the inner channel
            // directly so we don't add a per-header forwarding hop.
            return (header_rx, scan_handle);
        }
        let filter = self.filter.clone_without_cache();
        let (tx, rx_filtered) = mpsc::channel::<FileHeader>(64);
        tokio::spawn(filter_headers(header_rx, tx, filter));
        (rx_filtered, scan_handle)
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

async fn filter_headers(
    mut rx: mpsc::Receiver<FileHeader>,
    tx: mpsc::Sender<FileHeader>,
    filter: FileFilter,
) {
    use std::time::{Duration, UNIX_EPOCH};
    while let Some(header) = rx.recv().await {
        let rel = PathBuf::from(&header.relative_path);
        let mtime = if header.mtime_seconds > 0 {
            UNIX_EPOCH.checked_add(Duration::from_secs(header.mtime_seconds as u64))
        } else {
            None
        };
        if !filter.allows_entry(Some(&rel), &rel, header.size, mtime) {
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
    use std::sync::Mutex as StdMutex;
    use tokio::sync::mpsc::channel;

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
    }

    #[async_trait]
    impl TransferSource for StubSource {
        fn scan(
            &self,
            _filter: Option<FileFilter>,
            _unreadable: Arc<Mutex<Vec<String>>>,
        ) -> (
            mpsc::Receiver<FileHeader>,
            tokio::task::JoinHandle<Result<u64>>,
        ) {
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
            (rx, handle)
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
mod remote_tar_size_tests {
    use super::*;
    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;

    fn fh(rel: &str, size: u64) -> FileHeader {
        FileHeader {
            relative_path: rel.into(),
            size,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: vec![],
        }
    }

    #[test]
    fn accepts_within_bounds() {
        let headers = vec![fh("a", 1024), fh("b", 2048)];
        validate_remote_tar_shard_sizes(&headers).unwrap();
    }

    #[test]
    fn accepts_at_cap() {
        // Single entry exactly at the cap, total exactly at the cap.
        let headers = vec![fh("a", MAX_TAR_SHARD_BYTES)];
        validate_remote_tar_shard_sizes(&headers).unwrap();
    }

    #[test]
    fn rejects_per_entry_above_cap() {
        let headers = vec![fh("huge", MAX_TAR_SHARD_BYTES + 1)];
        let err = validate_remote_tar_shard_sizes(&headers).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("entry 'huge'") && msg.contains("exceeds"),
            "expected per-entry rejection, got: {msg}"
        );
    }

    #[test]
    fn rejects_cumulative_above_cap() {
        // Each entry is fine on its own (half the cap), but the sum
        // exceeds the cap. R6-F1's send-side analog.
        let half = MAX_TAR_SHARD_BYTES / 2 + 1;
        let headers = vec![fh("a", half), fh("b", half)];
        let err = validate_remote_tar_shard_sizes(&headers).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("total size") && msg.contains("exceeds"),
            "expected cumulative rejection, got: {msg}"
        );
    }

    #[test]
    fn rejects_u64_overflow_on_sum() {
        // A hostile peer claims two entries near u64::MAX so naive
        // summation overflows. checked_add must catch this before any
        // allocation. Per-entry sizes themselves exceed cap so the
        // first loop trips, but if the per-entry cap were ever
        // raised the cumulative check still bounds via checked_add.
        let headers = vec![fh("a", u64::MAX - 10), fh("b", u64::MAX - 10)];
        let err = validate_remote_tar_shard_sizes(&headers).unwrap_err();
        // Per-entry cap fires first, which is fine — the test pins
        // that one of the two checks rejects this input.
        assert!(err.to_string().contains("exceeds"));
    }
}

#[cfg(test)]
mod remote_bounded_read_tests {
    //! R11-F1 regression coverage. The bug: `try_reserve_exact`
    //! bounded the initial allocation but `read_to_end` would still
    //! grow the Vec past the bound if the remote source streamed
    //! extra bytes. Fix: wrap the reader with `take(size + 1)` so
    //! the read itself is bounded; the post-read length check
    //! rejects both over-read and under-read.

    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn happy_path_returns_exactly_declared_bytes() {
        let payload = b"abcdefghij";
        let reader = Cursor::new(payload.to_vec());
        let data = read_remote_entry_bounded(reader, payload.len() as u64, "ok.txt")
            .await
            .unwrap();
        assert_eq!(data, payload);
    }

    #[tokio::test]
    async fn rejects_under_read() {
        // Source sends fewer bytes than declared — must error, not
        // pad with zeros.
        let payload = b"only5"; // 5 bytes
        let reader = Cursor::new(payload.to_vec());
        let err = read_remote_entry_bounded(reader, 100, "short.txt")
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("returned") && msg.contains("100"),
            "expected length-mismatch error, got: {msg}"
        );
    }

    #[tokio::test]
    async fn rejects_over_read_bounded_at_size_plus_one() {
        // Source declares 4 bytes but streams 1 MiB. The bounded
        // read caps at 5 bytes (4 + 1) so the over-read is detected
        // without growing the buffer past the cap.
        let big_payload = vec![0xABu8; 1024 * 1024];
        let reader = Cursor::new(big_payload);
        let err = read_remote_entry_bounded(reader, 4, "lying.txt")
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("returned 5") && msg.contains("declared 4"),
            "expected over-read rejection at size+1, got: {msg}"
        );
    }

    #[tokio::test]
    async fn rejects_size_above_cap_defensively() {
        // Helper enforces its own size cap as defense in depth even
        // though the public callers pre-validate.
        use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
        let reader = Cursor::new(Vec::new());
        let err = read_remote_entry_bounded(reader, MAX_TAR_SHARD_BYTES + 1, "huge.txt")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("exceeds local cap"));
    }

    #[tokio::test]
    async fn empty_file_passes() {
        let reader = Cursor::new(Vec::<u8>::new());
        let data = read_remote_entry_bounded(reader, 0, "empty.txt")
            .await
            .unwrap();
        assert!(data.is_empty());
    }
}
