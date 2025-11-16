use std::path::Path;

use eyre::{bail, Context, Result};
use futures::StreamExt;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::generated::FileHeader;

use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};

pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1 * 1024 * 1024;
pub const DATA_PLANE_RECORD_FILE: u8 = 0;
pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
pub const DATA_PLANE_RECORD_END: u8 = 0xFF;

pub struct DataPlaneSession {
    stream: TcpStream,
    buffer: Vec<u8>,
    trace: bool,
    chunk_bytes: usize,
    payload_prefetch: usize,
    bytes_sent: u64,
}

macro_rules! trace_client {
    ($session:expr, $($arg:tt)*) => {
        if $session.trace {
            eprintln!("[data-plane-client] {}", format_args!($($arg)*));
        }
    };
}

impl DataPlaneSession {
    pub fn from_stream(
        stream: TcpStream,
        trace: bool,
        chunk_bytes: usize,
        payload_prefetch: usize,
    ) -> Self {
        let payload_prefetch = payload_prefetch.max(1);
        let buffer_len = chunk_bytes.max(64 * 1024);
        Self {
            stream,
            buffer: vec![0u8; buffer_len],
            trace,
            chunk_bytes: buffer_len,
            payload_prefetch,
            bytes_sent: 0,
        }
    }

    pub async fn connect(
        host: &str,
        port: u32,
        token: &[u8],
        chunk_bytes: usize,
        payload_prefetch: usize,
        trace: bool,
    ) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        if trace {
            eprintln!("[data-plane-client] connecting to {}", addr);
        }
        let mut stream = TcpStream::connect(addr.clone())
            .await
            .with_context(|| format!("connecting to data plane {}", addr))?;

        stream
            .write_all(token)
            .await
            .context("writing negotiation token")?;

        Ok(Self::from_stream(
            stream,
            trace,
            chunk_bytes,
            payload_prefetch,
        ))
    }

    pub async fn send_payloads(
        &mut self,
        source_root: &Path,
        payloads: Vec<TransferPayload>,
    ) -> Result<()> {
        let mut stream =
            prepared_payload_stream(payloads, source_root.to_path_buf(), self.payload_prefetch);
        while let Some(prepared) = stream.next().await {
            match prepared? {
                PreparedPayload::File(header) => {
                    if let Err(err) = self.send_file(source_root, &header).await {
                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
                    }
                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
                }
                PreparedPayload::TarShard { headers, data } => {
                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
                    if let Err(err) = self.send_prepared_tar_shard(headers, &data).await {
                        return Err(err.wrap_err("sending tar shard"));
                    }
                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
                }
            }
        }

        Ok(())
    }

    pub async fn finish(&mut self) -> Result<()> {
        self.stream
            .write_all(&[DATA_PLANE_RECORD_END])
            .await
            .context("writing transfer terminator")?;
        self.stream
            .flush()
            .await
            .context("flushing data plane stream")
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    async fn send_file(&mut self, source_root: &Path, header: &FileHeader) -> Result<()> {
        let rel = &header.relative_path;
        let path = source_root.join(rel);
        trace_client!(self, "sending file '{}' ({} bytes)", rel, header.size);

        let path_bytes = rel.as_bytes();
        if path_bytes.len() > u32::MAX as usize {
            bail!("relative path too long for transfer: {}", rel);
        }

        self.stream
            .write_all(&[DATA_PLANE_RECORD_FILE])
            .await
            .context("writing data-plane record tag")?;
        self.stream
            .write_all(&(path_bytes.len() as u32).to_be_bytes())
            .await
            .context("writing path length")?;
        self.stream
            .write_all(path_bytes)
            .await
            .context("writing path bytes")?;

        let metadata = fs::metadata(&path)
            .await
            .with_context(|| format!("stat {}", path.display()))?;
        if metadata.len() != header.size {
            bail!(
                "source file {} changed size (expected {}, found {})",
                path.display(),
                header.size,
                metadata.len()
            );
        }

        self.stream
            .write_all(&metadata.len().to_be_bytes())
            .await
            .context("writing file size")?;

        let mut file = fs::File::open(&path)
            .await
            .with_context(|| format!("opening {}", path.display()))?;

        let mut remaining = metadata.len();
        while remaining > 0 {
            let chunk = file
                .read(&mut self.buffer)
                .await
                .with_context(|| format!("reading {}", path.display()))?;
            if chunk == 0 {
                bail!(
                    "unexpected EOF while reading {} ({} bytes remaining)",
                    path.display(),
                    remaining
                );
            }
            self.stream
                .write_all(&self.buffer[..chunk])
                .await
                .with_context(|| format!("sending {}", path.display()))?;
            remaining -= chunk as u64;
        }

        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);

        Ok(())
    }

    async fn send_prepared_tar_shard(
        &mut self,
        headers: Vec<FileHeader>,
        data: &[u8],
    ) -> Result<()> {
        let shard_len = headers.len();
        let preview = headers
            .first()
            .map(|h| h.relative_path.as_str())
            .unwrap_or("<empty>");
        trace_client!(
            self,
            "sending tar shard with {} file(s), {} bytes (first='{}')",
            shard_len,
            data.len(),
            preview
        );
        self.stream
            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
            .await
            .context("writing tar shard record tag")?;
        self.stream
            .write_all(&(headers.len() as u32).to_be_bytes())
            .await
            .context("writing tar shard count")?;

        for header in headers {
            let rel_bytes = header.relative_path.as_bytes();
            if rel_bytes.len() > u32::MAX as usize {
                bail!(
                    "relative path too long for transfer: {}",
                    header.relative_path
                );
            }
            self.stream
                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
                .await
                .context("writing shard path length")?;
            self.stream
                .write_all(rel_bytes)
                .await
                .context("writing shard path bytes")?;
            self.stream
                .write_all(&header.size.to_be_bytes())
                .await
                .context("writing shard size")?;
            self.stream
                .write_all(&header.mtime_seconds.to_be_bytes())
                .await
                .context("writing shard mtime")?;
            self.stream
                .write_all(&header.permissions.to_be_bytes())
                .await
                .context("writing shard permissions")?;
        }

        self.stream
            .write_all(&(data.len() as u64).to_be_bytes())
            .await
            .context("writing tar shard length")?;
        for chunk in data.chunks(self.chunk_bytes.max(1)) {
            self.stream
                .write_all(chunk)
                .await
                .context("writing tar shard payload")?;
        }
        trace_client!(
            self,
            "tar shard payload sent ({} file(s), {} bytes)",
            shard_len,
            data.len()
        );

        Ok(())
    }
}
