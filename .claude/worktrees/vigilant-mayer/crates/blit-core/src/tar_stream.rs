//! Simplified tar streaming for small files
//! Pulled from streaming_batch.rs and simplified for Windows focus

use crossbeam_channel as mpsc;
use eyre::{bail, eyre, Result};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tar::{Archive, Builder};
use walkdir::WalkDir;

/// Events emitted during tar streaming (for optional callbacks)
#[derive(Clone, Debug)]
pub enum TarEvent {
    FileStarted(PathBuf),
    FileFinished(PathBuf, u64),
    BytesWritten(u64),
}

/// Configuration for tar streaming
#[derive(Debug, Clone)]
pub struct TarConfig {
    /// Buffer size for channel (number of chunks)
    pub channel_capacity: usize,
    /// Size of each chunk in bytes
    pub chunk_size: usize,
    /// Optional timeout for send-on-drop to avoid deadlocks (ms). None = block.
    pub send_timeout_ms: Option<u64>,
}

impl Default for TarConfig {
    fn default() -> Self {
        TarConfig {
            channel_capacity: 64,    // 64 chunks in flight
            chunk_size: 1024 * 1024, // 1MB chunks
            send_timeout_ms: Some(30_000),
        }
    }
}

fn sanitize_rel_path(rel: &Path) -> Result<PathBuf> {
    use std::path::Component::*;
    if rel.is_absolute() {
        bail!("refusing absolute tar entry path: {}", rel.display());
    }
    let mut clean = PathBuf::new();
    for comp in rel.components() {
        match comp {
            Normal(s) => clean.push(s),
            CurDir => {}
            ParentDir | RootDir | Prefix(_) => {
                bail!("unsafe component in tar entry path: {}", rel.display());
            }
        }
    }
    Ok(clean)
}

/// Channel writer that sends data through mpsc channel
struct ChannelWriter {
    tx: mpsc::Sender<Vec<u8>>,
    buffer: Vec<u8>,
    chunk_size: usize,
    send_timeout: Option<Duration>,
}

impl ChannelWriter {
    fn new(tx: mpsc::Sender<Vec<u8>>, chunk_size: usize, send_timeout: Option<Duration>) -> Self {
        Self {
            tx,
            buffer: Vec::with_capacity(chunk_size),
            chunk_size,
            send_timeout,
        }
    }

    fn flush_buffer(&mut self) -> io::Result<()> {
        if !self.buffer.is_empty() {
            let chunk = std::mem::replace(&mut self.buffer, Vec::with_capacity(self.chunk_size));
            self.tx
                .send(chunk)
                .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))?;
        }
        Ok(())
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut written = 0;
        let mut remaining = buf;

        while !remaining.is_empty() {
            let available = self.chunk_size - self.buffer.len();
            let to_write = remaining.len().min(available);

            self.buffer.extend_from_slice(&remaining[..to_write]);
            written += to_write;
            remaining = &remaining[to_write..];

            if self.buffer.len() >= self.chunk_size {
                self.flush_buffer()?;
            }
        }

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buffer()
    }
}

// On drop, attempt a blocking (or timed) flush of any buffered data to avoid silent loss.
impl Drop for ChannelWriter {
    fn drop(&mut self) {
        if !self.buffer.is_empty() {
            let chunk = std::mem::take(&mut self.buffer);
            if let Some(d) = self.send_timeout {
                let _ = self.tx.send_timeout(chunk, d);
            } else {
                let _ = self.tx.send(chunk);
            }
        }
    }
}

/// Stream files through tar without intermediate file
pub fn tar_stream_transfer(
    source: &Path,
    dest: &Path,
    config: &TarConfig,
    show_progress: bool,
    _start_offset: u64,
) -> Result<(u64, u64)> {
    // Back-compat wrapper: in-module progress removed; delegate to _cb()
    let _ = show_progress; // retained for signature compatibility
    tar_stream_transfer_cb(
        source,
        dest,
        config,
        Option::<fn(TarEvent)>::None,
        _start_offset,
    )
}

/// Core tar streaming with optional event callback (preferred API)
pub fn tar_stream_transfer_cb(
    source: &Path,
    dest: &Path,
    config: &TarConfig,
    on_event: Option<impl Fn(TarEvent) + Send + Sync + 'static>,
    _start_offset: u64,
) -> Result<(u64, u64)> {
    // Ensure destination exists
    fs::create_dir_all(dest)?;

    // Collect file list in a single traversal (or single file)
    let mut files: Vec<PathBuf> = Vec::new();
    if source.is_file() {
        files.push(source.to_path_buf());
    } else {
        for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }
    }
    let file_count = files.len();

    // Dynamic channel sizing: scale based on file count (cap by configured)
    let dynamic_buffer_size = match file_count {
        0..=100 => 16,
        101..=1000 => 32,
        1001..=10000 => 64,
        _ => 128.min(config.channel_capacity * 2),
    };
    let channel_buffer = dynamic_buffer_size.min(config.channel_capacity);

    // Create channel for streaming with dynamic size
    let (tx, rx) = mpsc::bounded::<Vec<u8>>(channel_buffer);

    let source_path = source.to_path_buf();
    let dest_path = dest.to_path_buf();
    let chunk_size = config.chunk_size;
    let send_timeout = config.send_timeout_ms.map(Duration::from_millis);
    let cb =
        on_event.map(|f| std::sync::Arc::new(f) as std::sync::Arc<dyn Fn(TarEvent) + Send + Sync>);

    // Thread 1: Create tar stream
    let packer = thread::spawn(move || -> Result<(u64, u64)> {
        let mut writer = ChannelWriter::new(tx, chunk_size, send_timeout);
        let mut file_count = 0u64;
        let mut total_bytes = 0u64;

        {
            let mut builder = Builder::new(&mut writer);

            // Add files from pre-collected list
            for path in files.iter() {
                // Compute safe relative path
                let rel_path = if source_path.is_file() {
                    // Single-file source: use filename
                    PathBuf::from(path.file_name().unwrap_or_default())
                } else {
                    let rp = path.strip_prefix(&source_path).map_err(|_| {
                        eyre!("failed to compute relative path for {}", path.display())
                    })?;
                    // Sanitize: disallow absolute and parent components
                    let mut clean = PathBuf::new();
                    for comp in rp.components() {
                        use std::path::Component::*;
                        match comp {
                            Normal(s) => clean.push(s),
                            CurDir => {}
                            ParentDir | RootDir | Prefix(_) => {
                                bail!("unsafe path component in tar entry: {}", rp.display())
                            }
                        }
                    }
                    clean
                };

                if let Ok(metadata) = path.metadata() {
                    total_bytes += metadata.len();
                    file_count += 1;
                    if let Some(cb) = &cb {
                        cb(TarEvent::FileStarted(rel_path.clone()));
                    }
                }

                // Add file to tar
                builder.append_path_with_name(path, rel_path.clone())?;
                if let Some(cb) = &cb {
                    cb(TarEvent::FileFinished(rel_path, 0));
                }
            }

            builder.finish()?;
        }

        writer.flush()?;
        Ok((file_count, total_bytes))
    });

    // Thread 2: Extract tar stream
    let unpacker = thread::spawn(move || -> Result<()> {
        let reader = ChannelReader::new(rx);
        let mut archive = Archive::new(reader);
        // Best effort preserve relevant metadata
        archive.set_unpack_xattrs(false);
        archive.set_preserve_permissions(true);
        archive.unpack(&dest_path)?;
        Ok(())
    });

    // Wait for both threads
    let (file_count, total_bytes) = packer
        .join()
        .map_err(|_| eyre!("Packer thread panicked"))??;

    unpacker
        .join()
        .map_err(|_| eyre!("Unpacker thread panicked"))??;

    Ok((file_count, total_bytes))
}

/// Stream an explicit list of files (src path + tar path) through tar without staging
pub fn tar_stream_transfer_list(
    files: &[(PathBuf, PathBuf)],
    dest: &Path,
    config: &TarConfig,
    show_progress: bool,
) -> Result<(u64, u64)> {
    let _ = show_progress; // retained for signature compatibility
    tar_stream_transfer_list_cb(files, dest, config, Option::<fn(TarEvent)>::None)
}

/// Core explicit-list streaming with optional event callback (preferred API)
pub fn tar_stream_transfer_list_cb(
    files: &[(PathBuf, PathBuf)],
    dest: &Path,
    config: &TarConfig,
    on_event: Option<impl Fn(TarEvent) + Send + Sync + 'static>,
) -> Result<(u64, u64)> {
    // Ensure destination exists
    fs::create_dir_all(dest)?;

    // Dynamic channel sizing based on explicit file list
    let file_count = files.len();
    let dynamic_buffer_size = match file_count {
        0..=100 => 16,
        101..=1000 => 32,
        1001..=10000 => 64,
        _ => 128.min(config.channel_capacity * 2),
    };
    let channel_buffer = dynamic_buffer_size.min(config.channel_capacity);

    // Create channel for streaming with dynamic size
    let (tx, rx) = mpsc::bounded::<Vec<u8>>(channel_buffer);

    let files_list = files.to_owned();
    let dest_path = dest.to_path_buf();
    let chunk_size = config.chunk_size;
    let send_timeout = config.send_timeout_ms.map(Duration::from_millis);
    let cb =
        on_event.map(|f| std::sync::Arc::new(f) as std::sync::Arc<dyn Fn(TarEvent) + Send + Sync>);

    // Thread 1: Create tar stream for explicit list
    let packer = thread::spawn(move || -> Result<(u64, u64)> {
        let mut writer = ChannelWriter::new(tx, chunk_size, send_timeout);
        let mut file_count = 0u64;
        let mut total_bytes = 0u64;

        {
            let mut builder = Builder::new(&mut writer);

            for (src_path, tar_rel_path) in files_list.iter() {
                if let Ok(metadata) = src_path.metadata() {
                    total_bytes += metadata.len();
                    file_count += 1;
                    if let Some(cb) = &cb {
                        cb(TarEvent::FileStarted(tar_rel_path.clone()));
                    }
                }

                let clean = sanitize_rel_path(tar_rel_path)?;
                builder.append_path_with_name(src_path, clean.clone())?;
                if let Some(cb) = &cb {
                    cb(TarEvent::FileFinished(clean, 0));
                }
            }

            builder.finish()?;
        }

        writer.flush()?;
        Ok((file_count, total_bytes))
    });

    // Thread 2: Extract tar stream
    let unpacker = thread::spawn(move || -> Result<()> {
        let reader = ChannelReader::new(rx);
        let mut archive = Archive::new(reader);
        archive.set_unpack_xattrs(false);
        archive.set_preserve_permissions(true);
        archive.unpack(&dest_path)?;
        Ok(())
    });

    // Wait for both threads
    let (file_count, total_bytes) = packer
        .join()
        .map_err(|_| eyre!("Packer thread panicked"))??;

    unpacker
        .join()
        .map_err(|_| eyre!("Unpacker thread panicked"))??;

    Ok((file_count, total_bytes))
}

/// Channel reader that receives data from mpsc channel
struct ChannelReader {
    rx: mpsc::Receiver<Vec<u8>>,
    buffer: Vec<u8>,
    buffer_pos: usize,
}

impl ChannelReader {
    fn new(rx: mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            rx,
            buffer: Vec::new(),
            buffer_pos: 0,
        }
    }
}

impl Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If we have data in our buffer, use it first
        if self.buffer_pos < self.buffer.len() {
            let available = self.buffer.len() - self.buffer_pos;
            let to_copy = available.min(buf.len());
            buf[..to_copy]
                .copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + to_copy]);
            self.buffer_pos += to_copy;
            return Ok(to_copy);
        }

        // Buffer is empty, get new chunk from channel
        match self.rx.recv() {
            Ok(chunk) => {
                if chunk.is_empty() {
                    return Ok(0);
                }

                self.buffer = chunk;
                self.buffer_pos = 0;

                // Now copy from the new buffer
                let to_copy = self.buffer.len().min(buf.len());
                buf[..to_copy].copy_from_slice(&self.buffer[..to_copy]);
                self.buffer_pos = to_copy;
                Ok(to_copy)
            }
            Err(_) => Ok(0), // Channel closed, EOF
        }
    }
}
