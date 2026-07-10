//! Client-side entry for initiating a unified transfer session.
//!
//! [`run_push_session`] declares the SOURCE role (push-equivalent,
//! otp-4): open the `Transfer` RPC, stream the manifest + payloads; the
//! daemon answers as the DESTINATION Responder. [`run_pull_session`]
//! declares the DESTINATION role (pull-equivalent, otp-5a): the daemon
//! answers as the SOURCE Responder and streams its module tree, which
//! this end diffs and writes. Both build a gRPC-backed [`FrameTransport`]
//! over `BlitClient::transfer` and run the matching role driver; role is
//! carried in `SessionOpen.initiator_role`, never a second code path.
//!
//! Not yet wired to CLI verbs — the verbs keep riding the old paths
//! until the otp-10 cutover; today the parity tests drive this. Both push
//! (otp-4b) and pull (otp-5b) default to the TCP data plane; the in-stream
//! carrier is the requested fallback either direction.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eyre::{eyre, Result};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Endpoint};

use crate::generated::blit_client::BlitClient;
use crate::generated::{
    ComparisonMode, FilterSpec, MirrorMode, ResumeSettings, SessionOpen, TransferRole,
    TransferSummary,
};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use crate::remote::transfer::source::TransferSource;
use crate::remote::transfer::ByteProgressSink;
use crate::transfer_plan::PlanOptions;
use crate::transfer_session::transport::{grpc_client_transport, GRPC_CHANNEL_FRAMES};
use crate::transfer_session::{
    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
    HelloConfig, SessionEndpoint, SourceSessionConfig,
};

/// The push-shaped subset of session options the landed slices support.
/// Mirror and filters are refused at OPEN until their client wiring
/// lands (the session itself honors them since otp-6), so they are
/// intentionally absent here.
pub struct PushSessionOptions {
    pub compare_mode: ComparisonMode,
    pub ignore_existing: bool,
    pub require_complete_scan: bool,
    pub plan_options: PlanOptions,
    /// Force the in-stream byte carrier instead of the TCP data plane
    /// (otp-4b). Default `false` = the responder grants a data plane and
    /// payloads ride TCP sockets; `true` is the diagnostics / unreachable
    /// data-plane fallback (`--force-grpc`-shaped).
    pub in_stream_bytes: bool,
    /// otp-7b: negotiate the resume block phase (`SessionOpen.resume`).
    /// Changed dest partials are then patched block-wise instead of
    /// re-transferred whole.
    pub resume: bool,
    /// Requested resume block size in bytes; `0` lets the DESTINATION
    /// choose (currently 1 MiB). The destination clamps to its
    /// carrier's bounds either way. Ignored unless `resume` is true.
    pub resume_block_size: u32,
}

impl Default for PushSessionOptions {
    fn default() -> Self {
        Self {
            compare_mode: ComparisonMode::SizeMtime,
            ignore_existing: false,
            require_complete_scan: false,
            plan_options: PlanOptions::default(),
            in_stream_bytes: false,
            resume: false,
            resume_block_size: 0,
        }
    }
}

/// Connect to `endpoint`'s daemon and run one SOURCE-role transfer
/// session pushing `source`'s tree into the endpoint's module/path.
/// Returns the destination-computed [`TransferSummary`] (contract:
/// DESTINATION is the scorer).
pub async fn run_push_session(
    endpoint: &RemoteEndpoint,
    source: Arc<dyn TransferSource>,
    options: PushSessionOptions,
) -> Result<TransferSummary> {
    // The responder resolves module→root; the initiator's own local
    // path never crosses the wire (contract §SessionOpen).
    let (module, path) = endpoint_module_path(endpoint)?;

    let mut client = connect_transfer_client(endpoint).await?;

    let open = SessionOpen {
        initiator_role: TransferRole::Source as i32,
        module,
        path,
        compare_mode: options.compare_mode as i32,
        ignore_existing: options.ignore_existing,
        require_complete_scan: options.require_complete_scan,
        // otp-4b: default to the TCP data plane; the responder grants it
        // in SessionAccept unless this asks for the in-stream fallback.
        in_stream_bytes: options.in_stream_bytes,
        // otp-7b: resume rides the open (plan D6 — the flag is in the
        // open, so resume runs identically whichever end initiated).
        resume: options.resume.then_some(ResumeSettings {
            enabled: true,
            block_size: options.resume_block_size,
        }),
        ..Default::default()
    };

    // Open the bidi RPC: the request stream is fed by `out_tx`, the
    // response stream is the inbound half. The handler returns its
    // response stream immediately (it spawns the session), so this
    // await resolves before any frame flows — no deadlock.
    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
    let inbound = client
        .transfer(ReceiverStream::new(out_rx))
        .await
        .map_err(|status| eyre::Report::new(transfer_open_refusal(status)))?
        .into_inner();
    let transport = grpc_client_transport(out_tx, inbound);

    let cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: options.plan_options,
        // The initiator dials the data plane on the same host it reached
        // the control plane on (contract §Transport: initiator dials).
        data_plane_host: Some(endpoint.host.clone()),
    };
    run_source(cfg, transport, source).await
}

/// The pull-shaped subset of session options the landed slices support.
/// Mirror and filters ride the open since otp-9a (the session honors
/// them since otp-6). The DESTINATION owns the compare decision; the
/// SOURCE owns the planner knobs (none cross the wire).
pub struct PullSessionOptions {
    pub compare_mode: ComparisonMode,
    pub ignore_existing: bool,
    pub require_complete_scan: bool,
    /// Force the in-stream byte carrier instead of the TCP data plane
    /// (otp-5b). Default `false` = the SOURCE responder grants a data
    /// plane and this DESTINATION initiator dials + receives over TCP
    /// sockets; `true` is the diagnostics / unreachable data-plane
    /// fallback. Symmetric with [`PushSessionOptions::in_stream_bytes`].
    pub in_stream_bytes: bool,
    /// otp-7b: negotiate the resume block phase — symmetric with
    /// [`PushSessionOptions::resume`] (plan D6: the flag is in the open,
    /// so resume runs identically whichever end initiated).
    pub resume: bool,
    /// Requested resume block size in bytes; `0` lets the DESTINATION
    /// (this end) choose. Ignored unless `resume` is true.
    pub resume_block_size: u32,
    /// otp-9a: source-side scan filter, riding `SessionOpen.filter`
    /// (the session honors it since otp-6a — this is the client
    /// wiring). `None` scans everything.
    pub filter: Option<FilterSpec>,
    /// otp-9a: mirror on the session (otp-6b's one delete rule — this
    /// DESTINATION diffs the complete source manifest against its tree
    /// at SourceDone and deletes extraneous entries locally). Explicit
    /// enabled + scope per the contract; `MirrorMode::Off` with
    /// `mirror_enabled` set is refused at OPEN.
    pub mirror_enabled: bool,
    pub mirror_kind: MirrorMode,
    /// otp-9a: live counter the session sink reports applied payload
    /// bytes against (the delegated dst daemon's jobs row, otp-9; CLI
    /// progress at otp-10).
    pub byte_progress: Option<ByteProgressSink>,
}

impl Default for PullSessionOptions {
    fn default() -> Self {
        Self {
            compare_mode: ComparisonMode::SizeMtime,
            ignore_existing: false,
            require_complete_scan: false,
            in_stream_bytes: false,
            resume: false,
            resume_block_size: 0,
            filter: None,
            mirror_enabled: false,
            mirror_kind: MirrorMode::Off,
            byte_progress: None,
        }
    }
}

/// Connect to `endpoint`'s daemon and run one DESTINATION-role transfer
/// session pulling the endpoint's module/path tree into `dest_root`
/// (pull-equivalent, otp-5a). The client initiates and declares
/// DESTINATION, so the daemon becomes the SOURCE Responder (streaming
/// its module tree). Returns the [`DestinationOutcome`] this end
/// computed (contract: the DESTINATION is the scorer).
///
/// otp-5b: the default carrier is the TCP data plane — the SOURCE
/// responder binds+grants+accepts sockets while sending, and this
/// DESTINATION initiator dials + receives over them (the transport/role
/// decoupling). `PullSessionOptions::in_stream_bytes` forces the in-stream
/// fallback (diagnostics / unreachable data plane). Not wired to CLI verbs
/// (otp-10).
pub async fn run_pull_session(
    endpoint: &RemoteEndpoint,
    dest_root: PathBuf,
    options: PullSessionOptions,
) -> Result<DestinationOutcome> {
    let client = connect_transfer_client(endpoint).await?;
    run_pull_session_with_client(client, endpoint, dest_root, options).await
}

/// [`run_pull_session`] over an already-connected client (otp-9b). The
/// delegated dst daemon connects separately so a connect failure keeps
/// its own error phase (`ConnectSource`) structurally, without string
/// matching on the session error.
pub async fn run_pull_session_with_client(
    mut client: BlitClient<Channel>,
    endpoint: &RemoteEndpoint,
    dest_root: PathBuf,
    options: PullSessionOptions,
) -> Result<DestinationOutcome> {
    let (module, path) = endpoint_module_path(endpoint)?;

    let open = SessionOpen {
        initiator_role: TransferRole::Destination as i32,
        module,
        path,
        compare_mode: options.compare_mode as i32,
        ignore_existing: options.ignore_existing,
        require_complete_scan: options.require_complete_scan,
        // otp-5b: default to the TCP data plane; the SOURCE responder
        // grants it in SessionAccept unless this asks for the in-stream
        // fallback.
        in_stream_bytes: options.in_stream_bytes,
        // otp-7b: resume rides the open, role-agnostic (plan D6).
        resume: options.resume.then_some(ResumeSettings {
            enabled: true,
            block_size: options.resume_block_size,
        }),
        // otp-9a: filter + mirror ride the open (otp-6a/6b session
        // support; this is the client wiring).
        filter: options.filter,
        mirror_enabled: options.mirror_enabled,
        mirror_kind: options.mirror_kind as i32,
        ..Default::default()
    };

    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
    let inbound = client
        .transfer(ReceiverStream::new(out_rx))
        .await
        .map_err(|status| eyre::Report::new(transfer_open_refusal(status)))?
        .into_inner();
    let transport = grpc_client_transport(out_tx, inbound);

    let cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        // The initiator dials the data plane on the same host it reached
        // the control plane on (contract §Transport: initiator dials).
        data_plane_host: Some(endpoint.host.clone()),
        byte_progress: options.byte_progress,
    };
    run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await
}

/// Derive the wire `(module, path)` from a resolved endpoint. Empty
/// module targets the daemon's default root export; a discovery-form
/// endpoint is not resolvable to a transfer target.
fn endpoint_module_path(endpoint: &RemoteEndpoint) -> Result<(String, String)> {
    match &endpoint.path {
        RemotePath::Module { module, rel_path } => {
            Ok((module.clone(), rel_path.to_string_lossy().into_owned()))
        }
        RemotePath::Root { rel_path } => {
            Ok((String::new(), rel_path.to_string_lossy().into_owned()))
        }
        RemotePath::Discovery => Err(eyre!(
            "a transfer session needs a resolved module or root endpoint, not a discovery form"
        )),
    }
}

/// The `Transfer` RPC failed at OPEN — before any session frame flowed.
/// A distinct error type (not a bare `SessionFault`) so callers can
/// classify EVERY open-time failure structurally as a negotiation
/// failure (codex otp-9b F3 — the old typed `PullSyncError` boundary
/// treated every pre-response RPC failure as NEGOTIATE); the inner
/// fault still carries the closest session code for the message.
#[derive(Debug)]
pub struct TransferOpenRefusal(pub crate::transfer_session::SessionFault);

impl std::fmt::Display for TransferOpenRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TransferOpenRefusal {}

/// Map an open-time gRPC status onto the `SessionFault` code the same
/// refusal would carry as a session frame. On a same-build fleet an
/// `Unimplemented` Transfer only means a pre-session peer — the
/// build-mismatch shape; `PermissionDenied` is the peer's own
/// delegation/ACL gate; anything else keeps INTERNAL, with the
/// [`TransferOpenRefusal`] wrapper preserving the open-phase identity.
fn transfer_open_refusal(status: tonic::Status) -> TransferOpenRefusal {
    use crate::generated::session_error::Code;
    let code = match status.code() {
        tonic::Code::Unimplemented => Code::BuildMismatch,
        tonic::Code::PermissionDenied => Code::DelegationRefused,
        _ => Code::Internal,
    };
    TransferOpenRefusal(crate::transfer_session::SessionFault::refusal(
        code,
        format!("opening Transfer RPC: {}", status.message()),
    ))
}

/// Build a `BlitClient` over `endpoint`'s control-plane URI with the
/// same bounded-connect policy `RemotePushClient::connect` uses.
/// `pub` since otp-9b: the delegated dst daemon connects separately
/// from running the session so connect failures keep their own phase.
pub async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
    let uri = endpoint.control_plane_uri();
    let conn = Endpoint::from_shared(uri.clone())
        .map_err(|e| eyre!("invalid endpoint uri {uri}: {e}"))?
        .connect_timeout(Duration::from_secs(30));
    let channel = tokio::time::timeout(Duration::from_secs(30), conn.connect())
        .await
        .map_err(|_| eyre!("timed out connecting to {uri}"))?
        .map_err(|e| eyre!("connecting to {uri}: {e}"))?;
    Ok(BlitClient::new(channel))
}
