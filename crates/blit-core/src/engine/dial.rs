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

use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
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

/// Resize policy (`ue-r2-2`): streams are the EXPENSIVE dial — a step
/// costs a control round-trip plus a TCP connect — so they move only
/// after the cheap dials are pinned at a bound and the signal has held
/// for [`RESIZE_SUSTAIN_TICKS`] consecutive ticks, and never within
/// [`RESIZE_COOLDOWN_TICKS`] of the previous settle. One stream per
/// epoch (the wire carries one `sub_token` per ADD).
pub const RESIZE_COOLDOWN_TICKS: u32 = 4;
pub const RESIZE_SUSTAIN_TICKS: i32 = 2;

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
    // ── ue-r2-2 resize state (all epochs are the wire's monotonic
    // resize ids; 0 is reserved for the initial stream set) ──────────
    /// Settled live stream count. Epoch-0 write is
    /// `set_negotiated_streams`; later writes come from
    /// `resize_settled` on an accepted epoch.
    live_streams: AtomicUsize,
    /// Last settled epoch (0 until the first accepted resize).
    resize_epoch: AtomicU32,
    /// In-flight proposal's epoch; 0 = none. While non-zero no new
    /// proposal is produced (the wire is idempotent but overlapping
    /// epochs would complicate sub-token registration).
    pending_epoch: AtomicU32,
    /// Resize-eligible ticks since the last settle (cooldown clock).
    ticks_since_settle: AtomicU32,
    /// Consecutive same-direction tick counter: positive = "pipe clean
    /// AND cheap dials maxed" streak, negative = "blocked AND cheap
    /// dials floored" streak. Any other tick resets it.
    resize_sustain: AtomicI32,
    // Profile-clamped bounds, fixed at construction.
    ceiling_chunk_bytes: usize,
    ceiling_prefetch: usize,
    ceiling_max_streams: usize,
    ceiling_tcp_buffer_bytes: usize,
}

/// One engine resize decision (`ue-r2-2`). The adapter that owns the
/// control stream turns this into a wire `DataPlaneResize` (the engine
/// stays wire-type-free here on purpose) and MUST eventually call
/// [`TransferDial::resize_settled`] for the epoch — with what actually
/// happened — or no further proposals are produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeProposal {
    /// The wire epoch for this change (`resize_epoch() + 1`).
    pub epoch: u32,
    /// Absolute desired live count (idempotent, per the proto).
    pub target_streams: usize,
    /// Convenience: `target_streams > live` at proposal time.
    pub add: bool,
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
            live_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
            resize_epoch: AtomicU32::new(0),
            pending_epoch: AtomicU32::new(0),
            ticks_since_settle: AtomicU32::new(0),
            resize_sustain: AtomicI32::new(0),
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
    /// (clamped to the dial's ceiling). This is the epoch-0 settle:
    /// it also seeds `live_streams`, the baseline every `ue-r2-2`
    /// resize proposal steps from.
    pub fn set_negotiated_streams(&self, streams: usize) -> usize {
        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
        self.initial_streams.store(clamped, Ordering::Relaxed);
        self.live_streams.store(clamped, Ordering::Relaxed);
        clamped
    }

    // ── ue-r2-2 resize policy ────────────────────────────────────────

    /// The settled live stream count (epoch-0 negotiation, then each
    /// accepted resize).
    pub fn live_streams(&self) -> usize {
        self.live_streams.load(Ordering::Relaxed)
    }

    /// Last settled resize epoch (0 = only the initial stream set).
    pub fn resize_epoch(&self) -> u32 {
        self.resize_epoch.load(Ordering::Relaxed)
    }

    /// True while a proposal is awaiting `resize_settled`.
    pub fn resize_pending(&self) -> bool {
        self.pending_epoch.load(Ordering::Relaxed) != 0
    }

    fn cheap_dials_maxed(&self) -> bool {
        self.chunk_bytes.load(Ordering::Relaxed) >= self.ceiling_chunk_bytes
            && self.prefetch_count.load(Ordering::Relaxed) >= self.ceiling_prefetch
    }

    fn cheap_dials_floored(&self) -> bool {
        self.chunk_bytes.load(Ordering::Relaxed)
            <= DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes)
            && self.prefetch_count.load(Ordering::Relaxed)
                <= DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch).max(1)
    }

    /// One resize-eligible tuner tick. Streams move only as the LAST
    /// escalation step in either direction: the cheap dials must
    /// already be pinned at their ceiling (ADD) or floor (REMOVE), the
    /// signal must hold for [`RESIZE_SUSTAIN_TICKS`] consecutive
    /// ticks, at least [`RESIZE_COOLDOWN_TICKS`] must have passed
    /// since the last settle, and no proposal may be in flight. Idle
    /// ticks (`delta_bytes == 0`) are no signal, matching the cheap
    /// tuner. Bounds: `1..=ceiling_max_streams` (the receiver profile
    /// folded in at construction — `CapacityProfile.max_streams` is
    /// authoritative per the proto). One stream per epoch.
    ///
    /// The caller must forward the returned proposal to the peer and
    /// call [`Self::resize_settled`] with the outcome; until then
    /// every subsequent tick returns `None`.
    pub fn resize_tick(&self, delta_bytes: u64, blocked_ratio: f64) -> Option<ResizeProposal> {
        if self.pending_epoch.load(Ordering::Relaxed) != 0 {
            return None;
        }
        let ticks = self
            .ticks_since_settle
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        if delta_bytes == 0 {
            self.resize_sustain.store(0, Ordering::Relaxed);
            return None;
        }
        let live = self.live_streams.load(Ordering::Relaxed).max(1);
        let sustain = if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO && self.cheap_dials_maxed() {
            let prev = self.resize_sustain.load(Ordering::Relaxed).max(0);
            let next = prev.saturating_add(1);
            self.resize_sustain.store(next, Ordering::Relaxed);
            next
        } else if blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO && self.cheap_dials_floored() {
            let prev = self.resize_sustain.load(Ordering::Relaxed).min(0);
            let next = prev.saturating_sub(1);
            self.resize_sustain.store(next, Ordering::Relaxed);
            next
        } else {
            self.resize_sustain.store(0, Ordering::Relaxed);
            0
        };
        if ticks < RESIZE_COOLDOWN_TICKS {
            return None;
        }
        let target = if sustain >= RESIZE_SUSTAIN_TICKS {
            (live + 1).min(self.ceiling_max_streams.max(1))
        } else if sustain <= -RESIZE_SUSTAIN_TICKS {
            live.saturating_sub(1).max(1)
        } else {
            return None;
        };
        if target == live {
            // Already at the bound in the wanted direction.
            self.resize_sustain.store(0, Ordering::Relaxed);
            return None;
        }
        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
        self.pending_epoch.store(epoch, Ordering::Relaxed);
        self.resize_sustain.store(0, Ordering::Relaxed);
        Some(ResizeProposal {
            epoch,
            target_streams: target,
            add: target > live,
        })
    }

    /// Settle the in-flight proposal with what ACTUALLY happened:
    /// `effective_streams` is the live count now in effect (from the
    /// peer's ack, or the local count if a post-ack dial failed and
    /// nothing changed). `accepted = false` leaves the live count
    /// untouched. Stale epochs (not the pending one) are ignored.
    /// Either way the cooldown clock restarts.
    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
        if self.pending_epoch.load(Ordering::Relaxed) != epoch || epoch == 0 {
            return;
        }
        self.pending_epoch.store(0, Ordering::Relaxed);
        self.ticks_since_settle.store(0, Ordering::Relaxed);
        self.resize_sustain.store(0, Ordering::Relaxed);
        if accepted {
            let clamped = effective_streams.clamp(1, self.ceiling_max_streams.max(1));
            self.live_streams.store(clamped, Ordering::Relaxed);
            self.resize_epoch.store(epoch, Ordering::Relaxed);
        }
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

/// Workload-shape-aware initial stream proposal (`ue-r2-1f`): the
/// end that KNOWS the workload shape proposes a starting stream
/// count — file count matters as much as bytes (many small files
/// parallelize on per-file overhead even at low byte totals). On push
/// that is the receiving daemon (it has the manifest) clamped to its
/// own advertised ceiling; on pull_sync it is the sending daemon (it
/// enumerated the source) clamped to the CLIENT's advertised
/// `receiver_capacity.max_streams` (`ue-r2-1g`) — either way the byte
/// receiver's profile is the bound. Table carried over verbatim from
/// the daemon push `desired_streams` ladder it retires (the ladder
/// the old `tuning.rs` doc said "wins"), now engine-owned. The
/// sender's dial clamps again on its side (`set_negotiated_streams`).
/// Live mid-transfer stream changes arrive with `ue-r2-2` resize.
pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
    if file_count == 0 {
        return 1;
    }
    let proposal: u32 = if total_bytes >= 32 * 1024 * 1024 * 1024 || file_count >= 200_000 {
        16
    } else if total_bytes >= 8 * 1024 * 1024 * 1024 || file_count >= 80_000 {
        12
    } else if total_bytes >= 2 * 1024 * 1024 * 1024 || file_count >= 50_000 {
        10
    } else if total_bytes >= 512 * 1024 * 1024 || file_count >= 10_000 {
        8
    } else if total_bytes >= 128 * 1024 * 1024 || file_count >= 2_000 {
        4
    } else if total_bytes >= 32 * 1024 * 1024 || file_count >= 256 {
        2
    } else {
        1
    };
    proposal.min(ceiling.max(1) as u32)
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

/// Growable per-transfer probe registry (`ue-r2-2`): resize adds a
/// probe when a stream joins and removes it when one retires, and the
/// tuner samples whatever is live each tick. Plain std mutex — locked
/// only for a snapshot fold every 500ms and on resize events.
pub type SharedStreamProbes =
    Arc<std::sync::Mutex<Vec<crate::remote::transfer::progress::StreamProbe>>>;

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
    spawn_dial_tuner_with_resize(dial, Arc::new(std::sync::Mutex::new(probes)), None)
}

/// `ue-r2-2` tuner: same cheap-dial stepping, but over a growable
/// probe registry, plus the stream-resize policy when `resize_tx` is
/// provided — each [`TransferDial::resize_tick`] proposal is forwarded
/// to the adapter that owns the control stream (unbounded so a
/// momentarily busy adapter cannot lose a proposal while the dial
/// holds it pending). Callers without resize pass `None` and get
/// exactly the ue-r2-1e behavior.
pub fn spawn_dial_tuner_with_resize(
    dial: &Arc<TransferDial>,
    probes: SharedStreamProbes,
    resize_tx: Option<tokio::sync::mpsc::UnboundedSender<ResizeProposal>>,
) -> tokio::task::JoinHandle<()> {
    let weak = Arc::downgrade(dial);
    tokio::spawn(async move {
        let mut last_blocked: u64 = 0;
        let mut last_bytes: u64 = 0;
        let mut last_tick = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(DIAL_TUNER_TICK).await;
            let Some(dial) = weak.upgrade() else { return };
            let (blocked, bytes, streams) = {
                let probes = probes.lock().expect("probe registry poisoned");
                let (b, n) = probes.iter().fold((0u64, 0u64), |(b, n), p| {
                    let snap = p.snapshot();
                    (b + snap.write_blocked_nanos, n + snap.bytes_sent)
                });
                (b, n, probes.len())
            };
            let elapsed = last_tick.elapsed();
            last_tick = tokio::time::Instant::now();
            // A retired stream leaves the registry, so the monotonic
            // sums can shrink across a REMOVE. Re-baseline and treat
            // the tick as no-signal rather than reading a bogus delta.
            if blocked < last_blocked || bytes < last_bytes {
                last_blocked = blocked;
                last_bytes = bytes;
                if let Some(tx) = &resize_tx {
                    let _ = tx; // no proposal possible on a no-signal tick
                    dial.resize_tick(0, 0.0);
                }
                continue;
            }
            let delta_blocked = blocked.saturating_sub(last_blocked);
            let delta_bytes = bytes.saturating_sub(last_bytes);
            last_blocked = blocked;
            last_bytes = bytes;
            // codex ue-r2-1e F2: an idle tick (no bytes moved) is NO
            // SIGNAL, not a clean pipe — stepping up during manifest /
            // preparation stalls would ramp without evidence and break
            // the conservative-start contract. ue-r2-2 review (panel
            // F3): the idle tick must still reach `resize_tick` so a
            // sustain streak cannot survive a stall — "consecutive
            // busy ticks" means consecutive.
            if delta_bytes == 0 {
                if resize_tx.is_some() {
                    dial.resize_tick(0, 0.0);
                }
                continue;
            }
            let ratio = blocked_ratio(delta_blocked, elapsed, streams);
            dial.apply_tick(ratio);
            if let Some(tx) = &resize_tx {
                if let Some(proposal) = dial.resize_tick(delta_bytes, ratio) {
                    if tx.send(proposal).is_err() {
                        // Controller gone (transfer tearing down):
                        // release the pending slot so the dial state
                        // stays honest for late readers.
                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
                    }
                }
            }
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
    fn initial_stream_proposal_matches_the_retired_daemon_table() {
        const MIB64: u64 = 1024 * 1024;
        const GIB: u64 = 1024 * MIB64;
        // Empty need-list → 1 (the old ladder's empty-guard).
        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
        // Byte-keyed tiers: exact lower boundaries AND just-below each
        // (codex ue-r2-1f: representative values would miss a doubled
        // threshold).
        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
        // File-count keys fire independently of bytes.
        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
        // Ceiling clamps the proposal (receiver profile authority).
        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
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
        let probes = [StreamProbe::new(StreamId(0)), StreamProbe::new(StreamId(1))];
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
        assert_eq!(dial.live_streams(), 3, "negotiation seeds the live count");
    }

    // ── ue-r2-2 resize policy ────────────────────────────────────────

    /// Burn the cooldown with busy, in-band ticks that move no dials.
    fn burn_cooldown(dial: &TransferDial) {
        for _ in 0..RESIZE_COOLDOWN_TICKS {
            assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band tick holds");
        }
    }

    #[test]
    fn resize_add_requires_maxed_cheap_dials_sustain_and_cooldown() {
        let dial = TransferDial::conservative();
        dial.set_negotiated_streams(4);

        // Clean pipe but cheap dials NOT at ceiling: never proposes,
        // no matter how long it holds (cheap dials escalate first).
        for _ in 0..16 {
            assert_eq!(dial.resize_tick(1024, 0.0), None);
        }

        // Pin the cheap dials at the ceiling, then a clean signal must
        // still survive the sustain requirement before proposing.
        while dial.step_up_cheap_dials() {}
        assert_eq!(dial.resize_tick(1024, 0.0), None, "sustain tick 1");
        let proposal = dial
            .resize_tick(1024, 0.0)
            .expect("sustained clean signal at maxed dials proposes");
        assert_eq!(
            proposal,
            ResizeProposal {
                epoch: 1,
                target_streams: 5,
                add: true
            }
        );
        assert!(dial.resize_pending());

        // In flight: no further proposals regardless of signal.
        for _ in 0..8 {
            assert_eq!(dial.resize_tick(1024, 0.0), None, "pending blocks");
        }

        // Accepted settle: live moves, epoch advances, cooldown blocks
        // the immediate next proposal even under a perfect signal.
        dial.resize_settled(1, 5, true);
        assert_eq!(dial.live_streams(), 5);
        assert_eq!(dial.resize_epoch(), 1);
        assert!(!dial.resize_pending());
        for _ in 0..(RESIZE_COOLDOWN_TICKS - 1) {
            assert_eq!(dial.resize_tick(1024, 0.0), None, "cooldown holds");
        }
        // Cooldown expired and the clean streak has been building the
        // whole time — the next clean tick proposes epoch 2.
        let next = dial.resize_tick(1024, 0.0).expect("epoch 2 proposes");
        assert_eq!(next.epoch, 2);
        assert_eq!(next.target_streams, 6);
    }

    #[test]
    fn resize_remove_requires_floored_cheap_dials_and_floors_at_one() {
        let dial = TransferDial::conservative();
        dial.set_negotiated_streams(2);
        burn_cooldown(&dial);

        // Blocked pipe with cheap dials at the floor (conservative
        // start IS the floor): two sustained ticks propose a drop.
        assert_eq!(dial.resize_tick(1024, 0.9), None, "sustain tick 1");
        let proposal = dial.resize_tick(1024, 0.9).expect("sustained block drops");
        assert_eq!(
            proposal,
            ResizeProposal {
                epoch: 1,
                target_streams: 1,
                add: false
            }
        );
        dial.resize_settled(1, 1, true);
        assert_eq!(dial.live_streams(), 1);

        // At one stream, a blocked pipe can never drop to zero.
        burn_cooldown(&dial);
        for _ in 0..8 {
            assert_eq!(dial.resize_tick(1024, 0.9), None, "floor at 1");
        }
    }

    #[test]
    fn resize_signal_interruptions_and_idle_reset_sustain() {
        let dial = TransferDial::conservative();
        dial.set_negotiated_streams(4);
        while dial.step_up_cheap_dials() {}
        burn_cooldown(&dial);

        // clean → idle → clean: the idle tick resets the streak, so
        // the second clean tick is streak 1, not 2.
        assert_eq!(dial.resize_tick(1024, 0.0), None);
        assert_eq!(dial.resize_tick(0, 0.0), None, "idle resets");
        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
        // clean → in-band → clean: same reset.
        assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band resets");
        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
        assert!(dial.resize_tick(1024, 0.0).is_some(), "streak completes");
    }

    #[test]
    fn resize_refusal_keeps_live_count_and_stale_settles_are_ignored() {
        let dial = TransferDial::conservative();
        dial.set_negotiated_streams(4);
        while dial.step_up_cheap_dials() {}
        burn_cooldown(&dial);
        assert_eq!(dial.resize_tick(1024, 0.0), None);
        let proposal = dial.resize_tick(1024, 0.0).expect("proposes");

        // A stale/foreign epoch must not clear the pending slot.
        dial.resize_settled(proposal.epoch + 7, 9, true);
        assert!(dial.resize_pending(), "stale settle ignored");

        // Refusal: pending clears, live count and epoch stay put.
        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
        assert!(!dial.resize_pending());
        assert_eq!(dial.live_streams(), 4);
        assert_eq!(dial.resize_epoch(), 0, "refused epoch never settles");
    }

    #[test]
    fn resize_target_clamps_to_the_profile_ceiling() {
        let dial = TransferDial::conservative_within(Some(&profile(4, 0, 0)));
        dial.set_negotiated_streams(4); // already at the profile ceiling
        while dial.step_up_cheap_dials() {}
        burn_cooldown(&dial);
        for _ in 0..8 {
            assert_eq!(
                dial.resize_tick(1024, 0.0),
                None,
                "cannot add past the receiver's advertised ceiling"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn tuner_forwards_resize_proposals_over_the_shared_registry() {
        use crate::remote::transfer::progress::{StreamId, StreamProbe};
        let dial = TransferDial::conservative().shared();
        dial.set_negotiated_streams(2);
        while dial.step_up_cheap_dials() {}
        let probe = StreamProbe::new(StreamId(0));
        let registry: SharedStreamProbes =
            Arc::new(std::sync::Mutex::new(vec![StreamProbe::from_telemetry(
                probe.id(),
                probe.telemetry(),
            )]));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = spawn_dial_tuner_with_resize(&dial, Arc::clone(&registry), Some(tx));
        tokio::task::yield_now().await;

        // Enough busy ticks to pass cooldown + sustain: every tick
        // records fresh bytes with zero blocked time.
        let mut proposal = None;
        for _ in 0..(RESIZE_COOLDOWN_TICKS + RESIZE_SUSTAIN_TICKS as u32 + 2) {
            probe.record_bytes(1024);
            tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
            for _ in 0..16 {
                tokio::task::yield_now().await;
            }
            if let Ok(p) = rx.try_recv() {
                proposal = Some(p);
                break;
            }
        }
        let proposal = proposal.expect("tuner forwarded a resize proposal");
        assert_eq!(proposal.target_streams, 3);
        assert!(proposal.add);
        assert!(dial.resize_pending());

        drop(dial);
        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("tuner exits after the dial drops")
            .expect("tuner does not panic");
    }
}
