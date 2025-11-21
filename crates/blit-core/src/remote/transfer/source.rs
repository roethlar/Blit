use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use eyre::{Result, eyre};
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
    async fn prepare_payload(
        &self,
        payload: TransferPayload,
    ) -> Result<PreparedPayload>;

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

    async fn prepare_payload(
        &self,
        payload: TransferPayload,
    ) -> Result<PreparedPayload> {
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
        let path = self.root.join(&header.relative_path);
        let file = fs::File::open(&path).await?;
        Ok(Box::new(file))
    }

    fn root(&self) -> &Path {
        &self.root
    }
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
        let root = self.root.clone(); // Use root as relative path base if needed, but scan_remote_files takes path relative to module

        let handle = tokio::spawn(async move {
            // For now, we assume we scan the root of the remote module/path
            // If self.root is "remote:/module/path", we might want to scan "." relative to that.
            // scan_remote_files takes a path relative to the client's endpoint path.
            // If client endpoint is "remote:/module", and we want to scan ".", we pass ".".
            let headers = client.scan_remote_files(Path::new(".")).await?;
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

    async fn prepare_payload(
        &self,
        payload: TransferPayload,
    ) -> Result<PreparedPayload> {
        match payload {
            TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
            TransferPayload::TarShard { headers } => {
                let mut builder = tar::Builder::new(Vec::new());
                for header in headers.clone() {
                    let mut stream = self.client.open_remote_file(Path::new(&header.relative_path)).await?;
                    let mut data = Vec::new();
                    stream.read_to_end(&mut data).await?;
                    
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
        eprintln!("DEBUG: RemoteTransferSource opening remote file {}", header.relative_path);
        let stream = self.client.open_remote_file(Path::new(&header.relative_path)).await?;
        Ok(Box::new(stream))
    }

    fn root(&self) -> &Path {
        &self.root
    }
}
