use std::path::Path;

use eyre::{bail, Context, Result};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task;

use crate::generated::FileHeader;

use super::payload::{build_tar_shard, TransferPayload};

pub(crate) const CONTROL_PLANE_CHUNK_SIZE: usize = 1 * 1024 * 1024;
pub(crate) const DATA_PLANE_RECORD_FILE: u8 = 0;
pub(crate) const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
pub(crate) const DATA_PLANE_RECORD_END: u8 = 0xFF;

pub(crate) struct DataPlaneSession {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl DataPlaneSession {
    pub(crate) async fn connect(host: &str, port: u32, token: &[u8]) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(addr.clone())
            .await
            .with_context(|| format!("connecting to data plane {}", addr))?;

        stream
            .write_all(token)
            .await
            .context("writing negotiation token")?;

        Ok(Self {
            stream,
            buffer: vec![0u8; 64 * 1024],
        })
    }

    pub(crate) async fn send_payloads(
        &mut self,
        source_root: &Path,
        payloads: Vec<TransferPayload>,
    ) -> Result<()> {
        for payload in payloads {
            match payload {
                TransferPayload::File(header) => {
                    self.send_file(source_root, &header).await?;
                }
                TransferPayload::TarShard { headers } => {
                    self.send_tar_shard(source_root, headers).await?;
                }
            }
        }

        Ok(())
    }

    pub(crate) async fn finish(&mut self) -> Result<()> {
        self.stream
            .write_all(&[DATA_PLANE_RECORD_END])
            .await
            .context("writing transfer terminator")?;
        self.stream
            .flush()
            .await
            .context("flushing data plane stream")
    }

    async fn send_file(&mut self, source_root: &Path, header: &FileHeader) -> Result<()> {
        let rel = &header.relative_path;
        let path = source_root.join(rel);

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

        Ok(())
    }

    async fn send_tar_shard(&mut self, source_root: &Path, headers: Vec<FileHeader>) -> Result<()> {
        let source_root = source_root.to_path_buf();
        let header_clone = headers.clone();
        let data = task::spawn_blocking(move || build_tar_shard(&source_root, &header_clone))
            .await
            .map_err(|err| eyre::eyre!("tar shard worker failed: {err}"))??;

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
        self.stream
            .write_all(&data)
            .await
            .context("writing tar shard payload")?;

        Ok(())
    }
}
