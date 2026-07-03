//! ue-r2-1b: old/new peer wire-compatibility tests for the dial
//! contract (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` §5, slice
//! `ue-r2-1b-wire-dial-contract`).
//!
//! The slice adds `CapacityProfile receiver_capacity` (negotiation
//! field 11 / spec field 12), the `resize_enabled` + `epoch0_sub_token`
//! negotiation fields, the `supports_stream_resize` capability bits
//! (PushHeader 8 / PeerCapabilities 5), and the `DataPlaneResize`/
//! `DataPlaneResizeAck` oneof variants. REV4's acceptance criterion:
//! old peers must see current behavior — no capacity profile means
//! today's static behavior, no resize support means no mid-transfer
//! add/drop — *before* any code depends on the fields.
//!
//! Technique: each `Old*` struct below replicates the exact pre-1b
//! wire shape of a message this slice extends (same field numbers and
//! types, minus the new fields), via test-local `#[derive(prost::
//! Message)]`. Encoding new→decoding old proves an old binary skips
//! the new fields; encoding old→decoding new proves a new binary sees
//! proto3 defaults (`None`/`false`/empty) and can treat "absent" as
//! "old peer". The reduced-oneof replicas are equivalent to full
//! replicas for this purpose: prost's unknown-field skipping is per
//! field number, so variants absent from the replica behave identically
//! whether the replica declares two known variants or all of them.
//!
//! The critical non-obvious property pinned here: `spec_version` stays
//! at 2. `NormalizedTransferOperation::from_spec` enforces the version
//! by exact match, so bumping it would make every old daemon reject
//! every new client. `receiver_capacity` is a pure optimization hint —
//! an old daemon skipping it is the intended mixed-version behavior —
//! so it ships without a bump (unlike v2's safety-critical
//! `require_complete_scan`).

use blit_core::generated::{
    client_push_request, server_pull_message, CapacityProfile, ClientPushRequest, DataPlaneResize,
    DataPlaneResizeAck, DataPlaneResizeOp, DataTransferNegotiation, DrainClass, PeerCapabilities,
    PushHeader, ServerPullMessage, TransferOperationSpec,
};
use blit_core::remote::transfer::operation_spec::{
    NormalizedTransferOperation, SUPPORTED_SPEC_VERSION,
};
use prost::Message;

// ─── Pre-ue-r2-1b wire-shape replicas ────────────────────────────────

/// `DataTransferNegotiation` as every released peer knows it: fields
/// 1-4, `reserved 5 to 10`.
#[derive(Clone, PartialEq, prost::Message)]
struct OldDataTransferNegotiation {
    #[prost(uint32, tag = "1")]
    tcp_port: u32,
    #[prost(string, tag = "2")]
    one_time_token: String,
    #[prost(bool, tag = "3")]
    tcp_fallback: bool,
    #[prost(uint32, tag = "4")]
    stream_count: u32,
}

/// `PushHeader` pre-1b: fields 1-7 (no `supports_stream_resize = 8`).
#[derive(Clone, PartialEq, prost::Message)]
struct OldPushHeader {
    #[prost(string, tag = "1")]
    module: String,
    #[prost(bool, tag = "2")]
    mirror_mode: bool,
    #[prost(string, tag = "3")]
    destination_path: String,
    #[prost(bool, tag = "4")]
    force_grpc: bool,
    #[prost(message, optional, tag = "5")]
    filter: Option<blit_core::generated::FilterSpec>,
    #[prost(enumeration = "blit_core::generated::MirrorMode", tag = "6")]
    mirror_kind: i32,
    #[prost(bool, tag = "7")]
    require_complete_scan: bool,
}

/// `PeerCapabilities` pre-1b: fields 1-4 (no `supports_stream_resize`).
#[derive(Clone, PartialEq, prost::Message)]
struct OldPeerCapabilities {
    #[prost(bool, tag = "1")]
    supports_resume: bool,
    #[prost(bool, tag = "2")]
    supports_tar_shards: bool,
    #[prost(bool, tag = "3")]
    supports_data_plane_tcp: bool,
    #[prost(bool, tag = "4")]
    supports_filter_spec: bool,
}

/// `TransferOperationSpec` pre-1b: fields 1-11 (no
/// `receiver_capacity = 12`), nesting the old capabilities shape.
#[derive(Clone, PartialEq, prost::Message)]
struct OldTransferOperationSpec {
    #[prost(uint32, tag = "1")]
    spec_version: u32,
    #[prost(string, tag = "2")]
    module: String,
    #[prost(string, tag = "3")]
    source_path: String,
    #[prost(message, optional, tag = "4")]
    filter: Option<blit_core::generated::FilterSpec>,
    #[prost(enumeration = "blit_core::generated::ComparisonMode", tag = "5")]
    compare_mode: i32,
    #[prost(enumeration = "blit_core::generated::MirrorMode", tag = "6")]
    mirror_mode: i32,
    #[prost(message, optional, tag = "7")]
    resume: Option<blit_core::generated::ResumeSettings>,
    #[prost(message, optional, tag = "8")]
    client_capabilities: Option<OldPeerCapabilities>,
    #[prost(bool, tag = "9")]
    force_grpc: bool,
    #[prost(bool, tag = "10")]
    ignore_existing: bool,
    #[prost(bool, tag = "11")]
    require_complete_scan: bool,
}

/// Reduced `ServerPullMessage` replica: declares only `Summary = 4` of
/// the 15 pre-1b variants (see the module doc for why reduced is
/// equivalent). `DataPlaneResize = 16` is unknown to it.
#[derive(Clone, PartialEq, prost::Message)]
struct OldServerPullMessage {
    #[prost(oneof = "old_server_pull_message::Payload", tags = "4")]
    payload: Option<old_server_pull_message::Payload>,
}
mod old_server_pull_message {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Payload {
        #[prost(message, tag = "4")]
        Summary(blit_core::generated::PullSummary),
    }
}

/// Reduced `ClientPushRequest` replica: declares only
/// `UploadComplete = 5`. `DataPlaneResize = 9` is unknown to it.
#[derive(Clone, PartialEq, prost::Message)]
struct OldClientPushRequest {
    #[prost(oneof = "old_client_push_request::Payload", tags = "5")]
    payload: Option<old_client_push_request::Payload>,
}
mod old_client_push_request {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Payload {
        #[prost(message, tag = "5")]
        UploadComplete(blit_core::generated::UploadComplete),
    }
}

/// Reduced `ServerPushResponse` replica: declares only `Ack = 1`.
/// `DataPlaneResizeAck = 5` is unknown to it.
#[derive(Clone, PartialEq, prost::Message)]
struct OldServerPushResponse {
    #[prost(oneof = "old_server_push_response::Payload", tags = "1")]
    payload: Option<old_server_push_response::Payload>,
}
mod old_server_push_response {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Payload {
        #[prost(message, tag = "1")]
        Ack(blit_core::generated::Ack),
    }
}

/// Reduced `ClientPullMessage` replica: declares only
/// `ManifestDone = 3`. `DataPlaneResizeAck = 5` is unknown to it.
#[derive(Clone, PartialEq, prost::Message)]
struct OldClientPullMessage {
    #[prost(oneof = "old_client_pull_message::Payload", tags = "3")]
    payload: Option<old_client_pull_message::Payload>,
}
mod old_client_pull_message {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Payload {
        #[prost(message, tag = "3")]
        ManifestDone(blit_core::generated::ManifestComplete),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────

fn full_capacity_profile() -> CapacityProfile {
    CapacityProfile {
        cpu_cores: 32,
        drain_class: DrainClass::SsdNvme as i32,
        load_percent: 42,
        max_streams: 16,
        drain_rate_bytes_per_sec: 3_200_000_000,
        max_chunk_bytes: 16 * 1024 * 1024,
        max_inflight_bytes: 512 * 1024 * 1024,
    }
}

fn new_spec_with_capacity() -> TransferOperationSpec {
    TransferOperationSpec {
        spec_version: SUPPORTED_SPEC_VERSION,
        module: "mod-a".into(),
        source_path: "dir/sub".into(),
        filter: None,
        compare_mode: blit_core::generated::ComparisonMode::SizeMtime as i32,
        mirror_mode: blit_core::generated::MirrorMode::Off as i32,
        resume: None,
        client_capabilities: Some(PeerCapabilities {
            supports_resume: true,
            supports_tar_shards: true,
            supports_data_plane_tcp: true,
            supports_filter_spec: true,
            supports_stream_resize: true,
        }),
        force_grpc: false,
        ignore_existing: false,
        require_complete_scan: true,
        receiver_capacity: Some(full_capacity_profile()),
    }
}

// ─── Old client / new daemon ─────────────────────────────────────────

#[test]
fn old_negotiation_decodes_as_new_with_dial_fields_absent() {
    // A new client (or new pull_sync daemon reading a spec — same
    // decode semantics) receiving an old peer's negotiation must see
    // "absent" dial fields and fall back to today's static behavior.
    let old = OldDataTransferNegotiation {
        tcp_port: 9401,
        one_time_token: "tok".into(),
        tcp_fallback: false,
        stream_count: 4,
    };
    let new = DataTransferNegotiation::decode(old.encode_to_vec().as_slice())
        .expect("old negotiation must decode as new");
    assert_eq!(new.tcp_port, 9401);
    assert_eq!(new.one_time_token, "tok");
    assert!(!new.tcp_fallback);
    assert_eq!(new.stream_count, 4);
    assert!(new.receiver_capacity.is_none());
    assert!(!new.resize_enabled);
    assert!(new.epoch0_sub_token.is_empty());
}

#[test]
fn old_spec_decodes_and_normalizes_on_new_daemon() {
    // The true old-client/new-daemon path: old spec bytes → new
    // generated type → the real normalization chokepoint. Must
    // normalize cleanly with no receiver profile.
    let old = OldTransferOperationSpec {
        spec_version: 2,
        module: "m".into(),
        source_path: "p".into(),
        filter: None,
        compare_mode: blit_core::generated::ComparisonMode::Checksum as i32,
        mirror_mode: blit_core::generated::MirrorMode::Off as i32,
        resume: None,
        client_capabilities: Some(OldPeerCapabilities {
            supports_resume: true,
            supports_tar_shards: true,
            supports_data_plane_tcp: true,
            supports_filter_spec: true,
        }),
        force_grpc: false,
        ignore_existing: false,
        require_complete_scan: false,
    };
    let new = TransferOperationSpec::decode(old.encode_to_vec().as_slice())
        .expect("old spec must decode as new");
    assert!(new.receiver_capacity.is_none());
    assert!(new
        .client_capabilities
        .as_ref()
        .is_some_and(|caps| !caps.supports_stream_resize));
    let normalized = NormalizedTransferOperation::from_spec(new)
        .expect("old spec must pass the new daemon's normalization gate");
    assert!(!normalized.capabilities.supports_stream_resize);
}

#[test]
fn old_push_header_decodes_as_new_daemon_with_resize_bit_false() {
    let old = OldPushHeader {
        module: "m".into(),
        mirror_mode: true,
        destination_path: "d".into(),
        force_grpc: false,
        filter: None,
        mirror_kind: blit_core::generated::MirrorMode::FilteredSubset as i32,
        require_complete_scan: true,
    };
    let new = PushHeader::decode(old.encode_to_vec().as_slice())
        .expect("old push header must decode as new");
    assert!(!new.supports_stream_resize);
    assert_eq!(new.module, "m");
    assert!(new.mirror_mode);
    assert!(new.require_complete_scan);
}

// ─── New client / old daemon ─────────────────────────────────────────

#[test]
fn new_negotiation_decodes_as_old_peer_ignoring_dial_fields() {
    let new = DataTransferNegotiation {
        tcp_port: 9401,
        one_time_token: "tok".into(),
        tcp_fallback: false,
        stream_count: 4,
        receiver_capacity: Some(full_capacity_profile()),
        resize_enabled: true,
        epoch0_sub_token: vec![7u8; 16],
    };
    let old = OldDataTransferNegotiation::decode(new.encode_to_vec().as_slice())
        .expect("new negotiation must decode on an old peer");
    assert_eq!(old.tcp_port, 9401);
    assert_eq!(old.one_time_token, "tok");
    assert!(!old.tcp_fallback);
    assert_eq!(old.stream_count, 4);
}

#[test]
fn new_spec_with_receiver_capacity_passes_old_daemon_version_gate() {
    // THE load-bearing compat property: receiver_capacity ships at
    // spec_version 2 (no bump). An old daemon must decode the new
    // spec with spec_version still == 2 — its exact-match version
    // gate passes and the unknown field is skipped, leaving today's
    // behavior.
    let new = new_spec_with_capacity();
    let old = OldTransferOperationSpec::decode(new.encode_to_vec().as_slice())
        .expect("new spec must decode on an old daemon");
    assert_eq!(old.spec_version, 2);
    assert_eq!(old.spec_version, SUPPORTED_SPEC_VERSION);
    assert_eq!(old.module, "mod-a");
    assert_eq!(old.source_path, "dir/sub");
    assert!(old.require_complete_scan);
    let caps = old.client_capabilities.expect("caps travel");
    assert!(caps.supports_resume);
    assert!(caps.supports_tar_shards);
    assert!(caps.supports_data_plane_tcp);
    assert!(caps.supports_filter_spec);
}

#[test]
fn new_push_header_decodes_as_old_daemon_ignoring_resize_bit() {
    let new = PushHeader {
        module: "m".into(),
        mirror_mode: false,
        destination_path: "d".into(),
        force_grpc: true,
        filter: None,
        mirror_kind: blit_core::generated::MirrorMode::Off as i32,
        require_complete_scan: false,
        supports_stream_resize: true,
    };
    let old = OldPushHeader::decode(new.encode_to_vec().as_slice())
        .expect("new push header must decode on an old daemon");
    assert_eq!(old.module, "m");
    assert!(old.force_grpc);
    assert!(!old.require_complete_scan);
}

// ─── Resize frames sent to an old peer decode as unknown payload ─────
//
// These pin what the capability gate protects against: if a resize
// frame ever reached an old binary, it would decode as `payload: None`
// (both existing None arms ignore the frame silently). The gate —
// resize frames only after `resize_enabled`, which old peers can never
// advertise — must therefore hold at the sender.

#[test]
fn resize_command_reads_as_unknown_payload_on_old_pull_client() {
    let msg = ServerPullMessage {
        payload: Some(server_pull_message::Payload::DataPlaneResize(
            DataPlaneResize {
                op: DataPlaneResizeOp::Add as i32,
                epoch: 1,
                target_stream_count: 5,
                sub_token: vec![1u8; 16],
            },
        )),
    };
    let old = OldServerPullMessage::decode(msg.encode_to_vec().as_slice())
        .expect("frame must still parse on an old peer");
    assert!(old.payload.is_none());

    // Control: a known variant still round-trips into the replica.
    let known = ServerPullMessage {
        payload: Some(server_pull_message::Payload::Summary(Default::default())),
    };
    let old_known = OldServerPullMessage::decode(known.encode_to_vec().as_slice()).unwrap();
    assert!(old_known.payload.is_some());
}

#[test]
fn resize_request_reads_as_unknown_payload_on_old_push_daemon() {
    let msg = ClientPushRequest {
        payload: Some(client_push_request::Payload::DataPlaneResize(
            DataPlaneResize {
                op: DataPlaneResizeOp::Remove as i32,
                epoch: 2,
                target_stream_count: 3,
                sub_token: Vec::new(),
            },
        )),
    };
    let old = OldClientPushRequest::decode(msg.encode_to_vec().as_slice())
        .expect("frame must still parse on an old daemon");
    assert!(old.payload.is_none());

    let known = ClientPushRequest {
        payload: Some(client_push_request::Payload::UploadComplete(
            Default::default(),
        )),
    };
    let old_known = OldClientPushRequest::decode(known.encode_to_vec().as_slice()).unwrap();
    assert!(old_known.payload.is_some());
}

#[test]
fn resize_ack_reads_as_unknown_payload_on_old_peers() {
    use blit_core::generated::{
        client_pull_message, server_push_response, ClientPullMessage, ServerPushResponse,
    };
    // Push direction: daemon→client ack.
    let push_ack = ServerPushResponse {
        payload: Some(server_push_response::Payload::DataPlaneResizeAck(
            DataPlaneResizeAck {
                epoch: 1,
                effective_stream_count: 5,
                accepted: true,
            },
        )),
    };
    let old_push = OldServerPushResponse::decode(push_ack.encode_to_vec().as_slice())
        .expect("frame must still parse on an old client");
    assert!(old_push.payload.is_none());

    // Pull direction: client→daemon ack.
    let pull_ack = ClientPullMessage {
        payload: Some(client_pull_message::Payload::DataPlaneResizeAck(
            DataPlaneResizeAck {
                epoch: 3,
                effective_stream_count: 2,
                accepted: false,
            },
        )),
    };
    let old_pull = OldClientPullMessage::decode(pull_ack.encode_to_vec().as_slice())
        .expect("frame must still parse on an old daemon");
    assert!(old_pull.payload.is_none());
}

// ─── New ↔ new round trips ───────────────────────────────────────────

#[test]
fn capacity_profile_round_trips_all_fields() {
    let profile = full_capacity_profile();
    let decoded = CapacityProfile::decode(profile.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, profile);
    assert_eq!(decoded.drain_class(), DrainClass::SsdNvme);
}

#[test]
fn data_plane_resize_round_trips_between_new_peers() {
    let add = DataPlaneResize {
        op: DataPlaneResizeOp::Add as i32,
        epoch: 7,
        target_stream_count: 9,
        sub_token: (0u8..16).collect(),
    };
    let decoded = DataPlaneResize::decode(add.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, add);
    assert_eq!(decoded.op(), DataPlaneResizeOp::Add);

    let remove = DataPlaneResize {
        op: DataPlaneResizeOp::Remove as i32,
        epoch: 8,
        target_stream_count: 4,
        sub_token: Vec::new(),
    };
    let decoded = DataPlaneResize::decode(remove.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, remove);

    let ack = DataPlaneResizeAck {
        epoch: 7,
        effective_stream_count: 9,
        accepted: true,
    };
    let decoded = DataPlaneResizeAck::decode(ack.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, ack);
}

#[test]
fn new_negotiation_round_trips_dial_fields() {
    let new = DataTransferNegotiation {
        tcp_port: 1,
        one_time_token: "t".into(),
        tcp_fallback: false,
        stream_count: 2,
        receiver_capacity: Some(full_capacity_profile()),
        resize_enabled: true,
        epoch0_sub_token: vec![9u8; 16],
    };
    let decoded = DataTransferNegotiation::decode(new.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, new);
}
