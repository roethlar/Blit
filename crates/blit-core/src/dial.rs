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
//! - **Cheap dials** — `chunk_bytes`, `prefetch_count`: atomics the
//!   tuner steps mid-transfer. Consumers read them when a session,
//!   pipeline, or fallback batch is set up, so a step takes effect for
//!   sockets/batches started afterwards (epoch-N resize adds, the next
//!   gRPC-fallback batch) — existing sessions keep their snapshot.
//! - **Connect-time dials** — `tcp_buffer_bytes`, buffer-pool sizing:
//!   read when a socket/pool is built; changes affect sockets opened
//!   afterwards (no setsockopt on live sockets this slice).
//! - **Stream membership** — epoch 0 starts at the receiver-bounded
//!   floor; later epochs move the live count one stream at a time from
//!   production telemetry, always within the profile-clamped safety
//!   limit.
//!
//! This replaces the size-keyed `determine_remote_tuning` static
//! ladder: byte and stream concurrency both start conservatively and
//! ramp on evidence instead of guessing from workload shape.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::generated::CapacityProfile;
pub use crate::remote::transfer::progress::SharedStreamProbes;
use crate::remote::transfer::progress::{StreamId, StreamProbe, StreamProbeRegistry};

const MIB: usize = 1024 * 1024;

/// Conservative epoch-0 values.
pub const DIAL_FLOOR_CHUNK_BYTES: usize = 16 * MIB;
pub const DIAL_FLOOR_PREFETCH: usize = 4;
pub const DIAL_FLOOR_INITIAL_STREAMS: usize = 4;

/// Default byte-dial ceilings. Stream concurrency has a separate
/// receiver safety limit; it is a bound, not a tuning target.
pub const DIAL_CEILING_CHUNK_BYTES: usize = 64 * MIB;
pub const DIAL_CEILING_PREFETCH: usize = 32;
pub const DIAL_DEFAULT_STREAM_LIMIT: usize = 32;
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

/// Why one live-dial sample did or did not change stream membership.
/// Pending and settlement state is reported by [`DialLifecycleReason`], not
/// reconstructed as a policy sample that production never takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialDecisionReason {
    Idle,
    Rebaseline,
    Hysteresis,
    CheapUp,
    CheapDown,
    Sustain,
    Cooldown,
    Bound,
    Add,
    Remove,
}

impl DialDecisionReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Rebaseline => "rebaseline",
            Self::Hysteresis => "hysteresis",
            Self::CheapUp => "cheap-up",
            Self::CheapDown => "cheap-down",
            Self::Sustain => "sustain",
            Self::Cooldown => "cooldown",
            Self::Bound => "bound",
            Self::Add => "add",
            Self::Remove => "remove",
        }
    }
}

/// Why a proposal lifecycle event was emitted. Keeping this typed beside the
/// policy removes string reconstruction from session-phase adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialLifecycleReason {
    Pending,
    Add,
    Remove,
    Refused,
}

impl DialLifecycleReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Add => "add",
            Self::Remove => "remove",
            Self::Refused => "refused",
        }
    }
}

/// The aggregate passed to the single policy step. Production fills every
/// field from live probes; deterministic tests replace only that sampling
/// operation. Invalid samples are membership/counter rebaselines and are
/// deliberately applied as no-signal ticks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DialSampleInput {
    pub(crate) delta_bytes: u64,
    pub(crate) delta_blocked_nanos: u64,
    pub(crate) elapsed_nanos: u64,
    pub(crate) sampled_streams: usize,
    pub(crate) blocked_ratio: f64,
    pub(crate) valid: bool,
}

impl DialSampleInput {
    #[cfg(test)]
    fn injected(delta_bytes: u64, blocked_ratio: f64) -> Self {
        let elapsed_nanos = 1_000_000_000_u64;
        let delta_blocked_nanos =
            (blocked_ratio.clamp(0.0, 1.0) * elapsed_nanos as f64).round() as u64;
        Self {
            delta_bytes,
            delta_blocked_nanos,
            elapsed_nanos,
            sampled_streams: 1,
            blocked_ratio: blocked_ratio.clamp(0.0, 1.0),
            valid: true,
        }
    }

    fn rebaseline(elapsed_nanos: u64, sampled_streams: usize) -> Self {
        Self {
            delta_bytes: 0,
            delta_blocked_nanos: 0,
            elapsed_nanos,
            sampled_streams,
            blocked_ratio: 0.0,
            valid: false,
        }
    }
}

/// Numeric, path-free snapshot emitted for each policy sample.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DialPolicyDecision {
    #[cfg(test)]
    pub(crate) reason: DialDecisionReason,
    pub(crate) proposal: Option<ResizeProposal>,
}

/// Numeric, path-free snapshot emitted for each observed policy sample.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DialSampleDecision {
    pub(crate) input: DialSampleInput,
    pub(crate) reason: DialDecisionReason,
    pub(crate) proposal: Option<ResizeProposal>,
    pub(crate) settled_epoch: u32,
    pub(crate) live_streams: usize,
    pub(crate) peak_streams: usize,
    pub(crate) receiver_ceiling: usize,
    pub(crate) chunk_bytes: usize,
    pub(crate) prefetch_count: usize,
    pub(crate) tcp_buffer_bytes: usize,
}

/// Lifecycle records share the same optional observer as policy samples.
#[derive(Debug, Clone, Copy)]
pub(crate) enum DialObservationEvent {
    Sample(DialSampleDecision),
    Pending {
        proposal: ResizeProposal,
        reason: DialLifecycleReason,
        live_streams: usize,
        peak_streams: usize,
        receiver_ceiling: usize,
    },
    Settlement {
        proposal: ResizeProposal,
        reason: DialLifecycleReason,
        accepted: bool,
        live_streams: usize,
        peak_streams: usize,
        receiver_ceiling: usize,
    },
}

#[derive(Clone)]
pub(crate) struct DialObserver(Arc<dyn Fn(DialObservationEvent) + Send + Sync + 'static>);

impl std::fmt::Debug for DialObserver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DialObserver")
    }
}

impl DialObserver {
    pub(crate) fn new(observe: impl Fn(DialObservationEvent) + Send + Sync + 'static) -> Self {
        Self(Arc::new(observe))
    }

    fn emit(&self, event: DialObservationEvent) {
        (self.0)(event);
    }
}

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
        max_streams: DIAL_DEFAULT_STREAM_LIMIT as u32,
        drain_rate_bytes_per_sec: 0,
        max_chunk_bytes: DIAL_CEILING_CHUNK_BYTES as u64,
        max_inflight_bytes: (DIAL_CEILING_CHUNK_BYTES * DIAL_CEILING_PREFETCH) as u64,
    }
}

/// Resolve the receiver's advertised stream ceiling with the wire
/// contract's `0 = unknown` semantics. Both the SOURCE-owned dial and the
/// DESTINATION's resize admission must call this one function; otherwise a
/// destination-initiated session can interpret the same profile as a
/// one-stream cap while its source interprets it as the default safety limit.
pub fn receiver_stream_ceiling(profile: Option<&CapacityProfile>) -> usize {
    profile
        .and_then(|capacity| (capacity.max_streams > 0).then_some(capacity.max_streams as usize))
        .unwrap_or(DIAL_DEFAULT_STREAM_LIMIT)
        .clamp(1, DIAL_DEFAULT_STREAM_LIMIT)
}

/// Canonical epoch-0 stream count: the conservative floor, lowered only
/// when the byte receiver advertises a smaller non-zero safety limit.
pub fn receiver_initial_streams(profile: Option<&CapacityProfile>) -> usize {
    DIAL_FLOOR_INITIAL_STREAMS.min(receiver_stream_ceiling(profile))
}

/// Serialized wire-epoch state. Resize proposals are rare (at most one per
/// control-lane round trip), so one short critical section is preferable to
/// a split-atomic check/CAS sequence that can reopen a refused transfer or
/// reuse an epoch after an intervening settlement.
#[derive(Debug, Default)]
struct ResizeEpochState {
    settled_epoch: u32,
    pending: Option<ResizeProposal>,
    refused: bool,
}

impl ResizeEpochState {
    fn settle(&mut self, epoch: u32, accepted: bool) -> bool {
        if self.pending.map(|pending| pending.epoch) != Some(epoch) || epoch == 0 {
            return false;
        }
        if !accepted {
            self.refused = true;
        }
        self.settled_epoch = epoch;
        self.pending = None;
        true
    }
}

struct ResizeEpochGuard<'a> {
    inner: std::sync::MutexGuard<'a, ResizeEpochState>,
    #[cfg(test)]
    acquisition: usize,
}

impl std::ops::Deref for ResizeEpochGuard<'_> {
    type Target = ResizeEpochState;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for ResizeEpochGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
impl ResizeEpochGuard<'_> {
    fn acquisition(&self) -> usize {
        self.acquisition
    }
}

#[cfg(test)]
struct ResizeTickTestHook {
    entered: std::sync::Barrier,
    release: std::sync::Barrier,
    entered_acquisition: AtomicUsize,
    claimed_acquisition: AtomicUsize,
}

#[cfg(test)]
impl std::fmt::Debug for ResizeTickTestHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResizeTickTestHook")
    }
}

#[cfg(test)]
impl ResizeTickTestHook {
    fn new() -> Self {
        Self {
            entered: std::sync::Barrier::new(2),
            release: std::sync::Barrier::new(2),
            entered_acquisition: AtomicUsize::new(0),
            claimed_acquisition: AtomicUsize::new(0),
        }
    }
}

#[cfg(test)]
struct ResizeSettleTestHook {
    observed: std::sync::Barrier,
    contended: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
impl std::fmt::Debug for ResizeSettleTestHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResizeSettleTestHook")
    }
}

#[cfg(test)]
impl ResizeSettleTestHook {
    fn new() -> Self {
        Self {
            observed: std::sync::Barrier::new(2),
            contended: std::sync::atomic::AtomicBool::new(false),
        }
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
    // ── ue-r2-2 resize state (all epochs are the wire's monotonic
    // resize ids; 0 is reserved for the initial stream set) ──────────
    /// Settled live stream count. Epoch-0 write is
    /// `set_negotiated_streams`; later writes come from
    /// `resize_settled` on an accepted epoch.
    live_streams: AtomicUsize,
    /// Highest settled logical membership reached in this transfer.
    /// This is deliberately separate from the final count and from the
    /// cumulative number of sockets ever opened.
    peak_streams: AtomicUsize,
    /// Last settled epoch, in-flight proposal, and terminal-refusal bit.
    /// These fields form one arbitration state: observing/claiming them
    /// separately permits an ABA race across a concurrent settlement.
    resize_epochs: Mutex<ResizeEpochState>,
    #[cfg(test)]
    resize_tick_test_hook: Mutex<Option<Arc<ResizeTickTestHook>>>,
    #[cfg(test)]
    resize_settle_test_hook: Mutex<Option<Arc<ResizeSettleTestHook>>>,
    #[cfg(test)]
    resize_lock_sequence: AtomicUsize,
    /// Resize-eligible ticks since the last settle (cooldown clock).
    ticks_since_settle: AtomicU32,
    /// Consecutive same-direction tick counter: positive = "pipe clean
    /// AND cheap dials maxed" streak, negative = "blocked AND cheap
    /// dials floored" streak. Any other tick resets it.
    resize_sustain: AtomicI32,
    /// Wakes deterministic drivers after a valid resize settlement.
    resize_settlement_notify: tokio::sync::Notify,
    // Profile-clamped bounds, fixed at construction.
    ceiling_chunk_bytes: usize,
    ceiling_prefetch: usize,
    ceiling_max_streams: usize,
    ceiling_tcp_buffer_bytes: usize,
    observer: Option<DialObserver>,
}

impl TransferDial {
    fn lock_resize_epochs(&self) -> ResizeEpochGuard<'_> {
        ResizeEpochGuard {
            inner: self
                .resize_epochs
                .lock()
                .expect("resize epoch state poisoned"),
            #[cfg(test)]
            acquisition: self
                .resize_lock_sequence
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
        }
    }

    /// Conservative start with default byte ceilings and stream safety limit.
    pub fn conservative() -> Self {
        Self::conservative_within(None)
    }

    /// Conservative start bounded by the receiver's advertised
    /// capacity profile. Per the `ue-r2-1b` contract, `0`/absent
    /// fields mean UNKNOWN and keep the (already conservative)
    /// default safety bound — never "unlimited". A profile can only lower
    /// bounds, never raise them above the defaults this slice.
    pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
        let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
        let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
        let ceiling_streams = receiver_stream_ceiling(profile);
        let initial_streams = receiver_initial_streams(profile);
        let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
        if let Some(profile) = profile {
            if profile.max_chunk_bytes > 0 {
                ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
            }
            if profile.max_inflight_bytes > 0 {
                // The in-flight budget bounds the CHUNK ceiling first
                // (codex ue-r2-1e F1: with max_chunk unknown, a budget
                // smaller than one chunk must still be honored — floor
                // 64 KiB, matching the session's minimum buffer), then
                // prefetch so prefetch × chunk stays within budget
                // (floor of 1 so work still moves).
                let inflight = profile.max_inflight_bytes as usize;
                ceiling_chunk =
                    ceiling_chunk.min(inflight.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR));
                let by_inflight = (inflight / ceiling_chunk.max(1)).max(1);
                ceiling_prefetch = ceiling_prefetch.min(by_inflight);
            }
        }
        Self {
            chunk_bytes: AtomicUsize::new(DIAL_FLOOR_CHUNK_BYTES.min(ceiling_chunk)),
            prefetch_count: AtomicUsize::new(DIAL_FLOOR_PREFETCH.min(ceiling_prefetch)),
            tcp_buffer_bytes: AtomicUsize::new(0),
            initial_streams: AtomicUsize::new(initial_streams),
            live_streams: AtomicUsize::new(initial_streams),
            peak_streams: AtomicUsize::new(initial_streams),
            resize_epochs: Mutex::new(ResizeEpochState::default()),
            #[cfg(test)]
            resize_tick_test_hook: Mutex::new(None),
            #[cfg(test)]
            resize_settle_test_hook: Mutex::new(None),
            #[cfg(test)]
            resize_lock_sequence: AtomicUsize::new(0),
            ticks_since_settle: AtomicU32::new(0),
            resize_sustain: AtomicI32::new(0),
            resize_settlement_notify: tokio::sync::Notify::new(),
            ceiling_chunk_bytes: ceiling_chunk,
            ceiling_prefetch,
            ceiling_max_streams: ceiling_streams,
            ceiling_tcp_buffer_bytes: ceiling_tcp,
            observer: None,
        }
    }

    /// Attach the optional aggregate observer before sharing this dial.
    /// A disabled trace leaves this `None`, so the hot path performs no
    /// event allocation or role-dependent work.
    pub(crate) fn with_observer(mut self, observer: Option<DialObserver>) -> Self {
        self.observer = observer;
        self
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
        self.peak_streams.store(clamped, Ordering::Relaxed);
        clamped
    }

    // ── ue-r2-2 resize policy ────────────────────────────────────────

    /// The settled live stream count (epoch-0 negotiation, then each
    /// accepted resize).
    pub fn live_streams(&self) -> usize {
        self.live_streams.load(Ordering::Relaxed)
    }

    /// Highest settled logical membership reached by this transfer.
    pub fn peak_streams(&self) -> usize {
        self.peak_streams.load(Ordering::Relaxed)
    }

    /// Last settled resize epoch (0 = only the initial stream set).
    pub fn resize_epoch(&self) -> u32 {
        self.resize_epochs
            .lock()
            .expect("resize epoch state poisoned")
            .settled_epoch
    }

    /// True while a proposal is awaiting `resize_settled`.
    pub fn resize_pending(&self) -> bool {
        self.resize_epochs
            .lock()
            .expect("resize epoch state poisoned")
            .pending
            .is_some()
    }

    pub(crate) fn resize_refused(&self) -> bool {
        self.resize_epochs
            .lock()
            .expect("resize epoch state poisoned")
            .refused
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
        self.resize_tick_decision(delta_bytes, blocked_ratio)
            .and_then(|(proposal, _)| proposal)
    }

    fn resize_tick_decision(
        &self,
        delta_bytes: u64,
        blocked_ratio: f64,
    ) -> Option<(Option<ResizeProposal>, DialDecisionReason)> {
        // Keep eligibility, direction, live count, and epoch claim in the
        // same critical section as settlement. Otherwise a resize can
        // settle/reset cooldown between signal calculation and claim, and a
        // stale tuner decision can immediately open the next epoch.
        let mut state = self.lock_resize_epochs();
        if state.refused || state.pending.is_some() {
            return None;
        }
        #[cfg(test)]
        let test_hook = self
            .resize_tick_test_hook
            .lock()
            .expect("resize tick test hook poisoned")
            .clone();
        #[cfg(test)]
        if let Some(hook) = test_hook.as_ref() {
            hook.entered_acquisition
                .store(state.acquisition(), Ordering::SeqCst);
            hook.entered.wait();
            hook.release.wait();
        }
        let ticks = self
            .ticks_since_settle
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        if delta_bytes == 0 {
            self.resize_sustain.store(0, Ordering::Relaxed);
            return Some((None, DialDecisionReason::Idle));
        }
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
        if sustain == 0 {
            return Some((None, DialDecisionReason::Hysteresis));
        }
        if ticks < RESIZE_COOLDOWN_TICKS {
            return Some((None, DialDecisionReason::Cooldown));
        }
        let add = if sustain >= RESIZE_SUSTAIN_TICKS {
            true
        } else if sustain <= -RESIZE_SUSTAIN_TICKS {
            false
        } else {
            return Some((None, DialDecisionReason::Sustain));
        };
        let live = self.live_streams.load(Ordering::Relaxed).max(1);
        let target = if add {
            (live + 1).min(self.ceiling_max_streams.max(1))
        } else {
            live.saturating_sub(1).max(1)
        };
        if target == live {
            // Already at the bound in the wanted direction.
            self.resize_sustain.store(0, Ordering::Relaxed);
            return Some((None, DialDecisionReason::Bound));
        }
        let Some(epoch) = state.settled_epoch.checked_add(1) else {
            return Some((None, DialDecisionReason::Bound));
        };
        let proposal = ResizeProposal {
            epoch,
            target_streams: target,
            add: target > live,
        };
        state.pending = Some(proposal);
        #[cfg(test)]
        if let Some(hook) = test_hook.as_ref() {
            hook.claimed_acquisition
                .store(state.acquisition(), Ordering::SeqCst);
        }
        self.resize_sustain.store(0, Ordering::Relaxed);
        Some((
            Some(proposal),
            if proposal.add {
                DialDecisionReason::Add
            } else {
                DialDecisionReason::Remove
            },
        ))
    }

    /// Settle the in-flight proposal with what ACTUALLY happened.
    /// Production passes `accepted = true` only after local membership reaches
    /// the proposal target; a post-ack membership failure faults the session
    /// instead of publishing a partial settlement. `accepted = false` leaves
    /// the live count untouched, consumes the refused epoch, and permanently
    /// disables further proposals on this transfer. Stale epochs (not the
    /// pending one) are ignored. Either way the cooldown clock restarts.
    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
        #[cfg(test)]
        let mut state = {
            let hook = self
                .resize_settle_test_hook
                .lock()
                .expect("resize settle test hook poisoned")
                .clone();
            if let Some(hook) = hook {
                match self.resize_epochs.try_lock() {
                    Ok(state) => {
                        hook.contended.store(false, Ordering::SeqCst);
                        hook.observed.wait();
                        state
                    }
                    Err(std::sync::TryLockError::WouldBlock) => {
                        hook.contended.store(true, Ordering::SeqCst);
                        hook.observed.wait();
                        self.resize_epochs
                            .lock()
                            .expect("resize epoch state poisoned")
                    }
                    Err(std::sync::TryLockError::Poisoned(_)) => {
                        panic!("resize epoch state poisoned")
                    }
                }
            } else {
                self.resize_epochs
                    .lock()
                    .expect("resize epoch state poisoned")
            }
        };
        #[cfg(not(test))]
        let mut state = self
            .resize_epochs
            .lock()
            .expect("resize epoch state poisoned");
        let proposal = state.pending.filter(|pending| pending.epoch == epoch);
        let settled = self.resize_settled_locked(&mut state, epoch, effective_streams, accepted);
        let observation = if settled {
            let live_streams = self.live_streams();
            if accepted {
                self.peak_streams.fetch_max(live_streams, Ordering::Relaxed);
            }
            proposal.map(|proposal| {
                let reason = if !accepted {
                    DialLifecycleReason::Refused
                } else if proposal.add {
                    DialLifecycleReason::Add
                } else {
                    DialLifecycleReason::Remove
                };
                (
                    proposal,
                    reason,
                    live_streams,
                    self.peak_streams(),
                    self.ceiling_max_streams,
                )
            })
        } else {
            None
        };
        drop(state);
        if let (Some(observer), Some((proposal, reason, live_streams, peak_streams, ceiling))) =
            (&self.observer, observation)
        {
            observer.emit(DialObservationEvent::Settlement {
                proposal,
                reason,
                accepted,
                live_streams,
                peak_streams,
                receiver_ceiling: ceiling,
            });
        }
        if settled {
            // Release the tuner only after the optional observer records this
            // epoch. Otherwise its next sample can publish epoch N+1 before
            // epoch N's settlement reaches the trace.
            self.resize_settlement_notify.notify_waiters();
        }
    }

    fn resize_settled_locked(
        &self,
        state: &mut ResizeEpochState,
        epoch: u32,
        effective_streams: usize,
        accepted: bool,
    ) -> bool {
        if state.pending.map(|pending| pending.epoch) != Some(epoch) || epoch == 0 {
            return false;
        }
        self.ticks_since_settle.store(0, Ordering::Relaxed);
        self.resize_sustain.store(0, Ordering::Relaxed);
        if accepted {
            let clamped = effective_streams.clamp(1, self.ceiling_max_streams.max(1));
            self.live_streams.store(clamped, Ordering::Relaxed);
        }
        // A refused request was still an observed wire epoch. Consuming it
        // keeps future/duplicate traffic monotonic even though live count
        // remains unchanged.
        let settled = state.settle(epoch, accepted);
        debug_assert!(settled);
        settled
    }

    /// Wait until `epoch` has settled. Registering the notification before
    /// checking the monotonic settled epoch avoids missing a settlement in
    /// the check-to-await window.
    pub(crate) async fn wait_for_resize_settlement(&self, epoch: u32) {
        loop {
            let notified = self.resize_settlement_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.resize_epoch() >= epoch {
                return;
            }
            notified.await;
        }
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

    /// Apply one already-aggregated production sample. Busy samples step
    /// cheap dials before evaluating stream resize; idle samples are no
    /// signal but still reset resize sustain. Timed sampling and
    /// deterministic test injection share this exact policy path.
    #[cfg(test)]
    pub(crate) fn apply_sample(
        &self,
        delta_bytes: u64,
        blocked_ratio: f64,
    ) -> Option<ResizeProposal> {
        self.apply_sample_input(DialSampleInput::injected(delta_bytes, blocked_ratio))
            .and_then(|decision| decision.proposal)
    }

    pub(crate) fn apply_sample_input(&self, input: DialSampleInput) -> Option<DialPolicyDecision> {
        let (proposal, mut reason) = if !input.valid {
            let (proposal, _) = self.resize_tick_decision(0, 0.0)?;
            (proposal, DialDecisionReason::Rebaseline)
        } else if input.delta_bytes == 0 {
            self.resize_tick_decision(0, 0.0)?
        } else {
            let cheap_reason = if input.blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO {
                self.step_up_cheap_dials()
                    .then_some(DialDecisionReason::CheapUp)
            } else if input.blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO {
                self.step_down_cheap_dials()
                    .then_some(DialDecisionReason::CheapDown)
            } else {
                None
            };
            let (proposal, resize_reason) =
                self.resize_tick_decision(input.delta_bytes, input.blocked_ratio)?;
            (proposal, cheap_reason.unwrap_or(resize_reason))
        };
        if proposal.is_some() {
            reason = if proposal.is_some_and(|proposal| proposal.add) {
                DialDecisionReason::Add
            } else {
                DialDecisionReason::Remove
            };
        }
        let policy = DialPolicyDecision {
            #[cfg(test)]
            reason,
            proposal,
        };
        if let Some(observer) = &self.observer {
            let decision = DialSampleDecision {
                input,
                reason,
                proposal,
                settled_epoch: self.resize_epoch(),
                live_streams: self.live_streams(),
                peak_streams: self.peak_streams(),
                receiver_ceiling: self.ceiling_max_streams,
                chunk_bytes: self.chunk_bytes(),
                prefetch_count: self.prefetch_count(),
                tcp_buffer_bytes: self.tcp_buffer_bytes().unwrap_or(0),
            };
            observer.emit(DialObservationEvent::Sample(decision));
            if let Some(proposal) = proposal {
                observer.emit(DialObservationEvent::Pending {
                    proposal,
                    reason: DialLifecycleReason::Pending,
                    live_streams: decision.live_streams,
                    peak_streams: decision.peak_streams,
                    receiver_ceiling: decision.receiver_ceiling,
                });
            }
        }
        Some(policy)
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

#[derive(Clone, Copy)]
struct ProbeCounters {
    bytes_sent: u64,
    write_blocked_nanos: u64,
}

fn probe_counters(probes: &StreamProbeRegistry) -> HashMap<StreamId, ProbeCounters> {
    probes
        .values()
        .map(|probe| {
            let snapshot = probe.snapshot();
            (
                snapshot.id,
                ProbeCounters {
                    bytes_sent: snapshot.bytes_sent,
                    write_blocked_nanos: snapshot.write_blocked_nanos,
                },
            )
        })
        .collect()
}

/// Return one valid per-stream delta sample and replace the baseline.
/// Membership changes, individual counter resets, and disagreement with
/// the dial's settled live count are all no-signal ticks. Per-stream
/// baselines prevent survivor progress from masking a removed/reset member.
fn sample_probe_deltas(
    probes: &StreamProbeRegistry,
    baselines: &mut HashMap<StreamId, ProbeCounters>,
    expected_streams: usize,
) -> Option<(u64, u64, usize)> {
    let current = probe_counters(probes);
    let membership_matches = current.len() == expected_streams
        && current.len() == baselines.len()
        && current.keys().all(|id| baselines.contains_key(id));
    let counters_monotonic = membership_matches
        && current.iter().all(|(id, now)| {
            let before = baselines
                .get(id)
                .expect("matching membership must have a baseline");
            now.bytes_sent >= before.bytes_sent
                && now.write_blocked_nanos >= before.write_blocked_nanos
        });
    let sample = counters_monotonic.then(|| {
        current.iter().fold(
            (0u64, 0u64, current.len()),
            |(blocked, bytes, streams), (id, now)| {
                let before = baselines
                    .get(id)
                    .expect("matching membership must have a baseline");
                (
                    blocked.saturating_add(
                        now.write_blocked_nanos
                            .saturating_sub(before.write_blocked_nanos),
                    ),
                    bytes.saturating_add(now.bytes_sent.saturating_sub(before.bytes_sent)),
                    streams,
                )
            },
        )
    });
    *baselines = current;
    sample
}

/// Spawn the live tuner for one transfer (ue-r2-1e): every
/// [`DIAL_TUNER_TICK`] it sums the PR1 per-stream `write_blocked`
/// telemetry and steps the dial's cheap dials. Holds only a `Weak` to
/// the dial, so it self-terminates within one tick of the transfer
/// dropping its dial; callers may also abort the handle for prompt
/// shutdown (`MultiStreamSender::finish` does).
pub fn spawn_dial_tuner(
    dial: &Arc<TransferDial>,
    probes: Vec<StreamProbe>,
) -> tokio::task::JoinHandle<()> {
    let probes = StreamProbeRegistry::from_probes(probes);
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
    let mut baselines = {
        let probes = probes.lock().expect("probe registry poisoned");
        probe_counters(&probes)
    };
    tokio::spawn(async move {
        let mut last_tick = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(DIAL_TUNER_TICK).await;
            let Some(dial) = weak.upgrade() else { return };
            let (sample, observed_streams) = {
                let probes = probes.lock().expect("probe registry poisoned");
                (
                    sample_probe_deltas(&probes, &mut baselines, dial.live_streams()),
                    probes.len(),
                )
            };
            let elapsed = last_tick.elapsed();
            last_tick = tokio::time::Instant::now();
            let Some((delta_blocked, delta_bytes, streams)) = sample else {
                if resize_tx.is_some() {
                    let _ = dial.apply_sample_input(DialSampleInput::rebaseline(
                        elapsed.as_nanos().min(u64::MAX as u128) as u64,
                        observed_streams,
                    ));
                }
                continue;
            };
            // codex ue-r2-1e F2: an idle tick (no bytes moved) is NO
            // SIGNAL, not a clean pipe — stepping up during manifest /
            // preparation stalls would ramp without evidence and break
            // the conservative-start contract. ue-r2-2 review (panel
            // F3): the idle tick must still reach `resize_tick` so a
            // sustain streak cannot survive a stall — "consecutive
            // busy ticks" means consecutive.
            let ratio = blocked_ratio(delta_blocked, elapsed, streams);
            if let Some(tx) = &resize_tx {
                let Some(decision) = dial.apply_sample_input(DialSampleInput {
                    delta_bytes,
                    delta_blocked_nanos: delta_blocked,
                    elapsed_nanos: elapsed.as_nanos().min(u64::MAX as u128) as u64,
                    sampled_streams: streams,
                    blocked_ratio: ratio,
                    valid: true,
                }) else {
                    if dial.resize_refused() {
                        return;
                    }
                    continue;
                };
                if let Some(proposal) = decision.proposal {
                    if tx.send(proposal).is_err() {
                        // Controller gone (transfer tearing down):
                        // release the pending slot so the dial state
                        // stays honest for late readers.
                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
                        return;
                    }
                    dial.wait_for_resize_settlement(proposal.epoch).await;
                    if dial.resize_refused() {
                        return;
                    }
                }
            } else if delta_bytes > 0 {
                dial.apply_tick(ratio);
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
    fn conservative_start_uses_the_receiver_bounded_floor() {
        let dial = TransferDial::conservative();
        assert_eq!(dial.chunk_bytes(), 16 * MIB);
        assert_eq!(dial.prefetch_count(), 4);
        assert_eq!(dial.tcp_buffer_bytes(), None);
        assert_eq!(dial.initial_streams(), 4);
        assert_eq!(receiver_initial_streams(None), 4);
        assert_eq!(receiver_initial_streams(Some(&profile(2, 0, 0))), 2);
        assert_eq!(receiver_initial_streams(Some(&profile(0, 0, 0))), 4);
        let bounded = TransferDial::conservative_within(Some(&profile(2, 0, 0)));
        assert_eq!(bounded.initial_streams(), 2);
        assert_eq!(bounded.live_streams(), 2);
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
        assert_eq!(dial.ceiling_max_streams(), DIAL_DEFAULT_STREAM_LIMIT);
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
        assert_eq!(generous.ceiling_max_streams(), DIAL_DEFAULT_STREAM_LIMIT);
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

    #[test]
    fn sampler_rebaselines_on_membership_change_even_when_totals_increase() {
        use crate::remote::transfer::progress::{StreamId, StreamProbe};

        let retired = StreamProbe::new(StreamId(0));
        let survivor = StreamProbe::new(StreamId(1));
        retired.record_bytes(1_000);
        survivor.record_bytes(1_000);
        let mut registry = StreamProbeRegistry::from_probes(vec![retired, survivor.clone()]);
        let mut baselines = probe_counters(&registry);

        // Survivor progress is larger than the retired member's total, so
        // aggregate counters would still rise and hide the membership swap.
        survivor.record_bytes(10_000);
        assert!(registry.unregister(StreamId(0)).is_some());
        let added = StreamProbe::new(StreamId(2));
        added.record_bytes(1);
        assert!(registry.register(added.clone()));
        assert_eq!(
            sample_probe_deltas(&registry, &mut baselines, 2),
            None,
            "identity change is a no-signal rebaseline"
        );

        survivor.record_bytes(5);
        added.record_bytes(7);
        survivor.add_write_blocked(13);
        added.add_write_blocked(17);
        assert_eq!(
            sample_probe_deltas(&registry, &mut baselines, 2),
            Some((30, 12, 2)),
            "the next stable interval is measured per member"
        );
    }

    #[test]
    fn sampler_rebaselines_while_registry_and_live_counts_disagree() {
        use crate::remote::transfer::progress::{StreamId, StreamProbe};

        let probe = StreamProbe::new(StreamId(0));
        let registry = StreamProbeRegistry::from_probes(vec![probe.clone()]);
        let mut baselines = probe_counters(&registry);
        probe.record_bytes(10);
        assert_eq!(sample_probe_deltas(&registry, &mut baselines, 2), None);
        probe.record_bytes(7);
        assert_eq!(
            sample_probe_deltas(&registry, &mut baselines, 1),
            Some((0, 7, 1)),
            "matching membership resumes from the mismatch baseline"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn tuner_steps_up_on_clean_telemetry_and_exits_when_dial_drops() {
        use crate::remote::transfer::progress::{StreamId, StreamProbe};
        let dial = TransferDial::conservative().shared();
        dial.set_negotiated_streams(2);
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

    #[tokio::test(start_paused = true)]
    async fn production_tuner_folds_blocked_telemetry_into_remove() {
        use crate::remote::transfer::progress::{StreamId, StreamProbe};

        let dial = TransferDial::conservative().shared();
        dial.set_negotiated_streams(2);
        let first = StreamProbe::new(StreamId(0));
        let second = StreamProbe::new(StreamId(1));
        let registry = Arc::new(std::sync::Mutex::new(StreamProbeRegistry::from_probes(
            vec![first.clone(), second],
        )));
        let (resize_tx, mut resize_rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = spawn_dial_tuner_with_resize(&dial, registry, Some(resize_tx));
        tokio::task::yield_now().await;

        for _ in 0..RESIZE_COOLDOWN_TICKS {
            first.record_bytes(1024);
            first.add_write_blocked(1_000_000_000);
            tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
            for _ in 0..16 {
                tokio::task::yield_now().await;
            }
        }

        let proposal = resize_rx
            .try_recv()
            .expect("sustained production blocked telemetry proposes REMOVE");
        assert_eq!(
            proposal,
            ResizeProposal {
                epoch: 1,
                target_streams: 1,
                add: false,
            }
        );
        dial.resize_settled(proposal.epoch, proposal.target_streams, true);
        drop(dial);
        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
        handle.await.expect("production tuner exits cleanly");
    }

    #[test]
    fn negotiated_streams_clamp_to_the_profile_ceiling() {
        let dial = TransferDial::conservative_within(Some(&profile(6, 0, 0)));
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
        assert_eq!(dial.apply_sample(1024, 0.0), None);
        assert_eq!(dial.apply_sample(0, 0.0), None, "idle resets");
        assert_eq!(dial.apply_sample(1024, 0.0), None, "streak restarted");
        // clean → in-band → clean: same reset.
        assert_eq!(dial.apply_sample(1024, 0.15), None, "in-band resets");
        assert_eq!(dial.apply_sample(1024, 0.0), None, "streak restarted");
        assert!(dial.apply_sample(1024, 0.0).is_some(), "streak completes");
    }

    fn policy_sample(dial: &TransferDial, bytes: u64, ratio: f64) -> DialPolicyDecision {
        dial.apply_sample_input(DialSampleInput {
            delta_bytes: bytes,
            delta_blocked_nanos: 0,
            elapsed_nanos: 1,
            sampled_streams: dial.live_streams(),
            blocked_ratio: ratio,
            valid: true,
        })
        .expect("active policy emits a decision")
    }

    #[test]
    fn observer_reason_strings_are_complete_and_exact() {
        let sample_reasons = [
            DialDecisionReason::Idle,
            DialDecisionReason::Rebaseline,
            DialDecisionReason::Hysteresis,
            DialDecisionReason::CheapUp,
            DialDecisionReason::CheapDown,
            DialDecisionReason::Sustain,
            DialDecisionReason::Cooldown,
            DialDecisionReason::Bound,
            DialDecisionReason::Add,
            DialDecisionReason::Remove,
        ]
        .map(DialDecisionReason::as_str);
        assert_eq!(
            sample_reasons,
            [
                "idle",
                "rebaseline",
                "hysteresis",
                "cheap-up",
                "cheap-down",
                "sustain",
                "cooldown",
                "bound",
                "add",
                "remove",
            ]
        );
        assert_eq!(
            [
                DialLifecycleReason::Pending,
                DialLifecycleReason::Add,
                DialLifecycleReason::Remove,
                DialLifecycleReason::Refused,
            ]
            .map(DialLifecycleReason::as_str),
            ["pending", "add", "remove", "refused"]
        );
    }

    #[test]
    fn observed_policy_names_every_sample_reason_and_lifecycle_quiesces() {
        let basic = TransferDial::conservative();
        let rebaseline = basic
            .apply_sample_input(DialSampleInput {
                valid: false,
                ..DialSampleInput::injected(0, 0.0)
            })
            .expect("rebaseline is an observed policy decision");
        assert_eq!(rebaseline.reason, DialDecisionReason::Rebaseline);
        assert_eq!(
            policy_sample(&basic, 0, 0.0).reason,
            DialDecisionReason::Idle
        );
        assert_eq!(
            policy_sample(&basic, 1024, 0.15).reason,
            DialDecisionReason::Hysteresis
        );
        assert_eq!(
            policy_sample(&basic, 1024, 0.0).reason,
            DialDecisionReason::CheapUp
        );
        assert_eq!(
            policy_sample(&basic, 1024, 1.0).reason,
            DialDecisionReason::CheapDown
        );

        let add = TransferDial::conservative();
        while add.step_up_cheap_dials() {}
        assert_eq!(
            policy_sample(&add, 1024, 0.0).reason,
            DialDecisionReason::Cooldown
        );
        burn_cooldown(&add);
        assert_eq!(
            policy_sample(&add, 1024, 0.0).reason,
            DialDecisionReason::Sustain
        );
        let proposed = policy_sample(&add, 1024, 0.0);
        assert_eq!(proposed.reason, DialDecisionReason::Add);
        let proposal = proposed.proposal.expect("ADD proposal");
        assert!(
            add.apply_sample_input(DialSampleInput::injected(1024, 0.0))
                .is_none(),
            "pending membership is a lifecycle wait, not a sample reason"
        );
        add.resize_settled(proposal.epoch, proposal.target_streams, true);
        assert_eq!(add.live_streams(), 5);
        assert_eq!(add.peak_streams(), 5);

        let bound = TransferDial::conservative_within(Some(&profile(4, 0, 0)));
        while bound.step_up_cheap_dials() {}
        burn_cooldown(&bound);
        assert_eq!(
            policy_sample(&bound, 1024, 0.0).reason,
            DialDecisionReason::Sustain
        );
        assert_eq!(
            policy_sample(&bound, 1024, 0.0).reason,
            DialDecisionReason::Bound
        );

        let remove = TransferDial::conservative();
        remove.set_negotiated_streams(2);
        burn_cooldown(&remove);
        assert_eq!(
            policy_sample(&remove, 1024, 1.0).reason,
            DialDecisionReason::Sustain
        );
        let proposed = policy_sample(&remove, 1024, 1.0);
        assert_eq!(proposed.reason, DialDecisionReason::Remove);
        let proposal = proposed.proposal.expect("REMOVE proposal");
        remove.resize_settled(proposal.epoch, remove.live_streams(), false);
        assert!(
            remove
                .apply_sample_input(DialSampleInput::injected(1024, 1.0))
                .is_none(),
            "terminal refusal is a lifecycle settlement, not a sample reason"
        );
        assert_eq!(remove.live_streams(), 2);
        assert_eq!(remove.peak_streams(), 2);
    }

    #[test]
    fn observer_emits_pending_and_exact_settlement_with_peak() {
        let events: Arc<Mutex<Vec<DialObservationEvent>>> = Arc::default();
        let captured = Arc::clone(&events);
        let dial =
            TransferDial::conservative().with_observer(Some(DialObserver::new(move |event| {
                captured.lock().unwrap().push(event);
            })));
        while dial.step_up_cheap_dials() {}
        burn_cooldown(&dial);
        assert_eq!(
            policy_sample(&dial, 1024, 0.0).reason,
            DialDecisionReason::Sustain
        );
        let proposal = policy_sample(&dial, 1024, 0.0)
            .proposal
            .expect("ADD proposal");
        dial.resize_settled(proposal.epoch, proposal.target_streams, true);

        let events = events.lock().unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            DialObservationEvent::Pending {
                proposal: pending,
                reason: DialLifecycleReason::Pending,
                ..
            }
                if *pending == proposal
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DialObservationEvent::Settlement {
                proposal: settled,
                reason: DialLifecycleReason::Add,
                accepted: true,
                live_streams: 5,
                peak_streams: 5,
                ..
            } if *settled == proposal
        )));
    }

    #[test]
    fn observer_names_remove_and_refused_settlements() {
        let events: Arc<Mutex<Vec<DialObservationEvent>>> = Arc::default();
        let captured = Arc::clone(&events);
        let observer = DialObserver::new(move |event| {
            captured.lock().unwrap().push(event);
        });

        let remove = TransferDial::conservative().with_observer(Some(observer.clone()));
        remove.set_negotiated_streams(2);
        burn_cooldown(&remove);
        assert_eq!(
            policy_sample(&remove, 1024, 1.0).reason,
            DialDecisionReason::Sustain
        );
        let removed = policy_sample(&remove, 1024, 1.0)
            .proposal
            .expect("REMOVE proposal");
        remove.resize_settled(removed.epoch, removed.target_streams, true);

        let refused = TransferDial::conservative().with_observer(Some(observer));
        refused.set_negotiated_streams(2);
        burn_cooldown(&refused);
        assert_eq!(
            policy_sample(&refused, 1024, 1.0).reason,
            DialDecisionReason::Sustain
        );
        let declined = policy_sample(&refused, 1024, 1.0)
            .proposal
            .expect("REMOVE proposal to refuse");
        refused.resize_settled(declined.epoch, refused.live_streams(), false);

        let events = events.lock().unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            DialObservationEvent::Settlement {
                proposal,
                reason: DialLifecycleReason::Remove,
                accepted: true,
                live_streams: 1,
                peak_streams: 2,
                ..
            } if *proposal == removed
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DialObservationEvent::Settlement {
                proposal,
                reason: DialLifecycleReason::Refused,
                accepted: false,
                live_streams: 2,
                peak_streams: 2,
                ..
            } if *proposal == declined
        )));
    }

    #[test]
    fn resize_refusal_is_terminal_consumes_epoch_and_ignores_stale_settles() {
        let dial = TransferDial::conservative();
        dial.set_negotiated_streams(4);
        while dial.step_up_cheap_dials() {}
        burn_cooldown(&dial);
        assert_eq!(dial.resize_tick(1024, 0.0), None);
        let proposal = dial.resize_tick(1024, 0.0).expect("proposes");

        // A stale/foreign epoch must not clear the pending slot.
        dial.resize_settled(proposal.epoch + 7, 9, true);
        assert!(dial.resize_pending(), "stale settle ignored");

        // Refusal: pending clears and live count stays put, but the wire
        // epoch is consumed and the sample controller becomes terminal.
        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
        assert!(!dial.resize_pending());
        assert_eq!(dial.live_streams(), 4);
        assert_eq!(dial.resize_epoch(), proposal.epoch);
        assert_eq!(
            dial.apply_sample(1024, 0.0),
            None,
            "the tuner must not retry a refused resize"
        );

        // Duplicate/stale settlements cannot reopen the terminal policy or
        // rewrite the consumed epoch/live count.
        dial.resize_settled(proposal.epoch, 5, true);
        assert_eq!(dial.resize_epoch(), proposal.epoch);
        assert_eq!(dial.live_streams(), 4);
        assert_eq!(dial.apply_sample(1024, 0.0), None);
    }

    #[tokio::test]
    async fn settlement_wait_does_not_miss_an_already_settled_epoch() {
        let dial = TransferDial::conservative();
        dial.set_negotiated_streams(4);
        while dial.step_up_cheap_dials() {}
        burn_cooldown(&dial);
        assert_eq!(dial.resize_tick(1024, 0.0), None);
        let proposal = dial.resize_tick(1024, 0.0).expect("proposes");
        dial.resize_settled(proposal.epoch, proposal.target_streams, true);

        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            dial.wait_for_resize_settlement(proposal.epoch),
        )
        .await
        .expect("settled-before-wait cannot miss the notification");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn settlement_observer_precedes_waiter_notification() {
        let observer_entered = Arc::new(std::sync::Barrier::new(2));
        let observer_release = Arc::new(std::sync::Barrier::new(2));
        let observer = DialObserver::new({
            let observer_entered = Arc::clone(&observer_entered);
            let observer_release = Arc::clone(&observer_release);
            move |event| {
                if matches!(event, DialObservationEvent::Settlement { .. }) {
                    observer_entered.wait();
                    observer_release.wait();
                }
            }
        });
        let dial = Arc::new(TransferDial::conservative().with_observer(Some(observer)));
        dial.set_negotiated_streams(4);
        while dial.step_up_cheap_dials() {}
        burn_cooldown(&dial);
        assert_eq!(dial.resize_tick(1024, 0.0), None, "sustain tick 1");
        let proposal = dial.resize_tick(1024, 0.0).expect("proposes epoch 1");

        let notification = dial.resize_settlement_notify.notified();
        tokio::pin!(notification);
        notification.as_mut().enable();
        let settler = {
            let dial = Arc::clone(&dial);
            std::thread::spawn(move || {
                dial.resize_settled(proposal.epoch, proposal.target_streams, true)
            })
        };
        observer_entered.wait();

        let notified_before_observer_returned =
            tokio::time::timeout(std::time::Duration::from_millis(50), notification.as_mut())
                .await
                .is_ok();
        observer_release.wait();
        if !notified_before_observer_returned {
            tokio::time::timeout(std::time::Duration::from_secs(1), notification.as_mut())
                .await
                .expect("settlement notification follows observer return");
        }
        settler.join().unwrap();

        assert!(
            !notified_before_observer_returned,
            "waiter notification must not overtake settlement observation"
        );
    }

    #[test]
    fn resize_epoch_lock_serializes_producers_across_settlement() {
        // The tuner lock spans eligibility, direction, live count, and claim:
        // pause after it acquires the lock and prove the arbitration mutex is
        // unavailable until the tick continues. This deterministically guards
        // the stale-decision window the re-review found.
        let accepted = Arc::new(TransferDial::conservative());
        while accepted.step_up_cheap_dials() {}
        burn_cooldown(&accepted);
        assert_eq!(accepted.resize_tick(1024, 0.0), None, "sustain tick 1");
        let hook = Arc::new(ResizeTickTestHook::new());
        *accepted
            .resize_tick_test_hook
            .lock()
            .expect("resize tick test hook poisoned") = Some(Arc::clone(&hook));
        let tuner = {
            let accepted = Arc::clone(&accepted);
            std::thread::spawn(move || accepted.resize_tick(1024, 0.0))
        };
        hook.entered.wait();
        let lock_spans_tick = matches!(
            accepted.resize_epochs.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        );
        // Start the matching accepted settlement while the tuner is paused.
        // The settlement hook reports only after `try_lock` has actually
        // observed epoch arbitration, so this does not rely on a pre-call
        // channel signal or scheduler timing.
        let settle_hook = Arc::new(ResizeSettleTestHook::new());
        *accepted
            .resize_settle_test_hook
            .lock()
            .expect("resize settle test hook poisoned") = Some(Arc::clone(&settle_hook));
        let settler = {
            let accepted = Arc::clone(&accepted);
            std::thread::spawn(move || accepted.resize_settled(1, 2, true))
        };
        settle_hook.observed.wait();
        let settlement_contended = settle_hook.contended.load(Ordering::SeqCst);
        hook.release.wait();
        *accepted
            .resize_tick_test_hook
            .lock()
            .expect("resize tick test hook poisoned") = None;
        assert!(lock_spans_tick, "tuner released arbitration before claim");
        let first = tuner.join().unwrap().expect("tuner owns epoch 1");
        let entered_acquisition = hook.entered_acquisition.load(Ordering::SeqCst);
        let claimed_acquisition = hook.claimed_acquisition.load(Ordering::SeqCst);
        assert_ne!(entered_acquisition, 0, "tuner acquisition was recorded");
        assert_eq!(
            claimed_acquisition, entered_acquisition,
            "tuner released and reacquired arbitration before claim"
        );
        assert_eq!(first.epoch, 1);
        settler.join().unwrap();
        *accepted
            .resize_settle_test_hook
            .lock()
            .expect("resize settle test hook poisoned") = None;
        assert!(
            settlement_contended,
            "accepted settlement reached arbitration without contention"
        );
        assert_eq!(accepted.live_streams(), 2, "accepted settlement applied");
        assert!(!accepted.resize_pending(), "accepted epoch settled");
        assert_eq!(
            accepted.resize_tick(1024, 0.0),
            None,
            "accepted settlement resets cooldown"
        );

        // Refusal crossing: another sample begins while the settlement lock
        // is held. Once refusal is recorded and the lock is released, the
        // waiter must observe terminal state and may not claim epoch 2.
        let refused = Arc::new(TransferDial::conservative());
        while refused.step_up_cheap_dials() {}
        burn_cooldown(&refused);
        assert_eq!(refused.resize_tick(1024, 0.0), None, "sustain tick 1");
        let first = refused.resize_tick(1024, 0.0).expect("tuner owns epoch 1");
        let mut refused_state = refused
            .resize_epochs
            .lock()
            .expect("resize epoch state poisoned");
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let sample = {
            let refused = Arc::clone(&refused);
            std::thread::spawn(move || {
                started_tx.send(()).unwrap();
                refused.apply_sample(1024, 0.0)
            })
        };
        started_rx.recv().unwrap();
        assert!(refused.resize_settled_locked(&mut refused_state, first.epoch, 1, false));
        drop(refused_state);
        assert_eq!(sample.join().unwrap(), None, "refusal is terminal");
        assert_eq!(refused.resize_epoch(), first.epoch);
        assert!(!refused.resize_pending());
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
        let probes = [StreamProbe::new(StreamId(0)), StreamProbe::new(StreamId(1))];
        let registry: SharedStreamProbes =
            Arc::new(std::sync::Mutex::new(StreamProbeRegistry::from_probes(
                probes
                    .iter()
                    .map(|probe| StreamProbe::from_telemetry(probe.id(), probe.telemetry()))
                    .collect(),
            )));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = spawn_dial_tuner_with_resize(&dial, Arc::clone(&registry), Some(tx));
        tokio::task::yield_now().await;

        // Enough busy ticks to pass cooldown + sustain: every tick
        // records fresh bytes with zero blocked time.
        let mut proposal = None;
        for _ in 0..(RESIZE_COOLDOWN_TICKS + RESIZE_SUSTAIN_TICKS as u32 + 2) {
            probes[0].record_bytes(1024);
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

        dial.resize_settled(proposal.epoch, proposal.target_streams, true);
        drop(dial);
        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("tuner exits after the dial drops")
            .expect("tuner does not panic");
    }
}
