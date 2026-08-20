//! The destination-comparison thread pool.
//!
//! Deciding whether a file needs transferring costs round trips to the
//! destination. Measured on a converged 46,041-file mirror to an SMB share,
//! that decision owned ~100% of the session's wall clock
//! (`docs/bench/ls1-phase-2026-07-31/`) — the work is latency-bound, so the
//! number of checks in flight is the lever, as `rclone --checkers` treats it.
//!
//! # No user-facing knob
//!
//! The concurrency is DISCOVERED AT RUNTIME and there is no advertised flag
//! for it. Per the owner's FAST, SIMPLE, RELIABLE principle
//! (`.agents/repo-guidance.md`), SIMPLE constrains the user-facing surface:
//! tuning the program can work out for itself must not become an option the
//! user has to reason about. The hidden `--checkers` exists only so a
//! diagnostic run can pin an exact value and compare it against the adaptive
//! one.
//!
//! # Why a dedicated pool and not rayon's global one
//!
//! cr-ls1-6: a first attempt ran these blocking destination calls on rayon's
//! global pool. That pool is CPU-sized and is shared with the tar-shard apply
//! path and with concurrent daemon sessions, so a slow destination could
//! stall unrelated transfers — a hazard no single-session benchmark can show.
//! This pool is built per session, sized for I/O concurrency rather than for
//! cores, and is touched by nothing else.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use eyre::{Context, Result};

/// Checker threads when the caller does not choose.
///
/// Matches `rclone --checkers`. Chosen for the same reason: it is enough
/// concurrency to hide per-file latency on a network destination without
/// being so many that the destination server becomes the constrained
/// resource. It is deliberately NOT derived from core count — these threads
/// spend their lives blocked on I/O, not computing.
pub const DEFAULT_CHECKERS: usize = 8;

/// Upper bound on `--checkers`.
///
/// A cap exists because the failure mode past the useful range is not "no
/// faster", it is "measurably slower and harder on the destination": at 32
/// threads on the reference SMB share, per-call latency inflated ~24× while
/// throughput moved 1.34×.
pub const MAX_CHECKERS: usize = 128;

/// Conservative starting concurrency when the count is discovered at runtime.
///
/// Mirrors `dial.rs`: start at the floor, no probe phase, no guess from
/// workload shape — begin immediately and ramp on evidence.
pub const ADAPTIVE_FLOOR: usize = 2;

/// The ladder the controller climbs. Doubling keeps the number of probe
/// steps logarithmic, so a long run reaches its plateau in a handful of
/// chunks rather than crawling.
const LADDER: [usize; 7] = [1, 2, 4, 8, 16, 32, 64];

/// Runtime concurrency controller for destination comparison.
///
/// The right number of in-flight checks is a property of the DESTINATION,
/// not of the machine or the workload: a local NVMe target wants very few, a
/// high-latency SMB share wants many, and neither is knowable before the
/// transfer starts. So it is not guessed — it is measured, one chunk at a
/// time, by the same rule `dial.rs` uses for stream membership: step one
/// rung, keep the step if throughput improved, give it back if it did not.
///
/// Concurrency is bounded by SLICING each chunk, not by resizing the pool —
/// rayon pools have a fixed thread count, and rebuilding one per step would
/// churn threads for no benefit.
#[derive(Debug)]
pub struct AdaptiveCheckers {
    rung: AtomicUsize,
    /// Best files-per-second seen so far, in millifiles/sec so it can live
    /// in an atomic.
    best_rate_milli: AtomicU64,
    settled: AtomicBool,
}

impl AdaptiveCheckers {
    pub fn new() -> Self {
        let rung = LADDER
            .iter()
            .position(|value| *value >= ADAPTIVE_FLOOR)
            .unwrap_or(0);
        Self {
            rung: AtomicUsize::new(rung),
            best_rate_milli: AtomicU64::new(0),
            settled: AtomicBool::new(false),
        }
    }

    /// Concurrency to use for the next chunk.
    pub fn limit(&self) -> usize {
        LADDER[self.rung.load(Ordering::Relaxed).min(LADDER.len() - 1)]
    }

    /// Fold in one chunk's result and decide the next rung.
    ///
    /// Returns the limit that will be used next, for logging and tests.
    pub fn observe(&self, files: usize, elapsed: std::time::Duration) -> usize {
        if files == 0 || elapsed.is_zero() || self.settled.load(Ordering::Relaxed) {
            return self.limit();
        }
        let rate_milli = ((files as f64 / elapsed.as_secs_f64()) * 1000.0) as u64;
        let best = self.best_rate_milli.load(Ordering::Relaxed);
        let rung = self.rung.load(Ordering::Relaxed);

        if best == 0 {
            // First measurement: nothing to compare against, so take a step
            // and see. This is the "begin immediately" half of the rule.
            self.best_rate_milli.store(rate_milli, Ordering::Relaxed);
            if rung + 1 < LADDER.len() {
                self.rung.store(rung + 1, Ordering::Relaxed);
            }
            return self.limit();
        }

        // 5% either way is noise on a filesystem benchmark; require a real
        // improvement before spending more concurrency on the destination.
        if rate_milli > best.saturating_mul(105) / 100 {
            self.best_rate_milli.store(rate_milli, Ordering::Relaxed);
            if rung + 1 < LADDER.len() {
                self.rung.store(rung + 1, Ordering::Relaxed);
            } else {
                self.settled.store(true, Ordering::Relaxed);
            }
        } else if rate_milli < best.saturating_mul(95) / 100 {
            // The last step made things worse — give it back and stop. More
            // concurrency past this point costs the destination without
            // buying throughput, which is the failure mode measured at 32
            // shared threads (24× per-call latency for 1.34× throughput).
            if rung > 0 {
                self.rung.store(rung - 1, Ordering::Relaxed);
            }
            self.settled.store(true, Ordering::Relaxed);
        } else {
            // Flat: this rung is the plateau.
            self.settled.store(true, Ordering::Relaxed);
        }
        self.limit()
    }

    pub fn is_settled(&self) -> bool {
        self.settled.load(Ordering::Relaxed)
    }
}

impl Default for AdaptiveCheckers {
    fn default() -> Self {
        Self::new()
    }
}

/// How a session decides its comparison concurrency.
#[derive(Debug)]
pub enum CheckerPolicy {
    /// Pinned to an exact count by the hidden diagnostic flag. Not a normal
    /// run: every real invocation is [`CheckerPolicy::Adaptive`].
    Fixed(usize),
    /// Discovered at runtime from observed throughput.
    Adaptive(AdaptiveCheckers),
}

impl CheckerPolicy {
    /// `requested == 0` means "work it out at runtime".
    pub fn from_request(requested: usize) -> Self {
        match requested {
            0 => Self::Adaptive(AdaptiveCheckers::new()),
            n => Self::Fixed(n.min(MAX_CHECKERS)),
        }
    }

    pub fn limit(&self) -> usize {
        match self {
            Self::Fixed(n) => *n,
            Self::Adaptive(adaptive) => adaptive.limit(),
        }
    }

    pub fn observe(&self, files: usize, elapsed: std::time::Duration) {
        if let Self::Adaptive(adaptive) = self {
            adaptive.observe(files, elapsed);
        }
    }

    /// The concurrency this run DISCOVERED, for the seed store (ph-3
    /// of `PERF_HISTORY_PLANNING`): `Some` only for an adaptive dial
    /// that actually settled. A run still probing when it ended
    /// returns `None`, and so does [`CheckerPolicy::Fixed`] — the
    /// hidden diagnostic pin discovered nothing and must never teach
    /// a seed.
    pub fn settled_limit(&self) -> Option<usize> {
        match self {
            Self::Fixed(_) => None,
            Self::Adaptive(adaptive) => adaptive.is_settled().then(|| adaptive.limit()),
        }
    }
}

/// A session's destination-comparison pool.
#[derive(Clone)]
pub struct CheckerPool {
    pool: Arc<rayon::ThreadPool>,
    threads: usize,
    policy: Arc<CheckerPolicy>,
    /// Times [`CheckerPool::install`] has run.
    ///
    /// cr-ls1-9: this exists so a test can prove the PRODUCTION diff actually
    /// routes through this pool. Without it, deleting the wiring at the call
    /// site leaves the pool's own unit tests green and the feature silently
    /// gone — which is exactly what the reviewer demonstrated.
    installs: Arc<AtomicU64>,
}

impl CheckerPool {
    /// Build the pool. `requested == 0` discovers the concurrency at
    /// runtime; any other value pins it (clamped to `1..=MAX_CHECKERS`).
    ///
    /// The pool is always built at the CEILING the policy could ask for,
    /// because rayon pools cannot be resized. Live concurrency is bounded by
    /// slicing work, not by the thread count — idle threads park and cost
    /// nothing.
    pub fn new(requested: usize) -> Result<Self> {
        let policy = CheckerPolicy::from_request(requested);
        let threads = match &policy {
            CheckerPolicy::Fixed(n) => *n,
            CheckerPolicy::Adaptive(_) => *LADDER.last().expect("ladder is non-empty"),
        }
        .clamp(1, MAX_CHECKERS);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("blit-checker-{index}"))
            .build()
            .context("building the destination-comparison thread pool")?;
        Ok(Self {
            pool: Arc::new(pool),
            threads,
            policy: Arc::new(policy),
            installs: Arc::new(AtomicU64::new(0)),
        })
    }

    /// How many times work has been dispatched onto this pool. Zero after a
    /// whole session means the diff never used it.
    pub fn installs(&self) -> u64 {
        self.installs.load(Ordering::Relaxed)
    }

    /// Resolve a requested count to the count that will actually be used.
    /// Split out so the clamping is testable without spawning threads.
    pub fn resolve(requested: usize) -> usize {
        match requested {
            0 => ADAPTIVE_FLOOR,
            n => n.min(MAX_CHECKERS),
        }
    }

    /// Threads the pool owns. This is the ceiling, not the live concurrency
    /// — see [`CheckerPool::limit`].
    pub fn threads(&self) -> usize {
        self.threads
    }

    /// In-flight checks permitted for the next chunk.
    pub fn limit(&self) -> usize {
        self.policy.limit().clamp(1, self.threads)
    }

    /// Fold one chunk's throughput back into the controller.
    pub fn observe(&self, files: usize, elapsed: std::time::Duration) {
        self.policy.observe(files, elapsed);
    }

    /// See [`CheckerPolicy::settled_limit`]; clamped like [`Self::limit`].
    pub fn settled_limit(&self) -> Option<usize> {
        self.policy
            .settled_limit()
            .map(|n| n.clamp(1, self.threads))
    }

    pub fn policy(&self) -> &CheckerPolicy {
        &self.policy
    }

    /// Run `op` inside the pool, so any rayon parallelism it starts is
    /// confined to these threads instead of the global pool.
    pub fn install<T: Send>(&self, op: impl FnOnce() -> T + Send) -> T {
        self.installs.fetch_add(1, Ordering::Relaxed);
        self.pool.install(op)
    }
}

impl std::fmt::Debug for CheckerPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckerPool")
            .field("threads", &self.threads)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[test]
    fn zero_means_adaptive_and_the_cap_binds() {
        assert_eq!(CheckerPool::resolve(0), ADAPTIVE_FLOOR);
        assert_eq!(CheckerPool::resolve(1), 1);
        assert_eq!(CheckerPool::resolve(16), 16);
        assert_eq!(CheckerPool::resolve(MAX_CHECKERS + 1_000), MAX_CHECKERS);
        assert!(matches!(
            CheckerPolicy::from_request(0),
            CheckerPolicy::Adaptive(_)
        ));
        assert!(matches!(
            CheckerPolicy::from_request(4),
            CheckerPolicy::Fixed(4)
        ));
    }

    /// The controller must CLIMB while throughput keeps improving. This is
    /// the high-latency-destination case: more in-flight checks keep paying
    /// off, so it should walk up the ladder rather than sit at the floor.
    #[test]
    fn improving_throughput_climbs_the_ladder() {
        let adaptive = AdaptiveCheckers::new();
        let start = adaptive.limit();
        assert_eq!(start, ADAPTIVE_FLOOR);

        // Each observation is faster than the last.
        let mut files = 100usize;
        for _ in 0..5 {
            adaptive.observe(files, std::time::Duration::from_millis(100));
            files *= 2;
        }
        assert!(
            adaptive.limit() > start,
            "throughput kept improving but concurrency stayed at {}",
            adaptive.limit()
        );
    }

    /// And it must STOP when a step stops paying, giving the last step back.
    /// This is the saturated-destination case — the one where piling on more
    /// concurrency costs the server without buying throughput.
    #[test]
    fn regressing_throughput_backs_off_and_settles() {
        let adaptive = AdaptiveCheckers::new();
        // Climb twice on genuine improvement, then hit a wall. Note the
        // second observation must be BETTER, not equal: an equal one is a
        // plateau and settles the dial before any regression arrives.
        adaptive.observe(1000, std::time::Duration::from_millis(100));
        adaptive.observe(2000, std::time::Duration::from_millis(100));
        let peak = adaptive.limit();
        assert!(!adaptive.is_settled(), "still climbing at this point");

        adaptive.observe(500, std::time::Duration::from_millis(100));
        assert!(adaptive.is_settled(), "a regression must settle the dial");
        assert!(
            adaptive.limit() < peak,
            "a regression must give the last step back: peak {peak}, now {}",
            adaptive.limit()
        );
    }

    #[test]
    fn a_flat_response_settles_without_climbing_further() {
        let adaptive = AdaptiveCheckers::new();
        adaptive.observe(1000, std::time::Duration::from_millis(100));
        let after_first = adaptive.limit();
        // Same rate: no reason to spend more concurrency.
        adaptive.observe(1000, std::time::Duration::from_millis(100));
        assert!(adaptive.is_settled());
        assert_eq!(adaptive.limit(), after_first);
    }

    #[test]
    fn a_pinned_count_never_moves() {
        // The hidden diagnostic pin must hold exactly, or a comparison run
        // against the adaptive policy would not be measuring what it claims.
        let policy = CheckerPolicy::from_request(3);
        assert_eq!(policy.limit(), 3);
        for _ in 0..10 {
            policy.observe(10_000, std::time::Duration::from_millis(1));
            assert_eq!(policy.limit(), 3);
        }
    }

    #[test]
    fn degenerate_observations_are_ignored() {
        let adaptive = AdaptiveCheckers::new();
        let before = adaptive.limit();
        adaptive.observe(0, std::time::Duration::from_millis(10));
        adaptive.observe(10, std::time::Duration::ZERO);
        assert_eq!(adaptive.limit(), before);
        assert!(!adaptive.is_settled());
    }

    #[test]
    fn an_unconfigured_pool_is_not_single_threaded() {
        // The defect this pool exists to fix was a hard-coded single
        // comparison thread. Asserted through `new` rather than on a
        // constant, so it covers the path an unconfigured caller actually
        // takes — and so it is not a compile-time-constant comparison, which
        // proves nothing.
        let pool = CheckerPool::new(0).expect("pool");
        assert!(
            pool.limit() > 1,
            "an unconfigured session started at {} concurrent check(s); a \
             single-threaded default is the bug, not a safe choice",
            pool.limit()
        );
        assert!(
            pool.threads() >= pool.limit(),
            "the pool must be able to supply the concurrency the controller \
             asks for"
        );
    }

    #[test]
    fn an_adaptive_pool_raises_its_own_limit_from_observations() {
        // End to end through the pool, not just the bare controller: an
        // unconfigured session must be able to move its own concurrency up
        // without anyone passing a number in.
        let pool = CheckerPool::new(0).expect("pool");
        let start = pool.limit();
        let mut files = 100usize;
        for _ in 0..4 {
            pool.observe(files, std::time::Duration::from_millis(50));
            files *= 3;
        }
        assert!(
            pool.limit() > start,
            "adaptive pool stayed at {start} despite improving throughput"
        );
    }

    /// The pool must actually run work concurrently — the property the whole
    /// feature rests on, and the one a file-set assertion cannot see
    /// (cr-ls1-8, where swapping the parallel iterator for a sequential one
    /// left the "guard" green).
    #[test]
    fn work_really_runs_concurrently() {
        use rayon::prelude::*;

        const THREADS: usize = 4;
        const ITEMS: usize = 64;
        let pool = CheckerPool::new(THREADS).expect("pool");
        assert_eq!(pool.threads(), THREADS);

        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let seen_threads: Arc<Mutex<HashSet<String>>> = Arc::default();

        pool.install(|| {
            (0..ITEMS).into_par_iter().for_each(|_| {
                let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                // Long enough that genuinely sequential execution cannot
                // produce overlap, short enough to keep the test quick.
                std::thread::sleep(std::time::Duration::from_millis(20));
                if let Some(name) = std::thread::current().name() {
                    seen_threads
                        .lock()
                        .expect("poisoned")
                        .insert(name.to_string());
                }
                in_flight.fetch_sub(1, Ordering::SeqCst);
            });
        });

        assert!(
            peak.load(Ordering::SeqCst) > 1,
            "peak in-flight was {} — the pool ran the work sequentially",
            peak.load(Ordering::SeqCst)
        );
        let names = seen_threads.lock().expect("poisoned");
        assert!(
            names.iter().all(|name| name.starts_with("blit-checker-")),
            "work escaped the dedicated pool onto {names:?} — the whole point \
             is that these blocking calls never touch a shared pool"
        );
    }

    #[test]
    fn one_checker_is_honoured_exactly() {
        // `--checkers 1` must mean one, so an operator can pin the old
        // behaviour on a destination that dislikes concurrency.
        use rayon::prelude::*;
        let pool = CheckerPool::new(1).expect("pool");
        let peak = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));
        pool.install(|| {
            (0..16).into_par_iter().for_each(|_| {
                let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(2));
                in_flight.fetch_sub(1, Ordering::SeqCst);
            });
        });
        assert_eq!(peak.load(Ordering::SeqCst), 1);
    }
}
