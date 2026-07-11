//! Wire round-trip pins for the dial-contract messages
//! (`CapacityProfile`, `DataPlaneResize`, `DataPlaneResizeAck`).
//!
//! History: ue-r2-1b landed this file as old/new MIXED-VERSION
//! compatibility tests (Old* prost replicas proving unknown-field
//! skipping in both directions). D-2026-07-05-2 then abolished
//! version tolerance outright — same-build peers only, refusal at
//! session open — and otp-10c-2 deleted the Push/PullSync messages
//! those replicas mirrored (`DataTransferNegotiation`, `PushHeader`,
//! the push/pull frame sets). The 10 mixed-version tests died with
//! the doctrine and the messages (called out in the otp-10c-2
//! finding doc); what remains are the same-build encode/decode
//! sanity pins for the surviving messages.

use blit_core::generated::{
    CapacityProfile, DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp, DrainClass,
};
use prost::Message;

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

#[test]
fn capacity_profile_round_trips_all_fields() {
    let profile = full_capacity_profile();
    let decoded = CapacityProfile::decode(profile.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, profile);
    assert_eq!(decoded.drain_class(), DrainClass::SsdNvme);
}

#[test]
fn data_plane_resize_round_trips_between_peers() {
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
