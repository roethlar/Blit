//! h2-small-frames (2026-11-20): a stream of small messages reaches a
//! receiver that takes them later than they arrive.
//!
//! The HTTP/2 library closes a connection once the small DATA frames it
//! holds unread have used up a budget, and derives that budget from the
//! connection's window. Blit's streams are small frames throughout (a file
//! header, a manifest entry, a `find` row), and with the libraries' own
//! windows the limit was met in ordinary use: a push of 200,000 empty
//! files failed after 43 ms, and a listing read through a pager ended part
//! way with "h2 protocol error". The policy that prevents it is
//! `blit_core::remote::grpc_server::HTTP2_CONNECTION_WINDOW`, applied by
//! the server builder and by `blit_core::client::endpoint`.
//!
//! Each guard here holds one of the three places a connection is made to
//! that policy, with a stand-in daemon that makes every frame small on
//! purpose — whether a real tree does depends on how fast each side runs,
//! and a guard must not. All three run on every OS.

use std::io::Read as _;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod common;
use common::spawn_fake_blit_server;

/// Small frames sent in one go. Unread, they cost the budget of a
/// connection with the libraries' own windows several times over (a
/// client's allows about 10,000 of them, a server's about 2,000), and fit
/// either stream window many times, so nothing but that budget stops them.
const ROWS: usize = 40_000;

#[derive(Default)]
struct Progress {
    /// The stand-in has handed every `find` row to its stream, or the
    /// stream has gone.
    rows_sent: AtomicBool,
    /// The stand-in may start reading the frames a `transfer` sent it.
    read_now: AtomicBool,
    frames_read: AtomicUsize,
}

fn wait_for(what: &str, flag: &AtomicBool) {
    let asked = Instant::now();
    while !flag.load(Ordering::SeqCst) {
        assert!(asked.elapsed() < Duration::from_secs(120), "{what}");
        std::thread::sleep(Duration::from_millis(10));
    }
    // What was handed over is on its way; let it arrive.
    std::thread::sleep(Duration::from_millis(500));
}

/// A daemon stand-in whose streams are made of SMALL frames, one message
/// to a frame, sent as fast as flow control allows — and which reads what
/// it is sent only when told to.
struct SmallFrames {
    progress: Arc<Progress>,
}

#[tonic::async_trait]
impl blit_core::generated::blit_server::Blit for SmallFrames {
    type FindStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::FindEntry, tonic::Status>,
    >;
    type DiskUsageStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::DiskUsageEntry, tonic::Status>,
    >;
    type DelegatedPullStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::DelegatedPullProgress, tonic::Status>,
    >;
    type SubscribeStream = std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<
                    Item = Result<blit_core::generated::DaemonEvent, tonic::Status>,
                > + Send,
        >,
    >;

    type TransferStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::TransferFrame, tonic::Status>,
    >;

    // otp-1: unified-session wire surface; fakes refuse like the
    // real service until otp-3/otp-4 (docs/TRANSFER_SESSION.md).
    async fn transfer(
        &self,
        request: tonic::Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
    ) -> Result<tonic::Response<Self::TransferStream>, tonic::Status> {
        // Nothing is read until the test says the sender has finished;
        // then every frame is counted, and the reply ends.
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let progress = Arc::clone(&self.progress);
        let mut frames = request.into_inner();
        tokio::spawn(async move {
            while !progress.read_now.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            loop {
                match frames.message().await {
                    Ok(Some(_)) => {
                        progress.frames_read.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(None) => break,
                    Err(status) => {
                        let _ = tx.send(Err(status)).await;
                        break;
                    }
                }
            }
        });
        Ok(tonic::Response::new(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        ))
    }

    type GetJobLogStream = tokio_stream::wrappers::ReceiverStream<
        Result<blit_core::generated::JobLogChunk, tonic::Status>,
    >;

    async fn get_job_log(
        &self,
        _: tonic::Request<blit_core::generated::GetJobLogRequest>,
    ) -> Result<tonic::Response<Self::GetJobLogStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("test double"))
    }

    async fn subscribe(
        &self,
        _: tonic::Request<blit_core::generated::SubscribeRequest>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn list(
        &self,
        _: tonic::Request<blit_core::generated::ListRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn purge(
        &self,
        _: tonic::Request<blit_core::generated::PurgeRequest>,
    ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn complete_path(
        &self,
        _: tonic::Request<blit_core::generated::CompletionRequest>,
    ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn list_modules(
        &self,
        _: tonic::Request<blit_core::generated::ListModulesRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ListModulesResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn find(
        &self,
        _: tonic::Request<blit_core::generated::FindRequest>,
    ) -> Result<tonic::Response<Self::FindStream>, tonic::Status> {
        // One row to a message, one message to a frame: the channel
        // holds a single row, so the encoder never finds two waiting.
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let progress = Arc::clone(&self.progress);
        tokio::spawn(async move {
            for row in 0..ROWS {
                let entry = blit_core::generated::FindEntry {
                    relative_path: format!("f{row:05}"),
                    is_dir: false,
                    size: 0,
                    mtime_seconds: 0,
                };
                if tx.send(Ok(entry)).await.is_err() {
                    break;
                }
                tokio::task::yield_now().await;
            }
            progress.rows_sent.store(true, Ordering::SeqCst);
        });
        Ok(tonic::Response::new(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        ))
    }

    async fn disk_usage(
        &self,
        _: tonic::Request<blit_core::generated::DiskUsageRequest>,
    ) -> Result<tonic::Response<Self::DiskUsageStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn filesystem_stats(
        &self,
        _: tonic::Request<blit_core::generated::FilesystemStatsRequest>,
    ) -> Result<tonic::Response<blit_core::generated::FilesystemStatsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn delegated_pull(
        &self,
        _: tonic::Request<blit_core::generated::DelegatedPullRequest>,
    ) -> Result<tonic::Response<Self::DelegatedPullStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn get_state(
        &self,
        _: tonic::Request<blit_core::generated::GetStateRequest>,
    ) -> Result<tonic::Response<blit_core::generated::DaemonState>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn cancel_job(
        &self,
        _: tonic::Request<blit_core::generated::CancelJobRequest>,
    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }

    async fn clear_recent(
        &self,
        _: tonic::Request<blit_core::generated::ClearRecentRequest>,
    ) -> Result<tonic::Response<blit_core::generated::ClearRecentResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("stale daemon"))
    }
}

/// The admin client (`blit_core::client::connect_with_timeout`), through
/// the real binary: `blit find` whose reader pauses — a pager, a slow
/// terminal — still prints the whole listing. Every row has reached the
/// CLI before anything is read from it. It used to end with "h2 protocol
/// error: error reading a body from connection", exit 1 and part of the
/// listing.
#[test]
fn a_listing_read_later_than_it_arrives_is_whole() {
    let progress = Arc::new(Progress::default());
    let daemon = spawn_fake_blit_server(
        SmallFrames {
            progress: Arc::clone(&progress),
        },
        "small-frames daemon",
    );
    let scratch = tempfile::tempdir().unwrap();
    let mut child = Command::new(common::cli_bin())
        .arg("--config-dir")
        .arg(scratch.path())
        .arg("find")
        .arg(format!("127.0.0.1:{}:/test/", daemon.port))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn blit find");
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let said = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    wait_for("the stand-in sends its rows", &progress.rows_sent);
    let mut listing = Vec::new();
    stdout.read_to_end(&mut listing).expect("read the listing");
    let status = child.wait().expect("wait for blit find");
    let said = said.join().unwrap();

    assert!(status.success(), "blit find: {said}");
    let listing = String::from_utf8_lossy(&listing);
    let rows = listing
        .lines()
        .filter(|line| line.trim_start().starts_with('f'))
        .count();
    assert_eq!(rows, ROWS, "every row is printed\n{said}");
}

/// The transfer client (`connect_transfer_client`, which every pull and
/// every daemon-to-daemon transfer connects through): a stream of small
/// frames taken later than it arrives is whole. A pull of many small
/// files is this, whenever the destination is the slower side.
#[tokio::test(flavor = "multi_thread")]
async fn a_transfer_clients_stream_read_later_than_it_arrives_is_whole() {
    let progress = Arc::new(Progress::default());
    let daemon = spawn_fake_blit_server(
        SmallFrames {
            progress: Arc::clone(&progress),
        },
        "small-frames daemon",
    );
    let blit_core::endpoints::Endpoint::Remote(remote) =
        blit_core::endpoints::parse_endpoint_or_local(&format!("127.0.0.1:{}:/test/", daemon.port))
    else {
        panic!("a remote endpoint");
    };
    let mut client = blit_core::remote::transfer::session_client::connect_transfer_client(&remote)
        .await
        .expect("connect");
    let mut rows = client
        .find(blit_core::generated::FindRequest::default())
        .await
        .expect("the stream opens")
        .into_inner();

    let sent = Arc::clone(&progress);
    tokio::task::spawn_blocking(move || wait_for("the stand-in sends its rows", &sent.rows_sent))
        .await
        .unwrap();
    let mut read = 0;
    while let Some(_row) = rows.message().await.expect("the stream stays open") {
        read += 1;
    }
    assert_eq!(read, ROWS);
}

/// The server (`production_server_builder`, which the daemon serves
/// through): small frames SENT to a daemon that takes them later than
/// they arrive all reach it. A push of many small files is this — the
/// source names files far faster than the destination creates them — and
/// 200,000 empty files pushed to a daemon on the same machine used to
/// fail after 43 ms with 1,159 written: "transfer transport peer closed".
#[tokio::test(flavor = "multi_thread")]
async fn small_frames_sent_to_a_daemon_that_reads_them_later_all_arrive() {
    let progress = Arc::new(Progress::default());
    let daemon = spawn_fake_blit_server(
        SmallFrames {
            progress: Arc::clone(&progress),
        },
        "small-frames daemon",
    );
    let mut client =
        blit_core::client::connect_with_timeout(format!("http://127.0.0.1:{}", daemon.port))
            .await
            .expect("connect");

    // One frame to a message, as the stand-in sends its rows.
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let sent_all = Arc::new(AtomicBool::new(false));
    let sender = {
        let sent_all = Arc::clone(&sent_all);
        tokio::spawn(async move {
            for _ in 0..ROWS {
                if tx
                    .send(blit_core::generated::TransferFrame::default())
                    .await
                    .is_err()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
            sent_all.store(true, Ordering::SeqCst);
        })
    };
    let mut reply = client
        .transfer(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await
        .expect("the stream opens")
        .into_inner();

    let flag = Arc::clone(&sent_all);
    tokio::task::spawn_blocking(move || wait_for("the frames are sent", &flag))
        .await
        .unwrap();
    progress.read_now.store(true, Ordering::SeqCst);
    sender.await.unwrap();
    while let Some(_frame) = reply.message().await.expect("the daemon read every frame") {}
    assert_eq!(progress.frames_read.load(Ordering::SeqCst), ROWS);
}
