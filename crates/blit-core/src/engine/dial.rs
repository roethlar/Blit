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
                // Prefetch is the in-flight payload budget: bound it so
                // prefetch × chunk stays within the receiver's stated
                // in-flight ceiling (floor of 1 so work still moves).
                let by_inflight =
                    (profile.max_inflight_bytes as usize / ceiling_chunk.max(1)).max(1);
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
    fn negotiated_streams_clamp_to_the_profile_ceiling() {
        let dial = TransferDial::conservative_within(Some(&profile(6, 0, 0)));
        dial.allow_streams_up_to(32);
        assert_eq!(dial.max_streams(), 6, "peer cannot exceed the profile");
        assert_eq!(dial.set_negotiated_streams(16), 6);
        assert_eq!(dial.set_negotiated_streams(3), 3);
    }
}
