//! The single live transfer dial (`ue-r2-1e`, REV4 Design §4).
//!
//! Sender-owned, receiver-bounded: the byte sender constructs one
//! `TransferDial` per transfer, clamped by the receiver's advertised
//! [`CapacityProfile`] (the `ue-r2-1b` wire fields), starts at the
//! conservative floor (D-2026-06-20-1/-2: no probe phase, no
//! size-gated start — begin immediately and tune live), and a tuner
//! steps the cheap dials from the PR1 stream telemetry.
//!
//! Mutability model (the C-ready seam `ue-r2-2` builds on):
//! - **Live dials** — `chunk_bytes`, `prefetch_count`: atomics read at
//!   each use site (per tar-shard chunking, per queued payload batch),
//!   so a tuner step takes effect mid-transfer.
//! - **Connect-time dials** — `tcp_buffer_bytes`, buffer-pool sizing:
//!   read when a socket/pool is built; changes affect sockets opened
//!   afterwards (no setsockopt on live sockets this slice).
//! - **Negotiated once** — `initial_streams`/`max_streams`: stream
//!   count becomes live at `ue-r2-2` (DataPlaneResize); until then the
//!   dial only carries the negotiation-time value and the
//!   profile-clamped ceiling.
//!
//! This replaces the size-keyed `determine_remote_tuning` static
//! ladder: the ladder's floor tier is the dial's start, its top tier
//! is the dial's default ceiling, and everything between is reached by
//! ramping on evidence instead of guessing from `total_bytes`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::generated::CapacityProfile;

const MIB: usize = 1024 * 1024;

/// Floor (conservative start) values — the old ladder's smallest tier.
pub const DIAL_FLOOR_CHUNK_BYTES: usize = 16 * MIB;
pub const DIAL_FLOOR_PREFETCH: usize = 4;
pub const DIAL_FLOOR_INITIAL_STREAMS: usize = 4;
pub const DIAL_FLOOR_MAX_STREAMS: usize = 8;

/// Default ceilings — the old ladder's top tier (a fully ramped dial
/// matches today's best static behavior).
pub const DIAL_CEILING_CHUNK_BYTES: usize = 64 * MIB;
pub const DIAL_CEILING_PREFETCH: usize = 32;
pub const DIAL_CEILING_MAX_STREAMS: usize = 32;
pub const DIAL_CEILING_TCP_BUFFER_BYTES: usize = 8 * MIB;

/// Tuner policy (initial, deliberately simple): sampled every
/// [`DIAL_TUNER_TICK`]; below [`DIAL_STEP_UP_BLOCKED_RATIO`] blocked
/// time the pipe is not back-pressured → step up; above
/// [`DIAL_STEP_DOWN_BLOCKED_RATIO`] → step down. One step per tick
/// (hysteresis by construction).
pub const DIAL_TUNER_TICK: std::time::Duration = std::time::Duration::from_millis(500);
pub const DIAL_STEP_UP_BLOCKED_RATIO: f64 = 0.05;
pub const DIAL_STEP_DOWN_BLOCKED_RATIO: f64 = 0.30;

/// The capacity profile this host advertises when it is the byte
/// RECEIVER (ue-r2-1e: the first real sender of the ue-r2-1b wire
/// fields). Honest system facts only — fields we cannot measure yet
/// stay 0 (= unknown per the wire contract), never fabricated:
/// ceilings mirror what today's receive paths actually accept.
pub fn local_receiver_capacity() -> CapacityProfile {
    CapacityProfile {
        cpu_cores: num_cpus::get() as u32,
        drain_class: 0,
        load_percent: 0,
        max_streams: DIAL_CEILING_MAX_STREAMS as u32,
        drain_rate_bytes_per_sec: 0,
        max_chunk_bytes: DIAL_CEILING_CHUNK_BYTES as u64,
        max_inflight_bytes: (DIAL_CEILING_CHUNK_BYTES * DIAL_CEILING_PREFETCH) as u64,
    }
}

/// The one mutable tuning object for a transfer.
#[derive(Debug)]
pub struct TransferDial {
    chunk_bytes: AtomicUsize,
    prefetch_count: AtomicUsize,
    /// 0 = unset (kernel default), matching the old `Option<usize>`.
    tcp_buffer_bytes: AtomicUsize,
    initial_streams: AtomicUsize,
    max_streams: AtomicUsize,
    // Profile-clamped bounds, fixed at construction.
    ceiling_chunk_bytes: usize,
    ceiling_prefetch: usize,
    ceiling_max_streams: usize,
    ceiling_tcp_buffer_bytes: usize,
}

impl TransferDial {
    /// Conservative start with default ceilings (no receiver profile).
    pub fn conservative() -> Self {
        Self::conservative_within(None)
    }

    /// Conservative start bounded by the receiver's advertised
    /// capacity profile. Per the `ue-r2-1b` contract, `0`/absent
    /// fields mean UNKNOWN and keep the (already conservative)
    /// default ceiling — never "unlimited". A profile can only lower
    /// ceilings, never raise them above the defaults this slice.
    pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
        let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
        let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
        let mut ceiling_streams = DIAL_CEILING_MAX_STREAMS;
        let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
        if let Some(profile) = profile {
            if profile.max_chunk_bytes > 0 {
                ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
            }
            if profile.max_streams > 0 {
                ceiling_streams = ceiling_streams.min(profile.max_streams as usize);
            }
            if profile.max_inflight_bytes > 0 {
                // The in-flight budget bounds the CHUNK ceiling first
                // (codex ue-r2-1e F1: with max_chunk unknown, a budget
                // smaller than one chunk must still be honored — floor
                // 64 KiB, matching the session's minimum buffer), then
                // prefetch so prefetch × chunk stays within budget
                // (floor of 1 so work still moves).
                let inflight = profile.max_inflight_bytes as usize;
                ceiling_chunk = ceiling_chunk.min(inflight.max(64 * 1024));
                let by_inflight = (inflight / ceiling_chunk.max(1)).max(1);
                ceiling_prefetch = ceiling_prefetch.min(by_inflight);
            }
        }
        Self {
            chunk_bytes: AtomicUsize::new(DIAL_FLOOR_CHUNK_BYTES.min(ceiling_chunk)),
            prefetch_count: AtomicUsize::new(DIAL_FLOOR_PREFETCH.min(ceiling_prefetch)),
            tcp_buffer_bytes: AtomicUsize::new(0),
            initial_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
            max_streams: AtomicUsize::new(DIAL_FLOOR_MAX_STREAMS.clamp(1, ceiling_streams.max(1))),
            ceiling_chunk_bytes: ceiling_chunk,
            ceiling_prefetch,
            ceiling_max_streams: ceiling_streams,
            ceiling_tcp_buffer_bytes: ceiling_tcp,
        }
    }

    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    // ── live reads ───────────────────────────────────────────────────
    pub fn chunk_bytes(&self) -> usize {
        self.chunk_bytes.load(Ordering::Relaxed)
    }
    pub fn prefetch_count(&self) -> usize {
        self.prefetch_count.load(Ordering::Relaxed)
    }
    /// `None` = leave the kernel default (old `tcp_buffer_size`
    /// semantics). Connect-time dial.
    pub fn tcp_buffer_bytes(&self) -> Option<usize> {
        match self.tcp_buffer_bytes.load(Ordering::Relaxed) {
            0 => None,
            n => Some(n),
        }
    }
    pub fn initial_streams(&self) -> usize {
        self.initial_streams.load(Ordering::Relaxed)
    }
    /// Ceiling on the negotiated stream count (profile-clamped).
    pub fn max_streams(&self) -> usize {
        self.max_streams.load(Ordering::Relaxed)
    }
    pub fn ceiling_max_streams(&self) -> usize {
        self.ceiling_max_streams
    }

    /// Record the stream count the negotiation actually settled on
    /// (clamped to the dial's ceiling). Stream-count LIVE changes
    /// arrive with `ue-r2-2`; until then this is set-once bookkeeping
    /// that `ue-r2-2` turns into the resize target.
    pub fn set_negotiated_streams(&self, streams: usize) -> usize {
        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
        self.initial_streams.store(clamped, Ordering::Relaxed);
        clamped
    }

    /// Raise max_streams toward the ceiling (used when a peer's
    /// negotiation allows more than the floor; still profile-bounded).
    pub fn allow_streams_up_to(&self, streams: usize) {
        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
        self.max_streams.store(clamped, Ordering::Relaxed);
    }

    // ── tuner steps ──────────────────────────────────────────────────
    /// One upward step of the cheap dials: chunk ×2 toward the
    /// ceiling, prefetch +50% (at least +1) toward the ceiling, and
    /// the tcp buffer to its ceiling (affects future sockets).
    /// Returns true if anything moved.
    pub fn step_up_cheap_dials(&self) -> bool {
        let mut moved = false;
        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
        let next = (chunk.saturating_mul(2)).min(self.ceiling_chunk_bytes);
        if next > chunk {
            self.chunk_bytes.store(next, Ordering::Relaxed);
            moved = true;
        }
        let prefetch = self.prefetch_count.load(Ordering::Relaxed);
        let next = (prefetch + (prefetch / 2).max(1)).min(self.ceiling_prefetch);
        if next > prefetch {
            self.prefetch_count.store(next, Ordering::Relaxed);
            moved = true;
        }
        let tcp = self.tcp_buffer_bytes.load(Ordering::Relaxed);
        if tcp < self.ceiling_tcp_buffer_bytes {
            self.tcp_buffer_bytes
                .store(self.ceiling_tcp_buffer_bytes, Ordering::Relaxed);
            moved = true;
        }
        moved
    }

    /// One downward step toward the floors. Returns true if anything
    /// moved.
    pub fn step_down_cheap_dials(&self) -> bool {
        let mut moved = false;
        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
        let next = (chunk / 2).max(DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes));
        if next < chunk {
            self.chunk_bytes.store(next, Ordering::Relaxed);
            moved = true;
        }
        let prefetch = self.prefetch_count.load(Ordering::Relaxed);
        let next = (prefetch / 2)
            .max(DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch))
            .max(1);
        if next < prefetch {
            self.prefetch_count.store(next, Ordering::Relaxed);
            moved = true;
        }
        moved
    }

    /// One tuner tick: adjust from the observed blocked-time ratio
    /// (write-blocked nanos across streams ÷ wall nanos × streams for
    /// the tick window). Between the thresholds nothing moves
    /// (hysteresis band).
    pub fn apply_tick(&self, blocked_ratio: f64) -> bool {
        if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO {
            self.step_up_cheap_dials()
        } else if blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO {
            self.step_down_cheap_dials()
        } else {
            false
        }
    }
}

/// Blocked-time ratio for one tuner tick: the share of the tick's
/// wall-clock (× stream count) the senders spent inside socket writes.
/// 0 streams or a zero-length tick reads as "no signal" (0.0 — the
/// hysteresis band holds the dial still rather than guessing).
pub(crate) fn blocked_ratio(
    delta_blocked_nanos: u64,
    elapsed: std::time::Duration,
    streams: usize,
) -> f64 {
    let denom = elapsed.as_nanos().saturating_mul(streams as u128);
    if denom == 0 {
        return 0.0;
    }
    (delta_blocked_nanos as f64 / denom as f64).clamp(0.0, 1.0)
}

/// Spawn the live tuner for one transfer (ue-r2-1e): every
/// [`DIAL_TUNER_TICK`] it sums the PR1 per-stream `write_blocked`
/// telemetry and steps the dial's cheap dials. Holds only a `Weak` to
/// the dial, so it self-terminates within one tick of the transfer
/// dropping its dial; callers may also abort the handle for prompt
/// shutdown (`MultiStreamSender::finish` does).
pub fn spawn_dial_tuner(
    dial: &Arc<TransferDial>,
    probes: Vec<crate::remote::transfer::progress::StreamProbe>,
) -> tokio::task::JoinHandle<()> {
    let weak = Arc::downgrade(dial);
    tokio::spawn(async move {
        let mut last_blocked: u64 = 0;
        let mut last_bytes: u64 = 0;
        let mut last_tick = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(DIAL_TUNER_TICK).await;
            let Some(dial) = weak.upgrade() else { return };
            let (blocked, bytes) = probes.iter().fold((0u64, 0u64), |(b, n), p| {
                let snap = p.snapshot();
                (b + snap.write_blocked_nanos, n + snap.bytes_sent)
            });
            let elapsed = last_tick.elapsed();
            last_tick = tokio::time::Instant::now();
            let delta_blocked = blocked.saturating_sub(last_blocked);
            let delta_bytes = bytes.saturating_sub(last_bytes);
            last_blocked = blocked;
            last_bytes = bytes;
            // codex ue-r2-1e F2: an idle tick (no bytes moved) is NO
            // SIGNAL, not a clean pipe — stepping up during manifest /
            // preparation stalls would ramp without evidence and break
            // the conservative-start contract.
            if delta_bytes == 0 {
                continue;
            }
            dial.apply_tick(blocked_ratio(delta_blocked, elapsed, probes.len()));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(max_streams: u32, max_chunk: u64, max_inflight: u64) -> CapacityProfile {
        CapacityProfile {
            cpu_cores: 0,
            drain_class: 0,
            load_percent: 0,
            max_streams,
            drain_rate_bytes_per_sec: 0,
            max_chunk_bytes: max_chunk,
            max_inflight_bytes: max_inflight,
        }
    }

    #[test]
    fn conservative_start_is_the_old_floor_tier() {
        let dial = TransferDial::conservative();
        assert_eq!(dial.chunk_bytes(), 16 * MIB);
        assert_eq!(dial.prefetch_count(), 4);
        assert_eq!(dial.tcp_buffer_bytes(), None);
        assert_eq!(dial.initial_streams(), 4);
        assert_eq!(dial.max_streams(), 8);
    }

    #[test]
    fn unknown_profile_fields_keep_default_ceilings() {
        let dial = TransferDial::conservative_within(Some(&profile(0, 0, 0)));
        // Ramp fully: unknown (0) fields must not lower — or lift —
        // anything relative to the defaults.
        while dial.step_up_cheap_dials() {}
        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
        assert_eq!(dial.prefetch_count(), DIAL_CEILING_PREFETCH);
        assert_eq!(dial.tcp_buffer_bytes(), Some(DIAL_CEILING_TCP_BUFFER_BYTES));
        assert_eq!(dial.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
    }

    #[test]
    fn profile_lowers_ceilings_but_never_raises_them() {
        let dial =
            TransferDial::conservative_within(Some(&profile(4, 32 * MIB as u64, 64 * MIB as u64)));
        while dial.step_up_cheap_dials() {}
        assert_eq!(dial.chunk_bytes(), 32 * MIB, "chunk ceiling from profile");
        // 64 MiB in-flight ÷ 32 MiB chunk ceiling = 2 payload budget.
        assert_eq!(dial.prefetch_count(), 2, "prefetch bounded by max_inflight");
        assert_eq!(dial.ceiling_max_streams(), 4);

        // codex F1: an in-flight budget smaller than one chunk bounds
        // the chunk ceiling itself, even with max_chunk unknown (0).
        let tight = TransferDial::conservative_within(Some(&profile(0, 0, 8 * MIB as u64)));
        while tight.step_up_cheap_dials() {}
        assert_eq!(tight.chunk_bytes(), 8 * MIB);
        assert_eq!(tight.prefetch_count(), 1);

        let generous = TransferDial::conservative_within(Some(&profile(999, u64::MAX, u64::MAX)));
        while generous.step_up_cheap_dials() {}
        assert_eq!(generous.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
        assert_eq!(generous.prefetch_count(), DIAL_CEILING_PREFETCH);
        assert_eq!(generous.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
    }

    #[test]
    fn steps_respect_floor_and_ceiling_with_hysteresis_band() {
        let dial = TransferDial::conservative();
        assert!(!dial.step_down_cheap_dials(), "already at the floor");
        assert!(dial.apply_tick(0.0), "clean telemetry steps up");
        assert_eq!(dial.chunk_bytes(), 32 * MIB);
        assert!(
            !dial.apply_tick(0.15),
            "inside the hysteresis band nothing moves"
        );
        assert!(dial.apply_tick(0.9), "blocked telemetry steps down");
        assert_eq!(dial.chunk_bytes(), 16 * MIB);
        while dial.apply_tick(0.0) {}
        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
        assert_eq!(dial.prefetch_count(), DIAL_CEILING_PREFETCH);
    }

    #[test]
    fn blocked_ratio_handles_edges() {
        use std::time::Duration;
        assert_eq!(blocked_ratio(0, Duration::from_millis(500), 4), 0.0);
        assert_eq!(blocked_ratio(1_000, Duration::ZERO, 4), 0.0, "no signal");
        assert_eq!(blocked_ratio(1_000, Duration::from_millis(500), 0), 0.0);
        let half = blocked_ratio(500_000_000, Duration::from_millis(500), 2);
        assert!((half - 0.5).abs() < 1e-9, "got {half}");
        assert_eq!(
            blocked_ratio(u64::MAX, Duration::from_nanos(1), 1),
            1.0,
            "clamped"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn tuner_steps_up_on_clean_telemetry_and_exits_when_dial_drops() {
        use crate::remote::transfer::progress::{StreamId, StreamProbe};
        let dial = TransferDial::conservative().shared();
        let probes = vec![StreamProbe::new(StreamId(0)), StreamProbe::new(StreamId(1))];
        let tuner_view: Vec<StreamProbe> = probes
            .iter()
            .map(|p| StreamProbe::from_telemetry(p.id(), p.telemetry()))
            .collect();
        let handle = spawn_dial_tuner(&dial, tuner_view);
        // Let the spawned task run to its first sleep so the timer is
        // registered before the clock moves.
        tokio::task::yield_now().await;

        // codex F2: an idle tick (no bytes moved) must NOT step.
        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
        assert_eq!(dial.chunk_bytes(), 16 * MIB, "idle tick is no signal");

        // One tick WITH byte progress and zero blocked time: step up.
        probes[0].record_bytes(1024);
        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
        for _ in 0..16 {
            if dial.chunk_bytes() > 16 * MIB {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(dial.chunk_bytes(), 32 * MIB, "stepped up once");

        // Drop the transfer's dial: the tuner must self-terminate.
        drop(dial);
        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("tuner exits after the dial drops")
            .expect("tuner does not panic");
    }

    #[test]
    fn negotiated_streams_clamp_to_the_profile_ceiling() {
        let dial = TransferDial::conservative_within(Some(&profile(6, 0, 0)));
        dial.allow_streams_up_to(32);
        assert_eq!(dial.max_streams(), 6, "peer cannot exceed the profile");
        assert_eq!(dial.set_negotiated_streams(16), 6);
        assert_eq!(dial.set_negotiated_streams(3), 3);
    }
}
