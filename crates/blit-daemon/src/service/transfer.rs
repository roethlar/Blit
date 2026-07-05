//! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
//!
//! otp-1 (D-2026-07-05-4) landed the wire surface (the RPC, the frame
//! set, `docs/TRANSFER_SESSION.md`) with the handler refusing
//! UNIMPLEMENTED. otp-4a lands the behavior: the daemon serves the RPC
//! by running `blit_core::transfer_session::run_destination` as the
//! Responder — the byte RECEIVER of a client-initiated SOURCE push.
//! The dispatcher in `core.rs::transfer` mirrors `push`: register a
//! jobs row, race the session against cancel/hangup via
//! `resolve_streaming_outcome`, return the response `ReceiverStream`.
//!
//! This module owns the two daemon-specific pieces the session driver
//! in blit-core cannot: (1) the [`OpenResolver`] that maps a wire
//! module/path to a local root and read-only decision (blit-core stays
//! free of module config and `tonic::Status`), and (2) the transport
//! assembly + outcome mapping.
//!
//! otp-4a uses the in-stream byte carrier only; the TCP data plane
//! grant + resize land at otp-4b. Progress-byte wiring
//! (`with_byte_progress`) is not threaded yet — session rows report
//! `bytes_completed=0`, matching today's push rows.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tonic::{Status, Streaming};

use blit_core::generated::session_error::Code;
use blit_core::generated::{SessionOpen, TransferFrame};
use blit_core::transfer_session::transport::grpc_daemon_transport;
use blit_core::transfer_session::{
    run_destination, DestinationSessionConfig, DestinationTarget, HelloConfig, OpenResolver,
    ResolvedEndpoint, SessionEndpoint, SessionFault,
};

use super::util::{resolve_contained_path, resolve_module, resolve_relative_path};
use crate::runtime::{ModuleConfig, RootExport};

/// Map a resolver `tonic::Status` onto a `SessionError` code. blit-core
/// is deliberately `Status`-free, so the daemon picks the wire code:
/// an unknown module is `MODULE_UNKNOWN`, a bad or escaping path is a
/// `PROTOCOL_VIOLATION` (the initiator sent an unusable request),
/// anything else is `INTERNAL`.
fn status_to_fault(status: Status) -> SessionFault {
    let code = match status.code() {
        tonic::Code::NotFound => Code::ModuleUnknown,
        tonic::Code::InvalidArgument | tonic::Code::PermissionDenied => Code::ProtocolViolation,
        _ => Code::Internal,
    };
    SessionFault::refusal(code, status.message().to_string())
}

/// Build the daemon's [`OpenResolver`]: given a received `SessionOpen`,
/// resolve its module + path to an absolute local root and report the
/// module's read-only flag. Mirrors the push Header sequence
/// (`resolve_module` → path validation → F2 canonical containment via
/// `resolve_contained_path`), refusing with a `SessionError` instead of
/// a `tonic::Status`. The closure is `Fn` (callable once per session)
/// and clones its captured handles per call so it stays `Send + Sync`.
pub(crate) fn make_open_resolver(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
) -> Box<OpenResolver> {
    Box::new(move |open: &SessionOpen| {
        let modules = Arc::clone(&modules);
        let default_root = default_root.clone();
        let module_name = open.module.clone();
        let wire_path = open.path.clone();
        Box::pin(async move {
            let config = resolve_module(&modules, default_root.as_ref(), &module_name)
                .await
                .map_err(status_to_fault)?;
            // Empty path targets the module root; a non-empty path is
            // validated and contained against the module's canonical
            // root (F2 symlink-escape protection — the same chokepoint
            // the per-file write path uses).
            let root = if wire_path.is_empty() {
                config.path.clone()
            } else {
                let rel = resolve_relative_path(&wire_path).map_err(status_to_fault)?;
                resolve_contained_path(&config, &rel).map_err(status_to_fault)?
            };
            Ok(ResolvedEndpoint {
                root,
                read_only: config.read_only,
            })
        })
    })
}

/// Run one daemon-side transfer session to completion as the DESTINATION
/// Responder, returning `Ok(())` on a clean transfer or `Err(Status)`
/// carrying the session fault's message for the jobs record. The
/// session communicates its own refusals to the peer as `SessionError`
/// *frames* (via the response stream); this `Status` is for the
/// daemon's outcome record and `resolve_streaming_outcome`'s terminal
/// handling, not the primary error channel.
pub(crate) async fn run_transfer_session(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    inbound: Streaming<TransferFrame>,
    tx: mpsc::Sender<Result<TransferFrame, Status>>,
) -> Result<(), Status> {
    let transport = grpc_daemon_transport(tx, inbound);
    let resolver = make_open_resolver(modules, default_root);
    let cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
    };
    match run_destination(cfg, transport, DestinationTarget::Resolve(resolver)).await {
        Ok(_outcome) => Ok(()),
        Err(report) => {
            // run_destination already emitted a SessionError frame to
            // the peer; surface the reason for the record.
            let msg = report
                .downcast_ref::<SessionFault>()
                .map(|f| f.message.clone())
                .unwrap_or_else(|| format!("{report:#}"));
            Err(Status::internal(msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_to_fault_maps_not_found_to_module_unknown() {
        let fault = status_to_fault(Status::not_found("module 'x' not found"));
        assert_eq!(fault.code, Code::ModuleUnknown);
        assert!(fault.message.contains("not found"));
    }

    #[test]
    fn status_to_fault_maps_permission_denied_to_protocol_violation() {
        let fault = status_to_fault(Status::permission_denied("path containment: escape"));
        assert_eq!(fault.code, Code::ProtocolViolation);
    }

    #[test]
    fn status_to_fault_maps_invalid_argument_to_protocol_violation() {
        let fault = status_to_fault(Status::invalid_argument("path not allowed"));
        assert_eq!(fault.code, Code::ProtocolViolation);
    }
}
