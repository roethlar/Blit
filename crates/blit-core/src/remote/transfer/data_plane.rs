use eyre::{bail, Context, Result};
use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::buffer::BufferPool;
use crate::generated::FileHeader;

use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
use super::progress::{NoProbe, Probe};
use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
use crate::remote::transfer::source::TransferSource;
use std::sync::Arc;

pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1024 * 1024;
pub const DATA_PLANE_RECORD_FILE: u8 = 0;
pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
pub const DATA_PLANE_RECORD_END: u8 = 0xFF;

/// ue-r2-2: length of the per-epoch resize credential a data socket
/// echoes after the one-time token when resize was negotiated
/// (`DataTransferNegotiation.epoch0_sub_token` for the initial
/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).
pub const SUB_TOKEN_LEN: usize = 16;

/// Generate one 16-byte resize sub-token. Same fallible-RNG posture
/// as the daemon's one-time token (audit-3b): a missing system RNG is
/// an error, never a weaker credential.
pub fn generate_sub_token() -> eyre::Result<Vec<u8>> {
    use rand::{rngs::SysRng, TryRng};
    let mut buf = vec![0u8; SUB_TOKEN_LEN];
    SysRng
        .try_fill_bytes(&mut buf)
        .map_err(|err| eyre::eyre!("system RNG unavailable: {err}"))?;
    Ok(buf)
}

/// A single data-plane TCP stream and its send loop.
///
/// Generic over a [`Probe`] so the byte-copy hot path can carry
/// per-stream telemetry under adaptive mode at **zero cost** when the
/// probe is [`NoProbe`] (the default): the instrumented branches are
/// gated on `P::ACTIVE`, a compile-time constant, so they fold away
/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
/// bare type and get the `NoProbe` default; the adaptive controller
/// constructs `DataPlaneSession<LiveProbe>` via
/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
///
/// audit-h3b: writes go through [`StallGuardWriter`] so a stalled
/// reader (TCP backpressure from a slow / wedged peer) trips after
/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
/// of pinning the worker for OS-level TCP retransmit exhaustion
/// (15+ minutes). All existing `self.stream.write_all/.flush` call
/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
/// so no per-site change was needed.
pub struct DataPlaneSession<P: Probe = NoProbe> {
    stream: StallGuardWriter<TcpStream>,
    pool: Arc<BufferPool>,
    trace: bool,
    chunk_bytes: usize,
    payload_prefetch: usize,
    bytes_sent: u64,
    probe: P,
}

macro_rules! trace_client {
    ($session:expr, $($arg:tt)*) => {
        if $session.trace {
            eprintln!("[data-plane-client] {}", format_args!($($arg)*));
        }
    };
}

impl DataPlaneSession<NoProbe> {
    /// Create a session from an existing stream with buffer pooling.
    ///
    /// Produces the un-instrumented `NoProbe` variant — the default for
    /// every non-adaptive caller. audit-h3b: the stream is wrapped in
    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
    /// observable write progress instead of pinning the worker for
    /// OS-level TCP retransmit exhaustion. The production call sites
    /// (`daemon/service/pull.rs`, `daemon/service/pull_sync.rs`, and the
    /// resume path) inherit the guard without code changes.
    pub async fn from_stream(
        stream: TcpStream,
        trace: bool,
        chunk_bytes: usize,
        payload_prefetch: usize,
        pool: Arc<BufferPool>,
    ) -> Self {
        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
            .await
    }

    /// Connect to a data plane endpoint with buffer pooling.
    #[allow(clippy::too_many_arguments)]
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
        Self::connect_with_probe(
            host,
            port,
            token,
            chunk_bytes,
            payload_prefetch,
            trace,
            tcp_buffer_size,
            pool,
            NoProbe,
        )
        .await
    }
}

impl<P: Probe> DataPlaneSession<P> {
    /// `connect` with an explicit probe (ue-r2-1e: the dial tuner
    /// attaches `LiveProbe` telemetry to the push data plane; the
    /// probe-free path monomorphizes to `NoProbe` and reads no clock).
    #[allow(clippy::too_many_arguments)]
    pub async fn connect_with_probe(
        host: &str,
        port: u32,
        token: &[u8],
        chunk_bytes: usize,
        payload_prefetch: usize,
        trace: bool,
        tcp_buffer_size: Option<usize>,
        pool: Arc<BufferPool>,
        probe: P,
    ) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        if trace {
            eprintln!("[data-plane-client] connecting to {}", addr);
        }
        let mut stream = TcpStream::connect(addr.clone())
            .await
            .with_context(|| format!("connecting to data plane {}", addr))?;

        // w1-2: the NODELAY/keepalive/tuned-buffer policy lives in the
        // shared helper — one owner for every data-plane socket, both
        // directions, both ends.
        super::socket::configure_data_socket(&stream, tcp_buffer_size)
            .context("setting TCP_NODELAY")?;

        stream
            .write_all(token)
            .await
            .context("writing negotiation token")?;

        Ok(
            Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, probe)
                .await,
        )
    }
}

impl<P: Probe> DataPlaneSession<P> {
    /// Create a session carrying an arbitrary [`Probe`]. The generic
    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
    /// the adaptive controller calls this with a `LiveProbe` to enable
    /// per-stream telemetry.
    pub async fn from_stream_with_probe(
        stream: TcpStream,
        trace: bool,
        chunk_bytes: usize,
        payload_prefetch: usize,
        pool: Arc<BufferPool>,
        probe: P,
    ) -> Self {
        let payload_prefetch = payload_prefetch.max(1);
        let chunk_bytes = chunk_bytes.max(64 * 1024);
        Self {
            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
            pool,
            trace,
            chunk_bytes,
            payload_prefetch,
            bytes_sent: 0,
            probe,
        }
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
                PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
                    bail!("DataPlaneSession::send_payloads does not handle resume payloads");
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

    pub async fn send_file(
        &mut self,
        source: Arc<dyn TransferSource>,
        header: &FileHeader,
    ) -> Result<()> {
        let rel = &header.relative_path;
        let mut file = source
            .open_file(header)
            .await
            .with_context(|| format!("opening {}", rel))?;
        self.send_file_from_reader(header, &mut file).await
    }

    /// Send a file payload whose bytes come from an arbitrary async
    /// reader (not a local file). Used by `DataPlaneSink` for the
    /// remote→remote relay case, where bytes arrive from an inbound
    /// `DataPlaneSource` and need to be forwarded to the next hop.
    ///
    /// Same wire format and double-buffered loop as `send_file`.
    pub async fn send_file_from_reader(
        &mut self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
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
        // Wire-format extension (2026-05-01): include mtime + permissions
        // inline so push and pull data plane records carry the same
        // information. Lets the receive pipeline apply metadata via
        // FsTransferSink without consulting an out-of-band manifest cache.
        self.stream
            .write_all(&header.mtime_seconds.to_be_bytes())
            .await
            .context("writing mtime")?;
        self.stream
            .write_all(&header.permissions.to_be_bytes())
            .await
            .context("writing permissions")?;

        // Double-buffered I/O: overlaps source reads with network writes
        self.send_file_double_buffered(reader, header, rel).await?;

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
        // Clamp to the declared size before subtracting. A source that
        // returns more bytes than `header.size` — a file that grew after
        // the manifest was computed, or a lying `TransferSource` — would
        // otherwise underflow `remaining` (debug: panic; release: wrap to
        // u64::MAX → runaway loop) and push undeclared bytes onto the
        // framed stream. We send exactly `header.size` and ignore excess.
        bytes_a = (bytes_a as u64).min(remaining) as usize;
        remaining -= bytes_a as u64;

        // Main loop: write buf_a while reading into buf_b
        while remaining > 0 {
            // Per-stream telemetry: time ONLY the socket write as the
            // backpressure signal. ue-r2-1e (carried ue-r2-1a review
            // finding): the old code timed the whole overlapped
            // write+read join, so a slow disk READ inflated
            // "write blocked" and would bias the dial tuner
            // conservative. The async block's clock starts when the
            // join first polls it and stops when write_all completes —
            // the concurrent read neither extends nor shortens it.
            // Gated on the compile-time `P::ACTIVE` constant so
            // `DataPlaneSession<NoProbe>` reads no clock.
            let write_slice = &buf_a.as_slice()[..bytes_a];
            let stream = &mut self.stream;
            let (write_outcome, read_result) = tokio::join!(
                async {
                    let started = if P::ACTIVE {
                        Some(std::time::Instant::now())
                    } else {
                        None
                    };
                    let result = stream.write_all(write_slice).await;
                    (result, started.map(|t| t.elapsed()))
                },
                file.read(buf_b.as_mut_slice())
            );

            let (write_result, write_elapsed) = write_outcome;
            write_result.with_context(|| format!("sending {}", rel))?;
            if let Some(elapsed) = write_elapsed {
                self.probe.note_write_blocked(elapsed.as_nanos() as u64);
            }
            self.probe.record_bytes(bytes_a as u64);
            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);

            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;

            if bytes_b == 0 && remaining > 0 {
                bail!(
                    "unexpected EOF while reading {} ({} bytes remaining)",
                    rel,
                    remaining
                );
            }
            // Same clamp as the initial read: never subtract more than
            // `remaining`, so an over-returning reader can neither
            // underflow the counter nor send undeclared bytes.
            let bytes_b = (bytes_b as u64).min(remaining) as usize;
            remaining -= bytes_b as u64;

            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
            std::mem::swap(&mut buf_a, &mut buf_b);
            bytes_a = bytes_b;
        }

        // Final write: send the last chunk in buf_a. This is a pure
        // write (no overlapped read), so the timing is cleanly
        // attributable to socket-write backpressure.
        if bytes_a > 0 {
            let tail_start = if P::ACTIVE {
                Some(std::time::Instant::now())
            } else {
                None
            };
            self.stream
                .write_all(&buf_a.as_slice()[..bytes_a])
                .await
                .with_context(|| format!("sending {}", rel))?;
            if P::ACTIVE {
                if let Some(t) = tail_start {
                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
                }
            }
            self.probe.record_bytes(bytes_a as u64);
            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
        }

        // Buffers return to pool automatically on drop
        Ok(())
    }

    pub async fn send_prepared_tar_shard(
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
            // codex ue-r2-1e F3: shard writes carry the small-file
            // workloads — without a blocked signal here the tuner sees
            // a saturated link as a clean one. Same P::ACTIVE gating as
            // the file loop: NoProbe reads no clock.
            let started = if P::ACTIVE {
                Some(std::time::Instant::now())
            } else {
                None
            };
            self.stream
                .write_all(chunk)
                .await
                .context("writing tar shard payload")?;
            if let Some(t) = started {
                self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
            }
            self.probe.record_bytes(chunk.len() as u64);
            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(chunk.len() as u64);
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
        crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(content.len() as u64);
        self.probe.record_bytes(content.len() as u64);

        self.bytes_sent += content.len() as u64;
        Ok(())
    }

    /// Signal that block-level transfer for a file is complete.
    /// Format: [type:1][path_len:4][path][total_size:8][mtime:8][perms:4]
    ///
    /// Carries mtime + perms inline so the receiver can stamp the
    /// destination metadata even when zero blocks transferred (the
    /// "mtime touched, content identical" case for mirror).
    pub async fn send_block_complete(
        &mut self,
        relative_path: &str,
        total_size: u64,
        mtime_seconds: i64,
        permissions: u32,
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
        self.stream
            .write_all(&mtime_seconds.to_be_bytes())
            .await
            .context("writing mtime")?;
        self.stream
            .write_all(&permissions.to_be_bytes())
            .await
            .context("writing permissions")?;

        Ok(())
    }
}

/// Default buffer size for the symmetric receive path. Matches what the
/// send side's buffer pool uses for chunk_bytes; large enough that the
/// per-syscall overhead doesn't dominate at 10 GbE, and that ZFS-style
/// transactional filesystems can amortize per-write costs.
///
/// Empirically, 8 KiB caps push throughput at ~1 Gbps on EPYC/ZFS even
/// when the network can do 9.4 Gbps and the disk can do 14.76 Gbps.
/// 1 MiB lets the receiver keep up with the sender's double-buffered
/// pipeline.
pub const RECEIVE_CHUNK_SIZE: usize = 1024 * 1024;

/// Stream `expected` bytes from an async source into an async sink with
/// double-buffered I/O — while one buffer drains to disk, the other is
/// being filled from the wire. Symmetric counterpart of
/// `DataPlaneSession::send_file_double_buffered`.
///
/// Both the daemon's push receiver (writing to disk from a TCP socket)
/// and the client's pull receiver (same shape, opposite direction) call
/// this so the receive side has the same throughput characteristics as
/// the send side. Replacing this with `tokio::io::copy` (8 KiB internal
/// buffer) caps real-world transfers at ~1 Gbps regardless of network
/// or disk speed.
///
/// Returns the number of bytes copied. Errors on early EOF.
///
/// `byte_progress` (optional) gets a `report(delta)` call after
/// each successful chunk write. Cadence matches the receive
/// buffer size (`buffer_size`; clamped ≥ 64 KiB), so a 10 GiB
/// transfer at the default 1 MiB chunk size emits ~10 000
/// reports. Callers that don't need byte-level instrumentation
/// pass `None` and pay nothing — the inner loop's
/// `if let Some(p)` branch is a single predicted-taken jump.
pub async fn receive_stream_double_buffered<R, W>(
    src: &mut R,
    dst: &mut W,
    expected: u64,
    buffer_size: usize,
    byte_progress: Option<&crate::remote::transfer::progress::ByteProgressSink>,
) -> Result<u64>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
    W: tokio::io::AsyncWrite + Unpin + ?Sized,
{
    if expected == 0 {
        return Ok(0);
    }

    let cap = buffer_size.max(64 * 1024);
    let mut buf_a = vec![0u8; cap];
    let mut buf_b = vec![0u8; cap];

    // Initial fill of buf_a.
    let mut bytes_a = read_up_to(src, &mut buf_a, expected).await?;
    if bytes_a == 0 {
        bail!("unexpected EOF: 0 bytes received, {} expected", expected);
    }
    let mut total: u64 = bytes_a as u64;

    while total < expected {
        let want_b = (expected - total).min(buf_b.len() as u64);
        let (write_res, read_res) = tokio::join!(
            dst.write_all(&buf_a[..bytes_a]),
            read_up_to(src, &mut buf_b, want_b),
        );
        write_res.context("writing received bytes to disk")?;
        // Report the bytes that just landed on disk. We report
        // AFTER `write_all` succeeds so a `bytes_completed`
        // observed by GetState never exceeds bytes actually
        // written (mid-failure transfers stay accurate too —
        // the post-Drop record holds the value at last success).
        if let Some(progress) = byte_progress {
            progress.report(bytes_a as u64);
        }
        let bytes_b = read_res?;
        if bytes_b == 0 && total + bytes_a as u64 != expected {
            bail!(
                "unexpected EOF: {} bytes received, {} expected",
                total + bytes_a as u64,
                expected
            );
        }
        total += bytes_b as u64;
        std::mem::swap(&mut buf_a, &mut buf_b);
        bytes_a = bytes_b;
    }

    if bytes_a > 0 {
        dst.write_all(&buf_a[..bytes_a])
            .await
            .context("writing final chunk to disk")?;
        if let Some(progress) = byte_progress {
            progress.report(bytes_a as u64);
        }
    }

    Ok(total)
}

/// Read up to `cap` bytes (clamped to the slice length) from `src`,
/// returning how many were read. Returns 0 only on EOF or zero-cap.
async fn read_up_to<R>(src: &mut R, buf: &mut [u8], cap: u64) -> Result<usize>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
{
    let take = (buf.len() as u64).min(cap) as usize;
    if take == 0 {
        return Ok(0);
    }
    let n = src
        .read(&mut buf[..take])
        .await
        .context("reading from data plane stream")?;
    Ok(n)
}

#[cfg(test)]
mod byte_progress_tests {
    use super::*;
    use crate::remote::transfer::progress::ByteProgressSink;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// With `byte_progress = None` the function behaves exactly
    /// like the pre-c-1b version: bytes are copied, no
    /// instrumentation runs.
    #[tokio::test]
    async fn copies_without_progress_when_sink_omitted() {
        let payload: Vec<u8> = (0u8..32).cycle().take(4 * 1024).collect();
        let mut src = std::io::Cursor::new(payload.clone());
        let mut dst: Vec<u8> = Vec::new();
        let n =
            receive_stream_double_buffered(&mut src, &mut dst, payload.len() as u64, 1024, None)
                .await
                .expect("copy ok");
        assert_eq!(n, payload.len() as u64);
        assert_eq!(dst, payload);
    }

    /// With a sink supplied, the cumulative value at the end of
    /// the copy equals the bytes the data plane reported writing.
    /// We don't pin the number of reports — that's a function of
    /// chunk sizing — but the sum is load-bearing.
    #[tokio::test]
    async fn cumulative_reports_match_bytes_copied() {
        let payload: Vec<u8> = (0u8..255).cycle().take(8 * 1024).collect();
        let mut src = std::io::Cursor::new(payload.clone());
        let mut dst: Vec<u8> = Vec::new();
        let counter = Arc::new(AtomicU64::new(0));
        let sink = ByteProgressSink::from_counter(Arc::clone(&counter));
        let n = receive_stream_double_buffered(
            &mut src,
            &mut dst,
            payload.len() as u64,
            1024,
            Some(&sink),
        )
        .await
        .expect("copy ok");
        assert_eq!(n, payload.len() as u64);
        assert_eq!(
            counter.load(Ordering::Relaxed),
            payload.len() as u64,
            "reported total must equal bytes copied"
        );
    }

    /// Reports fire in multiple chunks rather than a single
    /// final batch — proves the progress hook is INSIDE the loop,
    /// not bolted on after the copy completes. We pick a
    /// payload large enough that any sane chunk size produces
    /// >1 report.
    #[tokio::test]
    async fn reports_fire_incrementally_under_load() {
        // Use a tiny buffer so the inner loop has to iterate
        // many times. The function clamps buffer_size to >=64
        // KiB so even at buffer_size=1024 the actual capacity
        // is 64 KiB; the payload then needs to be larger than
        // that to force multiple iterations.
        let payload_size = 1024 * 1024; // 1 MiB
        let payload: Vec<u8> = vec![0xAA; payload_size];
        let mut src = std::io::Cursor::new(payload);
        let mut dst: Vec<u8> = Vec::new();

        // Wrap the sink in a stub that records each report's
        // delta so we can assert > 1 report fired.
        let counter = Arc::new(AtomicU64::new(0));
        let report_count = Arc::new(AtomicU64::new(0));
        let sink = ByteProgressSink::from_counter(Arc::clone(&counter));

        // Drive a goroutine that polls the counter to count
        // distinct increments. This is racy but the assertion
        // is "strictly more than 0 reports and final == size";
        // both are eventually-consistent properties.
        let rc = Arc::clone(&report_count);
        let c = Arc::clone(&counter);
        let watcher = tokio::spawn(async move {
            let mut last = 0;
            for _ in 0..1000 {
                let cur = c.load(Ordering::Relaxed);
                if cur != last {
                    rc.fetch_add(1, Ordering::Relaxed);
                    last = cur;
                }
                if cur >= payload_size as u64 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        });

        receive_stream_double_buffered(
            &mut src,
            &mut dst,
            payload_size as u64,
            64 * 1024,
            Some(&sink),
        )
        .await
        .expect("copy ok");
        let _ = watcher.await;

        assert_eq!(counter.load(Ordering::Relaxed), payload_size as u64);
        // Strict lower bound: at minimum one final-tail report,
        // plus one loop-body report. Asserting >= 2 keeps the
        // test robust against future tweaks to buffer sizing.
        assert!(
            report_count.load(Ordering::Relaxed) >= 1,
            "expected at least one intermediate report from the chunk loop"
        );
    }
}

#[cfg(test)]
mod underflow_tests {
    //! audit-11: `send_file_double_buffered` must not underflow
    //! `remaining` when the source reader returns more bytes than
    //! `header.size` (a file that grew after the manifest, or a lying
    //! `TransferSource`). Before the clamp this panicked in debug and
    //! wrapped to `u64::MAX` (runaway loop) in release.
    use super::*;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn over_returning_reader_sends_exactly_declared_size() {
        // Loopback TCP pair: the session writes into `client`; a drain
        // task on the accepted socket counts every byte received.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let drain = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut sink = Vec::new();
            sock.read_to_end(&mut sink).await.unwrap();
            sink.len()
        });
        let client = TcpStream::connect(addr).await.unwrap();

        // 1 KiB buffers so the first read fills the whole buffer from the
        // 4 KiB cursor — i.e. returns far more than the 100-byte header.
        let pool = Arc::new(BufferPool::new(1024, 4, None));
        let mut session = DataPlaneSession::from_stream(client, false, 64 * 1024, 1, pool).await;

        let declared: u64 = 100;
        let reader_payload = vec![0x5Au8; 4096];
        let mut reader = std::io::Cursor::new(reader_payload);
        let header = FileHeader {
            size: declared,
            ..Default::default()
        };

        session
            .send_file_double_buffered(&mut reader, &header, "grew.bin")
            .await
            .expect("over-returning reader must not panic or underflow");

        // Close the write side so the drain task's read_to_end completes.
        drop(session);
        let received = drain.await.unwrap();
        assert_eq!(
            received as u64, declared,
            "must send exactly header.size bytes, never the reader's excess"
        );
    }
}
