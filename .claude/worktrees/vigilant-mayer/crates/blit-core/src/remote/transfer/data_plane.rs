use eyre::{bail, Context, Result};
use futures::StreamExt;
use socket2::Socket;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::buffer::BufferPool;
use crate::generated::FileHeader;

use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
use crate::remote::transfer::source::TransferSource;
use std::sync::Arc;

pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1 * 1024 * 1024;
pub const DATA_PLANE_RECORD_FILE: u8 = 0;
pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
pub const DATA_PLANE_RECORD_END: u8 = 0xFF;

pub struct DataPlaneSession {
    stream: TcpStream,
    pool: Arc<BufferPool>,
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
    /// Create a session from an existing stream with buffer pooling.
    pub async fn from_stream(
        stream: TcpStream,
        trace: bool,
        chunk_bytes: usize,
        payload_prefetch: usize,
        pool: Arc<BufferPool>,
    ) -> Self {
        let payload_prefetch = payload_prefetch.max(1);
        let chunk_bytes = chunk_bytes.max(64 * 1024);
        Self {
            stream,
            pool,
            trace,
            chunk_bytes,
            payload_prefetch,
            bytes_sent: 0,
        }
    }

    /// Connect to a data plane endpoint with buffer pooling.
    pub async fn connect(
        host: &str,
        port: u32,
        token: &[u8],
        chunk_bytes: usize,
        payload_prefetch: usize,
        trace: bool,
        tcp_buffer_size: Option<usize>,
        pool: Arc<BufferPool>,
    ) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        if trace {
            eprintln!("[data-plane-client] connecting to {}", addr);
        }
        let stream = TcpStream::connect(addr.clone())
            .await
            .with_context(|| format!("connecting to data plane {}", addr))?;

        let std_stream = stream.into_std().context("converting to std stream")?;
        let socket = Socket::from(std_stream);
        socket
            .set_tcp_nodelay(true)
            .context("setting TCP_NODELAY")?;

        if let Some(size) = tcp_buffer_size {
            let _ = socket.set_send_buffer_size(size);
            let _ = socket.set_recv_buffer_size(size);
        }

        let std_stream: std::net::TcpStream = socket.into();
        let mut stream =
            TcpStream::from_std(std_stream).context("converting back to tokio stream")?;

        stream
            .write_all(token)
            .await
            .context("writing negotiation token")?;

        Ok(Self::from_stream(stream, trace, chunk_bytes, payload_prefetch, pool).await)
    }

    pub async fn send_payloads(
        &mut self,
        source: Arc<dyn TransferSource>,
        payloads: Vec<TransferPayload>,
    ) -> Result<()> {
        self.send_payloads_with_progress(source, payloads, None)
            .await
    }

    pub async fn send_payloads_with_progress(
        &mut self,
        source: Arc<dyn TransferSource>,
        payloads: Vec<TransferPayload>,
        progress: Option<&super::progress::RemoteTransferProgress>,
    ) -> Result<()> {
        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
        while let Some(prepared) = stream.next().await {
            match prepared? {
                PreparedPayload::File(header) => {
                    if let Err(err) = self.send_file(source.clone(), &header).await {
                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
                    }
                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
                    if let Some(progress) = progress {
                        progress.report_file_complete(header.relative_path.clone(), header.size);
                    }
                }
                PreparedPayload::TarShard { headers, data } => {
                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
                        return Err(err.wrap_err("sending tar shard"));
                    }
                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
                    if let Some(progress) = progress {
                        for header in &headers {
                            progress
                                .report_file_complete(header.relative_path.clone(), header.size);
                        }
                    }
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

    async fn send_file(
        &mut self,
        source: Arc<dyn TransferSource>,
        header: &FileHeader,
    ) -> Result<()> {
        let rel = &header.relative_path;
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

        self.stream
            .write_all(&header.size.to_be_bytes())
            .await
            .context("writing file size")?;

        let mut file = source
            .open_file(header)
            .await
            .with_context(|| format!("opening {}", rel))?;

        // Double-buffered I/O: overlaps disk reads with network writes
        self.send_file_double_buffered(&mut file, header, rel)
            .await?;

        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);

        Ok(())
    }

    /// Double-buffered file sending: overlaps disk reads with network writes.
    /// Uses two buffers from the pool to enable concurrent I/O operations.
    ///
    /// Pattern: While buffer A is being written to network, buffer B is filled from disk.
    /// This hides disk latency behind network latency for improved throughput.
    async fn send_file_double_buffered(
        &mut self,
        file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
        header: &FileHeader,
        rel: &str,
    ) -> Result<()> {
        let mut remaining = header.size;
        if remaining == 0 {
            return Ok(());
        }

        // Acquire two buffers for double-buffering
        let mut buf_a = self.pool.acquire().await;
        let mut buf_b = self.pool.acquire().await;

        // Initial read into buf_a
        let mut bytes_a = file
            .read(buf_a.as_mut_slice())
            .await
            .with_context(|| format!("reading {}", rel))?;

        if bytes_a == 0 {
            bail!(
                "unexpected EOF while reading {} ({} bytes remaining)",
                rel,
                remaining
            );
        }
        remaining -= bytes_a as u64;

        // Main loop: write buf_a while reading into buf_b
        while remaining > 0 {
            // Overlap: write from buf_a, read into buf_b concurrently
            let (write_result, read_result) = tokio::join!(
                self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
                file.read(buf_b.as_mut_slice())
            );

            write_result.with_context(|| format!("sending {}", rel))?;

            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;

            if bytes_b == 0 && remaining > 0 {
                bail!(
                    "unexpected EOF while reading {} ({} bytes remaining)",
                    rel,
                    remaining
                );
            }
            remaining -= bytes_b as u64;

            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
            std::mem::swap(&mut buf_a, &mut buf_b);
            bytes_a = bytes_b;
        }

        // Final write: send the last chunk in buf_a
        if bytes_a > 0 {
            self.stream
                .write_all(&buf_a.as_slice()[..bytes_a])
                .await
                .with_context(|| format!("sending {}", rel))?;
        }

        // Buffers return to pool automatically on drop
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

    /// Send a single block for block-level resume.
    /// Format: [type:1][path_len:4][path][offset:8][block_len:4][content]
    pub async fn send_block(
        &mut self,
        relative_path: &str,
        offset: u64,
        content: &[u8],
    ) -> Result<()> {
        let path_bytes = relative_path.as_bytes();
        if path_bytes.len() > u32::MAX as usize {
            bail!("relative path too long for transfer: {}", relative_path);
        }

        trace_client!(
            self,
            "sending block for '{}' at offset {} ({} bytes)",
            relative_path,
            offset,
            content.len()
        );

        self.stream
            .write_all(&[DATA_PLANE_RECORD_BLOCK])
            .await
            .context("writing block record tag")?;
        self.stream
            .write_all(&(path_bytes.len() as u32).to_be_bytes())
            .await
            .context("writing path length")?;
        self.stream
            .write_all(path_bytes)
            .await
            .context("writing path bytes")?;
        self.stream
            .write_all(&offset.to_be_bytes())
            .await
            .context("writing block offset")?;
        self.stream
            .write_all(&(content.len() as u32).to_be_bytes())
            .await
            .context("writing block length")?;
        self.stream
            .write_all(content)
            .await
            .context("writing block content")?;

        self.bytes_sent += content.len() as u64;
        Ok(())
    }

    /// Signal that block-level transfer for a file is complete.
    /// Format: [type:1][path_len:4][path][total_size:8]
    pub async fn send_block_complete(
        &mut self,
        relative_path: &str,
        total_size: u64,
    ) -> Result<()> {
        let path_bytes = relative_path.as_bytes();
        if path_bytes.len() > u32::MAX as usize {
            bail!("relative path too long for transfer: {}", relative_path);
        }

        trace_client!(
            self,
            "sending block complete for '{}' ({} bytes total)",
            relative_path,
            total_size
        );

        self.stream
            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
            .await
            .context("writing block complete record tag")?;
        self.stream
            .write_all(&(path_bytes.len() as u32).to_be_bytes())
            .await
            .context("writing path length")?;
        self.stream
            .write_all(path_bytes)
            .await
            .context("writing path bytes")?;
        self.stream
            .write_all(&total_size.to_be_bytes())
            .await
            .context("writing total size")?;

        Ok(())
    }
}
