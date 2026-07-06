//! Client-side entry for initiating a unified transfer session as the
//! SOURCE role (otp-4a).
//!
//! Builds a gRPC-backed [`FrameTransport`] over `BlitClient::transfer`
//! and runs [`run_source`], so a CLI push becomes "open the Transfer
//! RPC, declare SOURCE, stream the manifest + payloads." This is the
//! push-equivalent on the unified path; the daemon answers by running
//! `run_destination` as the Responder.
//!
//! Not yet wired to CLI verbs — the verbs keep riding the old push
//! path until the otp-10 cutover; today the parity tests drive this.
//! otp-4a uses the in-stream byte carrier only (`in_stream_bytes`);
//! the TCP data plane lands at otp-4b.

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
use crate::transfer_session::{run_source, HelloConfig, SessionEndpoint, SourceSessionConfig};

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
    // path never crosses the wire (contract §SessionOpen). Empty module
    // targets the daemon's default root export.
    let (module, path) = match &endpoint.path {
        RemotePath::Module { module, rel_path } => {
            (module.clone(), rel_path.to_string_lossy().into_owned())
        }
        RemotePath::Root { rel_path } => (String::new(), rel_path.to_string_lossy().into_owned()),
        RemotePath::Discovery => {
            return Err(eyre!(
                "a transfer session needs a resolved module or root endpoint, not a discovery form"
            ));
        }
    };

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
