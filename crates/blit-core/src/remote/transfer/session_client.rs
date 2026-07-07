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
use crate::generated::{ComparisonMode, SessionOpen, TransferRole, TransferSummary};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use crate::remote::transfer::source::TransferSource;
use crate::transfer_plan::PlanOptions;
use crate::transfer_session::transport::{grpc_client_transport, GRPC_CHANNEL_FRAMES};
use crate::transfer_session::{
    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
    HelloConfig, SessionEndpoint, SourceSessionConfig,
};

/// The push-shaped subset of session options otp-4a/4b supports. Mirror,
/// filters, and resume are refused at OPEN until their slices land
/// (otp-6/otp-7), so they are intentionally absent here.
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
}

impl Default for PushSessionOptions {
    fn default() -> Self {
        Self {
            compare_mode: ComparisonMode::SizeMtime,
            ignore_existing: false,
            require_complete_scan: false,
            plan_options: PlanOptions::default(),
            in_stream_bytes: false,
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
        .map_err(|status| eyre!("opening Transfer RPC: {}", status.message()))?
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

/// The pull-shaped subset of session options otp-5a supports. Mirror,
/// filters, and resume are refused at OPEN until their slices land, so
/// they are intentionally absent here. The DESTINATION owns the compare
/// decision; the SOURCE owns the planner knobs (none cross the wire).
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
}

impl Default for PullSessionOptions {
    fn default() -> Self {
        Self {
            compare_mode: ComparisonMode::SizeMtime,
            ignore_existing: false,
            require_complete_scan: false,
            in_stream_bytes: false,
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
    let (module, path) = endpoint_module_path(endpoint)?;

    let mut client = connect_transfer_client(endpoint).await?;

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
        ..Default::default()
    };

    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
    let inbound = client
        .transfer(ReceiverStream::new(out_rx))
        .await
        .map_err(|status| eyre!("opening Transfer RPC: {}", status.message()))?
        .into_inner();
    let transport = grpc_client_transport(out_tx, inbound);

    let cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        // The initiator dials the data plane on the same host it reached
        // the control plane on (contract §Transport: initiator dials).
        data_plane_host: Some(endpoint.host.clone()),
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

/// Build a `BlitClient` over `endpoint`'s control-plane URI with the
/// same bounded-connect policy `RemotePushClient::connect` uses.
async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
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
